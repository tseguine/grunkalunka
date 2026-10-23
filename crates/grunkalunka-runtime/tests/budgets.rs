//! Independent core-module observations, not a complete component budget policy.
#[cfg(target_has_atomic = "64")]
use wasmtime::{Config, Trap};
use wasmtime::{Engine, Instance, Module, Store, StoreLimitsBuilder};

#[test]
fn per_memory_limit_rejects_growth_that_the_same_module_otherwise_allows() {
    let engine = Engine::default();
    let bytes = wat::parse_str(
        r#"(module
            (memory (export "memory") 1 3)
            (func (export "grow") (param i32) (result i32)
                local.get 0
                memory.grow))"#,
    )
    .unwrap();
    let module = Module::from_binary(&engine, &bytes).unwrap();

    // Positive control: module maximum and growth itself permit a second page.
    let mut unrestricted = Store::new(&engine, ());
    let instance = Instance::new(&mut unrestricted, &module, &[]).unwrap();
    let memory = instance.get_memory(&mut unrestricted, "memory").unwrap();
    let grow = instance
        .get_typed_func::<i32, i32>(&mut unrestricted, "grow")
        .unwrap();
    assert_eq!(memory.size(&unrestricted), 1);
    assert_eq!(grow.call(&mut unrestricted, 1).unwrap(), 1);
    assert_eq!(memory.size(&unrestricted), 2);

    let limits = StoreLimitsBuilder::new().memory_size(65_536).build();
    let mut restricted = Store::new(&engine, limits);
    // Install before instantiation: initial memories and start functions matter.
    restricted.limiter(|limits| limits);
    let instance = Instance::new(&mut restricted, &module, &[]).unwrap();
    let memory = instance.get_memory(&mut restricted, "memory").unwrap();
    let grow = instance
        .get_typed_func::<i32, i32>(&mut restricted, "grow")
        .unwrap();
    assert_eq!(memory.size(&restricted), 1);
    assert_eq!(grow.call(&mut restricted, 0).unwrap(), 1);
    // Standard memory.grow failure returns -1; no trap-on-failure override.
    for _ in 0..2 {
        assert_eq!(grow.call(&mut restricted, 1).unwrap(), -1);
        assert_eq!(memory.size(&restricted), 1);
    }
}

#[test]
#[cfg(target_has_atomic = "64")]
fn expired_epoch_deadline_traps_on_a_subsequent_call_and_can_be_renewed() {
    let mut config = Config::new();
    config.epoch_interruption(true);
    config.consume_fuel(false);
    // Dedicated engine: other tests cannot advance this store's epoch.
    let engine = Engine::new(&config).unwrap();
    let bytes =
        wat::parse_str(r#"(module (func (export "value") (result i32) i32.const 42))"#).unwrap();
    let module = Module::from_binary(&engine, &bytes).unwrap();
    let mut store = Store::new(&engine, ());
    store.epoch_deadline_trap();
    store.set_epoch_deadline(1);
    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    let value = instance
        .get_typed_func::<(), i32>(&mut store, "value")
        .unwrap();

    assert_eq!(value.call(&mut store, ()).unwrap(), 42);
    engine.increment_epoch();
    let error = value.call(&mut store, ()).unwrap_err();
    assert_eq!(error.downcast_ref::<Trap>(), Some(&Trap::Interrupt));
    store.set_epoch_deadline(1);
    assert_eq!(value.call(&mut store, ()).unwrap(), 42);
}
