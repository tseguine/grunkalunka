use super::*;

#[test]
fn promise_relational_compares_builtin_strings_instead_of_identity() {
    compare_promises(
        &["const p=Promise.resolve(a);return p<p;"],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
    let mut bodies = Vec::new();
    for op in ["<", "<=", ">", ">="] {
        for pair in ["p", "q", "f()", "a?p:q"] {
            bodies.push(format!(
                "const p=Promise.resolve(a),q=Promise.resolve(b);const f=async()=>{{await a;return b;}};return p{op}({pair});"
            ));
        }
    }
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn promise_relational_keeps_each_selected_primitive_and_nan_result() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);const x=a?p:b;return x<b;",
            "const p=Promise.resolve(a);return (a?p:b)<a;",
            "const p=Promise.resolve(a);const x=a?p:b;return +(x<=p)+(x<b);",
            "const p=Promise.resolve(a);return Math.abs(p)<=p;",
            "const p=Promise.resolve(a);return (a?p:b)<+(b?p:a);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
    let mut bodies = Vec::new();
    for op in ["<", "<=", ">", ">="] {
        for value in [
            "b",
            "null",
            "undefined",
            "false",
            "true",
            "-0",
            "0/0",
            "a<b",
        ] {
            for expression in [
                format!("p{op}({value})"),
                format!("({value}){op}p"),
                format!("(a?p:{value}){op}b"),
                format!("b{op}(a?p:{value})"),
                format!("(a?p:{value}){op}(b?q:a)"),
            ] {
                bodies.push(format!(
                    "const p=Promise.resolve(a),q=Promise.resolve(b);return {expression};"
                ));
            }
        }
    }
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn promise_relational_keeps_private_calls_callbacks_and_captured_variants() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);const cmp=(x,y)=>x<=y;cmp(p,p);return cmp(a,b);",
            "const p=Promise.resolve(a);let cmp=a?(x,y)=>x<y:(x,y)=>x>=y;cmp(p,p);return cmp(a,p);",
            "const p=Promise.resolve(a);const cmp=(x,y)=>x<=y;const call=(fn,x,y)=>fn(x,y);call(cmp,p,b);return call(cmp,a,p);",
            "const p=Promise.resolve(a);const make=()=>()=>a?p:b;const read=make();return read()>=p;",
            "const p=Promise.resolve(a);let x=a?p:b;const read=()=>x;const old=read();x=p;await b;return old<=read();",
            "const p=Promise.resolve(a);const cmp=async(x,y)=>{await b;return x<=y;};await cmp(a,b);return await cmp(p,p);",
            "const p=Promise.resolve(a);let x=a?p:b;const old=x;x=b;return (old<=p)&&(x<p);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn promise_relational_evaluates_operands_once_in_order_across_await_and_failure() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);let n=0;const take=x=>{n=n*10+x;return p;};const result=take(1)<=take(2);return result?n:-n;",
            "const p=Promise.resolve(a);let x=p;return x<=(x=b,p);",
            "const p=Promise.resolve(a);let x=a?p:b;return x<=(await(x=b,p));",
            "const p=Promise.resolve(a);let x=a?p:b;return (await(x=p,b))<=x;",
            "const p=Promise.resolve(a);let n=0;try{return (n=1,p)<=(await Promise.reject(b));}catch(e){return n;}",
            "const p=Promise.resolve(a);let n=0;try{return (await Promise.reject(b))<=(n=1,p);}catch(e){return n;}",
            "const p=Promise.reject(b);const q=Promise.resolve(a);const result=p<=q;try{await p;}catch(e){}return result;",
            "const f=async()=>{await a;throw b;};const p=f();const q=Promise.resolve(a);const result=(a?p:b)>=q;try{await p;}catch(e){}return result;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn promise_relational_orders_builtin_promise_and_intrinsic_error_strings() {
    let cycle = "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();";
    let mut bodies = Vec::new();
    for op in ["<", "<=", ">", ">="] {
        for expression in [
            format!("p{op}e"),
            format!("e{op}p"),
            format!("(a?p:e){op}e"),
            format!("e{op}(a?p:e)"),
            format!("(a?p:e){op}(b?p:e)"),
        ] {
            bodies.push(format!(
                "{cycle}try{{await p;}}catch(e){{return {expression};}}"
            ));
        }
    }
    bodies.push(format!(
        "{cycle}try{{await p;}}catch(e){{const cmp=(x,y)=>x<=y;cmp(p,e);return cmp(e,p);}}"
    ));
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn promise_relational_keeps_erasure_source_budgets_and_conversion_boundaries() {
    let plain =
        source_for("const p=Promise.resolve(a);const cmp=(x,y)=>x<=y;cmp(a,b);return cmp(p,p);");
    let typed = plain
        .replace("run(a,b)", "run(a:Missing,b:never):Wrong")
        .replace(
            "cmp=(x,y)=>x<=y",
            "cmp=(x:Wrong,y:never):Bad=>(x as never)<=(y as any)",
        );
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
    for count in [0, 32] {
        let body = format!(
            "const p=Promise.resolve(a);{}return (a?p:b)<=p;",
            "await b;".repeat(count)
        );
        compare_promises(&[&body], &[(4.0, 2.0), (0.0, -0.0)]);
    }
    let over = source_for(&format!(
        "const p=Promise.resolve(a);{}return p<=p;",
        "await b;".repeat(33)
    ));
    for ts in [false, true] {
        assert!(
            compile_async(&over, ts)
                .err()
                .unwrap()
                .contains("32 await sites")
        );
        for body in [
            "const p=Promise.resolve(a);return p+b;",
            "const p=Promise.resolve(a);p.valueOf=()=>b;return p<b;",
            "const p=Promise.resolve(a);return p<'z';",
            "const p=Promise.resolve(a);return p<{};",
            "const p=Promise.resolve(a);return p<(()=>b);",
            "const p=Promise.resolve(a);return p<1n;",
        ] {
            assert!(compile_async(&source_for(body), ts).is_err(), "{body}");
        }
    }
}
