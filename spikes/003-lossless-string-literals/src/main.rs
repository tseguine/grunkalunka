//! Disposable evidence probe — do NOT promote into production interfaces.
//!
//! Question: Does Oxc's `JSStr::encode_utf16()` produce the same UTF-16
//! code unit sequence that Node/V8 sees at runtime for the full adversarial
//! corpus of JS string literal forms?
//!
//! Reads fixture JS sources from `fixtures/`, parses each with Oxc, collects
//! every top-level string literal's `encode_utf16()` output, then writes a
//! JSON manifest to stdout for the Node oracle to compare.
use std::error::Error;
use std::fmt::Write as _;

use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;

fn main() -> Result<(), Box<dyn Error>> {
    let fixtures_dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures"));

    // Collect (fixture_name, source) pairs deterministically.
    let mut entries: Vec<(String, String)> = std::fs::read_dir(fixtures_dir)?
        .filter_map(|e| {
            let e = e.ok()?;
            let name = e.file_name().to_string_lossy().into_owned();
            if name.ends_with(".js") {
                let src = std::fs::read_to_string(e.path()).ok()?;
                Some((name, src))
            } else {
                None
            }
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut out = String::from("[\n");
    for (name, source) in &entries {
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();

        // Oxc AST structure: leading bare string literals are parsed as
        // `program.directives` (like ESTree), not as ExpressionStatements.
        // Non-leading bare string literals appear in `program.body`.
        // We collect from both.
        let mut cases: Vec<(String, Vec<u16>)> = Vec::new();

        // 1. Directives — these ARE the string value (unescaped) stored in `directive.expression`.
        for directive in &parsed.program.directives {
            let sl = &directive.expression;
            let label = sl
                .raw
                .map(|r| r.as_str().to_owned())
                .unwrap_or_else(|| "<no-raw>".to_owned());
            let units: Vec<u16> = sl.value.encode_utf16().collect();
            cases.push((label, units));
        }

        // 2. Body: ExpressionStatement whose expression is a StringLiteral.
        // After directives, any bare string expression becomes a body statement.
        for stmt in &parsed.program.body {
            let Statement::ExpressionStatement(es) = stmt else {
                continue;
            };
            let Expression::StringLiteral(sl) = &es.expression else {
                continue;
            };
            let label = sl
                .raw
                .map(|r| r.as_str().to_owned())
                .unwrap_or_else(|| "<no-raw>".to_owned());
            let units: Vec<u16> = sl.value.encode_utf16().collect();
            cases.push((label, units));
        }

        // Emit JSON fragment for this fixture.
        for (label, units) in &cases {
            let units_json = units
                .iter()
                .enumerate()
                .fold(String::new(), |mut s, (i, u)| {
                    if i > 0 {
                        s.push(',');
                    }
                    write!(s, "{u}").unwrap();
                    s
                });
            writeln!(
                out,
                "  {{\"fixture\":{},\"label\":{},\"units\":[{}]}},",
                json_str(name),
                json_str(label),
                units_json,
            )
            .unwrap();
        }
        if cases.is_empty() {
            eprintln!("WARN {name}: no string literals found");
        } else {
            eprintln!("OK   {name}: {} case(s)", cases.len());
        }
    }

    // Strip trailing comma and close array.
    if out.ends_with(",\n") {
        out.truncate(out.len() - 2);
        out.push('\n');
    }
    out.push_str("]\n");
    print!("{out}");
    Ok(())
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                write!(out, "\\u{:04x}", c as u32).unwrap();
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
