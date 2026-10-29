use super::*;

fn compare_shared(sources: &[&str]) {
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
fn shared_promise_inputs_keep_the_saved_callee_before_argument_assignment() {
    compare_shared(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>Promise.reject(a);let h=f;try{await f(p);}catch(e){if(e!==p)return false;}try{await h(b,h=g);}catch(e){return e===b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>Promise.reject(a);let h=f;try{await f(p);}catch(e){if(e!==p)return false;}try{await h((h=g,b));}catch(e){return e===b;}}",
    ]);
}

#[test]
fn shared_promise_inputs_allow_standalone_compatible_function_reassignment() {
    compare_shared(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>Promise.reject(a);let h=f;try{await f(p);}catch(e){if(e!==p)return false;}h=g;try{await h(b);}catch(e){return e===a;}}",
    ]);
}

#[test]
fn shared_promise_inputs_close_the_original_numeric_catch_gap() {
    compare_shared(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>Promise.reject(a);let h=f;try{await f(p);}catch(e){if(e!==p)return false;}try{await h(b,h=g);}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>Promise.reject(a);let h=f;try{await f(p);}catch(e){if(e!==p)return false;}try{await h((h=g,b));}catch(e){return e+b;}}",
    ]);
}

#[test]
fn shared_promise_inputs_preserve_already_matching_input_profiles() {
    compare_shared(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>Promise.reject(a);let h=f;try{await f(p);}catch(e){if(e!==p)return false;}try{await g(p);}catch(e){}try{await h(b,h=g);}catch(e){return e===b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>Promise.reject(a);let h=f;try{await f(p);}catch(e){if(e!==p)return false;}h=g;try{await h(p);}catch(e){return e===a;}}",
    ]);
}

#[test]
fn shared_promise_inputs_cover_inline_choices_and_immediate_factories() {
    compare_shared(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>Promise.reject(a);try{await f(p);}catch(e){if(e!==p)return false;}try{await (a?f:g)(b);}catch(e){return a?e===b:e===a;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>Promise.reject(a),make=()=>a?f:g;try{await f(p);}catch(e){if(e!==p)return false;}try{await make()(b);}catch(e){return a?e===b:e===a;}}",
    ]);
}

#[test]
fn shared_promise_inputs_validate_unused_returned_constructor_families() {
    compare_shared(&[
        "export async function run(a,b){const p=Promise.resolve(a),make=()=>{const f=x=>Promise.resolve(x),g=x=>Promise.resolve(a);f(p);return a?f:g;};return b;}",
    ]);
}

#[test]
fn shared_promise_inputs_preserve_primitive_payloads_and_mixed_results() {
    for value in ["undefined", "null", "false", "true", "b"] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>Promise.reject(a);let h=f;try{{await f(p);}}catch(e){{if(e!==p)return false;}}try{{await h({value},h=g);}}catch(e){{return e;}}}}"
        );
        compare_shared(&[&source]);
    }
    compare_shared(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>x;let h=f;try{await f(p);}catch(e){if(e!==p)return false;}try{await h(b,h=g);}catch(e){if(e!==b)return false;}return (await h(b))===b;}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>Promise.reject(Math.abs(x));let h=f;try{await f(p);}catch(e){if(e!==p)return false;}try{await h(b,h=g);}catch(e){if(e!==b)return false;}try{await h(b);}catch(e){return e;}}",
    ]);
}

#[test]
fn shared_promise_inputs_preserve_async_prefixes_and_callback_calls() {
    compare_shared(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=async x=>{await a;throw x;},g=async x=>{await b;throw a;};let h=f;try{await f(p);}catch(e){if(e!==p)return false;}try{await h(b,h=g);}catch(e){return e===b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let n=0;const f=async x=>{n++;await a;throw x;},g=async x=>(n+=2,a);let h=f;try{await f(p);}catch(e){if(e!==p)return false;}try{await h(b,h=g);}catch(e){if(e!==b)return false;}const q=h(b),saved=n;return saved===4&&(await q)===a;}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>Promise.reject(a),apply=(fn,x)=>fn(x);try{await f(p);}catch(e){if(e!==p)return false;}try{await apply(a?f:g,b);}catch(e){return a?e===b:e===a;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>Promise.reject(a);let h=f;try{await f(p);}catch(e){if(e!==p)return false;}try{await h(b,await Promise.reject(p),h=g);}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>Promise.reject(a);let h=f;const saved=h;try{await f(p);}catch(e){if(e!==p)return false;}h=g;try{await saved(b);}catch(e){return e===b;}}",
    ]);
}

#[test]
fn shared_promise_inputs_join_each_position_and_keep_width_limits() {
    for count in [1000, 1001] {
        let left = (0..count)
            .map(|i| format!("x{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let right = (0..count)
            .map(|i| format!("y{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let mut first = vec!["b"; count];
        first[count - 1] = "p";
        let mut later = vec!["b"; count];
        later[count - 1] = "(h=g,b)";
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a),f=({left})=>Promise.reject(x{}),g=({right})=>Promise.reject(a);let h=f;try{{await f({});}}catch(e){{if(e!==p)return false;}}try{{await h({});}}catch(e){{return e===b;}}}}",
            count - 1,
            first.join(","),
            later.join(",")
        );
        if count == 1000 {
            compare_shared(&[&source]);
        } else {
            assert!(compile_async(&source, false).is_err());
        }
    }
}

#[test]
fn shared_promise_inputs_keep_callback_arity_and_result_domains_separate() {
    for source in [
        "export async function run(a,b){const p=Promise.resolve(a),f=(x,y)=>Promise.reject(x),g=x=>Promise.reject(a);let h=a?f:g;return await h(b,b);}",
        "export async function run(a,b){const p=Promise.resolve(a),f=fn=>fn(a),g=x=>Promise.reject(a);let h=a?f:g;return await h(p);}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>()=>x,g=x=>Promise.reject(a);const saved=f(p);let h=a?f:g;return await h(b);}",
    ] {
        assert!(compile_async(source, false).is_err(), "{source}");
    }
}

#[test]
fn shared_promise_inputs_erase_types_and_keep_await_admission() {
    let js = "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>Promise.reject(a);let h=f;try{await f(p);}catch(e){if(e!==p)return false;}try{await h(b,h=g);}catch(e){return e===b;}}";
    let ts = "export async function run(a:number,b:number):Promise<boolean>{const p=Promise.resolve(a),f=(x:unknown):Promise<unknown>=>Promise.reject(x),g=(x:never):Promise<number>=>Promise.reject(a);let h=f;try{await f<Missing>(p);}catch(e:unknown){if(e!==p)return false;}try{await h<Missing>(b,h=g);}catch(e:unknown){return e===b;}}";
    assert_eq!(
        compile_async(js, false).unwrap().wasm,
        compile_async(ts, true).unwrap().wasm
    );
    for count in [30, 31] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>Promise.reject(a);let h=f;try{{await f(p);}}catch(e){{if(e!==p)return false;}}{}try{{await h(b,h=g);}}catch(e){{return e===b;}}}}",
            "await a;".repeat(count)
        );
        if count == 30 {
            compare_shared(&[&source]);
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

#[test]
fn shared_promise_inputs_join_overlapping_families_and_keep_callback_positions() {
    compare_shared(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>Promise.reject(a),k=x=>Promise.reject(b);let left=f,right=g;try{await f(p);}catch(e){if(e!==p)return false;}left=g;right=k;try{await left(b);}catch(e){if(e!==a&&!(e!==e&&a!==a))return false;}try{await right(b);}catch(e){return e===b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),cb=x=>x+b,f=(x,fn)=>Promise.reject(x),g=(x,fn)=>Promise.reject(fn(a));let h=f;try{await f(p,cb);}catch(e){if(e!==p)return false;}try{await h(b,cb,h=g);}catch(e){if(e!==b)return false;}try{await h(b,cb);}catch(e){return e===cb(a);}}",
    ]);
}
