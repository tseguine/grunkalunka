use super::*;

// The shared native-checkpoint harness executes these JS twins. The test below
// establishes that adding type arguments emits the exact same guest bytes.
pub const SOURCES: &[&str] = &[
    "export async function run(a,b){const p=Promise.reject(b);p;return a;}",
    "export async function run(a,b){const p=Promise.reject(b);await a;try{await p;}catch(e){}return a;}",
];

#[test]
fn erased_promise_type_arguments_preserve_rejection_notifications() {
    for (index, plain) in SOURCES.iter().enumerate() {
        let typed = plain.replace("Promise.reject(", "Promise.reject<never>(");
        let original = compile_async(plain, false).unwrap().wasm;
        let wasm = compile_async(&typed, true).unwrap().wasm;
        assert_eq!(wasm, original);
        for (a, b) in [(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)] {
            let mut run = AsyncInstance::new(&wasm, a, b).unwrap();
            let mut events = run.take_promise_notifications().unwrap();
            let mut jobs = 0;
            while run.run_one_job().unwrap() {
                jobs += 1;
                events.extend(run.take_promise_notifications().unwrap());
            }
            assert_eq!(jobs, index * 2);
            let Some(PromiseNotification::Rejected {
                reason: Value::Number(reason),
                ..
            }) = events.first()
            else {
                panic!("{typed}: {events:?}");
            };
            assert!(reason.is_nan() && b.is_nan() || reason.to_bits() == b.to_bits());
            if index == 0 {
                assert_eq!(events.len(), 1);
            } else {
                assert!(
                    matches!(events.as_slice(), [PromiseNotification::Rejected { id, .. }, PromiseNotification::Handled { id: handled }] if id == handled)
                );
            }
            let AsyncState::Fulfilled(Value::Number(value)) = run.state().unwrap() else {
                panic!("{typed}: root must fulfill");
            };
            assert!(value.is_nan() && a.is_nan() || value.to_bits() == a.to_bits());
        }
    }
}
