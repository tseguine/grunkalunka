//! Execution of locally generated numeric, primitive and experimental heap modules.
//! No ambient imports are linked.
//! This is not a hardened arbitrary-module execution service.

use wasmtime::{Config, Engine, Instance, Module, Store, TypedFunc};

mod heap;
pub use heap::HeapInstance;

/// Current execution budget, renewed before each call.
pub const FUEL_PER_CALL: u64 = 10_000;

fn instantiate(wasm: &[u8], fuel: u64) -> wasmtime::Result<(Store<()>, Instance)> {
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let module = Module::from_binary(&engine, wasm)?;
    if module.imports().len() != 0 {
        wasmtime::bail!("this experiment admits no module imports");
    }
    let mut store = Store::new(&engine, ());
    store.set_fuel(fuel)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    Ok((store, instance))
}

/// An instantiated `(f64, f64) -> f64` numeric module with its own store.
pub struct NumericInstance {
    store: Store<()>,
    run: TypedFunc<(f64, f64), f64>,
}

impl NumericInstance {
    /// Validate and instantiate a locally generated, import-free module.
    pub fn new(wasm: &[u8]) -> wasmtime::Result<Self> {
        let (mut store, instance) = instantiate(wasm, FUEL_PER_CALL)?;
        let run = instance.get_typed_func::<(f64, f64), f64>(&mut store, "run")?;
        Ok(Self { store, run })
    }

    /// Execute one pair of primitive numbers, renewing the fuel budget.
    pub fn run(&mut self, a: f64, b: f64) -> wasmtime::Result<f64> {
        self.store.set_fuel(FUEL_PER_CALL)?;
        self.run.call(&mut self.store, (a, b))
    }
}

/// Value cases currently supported by the primitive execution profile.
#[derive(Clone, Debug)]
pub enum Value {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
}

/// An import-free `(f64, f64) -> (i32, f64)` primitive module.
/// The result is a discriminant and payload; this is separate from the numeric ABI.
pub struct ValueInstance {
    store: Store<()>,
    run: TypedFunc<(f64, f64), (i32, f64)>,
}

impl ValueInstance {
    /// Validate the primitive signature and instantiate in its own fuel-limited store.
    pub fn new(wasm: &[u8]) -> wasmtime::Result<Self> {
        let (mut store, instance) = instantiate(wasm, FUEL_PER_CALL)?;
        let run = instance.get_typed_func::<(f64, f64), (i32, f64)>(&mut store, "run")?;
        Ok(Self { store, run })
    }

    /// Execute with two numeric arguments and decode a canonical tagged result.
    pub fn run(&mut self, a: f64, b: f64) -> wasmtime::Result<Value> {
        self.store.set_fuel(FUEL_PER_CALL)?;
        let (tag, payload) = self.run.call(&mut self.store, (a, b))?;
        match tag {
            0 if payload.to_bits() == 0 => Ok(Value::Undefined),
            1 if payload.to_bits() == 0 => Ok(Value::Null),
            2 if [0, 1.0_f64.to_bits()].contains(&payload.to_bits()) => {
                Ok(Value::Boolean(payload == 1.0))
            }
            3 => Ok(Value::Number(payload)),
            _ => wasmtime::bail!("invalid primitive value tag or payload"),
        }
    }
}
