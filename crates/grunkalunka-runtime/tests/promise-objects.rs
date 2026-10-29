use std::{
    io::Write,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn fulfilled_peer() -> Vec<u8> {
    wat::parse_str(
        r#"(module
        (global $value (mut f64) (f64.const 0))
        (func (export "start") (param f64 f64) (result i32 i32 f64)
            local.get 1 global.set $value i32.const 3 i32.const 5 f64.const 32)
        (func (export "resume") (param i32 f64) (result i32 i32 f64) unreachable)
        (func (export "take_jobs") (result i32) i32.const 0)
        (func (export "promise_state") (param f64) (result i32 i32 i32 f64)
            i32.const 1 i32.const 0 i32.const 3 global.get $value))"#,
    )
    .unwrap()
}

#[test]
fn node_rejection_reason_is_a_real_promise() {
    run_node(&[("fulfilled", fulfilled_peer())]);
}

fn run_node<S: AsRef<str>>(cases: &[(S, Vec<u8>)]) {
    let cases = cases
        .iter()
        .map(|(name, wasm)| {
            let name = name.as_ref();
            format!(r#"{{"name":{name:?},"wasm":{wasm:?}}}"#)
        })
        .collect::<Vec<_>>()
        .join(",");
    run_node_json(format!("[{cases}]"));
}

fn run_source_node(cases: &[(&str, &str)]) {
    let cases = cases
        .iter()
        .map(|(name, source)| {
            let js = grunkalunka_compiler::compile_async(source, false).unwrap();
            let ts = grunkalunka_compiler::compile_async(source, true).unwrap();
            assert_eq!(js.wasm, ts.wasm, "{name}: type erasure");
            let wasm = js.wasm;
            format!(r#"{{"name":{name:?},"source":{source:?},"wasm":{wasm:?}}}"#)
        })
        .collect::<Vec<_>>()
        .join(",");
    run_node_json(format!("[{cases}]"));
}

fn run_node_json(cases: String) {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/promise-objects.mjs");
    let adapter = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("js/async.mjs");
    let mut child = Command::new("node")
        .arg(script)
        .arg(adapter)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(cases.as_bytes())
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("Promise object adapter comparison exceeded30s");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

const THROW_SOURCES: &[(&str, &str)] = &[
    (
        "fulfilled-direct",
        "export async function run(a,b){const p=Promise.resolve(a);throw p;}",
    ),
    (
        "fulfilled-mixed",
        "export async function run(a,b){const x=a?Promise.resolve(a):a;throw Promise.resolve(x);}",
    ),
    (
        "fulfilled",
        "async function f(p){throw p;}export async function run(a,b){return await f(Promise.resolve(a));}",
    ),
    (
        "pending",
        "export async function run(a,b){const f=async()=>{await a;await b;return b;};throw f();}",
    ),
    (
        "rejected",
        "export async function run(a,b){const f=async()=>{throw b;};throw f();}",
    ),
    (
        "rejected-unhandled",
        "export async function run(a,b){const f=async()=>{throw b;};throw f();}",
    ),
    (
        "self",
        "export async function run(a,b){let p;const f=async()=>{await a;throw p;};p=f();throw p;}",
    ),
    (
        "shared",
        "export async function run(a,b){const p=Promise.resolve(a);const f=async()=>{throw p;};f();throw p;}",
    ),
    (
        "nested",
        "export async function run(a,b){const q=Promise.resolve(a);const f=async()=>{throw q;};throw f();}",
    ),
    (
        "handled",
        "export async function run(a,b){const f=async()=>{throw b;};const p=f();try{await p;}catch(e){}throw p;}",
    ),
    (
        "fulfilled-finally",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}finally{await b;}}",
    ),
    (
        "fulfilled-suspended",
        "export async function run(a,b){let f=async()=>{await a;throw Promise.resolve(b);};return await f();}",
    ),
    (
        "choice",
        "export async function run(a,b){const f=a?async()=>{throw Promise.resolve(a);}:async()=>b;return await f();}",
    ),
];

#[test]
fn node_source_throws_expose_real_promise_objects_without_adoption() {
    run_source_node(THROW_SOURCES);
}

const REJECT_SOURCES: &[(&str, &str)] = &[
    (
        "fulfilled-direct",
        "export async function run(a,b){const p=Promise.resolve(a);return Promise.reject(p);}",
    ),
    (
        "fulfilled",
        "async function f(p){return Promise.reject(p);}export async function run(a,b){return await f(Promise.resolve(a));}",
    ),
    (
        "fulfilled-await",
        "export async function run(a,b){const p=Promise.resolve(a);return await Promise.reject(p);}",
    ),
    (
        "pending",
        "export async function run(a,b){let f=async()=>{await a;return b;};return await Promise.reject(f());}",
    ),
    (
        "pending",
        "export async function run(a,b){const f=async()=>{await a;return b;};const p=f();return await Promise.reject(p);}",
    ),
    (
        "mixed-reason",
        "export async function run(a,b){let f=async()=>{await a;return b;};return await Promise.reject(a?f():b);}",
    ),
    (
        "rejected",
        "export async function run(a,b){const p=Promise.reject(b);return Promise.reject(p);}",
    ),
    (
        "rejected-a",
        "export async function run(a,b){let f=async()=>{throw a;};return await Promise.reject(f());}",
    ),
    (
        "rejected-unhandled",
        "export async function run(a,b){const p=Promise.reject(b);return Promise.reject(p);}",
    ),
    (
        "self",
        "export async function run(a,b){let p;const f=async()=>{await a;return Promise.reject(p);};p=f();return Promise.reject(p);}",
    ),
    (
        "shared",
        "export async function run(a,b){const p=Promise.resolve(a);Promise.reject(p);return Promise.reject(p);}",
    ),
    (
        "nested",
        "export async function run(a,b){const p=Promise.resolve(a);return Promise.reject(Promise.reject(p));}",
    ),
    (
        "nested",
        "export async function run(a,b){throw Promise.reject(Promise.resolve(a));}",
    ),
    (
        "handled",
        "export async function run(a,b){const p=Promise.reject(b);try{await p;}catch(e){}return Promise.reject(p);}",
    ),
    (
        "fulfilled-finally",
        "export async function run(a,b){const p=Promise.resolve(a);try{return Promise.reject(p);}finally{await b;}}",
    ),
    (
        "choice",
        "export async function run(a,b){const f=a?async()=>Promise.reject(Promise.resolve(a)):async()=>b;return await f();}",
    ),
];

#[test]
fn node_source_rejections_expose_promise_objects_without_adoption() {
    run_source_node(REJECT_SOURCES);
}

#[test]
fn wasmtime_source_throws_keep_readonly_nested_and_self_reasons() {
    wasmtime_source_reasons(THROW_SOURCES);
}

#[test]
fn wasmtime_source_rejections_keep_readonly_nested_and_self_reasons() {
    wasmtime_source_reasons(REJECT_SOURCES);
}

fn wasmtime_source_reasons(sources: &[(&str, &str)]) {
    use grunkalunka_runtime::{AsyncState, PreparedAsyncModule, PromiseNotification, Value};
    for (name, source) in sources {
        let artifact = grunkalunka_compiler::compile_async(source, false).unwrap();
        let module = PreparedAsyncModule::new(&artifact.wasm).unwrap();
        for (a, b) in [
            (4.0, 2.0),
            (0.0, -0.0),
            (-2.0, -4.0),
            (f64::NAN, f64::INFINITY),
        ] {
            let mut run = module.instantiate(a, b).unwrap();
            let mut steps = 0;
            while run.pending_job_count().unwrap() != 0 {
                assert!(steps < 32, "{name}: bounded guest jobs");
                assert!(run.run_one_job().unwrap());
                steps += 1;
            }
            if *name == "choice" && (a == 0.0 || a.is_nan()) {
                let AsyncState::Fulfilled(Value::Number(actual)) = run.state().unwrap() else {
                    panic!("choice primitive fulfillment");
                };
                assert_eq!(actual.to_bits(), b.to_bits());
                continue;
            }
            if *name == "mixed-reason" && (a == 0.0 || a.is_nan()) {
                let AsyncState::Rejected(Value::Number(actual)) = run.state().unwrap() else {
                    panic!("mixed primitive rejection");
                };
                assert_eq!(actual.to_bits(), b.to_bits());
                continue;
            }
            let AsyncState::Rejected(Value::Promise { id }) = run.state().unwrap() else {
                panic!("{name}: root must reject with a raw Promise");
            };
            let first = run.promise_state(id).unwrap();
            let second = run.promise_state(id).unwrap();
            assert_eq!(
                format!("{first:?}"),
                format!("{second:?}"),
                "{name}: query is read-only"
            );
            match first {
                AsyncState::Fulfilled(Value::Number(actual)) => {
                    let expected =
                        if matches!(*name, "pending" | "mixed-reason" | "fulfilled-suspended") {
                            b
                        } else {
                            a
                        };
                    assert!(
                        actual.to_bits() == expected.to_bits()
                            || actual.is_nan() && expected.is_nan(),
                        "{name}"
                    );
                }
                AsyncState::Rejected(Value::Number(actual)) => {
                    assert!(matches!(
                        *name,
                        "rejected" | "rejected-a" | "rejected-unhandled" | "handled"
                    ));
                    let expected = if *name == "rejected-a" { a } else { b };
                    assert!(
                        actual.to_bits() == expected.to_bits()
                            || actual.is_nan() && expected.is_nan()
                    );
                }
                AsyncState::Rejected(Value::Promise { id: nested }) => {
                    if *name == "self" {
                        assert_eq!(nested, id, "raw rejection never invokes self-resolution");
                    } else {
                        assert_eq!(*name, "nested");
                        assert_ne!(nested, id);
                        let AsyncState::Fulfilled(Value::Number(actual)) =
                            run.promise_state(nested).unwrap()
                        else {
                            panic!("nested fulfillment");
                        };
                        assert!(actual.to_bits() == a.to_bits() || actual.is_nan() && a.is_nan());
                    }
                }
                state => panic!("{name}: unexpected reason state {state:?}"),
            }
            assert_eq!(run.pending_job_count().unwrap(), 0);
            let notifications = run.take_promise_notifications().unwrap();
            if *name == "shared" {
                assert!(notifications.iter().any(|event| matches!(event, PromiseNotification::Rejected { reason: Value::Promise { id: reason }, .. } if *reason == id)));
            }
        }
    }
}

fn object_peer(name: &str) -> Vec<u8> {
    let pending = matches!(name, "pending" | "trap");
    let snapshot = match name {
        "pending" => {
            "global.get $step i32.const 2 i32.lt_u if(result i32 i32 i32 f64) i32.const 0 i32.const 0 i32.const 0 f64.const 0 else i32.const 1 i32.const 0 i32.const 3 global.get $value end"
        }
        "trap" => "i32.const 0 i32.const 0 i32.const 0 f64.const 0",
        "self" => "i32.const 2 i32.const 0 i32.const 5 f64.const 32",
        "handled" => "i32.const 2 i32.const 1 i32.const 3 global.get $value",
        "shared" => {
            "local.get 0 f64.const 48 f64.eq if(result i32 i32 i32 f64) i32.const 2 i32.const 0 i32.const 5 f64.const 32 else i32.const 1 i32.const 0 i32.const 3 global.get $value end"
        }
        "nested" => {
            "local.get 0 f64.const 32 f64.eq if(result i32 i32 i32 f64) i32.const 2 i32.const 0 i32.const 5 f64.const 48 else i32.const 1 i32.const 0 i32.const 3 global.get $value end"
        }
        _ => "i32.const 2 i32.const 0 i32.const 3 global.get $value",
    };
    let event = if name == "handled" {
        "global.get $event i32.const 1 i32.add global.set $event global.get $event i32.const 1 i32.eq if(result i32 i32 i32 f64)i32.const 1 i32.const 32 i32.const 3 global.get $value else global.get $event i32.const 2 i32.eq if(result i32 i32 i32 f64)i32.const 2 i32.const 32 i32.const 0 f64.const 0 else i32.const 0 i32.const 0 i32.const 0 f64.const 0 end end".into()
    } else if pending {
        "i32.const 0 i32.const 0 i32.const 0 f64.const 0".into()
    } else {
        let (id, tag, payload) = match name {
            "self" => (32, 5, "f64.const 32"),
            "shared" => (48, 5, "f64.const 32"),
            "nested" => (32, 5, "f64.const 48"),
            _ => (32, 3, "global.get $value"),
        };
        format!(
            "global.get $event if(result i32 i32 i32 f64) i32.const 0 i32.const 0 i32.const 0 f64.const 0 else i32.const 1 global.set $event i32.const 1 i32.const {id} i32.const {tag} {payload} end"
        )
    };
    object_peer_with_events(pending, snapshot, &event, name == "trap")
}

fn object_peer_with_events(pending: bool, snapshot: &str, event: &str, trap: bool) -> Vec<u8> {
    let resume = if trap {
        "unreachable"
    } else {
        "global.get $step i32.const 1 i32.add global.set $step global.get $step i32.const 2 i32.lt_u global.set $issued i32.const 3 i32.const 5 f64.const 32"
    };
    wat::parse_str(format!(
        r#"(module
        (global $value (mut f64) (f64.const 0))
        (global $step (mut i32) (i32.const 0))
        (global $event (mut i32) (i32.const 0))
        (global $issued (mut i32) (i32.const {issued}))
        (func (export "start") (param f64 f64) (result i32 i32 f64)
            local.get 1 global.set $value i32.const 3 i32.const 5 f64.const 32)
        (func (export "resume") (param i32 f64) (result i32 i32 f64)
            {resume})
        (func (export "take_jobs") (result i32) (local $n i32)
            global.get $issued local.set $n i32.const 0 global.set $issued local.get $n)
        (func (export "take_promise_event") (result i32 i32 i32 f64) {event})
        (func (export "promise_state") (param f64) (result i32 i32 i32 f64) {snapshot}))"#,
        issued = u8::from(pending)
    ))
    .unwrap()
}

#[test]
fn node_promise_objects_keep_pending_self_and_shared_rejection_identities() {
    run_node(
        &[
            "pending",
            "rejected",
            "rejected-unhandled",
            "self",
            "shared",
            "nested",
            "handled",
        ]
        .map(|name| (name, object_peer(name))),
    );
}

#[test]
fn wasmtime_promise_queries_preserve_pending_work_and_recursive_reasons() {
    use grunkalunka_runtime::{AsyncInstance, AsyncState, PromiseNotification, Value};
    let mut run = AsyncInstance::new(&object_peer("pending"), 4.0, -0.0).unwrap();
    assert_eq!(run.pending_job_count().unwrap(), 1);
    assert!(matches!(
        run.promise_state(32).unwrap(),
        AsyncState::Pending
    ));
    assert_eq!(run.pending_job_count().unwrap(), 1);
    for last in [false, true] {
        assert!(run.run_one_job().unwrap());
        let state = run.promise_state(32).unwrap();
        if last {
            assert!(
                matches!(state,AsyncState::Fulfilled(Value::Number(n)) if n.to_bits()==(-0.0f64).to_bits())
            );
        } else {
            assert!(matches!(state, AsyncState::Pending));
        }
        assert!(matches!(
            run.state().unwrap(),
            AsyncState::Rejected(Value::Promise { id: 32 })
        ));
    }
    assert!(!run.run_one_job().unwrap());
    let mut cycle = AsyncInstance::new(&object_peer("self"), 4.0, 2.0).unwrap();
    assert!(matches!(
        cycle.promise_state(32).unwrap(),
        AsyncState::Rejected(Value::Promise { id: 32 })
    ));
    assert!(matches!(
        cycle.take_promise_notifications().unwrap().as_slice(),
        [PromiseNotification::Rejected {
            id: 32,
            reason: Value::Promise { id: 32 }
        }]
    ));
    assert!(cycle.take_promise_notifications().unwrap().is_empty());
    let mut shared = AsyncInstance::new(&object_peer("shared"), 4.0, 2.0).unwrap();
    assert!(matches!(
        shared.promise_state(48).unwrap(),
        AsyncState::Rejected(Value::Promise { id: 32 })
    ));
    assert!(matches!(
        shared.take_promise_notifications().unwrap().as_slice(),
        [PromiseNotification::Rejected {
            id: 48,
            reason: Value::Promise { id: 32 }
        }]
    ));
}

fn changing_peer() -> Vec<u8> {
    wat::parse_str(r#"(module
        (global $reads (mut i32) (i32.const 0))
        (func (export "start") (param f64 f64) (result i32 i32 f64) i32.const 3 i32.const 5 f64.const 32)
        (func (export "resume") (param i32 f64) (result i32 i32 f64) unreachable)
        (func (export "take_jobs") (result i32) i32.const 0)
        (func (export "promise_state") (param f64) (result i32 i32 i32 f64)
            global.get $reads i32.const 1 i32.add global.set $reads
            global.get $reads i32.const 1 i32.eq
            if(result i32 i32 i32 f64) i32.const 1 i32.const 0 i32.const 3 f64.const 2
            else i32.const 0 i32.const 0 i32.const 0 f64.const 0 end))"#).unwrap()
}

#[test]
fn wasmtime_promise_snapshots_cannot_revert_a_settlement() {
    let mut run = grunkalunka_runtime::AsyncInstance::new(&changing_peer(), 4.0, 2.0).unwrap();
    assert!(run.promise_state(32).is_err());
    assert!(run.state().is_err());
    assert!(run.run_one_job().is_err());
}

#[test]
fn node_promise_objects_reject_invalid_snapshots_and_late_traps() {
    let graph = protocol_peer(ProtocolPeer {
        root: "i32.const 3 i32.const 5 f64.const 32",
        snapshot: Some(
            "local.get 0 f64.const 32 f64.eq if(result i32 i32 i32 f64)i32.const 2 i32.const 0 i32.const 5 f64.const 48 else global.get $event i32.const 1 i32.add global.set $event global.get $event i32.const 1 i32.eq if(result i32 i32 i32 f64)i32.const 1 i32.const 0 i32.const 3 global.get $value else i32.const 3 i32.const 0 i32.const 0 f64.const 0 end end",
        ),
        event: None,
        queued: true,
    });
    run_node(&[
        ("changing", changing_peer()),
        ("trap", object_peer("trap")),
        ("invalid:graph", graph),
    ]);
}

struct ProtocolPeer<'a> {
    root: &'a str,
    snapshot: Option<&'a str>,
    event: Option<&'a str>,
    queued: bool,
}

fn protocol_peer(peer: ProtocolPeer<'_>) -> Vec<u8> {
    let query = peer
        .snapshot
        .map(|body| {
            format!(r#"(func(export "promise_state")(param f64)(result i32 i32 i32 f64){body})"#)
        })
        .unwrap_or_default();
    let queued = if peer.queued {
        r#"(func(export "take_jobs")(result i32)i32.const 0)"#
    } else {
        ""
    };
    let event = peer
        .event
        .map(|body| format!(r#"(func(export "take_promise_event")(result i32 i32 i32 f64){body})"#))
        .unwrap_or_default();
    wat::parse_str(format!(r#"(module
        (global $value(mut f64)(f64.const 0))(global $event(mut i32)(i32.const 0))
        (func(export "start")(param f64 f64)(result i32 i32 f64)local.get 1 global.set $value {root})
        (func(export "resume")(param i32 f64)(result i32 i32 f64)unreachable)
        {query}{queued}{event})"#,root=peer.root)).unwrap()
}

fn invalid_protocol_cases() -> Vec<(String, Vec<u8>)> {
    let root = "i32.const 3 i32.const 5 f64.const 32";
    let valid = "i32.const 1 i32.const 0 i32.const 3 global.get $value";
    let mut cases = Vec::new();
    for (name, snapshot) in [
        (
            "state-negative",
            "i32.const -1 i32.const 0 i32.const 0 f64.const 0",
        ),
        (
            "state-large",
            "i32.const 3 i32.const 0 i32.const 0 f64.const 0",
        ),
        (
            "handled-negative",
            "i32.const 1 i32.const -1 i32.const 3 global.get $value",
        ),
        (
            "handled-large",
            "i32.const 1 i32.const 2 i32.const 3 global.get $value",
        ),
        (
            "pending-kind",
            "i32.const 0 i32.const 0 i32.const 1 f64.const 0",
        ),
        (
            "pending-negative-zero",
            "i32.const 0 i32.const 0 i32.const 0 f64.const -0",
        ),
        (
            "fulfilled-promise",
            "i32.const 1 i32.const 0 i32.const 5 f64.const 32",
        ),
        (
            "missing-error-extension",
            "i32.const 1 i32.const 0 i32.const 4 f64.const 32",
        ),
        (
            "unknown-kind",
            "i32.const 2 i32.const 0 i32.const 6 f64.const 32",
        ),
        (
            "boolean-negative-zero",
            "i32.const 2 i32.const 0 i32.const 2 f64.const -0",
        ),
        (
            "bad-nested-promise",
            "i32.const 2 i32.const 0 i32.const 5 f64.const 32.5",
        ),
        ("query-trap", "unreachable"),
    ] {
        cases.push((
            format!("invalid:{name}"),
            protocol_peer(ProtocolPeer {
                root,
                snapshot: Some(snapshot),
                event: None,
                queued: true,
            }),
        ));
    }
    for payload in ["0", "-0", "-16", "nan", "inf", "32.5", "1048576"] {
        cases.push((
            format!("invalid:identity-{payload}"),
            protocol_peer(ProtocolPeer {
                root: &format!("i32.const 3 i32.const 5 f64.const {payload}"),
                snapshot: Some(valid),
                event: None,
                queued: true,
            }),
        ));
    }
    cases.push((
        "invalid:no-optin".into(),
        protocol_peer(ProtocolPeer {
            root,
            snapshot: None,
            event: None,
            queued: true,
        }),
    ));
    cases.push((
        "invalid:legacy-with-getter".into(),
        protocol_peer(ProtocolPeer {
            root,
            snapshot: Some(valid),
            event: None,
            queued: false,
        }),
    ));
    cases.push((
        "invalid:raw-fulfillment".into(),
        protocol_peer(ProtocolPeer {
            root: "i32.const 0 i32.const 5 f64.const 32",
            snapshot: Some(valid),
            event: None,
            queued: true,
        }),
    ));
    let rejection = "global.get $event if(result i32 i32 i32 f64)i32.const 0 i32.const 0 i32.const 0 f64.const 0 else i32.const 1 global.set $event i32.const 1 i32.const 32 i32.const 3 global.get $value end";
    cases.push((
        "invalid:event-fulfilled".into(),
        protocol_peer(ProtocolPeer {
            root,
            snapshot: Some(valid),
            event: Some(rejection),
            queued: true,
        }),
    ));
    let rejected = "i32.const 2 i32.const 0 i32.const 3 global.get $value";
    let wrong_reason = "global.get $event if(result i32 i32 i32 f64)i32.const 0 i32.const 0 i32.const 0 f64.const 0 else i32.const 1 global.set $event i32.const 1 i32.const 32 i32.const 3 f64.const 99 end";
    cases.push((
        "invalid:event-reason".into(),
        protocol_peer(ProtocolPeer {
            root,
            snapshot: Some(rejected),
            event: Some(wrong_reason),
            queued: true,
        }),
    ));
    let wrong_handled = "global.get $event i32.const 1 i32.add global.set $event global.get $event i32.const 1 i32.eq if(result i32 i32 i32 f64)i32.const 1 i32.const 32 i32.const 3 global.get $value else global.get $event i32.const 2 i32.eq if(result i32 i32 i32 f64)i32.const 2 i32.const 32 i32.const 0 f64.const 0 else i32.const 0 i32.const 0 i32.const 0 f64.const 0 end end";
    cases.push((
        "invalid:event-handled".into(),
        protocol_peer(ProtocolPeer {
            root,
            snapshot: Some(rejected),
            event: Some(wrong_handled),
            queued: true,
        }),
    ));
    let replayed = "global.get $event i32.const 1 i32.add global.set $event global.get $event i32.const 2 i32.eq if(result i32 i32 i32 f64)i32.const 2 i32.const 32 i32.const 0 f64.const 0 else global.get $event i32.const 4 i32.lt_u if(result i32 i32 i32 f64)i32.const 1 i32.const 32 i32.const 3 global.get $value else i32.const 0 i32.const 0 i32.const 0 f64.const 0 end end";
    cases.push((
        "invalid:event-replayed".into(),
        protocol_peer(ProtocolPeer {
            root,
            snapshot: Some("i32.const 2 i32.const 1 i32.const 3 global.get $value"),
            event: Some(replayed),
            queued: true,
        }),
    ));
    cases
}

#[test]
fn promise_object_protocol_rejects_malformed_roots_snapshots_and_events() {
    use grunkalunka_runtime::AsyncInstance;
    let cases = invalid_protocol_cases();
    for (name, wasm) in &cases {
        assert!(AsyncInstance::new(wasm, 4.0, 2.0).is_err(), "{name}");
    }
    run_node(&cases);
}

#[test]
fn wasmtime_rejection_reason_preserves_the_guest_promise_identity() {
    use grunkalunka_runtime::{AsyncInstance, AsyncState, Value};
    let run = AsyncInstance::new(&fulfilled_peer(), 4.0, -0.0).unwrap();
    assert!(matches!(
        run.state().unwrap(),
        AsyncState::Rejected(Value::Promise { id: 32 })
    ));
}

#[test]
fn actual_guest_promise_snapshots_validate_records_without_handling_them() {
    let artifact = grunkalunka_compiler::compile_async(
        "export async function run(a,b){Promise.reject(b);return a;}",
        false,
    )
    .unwrap();
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, &artifact.wasm).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let guest = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let start = guest
        .get_typed_func::<(f64, f64), (i32, i32, f64)>(&mut store, "start")
        .unwrap();
    let snapshot = guest
        .get_typed_func::<f64, (i32, i32, i32, f64)>(&mut store, "promise_state")
        .unwrap();
    let events = guest
        .get_typed_func::<(), (i32, i32, i32, f64)>(&mut store, "take_promise_event")
        .unwrap();
    let jobs = guest
        .get_typed_func::<(), i32>(&mut store, "take_jobs")
        .unwrap();
    assert_eq!(start.call(&mut store, (4.0, -0.0)).unwrap(), (0, 3, 4.0));
    let (kind, id, tag, payload) = events.call(&mut store, ()).unwrap();
    assert_eq!((kind, tag), (1, 3));
    assert_eq!(payload.to_bits(), (-0.0f64).to_bits());
    for _ in 0..2 {
        let (state, handled, tag, payload) = snapshot.call(&mut store, f64::from(id)).unwrap();
        assert_eq!((state, handled, tag), (2, 0, 3));
        assert_eq!(payload.to_bits(), (-0.0f64).to_bits());
    }
    assert_eq!(events.call(&mut store, ()).unwrap(), (0, 0, 0, 0.0));
    assert_eq!(jobs.call(&mut store, ()).unwrap(), 0);
    for bad in [
        0.0,
        -0.0,
        -1.0,
        f64::NAN,
        f64::INFINITY,
        1_048_576.0,
        f64::from(id) + 0.5,
        f64::from(id) + 16.0,
    ] {
        assert!(snapshot.call(&mut store, bad).is_err(), "{bad}");
    }
}

#[test]
fn actual_guest_promise_reasons_are_raw_references_and_never_fulfillment_payloads() {
    let mut artifact = grunkalunka_compiler::compile_async(
        "export async function run(a,b){Promise.resolve(a);return b;}",
        false,
    )
    .unwrap();
    let at = artifact.wat.rfind(')').unwrap();
    artifact.wat.insert_str(at,r#"
        (func(export "fixture_pending")(result f64)call $promise_new f64.convert_i32_u)
        (func(export "fixture_completed")(param i32 i32 f64)(result f64)
            local.get 0 local.get 1 local.get 2 call $promise_completed f64.convert_i32_u)
        (func(export "fixture_settle")(param f64 i32 i32 f64)(result i32)
            local.get 0 call $promise_reference local.get 1 local.get 2 local.get 3 call $promise_settle)
    "#);
    let wasm = wat::parse_str(&artifact.wat).unwrap();
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, wasm).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let guest = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let start = guest
        .get_typed_func::<(f64, f64), (i32, i32, f64)>(&mut store, "start")
        .unwrap();
    let new = guest
        .get_typed_func::<(), f64>(&mut store, "fixture_pending")
        .unwrap();
    let completed = guest
        .get_typed_func::<(i32, i32, f64), f64>(&mut store, "fixture_completed")
        .unwrap();
    let settle = guest
        .get_typed_func::<(f64, i32, i32, f64), i32>(&mut store, "fixture_settle")
        .unwrap();
    let snapshot = guest
        .get_typed_func::<f64, (i32, i32, i32, f64)>(&mut store, "promise_state")
        .unwrap();
    let jobs = guest
        .get_typed_func::<(), i32>(&mut store, "take_jobs")
        .unwrap();
    assert_eq!(start.call(&mut store, (4.0, 2.0)).unwrap(), (0, 3, 2.0));
    let p = new.call(&mut store, ()).unwrap();
    assert_eq!(snapshot.call(&mut store, p).unwrap(), (0, 0, 0, 0.0));
    let q = completed.call(&mut store, (1, 5, p)).unwrap();
    assert_ne!(p, q);
    assert_eq!(snapshot.call(&mut store, q).unwrap(), (2, 0, 5, p));
    assert_eq!(snapshot.call(&mut store, p).unwrap(), (0, 0, 0, 0.0));
    assert!(completed.call(&mut store, (0, 5, p)).is_err());
    assert_eq!(settle.call(&mut store, (p, 1, 5, p)).unwrap(), 1);
    assert_eq!(snapshot.call(&mut store, p).unwrap(), (2, 0, 5, p));
    for bad in [
        0.0,
        -0.0,
        -16.0,
        p + 0.5,
        p + 16.0,
        f64::NAN,
        f64::INFINITY,
        1_048_576.0,
    ] {
        assert!(completed.call(&mut store, (1, 5, bad)).is_err(), "{bad}");
    }
    assert_eq!(jobs.call(&mut store, ()).unwrap(), 0);
}

#[test]
fn invalid_public_promise_queries_and_late_traps_terminate_the_activation() {
    use grunkalunka_runtime::AsyncInstance;
    for id in [0, 15, 17, 1_048_576, u32::MAX] {
        let mut run = AsyncInstance::new(&fulfilled_peer(), 4.0, 2.0).unwrap();
        assert!(run.promise_state(id).is_err());
        assert!(run.state().is_err());
        assert!(run.pending_job_count().is_err());
        assert!(run.take_promise_notifications().is_err());
    }
    let mut run = AsyncInstance::new(&object_peer("trap"), 4.0, 2.0).unwrap();
    assert!(run.run_one_job().is_err());
    assert!(run.state().is_err());
    assert!(run.promise_state(32).is_err());
}
