use super::*;

#[test]
fn selected_async_returns_preserve_result_and_adoption_order() {
    compare_promises(
        &[
            "const f=async()=>{await a;return b;};const p=f();return a?p:b;",
            "let f=async()=>{await a;return b;};return a?f():b;",
            "let f=async()=>{await a;return b;};const g=async()=>a?f():b;return await g();",
            "const p=Promise.resolve(a);return a?b:p;",
            "const p=Promise.resolve(a);return a?p:null;",
            "const p=Promise.resolve(a);return a?undefined:p;",
            "const p=Promise.resolve(a);return a?p:a<b;",
            "const p=Promise.resolve(a);return p?b:Promise.resolve(a);",
            "const p=Promise.resolve(a);return a?(b?p:undefined):(b?null:Promise.resolve(b));",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn async_constructors_adopt_any_selected_promise_return_leaf() {
    compare_promises(
        &[
            "const f=async x=>a?Promise.resolve(x):x;return await f(b);",
            "const f=async x=>{const p=Promise.resolve(x);return a?p:x;};return await f(b);",
            "const p=Promise.resolve(b);const f=async x=>a?p:x;return await f(a);",
            "let f=a?async x=>a?Promise.resolve(x):null:async x=>b?false:Promise.resolve(x);return await f(b);",
            "const apply=async(fn,p,x)=>fn(p,x);const f=async(p,x)=>a?p:x;return await apply(f,Promise.resolve(b),a);",
            "let f=async x=>a?Promise.resolve(x):x;const p=f(a),q=f(b);return (await p)+(await q)*100;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn selected_return_prefixes_conditions_and_arms_run_once_in_source_order() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);return (await b,a?p:b);",
            "const p=Promise.resolve(a);return (await a)?p:b;",
            "let n=0;const p=Promise.resolve(a);const f=async()=>((n=n*10+1),await b,(n=n*10+2),a?(n=n*10+3,p):(n=n*10+4,b));const q=f();const before=n;return (await q)+before*100+n*1000;",
            "let n=0;const p=Promise.resolve(a);const f=async()=>a?(n=1,p):(n=2,b);const q=f();const before=n;return (await q)+before*100+n*1000;",
            "let n=0;const f=async()=>a?(n=1,Promise.resolve(b)):(n=2,b);const q=f();const before=n;return (await q)+before*100+n*1000;",
            "const p=Promise.resolve(a);return a?(await b,p):(await a,b);",
            "const f=async()=>{let p=Promise.resolve(a);return (p=Promise.resolve(b),a?p:a);};return await f();",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn selected_return_adoption_bypasses_own_catch_but_evaluation_throws_do_not() {
    compare_promises(
        &[
            "try{return a?Promise.reject(b):b;}catch(e){return 999;}",
            "const f=async()=>{try{return a?Promise.reject(b):b;}catch(e){return 999;}};try{return await f();}catch(e){return +e+100;}",
            "const f=async()=>{try{return a?await Promise.reject(b):b;}catch(e){return +e+7;}};return await f();",
            "const p=Promise.resolve(a);try{return (await Promise.reject(b),a?p:b);}catch(e){return +e+9;}",
            "const p=Promise.resolve(a);try{return (await Promise.reject(b))?p:a;}catch(e){return +e+11;}",
            "const f=async()=>a?(b?Promise.reject(b):null):false;try{return await f();}catch(e){return e;}",
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
fn selected_returns_preserve_intrinsic_error_payloads() {
    let cycle = "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();";
    let tails = [
        "return a?Promise.resolve(e):e;",
        "return a?e:Promise.reject(e);",
        "const use=async x=>a?Promise.resolve(x):x;return (await use(e))===e;",
    ]
    .map(|tail| format!("{cycle}try{{await p;}}catch(e){{{tail}}}"));
    compare_promises(
        &tails.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn selected_returns_keep_erasure_and_separate_source_sites_from_runtime_jobs() {
    compare_promises(
        &[
            "const apply=(fn,p,x)=>fn(p,x);const f=async(p,x)=>a?p:x;return await apply(f,Promise.resolve(b),a);",
        ],
        &[(4.0, 2.0), (0.0, -0.0)],
    );
    let plain = source_for("const p=Promise.resolve(a);const f=async x=>x?p:b;return await f(a);");
    let typed = plain
        .replace("run(a,b)", "run(a:never,b:Missing):Wrong")
        .replace(
            "async x=>x?p:b",
            "async(x:Bad):Other=>(x as never)?(p as unknown as number):(b as Wrong)",
        );
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
    for count in [30, 31, 32, 33] {
        let body = format!(
            "const p=Promise.resolve(a);{}return b?p:a;",
            "await a;".repeat(count)
        );
        let source = source_for(&body);
        if count == 33 {
            assert!(
                compile_async(&source, false)
                    .err()
                    .unwrap()
                    .contains("32 await sites")
            );
            continue;
        }
        assert!(compile_async(&source, false).is_ok());
        if count <= 30 {
            compare_promises(&[&body], &[(4.0, 2.0), (4.0, 0.0)]);
        } else {
            compare_promises(&[&body], &[(4.0, 0.0)]);
            let wasm = compile_async(&source, false).unwrap().wasm;
            let mut run = AsyncInstance::new(&wasm, 4.0, 2.0).unwrap();
            let error = loop {
                match run.run_one_job() {
                    Ok(true) => (),
                    Ok(false) => panic!("extra adoption jobs must exceed the production budget"),
                    Err(error) => break error,
                }
            };
            assert!(error.to_string().contains("budget"));
            assert!(run.state().is_err());
            assert!(run.run_one_job().is_err());
        }
    }
}
