use super::*;

#[test]
fn async_body_reasons_follow_returned_primitive_rejections() {
    compare_async_sources(
        &["export async function run(a,b){const p=Promise.resolve(a),f=async()=>Promise.reject(b);try{throw p;}catch(e){}try{await f();}catch(e){return e+b;}}".to_string()],
        &[(4.0,2.0),(0.0,-0.0),(-2.0,-4.0),(f64::NAN,f64::INFINITY)],
    );
}

fn compare_bodies(sources: &[&str]) {
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
fn async_body_reasons_keep_own_primitive_exceptions_primitive() {
    compare_bodies(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>{await Promise.reject(b);return a;};try{throw p;}catch(e){}try{await f();}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>{throw b;};try{throw p;}catch(e){}try{await f();}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>{await a;return Promise.reject(b);};try{throw p;}catch(e){}try{await f();}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>{if(a)throw b;await b;return a;};try{throw p;}catch(e){}try{await f();throw b;}catch(e){return e+b;}}",
    ]);
}

#[test]
fn async_body_reasons_exclude_handled_and_nested_sources() {
    compare_bodies(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>{try{await Promise.reject(p);}catch(e){}return b;};try{await f();throw b;}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>{try{throw p;}catch(e){}await a;return b;};try{await f();throw b;}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>{try{return await Promise.reject(p);}catch(e){return b;}};try{await f();throw b;}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>{const g=async()=>{await Promise.reject(p);};await a;return b;};try{await f();throw b;}catch(e){return e+b;}}",
    ]);
}

#[test]
fn async_body_reasons_preserve_escaping_raw_reasons() {
    compare_bodies(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>{throw p;};try{await f();}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>{await Promise.reject(p);return b;};try{await f();}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>{try{await Promise.reject(p);}catch(e){throw e;}};try{await f();}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>{try{await a;}finally{throw p;}};try{await f();}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>{try{return Promise.reject(p);}catch(e){return b;}};try{await f();}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),g=async()=>{throw p;},f=async()=>{await g();return b;};try{await f();}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>{try{throw b;}catch(e){throw p;}finally{await a;}};try{await f();}catch(e){return e===p;}}",
    ]);
    for body in ["throw p;", "await Promise.reject(p);return b;"] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a),f=async()=>{{{body}}};try{{await f();}}catch(e){{return e+b;}}}}"
        );
        assert!(compile_async(&source, false).is_err(), "{source}");
    }
}

#[test]
fn async_body_reasons_keep_callers_snapshots_and_cleanup() {
    compare_bodies(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>Promise.reject(b);try{await f(await Promise.reject(p));}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),g=async()=>Promise.reject(a);let f=async()=>Promise.reject(b);try{throw p;}catch(e){}try{await f(f=g);}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async x=>{await a;throw x;};try{throw p;}catch(e){}try{await f(b,b=a);}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>{await a;throw b;};try{throw p;}catch(e){}try{try{await f();}finally{await a;}}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async()=>{await a;throw b;},g=async()=>Promise.reject(a),h=a?f:g;try{throw p;}catch(e){}try{await h();}catch(e){return e+b;}}",
    ]);
    for value in ["undefined", "null", "false", "true", "b"] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a),f=async()=>{{await a;throw {value};}};try{{throw p;}}catch(e){{}}try{{await f();}}catch(e){{if(!(e==={value}||(e!==e&&{value}!=={value})))return false;return e+b;}}}}"
        );
        compare_bodies(&[&source]);
    }
}

#[test]
fn async_body_reasons_erase_types_and_preserve_limits() {
    let js = "export async function run(a,b){const p=Promise.resolve(a),f=async x=>{await a;throw x;};try{throw p;}catch(e){}try{await f(b);}catch(e){return e+b;}}";
    let ts = "export async function run(a: number,b: number): Promise<number>{const p=Promise.resolve(a),f=async (x: number): Promise<number>=>{await a;throw x;};try{throw p;}catch(e: unknown){}try{await f(b);}catch(e: unknown){return e+b;}}";
    assert_eq!(
        compile_async(js, false).unwrap().wasm,
        compile_async(ts, true).unwrap().wasm
    );
    compare_bodies(&[js]);
    for prefix in [30, 31] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a),f=async()=>{{await a;throw b;}};try{{throw p;}}catch(e){{}}{}try{{await f();}}catch(e){{return e+b;}}}}",
            "await a;".repeat(prefix)
        );
        if prefix == 30 {
            compare_bodies(&[&source]);
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
            "export async function run(a,b){{const p=Promise.resolve(a),f=async x=>{{await a;throw x;}};try{{throw p;}}catch(e){{}}try{{await f({});}}catch(e){{return e+b;}}}}",
            vec!["b"; count].join(",")
        );
        if count == 1000 {
            compare_bodies(&[&source]);
        } else {
            assert!(compile_async(&source, false).is_err());
        }
    }
    for call in ["f?.(b)", "f(...b)"] {
        let source = format!(
            "export async function run(a,b){{const f=async x=>{{await a;throw x;}};return await {call};}}"
        );
        assert!(compile_async(&source, false).is_err(), "{source}");
    }
}
