//! Non-slot synchronous, awaited and started callees must construct their
//! returned factories on the same source scheduler before evaluating arguments.
use super::BRIDGED_BLOCK_FACTORIES;

fn on_small_stack(body: &str, asynchronous: bool, typescript: bool) -> (Result<(), String>, usize) {
    let prefix = if asynchronous { "async " } else { "" };
    let source = format!("export {prefix}function run(a,b){{{body}}}");
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            BRIDGED_BLOCK_FACTORIES.with(|count| count.set(0));
            let result = if asynchronous {
                crate::compile_async(&source, typescript).map(|_| ())
            } else {
                crate::compile(&source, typescript).map(|_| ())
            };
            let bridged = BRIDGED_BLOCK_FACTORIES.with(std::cell::Cell::get);
            (result, bridged)
        })
        .expect("2 MiB callee continuation thread")
        .join()
        .expect("callee construction must not overflow the native stack")
}

fn check_scheduled(asynchronous: bool, cases: &[&str]) {
    for &body in cases {
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(body, asynchronous, typescript);
            assert!(
                result.is_ok(),
                "{body}, typescript={typescript}: {result:?}"
            );
            assert!(
                bridged > 0,
                "{body}, typescript={typescript}: callee factory bypassed the source scheduler"
            );
        }
    }
}

#[test]
fn synchronous_calculated_callees_use_the_source_scheduler() {
    check_scheduled(
        false,
        &[
            "const get=()=>{return x=>x+a;};return(b?get():get())(b);",
            "const get=()=>{return x=>x+a;};return(a,get())(b);",
            "const get=()=>{return x=>x+a;};return(get()||get())(b);",
            "const get=()=>{return x=>x+a;};return(get()??get())(b);",
            "const get=()=>{return x=>x+a;};return(b?get():get()).call(null,b);",
        ],
    );
}

#[test]
fn awaited_calculated_callees_use_the_source_scheduler() {
    check_scheduled(
        true,
        &[
            "const get=()=>{return async x=>{await x;return x+a;};};return await(b?get():get())(b);",
            "const get=()=>{return async x=>{await x;return x+a;};};return await(a,get())(b);",
            "const get=()=>{return async x=>{await x;return x+a;};};return await(b?get():get()).call(null,b);",
        ],
    );
}

#[test]
fn started_calculated_callees_use_the_source_scheduler() {
    // AsyncReturn and a source-owned call's Promise argument both request
    // Pending evaluation; a specialized const initializer alone would still
    // exercise the synchronous compatibility host instead of this route.
    check_scheduled(
        true,
        &[
            "const get=()=>{return async x=>{await x;return x+a;};};return(b?get():get())(b);",
            "const get=()=>{return async x=>{await x;return x+a;};};return(b?get():get()).call(null,b);",
            "const get=()=>{return async x=>{await x;return x+a;};};const consume=async p=>await p;return await consume((b?get():get())(b));",
        ],
    );
}

#[test]
fn calculated_callee_continuations_preserve_first_diagnostics() {
    for (asynchronous, body, expected) in [
        (
            false,
            "const f=(x,y)=>x+y;return(Math.noSuch(a),f)(b);",
            "unsupported at bytes 50..71: runtime closure call is missing declared arguments",
        ),
        (
            false,
            "const f=x=>x;return(Math.noSuch(a),f)(...b);",
            "unsupported at bytes 45..59: expression is outside the numeric experiment",
        ),
        (
            false,
            "const f=x=>x+a;const g=x=>x+b;return(a?f:g)(...b);",
            "unsupported at bytes 61..74: spread call arguments are not admitted",
        ),
        (
            false,
            "const f=x=>x;return(Math.noSuch(a),f)?.(b);",
            "unsupported at bytes 44..67: expression is outside the numeric experiment",
        ),
        (
            false,
            "const get=()=>{return x=>true;};return(a,get())(Math.noSuch(b));",
            "unsupported at bytes 63..88: primitive closure result requires a value or coercion position",
        ),
        (
            false,
            "const get=()=>{return x=>true;};return(Math.noSuch(a),get())(Math.noSuch(b));",
            "unsupported at bytes 64..78: expression is outside the numeric experiment",
        ),
        (
            true,
            "const f=async(x,y)=>x+y;return await(Math.noSuch(a),f)(b);",
            "unsupported at bytes 67..88: runtime closure call is missing declared arguments",
        ),
        (
            true,
            "const f=async x=>x;return await(Math.noSuch(a),f)(...b);",
            "unsupported at bytes 63..77: expression is outside the numeric experiment",
        ),
        (
            true,
            "const f=async(x,y)=>x+y;return(Math.noSuch(a),f)(b);",
            "unsupported at bytes 61..82: runtime closure call is missing declared arguments",
        ),
        (
            true,
            "const f=async x=>x;return(Math.noSuch(a),f)(...b);",
            "unsupported at bytes 57..71: expression is outside the numeric experiment",
        ),
    ] {
        for typescript in [false, true] {
            let (result, _) = on_small_stack(body, asynchronous, typescript);
            assert_eq!(
                result.unwrap_err(),
                expected,
                "{body}, typescript={typescript}"
            );
        }
    }
}

#[test]
fn calculated_callee_continuations_keep_the_original_depths() {
    for (asynchronous, awaited, accepted, expected) in [
        (
            false,
            false,
            125,
            "unsupported at bytes 62..324: expression is outside the numeric experiment",
        ),
        (
            true,
            true,
            124,
            "unsupported at bytes 86..346: expression is outside the numeric experiment",
        ),
        (
            true,
            false,
            125,
            "unsupported at bytes 80..342: expression is outside the numeric experiment",
        ),
    ] {
        for wrappers in [accepted, accepted + 1] {
            let arrow = if asynchronous { "async " } else { "" };
            let await_prefix = if awaited { "await " } else { "" };
            let body = format!(
                "const f={arrow}x=>x+a;const g={arrow}x=>x+b;return {await_prefix}{}(a?f:g){}(b);",
                "(".repeat(wrappers),
                ")".repeat(wrappers)
            );
            for typescript in [false, true] {
                let (result, _) = on_small_stack(&body, asynchronous, typescript);
                if wrappers == accepted {
                    assert!(
                        result.is_ok(),
                        "{body}, typescript={typescript}: {result:?}"
                    );
                } else {
                    assert_eq!(
                        result.unwrap_err(),
                        expected,
                        "{body}, typescript={typescript}"
                    );
                }
            }
        }
    }
}

#[test]
fn calculated_callee_continuations_keep_function_result_admission() {
    for depth in [1, 7, 8, 9] {
        let mut expression = String::from("a");
        for _ in 0..depth {
            expression = format!("(a,make({expression}))()");
        }
        let body = format!("const make=x=>{{return()=>x;}};return {expression};");
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, false, typescript);
            if depth < 9 {
                assert!(
                    result.is_ok(),
                    "{depth}, typescript={typescript}: {result:?}"
                );
                assert!(
                    bridged >= depth,
                    "nested callee factories bypassed the scheduler"
                );
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    "unsupported at bytes 128..135: at most 8 active local function-result calls are admitted",
                    "typescript={typescript}"
                );
            }
        }
    }
}
