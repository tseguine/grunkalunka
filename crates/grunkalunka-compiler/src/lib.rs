//! JS/TS parsing, verified typed region lowering and structured Wasm emission.
//! The admitted numeric contract is documented in docs/compiler.md.
mod emit;
#[cfg(test)]
mod evaluator_tests;
mod heap;
mod ir;
mod lower;

use lower::{Lowering, NumericBinding, NumericFunction};

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    BindingPattern, Declaration, ExportDefaultDeclarationKind, FormalParameterKind, FunctionType,
    ImportOrExportKind, Statement,
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

fn is_type_module_item(statement: &Statement<'_>) -> bool {
    is_type_declaration(statement)
        || match statement {
            Statement::ImportDeclaration(import) => import.import_kind == ImportOrExportKind::Type,
            Statement::ExportNamedDeclaration(export) => {
                export.export_kind == ImportOrExportKind::Type
            }
            Statement::ExportFromDeclaration(export) => {
                export.export_kind == ImportOrExportKind::Type
            }
            Statement::ExportAllDeclaration(export) => {
                export.export_kind == ImportOrExportKind::Type
            }
            _ => false,
        }
}

fn lower_source(source: &str, typescript: bool) -> Result<ir::Module, String> {
    lower_source_with_entry_type(source, typescript, ir::Type::Number)
}

fn lower_source_with_entry_type(
    source: &str,
    typescript: bool,
    entry_type: ir::Type,
) -> Result<ir::Module, String> {
    lower_source_profile(source, typescript, entry_type, false)
}

fn lower_source_profile(
    source: &str,
    typescript: bool,
    entry_type: ir::Type,
    async_entry: bool,
) -> Result<ir::Module, String> {
    lower_source_profile_with_identities(source, typescript, entry_type, async_entry, async_entry)
}

// Snapshot IR fixtures can exercise the logical completion operations without
// depending on the public async frontend's constructor normalization.
fn lower_source_profile_with_identities(
    source: &str,
    typescript: bool,
    entry_type: ir::Type,
    async_entry: bool,
    identities: bool,
) -> Result<ir::Module, String> {
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
    let built = SemanticBuilder::new_compiler()
        .with_build_nodes(true)
        .build(&parsed.program);
    if !built.diagnostics.is_empty() {
        return Err(format!("semantic diagnostics: {:?}", built.diagnostics));
    }
    let program = &parsed.program;
    // The original annotated AST stays available through lowering. Type syntax
    // is unvalidated metadata; the ABI and runtime operands determine admission.
    let runtime: Vec<_> = program
        .body
        .iter()
        .filter(|statement| !is_type_module_item(statement))
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
    let closure_signatures = std::cell::RefCell::new(ir::ClosureSignatures::default());
    let mut signatures = Vec::new();
    for (index, function) in declarations.iter().enumerate() {
        if function.r#type != FunctionType::FunctionDeclaration
            || function.id.is_none()
            || (index == 0
                && function
                    .id
                    .as_ref()
                    .is_none_or(|id| id.name.as_str() != "run"))
            || (index == 0 && function.r#async != async_entry)
            || (index != 0 && function.r#async && !async_entry)
            || function.generator
            || function.declare
            || function.this_param.is_some()
        {
            return Err(unsupported(
                *function,
                if index == 0 {
                    if async_entry {
                        "expected a plain async function named run"
                    } else {
                        "expected a plain, synchronous function named run"
                    }
                } else {
                    "expected a plain, synchronous numeric helper"
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
            return_type: if index == 0 || function.r#async {
                entry_type
            } else {
                ir::Type::Number
            },
            async_signature: (index != 0 && function.r#async).then(|| {
                closure_signatures
                    .borrow_mut()
                    .intern_async(vec![ir::Type::Number; function.params.items.len()])
            }),
        });
    }
    let promise_parameters =
        lower::infer_promise_parameters(&built.semantic, identities, &signatures);
    let callback_parameters = lower::infer_callback_parameters(
        &built.semantic,
        &signatures,
        &promise_parameters,
        &closure_signatures,
    )?;
    let mut general_bindings = lower::general_mutable_bindings(
        &built.semantic,
        &signatures,
        &callback_parameters,
        &closure_signatures.borrow(),
        &promise_parameters,
    );
    general_bindings
        .async_results
        .extend(&promise_parameters.asynchronous);
    let mut functions = Vec::new();
    let mut lifted = Vec::new();
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
            return_type: signatures[index].return_type,
            semantic: &built.semantic,
            functions: &signatures,
            general_bindings: &general_bindings.storage,
            general_parameters: &general_bindings.inputs,
            number_calls: &general_bindings.number_calls,
            general_results: &general_bindings.general_results,
            settled_results: &general_bindings.settled_results,
            async_results: &general_bindings.async_results,
            callback_parameters: &callback_parameters,
            promise_parameters: &promise_parameters,
            closure_signatures: &closure_signatures,
            bindings,
            local_count,
            statement_count,
            in_loop_body: false,
            closures: Vec::new(),
            closure_expansions: 0,
            active_local_calls: 0,
            active_block_calls: 0,
            source_charging: true,
            inline_depth: 0,
            statement_depth: 0,
            allow_await: function.r#async && async_entry,
            allow_async_closures: async_entry,
            lifted: Vec::new(),
            lifted_base: signatures.len() + lifted.len(),
            lifting_depth: 0,
            active_function_values: 0,
            active_function_results: 0,
            function_result_depth: 0,
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
                .filter(|statement| is_type_module_item(statement))
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
        if signatures[index].async_signature.is_some() {
            region.operations.push(ir::Operation {
                span: body.span(),
                kind: ir::Op::AsyncClosureEntry {
                    captures: Vec::new(),
                },
            });
        }
        lowering.initialize_general_parameters(&mut region);
        let mut completion = lowering.sequence(&body.statements, &mut region, 0)?;
        if lowering.allow_await && completion.normal {
            lowering.return_undefined(body.as_ref(), &mut region);
            completion = completion.then(ir::Completion::RETURN);
        }
        if completion != ir::Completion::RETURN
            && !(lowering.allow_await && completion.returns_or_throws())
        {
            return Err(unsupported(
                body.as_ref(),
                if lowering.allow_await {
                    "every structural path must return a primitive value or throw"
                } else if lowering.return_type == ir::Type::General {
                    "every structural path must return a primitive value"
                } else {
                    "every structural path must return a numeric value"
                },
            ));
        }
        local_count = lowering.local_count;
        statement_count = lowering.statement_count;
        lifted.append(&mut lowering.lifted);
        functions.push(lowering.finish(region, function.params.items.len()));
    }
    functions.extend(lifted.into_iter().map(|function| {
        std::rc::Rc::try_unwrap(function)
            .expect("completed lifted body has no remaining lowering owner")
    }));
    Ok(ir::Module {
        functions,
        closure_signatures: closure_signatures.into_inner(),
    })
}

pub fn compile(source: &str, typescript: bool) -> Result<Artifact, String> {
    let module = lower_source(source, typescript)?;
    let wat = emit::emit(&module)?;
    let wasm = wat::parse_str(&wat).map_err(|error| format!("WAT encoding: {error}"))?;
    Ok(Artifact { wat, wasm })
}

/// Compile an entry returning a tagged primitive value, with two Number inputs.
pub fn compile_value(source: &str, typescript: bool) -> Result<Artifact, String> {
    let module = lower_source_with_entry_type(source, typescript, ir::Type::General)?;
    let wat = emit::emit_with_entry_type(&module, ir::Type::General)?;
    let wasm = wat::parse_str(&wat).map_err(|error| format!("WAT encoding: {error}"))?;
    Ok(Artifact { wat, wasm })
}

/// Compile the experimental String/JSON-expression heap-return profile.
pub fn compile_heap_value(source: &str, typescript: bool) -> Result<Artifact, String> {
    heap::compile(source, typescript)
}

/// Compile a primitive-returning async entry into a tagged start/resume protocol.
pub fn compile_async(source: &str, typescript: bool) -> Result<Artifact, String> {
    let module = lower_source_profile(source, typescript, ir::Type::General, true)?;
    let wat = emit::emit_async(&module)?;
    let wasm = wat::parse_str(&wat).map_err(|error| format!("WAT encoding: {error}"))?;
    Ok(Artifact { wat, wasm })
}

#[cfg(test)]
mod tests {
    use super::compile;

    #[test]
    fn strict_boolean_equality_admits_same_kind_operands() {
        for source in [
            "export function run(a,b) { if(true===false) return a; return b; }",
            "export function run(a,b) { if(!a===!b) return a; return b; }",
            "export function run(a,b) { const flag=a<b; return flag!==true?1:0; }",
        ] {
            compile(source, false).unwrap();
        }
    }

    #[test]
    fn strict_boolean_equality_keeps_kind_scope_and_depth_boundaries() {
        for body in [
            "return true===false;",
            "const flag=flag===true; return a;",
            "const flag=true; { const copy=flag===true; const flag=false; } return a;",
            "return false?(true===!Math.random()?1:0):a;",
        ] {
            assert!(
                compile(&format!("export function run(a,b) {{ {body} }}"), false).is_err(),
                "{body}"
            );
        }
        for depth in [126, 127] {
            let source = format!(
                "export function run(a,b) {{ return {}true===false{}?a:b; }}",
                "(".repeat(depth),
                ")".repeat(depth)
            );
            assert_eq!(compile(&source, false).is_ok(), depth == 126);
            let source = format!(
                "export function run(a,b) {{ return true{} ===false?a:b; }}",
                "!".repeat(depth)
            );
            assert_eq!(compile(&source, true).is_ok(), depth == 126);
        }
    }

    #[test]
    fn discarded_immutable_predicate_bindings_are_admitted() {
        let mut failures = Vec::new();
        for (id, body) in [
            (
                "const-boolean-statement",
                "const flag=true; flag; return a;",
            ),
            (
                "unary-predicate-binding-reject-discard",
                "const flag=true; flag; return a;",
            ),
            (
                "arithmetic-predicate-binding-reject-discard",
                "const flag=true; flag; return a;",
            ),
            (
                "const-boolean-comma-prefix",
                "const flag=true; return (flag,a);",
            ),
            (
                "const-boolean-for-update",
                "const flag=true; for(let i=0;i<1;flag) { i++; } return a;",
            ),
        ] {
            if let Err(error) = compile(&format!("export function run(a,b) {{ {body} }}"), false) {
                failures.push((id, error));
            }
        }
        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn discarded_predicate_bindings_keep_root_joins_and_scope_guards() {
        for body in [
            "const flag=true; { flag; const flag=false; } return a;",
            "{const flag=true;} flag; return a;",
            "clock; return a;",
            "const flag=true; a=flag; return a;",
        ] {
            assert!(
                compile(&format!("export function run(a,b) {{ {body} }}"), false).is_err(),
                "{body}"
            );
        }
        for depth in [128, 129] {
            let source = format!(
                "export function run(a,b) {{ const flag=true; flag{},a; return a; }}",
                "!".repeat(depth - 1)
            );
            assert_eq!(compile(&source, true).is_ok(), depth == 128, "{source}");
        }
    }

    #[test]
    fn arithmetic_operators_read_immutable_predicate_bindings() {
        let mut failures = Vec::new();
        for (id, body) in [
            (
                "const-boolean-arithmetic",
                "const flag=true; return flag+a;",
            ),
            (
                "boolean-arithmetic-bare-const",
                "const flag=true; return flag+a;",
            ),
            (
                "unary-predicate-binding-reject-arithmetic",
                "const flag=true; return flag+a;",
            ),
            (
                "boolean-arithmetic-bare-const-right",
                "const flag=true; return a*flag;",
            ),
        ] {
            if let Err(error) = compile(&format!("export function run(a,b) {{ {body} }}"), false) {
                failures.push((id, error));
            }
        }
        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn unary_operators_read_immutable_predicate_bindings() {
        for body in [
            "const flag=true; return +flag;",
            "const flag=true; return ~flag;",
            "const x=true; return ~x;",
            "const flag=false; return -flag;",
            "const one=a<b; const two=one; return +two;",
        ] {
            compile(&format!("export function run(a,b) {{ {body} }}"), false).unwrap();
        }
    }

    #[test]
    fn unary_math_converts_original_boolean_guard_sources() {
        for source in [
            "export function run(a,b) { return Math.abs(true); }",
            "export function run(a,b) { const flag=true; return Math.abs(flag); }",
        ] {
            for typescript in [false, true] {
                compile(source, typescript).unwrap();
                crate::compile_value(source, typescript).unwrap();
            }
        }
    }

    #[test]
    fn unary_predicate_binding_bridge_keeps_scope_and_value_boundaries() {
        for body in [
            "const flag=true; return flag;",
            "const flag=+flag; return a;",
            "return +flag; const flag=true;",
            "{const flag=true;} return +flag;",
        ] {
            assert!(
                compile(&format!("export function run(a,b) {{ {body} }}"), false).is_err(),
                "{body}"
            );
        }
        for operator in ["+", "-", "~"] {
            for depth in [127, 128] {
                let source = format!(
                    "export function run(a,b) {{ const flag=true; return {operator}flag{}; }}",
                    "!".repeat(depth)
                );
                assert_eq!(compile(&source, true).is_ok(), depth == 127, "{source}");
            }
        }
    }

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
            (
                "export function run(a,b) { return ~(a?true:false); }",
                false,
            ),
            ("export function run(a,b) { return ~(true&&false); }", false),
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
        for body in ["return true;", "return ~!(0&&Math.random());"] {
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
            } else if initializer == "null" {
                "General bindings require a primitive value position"
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
            ("const flag=true; return flag+a;", false),
        ] {
            compile(
                &format!("export function run(a,b) {{ {body} }}"),
                typescript,
            )
            .unwrap();
        }
    }

    #[test]
    fn binary_bitwise_converts_former_boolean_guard_sources() {
        for source in [
            "export function run(a,b) { return a&true; }",
            "export function run(a,b) { return true<<a; }",
            "export function run(a,b) { const flag=true; return flag|a; }",
            "export function run(a,b) { const flag=true; return flag<<a; }",
        ] {
            for typescript in [false, true] {
                compile(source, typescript).unwrap();
                crate::compile_value(source, typescript).unwrap();
            }
        }
    }

    #[test]
    fn bitwise_compound_original_boolean_guard_is_admitted() {
        let source = "export function run(a,b) { a&=true; return a; }";
        for typescript in [false, true] {
            compile(source, typescript).unwrap();
            crate::compile_value(source, typescript).unwrap();
        }
    }
}

#[cfg(test)]
mod type_only_module_tests {
    use super::compile;

    fn erases_to(source: &str, javascript: &str) {
        let expected = compile(javascript, false).unwrap();
        let actual = compile(source, true).unwrap();
        assert_eq!(actual.wat, expected.wat);
        assert_eq!(actual.wasm, expected.wasm);
    }

    #[test]
    fn declaration_type_import_erases() {
        erases_to(
            "import type { T } from \"missing\";export function run(a,b) { return a; }",
            "export function run(a,b) { return a; }",
        );
    }

    #[test]
    fn declaration_type_export_list_erases() {
        erases_to(
            "type T=number; export type { T };export function run(a,b) { return a; }",
            "export function run(a,b) { return a; }",
        );
    }

    #[test]
    fn declaration_type_re_export_erases() {
        erases_to(
            "export type { T } from \"missing\";export function run(a,b) { return a; }",
            "export function run(a,b) { return a; }",
        );
    }

    #[test]
    fn declaration_type_export_star_erases() {
        erases_to(
            "export type * from \"missing\";export function run(a,b) { return a; }",
            "export function run(a,b) { return a; }",
        );
    }
    #[test]
    fn declaration_type_forms_preserve_source_order_and_scalar_artifacts() {
        for declaration in [
            "import type T from \"missing\";",
            "import type * as Types from \"missing\";",
            "import type { T as Local, U } from \"missing\";",
            "type T=number; interface U {} export type { T as Alias, U };",
            "export type {};",
            "export type { T as Local, U } from \"missing\";",
            "export type * as Types from \"missing\";",
        ] {
            let javascript =
                "function twice(x) { return x*2; } export function run(a,b) { return twice(a)+b; }";
            erases_to(&format!("{declaration}{javascript}"), javascript);
            erases_to(&format!("{javascript}{declaration}"), javascript);
        }
    }

    #[test]
    fn type_only_items_do_not_erase_runtime_module_forms() {
        for declaration in [
            "import { T } from \"missing\";",
            "import \"missing\";",
            "import {} from \"missing\";",
            "import { type T } from \"missing\";",
            "import { type T, value } from \"missing\";",
            "type T=number; export { type T };",
            "export {};",
            "export { T } from \"missing\";",
            "export { type T } from \"missing\";",
            "export * from \"missing\";",
            "import T = require(\"missing\");",
            "import type T = require(\"missing\");",
            "declare const value:number;",
        ] {
            let source = format!("{declaration}export function run(a,b) {{ return a; }}");
            let error = compile(&source, true).err().expect(&source);
            assert!(
                error.contains("exactly one exported function"),
                "{source}: {error}"
            );
        }
    }

    #[test]
    fn type_only_items_preserve_value_namespace_eligibility() {
        for (source, javascript) in [
            (
                "import type { Math } from \"missing\";export function run(a,b) { return Math.abs(a)+b; }",
                "export function run(a,b) { return Math.abs(a)+b; }",
            ),
            (
                "import type { Infinity, NaN } from \"missing\";export function run(a,b) { return Infinity+NaN; }",
                "export function run(a,b) { return Infinity+NaN; }",
            ),
            (
                "import type { Math } from \"missing\";export function run(a,b) { const Math=a; return Math+b; }",
                "export function run(a,b) { const Math=a; return Math+b; }",
            ),
        ] {
            erases_to(source, javascript);
        }
        for source in [
            "import type { T } from \"missing\";export function run(a,b) { return T; }",
            "import type { Math } from \"missing\";export function run(a,b) { const Math=a; return Math.abs(a); }",
            "import type { T } from \"missing\";export function run(a,b) { const T=T; return T; }",
        ] {
            assert!(compile(source, true).is_err(), "{source}");
        }
    }

    #[test]
    fn type_only_items_share_existing_statement_and_source_budgets() {
        for kind in 0..4 {
            let declarations = |count| {
                (0..count)
                    .map(|i| match kind {
                        0 => format!("import type {{ T as T{i} }} from \"missing\";"),
                        1 => "export type {};".to_string(),
                        2 => format!("export type {{ T{i} }} from \"missing\";"),
                        _ => format!("export type * from \"missing{i}\";"),
                    })
                    .collect::<String>()
            };
            erases_to(
                &format!(
                    "{}export function run(a,b) {{ return a; }}",
                    declarations(255)
                ),
                "export function run(a,b) { return a; }",
            );
            let error = compile(
                &format!(
                    "{}export function run(a,b) {{ return a; }}",
                    declarations(256)
                ),
                true,
            )
            .err()
            .unwrap();
            assert!(error.contains("at most 256 statements"), "{error}");
            let program =
                "function twice(x) { return x*2; } export function run(a,b) { return twice(a); }";
            erases_to(&format!("{}{program}", declarations(253)), program);
            assert!(
                compile(&format!("{}{program}", declarations(254)), true)
                    .err()
                    .unwrap()
                    .contains("at most 256 statements")
            );
            assert!(
                compile(
                    &format!(
                        "\"module hint\";{}export function run(a,b) {{ return a; }}",
                        declarations(255)
                    ),
                    true
                )
                .err()
                .unwrap()
                .contains("at most 256 statements")
            );
        }
        let prefix = "import type { T } from \"λ";
        let suffix = "\";export function run(a,b) { return a; }";
        let mut source = format!(
            "{prefix}{}{suffix}",
            "x".repeat(16_384 - prefix.len() - suffix.len())
        );
        assert_eq!(source.len(), 16_384);
        erases_to(&source, "export function run(a,b) { return a; }");
        source.push(' ');
        assert!(compile(&source, true).err().unwrap().contains("16 KiB"));
    }

    #[test]
    fn type_only_items_keep_original_parse_semantics_and_entry_checks() {
        for (source, diagnostic) in [
            (
                "import type { T } from \"missing\";",
                "exactly one exported function",
            ),
            (
                "export type * from \"missing\"; function run(a,b) { return a; }",
                "expected 'export function",
            ),
            (
                "export type { run }; function run(a,b) { return a; }",
                "expected 'export function",
            ),
            (
                "import type { T from \"missing\";export function run(a,b) { return a; }",
                "parse diagnostics",
            ),
            (
                "import type { T } from \"missing\"; export function run(a,b) { const x=a; const x=b; return x; }",
                "semantic diagnostics",
            ),
            (
                "import type { T } from \"missing\"; export function run(a,b) { return import(\"missing\"); }",
                "expression is outside",
            ),
        ] {
            let error = compile(source, true).err().expect(source);
            assert!(error.contains(diagnostic), "{source}: {error}");
        }
        assert!(
            compile(
                "import type { T } from \"missing\";export function run(a,b) { return a; }",
                false
            )
            .err()
            .unwrap()
            .contains("parse diagnostics")
        );
    }
    fn rejects_nested_module_item(item: &str) {
        for source in [
            format!("export function run(a,b) {{ {item} return a; }}"),
            format!("export function run(a,b) {{ {{ {item} }} return a; }}"),
            format!("export function run(a,b) {{ return a; {item} }}"),
            format!("export function run(a,b) {{ if(false) {{ {item} }} return a; }}"),
            format!(
                "function unused(x) {{ {item} return x; }} export function run(a,b) {{ return a; }}"
            ),
        ] {
            assert!(
                compile(&source, true).is_err(),
                "nested module item admitted: {source}"
            );
        }
    }

    #[test]
    fn nested_type_import_is_not_a_type_statement() {
        rejects_nested_module_item("import type { T } from \"missing\";");
    }

    #[test]
    fn nested_type_export_list_is_not_a_type_statement() {
        rejects_nested_module_item("export type {};");
    }

    #[test]
    fn nested_type_re_export_is_not_a_type_statement() {
        rejects_nested_module_item("export type { T } from \"missing\";");
    }

    #[test]
    fn nested_type_export_star_is_not_a_type_statement() {
        rejects_nested_module_item("export type * from \"missing\";");
    }

    #[test]
    fn original_type_statements_and_abrupt_diagnostics_are_unchanged() {
        erases_to(
            "type Module=number; interface ModuleShape {} export function run(a,b) { if(a) { type T=number; interface I {} return b; } return a; type Tail=number; interface End {} }",
            "export function run(a,b) { if(a) { return b; } return a; }",
        );
        for (source, diagnostic) in [
            (
                "export function run(a,b) { return a; type T=number; interface I {} a=b; }",
                "unsupported at bytes 67..71: statements after a guaranteed return are not admitted",
            ),
            (
                "export function run(a,b) { do { break; type T=number; interface I {} a=b; } while(0); return a; }",
                "unsupported at bytes 69..73: statements after a guaranteed abrupt completion are not admitted",
            ),
            (
                "export function run(a,b) { for(let i=0;i<1;i++) { continue; type T=number; interface I {} a=b; } return a; }",
                "unsupported at bytes 90..94: statements after a guaranteed abrupt completion are not admitted",
            ),
        ] {
            assert_eq!(compile(source, true).err().unwrap(), diagnostic);
        }
    }
}

#[cfg(test)]
mod generic_declaration_tests {
    use super::compile;
    fn erased(ts: &str, js: &str) {
        let actual = compile(ts, true).unwrap();
        let expected = compile(js, false).unwrap();
        assert_eq!(actual.wat, expected.wat);
        assert_eq!(actual.wasm, expected.wasm);
    }
    #[test]
    fn generic_entry_declaration_erases() {
        erased(
            "export function run<T>(a:T,b:T):number { return 0; }",
            "export function run(a,b) { return 0; }",
        );
    }
    #[test]
    fn generic_helper_declaration_erases() {
        erased(
            "function identity<T>(x:T):T { return x; } export function run(a,b) { return identity(a); }",
            "function identity(x) { return x; } export function run(a,b) { return identity(a); }",
        );
    }
    #[test]
    fn generic_metadata_variants_preserve_numeric_artifacts() {
        for parameters in [
            "<T>",
            "<T,U>",
            "<T extends number>",
            "<T=number>",
            "<const T>",
            "<T extends number=number,U=T>",
        ] {
            erased(
                &format!("export function run{parameters}(a:T,b:T):T {{ return a+b; }}"),
                "export function run(a,b) { return a+b; }",
            );
        }
        erased(
            "export function run<T=string>(a:T,b:T):T { return a-b; }",
            "export function run(a,b) { return a-b; }",
        );
    }
    #[test]
    fn generic_helpers_preserve_calls_and_snapshots() {
        erased(
            "export function run<T>(a:T,b:T):T { let x=a; return add(x,x=b)+x; } function add<T,U>(x:T,y:U):T { return x+y; }",
            "export function run(a,b) { let x=a; return add(x,x=b)+x; } function add(x,y) { return x+y; }",
        );
        erased(
            "function sum<T>(x:T):T { if(x<=0) return 0; return x+sum(x-1); } export function run<T>(a:T,b:T):T { return sum(3)+b; }",
            "function sum(x) { if(x<=0) return 0; return x+sum(x-1); } export function run(a,b) { return sum(3)+b; }",
        );
    }
    #[test]
    fn generic_type_names_create_no_runtime_values() {
        erased(
            "export function run<Math,T>(a:T,b:T):T { return Math.abs(a)+b; }",
            "export function run(a,b) { return Math.abs(a)+b; }",
        );
        for source in [
            "export function run<T>(a:T,b:T):T { return T; }",
            "export function run<T>(a:T,b:T):T { const T=T; return T; }",
            "export function run<T>(a:T,b:T):T { const x=a; x=b; return x; }",
            "function helper<T>(x:T):T { if(false) return Math.random(); return x; } export function run(a,b) { return a; }",
        ] {
            assert!(compile(source, true).is_err(), "{source}");
        }
    }
    #[test]
    fn generic_declarations_keep_adjacent_shape_guards() {
        for (source, expected) in [
            (
                "export async function run<T>(a:T,b:T):T { return a; }",
                "plain, synchronous function",
            ),
            (
                "export function* run<T>(a:T,b:T):T { return a; }",
                "plain, synchronous function",
            ),
            (
                "export declare function run<T>(a:T,b:T):T;",
                "plain, synchronous function",
            ),
            (
                "export function run<T>(this:T,a:T,b:T):T { return a; }",
                "plain, synchronous function",
            ),
            (
                "export function run<T>(a:T):T { return a; }",
                "exactly two ordinary",
            ),
            (
                "export function run<T>(a:T,b:T=1):T { return a; }",
                "default",
            ),
            (
                "function helper<T>(x:T):T { return x; } export function run(a,b) { return helper<number>(a); }",
                "generic calls",
            ),
            (
                "export function run<T>(a:T,b:T):T { return run<T>(a,b); }",
                "generic calls",
            ),
        ] {
            let error = compile(source, true).err().expect(source);
            assert!(error.contains(expected), "{source}: {error}");
        }
    }
    #[test]
    fn generic_metadata_keeps_statement_and_utf8_budgets() {
        let body = format!("{}return a;", "a=a;".repeat(255));
        erased(
            &format!("export function run<T>(a:T,b:T):T {{ {body} }}"),
            &format!("export function run(a,b) {{ {body} }}"),
        );
        assert!(
            compile(
                &format!("export function run<T>(a:T,b:T):T {{ a=a;{body} }}"),
                true
            )
            .err()
            .unwrap()
            .contains("256 statements")
        );
        let prefix = "export function run<T extends \"λ";
        let suffix = "\">(a:T,b:T):T { return a; }";
        let mut source = format!(
            "{prefix}{}{suffix}",
            "x".repeat(16_384 - prefix.len() - suffix.len())
        );
        erased(&source, "export function run(a,b) { return a; }");
        source.push(' ');
        assert!(compile(&source, true).err().unwrap().contains("16 KiB"));
    }
    #[test]
    fn generic_metadata_preserves_parse_and_semantic_precedence() {
        assert!(
            compile(
                "export function run<T extends >(a:T,b:T) { return a; }",
                true
            )
            .err()
            .unwrap()
            .contains("parse diagnostics")
        );
        assert!(
            compile(
                "export function run<T>(a:T,b:T) { const x=a;const x=b;return x; }",
                true
            )
            .err()
            .unwrap()
            .contains("semantic diagnostics")
        );
        assert!(
            compile("export function run<T>(a:T,b:T) { return a; }", false)
                .err()
                .unwrap()
                .contains("parse diagnostics")
        );
    }
}
