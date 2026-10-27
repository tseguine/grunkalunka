//! Encoding regressions independent of the full Markdown/native comparison.
#[path = "fixtures/document-outline/embedding.rs"]
mod embedding;

use wasmtime::Engine;
use wasmtime::component::Component;

const SMALL_ADAPTER: &str = r#"(component
    (core module)
    (core instance $guest (instantiate 0))
    (core module $adapter
        (import "guest" "value" (func $value (result i32)))
        (func (export "run") (result i32) (call $value)))
    (core instance $entry (instantiate $adapter (with "guest" (instance $guest))))
    (func (export "run") (result u32) (canon lift (core func $entry "run")))
)"#;

#[test]
fn raw_embedding_matches_text_encoding_across_section_length_widths() {
    let engine = Engine::default();
    for padding in [0, 128, 16384] {
        let body = format!(
            r#"(memory 1) (data (i32.const 0) "{}")
            (func (export "value") (result i32) (i32.const 42))"#,
            "x".repeat(padding)
        );
        let guest = wat::parse_str(format!("(module {body})")).unwrap();
        let actual = embedding::embed_guest(SMALL_ADAPTER, &guest).unwrap();
        let expected = wat::parse_str(SMALL_ADAPTER.replacen(
            "(core module)",
            &format!("(core module {body})"),
            1,
        ))
        .unwrap();
        assert_eq!(actual, expected, "padding: {padding}");
        Component::from_binary(&engine, &actual).unwrap();
    }
}

#[test]
fn actual_document_adapter_validates_with_the_declared_guest_exports() {
    // ABI-only stub: real parsing and native parity remain in check-document-outline.
    let guest = wat::parse_str(
        r#"(module
            (func (export "outline_begin"))
            (func (export "outline_push") (param i32) (result i32) (i32.const 0))
            (func (export "outline_finish") (result i32) (i32.const 0))
            (func (export "outline_field") (param i32 i32) (result i32) (i32.const -1))
            (func (export "outline_text_byte") (param i32 i32) (result i32) (i32.const -1))
        )"#,
    )
    .unwrap();
    let template = include_str!("fixtures/document-outline/inspector.wat.in");
    let bytes = embedding::embed_guest(template, &guest).unwrap();
    Component::from_binary(&Engine::default(), &bytes).unwrap();
}

#[test]
fn invalid_inputs_and_template_errors_fail_without_echoing_guest_bytes() {
    let empty = wat::parse_str("(module)").unwrap();
    for template in ["(component)", "(component (core module (memory 1)))"] {
        let error = embedding::embed_guest(template, &empty).unwrap_err();
        assert!(error.to_string().contains("unnamed empty core module"));
    }
    for guest in [b"not Wasm".to_vec(), wat::parse_str("(component)").unwrap()] {
        let error = embedding::embed_guest(SMALL_ADAPTER, &guest).unwrap_err();
        assert_eq!(error.to_string(), "expected a core module");
    }
    // Only the header is relevant: malformed template text fails before validation.
    let mut large = empty;
    large.resize(1024 * 1024, 0);
    let error = embedding::embed_guest("(component (core module) (invalid))", &large)
        .unwrap_err()
        .to_string();
    assert!(error.len() < 1024, "template diagnostic must remain small");
}
