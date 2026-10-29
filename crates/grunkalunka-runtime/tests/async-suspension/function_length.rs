use super::*;

#[test]
fn function_length_preserves_named_primitive_signature_inputs() {
    compare_async_sources(
        &[
            "async function f(x,y){await y;return x;}export async function run(a,b){const length=fn=>fn.length;const n=length(f);const p=f(null,b);return(await p)===null?n:0;}".into(),
            "async function f(x){return x;}export async function run(a,b){f(false);return f.length;}".into(),
        ],
        &[(4.0,2.0),(0.0,-0.0),(-2.0,-4.0),(f64::NAN,f64::INFINITY)],
    );
}

#[test]
fn function_length_preserves_async_receivers_and_dormant_bodies() {
    let mut sources = Vec::new();
    for body in [
        "const f=async(x,y)=>{await a;return x+y;};return f.length;",
        "let n=0;const f=async()=>{n++;await a;throw b;};return f.length+n;",
        "const f=a?async(x,y)=>{await a;return x;}:async(x,y)=>{await b;return y;};return f.length;",
        "let n=0;const make=x=>{n++;return async(y,z)=>{await x;return y+z;};};return make(await a).length*10+n;",
        "const f=async(x,y)=>{await x;return y;};let n=0;return(await(n=n+1),f).length*10+n;",
        "const f=async(x,y)=>x+y;try{return(await Promise.reject(a),f).length;}catch(e){return e;}",
        "const f=async(x,y)=>x+y;const n=f.length;await b;return n;",
    ] {
        sources.push(source_for(body));
    }
    sources.push("async function f(x,y,z){await x;throw y;}export async function run(a,b){return f.length+run.length;}".into());
    sources.push("async function f(x,y,z){await x;throw y;}export async function run(a,b){const length=fn=>fn.length;return length(f);}".into());
    compare_async_sources(
        &sources,
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-2.0, -4.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn function_length_keeps_async_type_erasure_and_receiver_domains() {
    let plain =
        "export async function run(a,b){const f=async(x,y)=>{await x;return y;};return f.length;}";
    let typed = "export async function run(a:never,b:Missing):Wrong{const f=async<T>(x:never,y:Missing):Wrong=>{await(x as any);return y as never;};return(f as Wrong).length;}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    for body in [
        "return a.length;",
        "return Promise.resolve(a).length;",
        "const f=async()=>a;return f?.length;",
        "const f=async()=>a;return f['length'];",
        "const f=async()=>a;f.length=1;return f.length;",
    ] {
        assert!(compile_async(&source_for(body), false).is_err(), "{body}");
    }
}
