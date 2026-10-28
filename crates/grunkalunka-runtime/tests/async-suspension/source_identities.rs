use super::*;

#[test]
fn completed_constructor_stress_prefix_fits_default_fuel() {
    let body = format!(
        "const id=p=>{{{}return p;}};return await id(Promise.resolve(b));",
        "p=Promise.resolve(a);".repeat(140)
    );
    let wasm = compile_async(&source_for(&body), false).unwrap().wasm;
    let mut config = wasmtime::Config::new();
    config.consume_fuel(true);
    let engine = wasmtime::Engine::new(&config).unwrap();
    let module = wasmtime::Module::from_binary(&engine, &wasm).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let observation_budget = grunkalunka_runtime::FUEL_PER_CALL * 10;
    store.set_fuel(observation_budget).unwrap();
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let start = instance
        .get_typed_func::<(f64, f64), (i32, i32, f64)>(&mut store, "start")
        .unwrap();
    assert_eq!(start.call(&mut store, (4.0, 2.0)).unwrap().0, 1);
    let used = observation_budget - store.get_fuel().unwrap();
    assert!(
        used <= grunkalunka_runtime::FUEL_PER_CALL,
        "constructor prefix used {used} fuel"
    );
}

#[test]
fn completed_source_promises_have_stable_identity() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);return p===p;",
            "const p=Promise.resolve(a),q=Promise.resolve(a);return p!==q;",
            "const p=Promise.resolve(a);const q=p;return p==q;",
            "const p=Promise.resolve();return await p;",
            "const p=Promise.resolve(null);return await p;",
            "const p=Promise.resolve(a<b);return await p;",
            "const p=Promise.resolve(a);return p===Promise.resolve(p);",
            "let p=Promise.resolve(a);const old=p;const q=Promise.resolve(p,p=Promise.resolve(b));return q===old&&p!==old;",
            "let p=Promise.resolve(a);const old=p;p=Promise.resolve(b);return old===(p=old);",
            "const p=Promise.resolve(a);return p!==a&&p!=null&&p!==b;",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, 1.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn source_constructors_snapshot_payloads_before_surplus_argument_effects() {
    compare_promises(
        &[
            "let x=a;const p=Promise.resolve(x,x=b);return (await p)+x;",
            "let x=a;const p=Promise.reject(x,x=b);try{await p;}catch(e){return e+x;}",
            "let n=0;const p=Promise.resolve((n=1,a),n=n*10+2,n=n*10+3);await p;return n;",
            "let n=0;const p=Promise.resolve((n=1,a),await(n=n*10+2),n=n*10+3);await p;return n;",
            "let p=Promise.resolve(a);const old=p;const q=Promise.resolve(p,await(p=Promise.resolve(b),a));return q===old&&p!==old;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn completed_promises_flow_through_private_factories_callbacks_and_captures() {
    compare_promises(
        &[
            "const make=x=>Promise.resolve(x);const p=make(a),q=make(a);return p!==q;",
            "const id=x=>x;const p=Promise.resolve(a);return id(p)===p;",
            "const id=a?x=>x:x=>Promise.resolve(x);const p=Promise.resolve(b);return id(p)===p;",
            "const apply=(fn,x)=>fn(x);const p=Promise.resolve(a);return apply(x=>Promise.resolve(x),p)===p;",
            "const make=x=>()=>Promise.resolve(++x);const f=make(a);const p=f(),q=f();return p!==q?(await p)+(await q):0;",
            "let p=Promise.resolve(a);const get=()=>p;const old=get();p=Promise.resolve(b);return old!==get();",
            "const make=x=>{const p=Promise.resolve(x);return p;};return await make(a);",
            "const make=a?x=>Promise.resolve(x):x=>Promise.reject(x);const p=make(b);try{return await p;}catch(e){return e;}",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn completed_source_returns_adopt_without_becoming_lexical_awaits() {
    compare_promises(
        &[
            "return Promise.resolve(a);",
            "return Promise.reject(a);",
            "const f=async x=>x;return f(a);",
            "const f=async x=>{throw x;};try{return f(a);}catch(e){return b;}",
            "const f=async x=>Promise.resolve(x);return await f(a);",
            "const make=x=>Promise.resolve(x);let f=async x=>{await a;return make(x);};const p=f(a),q=f(b);return (await p)+(await q)*100;",
            "const f=async x=>Promise.reject(x);try{return await f(a);}catch(e){return e;}",
            "const p=Promise.reject(a);try{return p;}catch(e){return b;}",
            "const f=a?async x=>x:async x=>{throw x;};const p=f(b);try{return await p;}catch(e){return e;}",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn source_constructors_preserve_intrinsic_error_identity() {
    let cycle = "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();";
    let tails = [
        "const q=Promise.resolve(e);return (await q)===e;",
        "const q=Promise.reject(e);try{await q;}catch(x){return x===e;}",
        "const h=async x=>x;const q=h(e);return (await q)===e;",
        "const h=async x=>{throw x;};const q=h(e);try{await q;}catch(x){return x===e;}",
    ]
    .map(|tail| format!("{cycle}try{{await p;}}catch(e){{{tail}}}"));
    compare_promises(
        &tails.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn source_identity_construction_keeps_erasure_and_await_site_limits() {
    let plain = source_for("const p=Promise.resolve(a);return p===Promise.resolve(p);");
    let typed = plain
        .replace("run(a,b)", "run(a:never,b:Missing):Wrong")
        .replace("const p=", "const p:Promise<Missing>=")
        .replace("return p===", "return (p as unknown as number)===");
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
    for count in [32, 33] {
        let body = format!(
            "const p=Promise.resolve(a);{}return p===p;",
            "await p;".repeat(count)
        );
        if count == 32 {
            compare_promises(&[&body], &[(4.0, 2.0)]);
        } else {
            assert!(
                compile_async(&source_for(&body), false)
                    .err()
                    .unwrap()
                    .contains("32 await sites")
            );
        }
    }
}

#[test]
fn immediate_async_calls_create_distinct_promises_and_preserve_aliases() {
    compare_promises(
        &[
            "const f=async x=>x;const p=f(a),alias=p,q=f(a);return p===alias&&p!==q;",
            "const f=async x=>x;const p=f(a);return p===Promise.resolve(p);",
            "let n=0;const f=async x=>{n=n*10+x;return n;};const p=f(1),q=f(2);return (await p)+(await q)*100+n;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn rejected_constructors_retain_identity_reason_and_await_reactions() {
    compare_promises(
        &[
            "const p=Promise.reject(a);const alias=p;try{await p;}catch(e){return p===alias;}",
            "const p=Promise.reject(a),q=Promise.reject(a);try{await p;}catch(e){}try{await q;}catch(e){return p!==q;}",
            "const p=Promise.reject(a);try{await p;}catch(e){return e;}",
            "const f=async x=>{throw x;};const p=f(a),q=Promise.resolve(p);try{await p;}catch(e){return p===q;}",
            "const p=Promise.reject();try{await p;}catch(e){return e;}",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, 1.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}
