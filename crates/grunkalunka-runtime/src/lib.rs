//! Execution of locally generated numeric modules; no ambient imports are linked.
//! This is not a hardened arbitrary-module execution service.

use wasmtime::{Config, Engine, Instance, Module, Store, TypedFunc};

/// Current numeric execution budget, renewed before each call.
pub const FUEL_PER_CALL: u64 = 10_000;

/// An instantiated `(f64, f64) -> f64` numeric module with its own store.
pub struct NumericInstance {
    store: Store<()>,
    run: TypedFunc<(f64, f64), f64>,
}

impl NumericInstance {
    /// Validate and instantiate a locally generated, import-free module.
    pub fn new(wasm: &[u8]) -> wasmtime::Result<Self> {
        let mut config = Config::new();
        config.consume_fuel(true);
        let engine = Engine::new(&config)?;
        let module = Module::from_binary(&engine, wasm)?;
        if module.imports().len() != 0 {
            wasmtime::bail!("this experiment admits no module imports");
        }
        let mut store = Store::new(&engine, ());
        store.set_fuel(FUEL_PER_CALL)?;
        let instance = Instance::new(&mut store, &module, &[])?;
        let run = instance.get_typed_func::<(f64, f64), f64>(&mut store, "run")?;
        Ok(Self { store, run })
    }

    /// Execute one pair of primitive numbers, renewing the fuel budget.
    pub fn run(&mut self, a: f64, b: f64) -> wasmtime::Result<f64> {
        self.store.set_fuel(FUEL_PER_CALL)?;
        self.run.call(&mut self.store, (a, b))
    }
}
