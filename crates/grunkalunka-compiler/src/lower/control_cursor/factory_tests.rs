//! Source-compiled evidence for argument-bearing returned block factories.
//! Admission and diagnostics were characterized on master 926a4aa before the
//! argument continuation migration. The bridge count verifies actual routing;
//! it is not a bound on all scratch validation or native compiler recursion.
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
        .expect("2 MiB returned-factory argument thread")
        .join()
        .expect("factory arguments must not overflow the native stack")
}

#[test]
fn returned_block_factory_arguments_use_the_source_scheduler() {
    for (name, body) in [
        (
            "scalar",
            "const make=x=>{return ()=>x+a;};return make(b)();",
        ),
        (
            "multi_scalar",
            "const make=(x,y)=>{return ()=>x*10+y+a;};return make(a,b)();",
        ),
        (
            "argument_effects",
            "let n=0;const make=(x,y)=>{n=n*10+3;return ()=>x*10+y+n;};return make((n=n*10+1),(n=n*10+2),n=n*10+4)()+n;",
        ),
        (
            "capture_snapshot",
            "let n=a;const make=(x,y)=>{return ()=>x*10+y+n;};return make(n,(n+=b),n+=10)()+n;",
        ),
        (
            "nested_scalar_body",
            "const step=x=>{return x+b;};const make=x=>{return ()=>x+a;};return make(step(b))();",
        ),
        (
            "callback",
            "const make=(fn,x)=>{return ()=>fn(x);};return make(y=>y+b,a)();",
        ),
        (
            "returned_callback",
            "const seed=x=>{return y=>x+y;};const make=(fn,x)=>{return ()=>fn(x);};return make(seed(a),b)();",
        ),
        (
            "callback_snapshot",
            "let current=x=>x+a;const make=(fn,x)=>{return ()=>fn(x);};return make(current,(current=x=>x-b,a))()+current(b);",
        ),
        (
            "returned_leaf",
            "const make=x=>{return ()=>x+b;};const wrap=x=>{return make(x);};return wrap(a)();",
        ),
        (
            "conditional_region",
            "const make=x=>{return ()=>x+b;};return (a?make(a)():make(b)())+b;",
        ),
        (
            "callee_before_outer_args",
            "let n=0;const make=x=>{n=n*10+2;return y=>x+y+n;};return make((n=n*10+1))(n=n*10+3)+n;",
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
                "{name}, typescript={typescript}: block factory bypassed the source scheduler"
            );
        }
    }
}

#[test]
fn returned_block_factory_arguments_preserve_first_diagnostics() {
    // Keep both the original reason and the original source span. In several
    // cases the signature query rejects the enclosing call before argument
    // lowering; introducing a continuation must not change that precedence.
    for (body, expected) in [
        (
            "const make=x=>{return ()=>x;};return make()();",
            "unsupported at bytes 62..70: expression is outside the numeric experiment",
        ),
        (
            "const make=x=>{return ()=>x;};return make(...a)();",
            "unsupported at bytes 62..74: expression is outside the numeric experiment",
        ),
        (
            "const make=x=>{return ()=>x;};return make?.(a)();",
            "unsupported at bytes 62..73: expression is outside the numeric experiment",
        ),
        (
            "const make=x=>{if(x)return ()=>x;};return make(a)();",
            "unsupported at bytes 36..59: block arrows must return a function on every structural path",
        ),
        (
            "const make=(x,y)=>{return ()=>x+y;};return make(void 0,...a)();",
            "unsupported at bytes 68..87: expression is outside the numeric experiment",
        ),
        (
            "const make=(fn,x)=>{return ()=>fn(x);};return make(a,b)();",
            "unsupported at bytes 37..39: callback argument requires a function with the inferred signature",
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
fn returned_block_factory_callback_nesting_keeps_its_existing_boundary() {
    // These are observed whole-source boundaries. The signature query and
    // active function-value admission remain separate policies.
    for depth in [1, 6, 7, 8, 9] {
        let mut value = String::from("()=>a");
        for _ in 0..depth {
            value = format!("id({value})");
        }
        let body = format!("const id=fn=>{{return fn;}};return {value}();");
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
                    format!(
                        "unsupported at bytes 58..{}: expression is outside the numeric experiment",
                        65 + 4 * depth
                    ),
                    "{depth}, typescript={typescript}"
                );
            }
        }
    }
}

#[test]
fn returned_block_factory_forwarding_keeps_the_active_body_limit() {
    for depth in [5, 6, 7, 8] {
        let mut body = String::from("const get0=x=>{return ()=>x;};");
        for index in 1..=depth {
            body.push_str(&format!(
                "const get{index}=x=>{{return get{}(x);}};",
                index - 1
            ));
        }
        body.push_str(&format!("return get{depth}(a)();"));
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, typescript);
            if depth < 7 {
                assert!(
                    result.is_ok(),
                    "{depth}, typescript={typescript}: {result:?}"
                );
                assert!(
                    bridged > 0,
                    "forwarded block factories bypassed the scheduler"
                );
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    "unsupported at bytes 47..52: at most 8 active runtime-closure lowering frames are admitted",
                    "{depth}, typescript={typescript}"
                );
            }
        }
    }
}
