//! P1 experimental rustc wrapper: replace only Cargo's *generated*
//! -C metadata=<16 hex> per crate, not extra-filename or unrelated flags.
//! Every other compiler argument goes to the exact rustc executable passed
//! by Cargo. Never install as a project build default or production wrapper.
use std::env;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::{Command, ExitCode};

const TARGET: &str = "wasm32-unknown-unknown";

fn extra_wasm_metadata(crate_name: &str, target: &str) -> Option<&'static str> {
    match (crate_name, target) {
        ("bitflags", TARGET) => Some("p1bitflags"),
        ("memchr", TARGET) => Some("p1memchr"),
        _ => None,
    }
}
fn stable_metadata(crate_name: &str, target: &str) -> Option<&'static str> {
    match (crate_name, target) {
        ("build_script_build", "host") => Some("p1script"),
        ("unicase", TARGET) => Some("p1unicase"),
        ("pulldown_cmark", TARGET) => Some("p1pulldown"),
        ("grunkalunka_markdown", TARGET) => Some("p1markdown"),
        ("outline_guest", TARGET) => Some("p1guest"),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MetadataRecord {
    crate_name: String,
    target: String,
    original: String,
    effective: String,
}

fn metadata_flag(input: &str) -> Option<&str> {
    input.strip_prefix("metadata=")
}
fn is_cargo_hash(value: &str) -> bool {
    value.len() == 16 && value.bytes().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

// Compilers may be queried by Cargo for version/target data. Pass those
// requests through unchanged and require complete data for actual crate builds.
fn rewrite(args: &mut [OsString], mode: &str) -> Result<Option<MetadataRecord>, String> {
    if !matches!(mode, "passthrough" | "target" | "target_plus_deps" | "all") {
        return Err("unsupported wrapper normalization mode".to_string());
    }
    let mut crate_name = None;
    let mut target = "host".to_string();
    let mut metadata_slots = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].to_string_lossy();
        if arg == "--crate-name" {
            if i + 1 >= args.len() {
                return Err("missing crate name".to_string());
            }
            crate_name = Some(args[i + 1].to_string_lossy().into_owned());
            i += 2;
            continue;
        }
        if arg == "--target" {
            if i + 1 >= args.len() {
                return Err("missing target triple".to_string());
            }
            target = args[i + 1].to_string_lossy().into_owned();
            i += 2;
            continue;
        }
        if arg == "-C" {
            if i + 1 >= args.len() {
                return Err("missing rustc codegen option".to_string());
            }
            if let Some(value) = metadata_flag(&args[i + 1].to_string_lossy()) {
                metadata_slots.push((i + 1, value.to_string()));
            }
            i += 2;
            continue;
        }
        // An unexpected combined spelling would invalidate our assertion
        // that we have inspected every effective -C metadata input.
        if arg.starts_with("-Cmetadata=") {
            return Err("unsupported combined -C metadata flag".to_string());
        }
        i += 1;
    }
    // Cargo performs target discovery with a synthetic "--crate-name ___",
    // several "--print=..." flags, and no generated metadata. That is not
    // an actual crate compile and must be sent to rustc unchanged.
    if args.iter().any(|item| item.to_string_lossy().starts_with("--print")) {
        if metadata_slots.is_empty() {
            return Ok(None);
        }
        return Err("unexpected generated metadata in rustc compiler probe".to_string());
    }
    let Some(crate_name) = crate_name else {
        return Ok(None); // rustc -vV and similar compiler probes
    };
    // Preserve the original #147 five-crate substitution in its modes.
    // Only the new target_plus_deps mode adds bitflags and memchr, so their
    // incremental impact can be attributed without changing old controls.
    let selected = stable_metadata(&crate_name, &target);
    let extra = extra_wasm_metadata(&crate_name, &target);
    if matches!(crate_name.as_str(), "build_script_build" | "unicase" |
        "pulldown_cmark" | "grunkalunka_markdown" | "outline_guest" |
        "bitflags" | "memchr") &&
        selected.is_none() && extra.is_none()
    {
        return Err(format!("selected crate on unexpected compilation target: {crate_name}/{target}"));
    }
    if metadata_slots.len() != 1 || !is_cargo_hash(&metadata_slots[0].1) {
        return Err(format!("expected exactly one generated Cargo metadata value for {crate_name}"));
    }
    let (position, original) = &metadata_slots[0];
    let replace_selected = selected.is_some() &&
        (mode == "all" || (target == TARGET &&
        (mode == "target" || mode == "target_plus_deps")));
    let replace_extra = extra.is_some() && mode == "target_plus_deps";
    let replace = replace_selected || replace_extra;
    let effective = if replace_selected {
        selected.unwrap()
    } else if replace_extra {
        extra.unwrap()
    } else {
        original
    };
    if replace {
        args[*position] = OsString::from(format!("metadata={effective}"));
    }
    Ok(Some(MetadataRecord {
        crate_name,
        target,
        original: original.clone(),
        effective: effective.to_string(),
    }))
}

fn record_change(change: &MetadataRecord) -> Result<(), String> {
    let log_file = env::var_os("P1_WRAPPER_LOG")
        .ok_or_else(|| "missing P1_WRAPPER_LOG path".to_string())?;
    let mut f = OpenOptions::new().create(true).append(true)
        .open(log_file).map_err(|e| format!("opening wrapper log: {e}"))?;
    // -j 1 means this is one short OS append write per rustc invocation.
    // No source paths, environment values or raw rustc flags are logged.
    f.write_all(format!("{}\t{}\t{}\t{}\n",
        change.crate_name, change.target, change.original, change.effective).as_bytes())
        .map_err(|e| format!("writing wrapper log: {e}"))
}

fn real_main() -> Result<i32, String> {
    let mut arguments = env::args_os().skip(1);
    let rustc = arguments.next()
        .ok_or_else(|| "Cargo did not supply the rustc executable".to_string())?;
    let mode = env::var("P1_WRAPPER_MODE")
        .map_err(|_| "missing P1_WRAPPER_MODE".to_string())?;
    let mut args: Vec<OsString> = arguments.collect();
    if let Some(change) = rewrite(&mut args, &mode)? {
        record_change(&change)?;
    }
    let status = Command::new(&rustc).args(args).status()
        .map_err(|e| format!("running rustc: {e}"))?;
    Ok(status.code().unwrap_or(1))
}

fn main() -> ExitCode {
    match real_main() {
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        Err(message) => {
            eprintln!("P1 metadata wrapper rejected invocation: {message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn argv(name: &str, target: bool) -> Vec<OsString> {
        let mut args: Vec<OsString> =
            vec!["--crate-name", name].into_iter().map(OsString::from).collect();
        if target {
            args.extend(["--target", TARGET].into_iter().map(OsString::from));
        }
        args.extend(["-C", "metadata=0123456789abcdef", "-C", "extra-filename=-cafebabe"]
            .into_iter().map(OsString::from));
        args
    }
    #[test]
    fn pass_through_keeps_generated_metadata_intact() {
        let mut args = argv("pulldown_cmark", true);
        let original = args.clone();
        let record = rewrite(&mut args, "passthrough").unwrap().unwrap();
        assert_eq!(args, original);
        assert_eq!(record.original, "0123456789abcdef");
        assert_eq!(record.effective, record.original);
    }
    #[test]
    fn target_override_alters_only_generated_metadata() {
        let mut args = argv("outline_guest", true);
        let record = rewrite(&mut args, "target").unwrap().unwrap();
        assert_eq!(record.effective, "p1guest");
        assert_eq!(args[5], "metadata=p1guest");
        assert_eq!(args[7], "extra-filename=-cafebabe");
    }
    #[test]
    fn new_mode_also_replaces_exact_two_remaining_wasm_dependency_hashes() {
        for (crate_name, replacement) in [
            ("bitflags", "p1bitflags"), ("memchr", "p1memchr")
        ] {
            let mut args = argv(crate_name, true);
            let old = args.clone();
            assert_eq!(rewrite(&mut args, "target").unwrap().unwrap().effective,
                "0123456789abcdef");
            assert_eq!(args, old);
            let changed = rewrite(&mut args, "target_plus_deps").unwrap().unwrap();
            assert_eq!(changed.effective, replacement);
            assert_eq!(args[5], OsString::from(format!("metadata={replacement}")));
        }
        let mut script = argv("build_script_build", false);
        assert_eq!(rewrite(&mut script, "target_plus_deps").unwrap().unwrap().effective,
            "0123456789abcdef");
    }
    #[test]
    fn target_mode_does_not_rewrite_host_build_script() {
        let mut args = argv("build_script_build", false);
        let record = rewrite(&mut args, "target").unwrap().unwrap();
        assert_eq!(record.effective, record.original);
        let all = rewrite(&mut args, "all").unwrap().unwrap();
        assert_eq!(all.effective, "p1script");
    }
    #[test]
    fn leaves_unselected_crates_unchanged_and_rejects_malformed_metadata() {
        let mut unknown = argv("bitflags", true);
        let original = unknown.clone();
        let unselected = rewrite(&mut unknown, "all").unwrap().unwrap();
        assert_eq!(unknown, original);
        assert_eq!(unselected.original, unselected.effective);
        let mut missing: Vec<OsString> = ["--crate-name", "outline_guest"]
            .into_iter().map(OsString::from).collect();
        assert!(rewrite(&mut missing, "target").is_err());
        let mut duplicated = argv("outline_guest", true);
        duplicated.extend(["-C", "metadata=1111111111111111"]
            .into_iter().map(OsString::from));
        assert!(rewrite(&mut duplicated, "target").is_err());
    }
    #[test]
    fn rustc_probes_pass_through_without_logging() {
        let mut args: Vec<OsString> = ["-vV"]
            .into_iter().map(OsString::from).collect();
        assert_eq!(rewrite(&mut args, "target").unwrap(), None);
        assert_eq!(args, vec![OsString::from("-vV")]);
    }
    #[test]
    fn cargo_synthetic_crate_information_probe_passes_through_unchanged() {
        let mut args: Vec<OsString> =
            ["-", "--crate-name", "___", "--print=file-names",
             "--crate-type", "bin", "--print=sysroot", "--print=cfg"]
            .into_iter().map(OsString::from).collect();
        let before = args.clone();
        assert_eq!(rewrite(&mut args, "target").unwrap(), None);
        assert_eq!(args, before);
        let mut selected_wrong_target: Vec<OsString> = ["--crate-name", "outline_guest",
            "-C", "metadata=0123456789abcdef"]
            .into_iter().map(OsString::from).collect();
        assert!(rewrite(&mut selected_wrong_target, "target").is_err());
    }
    #[test]
    fn rejects_invalid_modes_or_unsupported_combined_flags() {
        let mut args = argv("unicase", true);
        assert!(rewrite(&mut args, "unsupported").is_err());
        args.push(OsString::from("-Cmetadata=deadbeefdeadbeef"));
        assert!(rewrite(&mut args, "all").is_err());
    }
}
