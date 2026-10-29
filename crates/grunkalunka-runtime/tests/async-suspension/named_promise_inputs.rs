use super::*;

fn compare_named_inputs(programs: &[&str]) {
    compare_async_sources(
        &programs
            .iter()
            .map(|source| (*source).to_string())
            .collect::<Vec<_>>(),
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-2.0, -4.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn named_async_helpers_receive_promise_inputs() {
    compare_named_inputs(&[
        "async function f(x){return x;}export async function run(a,b){return await f(Promise.resolve(a));}",
        "async function f(p){return await p;}export async function run(a,b){return await f(Promise.resolve(a));}",
        "async function f(p){try{return await p;}catch(e){return e;}}export async function run(a,b){return await f(Promise.reject(a));}",
        "async function f(p){return Promise.resolve(p)===p;}export async function run(a,b){const p=Promise.resolve(a);return await f(p);}",
        "async function f(p,q){return p===q;}export async function run(a,b){const p=Promise.resolve(a);return await f(p,p);}",
        "async function f(p,q){return p===q;}export async function run(a,b){return await f(Promise.resolve(a),Promise.resolve(a));}",
        "async function g(x){return x;}async function f(p){return await p;}export async function run(a,b){return await f(g(a));}",
    ]);
}

#[test]
fn named_async_promise_inputs_preserve_mixed_values_and_cells() {
    compare_named_inputs(&[
        "async function f(p){return await p;}export async function run(a,b){return await f(a?Promise.resolve(b):b);}",
        "async function f(p){return p;}export async function run(a,b){const p=f(Promise.resolve(a)),q=f(b);return(+(await p))+(+(await q));}",
        "async function f(p){return +p;}export async function run(a,b){return await f(a?Promise.resolve(b):b);}",
        "async function f(p){const old=p;p=0;await old;return old===p;}export async function run(a,b){return await f(Promise.resolve(a));}",
        "async function f(p){const read=()=>p;const old=p;p=Promise.resolve(7);await p;return old===read();}export async function run(a,b){return await f(Promise.resolve(a));}",
        "async function f(n,p,m){const read=()=>p;await n;return(await read())+m;}export async function run(a,b){return await f(a,Promise.resolve(a),b);}",
    ]);
}

#[test]
fn named_async_promise_inputs_follow_aliases_and_forwarding() {
    compare_named_inputs(&[
        "async function f(p){return await p;}export async function run(a,b){const h=f;return await h(Promise.resolve(a));}",
        "async function f(p){return await p;}async function g(p){return await p;}export async function run(a,b){const h=a?f:g;return await h(Promise.resolve(b));}",
        "async function f(p){return await p;}export async function run(a,b){const apply=(fn,p)=>fn(p);return await apply(f,Promise.resolve(a));}",
        "async function f(p){return await p;}export async function run(a,b){const id=fn=>fn;return await id(f)(Promise.resolve(a));}",
        "async function f(p){return await p;}export async function run(a,b){const get=()=>f;return await get()(Promise.resolve(a));}",
        "async function f(p){return await p;}async function g(x){return x;}async function h(x){return x;}export async function run(a,b){return await f((a?g:h)(b));}",
        "async function f(p){return await p;}async function g(x){return x;}export async function run(a,b){const get=()=>g;return await f(get()(a));}",
        "async function f(p){return await p;}export async function run(a,b){const g=async p=>p;const h=a?f:g;return await h(Promise.resolve(a));}",
    ]);
}

#[test]
fn named_async_promise_inputs_preserve_argument_snapshots_and_failures() {
    compare_named_inputs(&[
        "async function f(p,n){await n;return p;}export async function run(a,b){let p=Promise.resolve(a);return await f(p,await(p=Promise.resolve(b)));}",
        "async function f(p){return p;}export async function run(a,b){let p=Promise.resolve(a);const old=p;const result=f(p,p=Promise.resolve(b));return(await result)===a&&old!==p;}",
        "async function f(p,q){return p===q;}export async function run(a,b){let p=Promise.resolve(a);return await f(p,(p=Promise.resolve(b)));}",
        "async function f(p,n){return await p;}export async function run(a,b){let n=0;try{return await f(Promise.resolve(a),await Promise.reject(b),n=1);}catch(e){return n;}}",
        "async function f(p){return await p;}export async function run(a,b){let n=0;try{return await f(await Promise.reject(a),n=1);}catch(e){return n;}}",
        "async function f(p){return(await p)+1;}async function g(p){return(await p)+2;}export async function run(a,b){let h=f;return await h((h=g,Promise.resolve(a)));}",
    ]);
}

#[test]
fn named_async_promise_inputs_keep_recursive_activations_and_adoption() {
    compare_named_inputs(&[
        "async function f(n,p){if(n<=0)return p;await n;return await f(n-1,p);}export async function run(a,b){return await f(3,Promise.resolve(a));}",
        "async function f(n,p){if(n<=0)return p;return await g(n-1,p);}async function g(n,p){if(n<=0)return await p;await n;return f(n-1,p);}export async function run(a,b){return await f(4,Promise.resolve(b));}",
        "async function f(p,n){const read=()=>p;await n;p=Promise.resolve(n);return await read();}export async function run(a,b){const p=f(Promise.resolve(a),a),q=f(Promise.resolve(b),b);return(+(await p))+(+(await q));}",
        "async function f(p,n){try{return p;}finally{await n;}}export async function run(a,b){return await f(Promise.resolve(a),b);}",
        "async function f(p,n){try{return await p;}finally{await n;}}export async function run(a,b){try{return await f(Promise.reject(a),b);}catch(e){return e;}}",
        "async function f(p){try{return await p;}catch(e){throw e;}}export async function run(a,b){try{return await f(Promise.reject(a));}catch(e){return e;}}",
        "async function f(p){return p;}export async function run(a,b){return await f(a?Promise.resolve(b):null);}",
        "async function f(p){return await p;}export async function run(a,b){const p=f(Promise.resolve(a)),q=f(false);return(await p)===a&&(await q)===false;}",
    ]);
}

#[test]
fn named_async_promise_inputs_keep_erasure_and_source_domain_guards() {
    let plain = "async function f(p,n){const read=()=>p;await n;return await read();}export async function run(a,b){return await f(Promise.resolve(a),b);}";
    let typed = "async function f<T>(p:never,n:Missing):Wrong{const read=()=>p;await(n as never);return await(read() as Wrong);}export async function run(a:never,b:Wrong):Missing{return await f<Bad>(Promise.resolve<never>(a as any),b as never);}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    compare_named_inputs(&[plain]);
    for source in [
        "async function f(p){return p+1;}export async function run(a,b){return await f(Promise.resolve(a));}",
        "function f(p){return p;}export async function run(a,b){return await f(Promise.resolve(a));}",
        "async function f(p,q){return await p;}export async function run(a,b){return await f(Promise.resolve(a));}",
        "async function f(p){return await p;}export async function run(a,b){return await f?.(Promise.resolve(a));}",
        "async function f(p){return await p;}export async function run(a,b){return await f({then:a});}",
        "async function f(p){return await p;}export async function run(a,b){return await f(missing as Promise<number>);}",
    ] {
        assert!(compile_async(source, true).is_err(), "{source}");
    }
    for count in [17, 32] {
        let params = (0..count)
            .map(|i| format!("p{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let actuals = (0..count)
            .map(|i| if i == 1 { "Promise.resolve(a)" } else { "a" })
            .collect::<Vec<_>>()
            .join(",");
        let source = format!(
            "async function f({params}){{return(await p1)+p0+p{};}}export async function run(a,b){{return await f({actuals});}}",
            count - 1
        );
        compare_named_inputs(&[&source]);
    }
}
