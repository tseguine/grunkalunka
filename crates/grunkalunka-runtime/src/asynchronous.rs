use wasmtime::{Store, TypedFunc};

/// Guest rejection tracking, in occurrence order at each invocation boundary.
/// IDs are private to one invocation. An embedding reports rejected IDs still
/// unhandled at its microtask checkpoint; a later Handled cancels that report
/// or provides the host's rejection-handled notification.
#[derive(Clone, Debug)]
pub enum PromiseNotification {
    Rejected { id: u32, reason: super::Value },
    Handled { id: u32 },
}

/// Observable settlement of the experimental primitive Promise profile.
#[derive(Clone, Debug)]
pub enum AsyncState {
    Pending,
    Fulfilled(super::Value),
    Rejected(super::Value),
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
    ready_jobs: u8,
    state: AsyncState,
    pending: Option<(i32, i32, f64)>,
    jobs_left: u8,
    usable: bool,
}

impl AsyncInstance {
    /// Run the synchronous prefix, stopping at the first await or settlement.
    pub fn new(wasm: &[u8], a: f64, b: f64) -> wasmtime::Result<Self> {
        let (mut store, instance) = super::instantiate(wasm, super::FUEL_PER_CALL)?;
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
        store.set_fuel(super::FUEL_PER_CALL)?;
        let result = start.call(&mut store, (a, b))?;
        let mut activation = Self {
            store,
            resume,
            take_jobs,
            take_promise_event,
            notifications: Vec::new(),
            rejections: std::collections::BTreeSet::new(),
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
        let value = super::decode_primitive(tag, payload)?;
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
        let Some(take) = &self.take_promise_event else {
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
                1 if !self.rejections.contains(&id) => {
                    let reason = super::decode_primitive(tag, payload)?;
                    self.rejections.insert(id);
                    PromiseNotification::Rejected { id, reason }
                }
                2 if tag == 0 && payload.to_bits() == 0 && self.rejections.remove(&id) => {
                    PromiseNotification::Handled { id }
                }
                _ => wasmtime::bail!("invalid Promise event transition"),
            };
            self.notifications.push(event);
        }
        wasmtime::bail!("too many Promise events")
    }
}
