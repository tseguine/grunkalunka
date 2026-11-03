//! Callback arguments of ordinary local and runtime closure calls must
//! yield to the shared source scheduler before lowering a returned factory.
//! These source admission and diagnostic boundaries are baseline 926a4aa.
use super::BRIDGED_BLOCK_FACTORIES;

fn on_small_stack(body: &str, typescript: bool) -> (Result<(), String>, usize) {
    let source = format!("export function run(a,b){{{body}}}");
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            BRIDGED_BLOCK_FACTORIES.with(|count| count.set(0));
            let result = crate::compile(&source, typescript).map(|_| ());
            let bridged = BRIDGED_BLOCK_FACTORIES.with(std::cell::Cell::get);
            (result, bridged)
        })
        .expect("2 MiB callback argument continuation thread")
        .join()
        .expect("callback arguments must not overflow the native stack")
}

fn check_scheduled(cases: &[(&str, &str)]) {
    for &(name, body) in cases {
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(body, typescript);
            assert!(
                result.is_ok(),
                "{name}, typescript={typescript}: {result:?}"
            );
            assert!(
                bridged > 0,
                "{name}, typescript={typescript}: callback factory bypassed the source scheduler"
            );
        }
    }
}

#[test]
fn local_call_callback_factories_use_the_source_scheduler() {
    check_scheduled(&[
        (
            "local_callback",
            "const make=x=>{return y=>x+y;};const apply=(fn,x)=>fn(x);return apply(make(a),b);",
        ),
        (
            "local_block_callback",
            "const make=x=>{return y=>x+y;};const apply=(fn,x)=>{return fn(x);};return apply(make(a),b);",
        ),
        (
            "local_multiple_callbacks",
            "const make=x=>{return y=>x+y;};const combine=(f,g,x)=>f(x)+g(x);return combine(make(a),make(b),a);",
        ),
        (
            "callback_scalar_extra_order",
            "let n=0;const make=x=>{n=n*10+1;return y=>x+y;};const apply=(fn,x)=>fn(x);return apply(make(a),(n=n*10+2,b),n=n*10+3)+n;",
        ),
        (
            "local_call_offset",
            "const make=x=>{return y=>x+y;};const apply=(fn,x)=>fn(x);return apply.call(null,make(a),b);",
        ),
        (
            "local_nested_region",
            "const make=x=>{return y=>x+y;};const apply=(fn,x)=>fn(x);return a?apply(make(a),b):apply(make(b),a);",
        ),
    ]);
}

#[test]
fn runtime_call_callback_factories_use_the_source_scheduler() {
    check_scheduled(&[
        (
            "runtime_callback",
            "const make=x=>{return y=>x+y;};const apply=a?(fn,x)=>fn(x):(fn,x)=>fn(x)+1;return apply(make(a),b);",
        ),
        (
            "runtime_call_offset",
            "const make=x=>{return y=>x+y;};const apply=a?(fn,x)=>fn(x):(fn,x)=>fn(x)+1;return apply.call(null,make(a),b);",
        ),
        (
            "runtime_callee_snapshot",
            "const make=x=>{return y=>x+y;};let apply=(fn,x)=>fn(x);return apply(make(a),(apply=(fn,x)=>fn(x)+10,b))+apply(y=>y,a);",
        ),
    ]);
}

#[test]
fn callback_argument_continuations_preserve_first_diagnostics() {
    for (body, expected) in [
        (
            "const make=x=>{return y=>x+y;};const apply=(fn,x)=>fn(x);return apply(make(a));",
            "unsupported at bytes 89..103: local call is missing declared arguments",
        ),
        (
            "const make=x=>{return y=>x+y;};const apply=(fn,x)=>fn(x);return apply(make(a),...b);",
            "unsupported at bytes 89..108: spread call arguments are not admitted",
        ),
        (
            "const make=x=>{return y=>x+y;};const apply=a?(fn,x)=>fn(x):(fn,x)=>fn(x)+1;return apply(make(a));",
            "unsupported at bytes 107..121: runtime closure call is missing declared arguments",
        ),
        (
            "const make=x=>{return y=>x+y;};const apply=a?(fn,x)=>fn(x):(fn,x)=>fn(x)+1;return apply(make(a),...b);",
            "unsupported at bytes 107..126: spread call arguments are not admitted",
        ),
        (
            "const apply=(fn,x)=>fn(x);return apply(a,b);",
            "unsupported at bytes 38..40: callback argument requires a function with the inferred signature",
        ),
        (
            "const apply=a?(fn,x)=>fn(x):(fn,x)=>fn(x)+1;return apply((x,y)=>x+y,b);",
            "unsupported at bytes 39..52: callback parameters require one fixed complete function signature",
        ),
        (
            "const make=x=>{if(x)return y=>x+y;};const apply=(fn,x)=>fn(x);return apply(make(a),b);",
            "unsupported at bytes 36..60: block arrows must return a function on every structural path",
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
fn callback_argument_continuations_keep_nested_factory_admission() {
    for depth in [0, 6, 7, 8] {
        let mut value = String::from("x=>x+a");
        for _ in 0..depth {
            value = format!("id({value})");
        }
        let body = format!(
            "const id=fn=>{{return fn;}};const apply=(fn,x)=>fn(x);return apply({value},b);"
        );
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, typescript);
            if depth < 8 {
                assert!(
                    result.is_ok(),
                    "{depth}, typescript={typescript}: {result:?}"
                );
                assert!(
                    bridged >= depth,
                    "nested callback factories bypassed the scheduler"
                );
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    "unsupported at bytes 46..48: Boolean bindings are not admitted in numeric value positions",
                    "{depth}, typescript={typescript}"
                );
            }
        }
    }
}

#[test]
fn callback_argument_continuations_preserve_original_expression_depth() {
    for wrappers in [127, 128] {
        let body = format!(
            "const apply=(fn,x)=>fn(x);return apply({}x=>x+a{},b);",
            "(".repeat(wrappers),
            ")".repeat(wrappers)
        );
        for typescript in [false, true] {
            let (result, _) = on_small_stack(&body, typescript);
            if wrappers == 127 {
                assert!(result.is_ok(), "typescript={typescript}: {result:?}");
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    "unsupported at bytes 192..198: expression nesting exceeds this experiment's limit",
                    "typescript={typescript}"
                );
            }
        }
    }
}
