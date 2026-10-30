use super::*;

fn compare_snapshots(sources: &[&str]) {
    compare_async_sources(
        &sources
            .iter()
            .map(|source| (*source).to_owned())
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
fn assigned_promise_snapshots_keep_the_last_completed_plain_write() {
    compare_snapshots(&[
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(a);q=Promise.reject(b);const alias=q;q=Promise.reject(p);try{await alias;}catch(e){const n=e+b;try{await q;}catch(x){return x===p?n:0;}}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q;q=Promise.reject(b);const alias=q;q=Promise.reject(p);try{await alias;}catch(e){const n=e+b;try{await q;}catch(x){return x===p?n:0;}}}",
    ]);
}

#[test]
fn assigned_promise_snapshots_follow_overwrites_and_containing_blocks() {
    compare_snapshots(&[
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(a);q=Promise.reject(p);try{await q;}catch{}q=Promise.reject(b);const alias=q;q=Promise.reject(p);try{await alias;}catch(e){const n=e+b;try{await q;}catch(x){return x===p?n:0;}}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(a);if(a)q=Promise.reject(p);try{await q;}catch{}q=Promise.reject(b);{const first=q,alias=first;q=Promise.reject(p);try{await alias;}catch(e){const n=e+b;try{await q;}catch(x){return x===p?n:0;}}}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(a);q=Promise.reject(b);try{await q;}catch{}if(a){const alias=q;q=Promise.reject(p);try{await alias;}catch(e){const n=e+b;try{await q;}catch(x){return x===p?n:0;}}}else{try{await q;}catch(e){return e;}}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(a);q=Promise.reject(b);try{await q;}catch(e){const n=e+b;q=Promise.reject(p);try{await q;}catch(x){return x===p?n:0;}}}",
    ]);
}

#[test]
fn assigned_promise_snapshots_preserve_raw_reasons_and_primitive_payloads() {
    compare_snapshots(&[
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(a);q=Promise.reject(p);const alias=q;q=Promise.reject(b);try{await alias;}catch(e){if(e!==p)return false;try{await q;}catch(x){return x===b;}}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(a);q=b;const alias=q;q=Promise.reject(p);const n=await alias;try{await q;}catch(e){return e===p?n+b:0;}}",
    ]);
    for value in ["undefined", "null", "false", "true", "b"] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a);let q=Promise.resolve(a);q=Promise.reject({value});const alias=q;q=Promise.reject(p);try{{await alias;}}catch(e){{const n=e+b;try{{await q;}}catch(x){{return x===p?n:0;}}}}}}"
        );
        compare_snapshots(&[&source]);
    }
}

#[test]
fn assigned_promise_snapshots_retain_conditional_and_nested_write_uncertainty() {
    assert!(compile_async("export async function run(a,b){let q=Promise.resolve(b);a&&(q=Promise.resolve(a));return await q;}",false).is_err());
    compare_snapshots(&[
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b);q=Promise.reject(b);try{await q;}catch{}if(a)q=Promise.reject(p);const alias=q;q=Promise.resolve(b);try{await alias;}catch(e){return e;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.reject(p);try{await q;}catch{}if(a)q=Promise.reject(b);const alias=q;q=Promise.resolve(b);try{await alias;}catch(e){return e;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.reject(p);try{await q;}catch{}if(a){q=Promise.reject(b);}const alias=q;q=Promise.resolve(b);try{await alias;}catch(e){return e;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b);q=Promise.reject(p);q=q;const alias=q;q=Promise.resolve(b);try{await alias;}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b);q=(q=Promise.reject(p),q);const alias=q;q=Promise.resolve(b);try{await alias;}catch(e){return e===p;}}",
    ]);
}

#[test]
fn assigned_promise_snapshots_distinguish_loop_writes_and_loop_reads() {
    for loop_write in [
        "for(let i=0;i<1;i++){q=Promise.reject(p);}",
        "let i=0;while(i<1){q=Promise.reject(p);i++;}",
        "do{q=Promise.reject(p);}while(false);",
    ] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a);let q=Promise.resolve(b);q=Promise.reject(b);const alias=q;{loop_write}try{{await alias;}}catch(e){{const n=e+b;try{{await q;}}catch(x){{return x===p?n:0;}}}}}}"
        );
        compare_snapshots(&[&source]);
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a);let q=Promise.resolve(b);{loop_write}try{{await q;}}catch{{}}q=Promise.reject(b);const alias=q;q=Promise.reject(p);try{{await alias;}}catch(e){{const n=e+b;try{{await q;}}catch(x){{return x===p?n:0;}}}}}}"
        );
        compare_snapshots(&[&source]);
    }
    compare_snapshots(&[
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b);q=Promise.reject(b);for(let i=0;i<2;i++){const alias=q;q=Promise.reject(p);try{await q;}catch(e){if(e!==p)return false;}try{await alias;}catch(e){if(i)return e===p;} }return false;}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b);q=Promise.reject(b);let i=0;while(i<2){const alias=q;q=Promise.reject(p);try{await q;}catch(e){if(e!==p)return false;}try{await alias;}catch(e){if(i)return e===p;}i++;}return false;}",
    ]);
}

#[test]
fn assigned_promise_snapshots_keep_captured_writes_and_activation_identity() {
    compare_snapshots(&[
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b);q=Promise.reject(b);try{await q;}catch{}const set=()=>{q=Promise.reject(p);};set();const alias=q;try{await alias;}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b);const set=()=>{q=Promise.reject(p);};q=Promise.reject(b);try{await q;}catch{}set();const alias=q;try{await alias;}catch(e){return e;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(b);q=Promise.reject(b);try{await q;}catch{}const get=()=>q;q=Promise.reject(p);try{await get();}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);let first=true,saved=()=>Promise.reject(b);const f=x=>{let q=Promise.resolve(b);q=first?Promise.reject(x):Promise.resolve(b);const alias=q,g=()=>alias;if(first){first=false;saved=g;}q=Promise.resolve(a);return first?q:saved();};try{await f(p);}catch(e){}try{await f(b);}catch(e){return e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a),f=async x=>{let q=Promise.resolve(a);q=Promise.reject(x);const alias=q;q=Promise.resolve(b);return await alias;};try{await f(p);}catch(e){if(e!==p)return false;}try{await f(b);}catch(e){return e+b;}}",
    ]);
}

#[test]
fn assigned_promise_snapshots_keep_typed_erasure_and_original_source_limits() {
    let js = "export async function run(a,b){const p=Promise.resolve(a);let q=Promise.resolve(a);q=Promise.reject(b);const alias=q;q=Promise.reject(p);try{await alias;}catch(e){const n=e+b;try{await q;}catch(x){return x===p?n:0;}}}";
    let ts = "export async function run(a:never,b:never):Promise<unknown>{const p=Promise.resolve<Wrong>(a);let q:Wrong=Promise.resolve<Wrong>(a);q=Promise.reject<Missing>(b);const alias:never=q as Wrong;q=Promise.reject<Missing>(p);try{await(alias as Wrong);}catch(e:unknown){const n=e+b;try{await(q as Wrong);}catch(x:unknown){return x===p?n:0;}}}";
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
