use super::*;

fn compare_resolve_sources(sources: &[&str]) {
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
fn promise_resolve_mixed_preserves_identity_and_selected_value() {
    compare_resolve_sources(&[
        "export async function run(a,b){const p=a?Promise.resolve(b):b;const q=Promise.resolve(p);return(a?q===p:q!==p)&&((await q)===b);}",
        "export async function run(a,b){let f=async()=>{await a;return b;};return await Promise.resolve(a?f():b,await a);}",
    ]);
}

#[test]
fn promise_resolve_mixed_keeps_existing_identities_and_fresh_primitive_wrappers() {
    compare_resolve_sources(&[
        "export async function run(a,b){const p=a?Promise.resolve(b):b;const q=Promise.resolve(p),r=Promise.resolve(p);return a?q===p&&q===r:q!==p&&q!==r;}",
        "export async function run(a,b){const p=a?Promise.reject(b):b;const q=Promise.resolve(p);try{return(await q)===b&&q!==p;}catch(e){return e===b&&q===p;}}",
        "export async function run(a,b){const p=a?Promise.resolve(b):b;const q=Promise.resolve(p),r=Promise.resolve(q,await a);return q===r&&((await r)===b);}",
        "export async function run(a,b){const p=a?Promise.resolve(b):b;const resolve=x=>Promise.resolve(x);const q=resolve(p);return(a?q===p:q!==p)&&((await q)===b);}",
        "export async function run(a,b){const p=a?Promise.resolve(b):b;let f=a?x=>Promise.resolve(x):x=>Promise.resolve(x);const q=f(p);return(a?q===p:q!==p)&&((await q)===b);}",
        "export async function run(a,b){const p=a?Promise.resolve(b):b;const make=x=>()=>Promise.resolve(x);const f=make(p);const q=f();return(a?q===p:q!==p)&&((await q)===b);}",
    ]);
}

#[test]
fn promise_resolve_mixed_snapshots_before_extra_effects_awaits_and_failure() {
    compare_resolve_sources(&[
        "export async function run(a,b){let p=a?Promise.resolve(b):b;const old=p;const q=Promise.resolve(p,p=a);return(a?q===old:q!==old)&&((await q)===b);}",
        "export async function run(a,b){let p=a?Promise.resolve(b):b;const q=Promise.resolve(p,await a,p=a);return await q;}",
        "export async function run(a,b){let p=a?Promise.resolve(b):b;return await Promise.resolve(p,await a,p=a);}",
        "export async function run(a,b){const f=async()=>{await a;return b;};const p=a?f():b;const q=Promise.resolve(p,b=a);return await q;}",
        "export async function run(a,b){const f=async()=>{await a;return b;};const p=a?f():b;return await Promise.resolve(p,b=a);}",
        "export async function run(a,b){let p=a?Promise.resolve(b):b;const read=()=>p;const q=Promise.resolve(read(),p=a,await b);return await q;}",
        "export async function run(a,b){const p=a?Promise.resolve(b):b;let n=0;try{const q=Promise.resolve(p,await Promise.reject(b),n=1);return await q;}catch(e){return e===b&&n===0;}}",
        "export async function run(a,b){const p=a?Promise.resolve(b):b;let n=0;try{return await Promise.resolve(p,await Promise.reject(b),n=1);}catch(e){return e===b&&n===0;}}",
        "export async function run(a,b){const p=a?Promise.resolve(b):b;let n=0;try{return await Promise.resolve((await Promise.reject(a),p),n=1);}catch(e){return e===a&&n===0;}}",
        "export async function run(a,b){const p=a?Promise.resolve(b):b;try{return Promise.resolve(p,await a);}finally{await b;}}",
    ]);
}

#[test]
fn promise_resolve_mixed_keeps_primitive_tags_and_erased_types() {
    compare_resolve_sources(&[
        "export async function run(a,b){let p;const f=async()=>{await a;return p;};p=f();try{await p;}catch(e){const x=a?Promise.resolve(b):e;const q=Promise.resolve(x);return a?(await q)===b:(await q)===e;}}",
    ]);
    for primitive in ["undefined", "null", "false", "true", "a"] {
        let source = format!(
            "export async function run(a,b){{const p=a?Promise.resolve(b):{primitive};const q=Promise.resolve(p,await b);if(!q)return a;return await q;}}"
        );
        compare_resolve_sources(&[&source]);
    }
    let plain = "export async function run(a,b){const p=a?Promise.resolve(b):b;const q=Promise.resolve(p,await a);return(a?q===p:q!==p)&&((await q)===b);}";
    let typed = "export async function run(a:never,b:Wrong):Missing{const p:NotPromise=(a as never)?Promise.resolve<never>(b as Wrong):(b as Missing);const q:never=Promise.resolve<Wrong>(p as Missing,await(a as Wrong));return((a as Wrong)?q===(p as never):q!==(p as Missing))&&((await q)===(b as never));}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    compare_resolve_sources(&[plain]);
}

#[test]
fn promise_resolve_mixed_keeps_intrinsic_domains_and_source_limits() {
    for source in [
        "export async function run(a,b){const p=Promise.resolve(b);return await Promise.resolve(a?p:'x');}",
        "export async function run(a,b){const p=Promise.resolve(b);return await Promise.resolve(a?p:{});}",
        "export async function run(a,b){const p=Promise.resolve(b);return await Promise.resolve(a?p:()=>a);}",
        "export async function run(a,b){const p=Promise.resolve(b);return await Promise.resolve(a?p:missing as never);}",
        "export async function run(a,b){const p=a?Promise.resolve(b):b;return await Promise.resolve?.(p);}",
        "export async function run(a,b){const p=a?Promise.resolve(b):b;return await Promise.resolve(...p);}",
        "export async function run(a,b){const p=a?Promise.resolve(b):b;return await Promise['resolve'](p);}",
    ] {
        assert!(compile_async(source, true).is_err(), "{source}");
    }
    for extras in [31, 32] {
        let source = format!(
            "export async function run(a,b){{const p=a?Promise.resolve(b):b;const q=Promise.resolve(p{});return await q;}}",
            ",await b".repeat(extras)
        );
        if extras == 31 {
            compare_resolve_sources(&[&source]);
        } else {
            assert!(
                compile_async(&source, false)
                    .err()
                    .expect("33 awaits reject")
                    .contains("32 await sites")
            );
        }
    }
    for actuals in [1000, 1001] {
        let source = format!(
            "export async function run(a,b){{const p=a?Promise.resolve(b):b;const q=Promise.resolve(p{});return await q;}}",
            ",b".repeat(actuals - 1)
        );
        let direct = format!(
            "export async function run(a,b){{const p=a?Promise.resolve(b):b;return await Promise.resolve(p{});}}",
            ",b".repeat(actuals - 1)
        );
        if actuals == 1000 {
            compare_resolve_sources(&[&source, &direct]);
        } else {
            let error = compile_async(&source, true)
                .err()
                .expect("1001 arguments reject");
            // Stored-call kind inference declines an invalid intrinsic before
            // expression lowering. Direct await retains its arity diagnostic.
            assert!(error.contains("outside the numeric experiment"), "{error}");
            assert!(
                compile_async(&direct, true)
                    .err()
                    .expect("1001 await arguments reject")
                    .contains("1000 ordinary")
            );
        }
    }
}
