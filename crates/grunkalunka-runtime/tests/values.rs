use grunkalunka_runtime::{Value, ValueInstance};

fn module(body: &str) -> Vec<u8> {
    wat::parse_str(format!(
        "(module (func (export \"run\") (param f64 f64) (result i32 f64) {body}))"
    ))
    .unwrap()
}

#[test]
fn undefined_and_null_have_distinct_kinds() {
    let mut undefined = ValueInstance::new(&module("i32.const 0 f64.const 0")).unwrap();
    let mut null = ValueInstance::new(&module("i32.const 1 f64.const 0")).unwrap();
    assert!(matches!(undefined.run(0.0, 0.0).unwrap(), Value::Undefined));
    assert!(matches!(null.run(0.0, 0.0).unwrap(), Value::Null));
}

#[test]
fn booleans_and_numbers_preserve_kind_and_number_bits() {
    let mut boolean = ValueInstance::new(&module("i32.const 2 local.get 0")).unwrap();
    assert!(matches!(
        boolean.run(0.0, 0.0).unwrap(),
        Value::Boolean(false)
    ));
    assert!(matches!(
        boolean.run(1.0, 0.0).unwrap(),
        Value::Boolean(true)
    ));
    let mut number = ValueInstance::new(&module("i32.const 3 local.get 0")).unwrap();
    for input in [
        0.0,
        -0.0,
        1.0,
        -1.0,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ] {
        let Value::Number(actual) = number.run(input, 0.0).unwrap() else {
            panic!("number kind lost");
        };
        if input.is_nan() {
            assert!(actual.is_nan());
        } else {
            assert_eq!(actual.to_bits(), input.to_bits());
        }
    }
}

#[test]
fn malformed_tags_and_noncanonical_payloads_are_errors() {
    for tag in [-1, 4, i32::MAX] {
        let mut instance =
            ValueInstance::new(&module(&format!("i32.const {tag} f64.const 0"))).unwrap();
        assert!(
            instance
                .run(0.0, 0.0)
                .unwrap_err()
                .to_string()
                .contains("invalid primitive value")
        );
    }
    for tag in [0, 1, 2] {
        let mut instance =
            ValueInstance::new(&module(&format!("i32.const {tag} local.get 0"))).unwrap();
        for payload in [-0.0, -1.0, 0.5, 2.0, f64::NAN, f64::INFINITY] {
            assert!(
                instance.run(payload, 0.0).is_err(),
                "tag {tag}, payload {payload:?}"
            );
        }
        assert!(
            instance.run(0.0, 0.0).is_ok(),
            "decode error must not invalidate the instance"
        );
    }
}

#[test]
fn primitive_fuel_exhaustion_and_recovery_are_observed() {
    let wasm =
        module("local.get 0 f64.const 0 f64.ne if loop br 0 end end i32.const 3 local.get 1");
    let mut instance = ValueInstance::new(&wasm).unwrap();
    let error = instance.run(1.0, 0.0).unwrap_err();
    assert_eq!(
        error.downcast_ref::<wasmtime::Trap>(),
        Some(&wasmtime::Trap::OutOfFuel)
    );
    assert!(matches!(
        instance.run(0.0, 42.0).unwrap(),
        Value::Number(42.0)
    ));
    let error = instance.run(1.0, 0.0).unwrap_err();
    assert_eq!(
        error.downcast_ref::<wasmtime::Trap>(),
        Some(&wasmtime::Trap::OutOfFuel)
    );
}

#[test]
fn finite_primitive_calls_each_receive_a_fresh_budget() {
    let wasm = module(
        "(local i32) i32.const 1000 local.set 2 loop local.get 2 i32.const 1 i32.sub local.tee 2 br_if 0 end i32.const 1 f64.const 0",
    );
    let mut instance = ValueInstance::new(&wasm).unwrap();
    for _ in 0..10 {
        assert!(matches!(instance.run(0.0, 0.0).unwrap(), Value::Null));
    }
}

#[test]
fn even_unused_imports_are_denied_before_instantiation() {
    let wasm=wat::parse_str("(module (import \"host\" \"read\" (func (result i32))) (func (export \"run\") (param f64 f64) (result i32 f64) i32.const 0 f64.const 0))").unwrap();
    let error = ValueInstance::new(&wasm)
        .err()
        .expect("must reject unused import");
    assert!(error.to_string().contains("admits no module imports"));
}

#[test]
fn numeric_and_primitive_signatures_stay_distinct() {
    let numeric =
        wat::parse_str("(module (func (export \"run\") (param f64 f64) (result f64) local.get 0))")
            .unwrap();
    assert!(ValueInstance::new(&numeric).is_err());
    let mut instance = grunkalunka_runtime::NumericInstance::new(&numeric).unwrap();
    assert_eq!(instance.run(7.0, 0.0).unwrap(), 7.0);
    assert!(grunkalunka_runtime::NumericInstance::new(&module("i32.const 0 f64.const 0")).is_err());
    let wrong_arity = wat::parse_str(
        "(module (func (export \"run\") (param f64) (result i32 f64) i32.const 0 f64.const 0))",
    )
    .unwrap();
    assert!(ValueInstance::new(&wrong_arity).is_err());
}
