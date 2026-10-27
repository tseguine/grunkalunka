//! Independent bounded Wasmtime/Node-V8 replay of one compiler-emitted core module.
use grunkalunka_compiler::compile;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;
use wasmtime::{Engine, Instance, Module, Store, TypedFunc};

const SOURCE: &str = include_str!("../../../examples/numeric-workflow/gcd.js");
const CASES: [(f64, f64, f64); 8] = [
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
            ("iterations", self.iterations, 1, 10_000),
            ("warmup", self.warmup, 0, 10_000),
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
    let mut seen = BTreeSet::new();
    let mut options = Options::default();
    for pair in args.chunks_exact(2) {
        let name = pair[0].as_str();
        if !seen.insert(name) {
            return Err(format!("duplicate option {name}"));
        }
        let destination = match name {
            "--samples" => &mut options.samples,
            "--iterations" => &mut options.iterations,
            "--warmup" => &mut options.warmup,
            _ => return Err(format!("unknown option {name}")),
        };
        *destination = pair[1]
            .parse::<usize>()
            .map_err(|_| format!("invalid integer for {name}"))?;
    }
    options.validate()?;
    Ok(options)
}

fn milliseconds(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

fn expected_checksum(iterations: usize) -> f64 {
    (0..iterations)
        .map(|index| CASES[index % CASES.len()].2)
        .sum()
}

fn batch(
    store: &mut Store<()>,
    run: &TypedFunc<(f64, f64), f64>,
    iterations: usize,
) -> wasmtime::Result<f64> {
    let mut checksum = 0.0;
    for index in 0..iterations {
        let (a, b, _) = CASES[index % CASES.len()];
        checksum += std::hint::black_box(run.call(&mut *store, (a, b))?);
    }
    Ok(checksum)
}

#[derive(Debug)]
pub struct Report {
    pub options: Options,
    pub wasm_bytes: usize,
    pub source_compile_ms: f64,
    pub module_compile_ms: f64,
    pub instantiation_ms: f64,
    pub first_call_ms: f64,
    pub wasmtime_samples_ms: Vec<f64>,
    pub node_json: String,
}

fn statistics(samples: &[f64]) -> String {
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let middle = sorted.len() / 2;
    let median = if sorted.len().is_multiple_of(2) {
        sorted[middle - 1] / 2.0 + sorted[middle] / 2.0
    } else {
        sorted[middle]
    };
    let samples_json = samples
        .iter()
        .map(|value| format!("{value:.9}"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"samplesMs":[{samples_json}],"minMs":{:.9},"medianMs":{median:.9},"maxMs":{:.9}}}"#,
        sorted[0],
        sorted.last().unwrap()
    )
}

impl Report {
    pub fn json(&self) -> String {
        let mut out = String::new();
        write!(
            out,
            r#"{{"schemaVersion":1,"workload":"core-gcd-two-engine","configuration":{{"samples":{},"iterations":{},"warmup":{}}},"artifact":{{"wasmBytes":{}}},"environment":{{"rustHostOs":"{}","rustHostArch":"{}","wasmtimeDependency":"=50.0.0"}},"sourceCompileMs":{:.9},"wasmtime":{{"moduleCompileMs":{:.9},"instantiationMs":{:.9},"firstCallMs":{:.9},"batches":{} }},"node":{} }}"#,
            self.options.samples,
            self.options.iterations,
            self.options.warmup,
            self.wasm_bytes,
            std::env::consts::OS,
            std::env::consts::ARCH,
            self.source_compile_ms,
            self.module_compile_ms,
            self.instantiation_ms,
            self.first_call_ms,
            statistics(&self.wasmtime_samples_ms),
            self.node_json
        )
        .unwrap();
        out
    }
}

fn measure_node(bytes: &[u8], options: Options) -> wasmtime::Result<String> {
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/bench/core-engine-replay.mjs");
    let mut child = Command::new("node")
        .arg(script)
        .arg(options.samples.to_string())
        .arg(options.iterations.to_string())
        .arg(options.warmup.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| std::io::Error::other("Node stdin unavailable"))?;
        stdin.write_all(bytes)?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        wasmtime::bail!(
            "Node core replay failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let json = String::from_utf8(output.stdout)?.trim().to_string();
    if !json.starts_with('{') || !json.ends_with('}') || !json.contains("\"engine\":\"node-v8\"") {
        wasmtime::bail!("Node core replay returned an invalid report");
    }
    Ok(json)
}

pub fn probe(options: Options) -> wasmtime::Result<Report> {
    options.validate().map_err(std::io::Error::other)?;
    let started = Instant::now();
    let artifact = compile(SOURCE, false).map_err(std::io::Error::other)?;
    let source_compile_ms = milliseconds(started);
    if artifact.wasm.len() > 1024 * 1024 {
        wasmtime::bail!("core Wasm exceeds 1 MiB replay bound");
    }
    let engine = Engine::default();
    let started = Instant::now();
    let module = Module::new(&engine, &artifact.wasm)?;
    let module_compile_ms = milliseconds(started);
    if module.imports().next().is_some() {
        wasmtime::bail!("core Wasm imports are outside this replay profile");
    }
    let mut store = Store::new(&engine, ());
    let started = Instant::now();
    let instance = Instance::new(&mut store, &module, &[])?;
    let instantiation_ms = milliseconds(started);
    let run = instance.get_typed_func::<(f64, f64), f64>(&mut store, "run")?;
    let started = Instant::now();
    let first = run.call(&mut store, (CASES[0].0, CASES[0].1))?;
    let first_call_ms = milliseconds(started);
    if first.to_bits() != CASES[0].2.to_bits() {
        wasmtime::bail!("Wasmtime first-call golden mismatch");
    }
    for &(a, b, expected) in &CASES {
        if run.call(&mut store, (a, b))?.to_bits() != expected.to_bits() {
            wasmtime::bail!("Wasmtime fixed golden mismatch");
        }
    }
    if batch(&mut store, &run, options.warmup)? != expected_checksum(options.warmup) {
        wasmtime::bail!("Wasmtime warmup checksum mismatch");
    }
    let mut samples = Vec::with_capacity(options.samples);
    let expected = expected_checksum(options.iterations);
    for _ in 0..options.samples {
        let started = Instant::now();
        let observed = batch(&mut store, &run, options.iterations)?;
        samples.push(milliseconds(started));
        if observed != expected {
            wasmtime::bail!("Wasmtime measured checksum mismatch");
        }
    }
    // The Node subprocess receives these same compiled bytes through stdin.
    let node_json = measure_node(&artifact.wasm, options)?;
    Ok(Report {
        options,
        wasm_bytes: artifact.wasm.len(),
        source_compile_ms,
        module_compile_ms,
        instantiation_ms,
        first_call_ms,
        wasmtime_samples_ms: samples,
        node_json,
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
        eprintln!("core engine replay: {error:#}");
        std::process::exit(1);
    }
}
