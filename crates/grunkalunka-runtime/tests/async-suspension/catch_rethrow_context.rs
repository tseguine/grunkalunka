use super::*;

fn compare_rethrows(sources: &[&str]) {
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
fn catch_rethrow_context_retains_each_call_argument_domain() {
    compare_rethrows(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=async x=>{try{throw x;}catch(e){throw e;}};try{await f(p);}catch(e){if(e!==p)return false;}try{await f(b);}catch(e){return e+b;}}",
    ]);
}

#[test]
fn catch_rethrow_context_follows_awaited_sources_and_forwarding() {
    for body in [
        "try{await Promise.reject(x);}catch(e){throw e;}",
        "try{await Promise.resolve(Promise.reject(x));}catch(e){throw e;}",
        "try{const q=Promise.reject(x);await q;}catch(e){throw e;}",
        "try{await (a?Promise.reject(x):Promise.reject(x));}catch(e){throw e;}",
        "try{await (a&&Promise.reject(x));throw x;}catch(e){throw e;}",
        "try{await (b,Promise.reject(x));}catch(e){throw e;}",
        "try{throw x;}catch(e){const alias=e;await a;throw alias;}",
        "try{try{throw x;}catch(inner){throw inner;}}catch(e){throw e;}",
        "try{if(a)throw x;await Promise.reject(x);}catch(e){return Promise.reject(e);}",
        "try{throw x;}catch(e){const forward=y=>Promise.reject(y);return forward(e);}",
        "try{throw x;}catch(e){try{await b;}finally{await a;}throw e;}",
    ] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a),f=async x=>{{{body}}};try{{await f(p);}}catch(e){{if(e!==p)return false;}}try{{await f(b);}}catch(e){{return e+b;}}}}"
        );
        compare_rethrows(&[&source]);
    }
}

#[test]
fn catch_rethrow_context_does_not_narrow_raw_or_written_sources() {
    compare_rethrows(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=async x=>{try{if(a)throw p;throw x;}catch(e){throw e;}};try{await f(b);}catch(e){return a?e===p:e===b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async x=>{try{throw x;}catch(e){e=p;throw e;}};try{await f(b);}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async x=>{try{await Promise.reject(p);throw x;}catch(e){throw e;}};try{await f(b);}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),reject=()=>Promise.reject(p),f=async x=>{try{await reject();throw x;}catch(e){throw e;}};try{await f(b);}catch(e){return e===p;}}",
    ]);
}

#[test]
fn catch_rethrow_context_keeps_lexical_destinations_and_nested_bodies_separate() {
    compare_rethrows(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=async x=>{try{try{throw p;}catch(inner){}throw x;}catch(e){throw e;}};try{await f(p);}catch(e){if(e!==p)return false;}try{await f(b);}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async x=>{try{const dormant=async()=>{throw p;};throw x;}catch(e){throw e;}};try{await f(p);}catch(e){if(e!==p)return false;}try{await f(b);}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async e=>{try{throw e;}catch(e){const g=async()=>{throw e;};return g();}};try{await f(p);}catch(e){if(e!==p)return false;}try{await f(b);}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async x=>{try{try{throw x;}finally{await a;}}catch(e){throw e;}};try{await f(p);}catch(e){if(e!==p)return false;}try{await f(b);}catch(e){return e+b;}}",
    ]);
}

#[test]
fn catch_rethrow_context_preserves_raw_capture_activations_and_primitive_tags() {
    compare_rethrows(&[
        "export async function run(a,b){const p=Promise.resolve(a);let first=true,saved=()=>Promise.reject(b);const f=async x=>{try{throw x;}catch(e){const g=()=>Promise.reject(e);if(first){first=false;saved=g;return b;}return saved();}};await f(p);try{await f(b);}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async x=>{try{throw x;}catch(e){const set=()=>{e=p;};set();throw e;}};try{await f(b);}catch(e){return e===p;}}",
    ]);
    for value in ["undefined", "null", "false", "true", "b"] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a),f=async x=>{{try{{throw x;}}catch(e){{throw e;}}}};try{{await f(p);}}catch(e){{if(e!==p)return false;}}try{{await f({value});}}catch(e){{if(!(e==={value}||(e!==e&&{value}!=={value})))return false;return e+b;}}}}"
        );
        compare_rethrows(&[&source]);
    }
}

#[test]
fn catch_rethrow_context_preserves_erased_types_and_existing_admission_guards() {
    let js = "export async function run(a,b){const p=Promise.resolve(a),f=async x=>{try{throw x;}catch(e){throw e;}};try{await f(p);}catch(e){if(e!==p)return false;}try{await f(b);}catch(e){return e+b;}}";
    let ts = "export async function run(a:number,b:number):Promise<number>{const p=Promise.resolve(a),f=async (x:unknown):Promise<never>=>{try{throw x;}catch(e:unknown){throw e;}};try{await f<Missing>(p);}catch(e:unknown){if(e!==p)return false;}try{await f<Missing>(b);}catch(e:unknown){return e+b;}}";
    assert_eq!(
        compile_async(js, false).unwrap().wasm,
        compile_async(ts, true).unwrap().wasm
    );
    for source in [
        "export async function run(a,b){const p=Promise.resolve(a),f=async x=>{const fail=()=>{throw p;};try{if(a)fail();throw x;}catch(e){throw e;}};try{await f(p);}catch(e){}try{await f(b);}catch(e){return a?e===p:e===b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async x=>{try{throw x;}catch({reason}){throw reason;}};return await f(b);}",
    ] {
        assert!(compile_async(source, false).is_err(), "{source}");
    }
    for count in [30, 31] {
        let source = js.replace(
            "try{await f(b);}",
            &format!("{}try{{await f(b);}}", "await a;".repeat(count)),
        );
        // The two root calls and added awaits share the original 32-site bound.
        if count == 30 {
            compare_rethrows(&[&source]);
        } else {
            assert!(
                compile_async(&source, false)
                    .err()
                    .unwrap()
                    .contains("32 await sites")
            );
        }
    }
    for count in [1000, 1001] {
        let source = js.replace("f(b)", &format!("f({})", vec!["b"; count].join(",")));
        if count == 1000 {
            compare_rethrows(&[&source]);
        } else {
            assert!(compile_async(&source, false).is_err());
        }
    }
}

#[test]
fn catch_rethrow_context_uses_the_existing_small_native_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut body = "throw x;".to_string();
            for index in 0..8 {
                body = format!("try{{{body}}}catch(e{index}){{throw e{index};}}");
            }
            let source = format!("export async function run(a,b){{const p=Promise.resolve(a),f=async x=>{{{body}}};try{{await f(p);}}catch(e){{if(e!==p)return false;}}try{{await f(b);}}catch(e){{return e+b;}}}}");
            compile_async(&source, false).unwrap();
            source
        })
        .unwrap()
        .join()
        .map(|source| compare_rethrows(&[&source]))
        .unwrap();
}
