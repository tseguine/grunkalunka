use wasmtime::{Memory, Store, TypedFunc};

const FUEL: u64 = 1_000_000;

/// Experimental String-only heap host; results own their raw UTF-16 units.
pub struct HeapInstance {
    store: Store<()>,
    run: TypedFunc<(f64, f64), (i32, f64)>,
    string_units: TypedFunc<u32, u32>,
    output_ptr: TypedFunc<(), u32>,
    output_len: TypedFunc<(), u32>,
    release: TypedFunc<u32, u32>,
    memory: Memory,
    usable: bool,
}

impl HeapInstance {
    /// Instantiate a locally composed, import-free module with the String protocol.
    /// Initialization has one million fuel units, separate from the per-call budget.
    pub fn new(wasm: &[u8]) -> wasmtime::Result<Self> {
        let (mut store, instance) = super::instantiate(wasm, FUEL)?;
        let run = instance.get_typed_func(&mut store, "run")?;
        let string_units = instance.get_typed_func(&mut store, "gk_json_string_units")?;
        let output_ptr = instance.get_typed_func(&mut store, "gk_json_output_ptr")?;
        let output_len = instance.get_typed_func(&mut store, "gk_json_output_len")?;
        let release = instance.get_typed_func(&mut store, "gk_json_release")?;
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| wasmtime::Error::msg("missing heap memory"))?;
        Ok(Self {
            store,
            run,
            string_units,
            output_ptr,
            output_len,
            release,
            memory,
            usable: true,
        })
    }

    /// Copy at most 65,536 raw UTF-16 units, then release the guest root.
    /// The one-million-fuel call budget includes readback and release. Every error
    /// makes this instance permanently unusable; interrupting Rust may strand its lock.
    pub fn run(&mut self, a: f64, b: f64) -> wasmtime::Result<Vec<u16>> {
        if !self.usable {
            wasmtime::bail!("heap instance is unusable after an earlier error");
        }
        self.usable = false;
        let result = self.run_value(a, b);
        if result.is_ok() {
            self.usable = true;
        }
        result
    }

    fn run_value(&mut self, a: f64, b: f64) -> wasmtime::Result<Vec<u16>> {
        self.store.set_fuel(FUEL)?;
        let (tag, payload) = self.run.call(&mut self.store, (a, b))?;
        if tag != 4
            || !payload.is_finite()
            || payload <= 0.0
            || payload > u32::MAX as f64
            || payload.fract() != 0.0
        {
            wasmtime::bail!("invalid String tag or root ID");
        }
        let status = self.string_units.call(&mut self.store, payload as u32)?;
        if status != 0 {
            wasmtime::bail!("String readback failed: status {status}");
        }
        let pointer = self.output_ptr.call(&mut self.store, ())?;
        let length = self.output_len.call(&mut self.store, ())?;
        if length > 65_536 || !pointer.is_multiple_of(2) {
            wasmtime::bail!("invalid String output bounds");
        }
        let mut bytes = vec![0; length as usize * 2];
        self.memory
            .read(&self.store, pointer as usize, &mut bytes)?;
        let result = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .collect();
        let status = self.release.call(&mut self.store, payload as u32)?;
        if status != 0 {
            wasmtime::bail!("String release failed: status {status}");
        }
        Ok(result)
    }
}
