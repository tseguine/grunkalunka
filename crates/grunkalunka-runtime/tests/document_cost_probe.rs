//! Check real component calls and bounded reporting, never a speed threshold.
#[allow(dead_code)] // The same example also has a binary entry point.
#[path = "../examples/document_cost.rs"]
mod document_cost;

use document_cost::{Options, parse_options, probe};

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[test]
fn options_bound_total_work_and_reject_malformed_requests() {
    assert_eq!(parse_options(&[]).unwrap(), Options::default());
    assert_eq!(
        parse_options(&args(&[
            "--samples",
            "2",
            "--iterations",
            "1",
            "--warmup",
            "0",
            "--bytes",
            "4096"
        ]))
        .unwrap(),
        Options {
            samples: 2,
            iterations: 1,
            warmup: 0,
            bytes: 4096
        }
    );
    for invalid in [
        vec!["--samples"],
        vec!["--samples", "1"],
        vec!["--samples", "21"],
        vec!["--iterations", "0"],
        vec!["--iterations", "10001"],
        vec!["--warmup", "10001"],
        vec!["--bytes", "4097"],
        vec!["--bytes", "-1"],
        vec!["--samples", "3", "--samples", "4"],
        vec!["--unsupported", "2"],
        vec!["--bytes", "NaN"],
    ] {
        assert!(parse_options(&args(&invalid)).is_err(), "{invalid:?}");
    }
}

fn verify_series(values: &[f64], count: usize) {
    assert_eq!(values.len(), count);
    assert!(
        values
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0)
    );
}

#[test]
fn granted_and_denied_component_calls_keep_counts_and_raw_samples() {
    let options = Options {
        samples: 2,
        iterations: 3,
        warmup: 2,
        bytes: 17,
    };
    let report = probe(options).unwrap();
    assert_eq!(report.options, options);
    assert!(report.component_bytes > 0);
    assert!(report.wat_parse_ms.is_finite() && report.wat_parse_ms >= 0.0);
    assert!(report.compile_ms.is_finite() && report.compile_ms >= 0.0);
    verify_series(&report.instantiation.samples_ms, options.samples);
    for mode in &report.modes {
        verify_series(&mode.samples_ms, options.samples);
        assert!(mode.min_ms <= mode.median_ms);
        assert!(mode.median_ms <= mode.max_ms);
    }
    assert_eq!(
        report.granted_reads,
        1 + options.warmup + options.samples * options.iterations
    );
    assert_eq!(report.denied_reads, 0);
    let output = report.json();
    for expected in [
        "\"schemaVersion\":1",
        "\"workload\":\"wit-document-read\"",
        "\"payloadBytes\":17",
        "\"componentGrantedReads\":9",
        "\"componentDeniedReads\":0",
        "\"instantiation\":",
        "\"componentGranted\":",
        "\"componentDenied\":",
        "\"nativeGranted\":",
        "\"nativeDenied\":",
    ] {
        assert!(output.contains(expected), "missing {expected}");
    }
    assert_eq!(output.matches("\"samplesMs\":").count(), 5);
}

#[test]
fn empty_and_maximum_utf8_payloads_are_valid_in_both_profiles() {
    for bytes in [0, 4096] {
        let report = probe(Options {
            samples: 2,
            iterations: 1,
            warmup: 0,
            bytes,
        })
        .unwrap();
        assert_eq!(report.granted_reads, 3);
        assert_eq!(report.denied_reads, 0);
    }
}

#[test]
fn direct_probes_cannot_bypass_configuration_bounds() {
    assert!(
        probe(Options {
            samples: 0,
            ..Options::default()
        })
        .is_err()
    );
    assert!(
        probe(Options {
            bytes: 4097,
            ..Options::default()
        })
        .is_err()
    );
}
