use super::*;

fn compare_catches(sources: &[&str]) {
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
fn catch_reasons_do_not_mix_independent_primitive_handlers() {
    compare_catches(&[
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){if(e!==p)return false;}try{throw b;}catch(e){return e+a;}}",
    ]);
}

#[test]
fn catch_reasons_keep_primitive_await_rejections_local() {
    compare_catches(&[
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{await Promise.reject(b);}catch(e){return e+a;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{await a;throw b;}catch(e){return e+a;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{await Promise.resolve(b);throw a;}catch(e){return e+b;}}",
    ]);
}

#[test]
fn catch_reasons_preserve_later_promise_writes_to_catch_cells() {
    compare_catches(&[
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{throw b;}catch(e){e=p;return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{throw b;}catch(e){const read=()=>e;e=p;await b;return read()===p;}}",
    ]);
}

#[test]
fn catch_reasons_respect_nested_handlers_and_function_boundaries() {
    compare_catches(&[
        "export async function run(a,b){const p=Promise.resolve(a);try{try{throw p;}catch(e){if(e!==p)throw b;}throw a;}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{const f=async()=>{throw p;};throw b;}catch(e){return e+a;}}",
        "async function f(a,b){try{throw b;}catch(e){return e+a;}}export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){if(e!==p)return false;}return await f(a,b);}",
        "export async function run(a,b){const p=Promise.resolve(a);const f=async()=>{throw p;};const x=f();try{throw b;}catch(e){const n=e+a;try{await x;}catch(x){return x===p?n:0;}}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{throw b;}catch(e){const q=e,read=()=>q;await a;return read()+a;}}",
    ]);
}

#[test]
fn catch_reasons_propagate_raw_rethrows_and_finalizer_failures() {
    compare_catches(&[
        "export async function run(a,b){const p=Promise.resolve(a);try{try{throw p;}catch(e){const read=()=>e;await b;throw read();}}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{try{throw b;}catch(e){throw p;}}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{try{throw b;}catch(e){return e+a;}finally{await a;throw p;}}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{try{throw p;}finally{await b;}}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch{await b;}try{throw b;}catch(e){return e+a;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{await Promise.reject(b,await Promise.reject(p));}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{try{await Promise.reject(b);}finally{await a;}}catch(e){return e+a;}}",
    ]);
}

#[test]
fn catch_reasons_keep_erased_types_and_unsupported_reason_guards() {
    let plain = "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{await Promise.reject(b);}catch(e){await a;return e+a;}}";
    let typed = "export async function run(a:never,b:Wrong):Missing{const p:Wrong=Promise.resolve<Missing>(a as never);try{throw(p as never);}catch(e){}try{await Promise.reject<never>(b as Wrong);}catch(e){await(a as never);return(e as Wrong)+(a as never);}}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    compare_catches(&[plain]);
    for source in [
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){return e+a;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{await Promise.reject(p);}catch(e){return e+a;}}",
        "export async function run(a,b){try{throw a;}catch({x}){return x;}}",
        "export async function run(a,b){try{throw missing as never;}catch(e){return e+a;}}",
    ] {
        assert!(compile_async(source, true).is_err(), "{source}");
    }
}
