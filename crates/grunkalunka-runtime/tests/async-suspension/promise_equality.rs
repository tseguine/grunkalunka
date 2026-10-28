use super::*;

#[test]
fn represented_promise_identity_equals_itself() {
    compare_promises(
        &["const f=async()=>{await a;return b;};const p=f();return p===p;"],
        &[(4.0, 2.0)],
    );
}

const PAIR: &str = "const f=async x=>await x;let p=f(a);const q=f(b);";

#[test]
fn represented_promise_equality_uses_identity_instead_of_completion() {
    let mut bodies = Vec::new();
    for op in ["===", "!==", "==", "!="] {
        for right in ["p", "q", "f(a)", "Promise.resolve(p)"] {
            bodies.push(format!("{PAIR}return p{op}{right};"));
        }
    }
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (2.0, 2.0), (0.0, -0.0), (f64::NAN, f64::NAN)],
    );
}

#[test]
fn represented_promise_equality_preserves_snapshots_helpers_and_jobs() {
    let tails = [
        "const old=p;p=q;return old===p;",
        "return p===(p=q);",
        "const get=()=>p;const old=get();p=q;return old===get();",
        "const eq=(x,y)=>x===y;return eq(p,p);",
        "const eq=a?(x,y)=>x===y:(x,y)=>x!==y;return eq(p,q);",
        "const apply=(fn,x,y)=>fn(x,y);return apply((x,y)=>x==y,p,p);",
        "let n=0;const take=x=>{n=n*10+x;return p;};const same=take(1)===take(2);return same?n:-n;",
        "return p===Promise.resolve(p,p=q);",
        "const same=p===Promise.resolve(p,await a);return same;",
        "await p;return p===p;",
        "const g=async()=>{await a;return p;};return p===g();",
    ]
    .map(|tail| format!("{PAIR}{tail}"));
    compare_promises(
        &tails.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn represented_promise_equality_distinguishes_general_values_in_both_orders() {
    let mut bodies = Vec::new();
    for op in ["===", "!==", "==", "!="] {
        for value in ["a", "b", "null", "undefined", "true", "false", "await p"] {
            for expression in [format!("p{op}{value}"), format!("{value}{op}p")] {
                bodies.push(format!("{PAIR}return {expression};"));
            }
        }
    }
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn represented_promise_is_distinct_from_its_error_reason() {
    let cycle = "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();";
    let mut bodies = Vec::new();
    for op in ["===", "!==", "==", "!="] {
        for expression in [format!("p{op}e"), format!("e{op}p")] {
            bodies.push(format!(
                "{cycle}try{{await p;}}catch(e){{return {expression};}}"
            ));
        }
    }
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn represented_promise_cannot_alias_a_number_equal_to_its_opaque_id() {
    use grunkalunka_runtime::PromiseNotification;
    let prefix = "const f=async()=>{await b;throw b;};const p=f();await b;await b;";
    let suffix = ";try{await p;}catch(e){}return same;";
    let body = format!("{prefix}const same=p==a{suffix}");
    let artifact = compile_async(&source_for(&body), false).unwrap();
    let mut run = AsyncInstance::new(&artifact.wasm, 0.0, 2.0).unwrap();
    while run.run_one_job().unwrap() {}
    let notifications = run.take_promise_notifications().unwrap();
    let [
        PromiseNotification::Rejected { id, .. },
        PromiseNotification::Handled { id: handled },
    ] = notifications.as_slice()
    else {
        panic!("delayed await must expose one rejected/handled pair: {notifications:?}");
    };
    assert_eq!(id, handled);
    let bodies = ["===", "!==", "==", "!="].map(|op| format!("{prefix}const same=p{op}a{suffix}"));
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(f64::from(*id), 2.0)],
    );
}

#[test]
fn represented_promise_comparison_does_not_handle_rejection_or_add_jobs() {
    use grunkalunka_runtime::PromiseNotification;
    for result in ["p===p", "true"] {
        let source = source_for(&format!(
            "const f=async()=>{{await b;throw b;}};const p=f();return {result};"
        ));
        let artifact = compile_async(&source, false).unwrap();
        let mut run = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
        assert!(matches!(
            run.state().unwrap(),
            AsyncState::Fulfilled(Value::Boolean(true))
        ));
        assert_eq!(run.pending_job_count().unwrap(), 1);
        assert!(run.run_one_job().unwrap());
        assert!(!run.run_one_job().unwrap());
        let notifications = run.take_promise_notifications().unwrap();
        assert!(
            matches!(notifications.as_slice(),[PromiseNotification::Rejected{reason:Value::Number(n),..}] if *n==2.0),
            "{notifications:?}"
        );
    }
}

#[test]
fn represented_promise_comparisons_keep_erasure_and_await_site_limits() {
    let plain = source_for(&format!("{PAIR}return p===q;"));
    let typed = plain
        .replace("run(a,b)", "run(a:never,b:Missing):Wrong")
        .replace(
            "return p===q;",
            "return (p as unknown as number)===(q as never);",
        );
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
    for count in [31, 32] {
        let mut body = "const f=async()=>{await a;return b;};const p=f();".to_string();
        for i in 0..count {
            body.push_str(&format!("const same{i}=p===p;await p;"));
        }
        body.push_str("return p===p;");
        if count == 31 {
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
    let snapshot =
        "const f=async()=>{await a;return b;};const p=f();return p===Promise.resolve(b);";
    assert!(compile_async(&source_for(snapshot), false).is_err());
}
