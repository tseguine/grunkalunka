//! Scalar WIT resource-call cost control; never pure host-dispatch overhead.
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Instant;
use wasmtime::component::{Component, HasSelf, Linker, Resource};
use wasmtime::{Engine, Store};

wasmtime::component::bindgen!({
    path: "tests/fixtures/capabilities",
    world: "runner",
    with: { "grunkalunka:r1/cells.cell": BackingCell },
    imports: { default: trappable },
});

pub struct BackingCell;
const WAT: &str = include_str!("../tests/fixtures/capabilities/reader.wat");
const CASES: [(f64, f64, f64); 3] = [(4.0, 1.0, 14.0), (-2.0, 3.0, 10.0), (0.0, 0.0, 9.0)];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub samples: usize,
    pub iterations: usize,
    pub warmup: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            samples: 9,
            iterations: 1000,
            warmup: 100,
        }
    }
}

impl Options {
    fn validate(self) -> Result<(), String> {
        for (name, value, min, max) in [
            ("samples", self.samples, 2, 20),
            ("iterations", self.iterations, 1, 5000),
            ("warmup", self.warmup, 0, 1000),
        ] {
            if !(min..=max).contains(&value) {
                return Err(format!("{name} must be within {min}..={max}"));
            }
        }
        Ok(())
    }
}

pub fn parse_options(args: &[String]) -> Result<Options, String> {
    if !args.len().is_multiple_of(2) {
        return Err("expected unique --option value pairs".into());
    }
    let mut controls = Options::default();
    let mut seen = BTreeSet::new();
    for pair in args.as_chunks::<2>().0 {
        let name = pair[0].as_str();
        if !seen.insert(name) {
            return Err(format!("duplicate option {name}"));
        }
        let slot = match name {
            "--samples" => &mut controls.samples,
            "--iterations" => &mut controls.iterations,
            "--warmup" => &mut controls.warmup,
            _ => return Err(format!("unknown option {name}")),
        };
        *slot = pair[1]
            .parse::<usize>()
            .map_err(|_| format!("invalid integer for {name}"))?;
    }
    controls.validate()?;
    Ok(controls)
}

#[derive(Clone)]
struct Host {
    allowed: bool,
    value: f64,
    attempts: usize,
    reads: usize,
}

impl Host {
    fn fetch(&mut self, representation: u32) -> wasmtime::Result<f64> {
        self.attempts += 1;
        if !self.allowed {
            wasmtime::bail!("cell denied by private test policy");
        }
        if representation != 7 {
            wasmtime::bail!("unknown private host cell representation");
        }
        self.reads += 1;
        Ok(self.value)
    }
}

impl grunkalunka::r1::cells::Host for Host {}
impl grunkalunka::r1::cells::HostCell for Host {
    fn read(&mut self, cell: Resource<Cell>) -> wasmtime::Result<f64> {
        self.fetch(cell.rep())
    }

    fn drop(&mut self, _: Resource<Cell>) -> wasmtime::Result<()> {
        wasmtime::bail!("the private probe never transfers ownership of cells")
    }
}

fn host(granted: bool) -> Host {
    Host {
        allowed: granted,
        value: 9.0,
        attempts: 0,
        reads: 0,
    }
}

fn instantiate(
    engine: &Engine,
    component: &Component,
    linker: &Linker<Host>,
    granted: bool,
) -> wasmtime::Result<(Store<Host>, Runner)> {
    let mut store = Store::new(engine, host(granted));
    let runner = Runner::instantiate(&mut store, component, linker)?;
    Ok((store, runner))
}

fn milliseconds(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

fn expected_checksum(iterations: usize) -> f64 {
    (0..iterations)
        .map(|i| CASES[i % CASES.len()].2)
        .sum::<f64>()
}

fn native_batch(host: &mut Host, iterations: usize) -> wasmtime::Result<f64> {
    let mut checksum = 0.0;
    for i in 0..iterations {
        let (a, b, _) = CASES[i % CASES.len()];
        let value = black_box(host.fetch(7)?);
        checksum += black_box(a + b + value);
    }
    Ok(checksum)
}

fn component_batch(
    store: &mut Store<Host>,
    runner: &Runner,
    iterations: usize,
) -> wasmtime::Result<f64> {
    let mut checksum = 0.0;
    for i in 0..iterations {
        let (a, b, _) = CASES[i % CASES.len()];
        checksum += black_box(runner.call_run(&mut *store, Resource::new_borrow(7), a, b)?);
    }
    Ok(checksum)
}

#[derive(Debug)]
pub struct Series {
    pub samples_ms: Vec<f64>,
    pub min_ms: f64,
    pub median_ms: f64,
    pub max_ms: f64,
}

impl Series {
    fn new(samples_ms: Vec<f64>) -> Self {
        let mut sorted = samples_ms.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let middle = sorted.len() / 2;
        let median_ms = if sorted.len().is_multiple_of(2) {
            sorted[middle - 1] / 2.0 + sorted[middle] / 2.0
        } else {
            sorted[middle]
        };
        Self {
            min_ms: sorted[0],
            median_ms,
            max_ms: *sorted.last().unwrap(),
            samples_ms,
        }
    }

    fn json(&self) -> String {
        let samples = self
            .samples_ms
            .iter()
            .map(|number| format!("{number:.9}"))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"{{"samplesMs":[{samples}],"minMs":{:.9},"medianMs":{:.9},"maxMs":{:.9}}}"#,
            self.min_ms, self.median_ms, self.max_ms
        )
    }
}

pub struct Report {
    pub options: Options,
    pub component_bytes: usize,
    pub wat_parse_ms: f64,
    pub compile_ms: f64,
    pub instantiation: Series,
    pub native: Series,
    pub component: Series,
    pub granted_reads: usize,
    pub denied_reads: usize,
}

impl Report {
    pub fn json(&self) -> String {
        let mut out = String::from(r#"{"schemaVersion":1,"workload":"scalar-wit-resource-read","#);
        write!(
            out,
            r#""configuration":{{"samples":{},"iterations":{},"warmup":{}}},"#,
            self.options.samples, self.options.iterations, self.options.warmup
        )
        .unwrap();
        write!(
            out,
            r#""environment":{{"os":"{}","arch":"{}","wasmtimeDependency":"=50.0.0"}},"#,
            std::env::consts::OS,
            std::env::consts::ARCH
        )
        .unwrap();
        write!(
            out,
            r#""component":{{"wasmBytes":{},"watParseMs":{:.9},"compileMs":{:.9}}},"#,
            self.component_bytes, self.wat_parse_ms, self.compile_ms
        )
        .unwrap();
        write!(
            out,
            r#""batches":{{"unit":"milliseconds per batch","order":"alternate native/component","instantiation":{},"native":{},"component":{}}},"#,
            self.instantiation.json(), self.native.json(), self.component.json()
        )
        .unwrap();
        write!(
            out,
            r#""outcomes":{{"grantedReads":{},"deniedReads":{},"goldenCases":{}}}}}"#,
            self.granted_reads,
            self.denied_reads,
            CASES.len()
        )
        .unwrap();
        out
    }
}

pub fn probe(options: Options) -> wasmtime::Result<Report> {
    options.validate().map_err(std::io::Error::other)?;
    let engine = Engine::default();

    let started = Instant::now();
    let bytes = wat::parse_str(WAT)?;
    let wat_parse_ms = milliseconds(started);
    let started = Instant::now();
    let component = Component::from_binary(&engine, &bytes)?;
    let compile_ms = milliseconds(started);

    let mut linker = Linker::new(&engine);
    Runner::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)?;

    let mut instantiation = Vec::with_capacity(options.samples);
    for _ in 0..options.samples {
        let started = Instant::now();
        let new_instance = instantiate(&engine, &component, &linker, true)?;
        instantiation.push(milliseconds(started));
        drop(new_instance);
    }

    // Errors are checked on distinct instances and never reused for timing.
    let (mut denied, denied_runner) = instantiate(&engine, &component, &linker, false)?;
    if denied_runner
        .call_run(&mut denied, Resource::new_borrow(7), 4.0, 1.0)
        .is_ok()
        || denied.data().attempts != 1
        || denied.data().reads != 0
    {
        wasmtime::bail!("private denied profile performed a backing read");
    }
    let (mut unknown, unknown_runner) = instantiate(&engine, &component, &linker, true)?;
    if unknown_runner
        .call_run(&mut unknown, Resource::new_borrow(8), 4.0, 1.0)
        .is_ok()
        || unknown.data().attempts != 1
        || unknown.data().reads != 0
    {
        wasmtime::bail!("ungranted private cell representation was accepted");
    }

    let (mut store, runner) = instantiate(&engine, &component, &linker, true)?;
    let mut native = host(true);
    for (a, b, expected) in CASES {
        let actual = runner.call_run(&mut store, Resource::new_borrow(7), a, b)?;
        if actual.to_bits() != expected.to_bits()
            || (a + b + native.fetch(7)?).to_bits() != expected.to_bits()
        {
            wasmtime::bail!("scalar host/component golden mismatch");
        }
    }

    if native_batch(&mut native, options.warmup)? != expected_checksum(options.warmup)
        || component_batch(&mut store, &runner, options.warmup)?
            != expected_checksum(options.warmup)
    {
        wasmtime::bail!("scalar host/component warmup mismatch");
    }

    let mut native_samples = Vec::with_capacity(options.samples);
    let mut component_samples = Vec::with_capacity(options.samples);
    for index in 0..options.samples {
        if index.is_multiple_of(2) {
            let started = Instant::now();
            let actual = native_batch(&mut native, options.iterations)?;
            native_samples.push(milliseconds(started));
            if actual != expected_checksum(options.iterations) {
                wasmtime::bail!("native scalar batch checksum mismatch");
            }
            let started = Instant::now();
            let actual = component_batch(&mut store, &runner, options.iterations)?;
            component_samples.push(milliseconds(started));
            if actual != expected_checksum(options.iterations) {
                wasmtime::bail!("component scalar batch checksum mismatch");
            }
        } else {
            let started = Instant::now();
            let actual = component_batch(&mut store, &runner, options.iterations)?;
            component_samples.push(milliseconds(started));
            if actual != expected_checksum(options.iterations) {
                wasmtime::bail!("component scalar batch checksum mismatch");
            }
            let started = Instant::now();
            let actual = native_batch(&mut native, options.iterations)?;
            native_samples.push(milliseconds(started));
            if actual != expected_checksum(options.iterations) {
                wasmtime::bail!("native scalar batch checksum mismatch");
            }
        }
    }

    let attempts = CASES.len() + options.warmup + options.samples * options.iterations;
    if native.attempts != attempts
        || native.reads != attempts
        || store.data().attempts != attempts
        || store.data().reads != attempts
    {
        wasmtime::bail!("scalar host read counters changed during measurement");
    }
    Ok(Report {
        options,
        component_bytes: bytes.len(),
        wat_parse_ms,
        compile_ms,
        instantiation: Series::new(instantiation),
        native: Series::new(native_samples),
        component: Series::new(component_samples),
        granted_reads: store.data().reads,
        denied_reads: denied.data().reads + unknown.data().reads,
    })
}

fn run_main() -> wasmtime::Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let options = parse_options(&args).map_err(std::io::Error::other)?;
    println!("{}", probe(options)?.json());
    Ok(())
}

fn main() {
    if let Err(error) = run_main() {
        eprintln!("scalar WIT cost probe: {error:#}");
        std::process::exit(1);
    }
}
