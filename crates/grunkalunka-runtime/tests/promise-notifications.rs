use grunkalunka_compiler::compile_async;
use grunkalunka_runtime::{AsyncInstance, AsyncState, PromiseNotification, Value};
use std::{
    io::Write,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const BODIES: &[&str] = &[
    "const f=async()=>{await a;throw b;};f();return a;",
    "const f=async()=>{await a;throw void 0;};f();return a;",
    "const f=async()=>{await a;throw null;};f();return a;",
    "const f=async()=>{await a;throw false;};f();return a;",
    "const f=async()=>{await a;throw true;};f();return a;",
    "const f=async()=>{await a;throw b;};const p=f();try{await p;}catch(e){}return a;",
    "const f=async()=>{await a;throw b;};const p=f();await a;await b;try{await p;}catch(e){}try{await p;}catch(e){}return a;",
    "const f=async()=>{if(a)throw b;await a;return b;};const p=f();try{await p;}catch(e){}return a;",
    "const f=async()=>{await a;throw b;};const g=async()=>{await b;throw a;};f();g();return a;",
    "const f=async()=>{if(a){await a;}else{await a;await b;}throw b;};f();if(a){await a;await b;await a;await b;}return a;",
    "const f=async()=>{await a;throw b;};const g=async()=>{try{await f();}catch(e){throw e;}};g();return a;",
];

// Deliberately malformed scheduler peers test adapter validation independently
// of the compiler/kernel. Each peer reports one event at each guest boundary.
fn event_peer(kind: i32, id: i32, tag: i32, payload: &str, next: bool, repeat: bool) -> Vec<u8> {
    wat::parse_str(format!(r#"(module
      (global $sent (mut i32) (i32.const 0))
      (global $issued (mut i32) (i32.const 1))
      (global $phase (mut i32) (i32.const 0))
      (func (export "start") (param f64 f64) (result i32 i32 f64) i32.const 0 i32.const 3 f64.const 4)
      (func (export "resume") (param i32 f64) (result i32 i32 f64)
        i32.const 1 global.set $phase i32.const 0 global.set $sent
        i32.const 0 i32.const 3 f64.const 4)
      (func (export "take_jobs") (result i32) (local $n i32)
        global.get $issued local.set $n i32.const 0 global.set $issued local.get $n)
      (func (export "take_promise_event") (result i32 i32 i32 f64)
        global.get $sent if i32.const 0 i32.const 0 i32.const 0 f64.const 0 return end
        i32.const {sent} global.set $sent
        global.get $phase i32.const {next} i32.ne
        if i32.const 1 i32.const 16 i32.const 3 f64.const 2 return end
        i32.const {kind} i32.const {id} i32.const {tag} f64.const {payload}))"#,
        sent=if repeat{0}else{1},next=i32::from(next))).unwrap()
}

#[test]
fn notification_protocol_errors_terminate_the_invocation() {
    for (kind, id, tag, payload) in [
        (7, 16, 0, "0"),
        (1, 0, 3, "2"),
        (1, 17, 3, "2"),
        (1, 16, 2, "-0"),
        (2, 16, 0, "0"),
        (0, 16, 0, "0"),
    ] {
        assert!(
            AsyncInstance::new(&event_peer(kind, id, tag, payload, false, false), 4.0, 2.0)
                .is_err()
        );
    }
    for (kind, id, tag, payload) in [
        (1, 16, 3, "2"),
        (2, 32, 0, "0"),
        (2, 16, 0, "-0"),
        (0, 0, 3, "0"),
    ] {
        let mut run =
            AsyncInstance::new(&event_peer(kind, id, tag, payload, true, false), 4.0, 2.0).unwrap();
        assert!(run.run_one_job().is_err());
        assert!(run.state().is_err());
        assert!(run.pending_job_count().is_err());
        assert!(run.take_promise_notifications().is_err());
        assert!(run.run_one_job().is_err());
    }
    assert!(AsyncInstance::new(&event_peer(1, 16, 3, "2", false, true), 4.0, 2.0).is_err());
    let mut run = AsyncInstance::new(&event_peer(2, 16, 0, "0", true, false), 4.0, 2.0).unwrap();
    assert!(matches!(
        &run.take_promise_notifications().unwrap()[..],
        [PromiseNotification::Rejected {
            id: 16,
            reason: Value::Number(2.0)
        }]
    ));
    assert!(run.run_one_job().unwrap());
    assert!(matches!(
        &run.take_promise_notifications().unwrap()[..],
        [PromiseNotification::Handled { id: 16 }]
    ));
    assert!(!run.run_one_job().unwrap());
}

#[test]
fn wasmtime_tracks_rejection_identity_and_late_handling_after_root_settlement() {
    for ts in [false, true] {
        for reason in [2.0, -0.0, f64::NAN, f64::INFINITY] {
            for (index, body) in BODIES.iter().enumerate() {
                let wasm = compile_async(&format!("export async function run(a,b){{{body}}}"), ts)
                    .unwrap()
                    .wasm;
                let mut run = AsyncInstance::new(&wasm, 4.0, reason).unwrap();
                if index == 0 {
                    assert!(matches!(
                        run.state().unwrap(),
                        AsyncState::Fulfilled(Value::Number(4.0))
                    ));
                    assert_eq!(run.pending_job_count().unwrap(), 1);
                }
                let mut events = run.take_promise_notifications().unwrap();
                while run.run_one_job().unwrap() {
                    events.extend(run.take_promise_notifications().unwrap());
                }
                assert!(run.take_promise_notifications().unwrap().is_empty());
                assert!(matches!(
                    run.state().unwrap(),
                    AsyncState::Fulfilled(Value::Number(4.0))
                ));
                let mut unhandled = std::collections::BTreeMap::new();
                let mut ids = std::collections::BTreeSet::new();
                for event in &events {
                    match event {
                        PromiseNotification::Rejected { id, reason } => {
                            assert!(*id >= 16 && id % 16 == 0);
                            assert!(ids.insert(*id));
                            assert!(unhandled.insert(*id, reason).is_none());
                        }
                        PromiseNotification::Handled { id } => {
                            assert!(unhandled.remove(id).is_some());
                        }
                    }
                }
                if [5, 6, 7].contains(&index) {
                    assert!(unhandled.is_empty(), "{body}");
                    if index == 6 || index == 7 {
                        assert_eq!(events.len(), 2);
                    } else {
                        assert!(events.is_empty());
                    }
                } else {
                    assert_eq!(unhandled.len(), if index == 8 { 2 } else { 1 }, "{body}");
                    assert_eq!(events.len(), unhandled.len());
                    let first = unhandled.values().next().unwrap();
                    match (index, *first) {
                        (1, Value::Undefined)
                        | (2, Value::Null)
                        | (3, Value::Boolean(false))
                        | (4, Value::Boolean(true)) => {}
                        (0 | 8 | 9 | 10, Value::Number(x)) => assert!(
                            x.is_nan() && reason.is_nan() || x.to_bits() == reason.to_bits()
                        ),
                        _ => panic!("unexpected reason {first:?} in {body}"),
                    }
                }
            }
        }
    }
}

#[test]
fn node_reports_detached_child_rejections_like_native_promises() {
    let mut cases = Vec::new();
    for body in BODIES {
        for ts in [false, true] {
            let source = format!("export async function run(a,b){{{body}}}");
            let compiled = compile_async(&source, ts).unwrap();
            cases.push(format!(
                "{{\"source\":{source:?},\"wasm\":{:?}}}",
                compiled.wasm
            ));
        }
    }
    for (kind, id, tag, payload, next) in [
        (7, 16, 0, "0", false),
        (1, 0, 3, "2", false),
        (1, 17, 3, "2", false),
        (1, 16, 2, "-0", false),
        (2, 16, 0, "0", false),
        (0, 16, 0, "0", false),
        (1, 16, 3, "2", true),
        (2, 32, 0, "0", true),
        (2, 16, 0, "-0", true),
        (0, 0, 3, "0", true),
    ] {
        let wasm = event_peer(kind, id, tag, payload, next, false);
        cases.push(format!(
            "{{\"protocol\":true,\"wasm\":{wasm:?},\"later\":{next}}}"
        ));
    }
    let runner =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/promise-notifications.mjs");
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
            panic!("Promise notification observation exceeded 30s");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("NOTIFICATIONS_DONE"));
}
