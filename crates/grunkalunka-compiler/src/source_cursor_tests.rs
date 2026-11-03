//! Test-only observations of the native source-control cursor call stack.
//!
//! Deep mixed syntax must remain on one heap continuation scheduler. This
//! probes that invariant independently of source-depth admission and the
//! expression evaluator's separately tracked reentry.
use std::cell::Cell;

thread_local! {
    static ACTIVE: Cell<usize> = const { Cell::new(0) };
    static MAXIMUM: Cell<usize> = const { Cell::new(0) };
    static ENTRIES: Cell<usize> = const { Cell::new(0) };
    static PENDING: Cell<usize> = const { Cell::new(0) };
    static EVALUATOR_CROSSINGS: Cell<usize> = const { Cell::new(0) };
}

pub(crate) struct Entry;

pub(crate) fn entry() -> Entry {
    if crate::evaluator_tests::active_depth() != 0 {
        EVALUATOR_CROSSINGS.with(|crossings| crossings.set(crossings.get() + 1));
    }
    ACTIVE.with(|active| {
        active.set(active.get() + 1);
        MAXIMUM.with(|maximum| maximum.set(maximum.get().max(active.get())));
    });
    ENTRIES.with(|entries| entries.set(entries.get() + 1));
    Entry
}

impl Drop for Entry {
    fn drop(&mut self) {
        ACTIVE.with(|active| active.set(active.get().checked_sub(1).expect("balanced cursor")));
    }
}

pub(crate) fn pending(count: usize) {
    PENDING.with(|pending| pending.set(pending.get().max(count)));
}

fn evaluator_crossings() -> usize {
    EVALUATOR_CROSSINGS.with(Cell::get)
}

fn observe<T>(run: impl FnOnce() -> T) -> (T, usize, usize, usize) {
    ACTIVE.with(|active| assert_eq!(active.get(), 0));
    MAXIMUM.with(|maximum| maximum.set(0));
    ENTRIES.with(|entries| entries.set(0));
    PENDING.with(|pending| pending.set(0));
    EVALUATOR_CROSSINGS.with(|crossings| crossings.set(0));
    let result = run();
    ACTIVE.with(|active| assert_eq!(active.get(), 0, "cursor must restore on all exits"));
    (
        result,
        MAXIMUM.with(Cell::get),
        ENTRIES.with(Cell::get),
        PENDING.with(Cell::get),
    )
}

#[test]
fn declaration_definition_checks_stay_on_one_source_cursor() {
    for (body, admitted) in [
        (
            "const f=()=>{const g=()=>{return a;};return g();};return f();",
            true,
        ),
        ("const make=x=>y=>x+y;return a;", true),
        (
            "const f=()=>{const bad=()=>Math.noSuch(a);return b;};return a;",
            false,
        ),
        (
            "const f=()=>{const g=()=>{const bad=()=>Math.noSuch(a);return b;};return g();};return a;",
            false,
        ),
    ] {
        for typescript in [false, true] {
            let source = format!("export function run(a,b){{{body}}}");
            let (result, maximum, entries, _) = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || observe(|| crate::compile(&source, typescript).map(|_| ())))
                .expect("2 MiB declaration-check thread")
                .join()
                .expect("definition checks must not exhaust the native stack");
            assert_eq!(result.is_ok(), admitted, "{body}: {result:?}");
            assert_eq!(
                maximum, 1,
                "declaration checking reentered the source cursor: {body}"
            );
            assert!(entries > 0);
        }
    }
}

#[test]
fn nested_real_lifted_bodies_use_one_source_cursor() {
    for (body, asynchronous) in [
        ("let f=x=>{let g=y=>x+y;return g(a);};return f(b);", false),
        ("let f=x=>y=>x+y;return f(a)(b);", false),
        ("return((fn,x)=>fn(x))(y=>{return y+a;},b);", false),
        (
            "let f=async x=>{await b;return x+a;};return await f(a);",
            true,
        ),
    ] {
        for typescript in [false, true] {
            let prefix = if asynchronous { "async " } else { "" };
            let source = format!("export {prefix}function run(a,b){{{body}}}");
            std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || {
                    let (result, maximum, entries, _) = observe(|| {
                        if asynchronous {
                            crate::compile_async(&source, typescript).map(|_| ())
                        } else {
                            crate::compile(&source, typescript).map(|_| ())
                        }
                    });
                    assert!(result.is_ok(), "{body}: {result:?}");
                    assert_eq!(maximum, 1, "real lifting must not nest the source cursor");
                    assert_eq!(
                        entries, 1,
                        "real lifting must not reenter the source cursor"
                    );
                })
                .expect("2 MiB lifted source-cursor thread")
                .join()
                .expect("real lifted bodies share one source cursor");
        }
    }
}

#[test]
fn unchecked_local_callees_use_one_source_cursor() {
    for (body, asynchronous) in [
        (
            "return(x=>{return(y=>{if(y)return y+x;return x;})(b);})(a);",
            false,
        ),
        ("return await(async x=>{await b;return x+a;})(b);", true),
    ] {
        for typescript in [false, true] {
            let prefix = if asynchronous { "async " } else { "" };
            let source = format!("export {prefix}function run(a,b){{{body}}}");
            std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || {
                    let (result, maximum, entries, _) = observe(|| {
                        if asynchronous {
                            crate::compile_async(&source, typescript).map(|_| ())
                        } else {
                            crate::compile(&source, typescript).map(|_| ())
                        }
                    });
                    assert!(result.is_ok(), "{body}: {result:?}");
                    assert_eq!(maximum, 1, "checking must not nest the source dispatcher");
                    assert_eq!(
                        entries, 1,
                        "checking must not reenter the source dispatcher"
                    );
                })
                .expect("2 MiB source checking thread")
                .join()
                .expect("checking and body lowering share one source cursor");
        }
    }
}

#[test]
fn unchecked_local_error_unwinds_one_source_cursor() {
    for typescript in [false, true] {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                let (result, maximum, entries, _) = observe(|| {
                    crate::compile(
                        "export function run(a,b){return(x=>Math.noSuch(x))(Math.noOther(a));}",
                        typescript,
                    )
                    .map(|_| ())
                });
                assert_eq!(
                    result.unwrap_err(),
                    "unsupported at bytes 35..49: expression is outside the numeric experiment"
                );
                assert_eq!(maximum, 1);
                assert_eq!(entries, 1);
            })
            .expect("2 MiB failed checking thread")
            .join()
            .expect("checking errors must restore the source cursor");
    }
}

#[test]
fn discarded_control_body_preserves_aggregate_expression_depth_limit() {
    for control in ["if(a)", "while(a)", "for(;a;)"] {
        for (branches, admitted) in [(126, true), (127, false)] {
            for typescript in [false, true] {
                let expression = format!("{}a", "a?0:".repeat(branches));
                let source = format!(
                    "export function run(a,b){{const f=()=>{{{control} {expression};return a;}};return f();}}"
                );
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile(&source, typescript))
                    .expect("2 MiB discarded-expression boundary thread")
                    .join()
                    .expect("discarded expression must not exhaust native stack");
                assert_eq!(
                    result.is_ok(),
                    admitted,
                    "{control}, {branches} branches, typescript={typescript}: {:?}",
                    result.err()
                );
                if let Err(error) = result {
                    assert!(
                        error.contains("block-arrow expansion exceeds 128 nested regions"),
                        "{control}: {error}"
                    );
                }
            }
        }
    }
}

#[test]
fn mixed_source_control_lowers_with_a_single_native_cursor_entry() {
    // All source-control families should share the *same* scheduling loop;
    // a second native entry would reveal an uncaught control/control SCC.
    for (body, asynchronous) in [
        (
            "if(a){while(b){break;}}else{for(let i=0;i<2;i++){a+=i;}}return a;",
            false,
        ),
        (
            "let n=a;do{if(n){n+=b;}else{n+=1;}}while(false);return n;",
            false,
        ),
        (
            "try{try{return a;}catch(e){return b;}}finally{await b;}",
            true,
        ),
    ] {
        let source = format!(
            "export {}function run(a,b){{{body}}}",
            if asynchronous { "async " } else { "" }
        );
        for ts in [false, true] {
            let (result, maximum, entries, _) = observe(|| {
                if asynchronous {
                    crate::compile_async(&source, ts)
                } else {
                    crate::compile(&source, ts)
                }
            });
            assert!(result.is_ok(), "{source}: {:?}", result.err());
            assert_eq!(maximum, 1, "{source}: source-control reentered");
            assert!(entries > 0, "{source}: no source cursor entry");
        }
    }
}

#[test]
fn rejected_mixed_source_nesting_exits_without_reentering_native_cursor() {
    let mut body = String::from("a+=b;");
    for index in 0..66 {
        body = if index % 2 == 0 {
            format!("if(a){{{body}}}else{{b+=a;}}")
        } else {
            format!("while(a){{{body}}}")
        };
    }
    let source = format!("export function run(a,b){{{body}return a;}}");
    let (result, maximum, entries, _) = observe(|| crate::compile(&source, false));
    let error = match result {
        Ok(_) => panic!("over-depth mixed source must reject"),
        Err(error) => error,
    };
    assert!(
        error.contains("64") || error.contains("128"),
        "mixed nesting: {error}"
    );
    assert_eq!(maximum, 1, "rejected mixed source reentered the cursor");
    assert!(entries > 0);
}

#[test]
fn suspended_if_predicate_preserves_expression_diagnostics_on_two_mib_stack() {
    // The condition now crosses the source -> expression request/result seam
    // instead of being executed inside the If entry transition. Preserve
    // ordering of comma/conditional expressions and failure before either arm.
    for (body, admitted) in [
        ("if((a=1,!b,a<b)) return a; return b;", true),
        ("if(a?(b?true:false):!a) return a; return b;", true),
        ("if(a ** b) return a; return b;", false),
    ] {
        for typescript in [false, true] {
            let source = format!("export function run(a,b) {{ {body} }}");
            let (result, maximum, entries, _) = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || observe(|| crate::compile(&source, typescript)))
                .expect("2 MiB source expression request thread")
                .join()
                .expect("condition request must not exhaust native stack");
            assert_eq!(result.is_ok(), admitted, "{body}: {:?}", result.err());
            assert_eq!(maximum, 1, "source cursor reentered for {body}");
            assert!(entries > 0, "no source cursor entered for {body}");
        }
    }
}

#[test]
fn suspended_while_predicate_preserves_region_order_on_two_mib_stack() {
    // The while condition must yield its result before the loop body is
    // allocated; rejected predicates must not continue to body lowering.
    for (body, admitted) in [
        ("let n=a;while(n<b){n+=1;}return n;", true),
        ("let n=a;while((n+=b)<10){n-=1;}return n;", true),
        ("while(a ** b) a+=b;return a;", false),
    ] {
        for typescript in [false, true] {
            let source = format!("export function run(a,b) {{ {body} }}");
            let (result, maximum, entries, _) = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || observe(|| crate::compile(&source, typescript)))
                .expect("2 MiB loop predicate request thread")
                .join()
                .expect("loop condition must not exhaust native stack");
            assert_eq!(result.is_ok(), admitted, "{body}: {:?}", result.err());
            assert_eq!(maximum, 1, "source cursor reentered for {body}");
            assert!(entries > 0, "no source cursor entered for {body}");
        }
    }
}

#[test]
fn suspended_do_while_test_preserves_body_before_predicate_order_on_two_mib_stack() {
    // Unlike while, a do-while test is lowered only after the body. This
    // ordering must hold even if the body has an abrupt return completion.
    for (body, admitted) in [
        ("let n=a;do{n++;}while(n<b);return n;", true),
        ("do{return a;}while(false);", true),
        ("do{a+=b;}while(a ** b);return a;", false),
    ] {
        for typescript in [false, true] {
            let source = format!("export function run(a,b) {{ {body} }}");
            let (result, maximum, entries, _) = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || observe(|| crate::compile(&source, typescript)))
                .expect("2 MiB do-while expression request thread")
                .join()
                .expect("do-while predicate must not overflow native stack");
            assert_eq!(result.is_ok(), admitted, "{body}: {:?}", result.err());
            assert_eq!(maximum, 1, "source cursor reentered for {body}");
            assert!(entries > 0, "no source cursor entered for {body}");
        }
    }
}

#[test]
fn suspended_for_test_preserves_header_and_omitted_test_on_two_mib_stack() {
    for (body, admitted) in [
        ("let n=0;for(let i=0;i<3;i++)n+=i;return n;", true),
        ("let n=0;for(let i=0;i<3;){n+=a;i++;}return n;", true),
        ("let n=0;for(let i=0;;i++){n+=a;break;}return n;", true),
        ("let n=0;for(let i=0;i ** b;i++)n+=a;return n;", false),
    ] {
        for typescript in [false, true] {
            let source = format!("export function run(a,b) {{ {body} }}");
            let (result, maximum, entries, _) = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || observe(|| crate::compile(&source, typescript)))
                .expect("2 MiB for-loop test request thread")
                .join()
                .expect("for-loop condition must not overflow native stack");
            assert_eq!(result.is_ok(), admitted, "{body}: {:?}", result.err());
            assert_eq!(maximum, 1, "source cursor reentered for {body}");
            assert!(entries > 0, "no source cursor entered for {body}");
        }
    }
}

#[test]
fn discarded_expression_request_preserves_side_effect_order_and_diagnostics() {
    for (body, admitted) in [
        ("a;!a;(a=1,!b);return a;", true),
        ("if(a){a=1;a+=b;}return a;", true),
        ("a ** b;return a;", false),
    ] {
        for typescript in [false, true] {
            let source = format!("export function run(a,b) {{ {body} }}");
            let (result, maximum, entries, _) = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || observe(|| crate::compile(&source, typescript)))
                .expect("2 MiB discard-expression request thread")
                .join()
                .expect("discard statement must not overflow native stack");
            assert_eq!(result.is_ok(), admitted, "{body}: {:?}", result.err());
            assert_eq!(maximum, 1, "source cursor reentered for {body}");
            assert!(entries > 0, "no source cursor entered for {body}");
        }
    }
}

#[test]
fn suspended_for_update_preserves_step_order_and_lexical_cells_on_two_mib_stack() {
    // The update executes after body lowering but before captured header
    // renewal is determined, including when continue may reach the step.
    for (body, admitted) in [
        ("let n=0;for(let i=0;i<3;i++){n=n*10+i;}return n;", true),
        (
            "let n=a;for(let i=0;i<3;(i++,n+=b)){if(i===1)continue;n+=i;}return n;",
            true,
        ),
        ("for(let i=0;i<2;){i++;}return a;", true),
        ("for(let i=0;i<2;i ** b){a+=i;}return a;", false),
    ] {
        for typescript in [false, true] {
            let source = format!("export function run(a,b) {{ {body} }}");
            let (result, maximum, entries, _) = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || observe(|| crate::compile(&source, typescript)))
                .expect("2 MiB for-update continuation thread")
                .join()
                .expect("for-update source lowering must not overflow native stack");
            assert_eq!(result.is_ok(), admitted, "{body}: {:?}", result.err());
            assert_eq!(maximum, 1, "source cursor reentered for {body}");
            assert!(entries > 0, "no source cursor entered for {body}");
        }
    }
}

#[test]
fn suspended_for_initializer_keeps_header_effects_and_source_diagnostics() {
    for (body, admitted) in [
        ("let n=0;for(a=0;a<3;a++){n+=a;}return n;", true),
        ("for((a=0,b=1);a<2;a++){b+=a;}return b;", true),
        ("let n=0;for(let i=0;i<3;i++){n+=i;}return n;", true),
        ("for(;a;){break;}return b;", true),
        ("for(a ** b;a<3;a++){b+=a;}return b;", false),
    ] {
        for typescript in [false, true] {
            let source = format!("export function run(a,b) {{ {body} }}");
            let (result, maximum, entries, _) = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || observe(|| crate::compile(&source, typescript)))
                .expect("2 MiB for initializer continuation thread")
                .join()
                .expect("for initializer must not overflow native stack");
            assert_eq!(result.is_ok(), admitted, "{body}: {:?}", result.err());
            assert_eq!(maximum, 1, "source cursor reentered for {body}");
            assert!(entries > 0, "no source cursor entered for {body}");
        }
    }
}

#[test]
fn nested_block_function_results_characterize_cross_phase_native_reentry() {
    // This reproduces a maintained runtime-closures admission boundary, but
    // observes the source cursor while its caller evaluator is still active.
    // Successful future L4 defunctionalization should drive the crossing count
    // down rather than tightening source admission or increasing thread stacks.
    for (depth, admitted) in [(0, true), (2, true), (4, true), (6, true), (7, false)] {
        let mut body = String::from("const get0=()=>{return ()=>a;};");
        for index in 1..=depth {
            body.push_str(&format!(
                "const get{index}=()=>{{return get{}();}};",
                index - 1
            ));
        }
        let source = format!("export function run(a,b){{{body}const f=get{depth}();return f();}}");
        for typescript in [false, true] {
            let source = source.clone();
            let (result, maximum, entries, crossings) = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || {
                    let (result, maximum, entries, _) =
                        observe(|| crate::compile(&source, typescript));
                    (result, maximum, entries, evaluator_crossings())
                })
                .expect("2 MiB block factory measurement thread")
                .join()
                .expect("block function result must not exhaust native stack");
            assert_eq!(
                result.is_ok(),
                admitted,
                "{depth} block factories: {:?}",
                result.as_ref().err()
            );
            assert!(
                maximum <= 10,
                "{depth} block factories reentered too deeply: {maximum}"
            );
            assert!(entries > 0);
            assert!(crossings <= entries);
            if let Err(error) = result {
                assert!(
                    error.starts_with("unsupported at bytes "),
                    "{depth} block factories: {error}"
                );
            }
            println!(
                "CROSS_PHASE|block_factories={depth}|max_source={maximum}|source_entries={entries}|eval_to_source={crossings}"
            );
        }
    }
}

#[test]
fn numeric_return_continuation_preserves_abrupt_paths_and_first_errors() {
    for (body, admitted) in [
        ("return (a+b)*(a-b);", true),
        ("if(a){return a+1;}return b+2;", true),
        ("while(a){return a+b;}return b;", true),
        ("return a ** b;", false),
        ("return;", false),
    ] {
        for typescript in [false, true] {
            let source = format!("export function run(a,b) {{ {body} }}");
            let (result, maximum, entries, _) = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || observe(|| crate::compile(&source, typescript)))
                .expect("2 MiB numeric return continuation thread")
                .join()
                .expect("numeric return must not overflow native stack");
            assert_eq!(result.is_ok(), admitted, "{body}: {:?}", result.err());
            assert_eq!(maximum, 1, "source cursor reentered for {body}");
            assert!(entries > 0, "no source cursor entered for {body}");
        }
    }
}

#[test]
fn async_throw_reason_continuation_preserves_catch_and_source_diagnostics() {
    for (body, admitted) in [
        ("try{throw a;}catch(e){return b;}", true),
        ("try{throw (a+b);}catch(e){return b;}", true),
        ("try{throw a ** b;}catch(e){return b;}", false),
    ] {
        for typescript in [false, true] {
            let source = format!("export async function run(a,b) {{ {body} }}");
            let (result, maximum, entries, _) = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || observe(|| crate::compile_async(&source, typescript)))
                .expect("2 MiB throw-reason continuation thread")
                .join()
                .expect("async throw lowering must not overflow native stack");
            assert_eq!(result.is_ok(), admitted, "{body}: {:?}", result.err());
            assert_eq!(maximum, 1, "source cursor reentered for {body}");
            assert!(entries > 0, "no source cursor entered for {body}");
        }
    }

    // Synchronous source throw is still outside the numeric profile, and
    // rejects before its argument can be evaluated or emitted.
    let source = "export function run(a,b) { throw a; }";
    let (result, maximum, entries, _) = observe(|| crate::compile(source, false));
    let error = result
        .err()
        .expect("synchronous throw must remain rejected");
    assert!(
        error.contains("statement is outside the numeric experiment"),
        "unexpected first diagnostic: {error}"
    );
    assert_eq!(maximum, 1);
    assert!(entries > 0);
}

#[test]
fn direct_primitive_returns_use_typed_source_outcomes_without_changing_admission() {
    for (body, admitted) in [
        ("return null;", true),
        ("return a?false:null;", true),
        ("if(a)return b;return undefined;", true),
        ("return a ** b;", false),
    ] {
        for typescript in [false, true] {
            let source = format!("export function run(a,b) {{ {body} }}");
            let (result, maximum, entries, _) = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || observe(|| crate::compile_value(&source, typescript)))
                .expect("2 MiB typed-return continuation thread")
                .join()
                .expect("primitive return must not overflow native stack");
            assert_eq!(result.is_ok(), admitted, "{body}: {:?}", result.err());
            assert_eq!(maximum, 1, "source cursor reentered for {body}");
            assert!(entries > 0, "no source cursor entered for {body}");
        }
    }
}
