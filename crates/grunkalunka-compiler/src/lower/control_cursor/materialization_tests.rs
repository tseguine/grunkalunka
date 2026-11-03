//! Real function construction must stay on the owning source cursor and keep
//! nested publication distinct from disposable definition-checking IR.
use super::BRIDGED_LIFTED_BODIES;
use crate::ir::{FunctionId, Op};

fn on_small_stack(body: &str, asynchronous: bool, typescript: bool) -> (Result<(), String>, usize) {
    let prefix = if asynchronous { "async " } else { "" };
    let source = format!("export {prefix}function run(a,b){{{body}}}");
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            BRIDGED_LIFTED_BODIES.with(|count| count.set(0));
            let result = if asynchronous {
                crate::compile_async(&source, typescript).map(|_| ())
            } else {
                crate::compile(&source, typescript).map(|_| ())
            };
            (result, BRIDGED_LIFTED_BODIES.with(std::cell::Cell::get))
        })
        .expect("2 MiB lifted-body thread")
        .join()
        .expect("lifted body construction must not exhaust the native stack")
}

#[test]
fn non_call_function_leaves_schedule_real_bodies() {
    for body in [
        "let f=x=>x+a;return f(b);",
        "let f=x=>{let g=y=>x+y;return g(a);};return f(b);",
        "let f=x=>y=>x+y;return f(a)(b);",
        "let f=x=>x?true:null;return +f(a);",
        "return((fn,x)=>fn(x))(y=>{return y+a;},b);",
    ] {
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(body, false, typescript);
            assert!(result.is_ok(), "{body}: {result:?}");
            assert!(
                bridged > 0,
                "real function body bypassed the source cursor: {body}"
            );
        }
    }
}

#[test]
fn lifted_async_and_promise_bodies_keep_their_source_scheduler() {
    for body in [
        "let f=async x=>{await b;return x+a;};return await f(a);",
        "let f=async x=>x+a;return await f(b);",
        "let f=async x=>{if(x)throw x;return b;};return await f(a);",
        "let f=p=>p;return await f(Promise.resolve(a));",
        "let f=x=>x;return await f(a?Promise.resolve(b):a);",
    ] {
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(body, true, typescript);
            assert!(result.is_ok(), "{body}: {result:?}");
            assert!(
                bridged > 0,
                "async function body bypassed the source cursor: {body}"
            );
        }
    }
}

#[test]
fn materialization_children_keep_the_existing_lifting_limit_and_span() {
    for (depth, admitted) in [(6, true), (7, false)] {
        let mut body = String::from("const get0=x=>{return ()=>x;};");
        for index in 1..=depth {
            body.push_str(&format!(
                "const get{index}=x=>{{return get{}(x);}};",
                index - 1
            ));
        }
        body.push_str(&format!("return get{depth}(a)();"));
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, false, typescript);
            assert_eq!(result.is_ok(), admitted, "depth={depth}: {result:?}");
            if let Err(error) = result {
                assert_eq!(
                    error,
                    "unsupported at bytes 47..52: at most 8 active runtime-closure lowering frames are admitted"
                );
            }
            assert!(bridged > 0, "nested lifting bypassed the source cursor");
        }
    }
}

#[test]
fn nested_materialization_keeps_the_final_capacity_check_and_span() {
    for (seed, selections, admitted) in [(false, 64, true), (true, 63, true), (true, 64, false)] {
        let mut body = String::from("const leaf=()=>{const g=(a,()=>a);return g();};");
        if seed {
            body.push_str("const seed=(a,()=>a);");
        }
        for index in 0..selections {
            body.push_str(&format!("const f{index}=a?leaf:leaf;"));
        }
        body.push_str("return a;");
        for typescript in [false, true] {
            let (result, bridged) = on_small_stack(&body, false, typescript);
            assert_eq!(
                result.is_ok(),
                admitted,
                "seed={seed}, selections={selections}: {result:?}"
            );
            if let Err(error) = result {
                assert_eq!(
                    error,
                    "unsupported at bytes 1486..1490: at most 256 lifted closure bodies are admitted"
                );
            }
            assert!(bridged > 0, "capacity fixture bypassed the source cursor");
        }
    }
}

#[test]
fn real_nested_bodies_publish_before_their_enclosing_function() {
    for source in [
        "export function run(a,b){let f=x=>{let g=y=>x+y;return g(a);};return f(b);}",
        "export function run(a,b){let f=x=>y=>x+y;return f(a)(b);}",
    ] {
        for typescript in [false, true] {
            let module = crate::lower_source(source, typescript).unwrap();
            assert_eq!(
                module.functions.len(),
                3,
                "scratch lifted pools cannot be published"
            );
            for (body, expected) in [(0, FunctionId(2)), (2, FunctionId(1))] {
                let mut constructed = Vec::new();
                crate::ir::visit_operations(&module.functions[body].body, &mut |operation| {
                    if let Op::MakeClosure { function, .. } = operation {
                        constructed.push(*function);
                    }
                });
                assert_eq!(
                    constructed,
                    [expected],
                    "function publication order: {source}"
                );
            }
        }
    }
}
