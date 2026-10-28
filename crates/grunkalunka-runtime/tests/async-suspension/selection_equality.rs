use super::*;

#[test]
fn selected_promise_or_primitive_equality_uses_the_selected_kind() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);return (a?p:b)===p;",
            "const p=Promise.resolve(a);return (a?p:b)!==p;",
            "const p=Promise.resolve(a);return p==(a?p:b);",
            "const p=Promise.resolve(a);return p!=(a?p:b);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn selected_equality_preserves_strict_and_loose_primitive_rules_in_both_orders() {
    let mut bodies = Vec::new();
    for op in ["===", "!==", "==", "!="] {
        for left in [
            "(a?p:b)",
            "(a?p:null)",
            "(a?p:undefined)",
            "(a?p:false)",
            "(a?p:a<b)",
        ] {
            for right in ["p", "q", "0", "false", "(b?q:a)"] {
                for expression in [format!("{left}{op}{right}"), format!("{right}{op}{left}")] {
                    bodies.push(format!(
                        "const p=Promise.resolve(a),q=Promise.resolve(b);return {expression};"
                    ));
                }
            }
        }
    }
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[
            (4.0, 2.0),
            (0.0, 0.0),
            (0.0, 1.0),
            (0.0, -0.0),
            (f64::NAN, f64::NAN),
        ],
    );
}

#[test]
fn selected_equality_supports_nested_logical_comma_and_helper_expressions() {
    let bodies = [
        "return ((a?p:b)||q)===p;",
        "return ((a&&p)||b)!==q;",
        "return (a?p:null)===(b?q:undefined);",
        "return (a?p:(b?q:null))==(b?q:p);",
        "return (await a,a?p:b)===(await b,b?q:a);",
        "const same=()=>(a?p:b)===p;return same();",
        "let same=a?()=>(a?p:b)===p:()=>(a?p:b)!==q;return same();",
        "const same=()=>(a?p:b)==p;const use=fn=>fn();return use(same);",
    ]
    .map(|tail| format!("const p=Promise.resolve(a),q=Promise.resolve(b);{tail}"));
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn selected_equality_evaluates_operands_once_and_retains_left_snapshots() {
    compare_promises(
        &[
            "let n=0;const p=Promise.resolve(a),q=Promise.resolve(b);const same=(a?(n=1,p):(n=2,b))===(n=n*10+3,q);return same?n:-n;",
            "let p=Promise.resolve(a);const q=p;return (a?p:b)===(await b,p=Promise.resolve(b),q);",
            "const p=Promise.resolve(a);let q=p;const change=async()=>{q=Promise.resolve(b);await a;return b;};return (a?q:b)===(await change(),q);",
            "const p=Promise.resolve(a);let n=0;const take=x=>{n=n*10+x;return p;};const same=(a?take(1):b)===(await b,take(2));return same?n:-n;",
            "const p=Promise.resolve(a);try{return (a?p:b)===(await Promise.reject(b));}catch(e){return e;}",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn selected_equality_keeps_intrinsic_error_identity_separate_from_promises() {
    let cycle = "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();";
    let mut bodies = Vec::new();
    for op in ["===", "!==", "==", "!="] {
        for expression in [
            format!("(a?q:e){op}e"),
            format!("e{op}(a?q:e)"),
            format!("((a?q:e)&&e){op}e"),
            format!("(a?q:e){op}q"),
        ] {
            bodies.push(format!(
                "{cycle}try{{await p;}}catch(e){{const q=Promise.resolve(e);return {expression};}}"
            ));
        }
    }
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn selected_equality_cannot_confuse_a_promise_handle_with_a_number() {
    use grunkalunka_runtime::PromiseNotification;
    let make = |op| {
        source_for(&format!(
            "const p=Promise.reject(b);await a;const same=(b?p:a){op}a;try{{await p;}}catch(e){{}}return same;"
        ))
    };
    let wasm = compile_async(&make("=="), false).unwrap().wasm;
    let mut run = AsyncInstance::new(&wasm, 0.0, 2.0).unwrap();
    while run.run_one_job().unwrap() {}
    let events = run.take_promise_notifications().unwrap();
    let [
        PromiseNotification::Rejected { id, .. },
        PromiseNotification::Handled { id: handled },
    ] = events.as_slice()
    else {
        panic!("expected rejected/handled identity pair: {events:?}");
    };
    assert_eq!(id, handled);
    compare_async_sources(
        &["===", "!==", "==", "!="].map(make),
        &[(f64::from(*id), 2.0), (f64::from(*id), 0.0)],
    );
}

#[test]
fn selected_equality_preserves_erasure_budgets_and_remaining_storage_guards() {
    let plain = source_for("const p=Promise.resolve(a);return (a?p:b)===p;");
    let typed = plain
        .replace("run(a,b)", "run(a:never,b:Missing):Wrong")
        .replace("(a?p:b)", "((a as never)?(p as Wrong):(b as Bad))");
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
    for count in [0, 32] {
        let body = format!(
            "const p=Promise.resolve(a);{}return (a?p:b)==p;",
            "await a;".repeat(count)
        );
        compare_promises(&[&body], &[(4.0, 2.0), (0.0, -0.0)]);
    }
    let too_many = source_for(&format!(
        "const p=Promise.resolve(a);{}return (a?p:b)==p;",
        "await a;".repeat(33)
    ));
    assert!(
        compile_async(&too_many, false)
            .err()
            .unwrap()
            .contains("32 await sites")
    );
    compare_promises(
        &["const p=Promise.resolve(a);const x=a?p:b;return x===p;"],
        &[(4.0, 2.0), (0.0, -0.0)],
    );
    compare_promises(
        &["const p=Promise.resolve(a);const eq=x=>x===p;eq(p);return eq(a);"],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
    let body = "const p=Promise.resolve(a);return (a?p:b)<a;";
    assert!(
        compile_async(&source_for(body), false).is_err(),
        "remaining gap: {body}"
    );
}
