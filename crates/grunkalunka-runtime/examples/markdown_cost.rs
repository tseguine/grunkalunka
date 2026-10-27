//! Measure the real Markdown guest through a private WIT component, without performance gates.
#[path = "../tests/fixtures/document-outline/embedding.rs"]
mod embedding;

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::hint::black_box;
use std::path::Path;
use std::time::Instant;
use wasmtime::component::{Component, HasSelf, Linker, Resource};
use wasmtime::{Engine, Store};

wasmtime::component::bindgen!({
    path: "tests/fixtures/document-outline",
    world: "inspector",
    with: { "grunkalunka:markdown/documents.document": BackingDocument },
    imports: { default: trappable },
});
use grunkalunka::markdown::documents::ReadError;

pub struct BackingDocument;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Workload {
    Empty,
    Unicode,
    Dense,
    Wide,
}

impl Workload {
    pub fn name(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Unicode => "unicode",
            Self::Dense => "dense",
            Self::Wide => "wide",
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        match input {
            "empty" => Some(Self::Empty),
            "unicode" => Some(Self::Unicode),
            "dense" => Some(Self::Dense),
            "wide" => Some(Self::Wide),
            _ => None,
        }
    }

    pub fn source(self) -> String {
        match self {
            Self::Empty => String::new(),
            Self::Unicode => "# Root\r\n### 子😀\n# End\n".into(),
            Self::Dense => {
                let mut text = String::new();
                for index in 0..24 {
                    writeln!(text, "# Heading {index}").unwrap();
                }
                text
            }
            Self::Wide => format!("# {}", "é".repeat(1800)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub samples: usize,
    pub iterations: usize,
    pub warmup: usize,
    pub workload: Workload,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            samples: 5,
            iterations: 20,
            warmup: 5,
            workload: Workload::Unicode,
        }
    }
}

impl Options {
    fn validate(self) -> Result<(), String> {
        for (name, value, min, max) in [
            ("samples", self.samples, 2, 12),
            ("iterations", self.iterations, 1, 100),
            ("warmup", self.warmup, 0, 100),
        ] {
            if !(min..=max).contains(&value) {
                return Err(format!("{name} must be within {min}..={max}"));
            }
        }
        let source = self.workload.source();
        if source.len() > 4096 {
            return Err("workload exceeds guest's UTF-8 source quota".into());
        }
        Ok(())
    }
}

pub fn parse_options(args: &[String]) -> Result<Options, String> {
    if !args.len().is_multiple_of(2) {
        return Err("expected unique --option value pairs".into());
    }
    let mut options = Options::default();
    let mut seen = BTreeSet::new();
    for pair in args.as_chunks::<2>().0 {
        let name = pair[0].as_str();
        if !seen.insert(name) {
            return Err(format!("duplicate option {name}"));
        }
        if name == "--workload" {
            options.workload = Workload::parse(&pair[1])
                .ok_or_else(|| format!("unknown workload {}", pair[1]))?;
            continue;
        }
        let slot = match name {
            "--samples" => &mut options.samples,
            "--iterations" => &mut options.iterations,
            "--warmup" => &mut options.warmup,
            _ => return Err(format!("unknown option {name}")),
        };
        *slot = pair[1]
            .parse::<usize>()
            .map_err(|_| format!("invalid integer for {name}"))?;
    }
    options.validate()?;
    Ok(options)
}

#[derive(Default)]
struct Host {
    allowed: bool,
    source: String,
    attempts: usize,
    reads: usize,
}

impl grunkalunka::markdown::documents::Host for Host {}
impl grunkalunka::markdown::documents::HostDocument for Host {
    fn read(
        &mut self,
        document: Resource<Document>,
    ) -> wasmtime::Result<Result<String, ReadError>> {
        self.attempts += 1;
        if !self.allowed {
            return Ok(Err(ReadError::Denied));
        }
        if document.rep() != 7 {
            return Ok(Err(ReadError::UnknownGrant));
        }
        if self.source.len() > 4096 {
            return Ok(Err(ReadError::TooLarge));
        }
        self.reads += 1;
        Ok(Ok(self.source.clone()))
    }

    fn drop(&mut self, _: Resource<Document>) -> wasmtime::Result<()> {
        wasmtime::bail!("the Markdown measurement never transfers owned resources")
    }
}

fn instantiate(
    engine: &Engine,
    component: &Component,
    linker: &Linker<Host>,
    allowed: bool,
    source: &str,
) -> wasmtime::Result<(Store<Host>, Inspector)> {
    let host = Host {
        allowed,
        source: source.into(),
        ..Host::default()
    };
    let mut store = Store::new(engine, host);
    let instance = Inspector::instantiate(&mut store, component, linker)?;
    Ok((store, instance))
}

fn milliseconds(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn rows(headings: &[Heading]) -> String {
    let mut output = String::new();
    for entry in headings {
        write!(
            output,
            "{} {} {} {} {} {} ",
            entry.level,
            entry.start,
            entry.end,
            entry.section_end,
            entry.parent.map_or(-1, i64::from),
            entry.text.len()
        )
        .unwrap();
        for byte in entry.text.bytes() {
            write!(output, "{byte:02x}").unwrap();
        }
        output.push('\n');
    }
    output
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(output, "{byte:02x}").unwrap();
    }
    output
}

fn validate_projection(workload: Workload, headings: &[Heading]) -> wasmtime::Result<()> {
    let valid = match workload {
        Workload::Empty => headings.is_empty(),
        Workload::Unicode => {
            headings.len() == 3
                && headings[0].text == "Root"
                && headings[1].text == "子😀"
                && headings[1].parent == Some(0)
                && headings[2].text == "End"
        }
        Workload::Dense => {
            headings.len() == 24
                && headings
                    .iter()
                    .enumerate()
                    .all(|(index, heading)| heading.text == format!("Heading {index}"))
        }
        Workload::Wide => headings.len() == 1 && headings[0].text == "é".repeat(1800),
    };
    if !valid {
        wasmtime::bail!("Markdown heading projection mismatched the selected fixture");
    }
    let source_bytes = workload.source().len();
    for heading in headings {
        if heading.start > heading.end
            || heading.end as usize > source_bytes
            || heading.section_end < heading.end
            || heading.section_end as usize > source_bytes
        {
            wasmtime::bail!("Markdown returned an invalid source byte range");
        }
    }
    Ok(())
}

fn batch(
    store: &mut Store<Host>,
    inspector: &Inspector,
    iterations: usize,
) -> wasmtime::Result<()> {
    for _ in 0..iterations {
        let _ = black_box(inspector.call_inspect(&mut *store, Resource::new_borrow(7))?);
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
    pub fn new(samples_ms: Vec<f64>) -> Self {
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
            .map(|value| format!("{value:.9}"))
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
    pub guest_bytes: usize,
    pub component_bytes: usize,
    pub embed_ms: f64,
    pub compile_ms: f64,
    pub instantiation: Series,
    pub granted: Series,
    pub denied: Series,
    pub granted_reads: usize,
    pub denied_reads: usize,
    pub granted_cleanups: usize,
    pub denied_cleanups: usize,
    pub source_hex: String,
    pub outline_hex: String,
}

impl Report {
    pub fn json(&self) -> String {
        let mut out = String::from("{\"schemaVersion\":1,\"workload\":\"markdown-wit-inspection\",");
        write!(
            out,
            "\"configuration\":{{\"case\":\"{}\",\"samples\":{},\"iterations\":{},\"warmup\":{}}},",
            self.options.workload.name(),
            self.options.samples,
            self.options.iterations,
            self.options.warmup
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
            "\"artifact\":{{\"guestBytes\":{},\"componentBytes\":{}}},",
            self.guest_bytes, self.component_bytes
        )
        .unwrap();
        out.push_str("\"timings\":{\"unit\":\"milliseconds per batch\",\"order\":\"alternating\",");
        write!(
            out,
            "\"embedMs\":{:.9},\"componentCompileMs\":{:.9},\"instantiation\":{},\"granted\":{},\"denied\":{}",
            self.embed_ms,
            self.compile_ms,
            self.instantiation.json(),
            self.granted.json(),
            self.denied.json()
        )
        .unwrap();
        out.push_str("},\"outcomes\":{");
        write!(
            out,
            "\"grantedBackingReads\":{},\"deniedBackingReads\":{},\"grantedCleanups\":{},\"deniedCleanups\":{}",
            self.granted_reads,
            self.denied_reads,
            self.granted_cleanups,
            self.denied_cleanups
        )
        .unwrap();
        out.push_str("},\"fixture\":{");
        write!(
            out,
            "\"sourceHex\":\"{}\",\"outlineHex\":\"{}\"",
            self.source_hex, self.outline_hex
        )
        .unwrap();
        out.push_str("}}");
        out
    }
}

pub fn probe(guest_file: &Path, options: Options) -> wasmtime::Result<Report> {
    options.validate().map_err(std::io::Error::other)?;
    let guest = std::fs::read(guest_file)?;
    if guest.len() > 1024 * 1024 {
        wasmtime::bail!("Markdown guest exceeds 1 MiB measurement bound");
    }
    let template = include_str!("../tests/fixtures/document-outline/inspector.wat.in");
    let source = options.workload.source();
    let engine = Engine::default();

    let started = Instant::now();
    let component_bytes = embedding::embed_guest(template, &guest)?;
    let embed_ms = milliseconds(started);
    let started = Instant::now();
    let component = Component::from_binary(&engine, &component_bytes)?;
    let compile_ms = milliseconds(started);

    let mut linker = Linker::new(&engine);
    Inspector::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)?;

    let mut instantiation = Vec::with_capacity(options.samples);
    for _ in 0..options.samples {
        let started = Instant::now();
        let instance = instantiate(&engine, &component, &linker, true, &source)?;
        instantiation.push(milliseconds(started));
        drop(instance);
    }

    let (mut granted_store, granted_guest) =
        instantiate(&engine, &component, &linker, true, &source)?;
    let (mut denied_store, denied_guest) =
        instantiate(&engine, &component, &linker, false, &source)?;

    // Validate typed results and error distinctions before sampling.
    let original = granted_guest.call_inspect(&mut granted_store, Resource::new_borrow(7))??;
    validate_projection(options.workload, &original)?;
    if denied_guest.call_inspect(&mut denied_store, Resource::new_borrow(7))?
        != Err(InspectError::Denied)
    {
        wasmtime::bail!("denied profile was not a structured policy error");
    }
    if granted_guest.call_inspect(&mut granted_store, Resource::new_borrow(8))?
        != Err(InspectError::UnknownGrant)
    {
        wasmtime::bail!("unknown grant was not rejected");
    }
    if granted_store.data().reads != 1 || denied_store.data().reads != 0 {
        wasmtime::bail!("policy checks read unauthorized backing data");
    }

    batch(&mut granted_store, &granted_guest, options.warmup)?;
    batch(&mut denied_store, &denied_guest, options.warmup)?;

    let mut granted_samples = Vec::with_capacity(options.samples);
    let mut denied_samples = Vec::with_capacity(options.samples);
    for sample in 0..options.samples {
        if sample.is_multiple_of(2) {
            let started = Instant::now();
            batch(&mut granted_store, &granted_guest, options.iterations)?;
            granted_samples.push(milliseconds(started));
            let started = Instant::now();
            batch(&mut denied_store, &denied_guest, options.iterations)?;
            denied_samples.push(milliseconds(started));
        } else {
            let started = Instant::now();
            batch(&mut denied_store, &denied_guest, options.iterations)?;
            denied_samples.push(milliseconds(started));
            let started = Instant::now();
            batch(&mut granted_store, &granted_guest, options.iterations)?;
            granted_samples.push(milliseconds(started));
        }
    }

    let repeated = options.warmup + options.samples * options.iterations;
    let granted_cleanups = granted_guest.call_cleanup_count(&mut granted_store)? as usize;
    let denied_cleanups = denied_guest.call_cleanup_count(&mut denied_store)? as usize;
    if granted_store.data().attempts != repeated + 2
        || granted_store.data().reads != repeated + 1
        || denied_store.data().attempts != repeated + 1
        || denied_store.data().reads != 0
        || granted_cleanups != repeated + 2
        || denied_cleanups != repeated + 1
    {
        wasmtime::bail!("Markdown read or post-return count changed during measurement");
    }

    // Lifting produces host-owned records that survive source instance disposal.
    drop(granted_store);
    drop(denied_store);
    let projection = rows(&original);
    validate_projection(options.workload, &original)?;

    Ok(Report {
        options,
        guest_bytes: guest.len(),
        component_bytes: component_bytes.len(),
        embed_ms,
        compile_ms,
        instantiation: Series::new(instantiation),
        granted: Series::new(granted_samples),
        denied: Series::new(denied_samples),
        granted_reads: repeated + 1,
        denied_reads: 0,
        granted_cleanups,
        denied_cleanups,
        source_hex: hex(source.as_bytes()),
        outline_hex: hex(projection.as_bytes()),
    })
}

fn run_main() -> wasmtime::Result<()> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let Some(path) = args.first() else {
        wasmtime::bail!("expected outline_guest.wasm path");
    };
    let rest = args[1..]
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let options = parse_options(&rest).map_err(std::io::Error::other)?;
    println!("{}", probe(Path::new(path), options)?.json());
    Ok(())
}

fn main() {
    if let Err(error) = run_main() {
        eprintln!("Markdown cost probe: {error:#}");
        std::process::exit(1);
    }
}
