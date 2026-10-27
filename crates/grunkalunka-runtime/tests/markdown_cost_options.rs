//! P1 Markdown component-cost configuration and fixed-fixture checks.
#[allow(dead_code)] // The included example is also a standalone executable.
#[path = "../examples/markdown_cost.rs"]
mod markdown_cost;

use markdown_cost::{Options, Series, Workload, parse_options};

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn options_reject_missing_duplicate_and_unbounded_measurements() {
    assert_eq!(parse_options(&[]).unwrap(), Options::default());
    assert_eq!(
        parse_options(&args(&[
            "--samples",
            "2",
            "--iterations",
            "1",
            "--warmup",
            "0",
            "--workload",
            "wide"
        ]))
        .unwrap(),
        Options {
            samples: 2,
            iterations: 1,
            warmup: 0,
            workload: Workload::Wide
        }
    );
    for invalid in [
        vec!["--samples"],
        vec!["--samples", "1"],
        vec!["--samples", "13"],
        vec!["--iterations", "0"],
        vec!["--iterations", "101"],
        vec!["--warmup", "101"],
        vec!["--warmup", "-1"],
        vec!["--samples", "2", "--samples", "3"],
        vec!["--workload", "unknown"],
        vec!["--misspelled", "2"],
        vec!["--iterations", "NaN"],
    ] {
        assert!(parse_options(&args(&invalid)).is_err(), "{invalid:?}");
    }
}

#[test]
fn fixture_payloads_and_unicode_ranges_stay_inside_guest_quotas() {
    for workload in [
        Workload::Empty,
        Workload::Unicode,
        Workload::Dense,
        Workload::Wide,
    ] {
        assert_eq!(Workload::parse(workload.name()), Some(workload));
        assert!(workload.source().len() <= 4096);
    }
    assert!(Workload::Empty.source().is_empty());
    assert!(Workload::Unicode.source().contains("\r\n### 子😀"));
    assert_eq!(Workload::Dense.source().matches("# Heading ").count(), 24);
    assert_eq!(Workload::Wide.source().len(), 3602);
}

#[test]
fn statistics_keep_all_samples_without_speed_thresholds() {
    let odd = Series::new(vec![4.0, 2.0, 3.0]);
    assert_eq!(odd.samples_ms, vec![4.0, 2.0, 3.0]);
    assert_eq!((odd.min_ms, odd.median_ms, odd.max_ms), (2.0, 3.0, 4.0));
    let even = Series::new(vec![5.0, 3.0, 1.0, 7.0]);
    assert_eq!((even.min_ms, even.median_ms, even.max_ms), (1.0, 4.0, 7.0));
    assert!(
        even.json()
            .contains("\"samplesMs\":[5.000000000,3.000000000,1.000000000,7.000000000]")
    );
}
