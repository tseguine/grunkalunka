use super::*;

#[test]
fn nested_logical_left_values_keep_promise_identity_until_selection() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);return (a&&p)||b;",
            "const p=Promise.resolve(a);return (a?p:b)&&p;",
            "const p=Promise.resolve(a);return (a?p:null)??b;",
            "const p=Promise.resolve(a);return (a||p)&&b;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn nested_async_constructor_joins_keep_raw_values_and_runtime_signatures() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);const f=async x=>(x?p:b)||b;return await f(a);",
            "const p=Promise.resolve(a);let f=async x=>(x&&p)||b;const q=f(a),r=f(b);return (await q)+(await r)*100;",
            "const p=Promise.resolve(a);let f=a?async x=>(x?p:0)&&b:async x=>(x||p)??b;return await f(b);",
            "const apply=async(fn,p,x)=>fn(p,x);const f=async(p,x)=>(x&&p)||b;return await apply(f,Promise.resolve(a),b);",
            "const p=Promise.resolve(a);return ((a?p:b)&&(b?p:a))||b;",
            "const p=Promise.resolve(a);return (a?(b?p:null):undefined)??b;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn nested_join_effects_run_once_and_only_selected_jobs_execute() {
    compare_promises(
        &[
            "let n=0;const p=Promise.resolve(a);const f=async()=>((n=1,a)?(n=2,p):(n=3,b))||(n=4,await b);const q=f();const before=n;return (await q)+before*100+n*1000;",
            "let n=0;const p=Promise.resolve(a);const f=async()=>((n=1,a)&&(n=2,p))&&(n=3,await b,n=4,b);const q=f();const before=n;return (await q)+before*100+n*1000;",
            "let n=0;const p=Promise.resolve(a);const f=async()=>((n=1,a)?(await b,n=2,p):(await a,n=3,null))??(n=4,b);const q=f();const before=n;return (await q)+before*100+n*1000;",
            "const f=async()=>{let p=Promise.resolve(a);return ((await b,a)?p:b)||(p=Promise.resolve(b));};return await f();",
            "const p=Promise.resolve(a);return ((await b,a)?p:b)||b;",
            "const p=Promise.resolve(undefined);return (a?p:null)??b;",
            "const p=Promise.resolve(false);return (a?p:b)||a;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn nested_join_rejection_stays_distinct_from_expression_failure() {
    compare_promises(
        &[
            "try{return (a?Promise.reject(b):0)||a;}catch(e){return 999;}",
            "const f=async()=>{try{return (a?Promise.reject(b):0)??a;}catch(e){return 999;}};try{return await f();}catch(e){return +e+7;}",
            "const p=Promise.resolve(a);try{return ((await Promise.reject(b))?p:0)||a;}catch(e){return +e+9;}",
            "const p=Promise.resolve(a);try{return (a?(await Promise.reject(b),p):null)??a;}catch(e){return +e+11;}",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn nested_join_error_payloads_retain_truthiness_and_identity() {
    let cycle = "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();";
    let tails = [
        "return (a?Promise.resolve(e):e)||b;",
        "return (a?e:Promise.resolve(e))??b;",
        "const p=Promise.resolve(e);const use=async()=> (a?p:e)&&e;return (await use())===e;",
    ]
    .map(|tail| format!("{cycle}try{{await p;}}catch(e){{{tail}}}"));
    compare_promises(
        &tails.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0)],
    );
    let comparison = source_for(&format!(
        "{cycle}try{{await p;}}catch(e){{const p=Promise.resolve(e);return ((a?p:e)&&e)===e;}}"
    ));
    compare_async_sources(&[comparison], &[(4.0, 2.0), (0.0, -0.0)]);
}

#[test]
fn nested_joins_keep_erasure_and_site_job_and_storage_boundaries() {
    let plain = source_for("const p=Promise.resolve(a);return (a?p:b)||b;");
    let typed = plain
        .replace("run(a,b)", "run(a:never,b:Missing):Wrong")
        .replace("(a?p:b)", "((a as never)?(p as Wrong):(b as unknown))");
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
    for (count, a) in [(30, 4.0), (32, 0.0)] {
        let body = format!(
            "const p=Promise.resolve(a);{}return (a?p:0)||b;",
            "await a;".repeat(count)
        );
        compare_promises(&[&body], &[(a, -0.0)]);
    }
    for ts in [false, true] {
        let over = source_for(&format!(
            "const p=Promise.resolve(a);{}return (a?p:0)||b;",
            "await a;".repeat(31)
        ));
        let wasm = compile_async(&over, ts).unwrap().wasm;
        let mut run = AsyncInstance::new(&wasm, 4.0, 2.0).unwrap();
        let error = loop {
            match run.run_one_job() {
                Ok(true) => (),
                Ok(false) => panic!("selected adoption exceeds budget"),
                Err(e) => break e,
            }
        };
        assert!(error.to_string().contains("32-job budget"));
        assert!(run.state().is_err());
        let too_deep = source_for(&format!(
            "const p=Promise.resolve(a);{}return (a?p:0)||b;",
            "await a;".repeat(33)
        ));
        assert!(
            compile_async(&too_deep, ts)
                .err()
                .unwrap()
                .contains("32 await sites")
        );
        // Source locals and private inputs still have their existing homogeneous contracts.
        for body in [
            "const p=Promise.resolve(a);const x=a?p:b;return x||b;",
            "const p=Promise.resolve(a);const f=async x=>x||b;await f(p);return await f(a);",
        ] {
            assert!(compile_async(&source_for(body), ts).is_err(), "{body}");
        }
    }
}
