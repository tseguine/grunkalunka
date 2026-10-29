//! P1 replay correctness and bounded-input checks, never timing thresholds.
#[allow(dead_code)] // The example also exposes a binary entry point.
#[path = "../examples/core_engine_cost.rs"]
mod core_engine_cost;

use core_engine_cost::{Options, parse_options, probe};
use sha2::{Digest, Sha256};

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
    let source = include_str!("../../../examples/numeric-workflow/gcd.js");
    let artifact = grunkalunka_compiler::compile(source, false).unwrap();
    assert_eq!(report.wasm_bytes, artifact.wasm.len());
    assert_eq!(
        report.wasm_sha256,
        format!("{:x}", Sha256::digest(&artifact.wasm))
    );
    for duration in [
        report.source_compile_ms,
        report.module_compile_ms,
        report.instantiation_ms,
        report.first_call_ms,
    ] {
        assert!(duration.is_finite() && duration >= 0.0);
    }
    assert_eq!(report.wasmtime_samples_ms.len(), options.samples);
    assert_eq!(report.fresh_instantiation_ms.len(), options.samples);
    assert_eq!(report.fresh_first_call_ms.len(), options.samples);
    assert_eq!(report.engine_creation_ms.len(), options.samples);
    assert_eq!(report.store_creation_ms.len(), options.samples);
    assert_eq!(report.node_fresh_process_ms.len(), options.samples);
    assert!(
        report
            .fresh_instantiation_ms
            .iter()
            .chain(report.fresh_first_call_ms.iter())
            .chain(report.engine_creation_ms.iter())
            .chain(report.store_creation_ms.iter())
            .chain(report.node_fresh_process_ms.iter())
            .all(|duration| duration.is_finite() && *duration >= 0.0)
    );
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        let memory = report
            .rust_process_memory
            .snapshots
            .expect("Linux/Windows CI must expose real process memory");
        assert!(
            memory
                .iter()
                .all(|snapshot| snapshot.peak_rss_bytes >= snapshot.rss_bytes)
        );
        assert!(memory[4].peak_rss_bytes >= memory[0].peak_rss_bytes);
        let label = if cfg!(target_os = "linux") {
            "linux-proc-status"
        } else {
            "windows-psapi-working-set"
        };
        assert!(report.json().contains(&format!("\"source\":\"{label}\"")));
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    assert!(report.rust_process_memory.snapshots.is_none());
    assert!(
        report
            .wasmtime_samples_ms
            .iter()
            .all(|duration| duration.is_finite() && *duration >= 0.0)
    );
    let output = report.json();
    let expected_rust = format!(
        r#""artifact":{{"wasmBytes":{},"sha256":"{}"}}"#,
        report.wasm_bytes, report.wasm_sha256
    );
    let expected_node = format!(
        r#""artifact":{{"bytes":{},"sha256":"{}","imports":0}}"#,
        report.wasm_bytes, report.wasm_sha256
    );
    assert!(
        output.contains(&expected_rust),
        "Rust artifact receipt mismatch"
    );
    assert!(
        output.contains(&expected_node),
        "Node artifact receipt mismatch"
    );
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
        "\"memory\":",
        "\"processMemory\":",
        "\"scope\":\"entire-rust-process\"",
        "\"scope\":\"entire-node-process\"",
        "\"rssBytes\":",
        "\"afterModule\":",
        "\"afterBatches\":",
        "\"freshInstances\":{\"count\":2,\"instantiation\":",
        "\"hostSetup\":{\"count\":2,\"engineCreation\":",
        "\"nodeFreshProcess\":{\"count\":2,\"boundary\":\"spawn-through-validated-exit-including-stdin-wasm-transfer\"",
    ] {
        assert!(output.contains(expected), "missing {expected}");
    }
    // Node receives the compiled binary on stdin; neither engine recompiles source.
    assert_eq!(output.matches("\"samplesMs\":").count(), 9);
    assert_eq!(output.matches("\"freshInstances\":").count(), 2);
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
