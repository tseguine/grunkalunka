use super::*;

#[test]
fn logical_async_returns_select_before_adopting() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);return a&&p;",
            "const p=Promise.resolve(a);return p||b;",
            "const p=Promise.resolve(a);return p&&b;",
            "const p=Promise.resolve(a);return a||p;",
            "const p=Promise.resolve(a);return p??b;",
            "const p=Promise.resolve(a);return null??p;",
            "const p=Promise.resolve(a);return undefined??p;",
            "const p=Promise.resolve(a);return false??p;",
            "const p=Promise.resolve(a);return (a<b)&&p;",
            "const p=Promise.resolve(a);return a&&(b?p:null);",
            "const p=Promise.resolve(a);return a||(b&&p);",
            "const p=Promise.resolve(a);return (await a)||p;",
            "const p=Promise.resolve(a);return (a?p:p)||b;",
            "const p=Promise.resolve(a);return (p&&p)||b;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn logical_returns_keep_erasure_and_selected_job_budgets() {
    let plain = source_for("const p=Promise.resolve(a);const f=async x=>x||p;return await f(b);");
    let typed = plain
        .replace("run(a,b)", "run(a:never,b:Missing):Wrong")
        .replace(
            "async x=>x||p",
            "async(x:Wrong):never=>(x as unknown)||(p as Missing)",
        );
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
    for (count, tail) in [(30, "p||b"), (32, "p&&b"), (32, "a&&p")] {
        let body = format!(
            "const p=Promise.resolve(a);{}return {tail};",
            "await a;".repeat(count)
        );
        // Zero selects the direct scalar in a&&p; p remains truthy when fulfilled by zero.
        compare_promises(&[&body], &[(0.0, -0.0)]);
    }
    let over = source_for(&format!(
        "const p=Promise.resolve(a);{}return p||b;",
        "await a;".repeat(31)
    ));
    let too_many_sites = source_for(&format!(
        "const p=Promise.resolve(a);{}return p&&b;",
        "await a;".repeat(33)
    ));
    for ts in [false, true] {
        let wasm = compile_async(&over, ts).unwrap().wasm;
        let mut run = AsyncInstance::new(&wasm, 0.0, -0.0).unwrap();
        let error = loop {
            match run.run_one_job() {
                Ok(true) => (),
                Ok(false) => panic!("selected adoption must exceed the 32-job budget"),
                Err(error) => break error,
            }
        };
        assert!(error.to_string().contains("32-job budget"));
        assert!(run.state().is_err());
        assert!(run.run_one_job().is_err());
        assert!(
            compile_async(&too_many_sites, ts)
                .err()
                .unwrap()
                .contains("32 await sites")
        );
    }
}

#[test]
fn logical_return_effects_keep_snapshots_and_skipped_jobs() {
    compare_promises(
        &[
            "let n=0;const f=async()=>((n=1,Promise.resolve(a))||(n=2,await Promise.reject(b)));const p=f();const before=n;return (await p)+before*100+n*1000;",
            "let n=0;const f=async()=>((n=1,Promise.resolve(a))&&(n=2,b));const p=f();const before=n;return (await p)+before*100+n*1000;",
            "let n=0;const f=async()=>((n=1,Promise.resolve(a))??(n=2,await Promise.reject(b)));const p=f();const before=n;return (await p)+before*100+n*1000;",
            "let n=0;const f=async()=>((n=1,a)&&(n=2,await b,n=3,Promise.resolve(b)));const p=f();const before=n;return (await p)+before*100+n*1000;",
            "const f=async()=>{let p=Promise.resolve(a);return p||(p=Promise.resolve(b));};return await f();",
            "const p=Promise.resolve(b);return (await a,p)&&a;",
            "const p=Promise.resolve(a);return p&&(await b,a);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn logical_return_rejection_differs_from_expression_failure() {
    compare_promises(
        &[
            "try{return a&&Promise.reject(b);}catch(e){return 999;}",
            "try{return a||Promise.reject(b);}catch(e){return 999;}",
            "try{return Promise.reject(b)||a;}catch(e){return 999;}",
            "try{return Promise.reject(b)??a;}catch(e){return 999;}",
            "const p=Promise.resolve(a);try{return p&&(await Promise.reject(b),a);}catch(e){return +e+7;}",
            "const p=Promise.resolve(a);try{return (await Promise.reject(b))||p;}catch(e){return +e+9;}",
            "const f=async()=>{try{return a&&Promise.reject(b);}catch(e){return 999;}};try{return await f();}catch(e){return +e+11;}",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn logical_return_intrinsic_errors_keep_object_truthiness_and_identity() {
    let cycle = "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();";
    let sources = [
        "return e&&Promise.resolve(e);",
        "return e||Promise.resolve(a);",
        "return Promise.resolve(e)&&e;",
        "const use=async x=>a&&Promise.resolve(x);return a?(await use(e))===e:await use(e);",
    ]
    .map(|tail| format!("{cycle}try{{await p;}}catch(e){{{tail}}}"));
    compare_promises(
        &sources.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn logical_async_constructors_preserve_selected_resolution() {
    compare_promises(
        &[
            "const f=async x=>a&&Promise.resolve(x);return await f(b);",
            "const f=async x=>a||Promise.resolve(x);return await f(b);",
            "const f=async x=>{const p=Promise.resolve(x);return p&&b;};return await f(a);",
            "const p=Promise.resolve(a);let f=a?async x=>p||x:async x=>x&&p;return await f(b);",
            "const apply=async(fn,p,x)=>fn(p,x);const f=async(p,x)=>x||p;return await apply(f,Promise.resolve(b),a);",
            "const p=Promise.resolve(a);let f=async x=>x&&p;const q=f(a),r=f(b);return (await q)+(await r)*100;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}
