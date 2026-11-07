//! Preserve function construction effects when the consumer needs only
//! Discard or truthiness. Baseline admission/diagnostics are from 926a4aa.
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
        .expect("2 MiB function-effect continuation thread")
        .join()
        .expect("function effects must not overflow the native stack")
}

#[test]
fn discarded_and_truthy_block_factories_use_the_source_scheduler() {
    for (name, body) in [
        (
            "discard_zero",
            "let n=a;const make=()=>{n+=b;return ()=>n;};make();return n;",
        ),
        (
            "discard_scalar",
            "let n=0;const make=x=>{n=n*10+2;return ()=>x+n;};make((n=n*10+1));return n;",
        ),
        (
            "predicate_scalar",
            "let n=0;const make=x=>{n=n*10+2;return ()=>x+n;};if(make((n=n*10+1)))n=n*10+3;return n;",
        ),
        (
            "predicate_conditional",
            "let n=a;const make=x=>{n+=x;return ()=>n;};return make(b)?n:b;",
        ),
        (
            "predicate_not",
            "let n=a;const make=x=>{n+=x;return ()=>n;};return +!!make(b)+n;",
        ),
        (
            "discard_and",
            "let n=a;const make=x=>{n+=x;return ()=>n;};a&&make(b);return n;",
        ),
        (
            "discard_or",
            "let n=a;const make=x=>{n+=x;return ()=>n;};a||make(b);return n;",
        ),
        (
            "discard_nullish",
            "let n=a;const make=x=>{n+=x;return ()=>n;};const left=a?null:false;left??make(b);return n;",
        ),
        (
            "function_and",
            "let n=a;const make=x=>{n+=x;return ()=>n;};make(b)&&(n+=4);return n;",
        ),
        (
            "function_or",
            "let n=a;const make=x=>{n+=x;return ()=>n;};make(b)||(n+=4);return n;",
        ),
        (
            "function_nullish",
            "let n=a;const make=x=>{n+=x;return ()=>n;};make(b)??(n+=4);return n;",
        ),
        (
            "predicate_logical",
            "let n=a;const make=x=>{n+=x;return ()=>n;};if(a&&make(b))n+=4;return n;",
        ),
        (
            "discard_callback",
            "let n=0;const make=(fn,x)=>{n=n*10+3;return ()=>fn(x);};make((n=n*10+1,y=>y+b),(n=n*10+2,a));return n;",
        ),
        (
            "predicate_returned_callback",
            "let n=0;const seed=x=>{n=n*10+1;return y=>x+y;};const make=(fn,x)=>{n=n*10+3;return ()=>fn(x);};if(make(seed(a),(n=n*10+2,b)))n=n*10+4;return n;",
        ),
        (
            "discard_surplus",
            "let n=0;const make=x=>{n=n*10+3;return ()=>x;};make((n=n*10+1),n=n*10+2);return n;",
        ),
        (
            "nested_body_effect",
            "let n=a;const make=x=>{n+=x;return ()=>n;};const touch=x=>{make(x);return n;};return touch(b);",
        ),
    ] {
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(body, typescript);
            assert!(
                result.is_ok(),
                "{name}, typescript={typescript}: {result:?}"
            );
            assert!(
                bridged > 0,
                "{name}, typescript={typescript}: function effect bypassed the source scheduler"
            );
        }
    }
}

#[test]
fn discarded_and_truthy_factories_preserve_first_diagnostics() {
    for (body, expected) in [
        (
            "const make=x=>{return ()=>x;};make();return a;",
            "unsupported at bytes 55..61: function-valued local calls require an immutable closure binding",
        ),
        (
            "const make=x=>{return ()=>x;};if(make(...a))return a;return b;",
            "unsupported at bytes 58..68: function-valued local calls require an immutable closure binding",
        ),
        (
            "const make=x=>{if(x)return ()=>x;};make(a);return b;",
            "unsupported at bytes 36..59: block arrows must return a function on every structural path",
        ),
        (
            "const make=x=>{if(x)return ()=>x;};if(make(a))return a;return b;",
            "unsupported at bytes 36..59: block arrows must return a function on every structural path",
        ),
        (
            "const make=(fn,x)=>{return ()=>fn(x);};make(a,b);return a;",
            "unsupported at bytes 37..39: callback argument requires a function with the inferred signature",
        ),
        (
            "const make=x=>{return ()=>x;};false&&make(...a);return b;",
            "unsupported at bytes 62..72: function-valued local calls require an immutable closure binding",
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
fn function_effects_keep_callback_nesting_admission() {
    for depth in [1, 7, 8] {
        let mut value = String::from("()=>a");
        for _ in 0..depth {
            value = format!("id({value})");
        }
        for statement in [
            format!("{value};return a;"),
            format!("if({value})return a;return b;"),
        ] {
            let body = format!("const id=fn=>{{return fn;}};{statement}");
            for typescript in [false, true] {
                let (result, bridged) = on_small_stack(&body, typescript);
                if depth < 8 {
                    assert!(
                        result.is_ok(),
                        "{depth}, typescript={typescript}: {result:?}"
                    );
                    assert!(
                        bridged >= depth,
                        "nested function effects bypassed the scheduler"
                    );
                } else {
                    // Retain the observed whole-source boundary and its
                    // first diagnostic; do not relabel a signature-query
                    // rejection as a measurement of native frame count.
                    assert_eq!(
                        result.unwrap_err(),
                        "unsupported at bytes 46..48: Boolean bindings are not admitted in numeric value positions",
                        "{statement}, typescript={typescript}"
                    );
                }
            }
        }
    }
}

#[test]
fn function_effects_preserve_aggregate_region_and_expression_depth() {
    for (count, expected) in [
        (126, None),
        (
            127,
            Some("unsupported at bytes 570..577: expression is outside the numeric experiment"),
        ),
    ] {
        let body = format!(
            "const make=x=>{{return ()=>x;}};return {}make(b)?1:0;",
            "a?0:".repeat(count)
        );
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, typescript);
            if let Some(expected) = expected {
                assert_eq!(
                    result.unwrap_err(),
                    expected,
                    "{count}, typescript={typescript}"
                );
            } else {
                assert!(
                    result.is_ok(),
                    "{count}, typescript={typescript}: {result:?}"
                );
                assert!(bridged > 0, "nested truthy factory bypassed the scheduler");
            }
        }
    }
    for (count, expected) in [
        (128, None),
        (
            129,
            Some("unsupported at bytes 55..56: predicate nesting exceeds this experiment's limit"),
        ),
    ] {
        let body = format!(
            "const make=x=>{{return ()=>x;}};{}make(b);return a;",
            "a&&".repeat(count)
        );
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, typescript);
            if let Some(expected) = expected {
                assert_eq!(
                    result.unwrap_err(),
                    expected,
                    "{count}, typescript={typescript}"
                );
            } else {
                assert!(
                    result.is_ok(),
                    "{count}, typescript={typescript}: {result:?}"
                );
                assert!(
                    bridged > 0,
                    "nested discarded factory bypassed the scheduler"
                );
            }
        }
    }
}
