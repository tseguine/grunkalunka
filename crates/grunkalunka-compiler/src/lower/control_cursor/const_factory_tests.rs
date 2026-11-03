//! Const factory metadata keeps its own admission and publication policy while
//! its ordered caller arguments use the shared source scheduler.
use super::SCHEDULED_DECLARATION_FACTORIES;

fn on_small_stack(
    body: &str,
    asynchronous: bool,
    typescript: bool,
) -> (Result<String, String>, usize) {
    let prefix = if asynchronous { "async " } else { "" };
    let source = format!("export {prefix}function run(a,b){{{body}}}");
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            SCHEDULED_DECLARATION_FACTORIES.with(|count| count.set(0));
            let result = if asynchronous {
                crate::compile_async(&source, typescript)
            } else {
                crate::compile(&source, typescript)
            };
            (
                result.map(|artifact| artifact.wat),
                SCHEDULED_DECLARATION_FACTORIES.with(std::cell::Cell::get),
            )
        })
        .expect("2 MiB const factory argument thread")
        .join()
        .expect("const factory arguments must not overflow the native stack")
}

fn check_scheduled(bodies: &[&str], asynchronous: bool) {
    for &body in bodies {
        for typescript in [false, true] {
            let (result, scheduled) = on_small_stack(body, asynchronous, typescript);
            result.unwrap_or_else(|error| panic!("{body}, typescript={typescript}: {error}"));
            assert!(
                scheduled > 0,
                "{body}, typescript={typescript}: const factory bypassed the source scheduler"
            );
        }
    }
}

#[test]
fn const_factory_arguments_use_the_source_scheduler() {
    check_scheduled(
        &[
            "const make=x=>y=>x+y;const f=make(a);return f(b);",
            "const make=x=>y=>x+y;const f=make(a),g=make(f(b));return f(b)+g(a);",
            "const make=(a,b)=>x=>a+b+x;const f=make(a,b);return f(a);",
            "const make=x=>y=>x+y;const f=(((make(a))));return f(b);",
            "const make=x=>y=>x+y;const get=x=>{const f=make(x);return f(b);};return get(a);",
            "const make=x=>y=>x+y;let n=0;for(let i=0;i<2;i++){const f=make(i+a);n+=f(b);}return n;",
            "let n=a;const make=x=>y=>x+y;const f=make(n+=b);return n;",
        ],
        false,
    );
}

#[test]
fn const_factory_metadata_does_not_force_runtime_materialization() {
    let body = "const make=x=>y=>x+y;const f=make(a),g=make(b);return f(b)+g(a);";
    for typescript in [false, true] {
        let (result, scheduled) = on_small_stack(body, false, typescript);
        let wat = result.unwrap_or_else(|error| panic!("typescript={typescript}: {error}"));
        assert!(scheduled >= 2, "const factories bypassed the scheduler");
        assert!(
            !wat.contains("call_indirect"),
            "known const metadata acquired a runtime call"
        );
    }
}

#[test]
fn consecutive_zero_argument_factories_keep_declaration_limits() {
    // Completion must return to the heap scheduler before advancing the next
    // local, including when no argument expression causes an intervening yield.
    for count in [32, 64, 126, 127, 128] {
        let declarations = (0..count)
            .map(|index| format!("f{index}=make()"))
            .collect::<Vec<_>>()
            .join(",");
        let body = format!("const make=()=>()=>a;const {declarations};return a;");
        for typescript in [false, true] {
            let (result, scheduled) = on_small_stack(&body, false, typescript);
            if count < 128 {
                result.unwrap_or_else(|error| panic!("{count}, typescript={typescript}: {error}"));
                assert_eq!(scheduled, count, "each const factory is scheduled once");
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    "unsupported at bytes 1466..1477: at most 128 source locals are admitted",
                    "typescript={typescript}"
                );
                assert_eq!(scheduled, 127, "rejection precedes the final initializer");
            }
        }
    }
}

#[test]
fn const_factory_callbacks_and_method_arguments_use_the_source_scheduler() {
    check_scheduled(
        &[
            "let n=0;const make=(x,y)=>z=>x+y+z+n;const f=make((n=n*10+1,a),(n=n*10+2,b),n=n*10+3);return f(n=n*10+4)+n;",
            "let n=0;const make=x=>y=>x+y+n;const f=make.call((n=n*10+1,null),(n=n*10+2,a),n=n*10+3);return f(n=n*10+4)+n;",
            "let current=x=>x+a;const make=(fn,x)=>y=>fn(y)+x;const f=make(current,(current=x=>x-b,a));return f(b)+current(a);",
            "const seed=x=>{return y=>x+y;};const make=(fn,x)=>y=>fn(y)+x;const f=make(seed(a),b);return f(a);",
            "const make=(fn,x)=>y=>fn(y)+x;const f=make(a?(x=>x+a):(x=>x-b),b);return f(a);",
        ],
        false,
    );
}

#[test]
fn const_factory_managed_arguments_use_the_source_scheduler() {
    check_scheduled(
        &[
            "const make=x=>()=>x.x===a?a:b;const read=make({x:a});return read();",
            "const make=x=>()=>x[0]===a?a:b;const read=make([a]);return read();",
        ],
        false,
    );
}

#[test]
fn const_factory_awaited_arguments_use_the_source_scheduler() {
    check_scheduled(
        &[
            "const make=x=>async y=>{await x;return x+y;};const f=make(a);return await f(b);",
            "const make=x=>y=>x+y;const f=make(await a);return f(b);",
            "const make=x=>y=>x+y;const f=make.call(await a,await b);return f(a);",
        ],
        true,
    );
}

#[test]
fn const_factory_arguments_preserve_first_diagnostics() {
    for (body, expected) in [
        (
            "const make=x=>y=>x+y;const f=make();return f(b);",
            "unsupported at bytes 54..60: local call is missing declared arguments",
        ),
        (
            "const make=x=>y=>x+y;const f=make(...a);return f(b);",
            "unsupported at bytes 54..64: spread call arguments are not admitted",
        ),
        (
            "const make=(x,y)=>z=>x+y+z;const f=make(Math.noSuch(a),...b);return f(a);",
            "unsupported at bytes 65..79: expression is outside the numeric experiment",
        ),
        (
            "const make=x=>y=>x+y;const f=make.call(Math.noSuch(a),b,...a);return f(b);",
            "unsupported at bytes 64..78: expression is outside the numeric experiment",
        ),
        (
            "const make=x=>y=>x+y;const f=make(Math.noSuch(a)),g=unknown();return b;",
            "unsupported at bytes 59..73: expression is outside the numeric experiment",
        ),
        (
            "const make=x=>y=>x+y;const f=make(a),g=Math.noSuch(b);return f(b);",
            "unsupported at bytes 64..78: expression is outside the numeric experiment",
        ),
        (
            "const make=x=>y=>Math.noSuch(y);const f=make(a);return b;",
            "unsupported at bytes 42..56: expression is outside the numeric experiment",
        ),
        (
            "const make=x=>y=>x+y;const f=make?.(a);return b;",
            "unsupported at bytes 54..63: expression is outside the numeric experiment",
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
fn const_factory_metadata_adds_no_function_value_charge() {
    // Only the discarded nested calls own ordinary function-value admission.
    // The outer declaration metadata call has never acquired that charge.
    for depth in [15, 16, 17] {
        let mut argument = String::from("a");
        for _ in 0..depth {
            argument = format!("(make({argument}),a)");
        }
        let body = format!("const make=x=>y=>x+y;const f=make({argument});return f(b);");
        for typescript in [false, true] {
            let (result, scheduled) = on_small_stack(&body, false, typescript);
            if depth < 17 {
                result.unwrap_or_else(|error| panic!("{depth}, typescript={typescript}: {error}"));
                assert!(scheduled > 0, "metadata budget case bypassed the scheduler");
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    "unsupported at bytes 156..163: at most 16 active function-value cursors are admitted",
                    "typescript={typescript}"
                );
            }
        }
    }
}

#[test]
fn const_factory_metadata_adds_no_local_result_charge() {
    for depth in [7, 8, 9] {
        let mut argument = String::from("a");
        for _ in 0..depth {
            argument = format!("(charged({argument}),a)");
        }
        let body = format!(
            "const make=x=>y=>x+y;const charged=x=>{{return y=>x+y;}};const f=make({argument});return f(b);"
        );
        for typescript in [false, true] {
            let (result, scheduled) = on_small_stack(&body, false, typescript);
            if depth < 9 {
                result.unwrap_or_else(|error| panic!("{depth}, typescript={typescript}: {error}"));
                assert!(
                    scheduled > 0,
                    "metadata result budget case bypassed the scheduler"
                );
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    "unsupported at bytes 166..176: at most 8 active local function-result calls are admitted",
                    "typescript={typescript}"
                );
            }
        }
    }
}

#[test]
fn const_factory_arguments_keep_original_expression_depth() {
    for wrappers in [126, 127, 128, 129] {
        let body = format!(
            "const make=x=>y=>x+y;const f=make({}a{});return f(b);",
            "(".repeat(wrappers),
            ")".repeat(wrappers)
        );
        for typescript in [false, true] {
            let (result, scheduled) = on_small_stack(&body, false, typescript);
            if wrappers < 128 {
                result
                    .unwrap_or_else(|error| panic!("{wrappers}, typescript={typescript}: {error}"));
                assert!(scheduled > 0, "wrapped metadata bypassed the scheduler");
            } else {
                let expected = if wrappers == 128 {
                    "unsupported at bytes 187..188: expression nesting exceeds this experiment's limit"
                } else {
                    "unsupported at bytes 187..190: expression nesting exceeds this experiment's limit"
                };
                assert_eq!(result.unwrap_err(), expected, "typescript={typescript}");
            }
        }
    }
}
