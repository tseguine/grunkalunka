use grunkalunka_compiler::compile;
use grunkalunka_runtime::NumericInstance;
use wasmtime::Trap;

#[test]
fn direct_calls_preserve_argument_snapshots_and_parameter_isolation() {
    let artifact = compile(
        "function pack(x,y) { x=x*10+y; return x; } export function run(a,b) { let x=5; const result=pack(x,x=3); return result*10+x; }",
        false,
    ).unwrap();
    let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
    assert_eq!(instance.run(0.0, 0.0).unwrap(), 533.0);
}

#[test]
fn excessive_finite_recursion_traps_and_the_instance_can_run_again() {
    // Deliberately deep but finite. Never replay this through a JS oracle.
    let artifact = compile(
        "function down(n) { if(n<=0) return 0; return down(n-1)+1; } export function run(a,b) { if(a) return down(100000); return b; }",
        false,
    ).unwrap();
    let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
    let error = instance.run(1.0, 0.0).unwrap_err();
    let trap = error.downcast_ref::<Trap>().unwrap();
    assert!(
        matches!(trap, Trap::OutOfFuel | Trap::StackOverflow),
        "{error}"
    );
    println!("observed excessive-recursion protection: {trap:?}");
    assert_eq!(instance.run(0.0, 42.0).unwrap(), 42.0);
}
