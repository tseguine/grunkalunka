//! Observational process memory for the bounded core-Wasm cost replay.
//! Linux reports whole-Rust-process values. Other hosts report unavailable.
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

#[cfg(not(target_os = "linux"))]
pub fn observe() -> Result<Option<Snapshot>, String> {
    Ok(None)
}

#[derive(Debug)]
pub struct ProcessMemory {
    pub snapshots: Option<[Snapshot; 5]>,
}

impl ProcessMemory {
    pub fn assemble(samples: [Option<Snapshot>; 5]) -> Result<Self, String> {
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
                return Err(format!("peak RSS below RSS at {}", PHASES[index]));
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
        let Some(values) = self.snapshots else {
            return r#"{"scope":"entire-rust-process","unit":"bytes","source":"unsupported-on-this-host","snapshots":null}"#.into();
        };
        let phases = PHASES
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
            r#"{{"scope":"entire-rust-process","unit":"bytes","source":"linux-proc-status","snapshots":{{{phases}}}}}"#
        )
    }
}
