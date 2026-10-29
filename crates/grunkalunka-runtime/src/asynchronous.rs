use wasmtime::{Config, Engine, Instance, Module, Store, TypedFunc};

/// Guest rejection tracking, in occurrence order at each invocation boundary.
/// IDs are private to one invocation. An embedding reports rejected IDs still
/// unhandled at its microtask checkpoint; a later Handled cancels that report
/// or provides the host's rejection-handled notification.
#[derive(Clone, Debug)]
pub enum PromiseNotification {
    Rejected { id: u32, reason: super::Value },
    Handled { id: u32 },
}

/// Observable settlement of the experimental Promise profile and opt-in objects.
#[derive(Clone, Debug)]
pub enum AsyncState {
    Pending,
    Fulfilled(super::Value),
    Rejected(super::Value),
}

/// A compiled import-free async module that can be instantiated repeatedly.
/// Each activation receives a fresh Store and guest Instance, preserving independent
/// fuel budgets, queued jobs, rejection identities and mutable guest state.
/// Keep the prepared module alive while creating new activations.
pub struct PreparedAsyncModule {
    engine: Engine,
    module: Module,
}

impl PreparedAsyncModule {
    /// Validate and compile once, rather than recompiling for each input vector.
    pub fn new(wasm: &[u8]) -> wasmtime::Result<Self> {
        let mut config = Config::new();
        config.consume_fuel(true);
        let engine = Engine::new(&config)?;
        let module = Module::from_binary(&engine, wasm)?;
        if module.imports().len() != 0 {
            wasmtime::bail!("this experiment admits no module imports");
        }
        Ok(Self { engine, module })
    }

    /// Start an isolated async invocation using the already compiled Wasm.
    pub fn instantiate(&self, a: f64, b: f64) -> wasmtime::Result<AsyncInstance> {
        let mut store = Store::new(&self.engine, ());
        store.set_fuel(super::FUEL_PER_CALL)?;
        let instance = Instance::new(&mut store, &self.module, &[])?;
        AsyncInstance::start_with_instance(store, instance, a, b)
    }
}

/// One invocation. After creation and each callback, enqueue any newly owed
/// `run_one_job` calls in the shared FIFO microtask queue. `pending_job_count`
/// includes callbacks already scheduled but not yet run. Root settlement does
/// not cancel the other activations. Drop releases all saved state.
pub struct AsyncInstance {
    store: Store<()>,
    resume: TypedFunc<(i32, f64), (i32, i32, f64)>,
    take_jobs: Option<TypedFunc<(), i32>>,
    take_promise_event: Option<TypedFunc<(), (i32, i32, i32, f64)>>,
    notifications: Vec<PromiseNotification>,
    rejections: std::collections::BTreeSet<u32>,
    rejection_ids: std::collections::BTreeSet<u32>,
    error_kind: Option<TypedFunc<f64, i32>>,
    promise_snapshot: Option<TypedFunc<f64, (i32, i32, i32, f64)>>,
    promise_states: std::collections::BTreeMap<u32, (i32, i32, i32, f64)>,
    ready_jobs: u8,
    state: AsyncState,
    pending: Option<(i32, i32, f64)>,
    jobs_left: u8,
    usable: bool,
}

impl AsyncInstance {
    /// Compile and run one invocation. For multiple input vectors, prefer
    /// PreparedAsyncModule::new followed by repeated instantiate calls.
    pub fn new(wasm: &[u8], a: f64, b: f64) -> wasmtime::Result<Self> {
        PreparedAsyncModule::new(wasm)?.instantiate(a, b)
    }

    fn start_with_instance(
        mut store: Store<()>,
        instance: Instance,
        a: f64,
        b: f64,
    ) -> wasmtime::Result<Self> {
        let start = instance.get_typed_func::<(f64, f64), (i32, i32, f64)>(&mut store, "start")?;
        let resume = instance.get_typed_func(&mut store, "resume")?;
        let take_jobs = instance
            .get_func(&mut store, "take_jobs")
            .map(|f| f.typed(&store))
            .transpose()?;
        let take_promise_event = instance
            .get_func(&mut store, "take_promise_event")
            .map(|f| f.typed(&store))
            .transpose()?;
        if take_promise_event.is_some() && take_jobs.is_none() {
            wasmtime::bail!("Promise events require queued async protocol");
        }
        let error_kind = instance
            .get_func(&mut store, "error_kind")
            .map(|f| f.typed(&store))
            .transpose()?;
        if error_kind.is_some() && take_jobs.is_none() {
            wasmtime::bail!("error objects require queued async protocol");
        }
        let promise_snapshot = instance
            .get_func(&mut store, "promise_state")
            .map(|f| f.typed(&store))
            .transpose()?;
        if promise_snapshot.is_some() && take_jobs.is_none() {
            wasmtime::bail!("Promise objects require queued async protocol");
        }
        store.set_fuel(super::FUEL_PER_CALL)?;
        let result = start.call(&mut store, (a, b))?;
        let mut activation = Self {
            store,
            resume,
            take_jobs,
            take_promise_event,
            notifications: Vec::new(),
            rejections: std::collections::BTreeSet::new(),
            rejection_ids: std::collections::BTreeSet::new(),
            error_kind,
            promise_snapshot,
            promise_states: std::collections::BTreeMap::new(),
            ready_jobs: 0,
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

    /// Read a Promise from this invocation without handling it or running a job.
    /// IDs are invocation-local. Query errors, traps and fuel failure terminate
    /// the activation, just as errors at ordinary invocation boundaries do.
    pub fn promise_state(&mut self, id: u32) -> wasmtime::Result<AsyncState> {
        self.state()?;
        self.usable = false;
        self.store.set_fuel(super::FUEL_PER_CALL)?;
        let (state, _, tag, payload) = self.read_promise(id)?;
        let result = match state {
            0 => AsyncState::Pending,
            1 => AsyncState::Fulfilled(self.decode(tag, payload, false)?),
            _ => AsyncState::Rejected(self.decode(tag, payload, true)?),
        };
        self.usable = true;
        Ok(result)
    }

    /// Consume tracking events after construction and every job, including jobs
    /// after root settlement. This does not run a job or perform a checkpoint.
    pub fn take_promise_notifications(&mut self) -> wasmtime::Result<Vec<PromiseNotification>> {
        self.state()?;
        Ok(std::mem::take(&mut self.notifications))
    }

    /// Outstanding callbacks, including those already placed on a host queue.
    /// Schedule the difference from callbacks the host has already registered,
    /// immediately after construction or a job, even when `state` is settled.
    pub fn pending_job_count(&self) -> wasmtime::Result<usize> {
        self.state()?;
        Ok(usize::from(self.ready_jobs) + usize::from(self.pending.is_some()))
    }

    /// Process at most one await continuation. True means one job ran; false
    /// means no ready job. Fuel/traps/protocol errors terminate this activation.
    pub fn run_one_job(&mut self) -> wasmtime::Result<bool> {
        self.state()?;
        if self.take_jobs.is_some() {
            if self.ready_jobs == 0 {
                return Ok(false);
            }
            self.ready_jobs -= 1;
            self.usable = false;
            self.store.set_fuel(super::FUEL_PER_CALL)?;
            let result = self.resume.call(&mut self.store, (0, 0.0))?;
            self.accept(result)?;
            self.usable = true;
            return Ok(true);
        }
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
        let value = self.decode(tag, payload, kind == 3)?;
        if let Some(take_jobs) = &self.take_jobs {
            if !matches!(kind, 0 | 1 | 3) {
                wasmtime::bail!("invalid queued async protocol status {kind}");
            }
            let issued = take_jobs.call(&mut self.store, ())?;
            if issued < 0 || issued > i32::from(self.jobs_left) {
                wasmtime::bail!("async activation exceeded its 32-job budget");
            }
            self.jobs_left -= issued as u8;
            self.ready_jobs += issued as u8;
            self.collect_notifications()?;
            self.state = match kind {
                0 => AsyncState::Fulfilled(value),
                3 => AsyncState::Rejected(value),
                _ => AsyncState::Pending,
            };
            return Ok(());
        }
        match kind {
            0 => {
                self.state = AsyncState::Fulfilled(value);
                self.pending = None;
            }
            3 => {
                self.state = AsyncState::Rejected(value);
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

    fn collect_notifications(&mut self) -> wasmtime::Result<()> {
        let Some(take) = self.take_promise_event.clone() else {
            return Ok(());
        };
        for _ in 0..32768 {
            let (kind, id, tag, payload) = take.call(&mut self.store, ())?;
            if kind == 0 {
                if id != 0 || tag != 0 || payload.to_bits() != 0 {
                    wasmtime::bail!("invalid empty Promise event");
                }
                return Ok(());
            }
            if id < 16 || id % 16 != 0 {
                wasmtime::bail!("invalid Promise event identity");
            }
            let id = id as u32;
            let event = match kind {
                1 if !self.rejections.contains(&id)
                    && !(self.promise_snapshot.is_some() && self.rejection_ids.contains(&id)) =>
                {
                    if self.promise_snapshot.is_some() {
                        let (state, _, record_tag, record_payload) = self.read_promise(id)?;
                        if state != 2
                            || record_tag != tag
                            || !Self::same_payload(record_payload, payload)
                        {
                            wasmtime::bail!("Promise rejection event disagrees with its record");
                        }
                    }
                    let reason = self.decode(tag, payload, true)?;
                    if self.promise_snapshot.is_some() {
                        self.rejection_ids.insert(id);
                    }
                    self.rejections.insert(id);
                    PromiseNotification::Rejected { id, reason }
                }
                2 if tag == 0 && payload.to_bits() == 0 && self.rejections.remove(&id) => {
                    if self.promise_snapshot.is_some() {
                        let (state, handled, _, _) = self.read_promise(id)?;
                        if state != 2 || handled != 1 {
                            wasmtime::bail!("Promise handling event disagrees with its record");
                        }
                    }
                    PromiseNotification::Handled { id }
                }
                _ => wasmtime::bail!("invalid Promise event transition"),
            };
            self.notifications.push(event);
        }
        wasmtime::bail!("too many Promise events")
    }

    fn same_payload(left: f64, right: f64) -> bool {
        left.to_bits() == right.to_bits() || left.is_nan() && right.is_nan()
    }

    fn promise_id(payload: f64) -> wasmtime::Result<u32> {
        if !payload.is_finite() || payload < 16.0 || payload > 1_048_544.0 || payload % 16.0 != 0.0
        {
            wasmtime::bail!("invalid Promise object identity");
        }
        Ok(payload as u32)
    }

    fn read_promise(&mut self, id: u32) -> wasmtime::Result<(i32, i32, i32, f64)> {
        Self::promise_id(f64::from(id))?;
        let query = self.promise_snapshot.clone().ok_or_else(|| {
            wasmtime::format_err!("Promise objects require a promise_state export")
        })?;
        let row @ (state, handled, tag, payload) = query.call(&mut self.store, f64::from(id))?;
        if !matches!(state, 0..=2) || !matches!(handled, 0 | 1) {
            wasmtime::bail!("invalid Promise snapshot state");
        }
        if state == 0 {
            if tag != 0 || payload.to_bits() != 0 {
                wasmtime::bail!("invalid pending Promise snapshot");
            }
        } else if state == 2 && tag == 5 {
            Self::promise_id(payload)?;
        } else {
            self.decode(tag, payload, false)?;
        }
        if let Some(previous) = self.promise_states.get(&id) {
            let same_payload = Self::same_payload(previous.3, payload);
            if previous.0 != 0 && (state != previous.0 || tag != previous.2 || !same_payload) {
                wasmtime::bail!("Promise settlement changed");
            }
            if previous.1 == 1 && handled == 0 {
                wasmtime::bail!("Promise handling changed");
            }
        } else if self.promise_states.len() >= 32768 {
            wasmtime::bail!("too many Promise objects");
        }
        self.promise_states.insert(id, row);
        Ok(row)
    }

    fn decode(
        &mut self,
        tag: i32,
        payload: f64,
        allow_promise: bool,
    ) -> wasmtime::Result<super::Value> {
        if tag == 5 && allow_promise {
            let id = Self::promise_id(payload)?;
            self.read_promise(id)?;
            return Ok(super::Value::Promise { id });
        }
        if tag == 4
            && let Some(kind) = &self.error_kind
        {
            if !payload.is_finite()
                || payload < 16.0
                || payload > 1_048_544.0
                || payload % 16.0 != 0.0
            {
                wasmtime::bail!("invalid error object identity");
            }
            if kind.call(&mut self.store, payload)? != 1 {
                wasmtime::bail!("invalid error object kind");
            }
            return Ok(super::Value::TypeError { id: payload as u32 });
        }
        super::decode_primitive(tag, payload)
    }
}
