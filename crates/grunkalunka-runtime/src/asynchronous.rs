use wasmtime::{Store, TypedFunc};

/// Observable settlement of the experimental primitive Promise profile.
#[derive(Clone, Debug)]
pub enum AsyncState {
    Pending,
    Fulfilled(super::Value),
    Rejected(super::Value),
}

/// One async activation. A scheduler must enqueue one `run_one_job` call for
/// each Pending state, in its shared FIFO microtask queue. Use a fresh instance
/// per source invocation; Drop releases the activation's saved state.
pub struct AsyncInstance {
    store: Store<()>,
    resume: TypedFunc<(i32, f64), (i32, i32, f64)>,
    state: AsyncState,
    pending: Option<(i32, i32, f64)>,
    jobs_left: u8,
    usable: bool,
}

impl AsyncInstance {
    /// Run the synchronous prefix, stopping at the first await or return.
    pub fn new(wasm: &[u8], a: f64, b: f64) -> wasmtime::Result<Self> {
        let (mut store, instance) = super::instantiate(wasm, super::FUEL_PER_CALL)?;
        let start = instance.get_typed_func::<(f64, f64), (i32, i32, f64)>(&mut store, "start")?;
        let resume = instance.get_typed_func(&mut store, "resume")?;
        store.set_fuel(super::FUEL_PER_CALL)?;
        let result = start.call(&mut store, (a, b))?;
        let mut activation = Self {
            store,
            resume,
            state: AsyncState::Pending,
            pending: None,
            jobs_left: 32,
            usable: true,
        };
        activation.accept(result)?;
        Ok(activation)
    }

    pub fn state(&self) -> wasmtime::Result<AsyncState> {
        if !self.usable {
            wasmtime::bail!("async activation is unusable after an earlier error");
        }
        Ok(self.state.clone())
    }

    /// Process at most one await continuation. True means one job ran; false
    /// means already settled. Fuel/traps/protocol errors terminate this activation.
    pub fn run_one_job(&mut self) -> wasmtime::Result<bool> {
        self.state()?;
        let Some((kind, tag, payload)) = self.pending.take() else {
            return Ok(false);
        };
        self.usable = false;
        if self.jobs_left == 0 {
            wasmtime::bail!("async activation exceeded its 32-job budget");
        }
        self.jobs_left -= 1;
        if kind == 2 {
            self.state = AsyncState::Rejected(super::decode_primitive(tag, payload)?);
        } else {
            self.store.set_fuel(super::FUEL_PER_CALL)?;
            let result = self.resume.call(&mut self.store, (tag, payload))?;
            self.accept(result)?;
        }
        self.usable = true;
        Ok(true)
    }

    fn accept(&mut self, (kind, tag, payload): (i32, i32, f64)) -> wasmtime::Result<()> {
        let value = super::decode_primitive(tag, payload)?;
        match kind {
            0 => {
                self.state = AsyncState::Fulfilled(value);
                self.pending = None;
            }
            1 | 2 => {
                if self.jobs_left == 0 {
                    wasmtime::bail!("async activation exceeded its 32-job budget");
                }
                self.state = AsyncState::Pending;
                self.pending = Some((kind, tag, payload));
            }
            _ => wasmtime::bail!("invalid async protocol status {kind}"),
        }
        Ok(())
    }
}
