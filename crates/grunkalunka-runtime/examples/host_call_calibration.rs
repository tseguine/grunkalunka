//! P1 scalar core-Wasm guest/import calibration; not an isolated host-call tax.
use std::{collections::BTreeSet, hint::black_box, time::Instant};
use wasmtime::{Caller, Engine, Linker, Module, Store, TypedFunc};

const WAT: &str = r#"
(module
  (import "host" "tick" (func $tick (param i32) (result i32)))
  (func (export "guest-only") (param i32) (result i32)
    local.get 0
    i32.const 1
    i32.add)
  (func (export "guest-to-host") (param i32) (result i32)
    local.get 0
    call $tick)
)"#;

// Independently specified goldens cover signed wraparound.
const CASES: [(i32, i32); 6] = [
    (0, 1),
    (-1, 0),
    (i32::MAX, i32::MIN),
    (i32::MIN, i32::MIN + 1),
    (1024, 1025),
    (-2048, -2047),
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Counters {
    calls: usize,
    input_sum: i64,
}

fn host_tick(counts: &mut Counters, input: i32) -> i32 {
    counts.calls += 1;
    counts.input_sum += i64::from(input);
    input.wrapping_add(1)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub samples: usize,
    pub iterations: usize,
    pub warmup: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            samples: 7,
            iterations: 1000,
            warmup: 100,
        }
    }
}

impl Options {
    fn validate(self) -> Result<(), String> {
        for (name, value, min, max) in [
            ("samples", self.samples, 2, 12),
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
    let mut seen = BTreeSet::new();
    let mut options = Options::default();
    for pair in args.as_chunks::<2>().0 {
        let name = pair[0].as_str();
        if !seen.insert(name) {
            return Err(format!("duplicate option {name}"));
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

fn expected(iterations: usize) -> (i64, i64) {
    let mut inputs = 0_i64;
    let mut outputs = 0_i64;
    for i in 0..iterations {
        let (input, output) = CASES[i % CASES.len()];
        inputs += i64::from(input);
        outputs += i64::from(output);
    }
    (inputs, outputs)
}

fn native_batch(counts: &mut Counters, iterations: usize) -> i64 {
    let mut checksum = 0_i64;
    for i in 0..iterations {
        let input = black_box(CASES[i % CASES.len()].0);
        checksum += i64::from(black_box(host_tick(counts, input)));
    }
    checksum
}

fn wasm_batch(
    store: &mut Store<Counters>,
    func: &TypedFunc<i32, i32>,
    iterations: usize,
) -> wasmtime::Result<i64> {
    let mut checksum = 0_i64;
    for i in 0..iterations {
        let input = black_box(CASES[i % CASES.len()].0);
        checksum += i64::from(black_box(func.call(&mut *store, input)?));
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
        sorted.sort_by(f64::total_cmp);
        let middle = sorted.len() / 2;
        let median_ms = if sorted.len().is_multiple_of(2) {
            sorted[middle - 1] / 2.0 + sorted[middle] / 2.0
        } else {
            sorted[middle]
        };
        Self {
            samples_ms,
            min_ms: sorted[0],
            median_ms,
            max_ms: *sorted.last().unwrap(),
        }
    }

    fn json(&self) -> String {
        let samples = self
            .samples_ms
            .iter()
            .map(|ms| format!("{ms:.9}"))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"{{"samplesMs":[{samples}],"minMs":{:.9},"medianMs":{:.9},"maxMs":{:.9}}}"#,
            self.min_ms, self.max_ms
        )
    }
}

#[derive(Debug)]
pub struct Report {
    pub options: Options,
    pub wasm_bytes: usize,
    pub modes: [Series; 3],
    pub native_calls: usize,
    pub host_calls: usize,
    pub native_input_sum: i64,
    pub host_input_sum: i64,
}

impl Report {
    pub fn json(&self) -> String {
        format!(
            r#"{{"schemaVersion":1,"workload":"core-wasm-scalar-host-call-calibration","configuration":{{"samples":{},"iterations":{},"warmup":{}}},"environment":{{"os":"{}","arch":"{}","wasmtimeDependency":"=50.0.0"}},"artifact":{{"wasmBytes":{}}},"batches":{{"unit":"milliseconds per batch","order":"alternate forward/reverse","native":{},"guestOnly":{},"guestToHost":{}}},"outcomes":{{"nativeCalls":{},"hostCalls":{},"nativeInputSum":{},"hostInputSum":{}}}}}"#,
            self.options.samples,
            self.options.iterations,
            self.options.warmup,
            std::env::consts::OS,
            std::env::consts::ARCH,
            self.wasm_bytes,
            self.modes[0].json(),
            self.modes[1].json(),
            self.modes[2].json(),
            self.native_calls,
            self.host_calls,
            self.native_input_sum,
            self.host_input_sum
        )
    }
}

pub fn probe(options: Options) -> wasmtime::Result<Report> {
    options.validate().map_err(std::io::Error::other)?;
    let bytes = wat::parse_str(WAT)?;
    if bytes.len() > 1024 {
        wasmtime::bail!("fixed scalar module exceeded the 1 KiB bound");
    }
    let engine = Engine::default();
    let module = Module::from_binary(&engine, &bytes)?;
    let mut linker = Linker::new(&engine);
    linker.func_wrap(
        "host",
        "tick",
        |mut caller: Caller<'_, Counters>, input: i32| host_tick(caller.data_mut(), input),
    )?;
    let mut store = Store::new(&engine, Counters::default());
    let instance = linker.instantiate(&mut store, &module)?;
    let guest = instance.get_typed_func::<i32, i32>(&mut store, "guest-only")?;
    let imported = instance.get_typed_func::<i32, i32>(&mut store, "guest-to-host")?;
    let mut native = Counters::default();

    // Independent fixed values before timed intervals.
    for (input, golden) in CASES {
        if host_tick(&mut native, input) != golden
            || guest.call(&mut store, input)? != golden
            || imported.call(&mut store, input)? != golden
        {
            wasmtime::bail!("native/guest/imported scalar golden mismatch");
        }
    }
    let (warm_input, warm_output) = expected(options.warmup);
    if native_batch(&mut native, options.warmup) != warm_output
        || wasm_batch(&mut store, &guest, options.warmup)? != warm_output
        || wasm_batch(&mut store, &imported, options.warmup)? != warm_output
    {
        wasmtime::bail!("warmup checksum mismatch");
    }

    let (batch_input, batch_output) = expected(options.iterations);
    let mut samples: [Vec<f64>; 3] = std::array::from_fn(|_| Vec::with_capacity(options.samples));
    for sample in 0..options.samples {
        let order = if sample.is_multiple_of(2) {
            [0, 1, 2]
        } else {
            [2, 1, 0]
        };
        for mode in order {
            let started = Instant::now();
            let checksum = match mode {
                0 => native_batch(&mut native, options.iterations),
                1 => wasm_batch(&mut store, &guest, options.iterations)?,
                2 => wasm_batch(&mut store, &imported, options.iterations)?,
                _ => unreachable!(),
            };
            let elapsed = started.elapsed().as_secs_f64() * 1000.0;
            if checksum != batch_output {
                wasmtime::bail!("scalar batch checksum mismatch in mode {mode}");
            }
            samples[mode].push(elapsed);
        }
    }

    let total_calls = CASES.len() + options.warmup + options.samples * options.iterations;
    let fixed_inputs = CASES.iter().map(|pair| i64::from(pair.0)).sum::<i64>();
    let total_inputs = fixed_inputs + warm_input + batch_input * options.samples as i64;
    if native.calls != total_calls
        || store.data().calls != total_calls
        || native.input_sum != total_inputs
        || store.data().input_sum != total_inputs
    {
        wasmtime::bail!("host callback counts/input sums changed during measurement");
    }
    Ok(Report {
        options,
        wasm_bytes: bytes.len(),
        modes: samples.map(Series::new),
        native_calls: native.calls,
        host_calls: store.data().calls,
        native_input_sum: native.input_sum,
        host_input_sum: store.data().input_sum,
    })
}

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = (|| -> wasmtime::Result<Report> {
        let options = parse_options(&args).map_err(std::io::Error::other)?;
        probe(options)
    })();
    match result {
        Ok(report) => println!("{}", report.json()),
        Err(error) => {
            eprintln!("host-call calibration: {error:#}");
            std::process::exit(1);
        }
    }
}
