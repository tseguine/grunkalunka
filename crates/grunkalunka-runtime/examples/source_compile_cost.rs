//! P1 repeated in-process source compilation, without compiler internals or speed gates.
use grunkalunka_compiler::{Artifact, compile};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::hint::black_box;
use std::time::Instant;
use wasmtime::{Engine, Instance, Module, Store};

const AFFINE_JS: &str = include_str!("../../../tests/compatibility/fixtures/affine.js");
const AFFINE_TS: &str = include_str!("../../../tests/compatibility/fixtures/affine.ts");
const GCD_JS: &str = include_str!("../../../examples/numeric-workflow/gcd.js");
const AFFINE_CASES: [(f64, f64, f64); 4] = [
    (4.0, 1.0, 3.0),
    (-9.0, 3.0, -5.0),
    (0.0, 0.0, 0.0),
    (2.0, -1.0, 1.0),
];
const GCD_CASES: [(f64, f64, f64); 8] = [
    (1071.0, 462.0, 21.0),
    (84.0, 30.0, 6.0),
    (-84.0, 30.0, 6.0),
    (0.0, 0.0, 0.0),
    (0.0, -9.0, 9.0),
    (13.0, 17.0, 1.0),
    (81.0, 27.0, 27.0),
    (-55.0, -34.0, 1.0),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Workload {
    AffineJs,
    AffineTs,
    GcdJs,
}

impl Workload {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "affine-js" => Some(Self::AffineJs),
            "affine-ts" => Some(Self::AffineTs),
            "gcd-js" => Some(Self::GcdJs),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::AffineJs => "affine-js",
            Self::AffineTs => "affine-ts",
            Self::GcdJs => "gcd-js",
        }
    }

    fn source(self) -> (&'static str, bool) {
        match self {
            Self::AffineJs => (AFFINE_JS, false),
            Self::AffineTs => (AFFINE_TS, true),
            Self::GcdJs => (GCD_JS, false),
        }
    }

    fn cases(self) -> &'static [(f64, f64, f64)] {
        match self {
            Self::AffineJs | Self::AffineTs => &AFFINE_CASES,
            Self::GcdJs => &GCD_CASES,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub workload: Workload,
    pub samples: usize,
    pub iterations: usize,
    pub warmup: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            workload: Workload::AffineJs,
            samples: 5,
            iterations: 5,
            warmup: 3,
        }
    }
}

impl Options {
    fn validate(self) -> Result<(), String> {
        for (name, value, min, max) in [
            ("samples", self.samples, 2, 12),
            ("iterations", self.iterations, 1, 32),
            ("warmup", self.warmup, 0, 32),
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
    let mut options = Options::default();
    let mut seen = BTreeSet::new();
    for pair in args.as_chunks::<2>().0 {
        let name = pair[0].as_str();
        if !seen.insert(name) {
            return Err(format!("duplicate option {name}"));
        }
        if name == "--workload" {
            options.workload =
                Workload::parse(&pair[1]).ok_or_else(|| format!("unknown workload {}", pair[1]))?;
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

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn compile_one(source: &str, ts: bool) -> wasmtime::Result<Artifact> {
    compile(source, ts)
        .map_err(std::io::Error::other)
        .map_err(Into::into)
}

fn verify_golden(artifact: &Artifact, workload: Workload) -> wasmtime::Result<()> {
    let engine = Engine::default();
    let module = Module::new(&engine, &artifact.wasm)?;
    if module.imports().next().is_some() {
        wasmtime::bail!("numeric fixture unexpectedly imports host capabilities");
    }
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    let run = instance.get_typed_func::<(f64, f64), f64>(&mut store, "run")?;
    for &(a, b, expected) in workload.cases() {
        if run.call(&mut store, (a, b))?.to_bits() != expected.to_bits() {
            wasmtime::bail!("{} emitted incorrect source semantics", workload.name());
        }
    }
    Ok(())
}

fn batch(source: &str, ts: bool, iterations: usize) -> wasmtime::Result<Vec<Artifact>> {
    let mut artifacts = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        artifacts.push(black_box(compile_one(source, ts)?));
    }
    Ok(artifacts)
}

fn verify_identical(artifacts: &[Artifact], golden: &Artifact) -> wasmtime::Result<()> {
    for artifact in artifacts {
        if artifact.wasm != golden.wasm || artifact.wat != golden.wat {
            wasmtime::bail!("successive public compiler calls emitted different artifact bytes");
        }
    }
    Ok(())
}

#[derive(Debug)]
pub struct Report {
    pub options: Options,
    pub source_bytes: usize,
    pub wat_bytes: usize,
    pub wasm_bytes: usize,
    pub source_sha256: String,
    pub wat_sha256: String,
    pub wasm_sha256: String,
    pub first_compile_ms: f64,
    pub samples_ms: Vec<f64>,
    pub release_ms: Vec<f64>,
    pub golden_cases: usize,
}

// Keep each observed series in its original measurement order, reporting
// distribution statistics separately without making a performance judgment.
fn statistics(samples: &[f64]) -> (String, f64, f64, f64) {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    let median = if sorted.len().is_multiple_of(2) {
        sorted[middle - 1] / 2.0 + sorted[middle] / 2.0
    } else {
        sorted[middle]
    };
    let ordered = samples
        .iter()
        .map(|value| format!("{value:.9}"))
        .collect::<Vec<_>>()
        .join(",");
    (ordered, sorted[0], median, *sorted.last().unwrap())
}

impl Report {
    pub fn json(&self) -> String {
        let (samples, min, median, max) = statistics(&self.samples_ms);
        let (release_samples, release_min, release_median, release_max) =
            statistics(&self.release_ms);
        let language = if self.options.workload == Workload::AffineTs {
            "ts"
        } else {
            "js"
        };
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        format!(
            r#"{{"schemaVersion":1,"workload":"in-process-source-compile","case":"{case}","language":"{language}","configuration":{{"samples":{sample_count},"iterations":{iterations},"warmup":{warmup}}},"environment":{{"os":"{os}","arch":"{arch}","profile":"{profile}","compilerWorkspaceVersion":"{version}"}},"artifact":{{"sourceBytes":{source_bytes},"sourceSha256":"{source_sha256}","watBytes":{wat_bytes},"watSha256":"{wat_sha256}","wasmBytes":{wasm_bytes},"wasmSha256":"{wasm_sha256}"}},"firstCompileMs":{first:.9},"batches":{{"unit":"milliseconds per batch","samplesMs":[{samples}],"minMs":{min:.9},"medianMs":{median:.9},"maxMs":{max:.9}}},"artifactRelease":{{"unit":"milliseconds per batch","samplesMs":[{release_samples}],"minMs":{release_min:.9},"medianMs":{release_median:.9},"maxMs":{release_max:.9}}},"goldenCases":{golden_cases}}}"#,
            case = self.options.workload.name(),
            sample_count = self.options.samples,
            iterations = self.options.iterations,
            warmup = self.options.warmup,
            os = std::env::consts::OS,
            arch = std::env::consts::ARCH,
            version = env!("CARGO_PKG_VERSION"),
            source_bytes = self.source_bytes,
            wat_bytes = self.wat_bytes,
            wasm_bytes = self.wasm_bytes,
            source_sha256 = self.source_sha256,
            wat_sha256 = self.wat_sha256,
            wasm_sha256 = self.wasm_sha256,
            first = self.first_compile_ms,
            min = min,
            max = max,
            release_samples = release_samples,
            release_min = release_min,
            release_median = release_median,
            release_max = release_max,
            golden_cases = self.golden_cases,
        )
    }
}

pub fn probe(options: Options) -> wasmtime::Result<Report> {
    options.validate().map_err(std::io::Error::other)?;
    let (source, ts) = options.workload.source();
    if source.len() > 16_384 {
        wasmtime::bail!("selected source exceeds the numeric compiler source bound");
    }
    let source_sha256 = sha256(source.as_bytes());

    // One independently executed emitted artifact, before any timing samples.
    let started = Instant::now();
    let golden = compile_one(source, ts)?;
    let first_compile_ms = started.elapsed().as_secs_f64() * 1000.0;
    if golden.wasm.len() < 8 || golden.wasm.len() > 1024 * 1024 {
        wasmtime::bail!("numeric artifact outside the bounded Wasm size profile");
    }
    verify_golden(&golden, options.workload)?;
    // Hash the actual preflight artifact outside timing; later calls must
    // independently reproduce both WAT and Wasm before disposal.
    let wat_sha256 = sha256(golden.wat.as_bytes());
    let wasm_sha256 = sha256(&golden.wasm);
    let warmed = batch(source, ts, options.warmup)?;
    verify_identical(&warmed, &golden)?;
    drop(warmed);

    let mut samples_ms = Vec::with_capacity(options.samples);
    let mut release_ms = Vec::with_capacity(options.samples);
    for _ in 0..options.samples {
        let started = Instant::now();
        // This timing includes each complete public API compile and holding the
        // returned WAT/Wasm artifacts; it excludes golden/result comparison.
        let artifacts = batch(source, ts, options.iterations)?;
        samples_ms.push(started.elapsed().as_secs_f64() * 1000.0);
        verify_identical(&artifacts, &golden)?;
        // Explicitly measure dropping the retained WAT/Wasm outputs, after
        // verification and outside the public compiler call's timing window.
        let started_release = Instant::now();
        drop(artifacts);
        release_ms.push(started_release.elapsed().as_secs_f64() * 1000.0);
    }
    Ok(Report {
        options,
        source_bytes: source.len(),
        wat_bytes: golden.wat.len(),
        wasm_bytes: golden.wasm.len(),
        source_sha256,
        wat_sha256,
        wasm_sha256,
        first_compile_ms,
        samples_ms,
        release_ms,
        golden_cases: options.workload.cases().len(),
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
        eprintln!("source compile cost: {error:#}");
        std::process::exit(1);
    }
}
