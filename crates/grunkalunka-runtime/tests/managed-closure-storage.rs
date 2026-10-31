//! Shared GC storage-fragment lifetime tests, not source capture admission.
use wasmtime::{Collector, Config, Engine, Instance, Module, Store};

#[test]
fn traced_environments_keep_shared_cells_cycles_and_snapshots_across_collection() {
    let cells = include_str!("../../grunkalunka-compiler/src/emit/gc_closures.wat");
    let values = include_str!("../../grunkalunka-compiler/src/emit/managed.wat");
    let wasm = wat::parse_str(format!(
        r#"(module
      {cells}
      (global $saved (mut (ref null $gc_frame)) (ref.null $gc_frame))
      (global $alias (mut (ref null $gc_frame)) (ref.null $gc_frame))
      (global $snapshot (mut (ref null $gc_value)) (ref.null $gc_value))
      (func $key (result (ref null $gc_key)) i32.const 120 array.new_fixed $gc_key 1)
      (func (export "run") (param f64 f64) (result f64)
        (local $cell (ref null $gc_cell)) (local $object (ref null $gc_object))
        call $gc_object_new local.set $object
        local.get $object call $key
        i32.const 5 f64.const 0 i32.const 0 i32.const 55296
        array.new_fixed $gc_key 2 struct.new $gc_value call $gc_set
        ;; Numeric lanes remain unboxed, alongside one traced value edge.
        i32.const 7 local.get 0
        i32.const 4 f64.const 0 local.get $object struct.new $gc_value
        struct.new $gc_cell local.set $cell
        local.get $cell struct.get $gc_cell 2 ref.cast (ref $gc_value) global.set $snapshot
        local.get $cell array.new_fixed $gc_frame 1 global.set $alias
        local.get $cell
        ;; A second cell carries the environment itself, making a cycle.
        struct.new_default $gc_cell
        array.new_fixed $gc_frame 2 global.set $saved
        global.get $saved i32.const 1 array.get $gc_frame
        global.get $saved struct.set $gc_cell 2
        local.get 0)
      (func (export "replace") (param f64)
        global.get $alias i32.const 0 array.get $gc_frame
        i32.const 3 local.get 0 ref.null eq struct.new $gc_value struct.set $gc_cell 2)
      (func (export "read") (result f64)
        global.get $saved i32.const 0 array.get $gc_frame
        global.get $alias i32.const 0 array.get $gc_frame ref.eq i32.eqz if unreachable end
        global.get $saved i32.const 1 array.get $gc_frame struct.get $gc_cell 2
        global.get $saved ref.eq i32.eqz if unreachable end
        global.get $saved i32.const 0 array.get $gc_frame struct.get $gc_cell 0
        i32.const 7 i32.ne if unreachable end
        global.get $saved i32.const 0 array.get $gc_frame struct.get $gc_cell 1
        f64.const 17 f64.ne if unreachable end
        ;; The old immutable carrier/object/String snapshot survives replacement.
        global.get $snapshot struct.get $gc_value 2 ref.cast (ref $gc_object)
        call $key call $gc_get struct.get $gc_value 2 ref.cast (ref $gc_key)
        i32.const 1 array.get_u $gc_key i32.const 55296 i32.ne if unreachable end
        global.get $alias i32.const 0 array.get $gc_frame struct.get $gc_cell 2
        ref.cast (ref $gc_value) struct.get $gc_value 1)
      (func (export "clear")
        ref.null $gc_frame global.set $saved
        ref.null $gc_frame global.set $alias
        ref.null $gc_value global.set $snapshot)
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
        for _ in 0..3 {
            store.gc(None).unwrap();
        }
        replace.call(&mut store, expected).unwrap();
        for _ in 0..3 {
            store.gc(None).unwrap();
            let got = read.call(&mut store, ()).unwrap();
            if expected.is_nan() {
                assert!(got.is_nan());
            } else {
                assert_eq!(got.to_bits(), expected.to_bits());
            }
        }
        clear.call(&mut store, ()).unwrap();
        store.gc(None).unwrap();
    }
}
