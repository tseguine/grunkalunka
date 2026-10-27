//! Repeat real public compiler builds on bounded JS/TS fixtures without timing gates.
#[allow(dead_code)] // The included example also provides an executable entry point.
#[path = "../examples/source_compile_cost.rs"]
mod source_compile_cost;

use source_compile_cost::{Options, Workload, parse_options, probe};

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
        assert_eq!(report.samples_ms.len(), options.samples);
        assert!(
            report
                .samples_ms
                .iter()
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
            "\"goldenCases\":",
            "\"profile\":",
            "\"watBytes\":",
        ] {
            assert!(json.contains(expected), "missing {expected}");
        }
        assert!(json.contains(&format!("\"case\":\"{}\"", workload.name())));
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
