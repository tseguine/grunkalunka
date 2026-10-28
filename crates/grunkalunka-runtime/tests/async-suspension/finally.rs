use super::*;

#[test]
fn finally_keeps_pending_returns_and_allows_an_abrupt_override() {
    compare_promises(
        &[
            "try{return a;}finally{await b;}",
            "try{throw a;}finally{return b;}",
            "try{return await a;}finally{return b;}",
            "try{return null;}finally{await a;}",
            "try{return;}finally{await a;}",
            "try{return false;}finally{await a;}",
            "try{await a;}finally{return -0;}",
            "try{throw a;}catch(e){return e;}finally{await b;}",
            "try{await a;}finally{a++;}",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn finally_routes_nested_rejections_and_rethrows_in_lexical_order() {
    compare_promises(
        &[
            "try{try{throw a;}finally{await b;}}catch(e){return e;}",
            "try{try{return a;}finally{throw b;}}catch(e){return e;}",
            "try{try{throw a;}catch(e){await b;throw e;}finally{a=b;}}catch(e){return e;}",
            "try{try{return a;}finally{return await Promise.reject(b);}}catch(e){return e;}",
            "try{return await a;}catch(e){return e;}finally{a=b;}",
            "try{return Promise.reject(a);}catch(e){return b;}finally{await b;}",
            "try{try{return a;}finally{try{throw b;}catch(e){await e;}}}finally{await b;}",
            "try{try{throw a;}finally{try{throw b;}catch(e){await e;}}}catch(e){return e;}",
            "let n=0;try{try{await Promise.reject(a);}catch(e){n=1;throw e;}finally{n=n+2;await b;}}catch(e){return n===3?e:b;}",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn finally_runs_once_for_lexical_loop_exits_and_overrides() {
    compare_promises(
        &[
            "let n=0;for(let i=0;i<3;i++){try{n=n+1;if(i===1)continue;}finally{n=n*2;await a;}}return n;",
            "let n=0;while(a){try{n++;break;}finally{await b;n=n+2;}}return n;",
            "let n=0;while(n<3){n++;try{continue;}finally{if(n===2)break;await b;}}return n;",
            "let n=0;while(n<2){n++;try{break;}finally{continue;}}return n;",
            "let n=0;do{try{n++;continue;}finally{await b;}}while(n<2);return n;",
            "try{return a;}finally{while(b){break;}await b;}",
            "let n=0;for(let i=0;i<2;i++){try{while(b){break;}n++;}finally{await a;n=n+2;}}return n;",
            "let n=0;while(n<2){try{try{n++;break;}finally{n++;}}finally{await a;n++;}}return n;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn finally_keeps_snapshots_cells_and_private_activation_boundaries() {
    compare_promises(
        &[
            "let v=a;try{return v;}finally{v=b;await a;}",
            "let n=0;const id=x=>x;try{n=id(a);n++;return n;}finally{n=b;await a;}",
            "let read=()=>a;try{let v=b;read=()=>v;return read();}finally{await a;}",
            "const f=async()=>{try{return a;}finally{await b;}};return await f();",
            "const f=async()=>{try{throw false;}catch(e){return e;}finally{await b;}};return await f();",
            "let f=async()=>{try{await a;}finally{b++;}return b;};return await f();",
            "let f=a?async()=>{try{return a;}finally{await b;}}:async()=>{try{return b;}finally{await a;}};return await f();",
            "let n=0;const f=async x=>{try{n=n*10+x;return n;}finally{await a;n=n*10+2;}};const p=f(1),q=f(3);const x=await p;await q;return n+x;",
            "const make=x=>async()=>{try{return x;}finally{await b;}};let f=make(a);return await f();",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn finally_precedes_return_adoption_and_keeps_raw_promise_identity() {
    compare_promises(
        &[
            "let n=0;const child=async()=>{await a;n=10;return b;};const p=child();const f=async()=>{try{return p;}finally{n=n+1;await b;n=n+2;}};const q=f();const at=n;await q;return at;",
            "const p=Promise.resolve(a);try{return p;}finally{return b;}",
            "let p=Promise.resolve(a);try{return p;}finally{p=Promise.resolve(b);await b;}",
            "const p=Promise.resolve(a);try{return a?p:b;}finally{await a;}",
            "let n=0;const child=async()=>{await a;return b;};const p=child();const f=async()=>{try{try{return p;}finally{n=n+1;await b;}}finally{n=n+2;}};const q=f();const at=n;await q;return at;",
            "const p=Promise.reject(a);try{try{return p;}finally{await b;}}catch(e){return b;}",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
    let cycle = "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();";
    compare_async_sources(
        &[
            format!(
                "export async function run(a,b){{{cycle}try{{await p;}}catch(e){{const read=async()=>{{try{{return e;}}finally{{await a;}}}};return(await read())===e;}}}}"
            ),
            format!(
                "export async function run(a,b){{{cycle}try{{await p;}}catch(e){{try{{try{{throw e;}}finally{{await a;}}}}catch(old){{return old===e;}}}}}}"
            ),
        ],
        &[(4.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn finally_keeps_erasure_source_await_and_execution_limits() {
    for depth in [1, 4, 16, 65] {
        let mut body = "await b;".to_string();
        for _ in 0..depth {
            body = format!("try{{return a;}}finally{{{body}}}");
        }
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile_async(&source_for(&body), false))
            .unwrap()
            .join()
            .unwrap();
        if depth <= 4 {
            assert!(result.is_ok(), "depth {depth}: {:?}", result.err());
        } else {
            let error = result
                .err()
                .expect("nested cleanup must reach a bounded source/state guard");
            assert!(
                error.contains("4096") || error.contains("nest"),
                "{depth}: {error}"
            );
        }
    }
    let plain = source_for("try{return a;}catch(e){return e;}finally{await b;}");
    let typed = plain
        .replace("run(a,b)", "run(a:Missing,b:never):Wrong")
        .replace("catch(e)", "catch(e:Bad)")
        .replace("return a;", "return (a as never);");
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
    for count in [0, 32] {
        let body = format!(
            "try{{if(a)return b;throw b;}}finally{{{}}}",
            "await a;".repeat(count)
        );
        compare_promises(&[&body], &[(4.0, 2.0), (0.0, -0.0)]);
        let body = format!(
            "for(let i=0;i<1;i++){{try{{if(a)break;continue;}}finally{{{}}}}}return b;",
            "await a;".repeat(count)
        );
        compare_promises(&[&body], &[(4.0, 2.0), (0.0, -0.0)]);
    }
    let over = source_for(&format!(
        "try{{return a;}}finally{{{}}}",
        "await b;".repeat(33)
    ));
    for ts in [false, true] {
        assert!(
            compile_async(&over, ts)
                .err()
                .unwrap()
                .contains("32 await sites")
        );
        assert!(
            compile_async(
                &source_for("try{return a;}catch({x}){return b;}finally{await a;}"),
                ts
            )
            .is_err()
        );
        assert!(
            compile_async(
                &source_for("const f=()=>{try{return a;}finally{b++;}};return f();"),
                ts
            )
            .is_err()
        );
    }
    assert_eq!(
        source_for("try{await a;}finally{a++;}"),
        "export async function run(a,b){try{await a;}finally{a++;}}"
    );
    let wasm = compile_async(&source_for("try{while(a){a=a;}}finally{return b;}"), false)
        .unwrap()
        .wasm;
    assert!(AsyncInstance::new(&wasm, 4.0, 2.0).is_err());
    assert!(matches!(
        AsyncInstance::new(&wasm, 0.0, 2.0)
            .unwrap()
            .state()
            .unwrap(),
        AsyncState::Fulfilled(Value::Number(2.0))
    ));
    let wasm = compile_async(
        &source_for("try{for(let i=0;i<33;i++)await a;}finally{return b;}"),
        false,
    )
    .unwrap()
    .wasm;
    let mut call = AsyncInstance::new(&wasm, 4.0, 2.0).unwrap();
    for _ in 0..31 {
        assert!(call.run_one_job().unwrap());
    }
    assert!(call.run_one_job().is_err());
    assert!(call.state().is_err());
    run_node(&format!(
        r#"import assert from 'node:assert/strict';import {{pathToFileURL}} from 'node:url';const {{createAsyncFunction}}=await import(pathToFileURL(process.argv[2]));await assert.rejects(createAsyncFunction(new Uint8Array({:?}))(4,2),/32-job budget/);"#,
        wasm
    ));
}
