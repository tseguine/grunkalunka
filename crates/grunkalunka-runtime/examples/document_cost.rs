//! Bounded P1 observation of private WIT document reads; not production throughput.
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Instant;
use wasmtime::component::{Component, HasSelf, Linker, Resource};
use wasmtime::{Engine, Store};

wasmtime::component::bindgen!({
    path: "tests/fixtures/document-transport",
    world: "reader",
    with: { "grunkalunka:documents/documents.document": BackingDocument },
    imports: { default: trappable },
});

pub struct BackingDocument;
const WAT: &str = include_str!("../tests/fixtures/document-transport/reader.wat");
const MODES: [&str; 4] = [
    "nativeGranted",
    "componentGranted",
    "nativeDenied",
    "componentDenied",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub samples: usize,
    pub iterations: usize,
    pub warmup: usize,
    pub bytes: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            samples: 9,
            iterations: 1000,
            warmup: 100,
            bytes: 128,
        }
    }
}

impl Options {
    fn validate(self) -> Result<(), String> {
        for (name, value, min, max) in [
            ("samples", self.samples, 2, 20),
            ("iterations", self.iterations, 1, 10_000),
            ("warmup", self.warmup, 0, 10_000),
            ("bytes", self.bytes, 0, 4096),
        ] {
            if !(min..=max).contains(&value) {
                return Err(format!("{name} must be within {min}..={max}"));
            }
        }
        Ok(())
    }
}

pub fn parse_options(args: &[String]) -> Result<Options, String> {
    if args.len() % 2 != 0 {
        return Err("expected unique --option value pairs".into());
    }
    let mut options = Options::default();
    let mut seen = BTreeSet::new();
    for pair in args.chunks_exact(2) {
        let name = pair[0].as_str();
        if !seen.insert(name) {
            return Err(format!("duplicate option {name}"));
        }
        let slot = match name {
            "--samples" => &mut options.samples,
            "--iterations" => &mut options.iterations,
            "--warmup" => &mut options.warmup,
            "--bytes" => &mut options.bytes,
            _ => return Err(format!("unknown option {name}")),
        };
        *slot = pair[1]
            .parse::<usize>()
            .map_err(|_| format!("invalid integer for {name}"))?;
    }
    options.validate()?;
    Ok(options)
}

#[derive(Clone)]
struct Host {
    allowed: bool,
    text: String,
    attempts: usize,
    reads: usize,
}

impl Host {
    fn fetch(&mut self, rep: u32) -> wasmtime::Result<Result<String, ReadError>> {
        self.attempts += 1;
        if !self.allowed {
            return Ok(Err(ReadError::Denied));
        }
        if rep != 7 {
            return Ok(Err(ReadError::UnknownGrant));
        }
        if self.text.len() > 4096 {
            return Ok(Err(ReadError::TooLarge));
        }
        self.reads += 1;
        Ok(Ok(self.text.clone()))
    }
}

impl grunkalunka::documents::documents::Host for Host {}
impl grunkalunka::documents::documents::HostDocument for Host {
    fn read(
        &mut self,
        document: Resource<Document>,
    ) -> wasmtime::Result<Result<String, ReadError>> {
        self.fetch(document.rep())
    }

    fn drop(&mut self, _: Resource<Document>) -> wasmtime::Result<()> {
        wasmtime::bail!("the benchmark only borrows documents")
    }
}

fn host(allowed: bool, text: &str) -> Host {
    Host {
        allowed,
        text: text.into(),
        attempts: 0,
        reads: 0,
    }
}

fn instance(
    engine: &Engine,
    component: &Component,
    linker: &Linker<Host>,
    allowed: bool,
    text: &str,
) -> wasmtime::Result<(Store<Host>, Reader)> {
    let mut store = Store::new(engine, host(allowed, text));
    let reader = Reader::instantiate(&mut store, component, linker)?;
    Ok((store, reader))
}

fn milliseconds(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

fn native_batch(host: &mut Host, iterations: usize) -> wasmtime::Result<()> {
    for _ in 0..iterations {
        black_box(host.fetch(7)?);
    }
    Ok(())
}

fn component_batch(
    store: &mut Store<Host>,
    reader: &Reader,
    iterations: usize,
) -> wasmtime::Result<()> {
    for _ in 0..iterations {
        black_box(reader.call_read(&mut *store, Resource::new_borrow(7))?);
    }
    Ok(())
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
        let median_ms = if sorted.len() % 2 == 0 {
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
    pub modes: [Series; 4],
    pub granted_reads: usize,
    pub denied_reads: usize,
}

impl Report {
    pub fn json(&self) -> String {
        let mut out = String::from("{\"schemaVersion\":1,\"workload\":\"wit-document-read\",");
        write!(
            out,
            "\"configuration\":{{\"samples\":{},\"iterations\":{},\"warmup\":{},\"payloadBytes\":{}}},",
            self.options.samples,
            self.options.iterations,
            self.options.warmup,
            self.options.bytes
        )
        .unwrap();
        write!(
            out,
            "\"environment\":{{\"os\":\"{}\",\"arch\":\"{}\",\"fuelEnabled\":false,\"wasmtimeDependency\":\"=50.0.0\"}},",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
        .unwrap();
        write!(
            out,
            "\"component\":{{\"wasmBytes\":{},\"watParseMs\":{:.9},\"compileMs\":{:.9}}},",
            self.component_bytes, self.wat_parse_ms, self.compile_ms
        )
        .unwrap();
        out.push_str("\"batches\":{\"unit\":\"milliseconds per batch\",\"order\":\"alternate forward/reverse\",");
        write!(out, "\"instantiation\":{},", self.instantiation.json()).unwrap();
        for (index, name) in MODES.iter().enumerate() {
            write!(out, "\"{name}\":{}", self.modes[index].json()).unwrap();
            if index != MODES.len() - 1 {
                out.push(',');
            }
        }
        write!(
            out,
            "}},\"outcomes\":{{\"componentGrantedReads\":{},\"componentDeniedReads\":{}}}}}",
            self.granted_reads, self.denied_reads
        )
        .unwrap();
        out
    }
}

fn payload(bytes: usize) -> String {
    let mut text = "é".repeat(bytes / 2);
    if bytes % 2 != 0 {
        text.push('x');
    }
    debug_assert_eq!(text.len(), bytes);
    text
}

pub fn probe(options: Options) -> wasmtime::Result<Report> {
    if let Err(message) = options.validate() {
        wasmtime::bail!("{message}");
    }
    let text = payload(options.bytes);
    let engine = Engine::default();

    let started = Instant::now();
    let binary = wat::parse_str(WAT)?;
    let wat_parse_ms = milliseconds(started);
    let started = Instant::now();
    let component = Component::from_binary(&engine, &binary)?;
    let compile_ms = milliseconds(started);

    let mut linker = Linker::new(&engine);
    Reader::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)?;

    let mut instantiation = Vec::with_capacity(options.samples);
    for _ in 0..options.samples {
        let started = Instant::now();
        let value = instance(&engine, &component, &linker, true, &text)?;
        instantiation.push(milliseconds(started));
        drop(value);
    }

    let (mut allowed, allowed_reader) = instance(&engine, &component, &linker, true, &text)?;
    let (mut denied, denied_reader) = instance(&engine, &component, &linker, false, &text)?;
    let mut native_allowed = host(true, &text);
    let mut native_denied = host(false, &text);

    // Check real goldens before timing, against independent paths.
    if allowed_reader.call_read(&mut allowed, Resource::new_borrow(7))? != Ok(text.clone())
        || native_allowed.fetch(7)? != Ok(text.clone())
    {
        wasmtime::bail!("granted document transport mismatch");
    }
    if denied_reader.call_read(&mut denied, Resource::new_borrow(7))? != Err(ReadError::Denied)
        || native_denied.fetch(7)? != Err(ReadError::Denied)
    {
        wasmtime::bail!("denied document transport mismatch");
    }
    if allowed_reader.call_read(&mut allowed, Resource::new_borrow(8))?
        != Err(ReadError::UnknownGrant)
    {
        wasmtime::bail!("unknown grant was not rejected");
    }

    native_batch(&mut native_allowed, options.warmup)?;
    component_batch(&mut allowed, &allowed_reader, options.warmup)?;
    native_batch(&mut native_denied, options.warmup)?;
    component_batch(&mut denied, &denied_reader, options.warmup)?;

    let mut samples: [Vec<f64>; 4] = std::array::from_fn(|_| Vec::with_capacity(options.samples));
    for index in 0..options.samples {
        let order = if index % 2 == 0 {
            [0, 1, 2, 3]
        } else {
            [3, 2, 1, 0]
        };
        for mode in order {
            let started = Instant::now();
            match mode {
                0 => native_batch(&mut native_allowed, options.iterations)?,
                1 => component_batch(&mut allowed, &allowed_reader, options.iterations)?,
                2 => native_batch(&mut native_denied, options.iterations)?,
                3 => component_batch(&mut denied, &denied_reader, options.iterations)?,
                _ => unreachable!(),
            }
            samples[mode].push(milliseconds(started));
        }
    }

    // Validate counter and cleanup behavior only after timed batches.
    let repeats = options.warmup + options.samples * options.iterations;
    if allowed.data().attempts != 2 + repeats
        || allowed.data().reads != 1 + repeats
        || denied.data().attempts != 1 + repeats
        || denied.data().reads != 0
        || native_allowed.attempts != 1 + repeats
        || native_allowed.reads != 1 + repeats
        || native_denied.attempts != 1 + repeats
        || native_denied.reads != 0
        || allowed_reader.call_cleanup_count(&mut allowed)? as usize != 2 + repeats
        || denied_reader.call_cleanup_count(&mut denied)? as usize != 1 + repeats
    {
        wasmtime::bail!("document read or cleanup count changed during measurement");
    }

    Ok(Report {
        options,
        component_bytes: binary.len(),
        wat_parse_ms,
        compile_ms,
        instantiation: Series::new(instantiation),
        modes: samples.map(Series::new),
        granted_reads: allowed.data().reads,
        denied_reads: denied.data().reads,
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
        eprintln!("document cost probe: {error:#}");
        std::process::exit(1);
    }
}
