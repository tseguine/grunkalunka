use grunkalunka_compiler::compile_async;
use grunkalunka_runtime::{AsyncInstance, AsyncState, PromiseNotification, Value};
use std::{
    io::Write,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[path = "async-suspension/promoted_identities.rs"]
mod promoted_identities;

#[path = "async-suspension/named_async_notifications.rs"]
mod named_async_notifications;

#[path = "async-suspension/named_promise_notifications.rs"]
mod named_promise_notifications;

#[path = "async-suspension/promise_type_arguments_notifications.rs"]
mod promise_type_arguments_notifications;

#[path = "async-suspension/promise_relational_notifications.rs"]
mod promise_relational_notifications;

#[path = "async-suspension/promise_resolve_notifications.rs"]
mod promise_resolve_notifications;

fn promoted_identity_case(source: &str, typescript: bool) -> String {
    fn bits(value: Value) -> String {
        match value {
            Value::Undefined => "undefined".into(),
            Value::Null => "null".into(),
            Value::Boolean(value) => value.to_string(),
            Value::Number(value) if value.is_nan() => "nan".into(),
            Value::Number(value) => format!("{:016x}", value.to_bits()),
            Value::TypeError { .. } | Value::Promise { .. } => {
                panic!("promoted primitive fixtures must not create TypeErrors")
            }
        }
    }
    let wasm = compile_async(source, typescript).unwrap().wasm;
    let mut rows = Vec::new();
    for (a, b) in [
        (4.0, 2.0),
        (4.0, -0.0),
        (4.0, f64::NAN),
        (0.0, f64::INFINITY),
        (0.0, -0.0),
    ] {
        let mut run = AsyncInstance::new(&wasm, a, b).unwrap();
        let mut events = run.take_promise_notifications().unwrap();
        while run.run_one_job().unwrap() {
            events.extend(run.take_promise_notifications().unwrap());
        }
        let mut unhandled = Vec::new();
        for event in events {
            match event {
                PromiseNotification::Rejected { id, reason } => unhandled.push((id, bits(reason))),
                PromiseNotification::Handled { id } => unhandled.retain(|(p, _)| *p != id),
            }
        }
        let (status, value) = match run.state().unwrap() {
            AsyncState::Fulfilled(value) => ("fulfilled", value),
            AsyncState::Rejected(value) => ("rejected", value),
            AsyncState::Pending => panic!("drained promoted fixture is still pending: {source}"),
        };
        rows.push(format!(
            "{{\"value\":[{status:?},{:?}],\"unhandled\":[{}]}}",
            bits(value),
            unhandled
                .iter()
                .map(|(_, reason)| format!("{reason:?}"))
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    format!(
        "{{\"source\":{source:?},\"wasm\":{wasm:?},\"rust\":[{}]}}",
        rows.join(",")
    )
}

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
    "const f=async()=>{await a;throw b;};const use=async p=>await p;use(f());return a;",
    "const f=async()=>{await a;throw b;};const p=f();const use=async p=>p;use(p);return a;",
    "Promise.reject(b);return a;",
    "const f=async()=>{throw b;};f();return a;",
    "const p=Promise.reject(b);const q=Promise.resolve(p);return p===q?a:a;",
    "const p=Promise.reject(b),q=Promise.reject(b);return p!==q?a:a;",
    "const p=Promise.reject(b);await a;try{await p;}catch(e){}return a;",
    "const p=Promise.reject(b);return p&&a;",
    "const f=async()=>{await a;throw b;};const p=f();return p&&a;",
    "const p=Promise.reject(b);const f=async()=>p||a;try{await f();}catch(e){}return a;",
    "const p=Promise.reject(b);return (a?p:0)&&a;",
    "const p=Promise.reject(b);const f=async()=>(a?p:0)||a;try{await f();}catch(e){}return a;",
    "const apply=(fn,x)=>fn(x);let f=async x=>{await x;throw b;};apply(f,a);return a;",
    "const apply=(fn,x)=>fn(x);let f=async x=>{await x;throw b;};const p=apply(f,a);await a;await b;try{await p;}catch(e){}return a;",
    "const p=Promise.reject(b);return (a?p:0)==p?a:a;",
    "const f=async()=>{await a;throw b;};const p=f();return (a?p:0)===p?a:a;",
    "const p=Promise.reject(b);const x=a?p:0;const y=x;y;return x==p?a:a;",
    "const f=async()=>{await a;throw b;};const p=f();const x=a?p:0;return x===p?a:a;",
    "const p=Promise.reject(b);const x=a?p:0;const read=async()=>{await a;try{await x;}catch(e){}return a;};return await read();",
    "const p=Promise.reject(b);let x=p;const y=x;x=a;x??=p;return y===p?a:a;",
    "const f=async()=>{await a;throw b;};const p=f();let x=p;const y=x;x=a;x&&=b;return y===p?a:a;",
    "let x=Promise.reject(b);const p=x;x=a;await a;x=p;try{await x;}catch(e){}return a;",
    "const p=Promise.reject(b);const same=x=>x===p;same(p);same(a);return a;",
    "const f=async()=>{await a;throw b;};const p=f();let same=a?x=>x===p:x=>x!==p;same(p);same(a);return a;",
    "const p=Promise.reject(b);const use=async x=>await x;await use(a);try{await use(p);}catch(e){}return a;",
    "const p=Promise.reject(b);const id=x=>x;id(p);id(a);return a;",
    "const f=async()=>{await a;throw b;};const p=f();let read=a?()=>p:()=>b;read();return a;",
    "const p=Promise.reject(b);const id=x=>x;id(p);id(a);await a;try{await id(p);}catch(e){}return a;",
    "const p=Promise.reject(b);try{return p;}finally{return a;}",
    "const p=Promise.reject(b);const f=async()=>{try{return p;}finally{await a;}};try{await f();}catch(e){}return a;",
    "const p=Promise.reject(b);const f=async()=>{try{return p;}finally{return a;}};await f();try{await p;}catch(e){}return a;",
    "const p=Promise.reject(b);+p;return a;",
    "const f=async()=>{await a;throw b;};const p=f();Math.abs(a?p:b);return a;",
    "const p=Promise.reject(b);let x=a?p:b;+x;await a;try{await p;}catch(e){}return a;",
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
                if [0, 18, 19, 21, 25, 26, 27, 28, 30, 31, 33, 34].contains(&index)
                    || (13..=16).contains(&index)
                {
                    assert!(matches!(
                        run.state().unwrap(),
                        AsyncState::Fulfilled(Value::Number(4.0))
                    ));
                    assert_eq!(
                        run.pending_job_count().unwrap(),
                        usize::from([0, 19, 26, 28, 31, 34].contains(&index))
                    );
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
                if [5, 6, 7, 17, 20, 22, 24, 29, 32, 35, 38, 40, 41, 44].contains(&index) {
                    assert!(unhandled.is_empty(), "{body}");
                    if [6, 7, 17, 20, 22, 24, 29, 32, 35, 38, 40, 41, 44].contains(&index) {
                        assert_eq!(events.len(), 2);
                    } else {
                        assert!(events.is_empty());
                    }
                } else {
                    assert_eq!(
                        unhandled.len(),
                        if [8, 16].contains(&index) { 2 } else { 1 },
                        "{body}"
                    );
                    assert_eq!(events.len(), if index == 12 { 3 } else { unhandled.len() });
                    let first = unhandled.values().next().unwrap();
                    match (index, *first) {
                        (1, Value::Undefined)
                        | (2, Value::Null)
                        | (3, Value::Boolean(false))
                        | (4, Value::Boolean(true)) => {}
                        (
                            0 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 18 | 19 | 21 | 23 | 25
                            | 26 | 27 | 28 | 30 | 31 | 33 | 34 | 36 | 37 | 39 | 42 | 43,
                            Value::Number(x),
                        ) => {
                            assert!(
                                x.is_nan() && reason.is_nan() || x.to_bits() == reason.to_bits()
                            )
                        }
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
    for source in promoted_identities::SOURCES
        .iter()
        .chain(promise_relational_notifications::SOURCES)
        .chain(named_async_notifications::SOURCES)
        .chain(named_promise_notifications::SOURCES)
        .chain(promise_type_arguments_notifications::SOURCES)
        .chain(promise_resolve_notifications::SOURCES)
    {
        for typescript in [false, true] {
            cases.push(promoted_identity_case(source, typescript));
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
