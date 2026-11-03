//! General concise function results evaluate a source body while retaining
//! their result admission and factory bindings. These are distinct from the
//! no-result-charge closure-metadata path in concise_factory_tests.rs.
use super::BRIDGED_FUNCTION_RESULT_VALUES;

fn on_small_stack(body: &str, asynchronous: bool, typescript: bool) -> (Result<(), String>, usize) {
    let prefix = if asynchronous { "async " } else { "" };
    let source = format!("export {prefix}function run(a,b){{{body}}}");
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            BRIDGED_FUNCTION_RESULT_VALUES.with(|count| count.set(0));
            let result = if asynchronous {
                crate::compile_async(&source, typescript).map(|_| ())
            } else {
                crate::compile(&source, typescript).map(|_| ())
            };
            let bridged = BRIDGED_FUNCTION_RESULT_VALUES.with(std::cell::Cell::get);
            (result, bridged)
        })
        .expect("2 MiB concise function-result thread")
        .join()
        .expect("concise function results must not overflow the native stack")
}

#[test]
fn concise_function_result_bodies_use_the_source_scheduler() {
    for body in [
        "const make=x=>a?(y=>x+y):(y=>x-y);return make(a)(b);",
        "let n=0;const make=x=>(n=n*10+2,y=>x+y+n);return make((n=n*10+1))(n=n*10+3)+n;",
        "const id=fn=>fn;return id(y=>y+a)(b);",
        "const id=fn=>fn;return id.call(null,y=>y+a)(b);",
        "const seed=x=>{return y=>x+y;};const make=x=>seed(x);return make(a)(b);",
        "const id=fn=>fn;const apply=(fn,x)=>fn(x);return apply(id(y=>y+a),b);",
        "let n=a;const make=x=>(n+=x,y=>x+y);make(b);return n;",
        "let n=a;const make=x=>(n+=x,y=>x+y);if(make(b))n+=1;return n;",
    ] {
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(body, false, typescript);
            assert!(
                result.is_ok(),
                "{body}, typescript={typescript}: {result:?}"
            );
            assert!(
                bridged > 0,
                "{body}, typescript={typescript}: concise result bypassed the source scheduler"
            );
        }
    }
}

#[test]
fn concise_function_result_async_callees_use_the_source_scheduler() {
    let body = "const make=x=>x?(async y=>x+y):(async y=>x-y);return await make(a)(b);";
    for typescript in [false, true] {
        let (result, bridged) = on_small_stack(body, true, typescript);
        assert!(result.is_ok(), "typescript={typescript}: {result:?}");
        assert!(
            bridged > 0,
            "async concise result bypassed the source scheduler"
        );
    }
}

#[test]
fn concise_function_results_preserve_first_diagnostics() {
    for (body, expected) in [
        (
            "const make=x=>(b,y=>x+y);return make()(b);",
            "unsupported at bytes 57..66: expression is outside the numeric experiment",
        ),
        (
            "const make=x=>(b,y=>x+y);return make(...a)(b);",
            "unsupported at bytes 57..70: expression is outside the numeric experiment",
        ),
        (
            "const id=fn=>fn;return id(a)(b);",
            "unsupported at bytes 48..56: expression is outside the numeric experiment",
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
fn concise_function_results_keep_the_eight_active_result_budget() {
    // Unlike metadata factories, general concise results own the shared
    // eight-result admission throughout caller arguments and body evaluation.
    for depth in [6, 7, 8, 9] {
        let mut argument = String::from("a");
        for _ in 0..depth {
            argument = format!("(make({argument}),a)");
        }
        let body = format!("const make=x=>(b,y=>x+y);return make({argument})(b);");
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, false, typescript);
            if depth < 8 {
                assert!(
                    result.is_ok(),
                    "{depth}, typescript={typescript}: {result:?}"
                );
                assert!(
                    bridged > depth,
                    "nested concise results bypassed the scheduler"
                );
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    format!(
                        "unsupported at bytes 105..{}: at most 8 active local function-result calls are admitted",
                        112 + 10 * (depth - 8)
                    ),
                    "{depth}, typescript={typescript}"
                );
            }
        }
    }
}

#[test]
fn concise_function_result_bodies_keep_their_own_function_value_admission() {
    // The surrounding metadata calls use function-value admissions but no
    // result admissions. The one general concise body must still acquire its
    // own sixteenth cursor before evaluating its returned expression.
    for depth in [13, 14, 15] {
        let mut argument = String::from("(make(a),a)");
        for _ in 0..depth {
            argument = format!("(meta({argument}),a)");
        }
        let body =
            format!("const make=x=>(b,y=>x+y);const meta=x=>y=>x+y;return meta({argument})(b);");
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, false, typescript);
            if depth == 13 {
                assert!(result.is_ok(), "typescript={typescript}: {result:?}");
                assert!(bridged > 0, "charged concise body bypassed the scheduler");
            } else {
                let span = if depth == 14 { "39..49" } else { "174..181" };
                assert_eq!(
                    result.unwrap_err(),
                    format!(
                        "unsupported at bytes {span}: at most 16 active function-value cursors are admitted"
                    ),
                    "{depth}, typescript={typescript}"
                );
            }
        }
    }
}

#[test]
fn concise_function_results_restore_caller_statement_depth_before_outer_arguments() {
    // The outer argument's block body combines the caller's 64 statement
    // levels with its own generated expression regions. Resetting statement
    // depth when the concise factory returns would incorrectly admit 64.
    for branches in [63, 64] {
        let body = format!(
            "const make=x=>(b,y=>x+y);const step=()=>{{return {}a;}};{}return make(a)(step());{}",
            "a?0:".repeat(branches),
            "{".repeat(64),
            "}".repeat(64),
        );
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, false, typescript);
            assert!(bridged > 0, "concise result bypassed the source scheduler");
            if branches == 63 {
                assert!(result.is_ok(), "typescript={typescript}: {result:?}");
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    "unsupported at bytes 327..328: block-arrow expansion exceeds 128 nested regions",
                    "typescript={typescript}"
                );
            }
        }
    }
}

#[test]
fn concise_function_result_bodies_reset_only_expression_depth() {
    for (body_wrappers, call_wrappers) in [(100, 100), (128, 0)] {
        let body = format!(
            "const make=x=>(b,{}y=>x+y{});return {}make(a){}(b);",
            "(".repeat(body_wrappers),
            ")".repeat(body_wrappers),
            "(".repeat(call_wrappers),
            ")".repeat(call_wrappers),
        );
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, false, typescript);
            if body_wrappers == 100 {
                assert!(result.is_ok(), "typescript={typescript}: {result:?}");
                assert!(bridged > 0, "wrapped concise result bypassed the scheduler");
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    "unsupported at bytes 169..177: expression nesting exceeds this experiment's limit",
                    "typescript={typescript}"
                );
            }
        }
    }
}
