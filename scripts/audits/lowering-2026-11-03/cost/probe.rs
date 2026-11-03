use grunkalunka_compiler::Artifact;
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::Instant;

const THREAD_STACK_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone)]
struct Case {
    id: String,
    profile: String,
    language: String,
    expected: String,
    source: String,
}

enum Outcome {
    Artifact(Artifact),
    Error(String),
}

impl Outcome {
    fn status(&self) -> &'static str {
        match self {
            Self::Artifact(_) => "ok",
            Self::Error(_) => "error",
        }
    }

    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Artifact(a), Self::Artifact(b)) => a.wat == b.wat && a.wasm == b.wasm,
            (Self::Error(a), Self::Error(b)) => a == b,
            _ => false,
        }
    }

    fn verify_files(&self, directory: &Path, id: &str) -> Result<(), String> {
        let stem = directory.join(id);
        let equal = match self {
            Self::Artifact(a) => {
                fs::read(stem.with_extension("wat")).map_err(|e| e.to_string())? == a.wat.as_bytes()
                    && fs::read(stem.with_extension("wasm")).map_err(|e| e.to_string())? == a.wasm
            }
            Self::Error(error) => {
                fs::read(stem.with_extension("error")).map_err(|e| e.to_string())?
                    == error.as_bytes()
            }
        };
        if equal {
            Ok(())
        } else {
            Err(format!("frozen output differs: {id}"))
        }
    }

    fn write_files(&self, directory: &Path, id: &str) -> Result<(), String> {
        let stem = directory.join(id);
        match self {
            Self::Artifact(a) => {
                fs::write(stem.with_extension("wat"), &a.wat).map_err(|e| e.to_string())?;
                fs::write(stem.with_extension("wasm"), &a.wasm).map_err(|e| e.to_string())?;
            }
            Self::Error(error) => {
                fs::write(stem.with_extension("error"), error).map_err(|e| e.to_string())?
            }
        }
        Ok(())
    }
}

fn compile_one(case: &Case) -> Outcome {
    let ts = case.language == "ts";
    let result = match case.profile.as_str() {
        "numeric" => grunkalunka_compiler::compile(black_box(&case.source), ts),
        "async" => grunkalunka_compiler::compile_async(black_box(&case.source), ts),
        _ => unreachable!("validated profile"),
    };
    match result {
        Ok(artifact) => Outcome::Artifact(artifact),
        Err(error) => Outcome::Error(error),
    }
}

fn cases(path: &Path, selected: Option<&str>) -> Result<Vec<Case>, String> {
    let mut result = Vec::new();
    for line in fs::read_to_string(path).map_err(|e| e.to_string())?.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 5 {
            return Err("manifest must have five columns".into());
        }
        let [id, profile, language, expected, path] = fields.as_slice() else {
            unreachable!()
        };
        if !id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
            || !matches!(*profile, "numeric" | "async")
            || !matches!(*language, "js" | "ts")
            || !matches!(*expected, "ok" | "error")
            || result.iter().any(|case: &Case| case.id == *id)
        {
            return Err(format!("invalid case row: {id}"));
        }
        if selected.is_some_and(|wanted| *id != wanted) {
            continue;
        }
        result.push(Case {
            id: id.to_string(),
            profile: profile.to_string(),
            language: language.to_string(),
            expected: expected.to_string(),
            source: fs::read_to_string(path).map_err(|e| e.to_string())?,
        });
    }
    Ok(result)
}

fn memory() -> Result<(u64, u64), String> {
    let status = fs::read_to_string("/proc/self/status").map_err(|e| e.to_string())?;
    let field = |name: &str| -> Result<u64, String> {
        let lines: Vec<_> = status
            .lines()
            .filter_map(|line| line.strip_prefix(name))
            .collect();
        if lines.len() != 1 {
            return Err(format!("expected one {name}"));
        }
        let values: Vec<_> = lines[0].split_whitespace().collect();
        if values.len() != 2 || values[1] != "kB" {
            return Err(format!("invalid {name}"));
        }
        values[0]
            .parse::<u64>()
            .map_err(|e| e.to_string())?
            .checked_mul(1024)
            .ok_or("memory overflow".into())
    };
    let rss = field("VmRSS:")?;
    let peak = field("VmHWM:")?;
    if peak < rss {
        return Err("VmHWM below VmRSS".into());
    }
    Ok((rss, peak))
}

fn batch(case: &Case, iterations: usize) -> Vec<Outcome> {
    let mut outcomes = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        outcomes.push(black_box(compile_one(case)));
    }
    outcomes
}

fn verify_batch(outcomes: &[Outcome], golden: &Outcome) -> Result<(), String> {
    if outcomes.iter().all(|outcome| outcome.same(golden)) {
        Ok(())
    } else {
        Err("repeated compilation changed output".into())
    }
}

fn measure(
    case: Case,
    frozen: PathBuf,
    samples: usize,
    iterations: usize,
    warmup: usize,
) -> Result<String, String> {
    let before = memory()?;
    let start = Instant::now();
    let golden = black_box(compile_one(&case));
    let first_ns = start.elapsed().as_nanos();
    let after_first = memory()?;
    if golden.status() != case.expected {
        return Err(format!("unexpected status for {}", case.id));
    }
    golden.verify_files(&frozen, &case.id)?;
    let after_validation = memory()?;
    let warmed = batch(&case, warmup);
    verify_batch(&warmed, &golden)?;
    drop(warmed);
    let after_warmup = memory()?;
    let mut compile_ns = Vec::with_capacity(samples);
    let mut release_ns = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        let outcomes = batch(&case, iterations);
        compile_ns.push(start.elapsed().as_nanos());
        verify_batch(&outcomes, &golden)?;
        let start = Instant::now();
        drop(outcomes);
        release_ns.push(start.elapsed().as_nanos());
    }
    let after_batches = memory()?;
    let observations = [
        before,
        after_first,
        after_validation,
        after_warmup,
        after_batches,
    ];
    if observations.windows(2).any(|w| w[1].1 < w[0].1) {
        return Err("peak RSS decreased".into());
    }
    let phases = [
        "beforeFirstCompile",
        "afterFirstCompile",
        "afterFrozenValidation",
        "afterWarmup",
        "afterBatches",
    ];
    let memory_json = phases
        .iter()
        .zip(observations)
        .map(|(phase, (rss, peak))| {
            format!("\"{phase}\":{{\"rssBytes\":{rss},\"peakRssBytes\":{peak}}}")
        })
        .collect::<Vec<_>>()
        .join(",");
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    Ok(format!(
        "{{\"schemaVersion\":1,\"case\":\"{}\",\"sourceProfile\":\"{}\",\"language\":\"{}\",\"outcome\":\"{}\",\"buildProfile\":\"{profile}\",\"threadStackBytes\":{THREAD_STACK_BYTES},\"samples\":{samples},\"iterations\":{iterations},\"warmup\":{warmup},\"firstCompileNs\":{first_ns},\"compileBatchNs\":{compile_ns:?},\"releaseBatchNs\":{release_ns:?},\"processMemory\":{{\"source\":\"linux-proc-status\",{memory_json}}},\"frozenOutputMatched\":true}}",
        case.id,
        case.profile,
        case.language,
        golden.status()
    ))
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("freeze") if args.len() == 3 => {
            let directory = PathBuf::from(&args[2]);
            fs::create_dir(&directory).map_err(|e| e.to_string())?;
            let mut summary = String::from("id\tprofile\tlanguage\texpected\tactual\n");
            for case in cases(Path::new(&args[1]), None)? {
                let input = case.clone();
                let outcome = std::thread::Builder::new().stack_size(THREAD_STACK_BYTES)
                    .spawn(move || compile_one(&input)).map_err(|e| e.to_string())?
                    .join().map_err(|_| format!("compiler panicked: {}", case.id))?;
                if outcome.status() != case.expected { return Err(format!("unexpected frozen status: {}", case.id)); }
                outcome.write_files(&directory, &case.id)?;
                summary.push_str(&format!("{}\t{}\t{}\t{}\t{}\n",case.id,case.profile,case.language,case.expected,outcome.status()));
            }
            fs::write(directory.join("summary.tsv"), summary).map_err(|e| e.to_string())?;
        }
        Some("measure") if args.len() == 8 => {
            let case = cases(Path::new(&args[1]), Some(&args[2]))?.into_iter().next().ok_or("unknown case")?;
            let frozen = PathBuf::from(&args[3]);
            let number = |index: usize, low: usize, high: usize| -> Result<usize, String> {
                let value = args[index].parse::<usize>().map_err(|e| e.to_string())?;
                if (low..=high).contains(&value) { Ok(value) } else { Err("sample controls out of range".into()) }
            };
            let samples = number(4, 2, 12)?;
            let iterations = number(5, 1, 32)?;
            let warmup = number(6, 0, 32)?;
            if args[7] != "linux-proc-status" { return Err("explicit memory source required".into()); }
            let result = std::thread::Builder::new().stack_size(THREAD_STACK_BYTES)
                .spawn(move || measure(case, frozen, samples, iterations, warmup))
                .map_err(|e| e.to_string())?.join().map_err(|_| "compiler worker panicked")??;
            println!("{result}");
        }
        _ => return Err("expected freeze MANIFEST NEW_OUTPUT or measure MANIFEST ID FROZEN SAMPLES ITERATIONS WARMUP linux-proc-status".into()),
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("lowering cost: {error}");
        std::process::exit(1);
    }
}
