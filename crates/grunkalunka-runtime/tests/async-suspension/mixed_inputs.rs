use super::*;

#[test]
fn mixed_private_inputs_keep_the_actual_kind_in_known_calls() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);const same=x=>x===p;same(p);return same(b);",
            "const p=Promise.resolve(a);const same=x=>x===p;same(b);return same(p);",
            "const p=Promise.resolve(a);const same=x=>x===p;return same(a?p:b);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn mixed_private_inputs_cross_runtime_and_callback_argument_frames() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);let same=a?x=>x===p:x=>x!==p;same(p);return same(b);",
            "const p=Promise.resolve(a);let same=a?x=>x===p:x=>x!==p;return same(a?p:b);",
            "const p=Promise.resolve(a);const same=x=>x===p;const call=(fn,x)=>fn(x);call(same,p);return call(same,b);",
            "const p=Promise.resolve(a);let same=a?x=>x===p:x=>x!==p;const call=(fn,x)=>fn(x);return call(same,a?p:b);",
            "const p=Promise.resolve(a);const choose=x=>x?1:2;choose(p);return choose(b);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn mixed_private_inputs_preserve_async_await_and_return_reactions() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);const read=async x=>{await b;return await x;};await read(p);return await read(b);",
            "const p=Promise.resolve(a);const read=async x=>{await b;return x;};await read(b);return await read(p);",
            "const p=Promise.resolve(a);let read=a?async x=>{await b;return x;}:async x=>await x;return await read(a?p:b);",
            "const p=Promise.resolve(a);const read=async x=>await x;const call=(fn,x)=>fn(x);return await call(read,a?p:b);",
            "const p=Promise.resolve(a);const read=async x=>{await a;return x;};let call=a?(fn,x)=>fn(x):(fn,x)=>fn(x);await call(read,p);return await call(read,b);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn mixed_private_inputs_preserve_argument_width_order_and_callee_snapshots() {
    compare_promises(
        &[
            "const p=Promise.resolve(a),q=Promise.resolve(b);const g=x=>x===q?1:2;let f=a?(n,x,y,fn,t)=>(x===p?n:(x===b?y:0))+fn(t):(n,x,y,fn,t)=>(x===p?y:(x===b?n:0))-fn(t);f(1,p,b,g,q);return f(2,b,a,g,b);",
            "const p=Promise.resolve(a);let x=a?p:b;const same=(left,right)=>left===right;same(p,p);return same(x,(await a,x=b,p));",
            "const p=Promise.resolve(a);let f=a?x=>x===p:x=>x!==p;return f((f=x=>x===p,a?p:b));",
            "const p=Promise.resolve(a);let n=0;const f=(x,y)=>x===y;f(p,p);const same=f((n=1,a?p:b),(n=n*10+2,await a,p),(n=n*10+3,b));return same?n:-n;",
            "const p=Promise.resolve(a);const f=x=>x===p;f(p);const child=async()=>{await a;return b;};return f(b,child());",
            "const p=Promise.resolve(a);const f=async x=>await x;const call=async(fn,x)=>await fn(x);await call(f,p);return await call(f,b);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn mixed_private_inputs_preserve_primitive_tags_factory_cells_and_errors() {
    let mut bodies = Vec::new();
    for value in [
        "b",
        "null",
        "undefined",
        "false",
        "true",
        "a<b",
        "-0",
        "0/0",
    ] {
        bodies.push(format!(
            "const p=Promise.resolve(a);let same=a?x=>x===p:x=>x!==p;same(p);return same({value});"
        ));
        bodies.push(format!(
            "const p=Promise.resolve(a);const read=async x=>{{const old=x;x=b;await a;return old;}};await read(p);return await read({value});"
        ));
    }
    bodies.extend([
        "const p=Promise.resolve(a);const make=x=>()=>x===p;const first=make(p),second=make(b);await a;return first()!==second();".into(),
        "const p=Promise.resolve(a);let make=a?n=>x=>x===p?n:0:n=>x=>x!==p?n:0;let f=make(a);f(p);return f(b);".into(),
        "const p=Promise.resolve(a);const f=x=>{const old=x;x=b;return old===p;};f(p);return f(b);".into(),
        "const p=Promise.resolve(a);const f=x=>{x=b;return x===b;};f(p);return f(b);".into(),
    ]);
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
    let cycle = "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();";
    compare_async_sources(
        &[
            format!(
                "export async function run(a,b){{{cycle}try{{await p;}}catch(e){{const q=Promise.resolve(e);const same=x=>x===e;same(q);return same(e);}}}}"
            ),
            format!(
                "export async function run(a,b){{{cycle}try{{await p;}}catch(e){{const q=Promise.resolve(e);const read=async x=>x;await read(q);return(await read(e))===e;}}}}"
            ),
        ],
        &[(4.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn mixed_private_inputs_preserve_erasure_limits_and_private_return_boundaries() {
    let plain = source_for(
        "const p=Promise.resolve(a);let same=a?x=>x===p:x=>x!==p;same(p);return same(b);",
    );
    let typed = plain
        .replace("run(a,b)", "run(a:never,b:Missing):Wrong")
        .replace(
            "a?x=>x===p:x=>x!==p",
            "(a as never)?(x:Wrong):Bad=>x===p:(x:Missing):Bad=>x!==p",
        );
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
    for count in [0, 32] {
        let body = format!(
            "const p=Promise.resolve(a);const same=x=>x===p;same(p);{}return same(b);",
            "await a;".repeat(count)
        );
        compare_promises(&[&body], &[(4.0, 2.0), (0.0, -0.0)]);
    }
    let over = source_for(&format!(
        "const p=Promise.resolve(a);const same=x=>x===p;same(p);{}return same(b);",
        "await a;".repeat(33)
    ));
    assert!(
        compile_async(&over, false)
            .err()
            .unwrap()
            .contains("32 await sites")
    );
    for count in [17, 32] {
        let params = (0..count)
            .map(|i| format!("p{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let promises = vec!["p"; count].join(",");
        let primitives = vec!["b"; count].join(",");
        let body = format!(
            "const p=Promise.resolve(a);let use=a?async({params})=>{{await a;return await p0;}}:async({params})=>{{await a;return await p0;}};await use({promises});return await use({primitives});"
        );
        compare_promises(&[&body], &[(4.0, 2.0), (0.0, -0.0)]);
    }
    for body in [
        "const p=Promise.resolve(a);const same=x=>x===p;same(p);return same(()=>a);",
        "const p=Promise.resolve(a);const same=x=>x===p;same(p);return same({});",
    ] {
        for ts in [false, true] {
            assert!(compile_async(&source_for(body), ts).is_err(), "{body}");
        }
    }
}
