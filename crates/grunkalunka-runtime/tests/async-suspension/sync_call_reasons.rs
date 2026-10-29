use super::*;

#[test]
fn sync_call_reasons_preserve_primitive_rejections() {
    compare_async_sources(
        &["export async function run(a,b){const p=Promise.resolve(a),f=()=>Promise.reject(b);try{throw p;}catch(e){}try{await f();}catch(e){return e+b;}}".to_string()],
        &[(4.0,2.0),(0.0,-0.0),(-2.0,-4.0),(f64::NAN,f64::INFINITY)],
    );
}

#[test]
fn sync_call_reasons_keep_non_promise_calls_primitive() {
    for body in ["b;", "return;", "if(a)return b;", "return b;"] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a),f=()=>{{{body}}};try{{throw p;}}catch(e){{}}try{{await f();throw b;}}catch(e){{return e+b;}}}}"
        );
        compare_async_sources(&[source], &[(4.0, 2.0), (0.0, -0.0)]);
    }
}

fn compare_calls(sources: &[&str]) {
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
fn sync_call_reasons_follow_all_complete_return_paths() {
    compare_calls(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=()=>{if(a)return Promise.reject(b);return Promise.reject(a);};try{throw p;}catch(e){}try{await f();}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x),g=x=>Promise.reject(x+a);const h=a?f:g,alias=h;try{throw p;}catch(e){}try{await alias(b);}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),make=x=>()=>Promise.reject(x);const f=make(b);try{throw p;}catch(e){}try{await f();}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=()=>a?Promise.reject(b):a;try{throw p;}catch(e){}try{await f();throw b;}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=()=>Promise.reject(b),g=()=>f();try{throw p;}catch(e){}try{await g();}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=()=>{const q=Promise.reject(b),r=q;return r;};try{throw p;}catch(e){}try{await f();}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=()=>{while(a>0){a--;if(a<b)return Promise.reject(a);}return Promise.reject(b);};try{throw p;}catch(e){}try{await f();}catch(e){return e+b;}}",
    ]);
    for primitive in ["undefined", "null", "false", "true", "b"] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a),f=()=>Promise.reject({primitive});try{{throw p;}}catch(e){{}}try{{await f();}}catch(e){{if(!(e==={primitive}||(e!==e&&{primitive}!=={primitive})))return false;return e+b;}}}}"
        );
        compare_calls(&[&source]);
    }
}

#[test]
fn sync_call_reasons_preserve_raw_reasons_from_every_selected_target() {
    compare_calls(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=()=>Promise.reject(b),g=()=>Promise.reject(p),h=a?f:g;try{await h();}catch(e){return a?e===b:e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x);try{await f(p);}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),make=x=>()=>Promise.reject(x),f=make(p);try{await f();}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=()=>Promise.resolve(Promise.reject(p));try{await f();}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=()=>{if(a)return Promise.reject(p);};try{await f();}catch(e){return e===p;}return b;}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b);const f=()=>q;q=Promise.reject(p);try{await f();}catch(e){return e===p;}}",
        "export async function run(a,b){const f=()=>Promise.reject(b),q=f();try{await Promise.reject(q);}catch(e){try{await q;}catch(x){return e===q&&x===b;}}}",
        "export async function run(a,b){const f=()=>Promise.reject(b),q=f();try{throw q;}catch(e){try{await q;}catch(x){return e===q&&x===b;}}}",
    ]);
    for source in [
        "export async function run(a,b){const p=Promise.resolve(a),f=()=>Promise.reject(b),g=()=>Promise.reject(p),h=a?f:g;try{await h();}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x);try{await f(p);}catch(e){return e+b;}}",
    ] {
        assert!(compile_async(source, false).is_err(), "{source}");
    }
}

#[test]
fn sync_call_reasons_keep_argument_failures_and_cleanup_lexical() {
    compare_calls(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=()=>Promise.reject(b);try{await f(await Promise.reject(p));}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),g=()=>Promise.reject(a);let f=()=>Promise.reject(b);try{throw p;}catch(e){}try{await f(f=g);}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x);try{throw p;}catch(e){}try{await f(b,b=a);}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=()=>Promise.reject(b);try{throw p;}catch(e){}try{try{await f();}finally{await a;}}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=()=>Promise.reject(b);try{throw p;}catch(e){}try{try{await f();}catch(e){await a;throw e;}}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=()=>Promise.reject(b);try{try{await f();}finally{throw p;}}catch(e){return e===p;}}",
    ]);
}

#[test]
fn sync_call_reasons_keep_async_bodies_conservative() {
    compare_calls(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>{await Promise.reject(p);return b;};try{await f();}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>{if(a)throw p;return b;};try{await f();}catch(e){return e===p;}return b;}",
    ]);
    let source = "export async function run(a,b){const p=Promise.resolve(a),f=async()=>{await Promise.reject(p);return b;};try{await f();}catch(e){return e+b;}}";
    assert!(compile_async(source, false).is_err());
}

#[test]
fn sync_call_reasons_keep_erasure_shared_queries_and_source_guards() {
    let plain = "export async function run(a,b){const p=Promise.resolve(a),f=x=>Promise.reject(x);try{throw p;}catch(e){}try{await f(b);}catch(e){return e+b;}}";
    let typed = "export async function run(a:Wrong,b:never):Missing{const p:Wrong=Promise.resolve<Missing>(a as never),f=<T>(x:never):Wrong=>Promise.reject<Missing>(x as Wrong);try{throw(p as never);}catch(e){}try{await(f as any)<Wrong>(b as Missing);}catch(e){return(e as Wrong)+(b as Missing);}}";
    compare_calls(&[plain]);
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    let mut source = String::from(
        "export async function run(a,b){const p=Promise.resolve(a),f0=()=>Promise.reject(b);",
    );
    for i in 1..=6 {
        source.push_str(&format!("const f{i}=()=>a?f{}():f{}();", i - 1, i - 1));
    }
    source.push_str("try{throw p;}catch(e){}try{await f6();}catch(e){return e+b;}}");
    compare_calls(&[&source]);
    for prefix in [31, 32] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a),f=()=>Promise.reject(b);try{{throw p;}}catch(e){{}}{}try{{await f();}}catch(e){{return e+b;}}}}",
            "await a;".repeat(prefix)
        );
        if prefix == 31 {
            compare_calls(&[&source]);
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
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a),f=x=>Promise.reject(x);try{{throw p;}}catch(e){{}}try{{await f({});}}catch(e){{return e+b;}}}}",
            vec!["b"; count].join(",")
        );
        if count == 1000 {
            compare_calls(&[&source]);
        } else {
            assert!(compile_async(&source, false).is_err());
        }
    }
    for body in [
        "const f=()=>{throw b;};return await f();",
        "const f=()=>Promise.reject('x');return await f();",
        "const f=()=>Promise.reject(b);return await f?.();",
        "const f=()=>Promise.reject(b);return await f(...b);",
    ] {
        let source = format!("export async function run(a,b){{{body}}}");
        assert!(compile_async(&source, false).is_err(), "{source}");
    }
}

#[test]
fn sync_call_reasons_preserve_primitive_rejections_with_normal_completion() {
    compare_calls(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=()=>{if(a)return Promise.reject(b);};try{throw p;}catch(e){}try{await f();throw b;}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=()=>{if(a)return;return Promise.reject(b);};try{throw p;}catch(e){}try{await f();throw b;}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=()=>{const g=()=>{return Promise.reject(p);};if(a)return Promise.reject(b);};try{throw p;}catch(e){}try{await f();throw b;}catch(e){return e+b;}}",
    ]);
}
