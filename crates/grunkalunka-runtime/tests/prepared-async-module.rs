use grunkalunka_compiler::compile_async;
use grunkalunka_runtime::{AsyncInstance, AsyncState, PreparedAsyncModule, Value};

const SOURCE: &str = "export async function run(a,b){return a+await Promise.resolve(b);}";

fn finish(mut instance: AsyncInstance) -> AsyncState {
    for _ in 0..32 {
        if !instance.run_one_job().unwrap() {
            return instance.state().unwrap();
        }
    }
    panic!("async activation did not drain within its 32-job budget");
}

fn assert_number(state: AsyncState, expected: f64) {
    let AsyncState::Fulfilled(Value::Number(result)) = state else {
        panic!("expected a fulfilled number");
    };
    if expected.is_nan() {
        assert!(result.is_nan());
    } else {
        assert_eq!(result.to_bits(), expected.to_bits());
    }
}

#[test]
fn prepared_async_module_preserves_fresh_store_and_activation_isolation() {
    let wasm = compile_async(SOURCE, false).unwrap().wasm;
    let prepared = PreparedAsyncModule::new(&wasm).unwrap();

    // Begin two invocations before advancing either continuation.
    let first = prepared.instantiate(4.0, 2.0).unwrap();
    let second = prepared.instantiate(-0.0, -0.0).unwrap();
    assert!(matches!(first.state().unwrap(), AsyncState::Pending));
    assert!(matches!(second.state().unwrap(), AsyncState::Pending));

    // The second activation must not consume the first activation's queued job.
    assert_number(finish(second), -0.0);
    assert!(matches!(first.state().unwrap(), AsyncState::Pending));
    assert_number(finish(first), 6.0);

    // A later invocation starts clean even after its siblings have settled.
    assert_number(
        finish(prepared.instantiate(f64::NAN, 2.0).unwrap()),
        f64::NAN,
    );
    // Existing single-invocation API remains behaviorally equivalent.
    assert_number(finish(AsyncInstance::new(&wasm, 4.0, 2.0).unwrap()), 6.0);
}

#[test]
fn prepared_module_preserves_import_rejection_and_validates_input() {
    let wasm = wat::parse_str("(module (import \"env\" \"read\" (func)))").unwrap();
    assert!(
        PreparedAsyncModule::new(&wasm)
            .err()
            .expect("import-bearing module must fail")
            .to_string()
            .contains("no module imports")
    );
    assert!(PreparedAsyncModule::new(b"not wasm").is_err());
}
