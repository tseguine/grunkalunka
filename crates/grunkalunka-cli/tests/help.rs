use std::process::{Command, Output, Stdio};

fn invoke(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grunkalunka"))
        .args(arguments)
        .stdin(Stdio::null())
        .output()
        .expect("launch CLI")
}

fn success(arguments: &[&str]) -> String {
    let result = invoke(arguments);
    assert!(result.status.success(), "{result:?}");
    assert!(result.stderr.is_empty());
    String::from_utf8(result.stdout).expect("UTF-8 help")
}

#[test]
fn help_and_short_alias_describe_both_profiles() {
    let text = success(&["--help"]);
    assert_eq!(success(&["-h"]), text);
    for command in [
        "compile SOURCE.{js,ts}",
        "wat SOURCE.{js,ts}",
        "eval OUTPUT.wasm",
        "compile-value SOURCE.{js,ts}",
        "wat-value SOURCE.{js,ts}",
        "eval-value OUTPUT.wasm",
    ] {
        assert!(text.contains(command), "missing {command}");
    }
    assert!(text.contains("refuses existing output"));
    assert!(text.contains("binary64 bit patterns, not decimals"));
    assert!(text.contains("--version"));
    assert!(text.ends_with('\n'));
}

#[test]
fn version_and_short_alias_use_the_package_version() {
    let expected = format!("grunkalunka {}\n", env!("CARGO_PKG_VERSION"));
    assert_eq!(success(&["--version"]), expected);
    assert_eq!(success(&["-V"]), expected);
}

#[test]
fn missing_unknown_and_extra_arguments_still_fail_with_usage() {
    let cases: &[&[&str]] = &[
        &[],
        &["unknown"],
        &["--help", "extra"],
        &["-h", "extra"],
        &["--version", "extra"],
        &["-V", "extra"],
        &["compile"],
        &["compile-value"],
    ];
    for arguments in cases {
        let result = invoke(arguments);
        assert_eq!(result.status.code(), Some(1), "{arguments:?}");
        assert!(result.stdout.is_empty());
        let diagnostic = String::from_utf8(result.stderr).unwrap();
        assert!(diagnostic.starts_with("ERROR: usage: grunkalunka "));
        if arguments.len() == 1 && arguments[0] == "compile-value" {
            assert!(diagnostic.contains("compile-value SOURCE"));
        }
    }
}
