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
fn catch_reasons_preserve_primitive_rejections_of_conditional_intrinsics() {
    compare_catches(&[
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{await(a?Promise.reject(b):Promise.reject(a));}catch(e){return e+b;}}",
    ]);
    for primitive in ["undefined", "null", "false", "true", "b"] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a);try{{throw p;}}catch(e){{}}try{{await(a?Promise.reject({primitive}):Promise.reject({primitive}));}}catch(e){{if(!(e==={primitive}||(e!==e&&{primitive}!=={primitive})))return a;return e+b;}}}}"
        );
        compare_catches(&[&source]);
    }
}

#[test]
fn catch_reasons_distinguish_resolve_inputs_from_raw_reject_arguments() {
    compare_catches(&[
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{await(a?Promise.resolve(Promise.reject(b)):Promise.reject(a));}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{await Promise.resolve(Promise.resolve(Promise.reject(b)));}catch(e){return e+a;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{await(a?Promise.resolve(Promise.resolve(b)):Promise.resolve(a));throw b;}catch(e){return e+a;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{await(a?Promise.reject(p):Promise.reject(b));}catch(e){return a?e===p:e===b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{await Promise.resolve(a?Promise.reject(p):Promise.reject(b));}catch(e){return a?e===p:e===b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{await Promise.reject(a?p:b);}catch(e){return a?e===p:e===b;}}",
    ]);
    for source in [
        "export async function run(a,b){const p=Promise.resolve(a);try{await(a?Promise.reject(p):Promise.reject(b));}catch(e){return e+b;}}",
        "export async function run(a,b){try{await Promise.reject(a?Promise.resolve(b):b);}catch(e){return e+b;}}",
        "export async function run(a,b){try{await Promise.resolve(a?Promise.reject(Promise.resolve(b)):Promise.reject(b));}catch(e){return e+b;}}",
    ] {
        assert!(compile_async(source, false).is_err(), "{source}");
    }
}

#[test]
fn catch_reasons_follow_stored_intrinsic_rejections() {
    compare_catches(&[
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.reject(b);try{throw p;}catch(e){}try{await q;}catch(e){return e+b;}}",
    ]);
}

#[test]
fn stored_catch_reasons_preserve_aliases_tags_and_constructor_roles() {
    compare_catches(&[
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.reject(b),r=q,s=Promise.resolve(r);try{throw p;}catch(e){}try{await(a?s:r);}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.reject(b);const r=q;try{throw p;}catch(e){}try{await r;}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),q=a?Promise.reject(b):Promise.reject(a);try{throw p;}catch(e){}try{await q;}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.resolve(b);try{throw p;}catch(e){}try{await q;throw a;}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let n=b;const q=Promise.reject(n);n=a;try{throw p;}catch(e){}try{await q;}catch(e){return e+n;}}",
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.reject(p),r=Promise.resolve(q);try{await r;}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.reject(b);try{await Promise.reject(q);}catch(e){try{await q;}catch(x){return e===q&&x===b;}}}",
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.reject(b);try{throw p;}catch(e){}try{await(q,q);}catch(e){return e+b;}}",
    ]);
    for primitive in ["undefined", "null", "false", "true", "b"] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a),q=Promise.reject({primitive}),r=q;try{{throw p;}}catch(e){{}}try{{await r;}}catch(e){{if(!(e==={primitive}||(e!==e&&{primitive}!=={primitive})))return false;return e+b;}}}}"
        );
        compare_catches(&[&source]);
    }
}

#[test]
fn stored_catch_reasons_follow_lexical_symbols_and_suspended_cleanup() {
    compare_catches(&[
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.reject(b);try{throw p;}catch(e){}const f=async()=>{try{await q;}catch(e){return e+b;}};return await f();}",
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.reject(b);{const q=Promise.reject(p);try{await q;}catch(e){if(e!==p)return false;}}try{await q;}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.reject(b);const f=async q=>{try{await q;}catch(e){return e===p;}};const ok=await f(Promise.reject(p));try{await q;}catch(e){return ok?e+b:0;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{const q=Promise.reject(b,await Promise.reject(p));await q;}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.reject(b);try{throw p;}catch(e){}try{try{await q;}finally{await a;}}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.reject(b);try{throw p;}catch(e){}try{try{await q;}catch(e){await a;throw e;}}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.reject(b);try{throw p;}catch(e){}try{try{await q;}finally{throw p;}}catch(e){return e===p;}}",
    ]);
}

#[test]
fn stored_catch_reasons_keep_writes_and_opaque_inputs_raw() {
    compare_catches(&[
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b);q=Promise.reject(p);try{await q;}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b);const change=()=>{q=Promise.reject(p);return b;};change();try{await q;}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b);q&&=Promise.reject(p);try{await q;}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.reject(b);const alias=q;q=Promise.reject(p);try{await alias;}catch(e){try{await q;}catch(x){return e===b&&x===p;}}}",
    ]);
    for source in [
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.reject(p);try{await q;}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b);q=Promise.reject(p);try{await q;}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b);const change=()=>{q=Promise.reject(p);return b;};change();try{await q;}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.reject(b);const f=async q=>{try{await q;}catch(e){return e+b;}};return await f(Promise.reject(p));}",
    ] {
        assert!(compile_async(source, false).is_err(), "{source}");
    }
}

#[test]
fn stored_catch_reasons_bound_shared_alias_queries_and_preserve_erasure() {
    let plain = "export async function run(a,b){const p=Promise.resolve(a),q=Promise.reject(b),r=q;try{throw p;}catch(e){}try{await Promise.resolve(r);}catch(e){return e+b;}}";
    compare_catches(&[plain]);
    let typed = "export async function run(a:Wrong,b:never):Missing{const p:Wrong=Promise.resolve<Missing>(a as never),q:Missing=Promise.reject<Wrong>(b as Missing),r:never=q as Wrong;try{throw(p as never);}catch(e){}try{await Promise.resolve<never>(r as Missing);}catch(e){return(e as Wrong)+(b as Missing);}}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    let merged = "export async function run(a,b){interface q{} const p=Promise.resolve(a),q=Promise.reject(b),r=q;try{throw p;}catch(e){}try{await Promise.resolve(r);}catch(e){return e+b;}}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(merged, true).unwrap().wasm
    );
    let mut source = String::from(
        "export async function run(a,b){const p=Promise.resolve(a),q0=Promise.reject(b);",
    );
    for i in 1..=32 {
        source.push_str(&format!("const q{i}=a?q{}:q{};", i - 1, i - 1));
    }
    source.push_str("try{throw p;}catch(e){}try{await q32;}catch(e){return e+b;}}");
    compare_catches(&[&source]);
    for prefix in [31, 32] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a),q=Promise.reject(b);try{{throw p;}}catch(e){{}}{}try{{await q;}}catch(e){{return e+b;}}}}",
            "await a;".repeat(prefix)
        );
        if prefix == 31 {
            compare_catches(&[&source]);
        } else {
            assert!(
                compile_async(&source, false)
                    .err()
                    .unwrap()
                    .contains("32 await sites")
            );
        }
    }
    for body in [
        "const q=q;return await q;",
        "const q=r,r=q;return await q;",
        "return await q;const q=Promise.reject(b);",
        "const q=Promise.reject('x');try{await q;}catch(e){return e;}",
        "const q=Promise.reject({x:a});try{await q;}catch(e){return e;}",
        "const q=Promise.reject(b);q=Promise.resolve(a);return await q;",
    ] {
        let source = format!("export async function run(a,b){{{body}}}");
        assert!(compile_async(&source, false).is_err(), "{source}");
    }
}

#[test]
fn catch_reasons_follow_logical_intrinsic_value_paths() {
    compare_catches(&[
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{await(a&&Promise.reject(b));throw a;}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{await(a||Promise.reject(b));throw a;}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{await((a?null:b)??Promise.reject(a));throw b;}catch(e){return e+a;}}",
    ]);
}

#[test]
fn catch_reasons_follow_sequence_intrinsic_value_paths() {
    compare_catches(&[
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{await(a,Promise.reject(b));}catch(e){return e+a;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{await(await a,Promise.reject(b));}catch(e){return e+a;}}",
    ]);
}

#[test]
fn catch_reasons_follow_simple_assignment_intrinsic_values() {
    compare_catches(&[
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}let x=a;try{await(x=Promise.reject(b));}catch(e){return e+a;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}let x=Promise.resolve(a);try{await(x=a?Promise.reject(b):Promise.reject(a));}catch(e){return e+b;}}",
    ]);
}

#[test]
fn catch_reasons_keep_composed_argument_failures_and_cleanup_lexical() {
    compare_catches(&[
        "export async function run(a,b){const p=Promise.resolve(a);try{await(a?Promise.reject(b,await Promise.reject(p)):Promise.reject(b));}catch(e){return a?e===p:e===b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{await(a?Promise.reject((await Promise.reject(p),b)):Promise.reject(b));}catch(e){return a?e===p:e===b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{await Promise.resolve((await Promise.reject(p),a?Promise.reject(b):Promise.reject(a)));}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),q=Promise.reject(p);try{await(q,Promise.reject(b));}catch(e){const n=e+b;try{await q;}catch(x){return x===p?n:0;}}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{await(a?Promise.reject(b,b=a):Promise.reject(a,a=b));}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{try{await(a?Promise.resolve(Promise.reject(b)):Promise.reject(a));}finally{await b;}}catch(e){return e+b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{try{await(a?Promise.reject(b):Promise.reject(a));}catch(e){await a;throw e;}}catch(e){return e+b;}}",
    ]);
    let raw_failure = "export async function run(a,b){const p=Promise.resolve(a);try{await(a?Promise.reject(b,await Promise.reject(p)):Promise.reject(b));}catch(e){return e+b;}}";
    assert!(compile_async(raw_failure, false).is_err());
}

#[test]
fn catch_reasons_keep_composed_intrinsic_erasure_and_limits() {
    let plain = "export async function run(a,b){const p=Promise.resolve(a);try{throw p;}catch(e){}try{await(a?Promise.resolve(Promise.reject(b)):Promise.reject(a));}catch(e){return e+b;}}";
    let typed = "export async function run(a:never,b:Wrong):Missing{const p:Wrong=Promise.resolve<Missing>(a as never);try{throw(p as never);}catch(e){}try{await((a as Wrong)?Promise.resolve<never>(Promise.reject<Missing>(b as never)):Promise.reject<Wrong>(a as Missing));}catch(e){return(e as Wrong)+(b as Missing);}}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    for prefix in [31, 32] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a);try{{throw p;}}catch(e){{}}{}try{{await(a?Promise.reject(b):Promise.reject(a));}}catch(e){{return e+b;}}}}",
            "await a;".repeat(prefix)
        );
        if prefix == 31 {
            compare_catches(&[&source]);
        } else {
            assert!(
                compile_async(&source, false)
                    .err()
                    .unwrap()
                    .contains("32 await sites")
            );
        }
    }
    for actuals in [1000, 1001] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a);try{{throw p;}}catch(e){{}}try{{await(a?Promise.reject(b{}):Promise.reject(a));}}catch(e){{return e+b;}}}}",
            ",b".repeat(actuals - 1)
        );
        if actuals == 1000 {
            compare_catches(&[&source]);
        } else {
            assert!(compile_async(&source, false).is_err());
        }
    }
    for call in [
        "Promise.reject?.(b)",
        "Promise.reject(...b)",
        "Promise['reject'](b)",
        "Promise.reject('x')",
        "Promise.reject({then:b})",
    ] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a);try{{throw p;}}catch(e){{}}try{{await(a?{call}:Promise.reject(a));}}catch(e){{return e+b;}}}}"
        );
        assert!(compile_async(&source, true).is_err(), "{source}");
    }
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
