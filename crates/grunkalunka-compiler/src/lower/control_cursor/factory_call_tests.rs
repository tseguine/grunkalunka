//! Block-factory .call invocations use the general function-value continuation.
//! The direct-block specialization retains its existing shape and outer-call
//! validation order. Baseline admission and diagnostics are from 6152b7b1.
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
        .expect("2 MiB block-factory .call thread")
        .join()
        .expect("block-factory .call must not overflow the native stack")
}

#[test]
fn block_factory_call_arguments_use_the_source_scheduler() {
    for body in [
        "const make=()=>{return ()=>a+b;};return make.call()();",
        "const make=x=>{return y=>x+y;};return make.call(null,a)(b);",
        "const make=x=>{return y=>x+y;};return ((make).call)(null,a)(b);",
        "let n=0;const make=(x,y)=>{n=n*10+5;return z=>x+y+z+n;};return make.call((n=n*10+1,null),(n=n*10+2,a),(n=n*10+3,b),n=n*10+4)(n=n*10+6)+n;",
        "let n=a;const make=(x,y)=>{return z=>x+y+z+n;};return make.call((n+=1,null),n,(n+=b),n+=10)(a)+n;",
        "const make=(a,b)=>{return x=>a+b+x;};return make.call(null,a,b)(a);",
        "let current=x=>x+a;const make=(fn,x)=>{return ()=>fn(x);};return make.call(null,current,(current=x=>x-b,a))()+current(b);",
        "const seed=x=>{return y=>x+y;};const make=(fn,x)=>{return ()=>fn(x);};return make.call(null,seed.call(null,a),b)();",
        "const make=x=>{return y=>x+y;};return (b?make.call(null,a):make.call(null,b))(a);",
        "const make=x=>{return y=>x+y;};const apply=(fn,x)=>fn(x);return apply(make.call(null,a),b);",
    ] {
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(body, false, typescript);
            assert!(
                result.is_ok(),
                "{body}, typescript={typescript}: {result:?}"
            );
            assert!(
                bridged > 0,
                "{body}, typescript={typescript}: block .call bypassed the source scheduler"
            );
        }
    }
}

#[test]
fn block_factory_call_discard_and_truthiness_use_the_source_scheduler() {
    for body in [
        "let n=0;const make=x=>{n=n*10+4;return ()=>x+n;};make.call((n=n*10+1,null),(n=n*10+2,a),n=n*10+3);return n;",
        "let n=0;const make=x=>{n=n*10+4;return ()=>x+n;};if(make.call((n=n*10+1,null),(n=n*10+2,a),n=n*10+3))n=n*10+5;return n;",
    ] {
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(body, false, typescript);
            assert!(
                result.is_ok(),
                "{body}, typescript={typescript}: {result:?}"
            );
            assert!(
                bridged > 0,
                "{body}, typescript={typescript}: block .call effect bypassed the source scheduler"
            );
        }
    }
}

#[test]
fn block_factory_call_async_callees_and_arguments_use_the_source_scheduler() {
    for body in [
        "const make=x=>{return async y=>{await x;return x+y;};};return await make.call(null,a)(b);",
        "const make=x=>{return y=>x+y;};return make.call(await a,await b)(a);",
    ] {
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(body, true, typescript);
            assert!(
                result.is_ok(),
                "{body}, typescript={typescript}: {result:?}"
            );
            assert!(
                bridged > 0,
                "{body}, typescript={typescript}: async block .call bypassed the source scheduler"
            );
        }
    }
}

#[test]
fn block_factory_call_preserves_outer_validation_and_first_diagnostics() {
    // In particular, an invalid receiver must not preempt missing outer-call
    // inputs. Broadening block_factory_shape would change that ordering.
    for (body, expected) in [
        (
            "const make=x=>{return y=>x+y;};return make.call(null)(b);".to_owned(),
            "unsupported at bytes 63..81: expression is outside the numeric experiment",
        ),
        (
            "const make=x=>{return y=>x+y;};return make.call(unknown(),a)();".to_owned(),
            "unsupported at bytes 63..87: runtime closure call is missing declared arguments",
        ),
        (
            "const make=x=>{return y=>x+y;};return make.call(unknown(),a)?.(b);".to_owned(),
            "unsupported at bytes 63..90: expression is outside the numeric experiment",
        ),
        (
            "const make=x=>{return y=>x+y;};return make.call?.(null,a)(b);".to_owned(),
            "unsupported at bytes 63..85: expression is outside the numeric experiment",
        ),
        (
            "const make=x=>{return y=>x+y;};return make.call(null,...a)(b);".to_owned(),
            "unsupported at bytes 63..86: expression is outside the numeric experiment",
        ),
        (
            "const make=x=>{return y=>x+y;};make.call(unknown(),a,...b);return a;".to_owned(),
            "unsupported at bytes 66..75: expression is outside the numeric experiment",
        ),
        (
            "const make=x=>{if(x)return ()=>x;};return make.call(null,a)();".to_owned(),
            "unsupported at bytes 36..59: block arrows must return a function on every structural path",
        ),
        (
            "const make=(fn,x)=>{return ()=>fn(x);};return make.call(null,a,b)();".to_owned(),
            "unsupported at bytes 37..39: callback argument requires a function with the inferred signature",
        ),
        (
            format!(
                "const make=x=>{{return y=>x+y;}};return make.call(unknown(),a)({});",
                vec!["b"; 1001].join(",")
            ),
            "unsupported at bytes 63..2088: at most 1000 ordinary runtime closure arguments are admitted",
        ),
        (
            format!(
                "const make=x=>{{return y=>x+y;}};make.call(unknown(),{});return a;",
                vec!["a"; 1000].join(",")
            ),
            "unsupported at bytes 56..2076: at most 1000 ordinary local call arguments are admitted",
        ),
    ] {
        for typescript in [false, true] {
            let (result, _) = on_small_stack(&body, false, typescript);
            assert_eq!(
                result.unwrap_err(),
                expected,
                "{body}, typescript={typescript}"
            );
        }
    }
}

#[test]
fn block_factory_call_keeps_the_eight_active_result_budget() {
    // Numeric prefixes keep the signature query shallow while holding each
    // caller's result token across its still-pending argument evaluation.
    for depth in [6, 7, 8, 9] {
        let mut argument = String::from("a");
        for _ in 0..depth {
            argument = format!("(make.call(null,{argument}),a)");
        }
        let body =
            format!("const make=x=>{{return y=>x+y;}};return make.call(null,{argument})(b);");
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, false, typescript);
            if depth < 8 {
                assert!(
                    result.is_ok(),
                    "{depth}, typescript={typescript}: {result:?}"
                );
                assert!(
                    bridged > depth,
                    "nested block .call results bypassed the scheduler"
                );
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    format!(
                        "unsupported at bytes 191..{}: at most 8 active local function-result calls are admitted",
                        208 + 20 * (depth - 8)
                    ),
                    "{depth}, typescript={typescript}"
                );
            }
        }
    }
}

#[test]
fn block_factory_call_callback_nesting_keeps_its_existing_boundary() {
    // The first rejected signature query is an observed whole-source boundary,
    // not a measurement of callback admissions or native activation count.
    for depth in [6, 7, 8, 9] {
        let mut value = String::from("()=>a");
        for _ in 0..depth {
            value = format!("id.call(null,{value})");
        }
        let body = format!("const id=fn=>{{return fn;}};return {value}();");
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, false, typescript);
            if depth < 8 {
                assert!(
                    result.is_ok(),
                    "{depth}, typescript={typescript}: {result:?}"
                );
                assert!(
                    bridged >= depth,
                    "callback block .call results bypassed the scheduler"
                );
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    format!(
                        "unsupported at bytes 58..{}: expression is outside the numeric experiment",
                        177 + 14 * (depth - 8)
                    ),
                    "{depth}, typescript={typescript}"
                );
            }
        }
    }
}

#[test]
fn block_factory_call_keeps_original_argument_expression_depth() {
    for wrappers in [126, 127, 128] {
        let body = format!(
            "const make=x=>{{return y=>x+y;}};return make.call(null,{}a{})(b);",
            "(".repeat(wrappers),
            ")".repeat(wrappers),
        );
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, false, typescript);
            if wrappers == 126 {
                assert!(result.is_ok(), "typescript={typescript}: {result:?}");
                assert!(
                    bridged > 0,
                    "wrapped block .call argument bypassed the scheduler"
                );
            } else {
                let end = if wrappers == 127 { 206 } else { 208 };
                assert_eq!(
                    result.unwrap_err(),
                    format!(
                        "unsupported at bytes 205..{end}: expression nesting exceeds this experiment's limit"
                    ),
                    "{wrappers}, typescript={typescript}"
                );
            }
        }
    }
}

#[test]
fn block_factory_call_restores_caller_depth_before_outer_arguments() {
    for branches in [63, 64] {
        let body = format!(
            "const make=x=>{{return y=>x+y;}};const step=()=>{{return {}a;}};{}return make.call(null,a)(step());{}",
            "a?0:".repeat(branches),
            "{".repeat(64),
            "}".repeat(64),
        );
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, false, typescript);
            assert!(
                bridged > 0,
                "block .call result bypassed the source scheduler"
            );
            if branches == 63 {
                assert!(result.is_ok(), "typescript={typescript}: {result:?}");
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    "unsupported at bytes 333..334: block-arrow expansion exceeds 128 nested regions",
                    "typescript={typescript}"
                );
            }
        }
    }
}
