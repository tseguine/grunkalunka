use grunkalunka_runtime::NumericInstance;
use wasmtime::{Collector, Config, Engine, Instance, Module, Store};

#[test]
fn sparse_array_edges_holes_and_identity_survive_copying_collection() {
    let kernel = include_str!("../../grunkalunka-compiler/src/emit/managed.wat");
    let wasm = wat::parse_str(format!(
        r#"(module
      (global $saved (mut (ref null $gc_array)) (ref.null $gc_array))
      (func $key (param i32) (result (ref null $gc_key))
        local.get 0 array.new_fixed $gc_key 1)
      (func $text (result (ref null $gc_value))
        i32.const 5 f64.const 0 i32.const 55296 i32.const 0
        array.new_fixed $gc_key 2 struct.new $gc_value)
      (func (export "run") (param f64 f64) (result f64)
        (local $array (ref null $gc_array)) (local $old (ref null $gc_value))
        i32.const 10 call $gc_array_new local.tee $array global.set $saved
        local.get $array i32.const 48 call $key
        i32.const 3 local.get 0 ref.null eq struct.new $gc_value call $gc_array_set
        local.get $array i32.const 48 call $key call $gc_array_get local.set $old
        local.get $array i32.const 48 call $key
        i32.const 3 local.get 1 ref.null eq struct.new $gc_value call $gc_array_set
        ;; Explicit Undefined at index 2, a hole at index 1, and a cycle at 3.
        local.get $array i32.const 50 call $key
        i32.const 0 f64.const 0 ref.null eq struct.new $gc_value call $gc_array_set
        local.get $array i32.const 51 call $key
        i32.const 4 f64.const 0 local.get $array struct.new $gc_value call $gc_array_set
        local.get $array i32.const 120 call $key call $text call $gc_array_set
        local.get $old struct.get $gc_value 1)
      (func (export "shape") (result i32)
        global.get $saved struct.get $gc_array 1 struct.get $gc_object 0 array.len)
      (func (export "read") (result f64)
        ;; Verify the cycle and text edge after actual collections.
        global.get $saved i32.const 51 call $key call $gc_array_get
        i32.const 4 f64.const 0 global.get $saved struct.new $gc_value
        call $gc_equal i32.eqz if unreachable end
        global.get $saved i32.const 120 call $key call $gc_array_get
        call $text call $gc_equal i32.eqz if unreachable end
        global.get $saved i32.const 49 call $key call $gc_array_get
        struct.get $gc_value 0 i32.eqz i32.eqz if unreachable end
        global.get $saved i32.const 48 call $key call $gc_array_get struct.get $gc_value 1)
      (func (export "shrink")
        global.get $saved i32.const 1 call $gc_array_resize
        global.get $saved i32.const 10 call $gc_array_resize
        global.get $saved i32.const 51 call $key call $gc_array_get
        struct.get $gc_value 0 i32.eqz i32.eqz if unreachable end
        global.get $saved i32.const 120 call $key call $gc_array_get
        call $text call $gc_equal i32.eqz if unreachable end)
      (func (export "clear")
        global.get $saved i32.const 0 call $gc_array_resize
        global.get $saved i32.const -1 call $gc_array_resize)
      {kernel})"#
    ))
    .unwrap();
    let mut config = Config::new();
    config.consume_fuel(true).collector(Collector::Copying);
    let engine = Engine::new(&config).unwrap();
    let module = Module::from_binary(&engine, &wasm).unwrap();
    let mut store = Store::new(&engine, ());
    store.set_fuel(10_000).unwrap();
    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    let run = instance
        .get_typed_func::<(f64, f64), f64>(&mut store, "run")
        .unwrap();
    let read = instance
        .get_typed_func::<(), f64>(&mut store, "read")
        .unwrap();
    let shape = instance
        .get_typed_func::<(), i32>(&mut store, "shape")
        .unwrap();
    let shrink = instance
        .get_typed_func::<(), ()>(&mut store, "shrink")
        .unwrap();
    let clear = instance
        .get_typed_func::<(), ()>(&mut store, "clear")
        .unwrap();
    for expected in [1.0_f64, -0.0, f64::NAN, f64::from_bits(1)] {
        store.set_fuel(10_000).unwrap();
        assert_eq!(run.call(&mut store, (7.0, expected)).unwrap(), 7.0);
        assert_eq!(shape.call(&mut store, ()).unwrap(), 4);
        for _ in 0..3 {
            store.gc(None).unwrap();
            store.set_fuel(10_000).unwrap();
            let actual = read.call(&mut store, ()).unwrap();
            if expected.is_nan() {
                assert!(actual.is_nan());
            } else {
                assert_eq!(actual.to_bits(), expected.to_bits());
            }
        }
        shrink.call(&mut store, ()).unwrap();
        store.gc(None).unwrap();
        assert_eq!(shape.call(&mut store, ()).unwrap(), 2);
        clear.call(&mut store, ()).unwrap();
        store.gc(None).unwrap();
        assert_eq!(shape.call(&mut store, ()).unwrap(), 1);
    }
}

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

#[test]
fn string_content_and_object_edges_survive_copying_collection() {
    let kernel = include_str!("../../grunkalunka-compiler/src/emit/managed.wat");
    let wasm = wat::parse_str(format!(
        r#"(module
      (global $saved (mut (ref null $gc_object)) (ref.null $gc_object))
      (func $text (result (ref null $gc_value))
        i32.const 5 f64.const 0 i32.const 55296 i32.const 0 i32.const 97
        array.new_fixed $gc_key 3 struct.new $gc_value)
      (func $key (result (ref null $gc_key)) i32.const 120 array.new_fixed $gc_key 1)
      (func (export "init")
        call $gc_object_new global.set $saved
        global.get $saved call $key call $text call $gc_set)
      (func (export "read") (result i32)
        global.get $saved call $key call $gc_get call $text call $gc_equal
        global.get $saved call $key call $gc_get call $gc_truthy i32.and)
      {kernel})"#
    ))
    .unwrap();
    let mut config = Config::new();
    config.consume_fuel(true).collector(Collector::Copying);
    let engine = Engine::new(&config).unwrap();
    let module = Module::from_binary(&engine, &wasm).unwrap();
    let mut store = Store::new(&engine, ());
    store.set_fuel(10_000).unwrap();
    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    let init = instance
        .get_typed_func::<(), ()>(&mut store, "init")
        .unwrap();
    let read = instance
        .get_typed_func::<(), i32>(&mut store, "read")
        .unwrap();
    init.call(&mut store, ()).unwrap();
    for _ in 0..6 {
        store.gc(None).unwrap();
        store.set_fuel(10_000).unwrap();
        assert_eq!(read.call(&mut store, ()).unwrap(), 1);
    }
}
