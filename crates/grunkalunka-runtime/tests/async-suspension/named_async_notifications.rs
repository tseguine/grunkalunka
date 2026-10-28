use super::*;

pub const SOURCES: &[&str] = &[
    "async function f(x){throw x;}export async function run(a,b){f(b);return a;}",
    "async function f(x){await x;throw x;}export async function run(a,b){f(b);return a;}",
    "async function f(x){throw x;}export async function run(a,b){const p=f(b);await a;try{await p;}catch(e){}return a;}",
    "async function f(x){await x;throw x;}export async function run(a,b){return f(b);}",
];

#[test]
fn named_async_calls_leave_discarded_rejections_unhandled_until_consumed() {
    for (index, source) in SOURCES.iter().enumerate() {
        for ts in [false, true] {
            let wasm = compile_async(source, ts).unwrap().wasm;
            for (a, b) in [(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)] {
                let mut run = AsyncInstance::new(&wasm, a, b).unwrap();
                let mut events = run.take_promise_notifications().unwrap();
                let mut jobs = 0;
                while run.run_one_job().unwrap() {
                    jobs += 1;
                    events.extend(run.take_promise_notifications().unwrap());
                }
                assert_eq!(jobs, index, "{source}");
                let mut unhandled = Vec::new();
                for event in &events {
                    match event {
                        PromiseNotification::Rejected { id, reason } => {
                            unhandled.push((*id, reason.clone()));
                        }
                        PromiseNotification::Handled { id } => {
                            unhandled.retain(|(rejected, _)| rejected != id);
                        }
                    }
                }
                // Root settlement is observed through state(), independently of
                // child rejection notifications. Both await and adoption handle
                // the named helper's rejected child identity.
                if index >= 2 {
                    assert!(
                        matches!(events.as_slice(), [PromiseNotification::Rejected { id, .. }, PromiseNotification::Handled { id: handled }] if id == handled)
                    );
                    assert!(unhandled.is_empty());
                } else {
                    let [(_, Value::Number(reason))] = unhandled.as_slice() else {
                        panic!("{source}: {events:?}");
                    };
                    assert!(reason.is_nan() && b.is_nan() || reason.to_bits() == b.to_bits());
                }
                let expected = if index == 3 {
                    let AsyncState::Rejected(value) = run.state().unwrap() else {
                        panic!("adopted rejection: {source}")
                    };
                    (value, b)
                } else {
                    let AsyncState::Fulfilled(value) = run.state().unwrap() else {
                        panic!("discarded or caught rejection: {source}")
                    };
                    (value, a)
                };
                let (Value::Number(actual), expected) = expected else {
                    panic!("numeric fixture outcome: {source}")
                };
                assert!(
                    actual.is_nan() && expected.is_nan() || actual.to_bits() == expected.to_bits()
                );
            }
        }
    }
}
