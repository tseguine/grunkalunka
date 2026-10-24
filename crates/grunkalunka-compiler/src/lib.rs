//! JS/TS parsing, verified numeric region lowering and structured Wasm emission.
//! The admitted numeric contract is documented in docs/compiler.md.
mod emit;
mod ir;
mod lower;

use lower::{Lowering, NumericBinding, NumericFunction};

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    BindingPattern, Declaration, ExportDefaultDeclarationKind, FormalParameterKind, FunctionType,
    Statement,
};
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

fn is_type_declaration(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::TSTypeAliasDeclaration(_) | Statement::TSInterfaceDeclaration(_) => true,
        Statement::ExportDeclaration(export) => matches!(
            export.declaration,
            Declaration::TSTypeAliasDeclaration(_) | Declaration::TSInterfaceDeclaration(_)
        ),
        Statement::ExportDefaultDeclaration(export) => matches!(
            export.declaration,
            ExportDefaultDeclarationKind::TSInterfaceDeclaration(_)
        ),
        _ => false,
    }
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
    let runtime: Vec<_> = program
        .body
        .iter()
        .filter(|statement| !is_type_declaration(statement))
        .collect();
    if runtime.is_empty() {
        return Err(unsupported(
            program,
            "exactly one exported function is admitted",
        ));
    }
    // Keep the original single-runtime-item diagnostics, including a bare run.
    if runtime.len() == 1 && !matches!(runtime[0], Statement::ExportDeclaration(_)) {
        return Err(unsupported(program, "expected 'export function run(a, b)'"));
    }
    let mut entry = None;
    let mut helpers = Vec::new();
    for statement in &runtime {
        match statement {
            Statement::ExportDeclaration(export) => {
                let Declaration::FunctionDeclaration(function) = &export.declaration else {
                    return Err(if runtime.len() == 1 {
                        unsupported(export.as_ref(), "only a function declaration is admitted")
                    } else {
                        unsupported(program, "exactly one exported function is admitted")
                    });
                };
                if entry.replace(function.as_ref()).is_some() {
                    return Err(unsupported(
                        program,
                        "exactly one exported function is admitted",
                    ));
                }
            }
            Statement::FunctionDeclaration(function)
                if !function.declare && function.body.is_some() =>
            {
                helpers.push(function.as_ref());
            }
            _ => {
                return Err(unsupported(
                    program,
                    "exactly one exported function is admitted",
                ));
            }
        }
    }
    let entry =
        entry.ok_or_else(|| unsupported(program, "exactly one exported function is admitted"))?;
    let declarations: Vec<_> = std::iter::once(entry).chain(helpers).collect();
    let mut signatures = Vec::new();
    for (index, function) in declarations.iter().enumerate() {
        if function.r#type != FunctionType::FunctionDeclaration
            || function.id.is_none()
            || (index == 0
                && function
                    .id
                    .as_ref()
                    .is_none_or(|id| id.name.as_str() != "run"))
            || function.r#async
            || function.generator
            || function.declare
            || function.type_parameters.is_some()
            || function.this_param.is_some()
        {
            return Err(unsupported(
                *function,
                if index == 0 {
                    "expected a plain, synchronous, non-generic function named run"
                } else {
                    "expected a plain, synchronous, non-generic numeric helper"
                },
            ));
        }
        if function.params.kind != FormalParameterKind::FormalParameter
            || (index == 0 && function.params.items.len() != 2)
            || function.params.rest.is_some()
        {
            return Err(unsupported(
                function.params.as_ref(),
                if index == 0 {
                    "exactly two ordinary numeric parameters are admitted"
                } else {
                    "ordinary numeric helper parameters are required"
                },
            ));
        }
        if function.params.items.len() > 1000 {
            return Err(unsupported(
                function.params.as_ref(),
                "at most 1000 function parameters are admitted",
            ));
        }
        let identifier = function.id.as_ref().ok_or("missing function identifier")?;
        let symbol = identifier
            .symbol_id
            .get()
            .ok_or("missing function binding")?;
        if built
            .semantic
            .scoping()
            .get_resolved_references(symbol)
            .any(|reference| reference.is_write())
        {
            return Err(unsupported(
                identifier,
                "writes to a function binding are not admitted",
            ));
        }
        signatures.push(NumericFunction {
            symbol,
            id: ir::FunctionId(index),
            parameter_count: function.params.items.len(),
        });
    }
    let mut functions = Vec::new();
    let mut local_count = 0;
    let mut statement_count = 0;
    for (index, function) in declarations.iter().enumerate() {
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
                ty: ir::Type::Number,
                owner: ir::RegionId(0),
                mutable: true,
                parameter: true,
                span: parameter.span(),
            });
        }
        let body = function.body.as_ref().ok_or("function must have a body")?;
        let mut lowering = Lowering {
            semantic: &built.semantic,
            functions: &signatures,
            bindings,
            local_count,
            statement_count,
            in_loop_body: false,
            slots,
            values: Vec::new(),
            next_region: 0,
        };
        if index == 0 {
            for directive in &program.directives {
                lowering.charge_statement(directive)?;
            }
            for declaration in program
                .body
                .iter()
                .filter(|statement| is_type_declaration(statement))
            {
                lowering.charge_statement(declaration)?;
            }
        } else {
            lowering.charge_statement(*function)?;
        }
        for directive in &body.directives {
            lowering.charge_statement(directive)?;
        }
        let mut region = lowering.region(body.as_ref());
        if lowering.sequence(&body.statements, &mut region, 0)? != ir::Completion::RETURN {
            return Err(unsupported(
                body.as_ref(),
                "every structural path must return a numeric value",
            ));
        }
        local_count = lowering.local_count;
        statement_count = lowering.statement_count;
        functions.push(lowering.finish(region, function.params.items.len()));
    }
    let module = ir::Module { functions };
    let wat = emit::emit(&module)?;
    let wasm = wat::parse_str(&wat).map_err(|error| format!("WAT encoding: {error}"))?;
    Ok(Artifact { wat, wasm })
}

#[cfg(test)]
mod tests {
    use super::compile;

    #[test]
    fn primitive_boolean_numeric_unary_is_admitted() {
        for expression in [
            "+false", "-false", "+true", "-true", "+(a<b)", "-(a===b)", "+!a", "-!!a",
        ] {
            compile(
                &format!("export function run(a,b) {{ return {expression}; }}"),
                false,
            )
            .unwrap();
        }
    }

    #[test]
    fn primitive_boolean_numeric_unary_keeps_shape_depth_and_bindings() {
        for body in [
            "return !a;",
            "return +(true&&false);",
            "return -(a?true:false);",
            "return +(0,true);",
            "const flag=true; return +flag;",
            "return +(true===a);",
            "return +!(0&&Math.random());",
            "const x=0; return +!(x=1);",
            "return -!x; const x=0;",
        ] {
            assert!(
                compile(&format!("export function run(a,b) {{ {body} }}"), true).is_err(),
                "{body}"
            );
        }
        for sign in ["+", "-"] {
            for (parenthesized, boundary) in [(false, 127), (true, 126)] {
                for count in [boundary, boundary + 1] {
                    let argument = format!("true{}", "!".repeat(count));
                    let argument = if parenthesized {
                        format!("({argument})")
                    } else {
                        argument
                    };
                    let source = format!("export function run(a,b) {{ return {sign}{argument}; }}");
                    assert_eq!(
                        compile(&source, true).is_ok(),
                        count == boundary,
                        "{source}"
                    );
                }
            }
            for count in [255, 256] {
                let source = format!(
                    "export function run(a,b) {{ {}return {sign}true; }}",
                    "0;".repeat(count)
                );
                assert_eq!(compile(&source, false).is_ok(), count == 255);
            }
        }
    }

    #[test]
    fn boolean_bit_not_original_requirements_are_admitted() {
        let outcomes: Vec<_> = [
            ("export function run(a,b) { return ~true; }", false),
            ("export function run(a,b) { return ~(a<b); }", false),
            ("export function run(a,b) { return ~(!a); }", false),
            (
                "export function run(a,b) { return ~(true as number); }",
                true,
            ),
        ]
        .into_iter()
        .map(|(source, ts)| compile(source, ts).map(|_| ()))
        .collect();
        assert!(outcomes.iter().all(Result::is_ok), "{outcomes:?}");
    }

    #[test]
    fn boolean_bit_not_keeps_shape_depth_and_source_guards() {
        for body in [
            "return true;",
            "const x=true; return ~x;",
            "return ~(a?true:false);",
            "return ~(true&&false);",
            "return ~!(0&&Math.random());",
            "return Math.abs(true);",
            "return ~(true==a);",
        ] {
            assert!(
                compile(&format!("export function run(a,b) {{ {body} }}"), true).is_err(),
                "{body}"
            );
        }
        for count in [127, 128] {
            let source = format!(
                "export function run(a,b) {{ return ~true{}; }}",
                "!".repeat(count)
            );
            let result = compile(&source, true);
            if count == 127 {
                result.unwrap();
            } else {
                assert!(result.err().unwrap().contains("nesting exceeds"));
            }
        }
        for count in [126, 127] {
            let source = format!(
                "export function run(a,b) {{ return ~(true{}); }}",
                "!".repeat(count)
            );
            assert_eq!(compile(&source, true).is_ok(), count == 126);
        }
        for count in [126, 127] {
            let source = format!(
                "export function run(a,b) {{ return ~!a{}; }}",
                "!".repeat(count)
            );
            assert_eq!(compile(&source, true).is_ok(), count == 126);
        }
        let source = format!(
            "export function run(a,b) {{ {} return ~true; }}",
            ";".repeat(255)
        );
        compile(&source, false).unwrap();
        assert!(compile(&source.replacen("return", ";return", 1), false).is_err());
        let artifact = compile(
            "export function run(a,b) { return ~(true as number); }",
            true,
        )
        .unwrap();
        assert!(artifact.wat.contains("f64.convert_i32_u"));
    }

    #[test]
    fn typescript_wrapped_write_targets_are_admitted() {
        for source in [
            "export function run(a:number,b:number):number { return (a as number)=b; }",
            "export function run(a:number,b:number):number { return (a as number)++; }",
        ] {
            compile(source, true).expect("retained TS wrappers must preserve the numeric target");
        }
    }

    #[test]
    fn typescript_wrapped_targets_preserve_depth_and_binding_guards() {
        for count in [128, 129] {
            for operation in ["=b", "++"] {
                let source = format!(
                    "export function run(a,b) {{ return a{} {operation}; }}",
                    "!".repeat(count)
                );
                let result = compile(&source, true);
                if count == 128 {
                    result.unwrap();
                } else {
                    assert!(result.err().unwrap().contains("expression nesting exceeds"));
                }
            }
        }
        for (count, admitted) in [(126, true), (127, false)] {
            let source = format!(
                "export function run(a,b) {{ return a=(b{} =a); }}",
                "!".repeat(count)
            );
            assert_eq!(compile(&source, true).is_ok(), admitted, "{source}");
        }
        for (count, admitted) in [(126, true), (127, false)] {
            let source = format!(
                "export function run(a,b) {{ return ((a{}) as number)=b; }}",
                "!".repeat(count)
            );
            assert_eq!(compile(&source, true).is_ok(), admitted, "{source}");
        }
        for (body, reason) in [
            ("const x=a; return (x as unknown)=b;", "const binding"),
            (
                "const x=a; if(0) (x as unknown)++; return a;",
                "const binding",
            ),
            ("let x=(x as number)=b; return x;", "not an initialized"),
            ("return (x as number)++; let x=a;", "not an initialized"),
            ("return (clock as number)=a;", "not an initialized"),
            ("return (Infinity as number)++;", "not an initialized"),
            ("return (a.x as number)=b;", "assignment target"),
            ("return (a[b] as number)++;", "update target"),
            ("return (a as number)**=b;", "only ="),
        ] {
            let error = compile(&format!("export function run(a,b) {{ {body} }}"), true)
                .err()
                .unwrap();
            assert!(error.contains(reason), "{body}: {error}");
        }
        let error = compile(
            "export function run(a,b) { let x=a; { let x=(x as number)=b; } return x; }",
            true,
        )
        .err()
        .unwrap();
        assert!(error.contains("not an initialized"));
        let error=compile("function unused(x) { const y=x; (y as number)++; return x; } export function run(a,b) { return a; }",true).err().unwrap();
        assert!(error.contains("const binding"));
        for target in [
            "Math as unknown",
            "<number>Math",
            "Math!",
            "Math satisfies number",
        ] {
            for source in [
                format!("export function run(a,b) {{ if(0) ({target})=a; return Math.abs(a); }}"),
                format!(
                    "function unused(x) {{ ({target})++; return x; }} export function run(a,b) {{ return Math.abs(a); }}"
                ),
            ] {
                assert!(
                    compile(&source, true)
                        .err()
                        .unwrap()
                        .contains("not an initialized")
                );
            }
        }
        assert!(
            compile(
                "export function run(Math,b) { (Math as number)+=b; return Math; }",
                true
            )
            .is_ok()
        );
        assert!(
            compile(
                "export function run(a,b) { const Math=a; (Math as number)=b; return a; }",
                true
            )
            .err()
            .unwrap()
            .contains("const binding")
        );
        for target in ["f as number", "<number>f", "f!", "f satisfies number"] {
            for operation in ["=a", "+=a", "++"] {
                let source = format!(
                    "function f(x) {{ return x; }} function unused(x) {{ if(0) ({target}){operation}; return x; }} export function run(a,b) {{ return a; }}"
                );
                assert!(
                    compile(&source, true)
                        .err()
                        .unwrap()
                        .contains("writes to a function binding")
                );
            }
        }
        for target in [
            "({} as number)",
            "(Math.random() as number)",
            "([a] as number)",
        ] {
            assert!(
                compile(
                    &format!("export function run(a,b) {{ return {target}=b; }}"),
                    true
                )
                .is_err()
            );
        }
    }

    #[test]
    fn remainder_binary_is_admitted() {
        compile("export function run(a,b) { return a % b; }", false)
            .expect("Number remainder must compile");
    }

    #[test]
    fn remainder_assignment_is_admitted() {
        compile(
            "export function run(a,b) { return (a as number)%=b; }",
            true,
        )
        .expect("remainder assignment retains wrapped numeric target");
        compile("export function run(a,b) { return a %= b; }", false)
            .expect("Number remainder assignment must compile");
    }

    #[test]
    fn remainder_helper_is_private_and_demanded_in_nested_regions() {
        let plain = compile("export function run(a,b) { return a+b; }", false).unwrap();
        assert!(!plain.wat.contains("$number_remainder"));
        for body in [
            "if(a) return b%a; return a;",
            "return a ? a : b%a;",
            "for(let i=5%2;i<2;i++) {} return a;",
            "for(let i=0;i%3<2;i++) {} return a;",
            "for(let i=0;i<2;i=(i+1)%3) {} return a;",
            "do { a%=3; } while(0); return a;",
        ] {
            let artifact =
                compile(&format!("export function run(a,b) {{ {body} }}"), false).unwrap();
            assert_eq!(artifact.wat.matches("(func $number_remainder").count(), 1);
            assert!(
                artifact.wat.find("(export \"run\")").unwrap()
                    < artifact.wat.find("(func $number_remainder").unwrap()
            );
        }
        for source in [
            "function unused(x,y) { return x%y; } export function run(a,b) { return a+b; }",
            "function number_remainder(x,y) { return x%y; } export function run(a,b) { return number_remainder(a,b); }",
        ] {
            let artifact = compile(source, false).unwrap();
            assert_eq!(artifact.wat.matches("(func $number_remainder").count(), 1);
            assert_eq!(artifact.wat.matches("(func").count(), 3);
            if source.contains("return number_remainder(a,b)") {
                assert!(artifact.wat.contains("call 1"));
            }
        }
    }

    #[test]
    fn remainder_keeps_numeric_target_and_source_budget_guards() {
        for (body, reason, typescript) in [
            ("return a%\"3\";", "expression is outside", false),
            ("return 1n%2n;", "expression is outside", false),
            ("const x=a; return x%=b;", "const binding", false),
            ("let x=(x%=b); return x;", "not an initialized", false),
            ("return a.x%=b;", "assignment target", false),
            (
                "if(false) return a%Math.random(); return a;",
                "expression is outside",
                false,
            ),
        ] {
            let error = compile(
                &format!("export function run(a,b) {{ {body} }}"),
                typescript,
            )
            .err()
            .unwrap();
            assert!(error.contains(reason), "{body}: {error}");
        }
        let source = format!(
            "export function run(a,b) {{ {} return a%b; }}",
            ";".repeat(255)
        );
        compile(&source, false).unwrap();
        let error = compile(&source.replacen("return", ";return", 1), false)
            .err()
            .unwrap();
        assert!(error.contains("at most 256 statements"));
    }

    #[test]
    fn direct_numeric_helpers_with_distinct_arities_are_admitted() {
        for source in [
            "function value() { return 7; } export function run(a,b) { return value(); }",
            "export function run(a,b) { return inc(a); } function inc(x) { return x+1; }",
            "function pack(x,y,z) { return x*100+y*10+z; } export function run(a,b) { return pack(a,b,3); }",
        ] {
            compile(source, false).unwrap();
        }
    }

    #[test]
    fn type_aliases_preserve_the_numeric_artifact() {
        let plain = compile("export function run(a,b) { return a; }", false).unwrap();
        let typed = compile(
            "type Scalar = number; export function run(a:Scalar,b:Scalar):Scalar { return a; }",
            true,
        )
        .unwrap();
        assert_eq!(plain.wasm, typed.wasm);
        assert_eq!(plain.wat, typed.wat);
    }

    #[test]
    fn type_declarations_preserve_abrupt_completions() {
        let plain = compile("export function run(a,b) { return a; }", false).unwrap();
        for body in ["type T=number; return a;", "return a; interface T {}"] {
            let typed = compile(&format!("export function run(a,b) {{ {body} }}"), true).unwrap();
            assert_eq!(plain.wasm, typed.wasm);
        }
    }

    #[test]
    fn type_declarations_share_the_statement_budget() {
        for (module_count, body_count, exported, directive) in [
            (255, 0, false, false),
            (0, 255, false, false),
            (127, 127, false, true),
            (255, 0, true, false),
        ] {
            let module = (0..module_count)
                .map(|i| format!("{}type M{i}=number;", if exported { "export " } else { "" }))
                .collect::<String>();
            let body = (0..body_count)
                .map(|i| format!("interface B{i} {{}}"))
                .collect::<String>();
            let source = format!(
                "{}{module}export function run(a,b) {{ return a; {body} }}",
                if directive { "\"hint\";" } else { "" }
            );
            compile(&source, true).unwrap();
            let over = source.replacen("return a;", "return a; type Extra=number;", 1);
            let error = compile(&over, true).err().unwrap();
            assert!(error.contains("at most 256 statements"), "{error}");
        }
    }

    #[test]
    fn type_declarations_do_not_allow_runtime_abrupt_tails() {
        for body in [
            "return a; type T=number; a=b;",
            "do { break; interface T {} a=b; } while(0); return a;",
            "for(let i=0;i<1;i++) { continue; type T=number; a=b; } return a;",
            "do { if(a) return a; else break; type T=number; a=b; } while(0); return b;",
        ] {
            let error = compile(&format!("export function run(a,b) {{ {body} }}"), true)
                .err()
                .unwrap();
            assert!(error.contains("statements after a guaranteed"), "{error}");
        }
    }

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
            let reason = if initializer == "true" {
                "Boolean bindings are not admitted in numeric value positions"
            } else {
                "expression is outside"
            };
            assert!(error.contains(reason), "{error}");
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

    #[test]
    fn discarded_boolean_statement() {
        compile("export function run(a,b) { true; return a; }", false).unwrap();
    }

    #[test]
    fn immutable_boolean_literals_and_comparisons_are_predicate_cells() {
        for source in [
            "export function run(a,b) { const flag=true; return flag?a:b; }",
            "export function run(a,b) { const flag=a<b; const copy=flag; return copy?1:0; }",
        ] {
            compile(source, false).unwrap();
        }
    }

    #[test]
    fn boolean_cell_numeric_frontiers_keep_original_source_spans() {
        for (source, typescript, span) in [
            (
                "export function run(a,b) { const x=a<b; return x; }",
                false,
                "47..48",
            ),
            (
                "export function run(a:unknown,b:unknown):unknown { const x:number=true; return x; }",
                true,
                "79..80",
            ),
        ] {
            assert_eq!(
                compile(source, typescript).err().unwrap(),
                format!(
                    "unsupported at bytes {span}: Boolean bindings are not admitted in numeric value positions"
                )
            );
        }
    }

    #[test]
    fn boolean_cells_preserve_tdz_and_numeric_only_uses() {
        for (source, reason) in [
            (
                "const flag=true; { const flag=flag; } return a;",
                "reference is not an initialized",
            ),
            (
                "const flag=true; { const x=flag; const flag=false; } return a;",
                "reference is not an initialized",
            ),
            (
                "const flag=true; flag; return a;",
                "Boolean bindings are not admitted",
            ),
            (
                "const flag=true; return ~flag;",
                "Boolean bindings are not admitted",
            ),
            (
                "const flag=true; let copy=flag; return a;",
                "Boolean bindings are not admitted",
            ),
            ("const flag=true&&a; return a;", "expression is outside"),
        ] {
            let source = format!("export function run(a,b) {{ {source} }}");
            assert!(compile(&source, false).err().unwrap().contains(reason));
        }
    }

    #[test]
    fn discarded_comparison_in_numeric_comma() {
        compile("export function run(a,b) { return (a++<b,a); }", false).unwrap();
    }

    #[test]
    fn discarded_mixed_logical_result() {
        compile(
            "export function run(a,b) { a && (b++<a); return b; }",
            false,
        )
        .unwrap();
    }

    #[test]
    fn discarded_mixed_conditional_result() {
        compile(
            "export function run(a,b) { a ? b++ : false; return b; }",
            false,
        )
        .unwrap();
    }

    #[test]
    fn boolean_arithmetic_operands_use_numeric_results() {
        for (body, typescript) in [
            ("return true+a;", false),
            ("return a-false;", false),
            ("return false*true;", false),
            ("return (a<b)/(!a);", false),
            ("return a%(a===b);", false),
            ("return true+1;", true),
            ("return ~((a<b) as number)+true;", true),
            ("return a%true;", false),
        ] {
            compile(
                &format!("export function run(a,b) {{ {body} }}"),
                typescript,
            )
            .unwrap();
        }
    }

    #[test]
    fn boolean_arithmetic_keeps_other_operand_boundaries() {
        for (body, reason) in [
            (
                "const flag=true; return flag+a;",
                "Boolean bindings are not admitted",
            ),
            ("a+=true; return a;", "expression is outside"),
            ("return true==a?1:0;", "expression is outside"),
            ("return a&true;", "expression is outside"),
            ("return (a?true:false)+1;", "expression is outside"),
        ] {
            let source = format!("export function run(a,b) {{ {body} }}");
            assert!(compile(&source, false).err().unwrap().contains(reason));
        }
    }
}
