//! Public optional syntax remains a rejected ChainExpression. These separate
//! helper tests expose the parser's inner optional CallExpression before semantic
//! building, so they can compare the extracted preparation stages with the
//! compatibility leaf's original admission/check/capture order. They do not establish an
//! admitted optional-syntax route through the source scheduler.
use super::*;
use oxc_allocator::{Allocator, ReplaceWith};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

#[derive(Debug, PartialEq, Eq)]
struct Rejection {
    error: String,
    active_results: usize,
    active_values: usize,
    locals: usize,
    statements: usize,
    expansions: usize,
    source_charging: bool,
    slots: String,
    values: Vec<Type>,
    region: String,
    signatures: String,
}

fn reject_inner_optional_call(source: &str, active: usize, prepared: bool) -> Rejection {
    let allocator = Allocator::default();
    let mut parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    assert!(!parsed.fatal_error && parsed.diagnostics.is_empty());
    let Statement::ExpressionStatement(last) = parsed.program.body.last_mut().unwrap() else {
        panic!("expected optional call statement");
    };
    last.expression.replace_with(|expression| {
        let Expression::ChainExpression(chain) = expression else {
            panic!("expected original ChainExpression");
        };
        chain.unbox().expression.into()
    });
    assert!(matches!(&last.expression, Expression::CallExpression(call) if call.optional));
    let built = SemanticBuilder::new_compiler()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert!(built.diagnostics.is_empty());

    let mut lowering = Lowering {
        return_type: Type::Number,
        semantic: &built.semantic,
        stored: &stored_functions::Fields::default(),
        functions: &[],
        ui_import: None,
        general_bindings: &Default::default(),
        general_parameters: &Default::default(),
        managed_kinds: &Default::default(),
        number_calls: &Default::default(),
        general_results: &Default::default(),
        settled_results: &Default::default(),
        async_results: &Default::default(),
        callback_parameters: &Default::default(),
        promise_parameters: &Default::default(),
        bindings: Vec::new(),
        slots: Vec::new(),
        values: Vec::new(),
        next_region: 0,
        local_count: 0,
        statement_count: 0,
        in_loop_body: false,
        closures: Vec::new(),
        closure_expansions: 0,
        active_local_calls: 0,
        active_block_calls: 0,
        source_charging: true,
        inline_depth: 0,
        statement_depth: 0,
        allow_await: false,
        allow_async_closures: false,
        lifted: Vec::new(),
        lifted_base: 0,
        lifting_depth: 0,
        active_function_values: 0,
        active_function_results: 0,
        function_result_depth: 0,
        closure_signatures: &Default::default(),
    };
    let mut region = lowering.region(&parsed.program);
    lowering
        .control_statement_iterative(
            control_cursor::ControlNode::Sequence(
                &parsed.program.body[..parsed.program.body.len() - 1],
            ),
            &mut region,
            0,
        )
        .unwrap();
    let Statement::ExpressionStatement(last) = parsed.program.body.last().unwrap() else {
        unreachable!();
    };
    let expression = &last.expression;
    let signature = lowering.function_arity(expression, 0).unwrap_or(1);
    lowering.active_function_results = active;
    lowering.active_function_values = 1;
    let error = if prepared {
        // A source leaf now returns a deferred Check request. This separate
        // helper oracle retains the frozen preparation-state comparison; actual
        // queued checking and unwind are exercised by source continuation tests.
        let result = (|| {
            let Expression::CallExpression(call) = expression else {
                unreachable!("extracted inner optional call");
            };
            let closure = lowering
                .closure_target(invocation::callee(call), invocation::depth(call, 1))
                .expect("known optional helper callee");
            if let Some((candidate, mut factory)) = lowering.select_concise_factory(expression, 0) {
                if !factory.checked {
                    factory = lowering.check_closure_for_test(factory)?;
                    lowering.capture_cells(&factory, &mut region);
                }
                if lowering
                    .finish_concise_factory_preparation(candidate, factory)?
                    .is_some()
                {
                    return Ok(());
                }
            }
            lowering
                .prepare_known_local_function_result(call, closure, &mut region)
                .map(|_| ())
        })();
        result.err()
    } else {
        lowering
            .function_value_leaf(expression, &mut region, signature, 0)
            .err()
    }
    .expect("the extracted optional call must still reject");
    assert_eq!(lowering.statement_depth, 0);
    assert_eq!(lowering.return_type, Type::Number);
    assert!(!lowering.in_loop_body);
    Rejection {
        error,
        active_results: lowering.active_function_results,
        active_values: lowering.active_function_values,
        locals: lowering.local_count,
        statements: lowering.statement_count,
        expansions: lowering.closure_expansions,
        source_charging: lowering.source_charging,
        slots: format!("{:?}", lowering.slots),
        values: lowering.values.clone(),
        region: format!("{region:?}"),
        signatures: format!("{:?}", lowering.closure_signatures.borrow()),
    }
}

#[test]
fn inner_optional_call_preserves_compatibility_rejection_and_state() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            // Frozen on dafef1f before replacing its source-leaf fallback.
            for (name, source, diagnostic, at_limit) in [
                (
                    "checked",
                    "const make=x=>{return y=>x+y;};make?.(1);",
                    "unsupported at bytes 31..40: optional local calls are not admitted",
                    Some("unsupported at bytes 31..40: at most 8 active local function-result calls are admitted"),
                ),
                (
                    "captured",
                    "let captured=2;(x=>{return y=>captured+x+y;})?.(1);",
                    "unsupported at bytes 15..50: optional local calls are not admitted",
                    Some("unsupported at bytes 15..50: at most 8 active local function-result calls are admitted"),
                ),
                (
                    "invalid_unused",
                    "(x=>{const unused=()=>Math.noSuch(x);return y=>x+y;})?.(1);",
                    "unsupported at bytes 22..36: expression is outside the numeric experiment",
                    Some("unsupported at bytes 0..58: at most 8 active local function-result calls are admitted"),
                ),
                (
                    "invalid_parameters",
                    "((x=1)=>{return y=>x+y;})?.(1);",
                    "unsupported at bytes 2..5: local arrow parameters cannot have defaults, destructuring or modifiers",
                    Some("unsupported at bytes 0..30: at most 8 active local function-result calls are admitted"),
                ),
                (
                    "missing",
                    "(x=>{return y=>x+y;})?.();",
                    "unsupported at bytes 0..25: optional local calls are not admitted",
                    Some("unsupported at bytes 0..25: at most 8 active local function-result calls are admitted"),
                ),
                (
                    "metadata",
                    "(x=>y=>x+y)?.(1);",
                    "unsupported at bytes 0..16: optional local calls are not admitted",
                    None,
                ),
                (
                    "higher_metadata",
                    "(()=>()=>()=>1)?.();",
                    "unsupported at bytes 0..19: optional local calls are not admitted",
                    None,
                ),
            ] {
                for active in [0, 7, 8] {
                    let expected = reject_inner_optional_call(source, active, false);
                    let actual = reject_inner_optional_call(source, active, true);
                    let diagnostic = if active == 8 {
                        at_limit.unwrap_or(diagnostic)
                    } else {
                        diagnostic
                    };
                    assert_eq!(actual.error, diagnostic, "{name}, active={active}");
                    assert_eq!(actual, expected, "{name}, active={active}");
                    assert_eq!(actual.active_results, active);
                    assert_eq!(actual.active_values, 1);
                    assert_eq!(actual.expansions, 0);
                    assert!(actual.source_charging);
                    if name == "captured" {
                        assert_eq!(actual.region.contains("CaptureCells"), active < 8);
                    }
                }
            }
        })
        .expect("2 MiB optional-call helper thread")
        .join()
        .expect("optional-call checking must not overflow the native stack");
}

#[test]
fn optional_chain_sources_keep_their_original_rejection_priority() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut cases = vec![
                (
                    "const make=x=>y=>x+y;return make?.(a)(b);".to_owned(),
                    "unsupported at bytes 53..65: expression is outside the numeric experiment",
                ),
                (
                    "const make=x=>{return y=>x+y;};return make.call?.(null,a)(b);".to_owned(),
                    "unsupported at bytes 63..85: expression is outside the numeric experiment",
                ),
                (
                    "const bad=x=>{return Math.noSuch(x);};const make=x=>{return y=>x+y;};return make?.(a)(b);".to_owned(),
                    "unsupported at bytes 46..60: expression is outside the numeric experiment",
                ),
                (
                    "return ((x)=>{Math.noSuch(x);return y=>x+y;})?.(a)(b);".to_owned(),
                    "unsupported at bytes 32..78: expression is outside the numeric experiment",
                ),
                (
                    "false&&((x)=>{Math.noSuch(x);return y=>x+y;})?.(a);return b;".to_owned(),
                    "unsupported at bytes 32..75: expression is outside the numeric experiment",
                ),
            ];
            for (depth, diagnostic) in [
                (7, "unsupported at bytes 191..200: expression is outside the numeric experiment"),
                (8, "unsupported at bytes 191..220: at most 8 active local function-result calls are admitted"),
            ] {
                let mut argument = String::from("(make?.(a),a)");
                for _ in 0..depth {
                    argument = format!("(make.call(null,{argument}),a)");
                }
                let body = format!(
                    "const make=x=>{{return y=>x+y;}};return make.call(null,{argument})(b);"
                );
                if depth == 8 {
                    cases.push((
                        format!("const unused=()=>Math.noSuch(a);{body}"),
                        "unsupported at bytes 42..56: expression is outside the numeric experiment",
                    ));
                }
                cases.push((body, diagnostic));
            }
            for (depth, diagnostic) in [
                (15, "unsupported at bytes 180..189: expression is outside the numeric experiment"),
                (16, "unsupported at bytes 180..199: at most 16 active function-value cursors are admitted"),
            ] {
                let mut argument = String::from("(make?.(a),a)");
                for _ in 0..depth {
                    argument = format!("(meta({argument}),a)");
                }
                cases.push((
                    format!(
                        "const make=x=>{{return y=>x+y;}};const meta=x=>y=>x+y;return meta({argument})(b);"
                    ),
                    diagnostic,
                ));
            }
            for (body, diagnostic) in cases {
                let source = format!("export function run(a,b){{{body}}}");
                for typescript in [false, true] {
                    assert_eq!(
                        crate::compile(&source, typescript).err().as_deref(),
                        Some(diagnostic),
                        "{body}, typescript={typescript}"
                    );
                }
            }
        })
        .expect("2 MiB optional-chain source thread")
        .join()
        .expect("optional-chain rejection must not overflow the native stack");
}
