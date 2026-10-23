//! JS/TS parsing, verified numeric region lowering and structured Wasm emission.
//! The admitted numeric contract is documented in docs/compiler.md.
mod emit;
mod ir;
mod lower;

use lower::{Lowering, NumericBinding};

use oxc_allocator::Allocator;
use oxc_ast::ast::{BindingPattern, Declaration, FormalParameterKind, FunctionType, Statement};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType};

pub struct Artifact {
    pub wat: String,
    pub wasm: Vec<u8>,
}

fn unsupported(node: &impl GetSpan, reason: &str) -> String {
    let span = node.span();
    format!(
        "unsupported at bytes {}..{}: {reason}",
        span.start, span.end
    )
}

pub fn compile(source: &str, typescript: bool) -> Result<Artifact, String> {
    if source.len() > 16_384 {
        return Err("source exceeds this experiment's 16 KiB limit".into());
    }
    let allocator = Allocator::default();
    let source_type = if typescript {
        SourceType::ts().with_module(true)
    } else {
        SourceType::mjs()
    };
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if parsed.fatal_error || !parsed.diagnostics.is_empty() {
        return Err(format!("parse diagnostics: {:?}", parsed.diagnostics));
    }
    let built = SemanticBuilder::new_compiler().build(&parsed.program);
    if !built.diagnostics.is_empty() {
        return Err(format!("semantic diagnostics: {:?}", built.diagnostics));
    }
    let program = &parsed.program;
    // The original annotated AST stays available through lowering. Type syntax
    // is unvalidated metadata; the ABI and runtime operands determine admission.
    if program.body.len() != 1 {
        return Err(unsupported(
            program,
            "exactly one exported function is admitted",
        ));
    }
    let Statement::ExportDeclaration(export) = &program.body[0] else {
        return Err(unsupported(program, "expected 'export function run(a, b)'"));
    };
    let Declaration::FunctionDeclaration(function) = &export.declaration else {
        return Err(unsupported(
            export.as_ref(),
            "only a function declaration is admitted",
        ));
    };
    if function.r#type != FunctionType::FunctionDeclaration
        || function
            .id
            .as_ref()
            .is_none_or(|id| id.name.as_str() != "run")
        || function.r#async
        || function.generator
        || function.declare
        || function.type_parameters.is_some()
        || function.this_param.is_some()
    {
        return Err(unsupported(
            function.as_ref(),
            "expected a plain, synchronous, non-generic function named run",
        ));
    }
    if function.params.kind != FormalParameterKind::FormalParameter
        || function.params.items.len() != 2
        || function.params.rest.is_some()
    {
        return Err(unsupported(
            function.params.as_ref(),
            "exactly two ordinary numeric parameters are admitted",
        ));
    }
    let mut bindings = Vec::new();
    let mut slots = Vec::new();
    for parameter in &function.params.items {
        if parameter.optional
            || parameter.initializer.is_some()
            || !parameter.decorators.is_empty()
            || parameter.accessibility.is_some()
            || parameter.readonly
            || parameter.r#override
        {
            return Err(unsupported(
                parameter,
                "parameter modifiers, defaults and decorators are not admitted",
            ));
        }
        let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern else {
            return Err(unsupported(
                parameter,
                "destructured parameters are not admitted",
            ));
        };
        bindings.push(NumericBinding {
            slot: ir::SlotId(bindings.len()),
            symbol: identifier
                .symbol_id
                .get()
                .ok_or("missing parameter binding")?,
            mutable: true,
        });
        slots.push(ir::Slot {
            owner: ir::RegionId(0),
            mutable: true,
            parameter: true,
            span: parameter.span(),
        });
    }
    let body = function.body.as_ref().ok_or("function must have a body")?;
    let mut lowering = Lowering {
        semantic: &built.semantic,
        bindings,
        local_count: 0,
        statement_count: 0,
        in_loop_body: false,
        slots,
        values: Vec::new(),
        next_region: 0,
    };
    // Oxc already recognized these literal expression statements. ESM is strict,
    // and their completion values are unobserved by the numeric return ABI.
    for directive in program.directives.iter().chain(&body.directives) {
        lowering.charge_statement(directive)?;
    }
    let mut region = lowering.region(body.as_ref());
    if lowering.sequence(&body.statements, &mut region, 0)? != ir::Completion::RETURN {
        return Err(unsupported(
            body.as_ref(),
            "every structural path must return a numeric value",
        ));
    }
    let function = lowering.finish(region);
    let wat = emit::emit(&function)?;
    let wasm = wat::parse_str(&wat).map_err(|error| format!("WAT encoding: {error}"))?;
    Ok(Artifact { wat, wasm })
}

#[cfg(test)]
mod tests {
    use super::compile;

    #[test]
    fn annotations_are_metadata_not_numeric_proofs() {
        let plain = compile("export function run(a,b) { const x=a+b; return x; }", false).unwrap();
        let annotated = compile(
            "export function run(a:unknown,b:MissingType):string { const x: { label: string }=a+b; return x; }",
            true,
        )
        .unwrap();
        assert_eq!(plain.wasm, annotated.wasm);
        assert_eq!(plain.wat, annotated.wat);
    }

    #[test]
    fn annotations_do_not_admit_nonnumeric_runtime_values() {
        for initializer in ["\"4\"", "{}", "true", "null", "Math.random()"] {
            let source = format!(
                "export function run(a:any,b:any):number {{ const x:number={initializer}; return x; }}"
            );
            let error = compile(&source, true).err().unwrap();
            assert!(error.contains("expression is outside"), "{error}");
        }
    }

    #[test]
    fn hashbang_preserves_the_numeric_artifact() {
        let source = "export function run(a,b) { return a+b; }";
        let plain = compile(source, false).unwrap();
        for prefix in [
            "#!\n",
            "#!/usr/bin/env node\r\n",
            "#!node\r",
            "#!node\u{2028}",
            "#!node\u{2029}",
            "#!λ 🚀 \u{feff}\n",
        ] {
            let commented = compile(&format!("{prefix}{source}"), false).unwrap();
            assert_eq!(plain.wasm, commented.wasm);
            assert_eq!(plain.wat, commented.wat);
        }
    }

    #[test]
    fn hashbang_is_not_a_source_statement() {
        for (module_count, body_count) in [(255, 0), (0, 255), (127, 128)] {
            let source = format!(
                "#!node\n{}export function run(a,b) {{ {}return a; }}",
                "\"hint\";".repeat(module_count),
                ";".repeat(body_count)
            );
            compile(&source, false).unwrap();
            let source = source.replacen("return a;", ";return a;", 1);
            let error = compile(&source, false).err().unwrap();
            assert!(error.contains("at most 256 statements"), "{error}");
        }
    }

    #[test]
    fn hashbang_uses_the_original_source_byte_budget() {
        let prefix = "#!λ🚀";
        let suffix = "\nexport function run(a,b) { return a; }";
        let mut source = format!(
            "{prefix}{}{suffix}",
            " ".repeat(16_384 - prefix.len() - suffix.len())
        );
        assert_eq!(source.len(), 16_384);
        compile(&source, false).unwrap();
        source.push(' ');
        assert_eq!(source.len(), 16_385);
        assert!(compile(&source, false).err().unwrap().contains("16 KiB"));
    }

    #[test]
    fn hashbang_location_is_checked_on_original_source() {
        let source = "#!node\nexport function run(a,b) { return a; }";
        for prefix in ["\u{feff}", "\u{feff}\u{feff}", " ", "\t", "\n", "/*x*/"] {
            for typescript in [false, true] {
                let error = compile(&format!("{prefix}{source}"), typescript)
                    .err()
                    .expect("misplaced hashbang must remain a syntax error");
                assert!(error.contains("parse diagnostics"), "{error}");
            }
        }
    }

    #[test]
    fn directives_preserve_the_numeric_artifact() {
        let plain = compile("export function run(a,b) { return a+b; }", false).unwrap();
        let decorated = compile("\"module hint\"; export function run(a,b) { \"use strict\"; \"body hint\"; return a+b; }", false).unwrap();
        assert_eq!(plain.wasm, decorated.wasm);
    }

    #[test]
    fn directive_statement_budget_is_shared_with_the_body() {
        for (module_count, body_count) in [(255, 0), (0, 255), (127, 128)] {
            let source = format!(
                "{}export function run(a,b) {{ {}return a; }}",
                "\"hint\";".repeat(module_count),
                "\"hint\";".repeat(body_count)
            );
            compile(&source, false).unwrap();
            let error = compile(&format!("\"one more\";{source}"), false)
                .err()
                .expect("257 source statements must reject");
            assert!(error.contains("at most 256 statements"), "{error}");
        }
    }

    #[test]
    fn directive_text_still_consumes_the_original_source_byte_budget() {
        let source = "\"λ\"; export function run(a,b) { return a; }";
        let mut source = format!("{source}{}", " ".repeat(16_384 - source.len()));
        compile(&source, false).unwrap();
        source.push(' ');
        assert!(compile(&source, false).err().unwrap().contains("16 KiB"));
    }
}
