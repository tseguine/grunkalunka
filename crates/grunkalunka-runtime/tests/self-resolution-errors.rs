use grunkalunka_compiler::compile_async;
use grunkalunka_runtime::{AsyncInstance, AsyncState, PromiseNotification, Value};
use std::{
    io::Write,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const SOURCE: &str = "export async function run(a,b){const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();let reason=null;try{await p;}catch(e){reason=e;}const h=async()=>{await a;throw reason;};h();return reason;}";
const OWN_KEYS_SOURCE: &str = "export async function run(a,b){let reason=null;try{Object.keys(null);}catch(e){reason=e;}const h=async()=>{await a;throw reason;};h();return reason;}";

fn peer(payload: &str, kind: Option<i32>, later: bool, trap: bool) -> Vec<u8> {
    let extension = kind
        .map(|kind| {
            format!(
                "(func (export \"error_kind\") (param f64) (result i32) {} i32.const {kind})",
                if trap { "unreachable" } else { "" }
            )
        })
        .unwrap_or_default();
    wat::parse_str(format!(r#"(module
      (global $issued (mut i32) (i32.const {issued}))
      (func (export "start") (param f64 f64) (result i32 i32 f64) {start})
      (func (export "resume") (param i32 f64) (result i32 i32 f64) i32.const 0 i32.const 4 f64.const {payload})
      (func (export "take_jobs") (result i32) (local $n i32) global.get $issued local.set $n i32.const 0 global.set $issued local.get $n)
      {extension})"#,issued=i32::from(later),start=if later{"i32.const 1 i32.const 0 f64.const 0".to_owned()}else{format!("i32.const 0 i32.const 4 f64.const {payload}")})).unwrap()
}

#[test]
fn self_resolution_errors_keep_identity_after_root_settlement_and_in_notifications() {
    for source in [SOURCE, OWN_KEYS_SOURCE] {
        for ts in [false, true] {
            let bytes = compile_async(source, ts).unwrap().wasm;
            let mut run = AsyncInstance::new(&bytes, 4.0, 2.0).unwrap();
            let mut events = Vec::new();
            events.extend(run.take_promise_notifications().unwrap());
            while run.run_one_job().unwrap() {
                events.extend(run.take_promise_notifications().unwrap());
            }
            let AsyncState::Fulfilled(Value::TypeError { id }) = run.state().unwrap() else {
                panic!("root must fulfill with an Error object");
            };
            assert!(id >= 16 && id % 16 == 0);
            assert!(
                matches!(&events[..],[PromiseNotification::Rejected{reason:Value::TypeError{id:reason},..}] if *reason==id)
            );
            assert!(run.take_promise_notifications().unwrap().is_empty());
            assert!(!run.run_one_job().unwrap());
            assert!(
                matches!(run.state().unwrap(),AsyncState::Fulfilled(Value::TypeError{id:again}) if again==id)
            );
        }
    }
}

#[test]
fn own_keys_source_throws_are_distinct_from_invalid_private_values_and_records() {
    let source =
        "export async function run(a,b){try{Object.keys(null);}catch(e){return e;}return a;}";
    let artifact = compile_async(source, false).unwrap();
    let end = artifact.wat.rfind(')').unwrap();
    let wat = format!(
        r#"{} (export "test_memory" (memory 0))
        (func (export "known_null")
          i32.const 1 f64.const 0 ref.null eq struct.new $gc_value
          call $gc_own_keys_known drop))"#,
        &artifact.wat[..end]
    );
    let mut config = wasmtime::Config::new();
    config.consume_fuel(true);
    let engine = wasmtime::Engine::new(&config).unwrap();
    let module = wasmtime::Module::new(&engine, wat::parse_str(wat).unwrap()).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    store.set_fuel(100_000).unwrap();
    let guest = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let start = guest
        .get_typed_func::<(f64, f64), (i32, i32, f64)>(&mut store, "start")
        .unwrap();
    let state = start.call(&mut store, (4.0, 2.0)).unwrap();
    assert_eq!((state.0, state.1), (0, 4));
    let kind = guest
        .get_typed_func::<f64, i32>(&mut store, "error_kind")
        .unwrap();
    assert_eq!(kind.call(&mut store, state.2).unwrap(), 2);
    let memory = guest.get_memory(&mut store, "test_memory").unwrap();
    for invalid in [0u32, 3, u32::MAX] {
        memory
            .write(&mut store, state.2 as usize + 4, &invalid.to_le_bytes())
            .unwrap();
        assert!(kind.call(&mut store, state.2).is_err());
        memory
            .write(&mut store, state.2 as usize + 4, &2u32.to_le_bytes())
            .unwrap();
    }
    assert_eq!(kind.call(&mut store, state.2).unwrap(), 2);
    assert!(
        guest
            .get_typed_func::<(), ()>(&mut store, "known_null")
            .unwrap()
            .call(&mut store, ())
            .is_err()
    );
    store.set_fuel(0).unwrap();
    let exhausted = start.call(&mut store, (4.0, 2.0)).unwrap_err();
    assert!(format!("{exhausted:?}").contains("fuel"));
}

#[test]
fn self_resolution_error_protocol_is_opt_in_and_validates_identity_kind_and_traps() {
    for (payload, kind, trap) in [
        ("32", None, false),
        ("32", Some(0), false),
        ("32", Some(3), false),
        ("32", Some(1), true),
        ("32", Some(2), true),
        ("0", Some(1), false),
        ("-0", Some(1), false),
        ("nan", Some(1), false),
        ("inf", Some(1), false),
        ("16.5", Some(1), false),
        ("17", Some(1), false),
        ("1048576", Some(1), false),
    ] {
        assert!(AsyncInstance::new(&peer(payload, kind, false, trap), 4.0, 2.0).is_err());
        let mut run = AsyncInstance::new(&peer(payload, kind, true, trap), 4.0, 2.0).unwrap();
        assert!(run.run_one_job().is_err());
        assert!(run.state().is_err());
        assert!(run.take_promise_notifications().is_err());
        assert!(run.pending_job_count().is_err());
        assert!(run.run_one_job().is_err());
    }
    for kind in [1, 2] {
        for later in [false, true] {
            let mut run =
                AsyncInstance::new(&peer("32", Some(kind), later, false), 4.0, 2.0).unwrap();
            if later {
                assert!(run.run_one_job().unwrap());
            }
            assert!(matches!(
                run.state().unwrap(),
                AsyncState::Fulfilled(Value::TypeError { id: 32 })
            ));
        }
    }
}

#[test]
fn actual_guest_error_records_reject_wrong_kind_alignment_range_and_numeric_payloads() {
    let wasm = compile_async(SOURCE, false).unwrap().wasm;
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &wasm).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let guest = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let start = guest
        .get_typed_func::<(f64, f64), (i32, i32, f64)>(&mut store, "start")
        .unwrap();
    let resume = guest
        .get_typed_func::<(i32, f64), (i32, i32, f64)>(&mut store, "resume")
        .unwrap();
    let jobs = guest
        .get_typed_func::<(), i32>(&mut store, "take_jobs")
        .unwrap();
    let kind = guest
        .get_typed_func::<f64, i32>(&mut store, "error_kind")
        .unwrap();
    let mut state = start.call(&mut store, (4.0, 2.0)).unwrap();
    let mut owed = jobs.call(&mut store, ()).unwrap();
    let mut count = 0;
    while owed > 0 {
        count += 1;
        assert!(count <= 32);
        owed -= 1;
        state = resume.call(&mut store, (0, 0.0)).unwrap();
        owed += jobs.call(&mut store, ()).unwrap();
    }
    assert_eq!((state.0, state.1), (0, 4));
    let id = state.2;
    assert_eq!(kind.call(&mut store, id).unwrap(), 1);
    for invalid in [
        0.0,
        -0.0,
        f64::NAN,
        f64::INFINITY,
        16.0,
        17.0,
        id + 1.0,
        id + 16.0,
        1_048_544.0,
    ] {
        assert!(kind.call(&mut store, invalid).is_err(), "{invalid}");
    }
    assert_eq!(kind.call(&mut store, id).unwrap(), 1);
}

#[test]
fn node_self_resolution_errors_are_native_objects_with_stable_and_separate_identities() {
    let mut cases = Vec::new();
    for ts in [false, true] {
        for source in [SOURCE, OWN_KEYS_SOURCE] {
            let wasm = compile_async(source, ts).unwrap().wasm;
            cases.push(format!("{{\"source\":{source:?},\"wasm\":{wasm:?}}}"));
        }
    }
    for kind in [1, 2] {
        for later in [false, true] {
            let wasm = peer("32", Some(kind), later, false);
            cases.push(format!("{{\"kind\":{kind},\"wasm\":{wasm:?}}}"));
        }
    }
    for (payload, kind, trap) in [
        ("32", None, false),
        ("32", Some(0), false),
        ("32", Some(3), false),
        ("32", Some(1), true),
        ("32", Some(2), true),
        ("0", Some(1), false),
        ("-0", Some(1), false),
        ("nan", Some(1), false),
        ("inf", Some(1), false),
        ("16.5", Some(1), false),
        ("17", Some(1), false),
        ("1048576", Some(1), false),
    ] {
        for later in [false, true] {
            let wasm = peer(payload, kind, later, trap);
            cases.push(format!("{{\"protocol\":true,\"wasm\":{wasm:?}}}"));
        }
    }
    let runner =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/self-resolution-errors.mjs");
    let mut child = Command::new("node")
        .arg(runner)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("[{}]", cases.join(",")).as_bytes())
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("Error observation exceeded30s");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("ERROR_OBJECTS_DONE"));
}
