//! Exercise a real scalar WIT resource round trip, not a timing threshold.
#[allow(dead_code)] // The included example has a standalone CLI entry point.
#[path = "../examples/scalar_wit_cost.rs"]
mod scalar_wit_cost;

use scalar_wit_cost::{Options, parse_options, probe};

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn options_are_bounded_and_unknown_or_duplicate_values_reject() {
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
            warmup: 0,
        }
    );
    for invalid in [
        vec!["--samples"],
        vec!["--samples", "1"],
        vec!["--samples", "21"],
        vec!["--iterations", "0"],
        vec!["--iterations", "5001"],
        vec!["--warmup", "1001"],
        vec!["--warmup", "-1"],
        vec!["--iterations", "NaN"],
        vec!["--unknown", "2"],
        vec!["--samples", "2", "--samples", "3"],
    ] {
        assert!(parse_options(&args(&invalid)).is_err(), "{invalid:?}");
    }
}

fn assert_series(samples: &[f64], count: usize) {
    assert_eq!(samples.len(), count);
    assert!(
        samples
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0)
    );
}

#[test]
fn live_scalar_resource_and_native_control_preserve_goldens_and_ownership() {
    let options = Options {
        samples: 2,
        iterations: 3,
        warmup: 1,
    };
    let result = probe(options).unwrap();
    assert_eq!(result.options, options);
    assert!(result.component_bytes > 0);
    assert!(result.wat_parse_ms.is_finite() && result.wat_parse_ms >= 0.0);
    assert!(result.compile_ms.is_finite() && result.compile_ms >= 0.0);
    for series in [&result.instantiation, &result.native, &result.component] {
        assert_series(&series.samples_ms, options.samples);
        assert!(series.min_ms <= series.median_ms);
        assert!(series.median_ms <= series.max_ms);
    }
    assert_eq!(result.granted_reads, 3 + 1 + 2 * 3);
    assert_eq!(result.denied_reads, 0);
    let report = result.json();
    for fragment in [
        "\"schemaVersion\":1",
        "\"workload\":\"scalar-wit-resource-read\"",
        "\"goldenCases\":3",
        "\"grantedReads\":10",
        "\"deniedReads\":0",
        "\"component\":",
        "\"native\":",
        "\"instantiation\":",
        "\"samplesMs\":",
    ] {
        assert!(report.contains(fragment), "missing {fragment}");
    }
    assert_eq!(report.matches("\"samplesMs\":").count(), 3);
}

#[test]
fn direct_probe_calls_cannot_bypass_bounds() {
    assert!(
        probe(Options {
            samples: 0,
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
