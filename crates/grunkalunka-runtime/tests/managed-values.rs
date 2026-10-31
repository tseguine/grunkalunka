use grunkalunka_runtime::NumericInstance;
use wasmtime::{Collector, Config, Engine, Instance, Module, Store};

fn wasm() -> Vec<u8> {
    let kernel = include_str!("../../grunkalunka-compiler/src/emit/managed.wat");
    wat::parse_str(format!(
        r#"(module
      (global $saved (mut (ref null $gc_object)) (ref.null $gc_object))
      (func $key (result (ref null $gc_key)) i32.const 120 array.new_fixed $gc_key 1)
      (func $self (result (ref null $gc_key))
        i32.const 115 i32.const 101 i32.const 108 i32.const 102 array.new_fixed $gc_key 4)
      (func (export "run") (param f64 f64) (result f64)
        (local $object (ref null $gc_object)) (local $old (ref null $gc_value))
        (local $a (ref null $gc_value)) (local $b (ref null $gc_value))
        call $gc_object_new local.tee $object global.set $saved
        i32.const 3 local.get 0 ref.null eq struct.new $gc_value local.set $a
        i32.const 3 local.get 1 ref.null eq struct.new $gc_value local.set $b
        local.get $object call $key local.get $a call $gc_set
        local.get $object call $key call $gc_get local.set $old
        local.get $object call $self
        i32.const 4 f64.const 0 local.get $object struct.new $gc_value call $gc_set
        local.get $object call $key local.get $b call $gc_set
        local.get $old local.get $a call $gc_equal f64.convert_i32_u
        local.get $object call $key call $gc_get local.get $b call $gc_equal
        f64.convert_i32_u f64.sub)
      (func (export "read") (result f64)
        global.get $saved call $self call $gc_get
        i32.const 4 f64.const 0 global.get $saved struct.new $gc_value
        call $gc_equal i32.eqz if unreachable end
        global.get $saved call $key call $gc_get struct.get $gc_value 1)
      {kernel})"#
    ))
    .unwrap()
}

#[test]
fn managed_kernel_preserves_number_snapshots_in_wasmtime() {
    let mut instance = NumericInstance::new(&wasm()).unwrap();
    let inputs = [
        0.0,
        -0.0,
        1.0,
        -7.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        f64::from_bits(1),
    ];
    for a in inputs {
        for b in inputs {
            let expected = f64::from(u8::from(!a.is_nan())) - f64::from(u8::from(!b.is_nan()));
            assert_eq!(instance.run(a, b).unwrap().to_bits(), expected.to_bits());
        }
    }
}

#[test]
fn managed_property_cycles_and_values_survive_copying_collection() {
    let mut config = Config::new();
    config.consume_fuel(true).collector(Collector::Copying);
    let engine = Engine::new(&config).unwrap();
    let module = Module::from_binary(&engine, &wasm()).unwrap();
    let mut store = Store::new(&engine, ());
    store.set_fuel(10_000).unwrap();
    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    let run = instance
        .get_typed_func::<(f64, f64), f64>(&mut store, "run")
        .unwrap();
    let read = instance
        .get_typed_func::<(), f64>(&mut store, "read")
        .unwrap();
    for expected in [1.0_f64, -0.0, f64::NAN, f64::from_bits(1)] {
        store.set_fuel(10_000).unwrap();
        run.call(&mut store, (7.0, expected)).unwrap();
        for _ in 0..3 {
            store.gc(None).unwrap();
            let actual = read.call(&mut store, ()).unwrap();
            if expected.is_nan() {
                assert!(actual.is_nan());
            } else {
                assert_eq!(actual.to_bits(), expected.to_bits());
            }
        }
    }
}
