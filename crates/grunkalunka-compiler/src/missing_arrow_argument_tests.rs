#[test]
fn omitted_local_arrow_arguments_preserve_undefined_and_effects() {
    for body in [
        "const f=x=>x===undefined?1:0;return f();",
        "const f=(x,y)=>y??x;return f(a);",
        "const f=x=>+x;return f();",
        "const f=x=>x;return +f();",
        "const f=x=>{x=3;return x;};return +f();",
        "const f=x=>{const g=()=>x===undefined?1:0;return g();};return f();",
        "const f=x=>x===undefined?1:0;return f()*100+f(a)*10+f(NaN);",
        "const f=(x,y)=>y===undefined?x:0;let n=0;const x=f((n++,a));return x+n;",
        "const f=x=>x===undefined?1:0;let n=0;const x=f.call((n++,a));return x*10+n;",
        "const f=x=>x===undefined?1:0;return f.call();",
        "return (x=>x===undefined?1:0)();",
        "const f=(x,y)=>y??x;const x=f('x');return x==='x'?a:b;",
        "const f=(x,y)=>y??x;const x=f({v:a});return +x.v;",
        "const f=x=>()=>x===undefined?1:0;const g=f();return g();",
        "const f=x=>x===undefined?1:0;const alias=f;return alias();",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let result = crate::compile(&source, typed);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
}

#[test]
fn omitted_local_arrow_returns_keep_the_public_value_contract() {
    for body in [
        "const f=x=>x;return f();",
        "return (x=>x)();",
        "const f=(x,y)=>y;return f(a);",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            assert!(
                crate::compile(&source, typed).is_err(),
                "numeric export: {source}"
            );
            let result = crate::compile_value(&source, typed);
            assert!(result.is_ok(), "value export: {source}: {:?}", result.err());
        }
    }
}

#[test]
fn omitted_local_arrow_inputs_erase_optional_annotations() {
    let js = "export function run(a,b){const f=(x,y)=>y??x;return f(a);}";
    let ts = "export function run(a?:number,b?:number){const f=(x?:unknown,y?:unknown)=>y??x;return f(a);}";
    assert_eq!(
        crate::compile(js, false).unwrap().wasm,
        crate::compile(ts, true).unwrap().wasm
    );
}

#[test]
fn omitted_arrow_inputs_keep_async_runtime_callback_and_result_guards() {
    for (source, asynchronous) in [
        (
            "export async function run(a,b){const f=async x=>x;return await f();}",
            true,
        ),
        (
            "export function run(a,b){const f=a?(x=>x):(x=>x+1);return f();}",
            false,
        ),
        (
            "export function run(a,b){const f=(fn,x)=>fn(x);return f();}",
            false,
        ),
        (
            "export function run(a,b){const f=x=>()=>x;const g=f();return g();}",
            false,
        ),
    ] {
        let result = if asynchronous {
            crate::compile_async(source, false)
        } else {
            crate::compile(source, false)
        };
        assert!(result.is_err(), "{source}");
    }
}

#[test]
fn omitted_arrow_formals_keep_the_thousand_parameter_limit_on_small_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for count in [1000, 1001] {
                let parameters = (0..count)
                    .map(|i| format!("p{i}"))
                    .collect::<Vec<_>>()
                    .join(",");
                let source =
                    format!("export function run(a,b){{const f=({parameters})=>+p0;return f();}}");
                let result = crate::compile(&source, false);
                assert_eq!(result.is_ok(), count == 1000, "{count}: {:?}", result.err());
            }
        })
        .unwrap()
        .join()
        .expect("omitted argument synthesis must not overflow native stack");
}
