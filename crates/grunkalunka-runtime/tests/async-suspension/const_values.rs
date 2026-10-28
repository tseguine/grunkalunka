use super::*;

#[test]
fn immutable_mixed_values_keep_the_selected_identity() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);const x=a?p:b;return x===p;",
            "const p=Promise.resolve(a);const x=a?p:b;const y=x;await b;return y!==p;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn immutable_mixed_values_are_raw_in_tests_and_resolved_by_async_consumers() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);const x=a?p:b;return x;",
            "const p=Promise.resolve(a);const x=a?p:b;return await x;",
            "const p=Promise.resolve(false);const x=a?p:b;return x?1:2;",
            "const p=Promise.resolve(null);const x=a?p:b;if(x)return a;return b;",
            "const p=Promise.resolve(a);const x=a?p:b;return !x;",
            "const p=Promise.resolve(a);const x=a?p:null;return x??b;",
            "const p=Promise.resolve(a);const x=a?p:b;return x||b;",
            "const p=Promise.resolve(a);const x=a?p:b;return x&&b;",
            "const p=Promise.resolve(a);const x=a?p:b;x;return b;",
            "const x=a?Promise.reject(b):b;try{return await x;}catch(e){return e;}",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn immutable_mixed_values_survive_captures_and_original_binding_mutation() {
    compare_promises(
        &[
            "let p=Promise.resolve(a);const x=a?p:b;const q=p;p=Promise.resolve(b);await b;return x===q;",
            "const p=Promise.resolve(a);const x=a?p:b;const eq=()=>x===p;await b;return eq();",
            "const p=Promise.resolve(a);const x=a?p:b;let eq=a?()=>x===p:()=>x!==p;await b;return eq();",
            "const p=Promise.resolve(a);const x=a?p:b;const read=async()=>{await b;return x;};return await read();",
            "const p=Promise.resolve(a);const make=()=>{const x=a?p:b;return ()=>x===p;};const eq=make();await b;return eq();",
            "const p=Promise.resolve(a);const x=a?p:b;let n=0;const q=(x===p)?(n=1,p):(n=2,p);return x===(await b,q)?n:-n;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn immutable_mixed_values_compose_with_later_selections_and_lexical_rejections() {
    compare_promises(
        &[
            "const p=Promise.resolve(false);const x=a?p:b;return await(x||b);",
            "const p=Promise.resolve(null);const x=a?p:b;return await(x&&b);",
            "const p=Promise.resolve(a);const x=a?p:null;return await(x??b);",
            "const p=Promise.resolve(a);const x=a?p:b;return await(b?x:null);",
            "const p=Promise.resolve(a);const x=a?p:b;return await(await b,x);",
            "const p=Promise.resolve(a);const x=a?p:b;const y=x||null;return await y;",
            "const x=a?Promise.reject(b):b;const read=async()=>{try{return await x;}catch(e){return a;}};return await read();",
            "const x=a?Promise.reject(b):b;const read=async()=>{try{return x;}catch(e){return a;}};try{return await read();}catch(e){return b;}",
            "let n=0;const f=async()=>{await a;return b;};const x=a?(n=1,f()):(n=2,b);await x;return n;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn immutable_mixed_values_preserve_each_primitive_kind_and_equality_rule() {
    let mut bodies = Vec::new();
    for primitive in ["b", "false", "true", "null", "undefined", "a<b"] {
        for op in ["===", "!==", "==", "!="] {
            for right in ["p", "b", "false", "null", "undefined"] {
                bodies.push(format!(
                    "const p=Promise.resolve(a);const x=a?p:{primitive};const y=x;await b;return y{op}{right};"
                ));
            }
        }
        bodies.push(format!(
            "const p=Promise.resolve(a);const x=a?p:{primitive};return await x;"
        ));
        bodies.push(format!(
            "const p=Promise.resolve(a);const x=a?p:{primitive};return x;"
        ));
    }
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0), (0.0, 1.0), (f64::NAN, f64::NAN)],
    );
    let cycle = "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();";
    compare_async_sources(
        &[
            format!(
                "export async function run(a,b){{{cycle}try{{await p;}}catch(e){{const q=Promise.resolve(e);const x=a?q:e;const eq=()=>x===e;await b;return eq();}}}}"
            ),
            format!(
                "export async function run(a,b){{{cycle}try{{await p;}}catch(e){{const q=Promise.resolve(e);const x=a?q:e;const y=await x;return y===e;}}}}"
            ),
        ],
        &[(4.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn immutable_mixed_values_preserve_erasure_limits_and_remaining_guards() {
    let plain = source_for("const p=Promise.resolve(a);const x=a?p:b;const y=x;return await y;");
    let typed = plain
        .replace("run(a,b)", "run(a:never,b:Wrong):Missing")
        .replace(
            "const x=a?p:b",
            "const x:Bad=(a as never)?(p as Wrong):(b as Missing)",
        )
        .replace("const y=x", "const y:Wrong=(x as never)");
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
    for count in [0, 31] {
        let body = format!(
            "const p=Promise.resolve(a);const x=a?p:b;{}return await x;",
            "await b;".repeat(count)
        );
        compare_promises(&[&body], &[(4.0, 2.0), (0.0, -0.0)]);
    }
    let too_many = source_for(&format!(
        "const p=Promise.resolve(a);const x=a?p:b;{}return await x;",
        "await b;".repeat(32)
    ));
    assert!(
        compile_async(&too_many, false)
            .err()
            .unwrap()
            .contains("32 await sites")
    );
    for ts in [false, true] {
        let body = format!(
            "const p=Promise.resolve(a);const x=a?p:b;{}return x===p;",
            (0..126)
                .map(|i| format!("const v{i}=x;"))
                .collect::<String>()
        );
        assert!(compile_async(&source_for(&body), ts).is_ok());
        let too_many = body.replace("return x===p;", "const extra=x;return x===p;");
        assert!(
            compile_async(&source_for(&too_many), ts)
                .err()
                .unwrap()
                .contains("128 source locals")
        );
    }
    compare_promises(
        &["const p=Promise.resolve(a);let x=a?p:b;return x===p;"],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
    compare_promises(
        &["const p=Promise.resolve(a);const x=a?p:b;const eq=y=>y===p;eq(p);return eq(x);"],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
    for body in [
        "const p=Promise.resolve(a);const x=a?p:b;x=b;return x;",
        "const p=Promise.resolve(a);const x=a?p:b;return x<b;",
    ] {
        assert!(compile_async(&source_for(body), false).is_err(), "{body}");
    }
}
