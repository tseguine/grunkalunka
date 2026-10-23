use grunkalunka_compiler::compile;
use grunkalunka_runtime::NumericInstance;
use wasmtime::Trap;

#[test]
fn fuel_exhaustion_is_distinct_and_the_same_instance_can_run_again() {
    // Deliberately infinite for a=1. This fixture never executes in a JS engine.
    let artifact = compile(
        "export function run(a,b) { while (a) { a=a; } return b; }",
        false,
    )
    .expect("numeric while must compile");
    let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
    let error = instance.run(1.0, 0.0).unwrap_err();
    assert_eq!(error.downcast_ref::<Trap>(), Some(&Trap::OutOfFuel));
    assert_eq!(instance.run(0.0, 42.0).unwrap(), 42.0);
    let error = instance.run(1.0, 0.0).unwrap_err();
    assert_eq!(error.downcast_ref::<Trap>(), Some(&Trap::OutOfFuel));
}
