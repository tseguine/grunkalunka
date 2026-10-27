//! Exact scalar host-import execution and bounded P1 observation, no speed gate.
#[allow(dead_code)] // Example is also a standalone binary.
#[path = "../examples/host_call_calibration.rs"]
mod host_call_calibration;

use host_call_calibration::{Options, parse_options, probe};

fn args(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_owned()).collect()
}

#[test]
fn options_are_bounded_and_unique() {
    assert_eq!(parse_options(&[]).unwrap(), Options::default());
    assert_eq!(
        parse_options(&args(&["--samples", "2", "--iterations", "1", "--warmup", "0"]))
            .unwrap(),
        Options {
            samples: 2,
            iterations: 1,
            warmup: 0
        }
    );
    for invalid in [
        vec!["--samples"],
        vec!["--samples", "1"],
        vec!["--samples", "13"],
        vec!["--iterations", "0"],
        vec!["--iterations", "5001"],
        vec!["--warmup", "1001"],
        vec!["--samples", "2", "--samples", "3"],
        vec!["--unknown", "1"],
        vec!["--iterations", "NaN"],
        vec!["--warmup", "-1"],
    ] {
        assert!(parse_options(&args(&invalid)).is_err(), "{invalid:?}");
    }
}

#[test]
fn real_import_is_called_once_per_sampled_guest_boundary() {
    let options = Options {
        samples: 2,
        iterations: 7,
        warmup: 3,
    };
    let report = probe(options).unwrap();
    assert_eq!(report.options, options);
    assert!(report.wasm_bytes > 0 && report.wasm_bytes <= 1024);
    assert_eq!(report.native_calls, 6 + 3 + 2 * 7);
    assert_eq!(report.host_calls, report.native_calls);
    assert_eq!(report.host_input_sum, report.native_input_sum);
    for series in &report.modes {
        assert_eq!(series.samples_ms.len(), options.samples);
        assert!(series.samples_ms.iter().all(|ms| ms.is_finite() && *ms >= 0.0));
        assert!(series.min_ms <= series.median_ms && series.median_ms <= series.max_ms);
    }
    let json = report.json();
    for field in [
        "\"workload\":\"core-wasm-scalar-host-call-calibration\"",
        "\"native\":",
        "\"guestOnly\":",
        "\"guestToHost\":",
        "\"hostCalls\":23",
        "\"wasmtimeDependency\":\"=50.0.0\"",
    ] {
        assert!(json.contains(field), "missing {field}");
    }
    assert_eq!(json.matches("\"samplesMs\":").count(), 3);
}

#[test]
fn zero_warmup_and_cyclic_case_boundaries_preserve_goldens() {
    for iterations in [1, 6, 7] {
        let report = probe(Options {
            samples: 2,
            iterations,
            warmup: 0,
        })
        .unwrap();
        assert_eq!(report.host_calls, 6 + 2 * iterations);
        assert_eq!(report.host_input_sum, report.native_input_sum);
    }
}

#[test]
fn direct_probe_calls_do_not_bypass_configuration_limits() {
    assert!(
        probe(Options {
            samples: 1,
            ..Options::default()
        })
        .is_err()
    );
    assert!(
        probe(Options {
            iterations: 5001,
            ..Options::default()
        })
        .is_err()
    );
}
