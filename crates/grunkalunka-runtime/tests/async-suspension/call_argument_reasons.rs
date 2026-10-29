use super::*;

#[test]
fn call_argument_reasons_keep_primitive_calls_separate_from_raw_calls() {
    compare_async_sources(
        &["export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x);try{await f(p);}catch(e){if(e!==p)return false;}try{await f(b);}catch(e){return e+b;}}".to_string()],
        &[(4.0,2.0),(0.0,-0.0),(-2.0,-4.0),(f64::NAN,f64::INFINITY)],
    );
}

#[test]
fn call_argument_reasons_follow_async_return_await_and_throw_sources() {
    for body in [
        "return Promise.reject(x);",
        "await a;throw x;",
        "await Promise.reject(x);return b;",
        "const y=x;await a;throw y;",
        "return x;",
    ] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a),f=async x=>{{{body}}};try{{await f(p);}}catch(e){{if(e!==p)return false;}}try{{await f(b);throw b;}}catch(e){{return e+b;}}}}"
        );
        compare_arguments(&[&source]);
    }
    compare_arguments(&[
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.reject(p),f=async x=>{await x;throw b;};try{await f(q);}catch(e){if(e!==p)return false;}try{await f(b);}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async x=>{await a;x=p;throw x;};try{await f(b);}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let first=true,h=async()=>Promise.reject(b);const f=async x=>{const g=async()=>Promise.reject(x);if(first){first=false;h=g;return Promise.reject(b);}return h();};try{await f(p);}catch(e){}try{await f(b);}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>b;try{throw p;}catch(e){}try{await f();throw b;}catch(e){return e+b;}}",
    ]);
}

fn compare_arguments(sources: &[&str]) {
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
fn call_argument_reasons_follow_aliases_forwarding_and_captures() {
    compare_arguments(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>{const y=x;return Promise.reject(y);};try{await f(p);}catch(e){if(e!==p)return false;}try{await f(b);}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=y=>f(y);try{await g(p);}catch(e){if(e!==p)return false;}try{await g(b);}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>{const g=()=>Promise.reject(x);return g();};try{await f(p);}catch(e){if(e!==p)return false;}try{await f(b);}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=(x,y)=>Promise.reject(x);try{await f(p,b);}catch(e){if(e!==p)return false;}try{await f(b,p);}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>x;try{await f(Promise.reject(p));}catch(e){if(e!==p)return false;}try{await f(b);throw b;}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=y=>Promise.reject(y),h=a?f:g;try{await h(p);}catch(e){if(e!==p)return false;}try{await h(b);}catch(e){return e+b;}}",
    ]);
}

#[test]
fn call_argument_reasons_keep_distinct_contexts_at_the_same_depth() {
    compare_arguments(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=y=>f(y),h=()=>a?g(p):g(b);try{await h();}catch(e){return a?e===p:e===b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),g=x=>{const q=Promise.reject(x);return a?q:q;},h=()=>a?g(p):g(b);try{await h();}catch(e){return a?e===p:e===b;}}",
    ]);
}

#[test]
fn call_argument_reasons_do_not_specialize_written_parameters() {
    compare_arguments(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>{x=p;return Promise.reject(x);};try{await f(b);}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>{const set=()=>{x=p;};set();return Promise.reject(x);};try{await f(b);}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x);let q=b;try{await f(q,q=p);}catch(e){if(e!==b)return false;}try{await f(q);}catch(e){return e===p;}}",
    ]);
}

#[test]
fn call_argument_reasons_preserve_captures_from_earlier_activations() {
    compare_arguments(&[
        "export async function run(a,b){const p=Promise.resolve(a);let first=true,h=()=>Promise.reject(b);const f=x=>{const g=()=>Promise.reject(x);if(first){first=false;h=g;return Promise.reject(b);}return h();};try{await f(p);}catch(e){}try{await f(b);}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let first=true,h=()=>Promise.reject(b);const f=x=>{const g=()=>Promise.reject(x);if(first){first=false;h=g;return Promise.reject(b);}const alias=h;return alias();};try{await f(p);}catch(e){}try{await f(b);}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let first=true,h=()=>Promise.reject(b);const f=x=>{const g=()=>Promise.reject(x);if(first){first=false;h=g;return Promise.reject(b);}return (a?h:g)();};try{await f(p);}catch(e){}try{await f(b);}catch(e){return a?e===p:e===b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let first=true,h=()=>Promise.reject(b);const f=x=>{const g=()=>Promise.reject(x);if(first){first=false;h=g;return Promise.reject(b);}const get=()=>h;return get()();};try{await f(p);}catch(e){}try{await f(b);}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let first=true,h=()=>Promise.reject(b);const f=(x,k)=>{const g=()=>Promise.reject(x);if(first){first=false;h=g;return Promise.reject(b);}return k();};try{await f(p,h);}catch(e){}try{await f(b,h);}catch(e){return e===p;}}",
    ]);
}

#[test]
fn call_argument_reasons_preserve_primitive_tags_snapshots_and_argument_failures() {
    compare_arguments(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x);try{await f(p);}catch(e){if(e!==p)return false;}try{await f(b,b=a);}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x);try{await f(p);}catch(e){if(e!==p)return false;}try{await f(b,await Promise.reject(p));}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x);try{await f(p);}catch(e){if(e!==p)return false;}try{try{await f(b);}finally{await a;}}catch(e){return e+b;}}",
    ]);
    for value in ["undefined", "null", "false", "true", "b"] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a),f=x=>Promise.reject(x);try{{await f(p);}}catch(e){{if(e!==p)return false;}}try{{await f({value});}}catch(e){{if(!(e==={value}||(e!==e&&{value}!=={value})))return false;return e+b;}}}}"
        );
        compare_arguments(&[&source]);
    }
}

#[test]
fn call_argument_reasons_keep_the_original_proof_when_contexts_are_exhausted() {
    fn select(choices: &[String]) -> String {
        if choices.len() == 1 {
            return choices[0].clone();
        }
        let middle = choices.len() / 2;
        format!(
            "a?({}):({})",
            select(&choices[..middle]),
            select(&choices[middle..])
        )
    }
    let choices = (0..130)
        .map(|index| format!("x{index}=>Promise.reject(x{index})"))
        .collect::<Vec<_>>();
    let source = format!(
        "export async function run(a,b){{const p=Promise.resolve(a),h={};try{{throw p;}}catch(e){{}}try{{await h(b);}}catch(e){{return e+b;}}}}",
        select(&choices)
    );
    compare_arguments(&[&source]);
}

#[test]
fn call_argument_reasons_erase_types_and_preserve_limits() {
    let js = "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x);try{await f(p);}catch(e){if(e!==p)return false;}try{await f(b);}catch(e){return e+b;}}";
    let ts = "export async function run(a:number,b:number):Promise<number>{const p=Promise.resolve(a),f=(x:unknown):Promise<unknown>=>Promise.reject(x);try{await f<Missing>(p);}catch(e:unknown){if(e!==p)return false;}try{await f<Missing>(b);}catch(e:unknown){return e+b;}}";
    assert_eq!(
        compile_async(js, false).unwrap().wasm,
        compile_async(ts, true).unwrap().wasm
    );
    let async_js = js.replace("f=x=>", "f=async x=>");
    let async_ts = ts.replace("f=(x:unknown)", "f=async (x:unknown)");
    assert_eq!(
        compile_async(&async_js, false).unwrap().wasm,
        compile_async(&async_ts, true).unwrap().wasm
    );
    for count in [1000, 1001] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a),f=x=>Promise.reject(x);try{{await f(p);}}catch(e){{}}try{{await f({});}}catch(e){{return e+b;}}}}",
            vec!["b"; count].join(",")
        );
        if count == 1000 {
            compare_arguments(&[&source]);
        } else {
            assert!(compile_async(&source, false).is_err());
        }
    }
    for count in [30, 31] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a),f=x=>Promise.reject(x);try{{await f(p);}}catch(e){{}}{}try{{await f(b);}}catch(e){{return e+b;}}}}",
            "await a;".repeat(count)
        );
        if count == 30 {
            compare_arguments(&[&source]);
        } else {
            assert!(
                compile_async(&source, false)
                    .err()
                    .unwrap()
                    .contains("32 await sites")
            );
        }
    }
    for call in ["f?.(b)", "f(...b)"] {
        let source = format!(
            "export async function run(a,b){{const f=x=>Promise.reject(x);return await {call};}}"
        );
        assert!(compile_async(&source, false).is_err(), "{source}");
    }
}
