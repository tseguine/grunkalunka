//! P1 replay correctness and bounded-input checks, never timing thresholds.
#[allow(dead_code)] // The example also exposes a binary entry point.
#[path = "../examples/core_engine_cost.rs"]
mod core_engine_cost;

use core_engine_cost::{Options, parse_options, probe};

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[test]
fn options_reject_invalid_or_unbounded_work() {
    assert_eq!(parse_options(&[]).unwrap(), Options::default());
    assert_eq!(
        parse_options(&args(&[
            "--samples",
            "2",
            "--iterations",
            "3",
            "--warmup",
            "0"
        ]))
        .unwrap(),
        Options {
            samples: 2,
            iterations: 3,
            warmup: 0
        }
    );
    for invalid in [
        vec!["--samples"],
        vec!["--samples", "1"],
        vec!["--samples", "21"],
        vec!["--iterations", "0"],
        vec!["--iterations", "10001"],
        vec!["--warmup", "10001"],
        vec!["--warmup", "-1"],
        vec!["--samples", "3", "--samples", "4"],
        vec!["--bogus", "2"],
    ] {
        assert!(parse_options(&args(&invalid)).is_err(), "{invalid:?}");
    }
}

#[test]
fn one_compiled_artifact_replays_with_independent_goldens_in_both_engines() {
    let options = Options {
        samples: 2,
        iterations: 3,
        warmup: 1,
    };
    let report = probe(options).unwrap();
    assert_eq!(report.options, options);
    assert!(report.wasm_bytes > 0 && report.wasm_bytes < 1024 * 1024);
    for duration in [
        report.source_compile_ms,
        report.module_compile_ms,
        report.instantiation_ms,
        report.first_call_ms,
    ] {
        assert!(duration.is_finite() && duration >= 0.0);
    }
    assert_eq!(report.wasmtime_samples_ms.len(), options.samples);
    assert!(
        report
            .wasmtime_samples_ms
            .iter()
            .all(|duration| duration.is_finite() && *duration >= 0.0)
    );
    let output = report.json();
    for expected in [
        "\"schemaVersion\":1",
        "\"workload\":\"core-gcd-two-engine\"",
        "\"engine\":\"node-v8\"",
        "\"sha256\":",
        "\"wasmtime\":",
        "\"node\":",
        "\"checksum\":33",
        "\"goldenCases\":8",
        "\"samplesMs\":",
    ] {
        assert!(output.contains(expected), "missing {expected}");
    }
    // Node receives the compiled binary on stdin; neither engine recompiles source.
    assert_eq!(output.matches("\"samplesMs\":").count(), 2);
}

#[test]
fn direct_probe_calls_cannot_skip_bounds() {
    assert!(
        probe(Options {
            samples: 0,
            ..Options::default()
        })
        .is_err()
    );
}
