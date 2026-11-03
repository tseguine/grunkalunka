//! Factory checks retain distinct metadata, charged-result and returned-binding
//! lifetimes while entering disposable children on the shared source cursor.
use super::*;

fn on_small_stack(
    body: &str,
    asynchronous: bool,
    typescript: bool,
) -> (Result<(), String>, [usize; 3]) {
    let prefix = if asynchronous { "async " } else { "" };
    let source = format!("export {prefix}function run(a,b){{{body}}}");
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            SCHEDULED_METADATA_FACTORY_CHECKS.with(|count| count.set(0));
            SCHEDULED_LOCAL_FACTORY_CHECKS.with(|count| count.set(0));
            SCHEDULED_RETURNED_FACTORY_CHECKS.with(|count| count.set(0));
            let result = if asynchronous {
                crate::compile_async(&source, typescript).map(|_| ())
            } else {
                crate::compile(&source, typescript).map(|_| ())
            };
            (
                result,
                [
                    SCHEDULED_METADATA_FACTORY_CHECKS.with(std::cell::Cell::get),
                    SCHEDULED_LOCAL_FACTORY_CHECKS.with(std::cell::Cell::get),
                    SCHEDULED_RETURNED_FACTORY_CHECKS.with(std::cell::Cell::get),
                ],
            )
        })
        .expect("2 MiB factory-check thread")
        .join()
        .expect("factory checking must not overflow the native stack")
}

fn check_scheduled(bodies: &[&str], kind: usize, asynchronous: bool) {
    for &body in bodies {
        for typescript in [false, true] {
            let (result, counts) = on_small_stack(body, asynchronous, typescript);
            assert!(
                result.is_ok(),
                "{body}, typescript={typescript}: {result:?}"
            );
            assert!(
                counts[kind] > 0,
                "{body}, typescript={typescript}: factory check bypassed the source scheduler"
            );
        }
    }
}

#[test]
fn metadata_factory_preparation_checks_use_the_source_scheduler() {
    check_scheduled(
        &[
            "return(x=>y=>x+y)(a)(b);",
            "return(x=>y=>x+y).call(null,a)(b);",
            "const f=(x=>y=>x+y)(a);return f(b);",
            "return(x=>()=>()=>x)(a)()();",
            "const inner=()=>a;return(()=>inner)()();",
            "return(x=>()=>x.v===a?a:b)({v:a})();",
        ],
        0,
        false,
    );
}

#[test]
fn local_result_preparation_checks_use_the_source_scheduler() {
    check_scheduled(
        &[
            "return(x=>{return y=>x+y;})(a)(b);",
            "return(x=>{return y=>x+y;}).call(null,a)(b);",
            "return(x=>a?(y=>x+y):(y=>x-y))(a)(b);",
            "const f=(x=>a?(y=>x+y):(y=>x-y))(a);return f(b);",
        ],
        1,
        false,
    );
}

#[test]
fn returned_factory_metadata_checks_use_the_source_scheduler() {
    check_scheduled(
        &[
            "const make=x=>y=>x+y;return make(a)(b);",
            "const make=x=>y=>x+y;const f=make(a);return f(b);",
            "const make=x=>y=>x+y;const f=make(a),g=make(b);return f(b)+g(a);",
        ],
        2,
        false,
    );
    for typescript in [false, true] {
        let (result, counts) = on_small_stack(
            "const inner=()=>a;const make=()=>inner;const f=make();return f();",
            false,
            typescript,
        );
        assert!(result.is_ok(), "{result:?}");
        assert_eq!(
            counts[2], 0,
            "checked aliases must not repeat definition costs"
        );
    }
}

#[test]
fn async_factory_definition_checks_use_the_source_scheduler() {
    check_scheduled(
        &[
            "return await(x=>async y=>{await x;return x+y;})(a)(b);",
            "const f=(x=>async y=>{await x;return x+y;})(a);return await f(b);",
        ],
        0,
        true,
    );
}

#[test]
fn factory_checks_preserve_original_first_diagnostics() {
    for (body, expected) in [
        (
            "return(x=>y=>x+y)()(b);",
            "unsupported at bytes 31..47: expression is outside the numeric experiment",
        ),
        (
            "return(x=>{return y=>x+y;})()(b);",
            "unsupported at bytes 31..57: expression is outside the numeric experiment",
        ),
        (
            "return(x=>a?(y=>x+y):(y=>x-y))()(b);",
            "unsupported at bytes 31..60: expression is outside the numeric experiment",
        ),
        (
            "return(x=>y=>x+y)(...a)(b);",
            "unsupported at bytes 31..51: expression is outside the numeric experiment",
        ),
        (
            "return(x=>{return y=>x+y;})(...a)(b);",
            "unsupported at bytes 31..61: expression is outside the numeric experiment",
        ),
        (
            "return(x=>a?(y=>x+y):(y=>x-y))(...a)(b);",
            "unsupported at bytes 31..64: expression is outside the numeric experiment",
        ),
        (
            "return(x=>y=>Math.noSuch(x))(a)(b);",
            "unsupported at bytes 38..52: expression is outside the numeric experiment",
        ),
        (
            "return(x=>{const unused=()=>Math.noSuch(x);return y=>x+y;})(a)(b);",
            "unsupported at bytes 53..67: expression is outside the numeric experiment",
        ),
        (
            "return((x=a)=>y=>x+y)(a)(b);",
            "unsupported at bytes 33..36: local arrow parameters cannot have defaults, destructuring or modifiers",
        ),
        (
            "return((x=a)=>{return y=>x+y;})(a)(b);",
            "unsupported at bytes 33..36: local arrow parameters cannot have defaults, destructuring or modifiers",
        ),
        (
            "return((x=a)=>a?(y=>x+y):(y=>x-y))(a)(b);",
            "unsupported at bytes 33..36: local arrow parameters cannot have defaults, destructuring or modifiers",
        ),
    ] {
        for typescript in [false, true] {
            let (result, _) = on_small_stack(body, false, typescript);
            assert_eq!(
                result.unwrap_err(),
                expected,
                "{body}, typescript={typescript}"
            );
        }
    }
}

#[test]
fn factory_definition_checks_keep_their_distinct_result_admission() {
    for (metadata, depth, expected) in [
        (
            false,
            0,
            "unsupported at bytes 86..100: expression is outside the numeric experiment",
        ),
        (
            false,
            7,
            "unsupported at bytes 128..142: expression is outside the numeric experiment",
        ),
        (
            false,
            8,
            "unsupported at bytes 112..168: at most 8 active local function-result calls are admitted",
        ),
        (
            true,
            0,
            "unsupported at bytes 71..85: expression is outside the numeric experiment",
        ),
        (
            true,
            7,
            "unsupported at bytes 113..127: expression is outside the numeric experiment",
        ),
        (
            true,
            8,
            "unsupported at bytes 119..133: expression is outside the numeric experiment",
        ),
    ] {
        let mut argument = if metadata {
            "((x=>y=>Math.noSuch(x))(a),a)".to_owned()
        } else {
            "((x=>{const unused=()=>Math.noSuch(x);return y=>x+y;})(a),a)".to_owned()
        };
        for _ in 0..depth {
            argument = format!("(wrap({argument}),a)");
        }
        let body = format!("const wrap=x=>{{return y=>x+y;}};return {argument};");
        for typescript in [false, true] {
            let (result, counts) = on_small_stack(&body, false, typescript);
            assert_eq!(
                result.unwrap_err(),
                expected,
                "metadata={metadata}, {depth}, typescript={typescript}"
            );
            if metadata {
                assert!(
                    counts[0] > 0,
                    "metadata checking bypassed the source cursor"
                );
            } else if depth < 8 {
                assert!(counts[1] > 0, "local checking bypassed the source cursor");
            } else {
                assert_eq!(counts[1], 0, "result8 must reject before local checking");
            }
        }
    }
}

#[test]
fn returned_metadata_check_keeps_the_callers_result_context() {
    for depth in [0, 7, 8] {
        let mut argument = "(make(a),a)".to_owned();
        for _ in 0..depth {
            argument = format!("(wrap({argument}),a)");
        }
        let body = format!(
            "const wrap=x=>{{return y=>x+y;}};const make=x=>()=>(wrap(x),x);return {argument};"
        );
        for typescript in [false, true] {
            let (result, counts) = on_small_stack(&body, false, typescript);
            if depth < 8 {
                assert!(
                    result.is_ok(),
                    "{depth}, typescript={typescript}: {result:?}"
                );
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    "unsupported at bytes 75..82: at most 8 active local function-result calls are admitted"
                );
            }
            assert!(
                counts[2] > 0,
                "returned metadata bypassed checking children"
            );
        }
    }
}

#[test]
fn factory_check_errors_restore_parent_guards_bindings_and_source_modes() {
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_semantic::SemanticBuilder;
    use oxc_span::SourceType;

    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for (source, active_results, returned) in [
                (
                    "let a=1;(x=>{const bad=()=>Math.noSuch(x);return y=>x+y;})(a);",
                    7,
                    false,
                ),
                ("let a=1;(x=>y=>Math.noSuch(x))(a);", 8, false),
                (
                    "let a=1;const wrap=x=>{return y=>x+y;};const make=x=>()=>(wrap(x),x);make(a);",
                    8,
                    true,
                ),
            ] {
                let allocator = Allocator::default();
                let parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
                assert!(!parsed.fatal_error && parsed.diagnostics.is_empty());
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
                let mut output = lowering.region(&parsed.program);
                let split = parsed.program.body.len() - 1;
                lowering
                    .control_statement_iterative(
                        ControlNode::Sequence(&parsed.program.body[..split]),
                        &mut output,
                        0,
                    )
                    .unwrap();
                let bindings = lowering
                    .bindings
                    .iter()
                    .map(|binding| (binding.symbol, binding.slot, binding.mutable))
                    .collect::<Vec<_>>();
                let closures = lowering.closures.len();
                let locals = lowering.local_count;
                let statements = lowering.statement_count;
                let expansions = lowering.closure_expansions;
                let slots = lowering.slots.len();
                lowering.active_function_values = 2;
                lowering.active_function_results = active_results;
                lowering.inline_depth = 3;
                lowering.statement_depth = 5;
                lowering.function_result_depth = 7;
                SCHEDULED_METADATA_FACTORY_CHECKS.with(|count| count.set(0));
                SCHEDULED_LOCAL_FACTORY_CHECKS.with(|count| count.set(0));
                SCHEDULED_RETURNED_FACTORY_CHECKS.with(|count| count.set(0));
                let error = lowering
                    .control_statement_iterative(
                        ControlNode::Sequence(&parsed.program.body[split..]),
                        &mut output,
                        5,
                    )
                    .unwrap_err();
                let diagnostic = if returned {
                    "at most 8 active local function-result calls are admitted"
                } else {
                    "expression is outside the numeric experiment"
                };
                assert!(error.ends_with(diagnostic), "{source}: {error}");
                assert_eq!(lowering.active_function_results, active_results);
                assert_eq!(lowering.active_function_values, 2);
                assert_eq!(lowering.active_block_calls, 0);
                assert_eq!(lowering.active_local_calls, 0);
                assert_eq!(lowering.lifting_depth, 0);
                assert!(lowering.source_charging);
                assert_eq!(lowering.inline_depth, 3);
                assert_eq!(lowering.statement_depth, 5);
                assert_eq!(lowering.function_result_depth, 7);
                assert_eq!(lowering.return_type, Type::Number);
                assert_eq!(lowering.closures.len(), closures);
                assert_eq!(
                    lowering.local_count, locals,
                    "failed child costs stay disposable"
                );
                assert_eq!(lowering.statement_count, statements + 1);
                assert_eq!(
                    lowering.closure_expansions,
                    expansions + usize::from(returned)
                );
                assert_eq!(lowering.slots.len(), slots + usize::from(returned));
                assert_eq!(
                    lowering
                        .bindings
                        .iter()
                        .map(|binding| (binding.symbol, binding.slot, binding.mutable))
                        .collect::<Vec<_>>(),
                    bindings,
                    "factory parameters must be restored before error propagation"
                );
                let checks = if returned {
                    SCHEDULED_RETURNED_FACTORY_CHECKS.with(std::cell::Cell::get)
                } else if active_results == 8 {
                    SCHEDULED_METADATA_FACTORY_CHECKS.with(std::cell::Cell::get)
                } else {
                    SCHEDULED_LOCAL_FACTORY_CHECKS.with(std::cell::Cell::get)
                };
                assert!(
                    checks > 0,
                    "error must unwind an actual owned checking frame"
                );
            }
        })
        .expect("2 MiB factory error-state thread")
        .join()
        .expect("factory checking cleanup must not exhaust the native stack");
}
