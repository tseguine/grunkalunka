//! Observational whole-process memory for P1 cost probes.
//! Linux VmRSS/VmHWM and Windows PSAPI working set use distinct accounting.
//! macOS remains explicitly unavailable instead of inventing zero memory.
pub const PHASES: [&str; 5] = [
    "beforeModule",
    "afterModule",
    "afterInstantiation",
    "afterWarmup",
    "afterBatches",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub rss_bytes: u64,
    pub peak_rss_bytes: u64,
}

#[cfg(any(target_os = "linux", test))]
fn parse_field(status: &str, field: &str) -> Result<u64, String> {
    let entries = status
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix(field))
        .collect::<Vec<_>>();
    if entries.len() != 1 {
        return Err(format!("expected one {field} field"));
    }
    let parts = entries[0].split_whitespace().collect::<Vec<_>>();
    if parts.len() != 2 || parts[1] != "kB" {
        return Err(format!("invalid {field} units"));
    }
    let kib = parts[0]
        .parse::<u64>()
        .map_err(|_| format!("invalid {field} value"))?;
    kib.checked_mul(1024)
        .ok_or_else(|| format!("{field} byte count overflow"))
}

#[cfg(any(target_os = "linux", test))]
pub fn parse_linux_status(status: &str) -> Result<Snapshot, String> {
    let snapshot = Snapshot {
        rss_bytes: parse_field(status, "VmRSS:")?,
        peak_rss_bytes: parse_field(status, "VmHWM:")?,
    };
    if snapshot.peak_rss_bytes < snapshot.rss_bytes {
        return Err("VmHWM is smaller than VmRSS".into());
    }
    Ok(snapshot)
}

#[cfg(target_os = "linux")]
pub fn observe() -> Result<Option<Snapshot>, String> {
    let status = std::fs::read_to_string("/proc/self/status")
        .map_err(|error| format!("cannot sample /proc/self/status: {error}"))?;
    parse_linux_status(&status).map(Some)
}

#[cfg(target_os = "windows")]
mod windows {
    use super::Snapshot;
    use std::ffi::c_void;
    use std::mem::size_of;

    // PROCESS_MEMORY_COUNTERS: two DWORDs followed by eight native SIZE_Ts.
    #[repr(C)]
    #[derive(Default)]
    #[allow(dead_code)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn K32GetProcessMemoryInfo(
            process: *mut c_void,
            counters: *mut ProcessMemoryCounters,
            size: u32,
        ) -> i32;
    }

    pub fn observe() -> Result<Snapshot, String> {
        let expected_size = if cfg!(target_pointer_width = "64") {
            72
        } else {
            40
        };
        let size = size_of::<ProcessMemoryCounters>();
        if size != expected_size {
            return Err("unexpected PROCESS_MEMORY_COUNTERS layout".into());
        }
        let cb = u32::try_from(size).map_err(|_| "Windows counter size overflow")?;
        let mut counters = ProcessMemoryCounters {
            cb,
            ..Default::default()
        };
        // SAFETY: the pseudo-handle is this process; counters is an initialized
        // writable #[repr(C)] structure with the exact API size.
        let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, cb) };
        if ok == 0 {
            return Err(format!(
                "K32GetProcessMemoryInfo: {}",
                std::io::Error::last_os_error()
            ));
        }
        let rss_bytes = u64::try_from(counters.working_set_size)
            .map_err(|_| "Windows working set exceeds u64")?;
        let peak_rss_bytes = u64::try_from(counters.peak_working_set_size)
            .map_err(|_| "Windows peak working set exceeds u64")?;
        if peak_rss_bytes < rss_bytes {
            return Err("Windows peak working set below current working set".into());
        }
        Ok(Snapshot {
            rss_bytes,
            peak_rss_bytes,
        })
    }
}

#[cfg(target_os = "windows")]
pub fn observe() -> Result<Option<Snapshot>, String> {
    windows::observe().map(Some)
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn observe() -> Result<Option<Snapshot>, String> {
    Ok(None)
}

pub fn source_name() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        "linux-proc-status"
    }
    #[cfg(target_os = "windows")]
    {
        "windows-psapi-working-set"
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        "unsupported-on-this-host"
    }
}

#[derive(Debug)]
pub struct ProcessMemory {
    pub snapshots: Option<[Snapshot; 5]>,
}

impl ProcessMemory {
    pub fn assemble(samples: [Option<Snapshot>; 5]) -> Result<Self, String> {
        Self::assemble_with_phases(samples, &PHASES)
    }

    pub fn assemble_with_phases(
        samples: [Option<Snapshot>; 5],
        phases: &[&str; 5],
    ) -> Result<Self, String> {
        let present = samples.iter().filter(|sample| sample.is_some()).count();
        if present == 0 {
            return Ok(Self { snapshots: None });
        }
        if present != samples.len() {
            return Err("partial process memory samples are not valid evidence".into());
        }
        let values = samples.map(|sample| sample.expect("all samples present"));
        for (index, sample) in values.iter().enumerate() {
            if sample.peak_rss_bytes < sample.rss_bytes {
                return Err(format!("peak RSS below RSS at {}", phases[index]));
            }
            if index > 0 && sample.peak_rss_bytes < values[index - 1].peak_rss_bytes {
                return Err("process peak RSS decreased".into());
            }
        }
        Ok(Self {
            snapshots: Some(values),
        })
    }

    pub fn json(&self) -> String {
        self.json_with_phases(&PHASES)
    }

    pub fn json_with_phases(&self, phase_names: &[&str; 5]) -> String {
        let Some(values) = self.snapshots else {
            return r#"{"scope":"entire-rust-process","unit":"bytes","source":"unsupported-on-this-host","snapshots":null}"#.into();
        };
        let phases = phase_names
            .iter()
            .zip(values)
            .map(|(phase, value)| {
                format!(
                    r#""{phase}":{{"rssBytes":{},"peakRssBytes":{}}}"#,
                    value.rss_bytes, value.peak_rss_bytes
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"{{"scope":"entire-rust-process","unit":"bytes","source":"{}","snapshots":{{{phases}}}}}"#,
            source_name()
        )
    }
}
