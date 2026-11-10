//! Catch changes to the compatibility replay's expected rejections in fast PR CI.
use grunkalunka_compiler::{compile, compile_value};
use serde::Deserialize;
use std::{path::Path, process::Command};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Profile {
    Numeric,
    Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Language {
    Js,
    Ts,
}

#[derive(Deserialize)]
struct Fixture {
    id: String,
    profile: Profile,
    language: Language,
    source: String,
    diagnostic: Option<String>,
}

#[test]
fn compatibility_sources_still_have_their_expected_rejections() {
    let catalog = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/compatibility/compiler-rejection-inputs.mjs");
    let output = Command::new("node")
        .arg(catalog)
        .output()
        .expect("run Node");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let fixtures: Vec<Fixture> =
        serde_json::from_slice(&output.stdout).expect("read compatibility catalog");
    assert!(!fixtures.is_empty(), "empty compatibility catalog");
    let mut failures = Vec::new();
    for fixture in fixtures {
        let typescript = matches!(fixture.language, Language::Ts);
        let result = match fixture.profile {
            Profile::Numeric => compile(&fixture.source, typescript),
            Profile::Value => compile_value(&fixture.source, typescript),
        };
        let mismatch = match (result, fixture.diagnostic) {
            (Ok(_), _) => "unexpected admission".to_owned(),
            (Err(actual), Some(expected)) if !format!("ERROR: {actual}").contains(&expected) => {
                format!("expected {expected:?}, got {actual:?}")
            }
            (Err(_), _) => continue,
        };
        failures.push(format!(
            "{} ({:?}, {:?}): {mismatch}",
            fixture.id, fixture.profile, fixture.language
        ));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
