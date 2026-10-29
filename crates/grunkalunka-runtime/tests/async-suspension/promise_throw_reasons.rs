use super::*;

fn compare_throw_sources(sources: &[&str]) {
    compare_async_sources(
        &sources.iter().map(|s| (*s).to_string()).collect::<Vec<_>>(),
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-2.0, -4.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn promise_throw_reasons_preserve_caught_identity() {
    compare_throw_sources(&[
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.resolve(a);try{throw p;}catch(e){return e!==q;}}",
        "export async function run(a,b){const p=a?Promise.resolve(b):b;try{throw p;}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw Promise.resolve(a);}catch(e){return e!==p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);const f=()=>p;try{throw f();}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.reject(a);try{throw p;}catch(e){try{await e;}catch(x){return x===a;}}}",
        "export async function run(a,b){let p=Promise.resolve(a);const old=p;try{try{throw p;}finally{p=Promise.resolve(b);await b;}}catch(e){return e===old&&e!==p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{try{throw p;}catch(e){await b;throw e;}}catch(e){return e===p;}}",
    ]);
}

#[test]
fn promise_throw_reasons_cross_async_calls_without_adoption() {
    compare_throw_sources(&[
        "async function f(p){throw p;}export async function run(a,b){const p=Promise.resolve(a);try{await f(p);}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);const f=async()=>{throw p;};try{await f();}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);const f=async()=>{await b;throw p;};try{await f();}catch(e){return e===p;}}",
    ]);
}

#[test]
fn promise_throw_reasons_keep_aliases_rethrow_and_finally_precedence() {
    compare_throw_sources(&[
        "export async function run(a,b){const p=Promise.resolve(a);try{try{throw p;}catch(e){const q=e;const read=()=>q;await b;throw read();}}catch(e){return e===p;}}",
        "async function f(p,n){if(n<=0)throw p;await n;return await f(p,n-1);}export async function run(a,b){const p=Promise.resolve(a);try{await f(p,3);}catch(e){return e===p;}}",
        "async function f(p){throw p;}async function g(p){return await f(p);}export async function run(a,b){const p=Promise.resolve(a);try{await g(p);}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{try{throw p;}finally{return b;}}catch(e){return a;}}",
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.resolve(b);try{try{throw p;}finally{await a;throw q;}}catch(e){return e===q&&e!==p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{try{throw p;}finally{await Promise.reject(b);}}catch(e){return e===b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let n=0;try{throw(p,n=1,p);}catch(e){return e===p&&n===1;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{try{throw p;}catch(e){e=b;await a;throw e;}}catch(e){return e===b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw(a?p:null);}catch(e){return a?e===p:e===null;}}",
    ]);
}

#[test]
fn promise_throw_reasons_erase_types_and_keep_remaining_source_guards() {
    let plain = "async function f(p){throw p;}export async function run(a,b){const p=Promise.resolve(a);try{await f(p);}catch(e){await b;return e===p;}}";
    let typed = "async function f<T>(p:never):Wrong{throw(p as Missing);}export async function run(a:never,b:Wrong):Missing{const p:never=Promise.resolve<Wrong>(a);try{await f<Missing>(p);}catch(e){await(b as never);return(e as Wrong)===(p as never);}}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    compare_throw_sources(&[plain]);
    for source in [
        "export async function run(a,b){throw {then:a};}",
        "export async function run(a,b){throw missing as Promise<number>;}",
        "export async function run(a,b){throw Promise.resolve(a)+1;}",
        "export function run(a,b){throw Promise.resolve(a);}",
    ] {
        assert!(compile_async(source, true).is_err(), "{source}");
    }
}

#[test]
fn promise_throw_reasons_adopt_only_on_return_and_keep_primitive_tags() {
    compare_throw_sources(&[
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){return e;}}",
        "export async function run(a,b){const f=async()=>{await a;return b;};const p=f();try{throw p;}catch(e){try{return e;}finally{await a;}}}",
        "export async function run(a,b){const p=Promise.resolve(a);while(a){try{throw p;}finally{break;}}return b;}",
        "export async function run(a,b){const p=Promise.resolve(a);let n=2;while(n>0){n--;try{throw p;}finally{continue;}}return b;}",
    ]);
    for primitive in ["undefined", "null", "false", "true", "b"] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a);try{{throw(a?p:{primitive});}}catch(e){{return a?e===p:e==={primitive};}}}}"
        );
        compare_throw_sources(&[&source]);
    }
}
