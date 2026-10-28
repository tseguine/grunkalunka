use super::*;

pub const SOURCES: &[&str] = &[
    "export async function run(a,b){const p=Promise.reject(b),q=Promise.resolve(a);p<=q;return a;}",
    "export async function run(a,b){const f=async()=>{await a;throw b;};const p=f(),q=Promise.resolve(a);(a?p:b)>=q;return a;}",
    "export async function run(a,b){const p=Promise.reject(b),q=Promise.resolve(a);let x=a?p:b;x<=q;await a;try{await p;}catch(e){}return a;}",
];

#[test]
fn relational_operands_leave_rejections_and_jobs_to_explicit_consumers() {
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
                match events.as_slice() {
                    [
                        PromiseNotification::Rejected {
                            reason: Value::Number(x),
                            ..
                        },
                    ] if index < 2 => {
                        assert!(x.is_nan() && b.is_nan() || x.to_bits() == b.to_bits());
                    }
                    [
                        PromiseNotification::Rejected { id, .. },
                        PromiseNotification::Handled { id: handled },
                    ] if index == 2 => assert_eq!(id, handled),
                    _ => panic!("{source}: {events:?}"),
                }
                let AsyncState::Fulfilled(Value::Number(value)) = run.state().unwrap() else {
                    panic!("root outcome: {source}")
                };
                assert!(value.is_nan() && a.is_nan() || value.to_bits() == a.to_bits());
            }
        }
    }
}
