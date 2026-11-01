//! Production storage fragments exercise boxed-function tracing and cycles.
use wasmtime::{Collector, Config, Engine, Instance, Module, Store};

#[test]
fn boxed_functions_retain_cyclic_environments_after_defining_calls_return() {
    let cells = include_str!("../../grunkalunka-compiler/src/emit/gc_closures.wat");
    let values = include_str!("../../grunkalunka-compiler/src/emit/managed.wat");
    let wasm = wat::parse_str(format!(
        r#"(module
      {cells}
      (global $saved (mut (ref null $gc_value)) (ref.null $gc_value))
      (func $key (result (ref null $gc_key)) i32.const 102 array.new_fixed $gc_key 1)
      (func (export "run") (param f64 f64) (result f64)
        (local $object (ref null $gc_object)) (local $cell (ref null $gc_cell))
        (local $function (ref null $gc_function))
        call $gc_object_new local.set $object
        i32.const 0 local.get 0
        i32.const 4 f64.const 0 local.get $object struct.new $gc_value
        struct.new $gc_cell local.set $cell
        i32.const 7 local.get $cell array.new_fixed $gc_frame 1
        struct.new $gc_function local.set $function
        i32.const 6 f64.const 0 local.get $function struct.new $gc_value global.set $saved
        ;; Function -> environment -> cell -> object -> the same function.
        local.get $object call $key global.get $saved call $gc_set
        local.get 0)
      (func $function (result (ref null $gc_function))
        global.get $saved struct.get $gc_value 2 ref.cast (ref $gc_function))
      (func $cell (result (ref null $gc_cell))
        call $function struct.get $gc_function 1 i32.const 0 array.get $gc_frame)
      (func (export "replace") (param f64)
        call $cell local.get 0 struct.set $gc_cell 1)
      (func (export "read") (result f64)
        call $function struct.get $gc_function 0 i32.const 7 i32.ne if unreachable end
        global.get $saved call $gc_truthy i32.eqz if unreachable end
        call $cell struct.get $gc_cell 2 ref.cast (ref $gc_value)
        struct.get $gc_value 2 ref.cast (ref $gc_object)
        call $key call $gc_get global.get $saved call $gc_equal
        i32.eqz if unreachable end
        call $cell struct.get $gc_cell 1)
      (func (export "clear") ref.null $gc_value global.set $saved)
      {values})"#
    ))
    .unwrap();
    let mut config = Config::new();
    config.consume_fuel(true).collector(Collector::Copying);
    let engine = Engine::new(&config).unwrap();
    let module = Module::new(&engine, wasm).unwrap();
    let mut store = Store::new(&engine, ());
    store.set_fuel(100_000).unwrap();
    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    let run = instance
        .get_typed_func::<(f64, f64), f64>(&mut store, "run")
        .unwrap();
    let read = instance
        .get_typed_func::<(), f64>(&mut store, "read")
        .unwrap();
    let replace = instance
        .get_typed_func::<f64, ()>(&mut store, "replace")
        .unwrap();
    let clear = instance
        .get_typed_func::<(), ()>(&mut store, "clear")
        .unwrap();
    for expected in [7.0, -0.0, f64::NAN, f64::from_bits(1)] {
        store.set_fuel(100_000).unwrap();
        run.call(&mut store, (17.0, 0.0)).unwrap();
        store.gc(None).unwrap();
        assert_eq!(read.call(&mut store, ()).unwrap(), 17.0);
        replace.call(&mut store, expected).unwrap();
        for _ in 0..4 {
            store.gc(None).unwrap();
            let actual = read.call(&mut store, ()).unwrap();
            if expected.is_nan() {
                assert!(actual.is_nan());
            } else {
                assert_eq!(actual.to_bits(), expected.to_bits());
            }
        }
        clear.call(&mut store, ()).unwrap();
        store.gc(None).unwrap();
        assert!(read.call(&mut store, ()).is_err());
    }
}
