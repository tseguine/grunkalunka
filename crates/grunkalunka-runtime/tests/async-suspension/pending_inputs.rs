use super::*;

#[test]
fn pending_promise_inputs_preserve_identity_and_async_reactions() {
    compare_promises(
        &[
            "const f=async()=>{await a;return b;};const p=f();const use=async p=>await p;return await use(p);",
            "const f=async()=>{await a;return b;};const use=async p=>{await a;return await p;};return await use(f());",
            "const f=async()=>{await a;return b;};const p=f();const id=p=>p;return await id(p);",
            "const f=async()=>{await a;return b;};const p=f();const id=p=>{return p;};return await id(p);",
            "const f=async()=>{await a;return b;};const id=p=>p;const use=async p=>await p;return await use(id(f()));",
            "const f=async()=>{await a;return b;};const use=async p=>await p;const apply=async(fn,p)=>await fn(p);return await apply(use,f());",
            "const f=async()=>{await a;return b;};const g=async()=>{await b;return a;};const p=f();const q=g();const use=async p=>await p;return await(a?use:use)(a?p:q);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn pending_promise_inputs_preserve_captured_handles_and_call_snapshots() {
    compare_promises(
        &[
            "const f=async x=>{await x;return x;};const make=p=>()=>p;const get=make(f(b));return await get();",
            "const f=async x=>{await x;return x;};const make=p=>{return()=>p;};const get=make(f(b));return await get();",
            "const f=async x=>{await x;return x;};const make=p=>async()=>{await a;return await p;};const get=make(f(b));return await get();",
            "const f=async x=>{await x;return x;};let p=f(a);const use=async p=>{await b;return await p;};const q=use(p);p=f(b);return +(await q)*1000+(+(await p));",
            "const f=async x=>{await x;return x;};let p=f(a);const use=async p=>{await b;return await p;};const q=use(p,p=f(b));return +(await q)*1000+(+(await p));",
            "const f=async x=>{await x;return x;};let use=async p=>await p;const other=async p=>{await b;return a;};const q=use(f(b),(use=other));const r=use(f(a));return +(await q)*1000+(+(await r));",
            "const f=async()=>{await a;return b;};const id=p=>p;const forward=(fn,p)=>fn(p);return await forward(id,f());",
            "const f=async()=>{await a;return b;};const use=async p=>await p;const apply=async(fn,p)=>await fn(p);const relay=async(fn,p)=>await apply(fn,p);return await relay(use,f());",
            "const f=async()=>{await a;return b;};const id=p=>{let q=p;q=p;return q;};const p=id(f());await p;return await p;",
            "const f=async x=>{await x;return x;};const q=f(a);const use=async p=>{const old=p;p=q;await old;return await p;};return await use(f(b));",
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
fn pending_promise_inputs_preserve_primitive_rejection_and_late_handling() {
    compare_promises(
        &[
            "const f=async()=>{await a;throw b;};const use=async p=>{try{return await p;}catch(e){return e;}};return await use(f());",
            "const f=async()=>{await a;throw null;};const use=async p=>{await a;await b;try{return await p;}catch(e){return e;}};return await use(f());",
            "const f=async()=>{await a;throw false;};const id=p=>p;const use=async p=>{try{await p;}catch(e){}try{return await p;}catch(e){return e;}};return await use(id(f()));",
            "const f=async()=>{await a;throw void 0;};const use=async p=>await p;try{return await use(f());}catch(e){return e;}",
            "const f=async()=>{await a;throw b;};const use=async p=>await p;const apply=async(fn,p)=>await fn(p);try{return await apply(use,f());}catch(e){return e;}",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn pending_promise_inputs_keep_erasure_domains_and_resource_bounds() {
    let plain = "export async function run(a,b){const f=async x=>{await a;return x;};const use=async p=>await p;return await use(f(b));}";
    let typed = "export async function run(a:never,b:Missing):Wrong{const f=async(x:NotNumber):Wrong=>{await(a as any);return x as never;};const use=async(p:NotPromise):Wrong=>await(p as string);return await(use as never)(f(b as any));}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    let prefix = "const f=async()=>{await a;return b;};const p=f();";
    let params = (0..32)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let args = vec!["p"; 32].join(",");
    let wide = format!(
        "{prefix}const use=async({params})=>{{await p0;return await p31;}};return await use({args});"
    );
    compare_promises(&[&wide], &[(4.0, 2.0), (0.0, -0.0)]);
    for count in [1000, 1001] {
        let params = (0..count)
            .map(|i| format!("p{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let args = vec!["p"; count].join(",");
        let source = source_for(&format!(
            "{prefix}const use=async({params})=>await p{};return await use({args});",
            count - 1
        ));
        for ts in [false, true] {
            let result = compile_async(&source, ts);
            if count == 1000 {
                result.unwrap();
            } else {
                assert!(
                    result
                        .err()
                        .expect("1001 parameters reject")
                        .contains("1000")
                );
            }
        }
    }
    for body in [
        "const f=async()=>{await a;return b;};const use=async p=>await p;use(f());return await use(a);",
        "const f=async()=>{await a;return b;};const use=async p=>{p=a;return await p;};return await use(f());",
        "const f=async()=>{await a;return b;};const use=async p=>await p;return await use(f(),{});",
        "const f=async()=>{await a;return b;};const use=async p=>+p;return await use(f());",
        "const f=async()=>{await a;return b;};const use=async p=>await p;return await use();",
        "const f=async()=>{await a;return b;};const use=async p=>await p;return await use?.(f());",
        "const f=async()=>{await a;return b;};const use=async p=>await p;return await use(...a);",
        "const f=async()=>{await a;return b;};const use=async p=>await p;return await use(Promise.resolve(a));return await use(f());",
    ] {
        for ts in [false, true] {
            assert!(compile_async(&source_for(body), ts).is_err(), "{body}");
        }
    }
}
