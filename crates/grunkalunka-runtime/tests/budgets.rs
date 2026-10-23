//! Independent core-module observations, not a complete component budget policy.
#[cfg(target_has_atomic = "64")]
use wasmtime::{Caller, Config, Linker, Trap};
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

#[test]
#[cfg(target_has_atomic = "64")]
fn host_triggered_epoch_expiry_interrupts_an_active_finite_guest_invocation() {
    struct TickState {
        enabled: bool,
        calls: usize,
    }

    let mut config = Config::new();
    config.epoch_interruption(true);
    config.consume_fuel(false);
    let engine = Engine::new(&config).unwrap();
    let bytes = wat::parse_str(include_str!("fixtures/epoch-during-call.wat")).unwrap();
    let module = Module::from_binary(&engine, &bytes).unwrap();
    let mut linker = Linker::new(&engine);
    let tick_engine = engine.clone();
    linker
        .func_wrap("host", "tick", move |mut caller: Caller<'_, TickState>| {
            caller.data_mut().calls += 1;
            if caller.data().enabled {
                tick_engine.increment_epoch();
            }
        })
        .unwrap();

    // Identical module, fresh instance for each outcome. Both guests are finite.
    for enabled in [false, true] {
        let mut store = Store::new(&engine, TickState { enabled, calls: 0 });
        store.epoch_deadline_trap();
        store.set_epoch_deadline(1);
        let instance = linker.instantiate(&mut store, &module).unwrap();
        let run = instance
            .get_typed_func::<(), i32>(&mut store, "run")
            .unwrap();
        let iterations = instance.get_global(&mut store, "iterations").unwrap();
        let result = run.call(&mut store, ());
        assert_eq!(store.data().calls, 1);
        // Inspect from the host; do not make another guest call after a trap.
        let iterations = iterations.get(&mut store).i32().unwrap();
        if enabled {
            let error = result.unwrap_err();
            assert_eq!(error.downcast_ref::<Trap>(), Some(&Trap::Interrupt));
            assert!((1..3).contains(&iterations), "iterations={iterations}");
        } else {
            assert_eq!(result.unwrap(), 42);
            assert_eq!(iterations, 3);
        }
    }
}
