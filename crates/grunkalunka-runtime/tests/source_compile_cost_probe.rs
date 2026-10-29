//! Repeat real public compiler builds on bounded JS/TS fixtures without timing gates.
#[allow(dead_code)] // The included example also provides an executable entry point.
#[path = "../examples/source_compile_cost.rs"]
mod source_compile_cost;

use sha2::{Digest, Sha256};
use source_compile_cost::{Options, Workload, parse_options, probe};

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn fixture(workload: Workload) -> (&'static str, bool) {
    match workload {
        Workload::AffineJs => (
            include_str!("../../../tests/compatibility/fixtures/affine.js"),
            false,
        ),
        Workload::AffineTs => (
            include_str!("../../../tests/compatibility/fixtures/affine.ts"),
            true,
        ),
        Workload::GcdJs => (
            include_str!("../../../examples/numeric-workflow/gcd.js"),
            false,
        ),
    }
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn options_reject_unbounded_duplicates_unknown_cases_and_missing_values() {
    assert_eq!(parse_options(&[]).unwrap(), Options::default());
    assert_eq!(
        parse_options(&args(&[
            "--workload",
            "gcd-js",
            "--samples",
            "2",
            "--iterations",
            "1",
            "--warmup",
            "0"
        ]))
        .unwrap(),
        Options {
            workload: Workload::GcdJs,
            samples: 2,
            iterations: 1,
            warmup: 0,
        }
    );
    for invalid in [
        vec!["--samples"],
        vec!["--samples", "1"],
        vec!["--samples", "13"],
        vec!["--iterations", "0"],
        vec!["--iterations", "33"],
        vec!["--warmup", "33"],
        vec!["--workload", "fake"],
        vec!["--samples", "2", "--samples", "3"],
        vec!["--unknown", "1"],
        vec!["--warmup", "-1"],
        vec!["--iterations", "NaN"],
    ] {
        assert!(parse_options(&args(&invalid)).is_err(), "{invalid:?}");
    }
}

#[test]
fn public_compiler_artifacts_are_repeatable_and_execute_fixed_goldens() {
    for workload in [Workload::AffineJs, Workload::AffineTs, Workload::GcdJs] {
        let options = Options {
            workload,
            samples: 2,
            iterations: 2,
            warmup: 1,
        };
        let report = probe(options).unwrap();
        assert_eq!(report.options, options);
        assert!(report.source_bytes > 0 && report.source_bytes < 16_384);
        assert!(report.wat_bytes > 0 && report.wasm_bytes >= 8 && report.wasm_bytes <= 1024 * 1024);
        assert!(report.first_compile_ms.is_finite() && report.first_compile_ms >= 0.0);
        let (input, ts) = fixture(workload);
        assert_eq!(report.source_sha256, sha256(input.as_bytes()));
        let artifact = grunkalunka_compiler::compile(input, ts).unwrap();
        assert_eq!(report.wat_sha256, sha256(artifact.wat.as_bytes()));
        assert_eq!(report.wasm_sha256, sha256(&artifact.wasm));
        for digest in [
            &report.source_sha256,
            &report.wat_sha256,
            &report.wasm_sha256,
        ] {
            assert_eq!(digest.len(), 64);
            assert!(
                digest
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            );
        }
        assert_eq!(report.samples_ms.len(), options.samples);
        assert_eq!(report.release_ms.len(), options.samples);
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let samples = report
                .process_memory
                .snapshots
                .as_ref()
                .expect("Linux/Windows must observe real compiler host process memory");
            assert!(
                samples
                    .iter()
                    .all(|sample| sample.peak_rss_bytes >= sample.rss_bytes)
            );
            for pair in samples.windows(2) {
                assert!(pair[1].peak_rss_bytes >= pair[0].peak_rss_bytes);
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        assert!(report.process_memory.snapshots.is_none());
        assert!(
            report
                .samples_ms
                .iter()
                .chain(report.release_ms.iter())
                .all(|ms| ms.is_finite() && *ms >= 0.0)
        );
        assert_eq!(
            report.golden_cases,
            if workload == Workload::GcdJs { 8 } else { 4 }
        );
        let json = report.json();
        for expected in [
            "\"schemaVersion\":1",
            "\"workload\":\"in-process-source-compile\"",
            "\"samplesMs\":",
            "\"firstCompileMs\":",
            "\"artifactRelease\":{",
            "\"goldenCases\":",
            "\"profile\":",
            "\"watBytes\":",
            "\"sourceSha256\":",
            "\"watSha256\":",
            "\"wasmSha256\":",
            "\"processMemory\":",
        ] {
            assert!(json.contains(expected), "missing {expected}");
        }
        assert!(json.contains(&format!("\"case\":\"{}\"", workload.name())));
        assert_eq!(json.matches("\"samplesMs\":").count(), 2);
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let label = source_compile_cost::process_memory::source_name();
            assert!(json.contains(&format!("\"source\":\"{label}\"")));
            for phase in source_compile_cost::MEMORY_PHASES {
                assert!(json.contains(&format!("\"{phase}\":")), "{phase}");
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        assert!(json.contains("\"source\":\"unsupported-on-this-host\""));
    }
}

#[test]
fn callers_cannot_bypass_validation_by_constructing_options_directly() {
    assert!(
        probe(Options {
            samples: 1,
            ..Options::default()
        })
        .is_err()
    );
    assert!(
        probe(Options {
            iterations: 33,
            ..Options::default()
        })
        .is_err()
    );
}
