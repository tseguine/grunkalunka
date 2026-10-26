// Reuse the private, isolated probe without creating a production adapter API.
include!("../src/main.rs");

fn paired_observation(typescript: &str, javascript: &str, expected_runtime: &str) {
    let span = oxc_binary_expression_span(typescript, SourceType::ts().with_module(true)).unwrap();
    let (syntax, semantic, hint, range) =
        check_with_tsrs(typescript, "ts", Some(ExpressionSelector::OxcSpan(span))).unwrap();
    assert_eq!((syntax, semantic), (0, 0), "file diagnostics: {typescript}");
    assert_eq!(hint.as_deref(), Some("number"));
    assert_eq!(range, Some((span.0 as u32, span.1 as u32)));
    let output = std::process::Command::new("node")
        .args(["--input-type=module", "--eval"])
        .arg(format!(
            "{javascript}\nprocess.stdout.write(typeof result + ':' + String(result));"
        ))
        .output()
        .expect("Node is required; a missing oracle is not a passing test");
    assert!(
        output.status.success(),
        "Node failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected_runtime);
}

#[test]
fn original_query_and_diagnostic_playback_is_retained() {
    main().unwrap();
}

#[test]
fn assertion_can_disagree_with_the_runtime_kind_without_file_diagnostics() {
    paired_observation(
        r#"const seed: number = "40" as unknown as number; const result = seed + 2;"#,
        r#"const seed = "40"; const result = seed + 2;"#,
        "string:402",
    );
}

#[test]
fn mutable_alias_can_disagree_without_any_or_assertions() {
    paired_observation(
        r#"
const state: { value: number } = { value: 40 };
const alias: { value: number | string } = state;
alias.value = "40";
const seed = state.value;
const result = seed + 2;
"#,
        r#"
const state = { value: 40 };
const alias = state;
alias.value = "40";
const seed = state.value;
const result = seed + 2;
"#,
        "string:402",
    );
}

#[test]
fn primitive_snapshot_survives_a_later_alias_write() {
    paired_observation(
        r#"
const state: { value: number } = { value: 40 };
const alias: { value: number | string } = state;
const seed = state.value;
alias.value = "40";
const result = seed + 2;
"#,
        r#"
const state = { value: 40 };
const alias = state;
const seed = state.value;
alias.value = "40";
const result = seed + 2;
"#,
        "number:42",
    );
}

#[test]
fn source_handoff_uses_original_utf8_bytes_including_crlf() {
    for prefix in ["", "// café 🦀\n", "/* café 🦀 */\r\n"] {
        let source = format!("{prefix}const seed = 40; const result = seed + 2;");
        for (extension, mode) in [
            ("js", SourceType::mjs()),
            ("ts", SourceType::ts().with_module(true)),
        ] {
            let span = oxc_binary_expression_span(&source, mode).unwrap();
            let start = source.find("seed + 2").unwrap();
            assert_eq!(span, (start, start + "seed + 2".len()));
            let (syntax, semantic, hint, range) =
                check_with_tsrs(&source, extension, Some(ExpressionSelector::OxcSpan(span)))
                    .unwrap();
            assert_eq!((syntax, semantic), (0, 0));
            assert_eq!(hint.as_deref(), Some("number"));
            assert_eq!(range, Some((span.0 as u32, span.1 as u32)));
        }
    }
}

#[test]
fn text_only_selection_must_not_choose_an_arbitrary_duplicate() {
    let source = "const seed = 40; const first = seed + 2; const result = seed + 2;";
    let error = check_with_tsrs(source, "ts", Some(ExpressionSelector::TsrsSource("seed + 2")))
        .unwrap_err();
    assert!(error.to_string().contains("2 matching binary expressions"));
}
