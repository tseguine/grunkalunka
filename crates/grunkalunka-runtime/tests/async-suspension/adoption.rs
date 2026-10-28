use super::*;

#[test]
fn pending_promise_returns_promote_original_guard_sources() {
    compare_promises(
        &[
            "let f=async()=>{await a;return b;};return f();",
            "let f=async()=>{await a;return b;};const g=async()=>f();return await g();",
            "const f=async()=>await a;return f();",
            "const f=async()=>{return await a;};return f();",
            "const f=async()=>{await a;return b;};const p=f();return p;",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn pending_promise_returns_adopt_with_native_reaction_order() {
    compare_promises(
        &[
            "const f=async()=>{await a;return b;};return f();",
            "const f=async()=>{await a;return b;};const p=f();const id=async p=>p;return await id(p);",
            "const f=async()=>{await a;return b;};const use=async p=>p;return await use(f());",
            "const f=async()=>{await a;return b;};const p=f();await a;await b;return p;",
            "const f=async()=>{await a;return b;};const p=f();const use=async p=>{return p;};return await use(p);",
            "const f=async()=>{await a;return b;};const p=f();const use=async()=>p;return await use();",
            "const f=async()=>{await a;return b;};const p=f();const id=async p=>p;return id(id(p));",
            "const f=async()=>{await a;return b;};const p=f();const use=async p=>{await a;return p;};return await use(p);",
            "const f=async()=>{await a;return null;};return f();",
            "const f=async()=>{await a;return false;};const p=f();const id=async p=>p;return id(p);",
            "const f=async()=>{await a;return void 0;};return f();",
            "const f=async()=>{await a;return b;};const p=f();return Promise.resolve(p);",
            "const f=async()=>{await a;return b;};const p=f();return(await a,p);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn pending_promise_returns_keep_erasure_and_distinct_site_job_budgets() {
    let plain = "export async function run(a,b){const f=async()=>{await a;return b;};const p=f();const use=async p=>p;return await use(p);}";
    let typed = "export async function run(a:Missing,b:never):Wrong{const f=async():NotPromise=>{await(a as any);return b as string;};const p:Wrong=f();const use=async(p:NotNumber):never=>p as any;return await(use as Missing)(p as never);}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    let prefix = "const f=async()=>{await a;return b;};const p=f();";
    let boundary = format!("{prefix}{}return p;", "await a;".repeat(29));
    compare_promises(&[&boundary], &[(4.0, 2.0), (0.0, -0.0)]);
    let over = format!(
        "{prefix}try{{{}return p;}}catch(e){{return b;}}",
        "await a;".repeat(30)
    );
    for ts in [false, true] {
        let wasm = compile_async(&source_for(&over), ts).unwrap().wasm;
        let mut run = AsyncInstance::new(&wasm, 4.0, 2.0).unwrap();
        let error = loop {
            match run.run_one_job() {
                Ok(true) => {}
                Ok(false) => panic!("33rd job did not fail"),
                Err(error) => break error,
            }
        };
        assert!(error.to_string().contains("32-job budget"));
        assert!(run.state().is_err());
        run_node(&format!(
            "import assert from 'node:assert/strict';import{{pathToFileURL}}from'node:url';const{{createAsyncFunction}}=await import(pathToFileURL(process.argv[2]));await assert.rejects(createAsyncFunction(Uint8Array.from({wasm:?}))(4,2),/32-job budget/);"
        ));
    }
    for extra in [31, 32] {
        let source = source_for(&format!("{prefix}{}return p;", "await a;".repeat(extra)));
        for ts in [false, true] {
            let result = compile_async(&source, ts);
            if extra == 31 {
                result.unwrap();
            } else {
                assert!(
                    result
                        .err()
                        .expect("33 source awaits reject")
                        .contains("32 await sites")
                );
            }
        }
    }
    for body in ["const f=async()=>{await a;return {};};return f();"] {
        for ts in [false, true] {
            assert!(compile_async(&source_for(body), ts).is_err(), "{body}");
        }
    }
}

#[test]
fn pending_promise_returns_keep_rejection_outside_local_catches() {
    compare_promises(
        &[
            "const f=async()=>{await a;throw b;};const p=f();try{return p;}catch(e){return a;}",
            "const f=async()=>{await a;throw b;};const p=f();const use=async p=>{try{return p;}catch(e){return a;}};try{return await use(p);}catch(e){return e;}",
            "const f=async()=>{await a;throw null;};const p=f();const use=async p=>p;try{return await use(p);}catch(e){return e;}",
            "const f=async()=>{await a;throw false;};const p=f();const use=async p=>p;return use(p);",
            "const f=async()=>{await a;throw void 0;};const p=f();const use=async()=>{await a;return p;};return use();",
            "const f=async()=>{await a;return b;};const p=f();try{return(await Promise.reject(a),p);}catch(e){return e;}",
            "const f=async()=>{await a;throw b;};const p=f();const use=async p=>{try{return(await Promise.reject(a),p);}catch(e){try{await p;}catch(e){}return e;}};return await use(p);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn pending_promise_returns_preserve_sibling_frames_and_identity_snapshots() {
    compare_promises(
        &[
            "const f=async x=>{await x;return x;};let id=async p=>p;const p=id(f(a));const q=id(f(b));return +(await p)*1000+(+(await q));",
            "const f=async x=>{await x;return x;};let p=f(a);const use=async p=>p;const q=use(p,p=f(b));return +(await q)*1000+(+(await p));",
            "const f=async()=>{await a;return b;};const p=f();const use=a?async p=>p:async p=>{await a;return p;};return await use(p);",
            "const f=async()=>{await a;return b;};const p=f();const use=async p=>p;const apply=async(fn,p)=>fn(p);return await apply(use,p);",
            "const f=async()=>{await a;return b;};const p=f();const make=p=>async()=>p;const use=make(p);return await use();",
            "const f=async()=>{await a;return b;};const p=f();const use=async p=>{if(a)return p;return b;};return await use(p);",
            "const f=async()=>{await a;return b;};const p=f();const use=async p=>{let i=0;while(i<2){i++;if(a)return p;}return a;};return await use(p);",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}
