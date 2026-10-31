use super::*;

fn compare_method_calls(sources: &[&str]) {
    compare_async_sources(
        &sources
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
fn function_call_keeps_promise_constructor_and_this_domains_separate() {
    for source in [
        "export async function run(a,b){return await Promise.resolve.call(null,b);}",
        "export async function run(a,b){return await Promise.reject.call(undefined,b);}",
        "export async function run(a,b){const f=x=>this;return await f.call(a,b);}",
    ] {
        assert!(compile_async(source, false).is_err(), "{source}");
    }
}

#[test]
fn function_call_preserves_async_and_promise_inputs() {
    compare_method_calls(&[
        "export async function run(a,b){const f=async x=>{await a;return x+b;};return await f.call(null,a);}",
        "async function helper(x){await x;return x+1;}export async function run(a,b){return await helper.call(null,b);}",
        "export async function run(a,b){const f=a?async x=>{await a;return x+b;}:async x=>{await b;return x-b;};return await f.call(null,a);}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x);try{await f.call(null,p);}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x);try{await f.call(null,p);}catch(e){if(e!==p)return false;}try{await f.call(null,b);}catch(e){return e+b;}}",
    ]);
}

#[test]
fn function_call_preserves_primitive_module_helper_results_through_closures() {
    compare_method_calls(&[
        "function helper(x){return null;}export async function run(a,b){const f=async x=>{await a;return helper.call(null,x);};return await f(b);}",
        "function helper(x){return x?null:false;}export async function run(a,b){const f=x=>helper.call(null,x);return f(b);}",
    ]);
}

#[test]
fn function_call_joins_inline_shared_promise_input_interfaces() {
    compare_method_calls(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>Promise.reject(a);try{await f(p);}catch(e){if(e!==p)return false;}try{await(a?f:g).call(null,b);}catch(e){return a?e===b:e===a;}}",
    ]);
}

#[test]
fn function_call_preserves_receiver_awaits_and_abrupt_completion() {
    compare_method_calls(&[
        "export async function run(a,b){let f=async x=>{await a;return x+a;};const g=async x=>{await b;return x-a;};return await f.call((await b,f=g,null),b);}",
        "export async function run(a,b){let n=0;const f=async x=>{n=3;await a;return x;};try{await (n++,f).call(await Promise.reject(b),n=2);}catch(e){return e===b&&n===1;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let n=0;const f=async x=>{n=3;return x;};try{await (n++,f).call(await Promise.reject(p),n=2);}catch(e){return e===p&&n===1;}}",
        "export async function run(a,b){const f=x=>x+b;return f.call(Promise.resolve(a),a);}",
        "export async function run(a,b){const make=a?x=>y=>x+y:x=>y=>x-y;const f=make.call(await b,a);return f(b);}",
        "export async function run(a,b){const use=(fn,x)=>fn.call(null,x);return use(y=>y+b,a);}",
        "export async function run(a,b){const f=x=>x;return f.call(null,false);}",
    ]);
}

#[test]
fn function_call_erases_async_types_and_keeps_await_limits() {
    let js = "export async function run(a,b){const f=async x=>{await a;return x+b;};return await f.call(null,a);}";
    let ts = "export async function run(a:number,b:number):Promise<number>{const f=async(x:never):Promise<Wrong>=>{await a;return x+b;};return await((f as Missing).call)<Wrong>(null,a);}";
    assert_eq!(
        compile_async(js, false).unwrap().wasm,
        compile_async(ts, true).unwrap().wasm
    );
    for count in [30, 31] {
        let source = js.replace(
            "return await",
            &format!("{}return await", "await b;".repeat(count)),
        );
        if count == 30 {
            compare_method_calls(&[&source]);
        } else {
            assert!(
                compile_async(&source, false)
                    .err()
                    .unwrap()
                    .contains("32 await sites")
            );
        }
    }
}
