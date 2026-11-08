#[test]
fn omitted_runtime_closure_arguments_keep_undefined_tags_and_callee_snapshots() {
    for body in [
        "const f=a?(x=>x===undefined?1:0):(x=>x===undefined?2:0);return f();",
        "let f=x=>x===undefined?1:0;return f();",
        "const f=a?((x,y)=>y??x):((x,y)=>y??x+1);return f(b);",
        "const f=a?(x=>+x):(x=>+x+1);return f();",
        "const f=a?(x=>x===undefined?1:0):(x=>x===undefined?2:0);let n=0;const r=f.call((n++,a));return r*10+n;",
        "let n=0;const f=(x,y)=>y===undefined?x:0;return (n++,f)((n++,a))+n;",
        "let f=(x,y)=>y===undefined?x:0;const old=f;const r=f((f=(x,y)=>0,a));return r+old(b);",
        "const o={f:x=>x===undefined?1:0};return o.f();",
        "const fs=[x=>x===undefined?1:0];return fs[0]();",
        "const make=x=>{return y=>y===undefined?x:0;};return (a?make(a):make(b))();",
        "const make=x=>{return y=>y===undefined?x:0;};const f=a?make(a):make(b);return f();",
        "const apply=a?((fn,x)=>fn(x)):((fn,x)=>fn(x)+1);return apply(x=>+x);",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let result = crate::compile(&source, typed);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
}

#[test]
fn omitted_runtime_results_preserve_the_primitive_export_boundary() {
    for body in [
        "let f=x=>x;return f();",
        "const f=a?(x=>x):(x=>x);return f();",
        "const o={f:x=>x};return o.f();",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(
            crate::compile(&source, false).is_err(),
            "numeric export: {source}"
        );
        let result = crate::compile_value(&source, false);
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
}

#[test]
fn omitted_runtime_inputs_keep_async_callback_promise_and_spread_guards() {
    for source in [
        "export async function run(a,b){let f=async x=>x;return await f();}",
        "export async function run(a,b){const f=a?(async x=>x):(async x=>x+1);return await f();}",
        "export async function run(a,b){let f=async x=>x;return f();}",
        "export async function run(a,b){let f=p=>p.then(x=>x);return await f();}",
    ] {
        assert!(crate::compile_async(source, false).is_err(), "{source}");
    }
    for source in [
        "export function run(a,b){let f=(fn,x)=>fn(x);return f();}",
        "export function run(a,b){let f=x=>x;return f(Math.random());}",
    ] {
        assert!(crate::compile(source, false).is_err(), "{source}");
    }
}

#[test]
fn omitted_runtime_formals_keep_complete_signature_and_parameter_limits() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for count in [1000, 1001] {
                let parameters = (0..count)
                    .map(|i| format!("p{i}"))
                    .collect::<Vec<_>>()
                    .join(",");
                let source =
                    format!("export function run(a,b){{let f=({parameters})=>+p0;return f();}}");
                let result = crate::compile(&source, false);
                assert_eq!(result.is_ok(), count == 1000, "{count}: {:?}", result.err());
            }
        })
        .unwrap()
        .join()
        .expect("runtime omitted arguments must keep bounded native stack");
}

// Exact master-CI regressions: unrelated zero-argument calls must not activate
// omitted-input representation inference for other constructors in the module.
#[test]
fn zero_argument_calls_preserve_async_constructor_and_promise_signatures() {
    for source in [
        "export async function run(a,b){const f=x=>()=>x;let p=f;const keep=p(null);const g=x=>()=>x;const h=(a?f:g)(await b);await a;return h();}",
        "export async function run(a,b){const f=x=>()=>x;let p=f;const keep=p(null);const g=x=>()=>x;let make=a?f:g;const h=make(await b);await a;return h();}",
        "export async function run(a,b){const f=async x=>{await x;return x;};const make=p=>async()=>{await a;return await p;};const get=make(f(b));return await get();}",
        "export async function run(a,b){const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();try{await p;}catch(e){const cmp=(x,y)=>x<=y;cmp(p,e);return cmp(e,p);}}",
    ] {
        for typed in [false, true] {
            let result = crate::compile_async(source, typed);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
}
