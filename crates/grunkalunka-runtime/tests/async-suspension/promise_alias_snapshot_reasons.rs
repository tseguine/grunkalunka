use super::*;

fn compare_snapshots(sources: &[&str]) {
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
fn promise_alias_snapshot_reasons_keep_the_original_saved_handle_domain() {
    compare_snapshots(&[
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.reject(b);const alias=q;q=Promise.reject(p);try{await alias;}catch(e){const n=e+b;try{await q;}catch(x){return x===p?n:0;}}}",
    ]);
}

#[test]
fn promise_alias_snapshot_reasons_follow_stable_alias_chains_and_earlier_awaits() {
    compare_snapshots(&[
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.reject(b);const first=q,alias=first;await a;q=Promise.reject(p);try{await alias;}catch(e){const n=e+b;try{await q;}catch(x){return x===p?n:0;}}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.reject(b);try{await q;}catch(e){const n=e+b;q=Promise.reject(p);try{await q;}catch(x){return x===p?n:0;}}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.reject(b);const alias=a?q:q;if(a){q=Promise.reject(p);}try{await alias;}catch(e){const n=e+b;try{await q;}catch(x){return a?x===p?n:0:n;}}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.reject(b);const alias=q;const read=()=>alias;q=Promise.reject(p);try{await read();}catch(e){const n=e+b;try{await q;}catch(x){return x===p?n:0;}}}",
    ]);
}

#[test]
fn promise_alias_snapshot_reasons_do_not_ignore_earlier_writes_or_other_activations() {
    compare_snapshots(&[
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b);q=Promise.reject(p);const alias=q;try{await alias;}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b);const get=()=>q;q=Promise.reject(p);const alias=get();try{await alias;}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b);const set=()=>{q=Promise.reject(p);};set();const alias=q;try{await alias;}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let first=true,saved=()=>Promise.reject(b);const f=x=>{let q=first?Promise.reject(x):Promise.resolve(b);const alias=q,g=()=>alias;if(first){first=false;saved=g;}q=Promise.resolve(a);return first?q:saved();};try{await f(p);}catch(e){}try{await f(b);}catch(e){return e===p;}}",
    ]);
}

#[test]
fn promise_alias_snapshot_reasons_do_not_use_text_order_across_loop_backedges() {
    compare_snapshots(&[
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b),i=0;while(i<2){const alias=q;q=Promise.reject(p);try{await q;}catch(e){if(e!==p)return false;}try{await alias;}catch(e){return e===p;}i++;}return false;}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b);for(let i=0;i<2;i++){const alias=q;q=Promise.reject(p);try{await q;}catch(e){if(e!==p)return false;}try{await alias;}catch(e){return e===p;}}return false;}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b),i=0;do{const alias=q;q=Promise.reject(p);try{await q;}catch(e){if(e!==p)return false;}try{await alias;}catch(e){return e===p;}i++;}while(i<2);return false;}",
    ]);
}

#[test]
fn promise_alias_snapshot_reasons_allow_reads_before_future_loop_writes() {
    for later in [
        "for(let i=0;i<1;i++){q=Promise.reject(p);}",
        "let i=0;while(i<1){q=Promise.reject(p);i++;}",
        "do{q=Promise.reject(p);}while(false);",
    ] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a);let q=Promise.reject(b);const alias=q;{later}try{{await alias;}}catch(e){{const n=e+b;try{{await q;}}catch(x){{return x===p?n:0;}}}}}}"
        );
        compare_snapshots(&[&source]);
    }
}

#[test]
fn promise_alias_snapshot_reasons_keep_raw_initial_reasons_and_primitive_bits() {
    compare_snapshots(&[
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.reject(p);const alias=q;q=Promise.reject(b);try{await alias;}catch(e){if(e!==p)return false;try{await q;}catch(x){return x===b;}}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=b;const alias=q;q=Promise.reject(p);const n=await alias;try{await q;}catch(e){return e===p?n+b:0;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.reject(p);const alias=q;q=b;try{await alias;}catch(e){return e===p;}}",
    ]);
    for value in ["undefined", "null", "false", "true", "b"] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a);let q=Promise.reject({value});const alias=q;q=Promise.reject(p);try{{await alias;}}catch(e){{const n=e+b;try{{await q;}}catch(x){{return x===p?n:0;}}}}}}"
        );
        compare_snapshots(&[&source]);
    }
}

#[test]
fn promise_alias_snapshot_reasons_keep_current_arrow_call_contexts() {
    compare_snapshots(&[
        "export async function run(a,b){const p=Promise.resolve(a),f=async x=>{let q=Promise.reject(x);const alias=q;q=Promise.resolve(a);return await alias;};try{await f(p);}catch(e){if(e!==p)return false;}try{await f(b);}catch(e){return e+b;}}",
    ]);
}

#[test]
fn promise_alias_snapshot_reasons_preserve_erased_types_and_resource_guards() {
    let js = "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.reject(b);const alias=q;q=Promise.reject(p);try{await alias;}catch(e){const n=e+b;try{await q;}catch(x){return x===p?n:0;}}}";
    let ts = "export async function run(a:number,b:number):Promise<number>{const p:Wrong=Promise.resolve<Wrong>(a);let q:Promise<never>=Promise.reject<Wrong>(b);const alias:never=q as Missing;q=Promise.reject<Missing>(p);try{await(alias as Wrong);}catch(e:unknown){const n=e+b;try{await(q as Wrong);}catch(x:unknown){return x===p?n:0;}}}";
    assert_eq!(
        compile_async(js, false).unwrap().wasm,
        compile_async(ts, true).unwrap().wasm
    );
    for count in [30, 31] {
        let source = js.replace(
            "const alias=q;",
            &format!("const alias=q;{}", "await a;".repeat(count)),
        );
        if count == 30 {
            compare_snapshots(&[&source]);
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
        let source = js.replace(
            "Promise.reject(b)",
            &format!("Promise.reject({})", vec!["b"; count].join(",")),
        );
        if count == 1000 {
            compare_snapshots(&[&source]);
        } else {
            assert!(compile_async(&source, false).is_err());
        }
    }
}
