use super::*;

#[test]
fn stored_pending_async_results_preserve_aliases_assignments_and_captured_getters() {
    compare_promises(
        &[
            "const f=async x=>await x;let p=f(a);const old=p;p=f(b);const l=await old;return +l-(+(await p));",
            "const f=async x=>await x;let p=f(a);const get=()=>p;const old=get();p=f(b);await old;return await get();",
            "const f=async x=>await x;let p=f(a);const get=()=>{return p;};const old=get();p=f(b);await old;return await get();",
            "const f=async()=>{await a;return b;};const p=f();const g=async()=>{await a;return await p;};const q=g();return await q;",
            "const f=async()=>{await a;return b;};const p=f();return await Promise.resolve(p);",
            "const f=async()=>{await a;return b;};const p=f();const q=Promise.resolve(p,await b);return await q;",
            "const f=async()=>{await a;return a;};const p=f();const q=Promise.resolve(p,a=b);return +(await q)+a;",
            "let f=async()=>{await a;return b;};return await Promise.resolve(f(),await b);",
            "const f=async()=>{await a;return b;};const p=f();await p;await p;return await p;",
            "let f=async()=>{await a;return b;};const p=f();return await p;",
            "const f=async x=>{await x;return x;};let p=f(a);const old=(p=f(b));await p;return await old;",
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
fn stored_pending_async_results_keep_erasure_admission_and_site_limits() {
    let plain = "export async function run(a,b){const f=async x=>{await a;return x;};let p=f(b);const old=p;p=f(a);await old;return await p;}";
    let typed = "export async function run(a:never,b:Missing):Wrong{const f=async(x:NotNumber)=>{await(a as any);return x;};let p:NotPromise=f(b as never);const old=p;p=f(a as any);await(old as any);return await(p as Wrong);}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    let mut selection = "p".to_owned();
    for _ in 0..48 {
        selection = format!("(a?p:{selection})");
    }
    compare_promises(
        &[&format!(
            "const f=async()=>{{await a;return b;}};const p=f();const q={selection};return await q;"
        )],
        &[(4.0, 2.0), (0.0, -0.0)],
    );
    let deep = format!(
        "const f=async()=>{{await a;return b;}};const p=f();const q={}p{};return await q;",
        "(".repeat(130),
        ")".repeat(130)
    );
    assert!(compile_async(&source_for(&deep), false).is_err());
    for count in [31, 32] {
        let body = format!(
            "const f=async()=>{{await a;return b;}};const p=f();{}return b;",
            "await p;".repeat(count)
        );
        if count == 31 {
            compare_promises(&[&body], &[(4.0, 2.0)]);
        } else {
            assert!(
                compile_async(&source_for(&body), false)
                    .err()
                    .expect("33 source sites reject")
                    .contains("32 await sites")
            );
        }
    }
    for body in [
        "const f=async()=>{await a;return b;};const p=f();return +p;",
        "const f=async()=>{await a;return b;};let p=f();p=b;return await p;",
        "const f=async()=>{await a;return b;};const p=f();p=f();return await p;",
        "const f=async()=>{await a;return b;};const p=f();return await Promise.reject(p);",
        "const f=async()=>{await a;return b;};const p=f();const q=a?p:Promise.resolve(b);return await q;",
    ] {
        for ts in [false, true] {
            assert!(compile_async(&source_for(body), ts).is_err(), "{body}");
        }
    }
    let under =
        "const f=async()=>{await a;return b;};const p=f();for(let i=0;i<31;i++)await p;return b;";
    compare_promises(&[under], &[(4.0, 2.0)]);
    let over = "const f=async()=>{await a;return b;};const p=f();try{for(let i=0;i<32;i++)await p;}catch(e){return 0;}return b;";
    let bytes = compile_async(&source_for(over), false).unwrap().wasm;
    let mut run = AsyncInstance::new(&bytes, 4.0, 2.0).unwrap();
    let mut failure = None;
    for _ in 0..33 {
        match run.run_one_job() {
            Ok(true) => (),
            Ok(false) => panic!("over-budget source unexpectedly finished"),
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
    }
    assert!(
        failure
            .expect("33rd issued job fails")
            .to_string()
            .contains("32-job budget")
    );
    assert!(run.state().is_err());
    run_node(&format!(
        r#"import assert from 'node:assert/strict';import{{pathToFileURL}}from'node:url';const{{createAsyncFunction}}=await import(pathToFileURL(process.argv[2]));await assert.rejects(createAsyncFunction(Uint8Array.from({bytes:?}))(4,2),e=>e instanceof RangeError&&e.message.includes('32-job budget'));"#
    ));
}

#[test]
fn stored_pending_async_results_keep_object_selections_and_skipped_jobs() {
    compare_promises(
        &[
            "const f=async x=>await x;const p=f(a);const q=f(b);return await(a?p:q);",
            "const f=async()=>{await a;return b;};const p=f();return await Promise.resolve(a?p:f(),await b);",
            "const f=async x=>await x;const p=f(a);return await(p&&f(b));",
            "const f=async x=>await x;const p=f(a);return await(p||(await Promise.reject(a),f(b)));",
            "const f=async x=>await x;const p=f(a);return await(p??(await b,f(b)));",
            "const f=async x=>await x;let p=f(a);p&&=f(b);return await p;",
            "const f=async x=>await x;let p=f(a);p||=(await Promise.reject(b),f(b));return await p;",
            "const f=async x=>await x;let p=f(a);p??=(await b,f(b));return await p;",
            "let f=async()=>{await a;return b;};if(f())return a;return b;",
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
fn stored_pending_async_results_preserve_rejection_and_argument_order() {
    compare_promises(
        &[
            "const f=async()=>{await a;throw b;};const p=f();try{return await p;}catch(e){await a;return e;}",
            "const f=async()=>{await a;return await Promise.reject(null);};let p=f();const q=p;try{await q;}catch(e){try{return await p;}catch(r){return r;}}",
            "const f=async()=>{await a;throw void 0;};const p=f();return await p;",
            "const f=async()=>{await a;throw a?false:true;};const p=f();try{return await p;}catch(e){return e;}",
            "let f=async x=>{await a;return x;};const g=async x=>{await b;return x+b;};const p=f((f=g,a),await b);const q=f(b);const l=await p;const r=await q;return +l*1000+(+r);",
            "const f=async(x,y)=>{await a;return x-y;};let x=a;const p=f(x,x=b,await a);return await p;",
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
fn stored_pending_async_results_keep_ready_work_after_the_root_returns() {
    compare_promises(
        &[
            "let f=async()=>{await a;return b;};f();return b;",
            "let f=async()=>{await a;return b;};return void f();",
            "const f=async()=>{await a;await b;return b;};f();f();return a;",
            "let x=0;const f=async()=>{x+=a;await b;x+=b;return x;};f();f();await b;return x;",
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
fn stored_pending_async_results_start_eagerly_and_are_awaited_later() {
    compare_promises(
        &[
            "let f=async()=>{await a;return b;};let p=f();return await p;",
            "const f=async(x,y)=>{await x;return y;};const p=f(a,b);const q=f(b,a);await p;return await q;",
            "let x=0;const f=async()=>{x+=a;await b;x+=b;return x;};const p=f();const q=f();const l=await p;const r=await q;return +l*1000+(+r);",
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
fn stored_pending_async_results_promote_original_guards() {
    compare_promises(
        &[
            "let f=async()=>{await a;return b;};if(a?f():b)return a;return b;",
            "let f=async()=>{await a;return b;};return await(f()?a:b);",
            "let f=async()=>{await a;return b;};return await(f(),a?b:a);",
            "const read=async p=>await p;const p=read(Promise.resolve(a));return await p;",
            "const f=async()=>{await b;return a;};return await Promise.resolve(f(),b);",
            "const f=async()=>await a;const p=f();return await p;",
            "const f=async()=>{return await a;};const p=f();return await p;",
            "let f=async()=>{await a;return b;};const p=f();return await p;",
            "let f=async()=>{await a;return b;};f();return b;",
            "let f=async()=>{await a;return b;};return void f();",
            "let f=async()=>{await a;return b;};if(f())return a;return b;",
            "let f=async()=>{await a;return b;};return await Promise.resolve(f(),await b);",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}
