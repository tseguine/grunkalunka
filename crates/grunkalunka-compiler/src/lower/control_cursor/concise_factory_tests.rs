//! Concise factories return closure metadata after their caller arguments.
//! Their source-owned argument continuation must retain that distinct contract.
use super::BRIDGED_CONCISE_FACTORIES;

fn on_small_stack(body: &str, typescript: bool) -> (Result<(), String>, usize) {
    on_small_stack_mode(body, false, typescript)
}

fn on_small_stack_mode(
    body: &str,
    asynchronous: bool,
    typescript: bool,
) -> (Result<(), String>, usize) {
    let prefix = if asynchronous { "async " } else { "" };
    let source = format!("export {prefix}function run(a,b){{{body}}}");
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            BRIDGED_CONCISE_FACTORIES.with(|count| count.set(0));
            let result = if asynchronous {
                crate::compile_async(&source, typescript).map(|_| ())
            } else {
                crate::compile(&source, typescript).map(|_| ())
            };
            let bridged = BRIDGED_CONCISE_FACTORIES.with(std::cell::Cell::get);
            (result, bridged)
        })
        .expect("2 MiB concise factory argument thread")
        .join()
        .expect("concise factory arguments must not overflow the native stack")
}

fn check_scheduled(cases: &[&str]) {
    for &body in cases {
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(body, typescript);
            assert!(
                result.is_ok(),
                "{body}, typescript={typescript}: {result:?}"
            );
            assert!(
                bridged > 0,
                "{body}, typescript={typescript}: concise factory bypassed the source scheduler"
            );
        }
    }
}

#[test]
fn concise_factory_arguments_use_the_source_scheduler() {
    check_scheduled(&[
        "const make=x=>y=>x+y;return make()(b);",
        "const make=x=>y=>x+y;return make(a)(b);",
        "let n=0;const make=(x,y)=>z=>x*100+y*10+z+n;return make((n=n*10+1),(n=n*10+2),n=n*10+3)(n=n*10+4)+n;",
        "const make=(a,b)=>x=>a+b+x;return make(a,b)(a);",
        "const inner=x=>x+a;const make=()=>inner;return make()(b);",
        "const make=x=>y=>x+y;return (make(a))(b);",
        "const make=x=>y=>x+y;return make.call(null,a)(b);",
        "const make=x=>y=>x+y;return a?make(a)(b):make(b)(a);",
        "const make=x=>y=>x+y;return(b?make(a):make(b))(a);",
        "const make=x=>y=>x+y;const wrap=x=>{return make(x);};return wrap(a)(b);",
    ]);
}

#[test]
fn concise_factory_callbacks_and_effects_use_the_source_scheduler() {
    check_scheduled(&[
        "const seed=x=>{return y=>x+y;};const make=(fn,x)=>y=>fn(y)+x;return make(seed(a),b)(a);",
        "let f=x=>x+a;const make=(fn,x)=>y=>fn(y)+x;return make(f,(f=x=>x-b,a))(b)+f(a);",
        "const make=x=>y=>x+y;const apply=(fn,x)=>fn(x);return apply(make(a),b);",
        "let n=a;const make=x=>y=>x+y;make(n+=b);return n;",
        "let n=0;const make=x=>y=>x+y;if(make(n=n*10+1))n=n*10+2;return n;",
    ]);
}

#[test]
fn concise_factory_async_callees_use_the_source_scheduler() {
    let body = "const make=x=>async y=>x+y;return await(b?make(a):make(b))(a);";
    for typescript in [false, true] {
        let (result, bridged) = on_small_stack_mode(body, true, typescript);
        assert!(result.is_ok(), "typescript={typescript}: {result:?}");
        assert!(
            bridged > 0,
            "awaited concise factory bypassed the source scheduler"
        );
    }
}

#[test]
fn concise_factory_arguments_preserve_first_diagnostics() {
    for (body, expected) in [
        (
            "const make=x=>y=>x+y;return make(...a)(b);",
            "unsupported at bytes 53..63: spread call arguments are not admitted",
        ),
        (
            "const make=(fn,x)=>y=>fn(y)+x;return make(a,b)(a);",
            "unsupported at bytes 37..39: callback argument requires a function with the inferred signature",
        ),
        (
            "const make=(x=1)=>y=>x+y;return make(a)(b);",
            "unsupported at bytes 37..40: local arrow parameters cannot have defaults, destructuring or modifiers",
        ),
        (
            "const make=x=>y=>x+y;return make?.(a)(b);",
            "unsupported at bytes 53..65: expression is outside the numeric experiment",
        ),
    ] {
        for typescript in [false, true] {
            let (result, _) = on_small_stack(body, typescript);
            assert_eq!(
                result.unwrap_err(),
                expected,
                "{body}, typescript={typescript}"
            );
        }
    }
}

#[test]
fn concise_factory_arguments_keep_their_function_value_budget() {
    // Metadata factories never owned the separate eight-result charge. Their
    // nested discarded arguments reach the existing sixteen-function-value
    // boundary instead; sharing block-factory scheduling must retain that.
    for depth in [7, 8, 15, 16] {
        let mut argument = String::from("a");
        for _ in 0..depth {
            argument = format!("(make({argument}),a)");
        }
        let body = format!("const make=x=>y=>x+y;return make({argument})(b);");
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, typescript);
            if depth < 16 {
                assert!(
                    result.is_ok(),
                    "{depth}, typescript={typescript}: {result:?}"
                );
                assert!(
                    bridged > depth,
                    "nested concise factories bypassed the scheduler"
                );
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    "unsupported at bytes 149..156: at most 16 active function-value cursors are admitted",
                    "typescript={typescript}"
                );
            }
        }
    }
}

#[test]
fn concise_factory_arguments_keep_original_expression_depth() {
    for wrappers in [126, 127] {
        let body = format!(
            "const make=x=>y=>x+y;return make({}a{})(b);",
            "(".repeat(wrappers),
            ")".repeat(wrappers)
        );
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, typescript);
            if wrappers == 126 {
                assert!(result.is_ok(), "typescript={typescript}: {result:?}");
                assert!(
                    bridged > 0,
                    "wrapped concise factory bypassed the scheduler"
                );
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    "unsupported at bytes 185..186: expression nesting exceeds this experiment's limit",
                    "typescript={typescript}"
                );
            }
        }
    }
}
