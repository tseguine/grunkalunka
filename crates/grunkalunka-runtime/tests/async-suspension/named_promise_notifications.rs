use super::*;

pub const SOURCES: &[&str] = &[
    "async function f(p){return 0;}export async function run(a,b){const p=Promise.reject(b);f(p);return a;}",
    "async function f(p){return 0;}export async function run(a,b){const p=Promise.reject(b);await f(p);return a;}",
    "async function f(p){return await p;}export async function run(a,b){f(Promise.reject(b));return a;}",
    "async function f(p){return p;}export async function run(a,b){f(Promise.reject(b));return a;}",
    "async function f(p,n){return await p;}export async function run(a,b){const p=Promise.reject(b);try{await f(p,await Promise.reject(a));}catch(e){}return a;}",
    "async function f(p){return await p;}export async function run(a,b){const p=Promise.reject(b);await a;try{await f(p);}catch(e){}return a;}",
    "async function f(p){return p;}export async function run(a,b){const p=Promise.reject(b);try{await f(p);}catch(e){}return a;}",
];

#[test]
fn named_promise_input_transport_does_not_handle_or_adopt_the_argument() {
    for (index, source) in SOURCES.iter().enumerate() {
        for ts in [false, true] {
            let wasm = compile_async(source, ts).unwrap().wasm;
            for (a, b) in [(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)] {
                let mut run = AsyncInstance::new(&wasm, a, b).unwrap();
                let mut events = run.take_promise_notifications().unwrap();
                while run.run_one_job().unwrap() {
                    events.extend(run.take_promise_notifications().unwrap());
                }
                let mut unhandled = std::collections::BTreeMap::new();
                for event in events {
                    match event {
                        PromiseNotification::Rejected { id, reason } => {
                            assert!(unhandled.insert(id, reason).is_none());
                        }
                        PromiseNotification::Handled { id } => {
                            assert!(unhandled.remove(&id).is_some());
                        }
                    }
                }
                assert_eq!(unhandled.len(), usize::from(index < 5), "{source}");
                for reason in unhandled.values() {
                    let Value::Number(reason) = *reason else {
                        panic!("{source}: {reason:?}")
                    };
                    assert!(reason.is_nan() && b.is_nan() || reason.to_bits() == b.to_bits());
                }
                let AsyncState::Fulfilled(Value::Number(actual)) = run.state().unwrap() else {
                    panic!("{source}: root must fulfill")
                };
                assert!(actual.is_nan() && a.is_nan() || actual.to_bits() == a.to_bits());
            }
        }
    }
}
