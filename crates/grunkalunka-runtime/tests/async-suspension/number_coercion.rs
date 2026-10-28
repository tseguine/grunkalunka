use super::*;

#[test]
fn numeric_coercion_keeps_promise_and_selected_primitive_values_distinct() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);return +p;",
            "const f=async()=>{await a;return b;};const use=async p=>+p;return await use(f());",
            "let f=async()=>{await a;return b;};return +f();",
            "const id=p=>p;return +id(Promise.resolve(a));",
            "let f=()=>Promise.resolve(a);return +f();",
            "const p=Promise.resolve(a);const x=a?p:b;return +x;",
            "const p=Promise.resolve(a);const plus=x=>+x;plus(p);return plus(b);",
            "let x=a?Promise.resolve(a):b;return +x;",
            "const p=Promise.resolve(a);const choose=()=>a?p:b;return +choose();",
            "const f=async()=>{await a;return b;};const p=f();return +p;",
            "const p=Promise.resolve(a);return +(a?p:b);",
            "const p=Promise.resolve(a);let x=a?p:b;return +x;",
            "const p=Promise.resolve(a);const f=x=>+x;f(p);return f(b);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
    let mut bodies = Vec::new();
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
        for operator in ["+", "-", "~"] {
            bodies.push(format!(
                "const p=Promise.resolve(a);const x=a?p:{value};return {operator}x;"
            ));
        }
    }
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[
            (4.0, 2.0),
            (4.0, -0.0),
            (0.0, -0.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn numeric_coercion_handles_arithmetic_bitwise_and_shift_operands() {
    let mut bodies = Vec::new();
    for operator in ["-", "*", "/", "%", "&", "|", "^", "<<", ">>", ">>>"] {
        bodies.push(format!(
            "const p=Promise.resolve(a);return (a?p:b){operator}b;"
        ));
        bodies.push(format!(
            "const p=Promise.resolve(a);return b{operator}(a?p:b);"
        ));
        bodies.push(format!(
            "const p=Promise.resolve(a);return (a?p:b){operator}(b?p:a);"
        ));
    }
    bodies.push("const p=Promise.resolve(a);return (+p)+b;".into());
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[
            (4.0, 2.0),
            (4.0, -0.0),
            (0.0, -0.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn numeric_coercion_handles_math_and_primitive_compound_rhs() {
    let mut bodies = Vec::new();
    for method in ["abs", "sqrt", "floor", "ceil", "trunc"] {
        bodies.push(format!(
            "const p=Promise.resolve(a);return Math.{method}(p);"
        ));
        bodies.push(format!(
            "const p=Promise.resolve(a);return Math.{method}(a?p:b);"
        ));
        bodies.push(format!(
            "const p=Promise.resolve(a);const v=Math.{method}(a?p:b,await(a=b));return v===v?v:a;"
        ));
    }
    for operator in [
        "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=", ">>>=",
    ] {
        bodies.push(format!(
            "const p=Promise.resolve(b);let x=a;x{operator}(a?p:b);return x;"
        ));
        bodies.push(format!(
            "const p=Promise.resolve(b);let x=null;x{operator}(a?p:b);return x;"
        ));
    }
    bodies.push("const p=Promise.resolve(a);let x=a;x|=(x=b,b?p:1);return x;".into());
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[
            (4.0, 2.0),
            (4.0, -0.0),
            (0.0, -0.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn numeric_coercion_keeps_call_capture_snapshot_and_operand_order() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);let coerce=a?x=>+x:x=>-x;coerce(p);return coerce(b);",
            "const p=Promise.resolve(a);const coerce=x=>~x;const call=(fn,x)=>fn(x);call(coerce,p);return call(coerce,b);",
            "const p=Promise.resolve(a);const make=()=>()=>a?p:b;const read=make();return +read();",
            "const p=Promise.resolve(a);let x=a?p:b;const read=()=>x;const old=+read();x=p;await b;return old;",
            "const p=Promise.resolve(a);let x=a?p:b;return x-(await(x=p,b));",
            "const p=Promise.resolve(a);let n=0;const read=()=>{n++;return a?p:b;};const v=+read();return v===v?v:n;",
            "const p=Promise.resolve(a);try{return +(a?(await Promise.reject(b),p):null);}catch(e){return e;}",
            "let x=a;try{x-=(await Promise.reject(b));}catch(e){return x;}return b;",
        ],
        &[
            (4.0, 2.0),
            (4.0, -0.0),
            (0.0, -0.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn numeric_coercion_does_not_settle_promises_and_preserves_error_values() {
    compare_promises(
        &[
            "const p=Promise.reject(a);const x=+p;try{await p;}catch(e){}return x;",
            "const f=async()=>{await a;throw b;};const p=f();const x=+p;try{await p;}catch(e){}return x;",
            "const p=Promise.resolve(a);try{return +p;}finally{await b;}",
            "const p=Promise.resolve(a);const f=async x=>+x;await f(p);return await f(b);",
            "const p=Promise.resolve(a);const f=async()=>{await b;return +(a?p:b);};return await f();",
        ],
        &[
            (4.0, 2.0),
            (4.0, -0.0),
            (0.0, -0.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    let cycle = "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();";
    compare_async_sources(
        &[format!(
            "export async function run(a,b){{{cycle}try{{await p;}}catch(e){{const q=Promise.resolve(a);let x=a?q:e;return +x;}}}}"
        )],
        &[(4.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn numeric_coercion_keeps_erasure_budgets_and_non_numeric_boundaries() {
    let plain = source_for("const p=Promise.resolve(a);const f=x=>+x;f(p);return f(b);");
    let typed = plain
        .replace("run(a,b)", "run(a:Missing,b:never):Wrong")
        .replace("f=x=>+x", "f=(x:Bad):Nope=>+(x as never)");
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
    for count in [0, 32] {
        let body = format!(
            "const p=Promise.resolve(a);{}return +(a?p:b);",
            "await b;".repeat(count)
        );
        compare_promises(&[&body], &[(4.0, 2.0), (0.0, -0.0)]);
    }
    let over = source_for(&format!(
        "const p=Promise.resolve(a);{}return +p;",
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
            "let x=a?Promise.resolve(a):b;return x+b;",
            "let x=a?Promise.resolve(a):b;x-=b;return x;",
            "let x=a?Promise.resolve(a):b;x++;return x;",
            "const p=Promise.resolve(a);p.valueOf=()=>b;return +p;",
            "return +(a?Promise.resolve(b):'1');",
            "const p=Promise.resolve(a);return p**b;",
        ] {
            assert!(compile_async(&source_for(body), ts).is_err(), "{body}");
        }
    }
}
