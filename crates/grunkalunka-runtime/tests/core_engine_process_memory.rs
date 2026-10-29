//! Validate native process-memory observations with platform-specific labels.
#[path = "../examples/core_engine_cost/process_memory.rs"]
mod process_memory;

use process_memory::{ProcessMemory, Snapshot, parse_linux_status};

fn status(rss: u64, peak: u64) -> String {
    format!(
        "Name:\tgrunkalunka\nVmSize:\t123456 kB\nVmRSS:\t{rss} kB\nVmHWM:\t{peak} kB\nThreads:\t2\n"
    )
}

#[test]
fn parse_whole_process_rss_and_high_water_mark_as_bytes() {
    let measured = parse_linux_status(&status(4096, 8192)).unwrap();
    assert_eq!(measured.rss_bytes, 4_194_304);
    assert_eq!(measured.peak_rss_bytes, 8_388_608);
    assert_eq!(
        parse_linux_status("  VmHWM: 2 kB\nVmRSS: 1 kB\n").unwrap(),
        Snapshot {
            rss_bytes: 1024,
            peak_rss_bytes: 2048
        }
    );
}

#[test]
fn missing_duplicate_wrong_unit_overflow_and_inconsistent_values_reject() {
    let good = status(2, 3);
    for invalid in [
        "Name: none\n".to_string(),
        good.replace("VmHWM:\t3 kB", ""),
        good.replace("VmRSS:\t2 kB", "VmRSS:\t2 MiB"),
        good.replace("VmRSS:\t2 kB", "VmRSS:\tnope kB"),
        good.replace("VmRSS:\t2 kB", "VmRSS:\t2 kB\nVmRSS:\t2 kB"),
        good.replace("VmRSS:\t2 kB", &format!("VmRSS:\t{} kB", u64::MAX)),
        status(4, 3),
    ] {
        assert!(parse_linux_status(&invalid).is_err(), "{invalid:?}");
    }
}

#[test]
fn coherent_sample_sets_preserve_phases_and_reject_partial_or_decreasing_peak() {
    let first = Snapshot {
        rss_bytes: 4096,
        peak_rss_bytes: 8192,
    };
    let second = Snapshot {
        rss_bytes: 2048,
        peak_rss_bytes: 12_288,
    };
    let memory = ProcessMemory::assemble([
        Some(first),
        Some(first),
        Some(second),
        Some(second),
        Some(second),
    ])
    .unwrap();
    assert!(memory.snapshots.is_some());
    assert_eq!(memory.snapshots.unwrap()[4], second);
    let json = memory.json();
    let labeled_source = format!("\"source\":\"{}\"", process_memory::source_name());
    for expected in [
        "\"scope\":\"entire-rust-process\"",
        labeled_source.as_str(),
        "\"unit\":\"bytes\"",
        "\"beforeModule\":{\"rssBytes\":4096,\"peakRssBytes\":8192}",
        "\"afterBatches\":{\"rssBytes\":2048,\"peakRssBytes\":12288}",
    ] {
        assert!(json.contains(expected), "missing {expected}");
    }
    assert!(ProcessMemory::assemble([Some(first), None, None, None, None]).is_err());
    assert!(
        ProcessMemory::assemble([
            Some(second),
            Some(second),
            Some(first),
            Some(first),
            Some(first),
        ])
        .is_err()
    );
    assert!(
        ProcessMemory::assemble([
            Some(Snapshot {
                rss_bytes: 9000,
                peak_rss_bytes: 8000,
            }),
            Some(first),
            Some(first),
            Some(first),
            Some(first),
        ])
        .is_err()
    );
}

#[test]
fn absent_host_support_is_explicit_null_not_fabricated_zero() {
    let report = ProcessMemory::assemble([None; 5]).unwrap();
    assert!(report.snapshots.is_none());
    assert_eq!(report.snapshots, None);
    assert_eq!(
        report.json(),
        r#"{"scope":"entire-rust-process","unit":"bytes","source":"unsupported-on-this-host","snapshots":null}"#
    );
}

#[test]
fn observable_host_support_matches_the_actual_platform() {
    let snapshot = process_memory::observe().unwrap();
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        let snapshot = snapshot.expect("native Linux/Windows process memory is required");
        assert!(snapshot.peak_rss_bytes >= snapshot.rss_bytes);
        assert!(snapshot.rss_bytes > 0);
        assert!(snapshot.peak_rss_bytes > 0);
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    assert_eq!(snapshot, None);
}

#[test]
fn synthetic_sample_records_actual_platform_source_label() {
    let memory = ProcessMemory::assemble(
        [Some(Snapshot {
            rss_bytes: 4096,
            peak_rss_bytes: 8192,
        }); 5],
    )
    .unwrap();
    let source = process_memory::source_name();
    assert!(memory.json().contains(&format!("\"source\":\"{source}\"")));
    #[cfg(target_os = "windows")]
    {
        assert_eq!(source, "windows-psapi-working-set");
        assert!(!memory.json().contains("linux-proc-status"));
    }
}
