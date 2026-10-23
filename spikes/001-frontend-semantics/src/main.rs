//! Disposable evidence probe; deliberately not a compiler frontend API.
use std::{collections::BTreeMap, error::Error, fs, path::Path};

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

type Bindings = BTreeMap<String, Vec<usize>>;

fn inspect(name: &str, source: &str, source_type: SourceType) -> Bindings {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    assert!(
        parsed.diagnostics.is_empty(),
        "{name}: {:?}",
        parsed.diagnostics
    );
    assert!(!parsed.fatal_error, "{name}: fatal parse error");
    let built = SemanticBuilder::new_compiler()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert!(
        built.diagnostics.is_empty(),
        "{name}: {:?}",
        built.diagnostics
    );
    let semantic = built.semantic;
    let scoping = semantic.scoping();
    let mut bindings = Bindings::new();
    let mut x_bindings = Vec::new();
    for symbol in scoping.symbol_ids() {
        let symbol_name = scoping.symbol_name(symbol).to_string();
        let mut references: Vec<_> = semantic
            .symbol_references(symbol)
            .map(|reference| semantic.reference_span(reference).start as usize)
            .collect();
        references.sort_unstable();
        if symbol_name == "x" && name.starts_with("bindings") {
            x_bindings.push((
                scoping.symbol_span(symbol).start as usize,
                references.clone(),
            ));
        }
        bindings
            .entry(symbol_name)
            .or_default()
            .push(references.len());
    }
    for counts in bindings.values_mut() {
        counts.sort_unstable();
    }
    let mut unresolved: Vec<_> = scoping
        .root_unresolved_references()
        .keys()
        .map(ToString::to_string)
        .collect();
    unresolved.sort();
    let expected_unresolved = if name.starts_with("ambient") {
        vec!["clock"]
    } else {
        vec![]
    };
    assert_eq!(unresolved, expected_unresolved, "{name}: unresolved names");

    if name.starts_with("bindings") {
        // Check the target of EACH reference, not just indistinguishable counts.
        let outer_decl = source.find("const x").unwrap() + "const ".len();
        let inner_decl = source.find("inner(x").unwrap() + "inner(".len();
        let outer_ref = source.find("return x + inner").unwrap() + "return ".len();
        let inner_ref = source.find("=> x + 1").unwrap() + "=> ".len();
        let mut expected = vec![(outer_decl, vec![outer_ref]), (inner_decl, vec![inner_ref])];
        expected.sort();
        x_bindings.sort();
        assert_eq!(x_bindings, expected, "{name}: shadowing/closure targets");
    }
    let symbol_count: usize = bindings.values().map(Vec::len).sum();
    let resolved_count: usize = bindings.values().flatten().sum();
    println!(
        "PASS {name}: bytes={} symbols={symbol_count} resolved={resolved_count} unresolved={unresolved:?}",
        source.len()
    );
    bindings
}

fn reject_probes() {
    let allocator = Allocator::default();
    let invalid = Parser::new(&allocator, "export function (", SourceType::mjs()).parse();
    assert!(
        !invalid.diagnostics.is_empty(),
        "invalid syntax must produce diagnostics"
    );
    println!(
        "PASS invalid syntax: parser diagnostics={}",
        invalid.diagnostics.len()
    );

    let duplicate = Parser::new(
        &allocator,
        "let duplicate; let duplicate;",
        SourceType::mjs(),
    )
    .parse();
    assert!(!duplicate.fatal_error);
    assert!(
        duplicate.diagnostics.is_empty(),
        "duplicate-binding case must reach semantics"
    );
    let built = SemanticBuilder::new_compiler().build(&duplicate.program);
    assert!(
        !built.diagnostics.is_empty(),
        "duplicate binding must produce semantic diagnostics"
    );
    println!(
        "PASS duplicate binding: semantic diagnostics={}",
        built.diagnostics.len()
    );
}

fn main() -> Result<(), Box<dyn Error>> {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    for group in ["bindings", "ambient", "coercion", "type-mismatch"] {
        let js = fs::read_to_string(fixtures.join(format!("{group}.js")))?;
        let ts = fs::read_to_string(fixtures.join(format!("{group}.ts")))?;
        let js_bindings = inspect(&format!("{group}.js"), &js, SourceType::mjs());
        let ts_bindings = inspect(
            &format!("{group}.ts"),
            &ts,
            SourceType::ts().with_module(true),
        );
        assert_eq!(
            js_bindings, ts_bindings,
            "{group}: paired binding shape differs"
        );
    }
    reject_probes();
    println!(
        "PASS: 8 module fixtures + 2 rejection probes; type mismatch accepted (no TS type checker)"
    );
    Ok(())
}
