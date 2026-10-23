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

#[test]
fn finite_repeated_remainder_exhausts_fuel_and_instance_can_be_reused() {
    // Finite, deliberately over budget; never run this workload in a JS oracle.
    let artifact = compile(
        "export function run(a,b) { let x=a; for(let i=0;i<b;i++) { x%=3; } return x; }",
        false,
    )
    .unwrap();
    let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
    let error = instance.run(5.0, 1000.0).unwrap_err();
    assert_eq!(error.downcast_ref::<Trap>(), Some(&Trap::OutOfFuel));
    assert_eq!(instance.run(5.0, 1.0).unwrap(), 2.0);
    assert_eq!(instance.run(-4.0, 0.0).unwrap(), -4.0);
    assert_eq!(
        instance.run(-6.0, 1.0).unwrap().to_bits(),
        (-0.0_f64).to_bits()
    );
}
