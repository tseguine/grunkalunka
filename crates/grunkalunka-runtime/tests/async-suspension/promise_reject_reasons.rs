use super::*;

fn compare_reject_sources(sources: &[&str]) {
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
fn promise_reject_reasons_preserve_stored_and_direct_identity() {
    compare_reject_sources(&[
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.reject(p);try{await q;}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{await Promise.reject(p);}catch(e){return e===p;}}",
    ]);
}

#[test]
fn promise_reject_reasons_keep_selections_aliases_and_return_adoption() {
    compare_reject_sources(&[
        "export async function run(a,b){const p=Promise.resolve(a);try{return(await Promise.reject(p))+b;}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{if(await Promise.reject(p))return b;}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{await Promise.reject(a?p:b);}catch(e){return a?e===p:e===b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{await Promise.reject(a&&p);}catch(e){return a?e===p:e===a;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{await Promise.reject(a||p);}catch(e){return a?e===a:e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);const make=x=>Promise.reject(x),f=make;try{await f(p);}catch(e){return e===p;}}",
        "async function f(p){return Promise.reject(p);}async function g(p){return await f(p);}export async function run(a,b){const p=Promise.resolve(a);try{await g(p);}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.reject(p);try{await Promise.reject(q);}catch(e){if(e!==q)return false;try{await e;}catch(x){return x===p;}}}",
        "export async function run(a,b){const p=Promise.resolve(a);const f=async()=>{await b;return Promise.reject(p);};try{await f();}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{await Promise.reject(p);}catch(e){return e;}}",
    ]);
}

#[test]
fn promise_reject_reasons_snapshot_before_surplus_effects_and_failures() {
    compare_reject_sources(&[
        "export async function run(a,b){let p=Promise.resolve(a);const old=p;try{await Promise.reject(p,p=Promise.resolve(b));}catch(e){return e===old&&e!==p;}}",
        "export async function run(a,b){let p=Promise.resolve(a);const old=p;const q=Promise.reject(p,p=Promise.resolve(b));try{await q;}catch(e){return e===old&&e!==p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let n=0;try{await Promise.reject((n=1,p),await b,n=2);}catch(e){return e===p&&n===2;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let n=0;try{await Promise.reject(p,await Promise.reject(b),n=1);}catch(e){return e===b&&n===0;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let n=0;try{await Promise.reject((await Promise.reject(b),p),n=1);}catch(e){return e===b&&n===0;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{try{await Promise.reject(p);}finally{await b;}}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.resolve(b);try{try{await Promise.reject(p);}finally{await a;throw q;}}catch(e){return e===q;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{await Promise.reject(p);}catch(e){const q=e;const read=()=>q;await b;try{await Promise.reject(read());}catch(x){return x===p;}}}",
    ]);
}

#[test]
fn promise_reject_reasons_keep_mixed_primitive_tags_and_erased_types() {
    let plain_rejection = "export async function run(a,b){const p=Promise.resolve(a);return await Promise.reject(p);}";
    let typed_rejection = "export async function run(a,b){const p=Promise.resolve(a);return await Promise.reject<never>(p);}";
    assert_eq!(
        compile_async(plain_rejection, false).unwrap().wasm,
        compile_async(typed_rejection, true).unwrap().wasm
    );
    for primitive in ["undefined", "null", "false", "true", "b"] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a);const q=Promise.reject(a?p:{primitive});try{{await q;}}catch(e){{return a?e===p:e==={primitive};}}}}"
        );
        compare_reject_sources(&[&source]);
    }
    let plain = "async function f(p){return Promise.reject(p);}export async function run(a,b){const p=Promise.resolve(a);try{return await f(p);}catch(e){await b;return e===p;}}";
    let typed = "async function f<T>(p:never):Wrong{return Promise.reject<Missing>(p as Wrong);}export async function run(a:never,b:Wrong):Missing{const p:never=Promise.resolve<Wrong>(a as Missing);try{return await f<never>(p as Wrong);}catch(e){await(b as never);return(e as Wrong)===(p as Missing);}}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    compare_reject_sources(&[plain]);
}

#[test]
fn promise_reject_reasons_keep_intrinsic_and_value_domain_guards() {
    for source in [
        "export async function run(a,b){return Promise.reject({then:a});}",
        "export async function run(a,b){return Promise.reject([a]);}",
        "export async function run(a,b){return Promise.reject(()=>a);}",
        "export async function run(a,b){return Promise.reject('x');}",
        "export async function run(a,b){return Promise.reject(missing as Promise<number>);}",
        "export async function run(a,b){const p=Promise.resolve(a);return Promise.reject?.(p);}",
        "export async function run(a,b){const p=Promise.resolve(a);return Promise.reject(...p);}",
        "export async function run(a,b){const p=Promise.resolve(a);return Promise['reject'](p);}",
        "export async function run(a,b){const p=Promise.resolve(a),Promise=a;return Promise.reject(p);}",
    ] {
        assert!(compile_async(source, true).is_err(), "{source}");
    }
}

#[test]
fn promise_reject_reasons_keep_source_await_and_intrinsic_argument_limits() {
    for preceding in [31, 32] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a);{}try{{await Promise.reject(p);}}catch(e){{return e===p;}}}}",
            "await b;".repeat(preceding)
        );
        if preceding == 31 {
            compare_reject_sources(&[&source]);
        } else {
            assert!(
                compile_async(&source, false)
                    .err()
                    .expect("33 source awaits reject")
                    .contains("32 await sites")
            );
        }
    }
    for actuals in [1000, 1001] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a);try{{await Promise.reject(p{});}}catch(e){{return e===p;}}}}",
            ",b".repeat(actuals - 1)
        );
        if actuals == 1000 {
            compare_reject_sources(&[&source]);
        } else {
            assert!(
                compile_async(&source, true)
                    .err()
                    .expect("1001 intrinsic arguments reject")
                    .contains("1000 ordinary")
            );
        }
    }
}
