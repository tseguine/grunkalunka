//! Runtime function results schedule their callee before the shared argument
//! continuation. Admission and exact errors were frozen on predecessor 59629b1;
//! the bridge count distinguishes this source-owned path from compatibility.
use super::BRIDGED_RUNTIME_FACTORIES;

fn on_small_stack(body: &str, asynchronous: bool, typescript: bool) -> (Result<(), String>, usize) {
    let prefix = if asynchronous { "async " } else { "" };
    let source = format!("export {prefix}function run(a,b){{{body}}}");
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            BRIDGED_RUNTIME_FACTORIES.with(|count| count.set(0));
            let result = if asynchronous {
                crate::compile_async(&source, typescript).map(|_| ())
            } else {
                crate::compile(&source, typescript).map(|_| ())
            };
            let bridged = BRIDGED_RUNTIME_FACTORIES.with(std::cell::Cell::get);
            (result, bridged)
        })
        .expect("2 MiB runtime factory continuation thread")
        .join()
        .expect("runtime factories must not overflow the native stack")
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
                "{body}, typescript={typescript}: runtime factory bypassed the source scheduler"
            );
        }
    }
}

#[test]
fn runtime_factory_callees_use_the_source_scheduler() {
    check_scheduled(
        false,
        &[
            "let make=x=>()=>x+a;return make(b)();",
            "let make=x=>()=>x+a;return make((make=x=>()=>x-b,a))()+make(b)();",
            "let f=x=>()=>x+a;let g=x=>()=>x-b;return(a?f:g)(b)();",
            "let n=0;let make=x=>()=>x+n;return(n=n*10+1,make)((n=n*10+2,a),n=n*10+3)()+n;",
            "let f=x=>()=>x+a;let g=x=>()=>x-b;return(f||g)(b)();",
            "let f=x=>()=>x+a;let g=x=>()=>x-b;return(f=g)(a)()+f(b)();",
            "let f=x=>()=>x+a;let g=x=>()=>x-b;return(a?f:g).call(null,b)();",
        ],
    );
}

#[test]
fn runtime_factory_typed_arguments_use_the_source_scheduler() {
    check_scheduled(
        false,
        &[
            "let make=(fn,x)=>()=>fn(x);return make(y=>y+a,b)();",
            "const seed=x=>{return y=>x+y;};let make=(fn,x)=>()=>fn(x);return make(seed(a),b)();",
            "let fn=x=>x+a;let make=(fn,x)=>()=>fn(x);return make(fn,(fn=x=>x-b,b))()+fn(a);",
            "let n=0;let make=x=>()=>x+n;return make.call((n=1,null),(n=n*10+2,a),n=n*10+3)()+n;",
            "let n=0;let make=x=>()=>x+n;const extra=x=>y=>x+y;return make(a,extra(n+=b))()+n;",
            "const step=x=>{return x+a;};let make=x=>()=>x;return make(step(b))();",
            "let make=x=>()=>x;return +make(null)();",
            "let make=x=>()=>x.x===a?a:b;return make({x:a})();",
            "let make=x=>()=>x[0]===a?a:b;return make([a])();",
            "let make=x=>()=>x===x?a:b;return make(a?{x:a}:[b])();",
        ],
    );
}

#[test]
fn runtime_factory_async_paths_use_the_source_scheduler() {
    check_scheduled(
        true,
        &[
            "let make=p=>()=>p;return await make(Promise.resolve(a))();",
            "const get=async x=>{await x;return x;};let make=p=>()=>p;return await make(get(a))();",
            "let make=x=>()=>x;return await make(a?Promise.resolve(b):a)();",
            "const get=async x=>{await x;return x+b;};let make=x=>()=>x+a;return make(await get(a))();",
            "let make=fn=>async x=>await fn(x);return await make(async x=>{await a;return x+b;})(a);",
            "const consume=async p=>await p;let make=x=>async y=>x+y;return await consume(make(a)(b));",
            "let make=x=>async()=>x.x===a?a:b;return await make({x:a})();",
        ],
    );
}

#[test]
fn runtime_factory_continuations_preserve_first_diagnostics() {
    for (body, expected) in [
        (
            "let make=(x,y)=>()=>x+y;return(Math.noSuch(a),make)(b)();",
            "unsupported at bytes 55..79: runtime factory requires a plain call, all declared inputs and at most 1000 arguments",
        ),
        (
            "let make=x=>()=>x;return(Math.noSuch(a),make)(Math.noSuch(b))();",
            "unsupported at bytes 50..64: expression is outside the numeric experiment",
        ),
        (
            "let make=x=>()=>x;return make(Math.noSuch(a),...b)();",
            "unsupported at bytes 55..69: expression is outside the numeric experiment",
        ),
        (
            "let make=x=>()=>x;return make.call(Math.noSuch(a),Math.noSuch(b))();",
            "unsupported at bytes 60..74: expression is outside the numeric experiment",
        ),
        (
            "let make=x=>()=>x;return make.call(Math.noSuch(a),...b)();",
            "unsupported at bytes 60..74: expression is outside the numeric experiment",
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

fn metadata_wrappers(mut argument: String, count: usize) -> String {
    for _ in 0..count {
        argument = format!("wrap(({argument},a))");
    }
    argument
}

#[test]
fn runtime_factory_slot_callees_keep_ordinary_function_value_admission() {
    // All callees, including slots, acquire the ordinary sixteen-value guard.
    // The callback-specific eight-value guard must not replace it.
    for wrappers in [6, 8, 14, 15] {
        let argument = metadata_wrappers(String::from("make(a)"), wrappers);
        let body = format!("const wrap=x=>()=>x;let make=x=>()=>x;return({argument},a);");
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, false, typescript);
            if wrappers < 15 {
                assert!(
                    result.is_ok(),
                    "{wrappers}, typescript={typescript}: {result:?}"
                );
                assert!(bridged > 0, "slot runtime callee bypassed the scheduler");
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    "unsupported at bytes 160..164: at most 16 active function-value cursors are admitted"
                );
            }
        }
    }
    // Preparation's grouped validation precedes that callee guard, which in
    // turn precedes the first argument's effects and diagnostics.
    for (call, expected) in [
        (
            "make()",
            "unsupported at bytes 160..166: runtime factory requires a plain call, all declared inputs and at most 1000 arguments",
        ),
        (
            "make(Math.noSuch(b))",
            "unsupported at bytes 160..164: at most 16 active function-value cursors are admitted",
        ),
    ] {
        let argument = metadata_wrappers(call.to_string(), 15);
        let body = format!("const wrap=x=>()=>x;let make=x=>()=>x;return({argument},a);");
        for typescript in [false, true] {
            assert_eq!(
                on_small_stack(&body, false, typescript).0.unwrap_err(),
                expected
            );
        }
    }
}

#[test]
fn runtime_factory_callee_charge_ends_before_callback_arguments() {
    // Six surrounding metadata factories leave exactly enough admission for
    // this callback after the runtime callee's own charge has been released.
    for wrappers in [6, 7] {
        let argument = metadata_wrappers(String::from("make(x=>x+b)"), wrappers);
        let body = format!("const wrap=x=>()=>x;let make=fn=>()=>fn(a);return({argument},a);");
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, false, typescript);
            if wrappers == 6 {
                assert!(result.is_ok(), "typescript={typescript}: {result:?}");
                assert!(bridged > 0, "runtime callback bypassed the scheduler");
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    "unsupported at bytes 122..128: callback arguments require fewer than 8 active function-value cursors"
                );
            }
        }
    }
}

#[test]
fn runtime_factory_arguments_keep_the_active_result_budget() {
    for depth in [8, 9] {
        let mut expression = String::from("a");
        for _ in 0..depth {
            expression = format!("make({expression})()");
        }
        let body = format!("let make=x=>()=>x;return {expression};");
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, false, typescript);
            if depth == 8 {
                assert!(result.is_ok(), "typescript={typescript}: {result:?}");
                assert!(
                    bridged >= depth,
                    "nested runtime results bypassed the scheduler"
                );
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    "unsupported at bytes 90..97: at most 8 active runtime function-result calls are admitted"
                );
            }
        }
    }
    let mut expression = String::from("make()");
    for _ in 0..8 {
        expression = format!("make({expression}())");
    }
    let body = format!("let make=x=>()=>x;return {expression}();");
    for typescript in [false, true] {
        assert_eq!(
            on_small_stack(&body, false, typescript).0.unwrap_err(),
            // The innermost omitted input now carries Undefined. This source
            // asks the numeric export to return it through runtime factories;
            // the value-position guard precedes expansion of the nested input.
            "unsupported at bytes 50..122: expression is outside the numeric experiment: managed value requires an identity or managed storage position"
        );
    }
}

#[test]
fn runtime_factory_children_keep_original_expression_depth() {
    for (callee, accepted, rejected) in [(true, 125, 126), (false, 126, 127)] {
        for wrappers in [accepted, rejected] {
            let body = if callee {
                format!(
                    "let make=x=>()=>x;return({}make{})(a)();",
                    "(".repeat(wrappers),
                    ")".repeat(wrappers)
                )
            } else {
                format!(
                    "let make=x=>()=>x;return make({}a{})();",
                    "(".repeat(wrappers),
                    ")".repeat(wrappers)
                )
            };
            for typescript in [false, true] {
                let (result, bridged) = on_small_stack(&body, false, typescript);
                if wrappers == accepted {
                    assert!(
                        result.is_ok(),
                        "callee={callee}, typescript={typescript}: {result:?}"
                    );
                    assert!(
                        bridged > 0,
                        "wrapped runtime factory bypassed the scheduler"
                    );
                } else {
                    assert_eq!(
                        result.unwrap_err(),
                        if callee {
                            "unsupported at bytes 49..312: expression is outside the numeric experiment"
                        } else {
                            "unsupported at bytes 182..183: expression nesting exceeds this experiment's limit"
                        }
                    );
                }
            }
        }
    }
}

#[test]
fn runtime_factory_expansion_is_charged_after_argument_effects() {
    let mut calls = vec!["make(a)()"; 127];
    calls.extend(["make(a)", "make(Math.noSuch(b))()"]);
    let body = format!("let make=x=>()=>x;return({});", calls.join(","));
    for typescript in [false, true] {
        assert_eq!(
            on_small_stack(&body, false, typescript).0.unwrap_err(),
            "unsupported at bytes 1333..1347: expression is outside the numeric experiment"
        );
    }
}

#[test]
fn runtime_factory_slot_snapshot_keeps_callee_and_call_spans() {
    use crate::ir::{Op, Type};
    let source = "export function run(a,b){let make=x=>()=>x+a;return make((make=x=>()=>x-b,a))()+make(b)();}";
    for typescript in [false, true] {
        let module = crate::lower_source(source, typescript).unwrap();
        let function = &module.functions[0];
        let operations = &function.body.operations;
        let (call_index, callee) = operations
            .iter()
            .enumerate()
            .find_map(|(index, operation)| match &operation.kind {
                Op::ClosureCall { result, callee, .. }
                    if matches!(function.values[result.0], Type::Closure(_)) =>
                {
                    Some((index, *callee))
                }
                _ => None,
            })
            .expect("runtime factory ClosureCall");
        let (load_index, slot) = operations
            .iter()
            .enumerate()
            .find_map(|(index, operation)| match &operation.kind {
                Op::Load { result, slot } if *result == callee => Some((index, *slot)),
                _ => None,
            })
            .expect("runtime callee snapshot");
        let load_span = operations[load_index].span;
        let callee_start = source.find("return make").unwrap() + "return ".len();
        assert_eq!(
            (load_span.start as usize, load_span.end as usize),
            (callee_start, callee_start + 4)
        );
        let call_span = operations[call_index].span;
        assert_eq!(
            &source[call_span.start as usize..call_span.end as usize],
            "make((make=x=>()=>x-b,a))"
        );
        assert!(
            operations[load_index + 1..call_index]
                .iter()
                .any(|operation| {
                    matches!(&operation.kind, Op::Store { slot: changed, .. } if *changed == slot)
                }),
            "callee snapshot must precede its argument's assignment"
        );
    }
}
