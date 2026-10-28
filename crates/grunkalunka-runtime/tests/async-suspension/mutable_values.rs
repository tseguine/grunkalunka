use super::*;

#[test]
fn mutable_mixed_values_keep_raw_kind_across_reassignment_and_aliases() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);let x=a?p:b;const y=x;x=b;return y===p;",
            "const p=Promise.resolve(a);let x=b;x=p;return x===p;",
            "const p=Promise.resolve(a);let x=p;x=b;return x===b;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn mutable_mixed_assignment_results_are_consumed_after_the_store() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);let x=b;return(x=p);",
            "const p=Promise.resolve(a);let x=b;return await(x=p);",
            "const p=Promise.resolve(false);let x=b;return(x=p)?a:b;",
            "const p=Promise.resolve(a);let x=p;return(x=b)===b;",
            "const p=Promise.resolve(a);let x;const y=x;x=p;return y===undefined;",
            "const p=Promise.resolve(a);let x=a?p:b;let y=x;x=b;await a;return y===p;",
            "const p=Promise.resolve(a);let x=a?p:b;const read=()=>{x=b;return x===b;};await a;return read();",
            "const p=Promise.resolve(a);let x=a?p:b;const read=async()=>{await b;return(x=p);};return await read();",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn mutable_mixed_logical_assignments_and_error_identity_keep_raw_values() {
    let mut bodies = Vec::new();
    for operator in ["&&=", "||=", "??="] {
        for initial in ["a?p:b", "a?p:null", "a?p:false", "a?p:undefined"] {
            bodies.push(format!(
                "const p=Promise.resolve(a);let x={initial};const old=x;let n=0;const y=(x{operator}(n=1,await b,p));return y===old?n:-n;"
            ));
            bodies.push(format!(
                "const p=Promise.resolve(a);let x={initial};const y=(x{operator}b);return y===x;"
            ));
        }
    }
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
    let cycle = "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();";
    compare_async_sources(
        &[
            format!(
                "export async function run(a,b){{{cycle}try{{await p;}}catch(e){{const q=Promise.resolve(e);let x=q;const old=x;x=e;return x===e&&old===q;}}}}"
            ),
            format!(
                "export async function run(a,b){{{cycle}try{{await p;}}catch(e){{let x=e;const old=x;x=Promise.resolve(e);return(await x)===old;}}}}"
            ),
        ],
        &[(4.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn mutable_mixed_values_preserve_tags_payloads_and_captured_cells() {
    let mut bodies = Vec::new();
    for value in [
        "b",
        "null",
        "undefined",
        "false",
        "true",
        "a<b",
        "0/0",
        "-0",
        "q",
    ] {
        for initial in ["p", "b", "undefined"] {
            bodies.push(format!(
                "const p=Promise.resolve(a),q=Promise.resolve(b);let x={initial};const old=x;const same=()=>x===q;x={value};await b;return same();"
            ));
            bodies.push(format!(
                "const p=Promise.resolve(a),q=Promise.resolve(b);let x={initial};const read=async()=>{{await b;return x;}};x=p;x={value};return await read();"
            ));
        }
    }
    bodies.extend([
        "let x;const old=x;x=Promise.resolve(a);return old===undefined;".into(),
        "let x=b;const old=x;const read=async()=>x;x=Promise.resolve(a);const y=x;x=b;await a;return (await read())===old;".into(),
        "let x=b;const make=()=>()=>x===b;const same=make();x=Promise.resolve(a);await b;x=b;return same();".into(),
        "const p=Promise.resolve(a);let x=p;let y=x;x=b;await b;return y===p;".into(),
    ]);
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn mutable_mixed_assignments_keep_rhs_snapshots_effects_and_rejection_scopes() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);let x=a?p:b;let n=0;const y=(x||=(n=1,await b,x=b,p));return y===x?n:-n;",
            "const p=Promise.resolve(a);let x=a?p:b;let n=0;const y=(x&&=(n=1,await b,x=b,p));return y===x?n:-n;",
            "const p=Promise.resolve(a);let x=a?p:null;let n=0;const y=(x??=(n=1,await b,x=b,p));return y===x?n:-n;",
            "const p=Promise.resolve(a);let x=a?p:b;const same=x===(await b,x=b,p);return same;",
            "const p=Promise.resolve(a);let x=a?p:b;const old=x;try{x=await Promise.reject(b);}catch(e){return x===old;}return false;",
            "const p=Promise.resolve(a);let x=b;try{x=(x=p,await Promise.reject(b));}catch(e){return x===p;}return false;",
            "const p=Promise.resolve(a);let x=a?p:null;try{x??=(x=b,await Promise.reject(a));}catch(e){return x===b;}return x===p;",
            "const p=Promise.resolve(a);let x=a?p:null;const old=x;try{x??=await Promise.reject(b);}catch(e){return x===old;}return x===old;",
            "const p=Promise.resolve(a);let x=a?p:b;const read=async()=>x;const result=read();x=b;return await result;",
            "let x=a;const read=async()=>{try{return(x=Promise.reject(b));}catch(e){return a;}};try{return await read();}catch(e){return b;}",
            "let x=a;const read=async()=>{try{return await(x=Promise.reject(b));}catch(e){return a;}};return await read();",
            "let x=a;const read=async()=>{try{return(x||=Promise.reject(b));}catch(e){return a;}};try{return await read();}catch(e){return b;}",
            "let x=a;const read=async()=>{try{return await(x&&=Promise.reject(b));}catch(e){return a;}};return await read();",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn mutable_mixed_values_keep_erasure_limits_and_unrepresented_boundaries() {
    let plain = source_for(
        "const p=Promise.resolve(a);let x=a?p:b;const old=x;x||=(await b,p);return old===x;",
    );
    let typed = plain
        .replace("run(a,b)", "run(a:never,b:Missing):Wrong")
        .replace(
            "let x=a?p:b",
            "let x:Bad=(a as never)?(p as Wrong):(b as Missing)",
        )
        .replace("x||=(await b,p)", "x||=((await b) as never,p as Wrong)");
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
    for count in [0, 31] {
        let body = format!(
            "let x=b;x=Promise.resolve(a);{}return await x;",
            "await b;".repeat(count)
        );
        compare_promises(&[&body], &[(4.0, 2.0), (0.0, -0.0)]);
    }
    let too_many = source_for(&format!(
        "let x=b;x=Promise.resolve(a);{}return await x;",
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
            "let x=a?Promise.resolve(a):b;{}return x===b;",
            (0..127)
                .map(|i| format!("let v{i}=x;v{i}=b;"))
                .collect::<String>()
        );
        assert!(compile_async(&source_for(&body), ts).is_ok());
        let over = body.replace("return x===b;", "let extra=x;return x===b;");
        assert!(
            compile_async(&source_for(&over), ts)
                .err()
                .unwrap()
                .contains("128 source locals")
        );
        for body in [
            "const p=Promise.resolve(a);const x=a?p:b;x=b;return x;",
            "let x=a?Promise.resolve(a):b;x+=b;return x;",
            "let x=a?Promise.resolve(a):b;x++;return x;",
            "let x=a?Promise.resolve(a):b;x=()=>b;return x;",
        ] {
            assert!(compile_async(&source_for(body), ts).is_err(), "{body}");
        }
    }
}
