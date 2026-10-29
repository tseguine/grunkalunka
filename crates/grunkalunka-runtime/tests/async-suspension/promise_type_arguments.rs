use super::*;

fn compare_typed(pairs: &[(&str, &str)]) {
    for (plain, typed) in pairs {
        assert_eq!(
            compile_async(plain, false).unwrap().wasm,
            compile_async(typed, true)
                .unwrap_or_else(|error| panic!("{typed}: {error}"))
                .wasm,
            "type metadata changed emission: {typed}"
        );
    }
    compare_async_sources(
        &pairs
            .iter()
            .map(|(plain, _)| (*plain).to_string())
            .collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn promise_type_arguments_are_erased_on_direct_await() {
    compare_typed(&[
        (
            "export async function run(a,b){return await Promise.resolve(a);}",
            "export async function run(a,b){return await Promise.resolve<Missing>(a);}",
        ),
        (
            "export async function run(a,b){try{return await Promise.reject(b);}catch(e){return e;}}",
            "export async function run(a,b){try{return await Promise.reject<never>(b);}catch(e){return e;}}",
        ),
    ]);
}

#[test]
fn promise_type_arguments_promote_exact_former_intrinsic_guards() {
    for body in [
        "const f=x=>x;return f(await Promise.resolve<number>(null));",
        "return await Promise.resolve<number>(a,b);",
        "const p=Promise.resolve<number>(a,b);return await p;",
    ] {
        let typed = source_for(body);
        let plain = typed.replace("<number>", "");
        compare_typed(&[(&plain, &typed)]);
    }
    compare_typed(&[(
        "export async function run(a,b){const p=Promise.resolve(a);return await p;}",
        "export async function run(a,b){const p=Promise.resolve<number>(a);return await p;}",
    )]);
}

#[test]
fn promise_type_arguments_keep_identities_and_private_value_flow() {
    for (plain, typed) in [
        (
            "const p=Promise.resolve(a),q=Promise.resolve(p);await q;return p===q;",
            "const p=Promise.resolve<never>(a),q=Promise.resolve<Missing>(p);await q;return p===q;",
        ),
        (
            "const p=Promise.resolve(a);const use=x=>x;return await use(p);",
            "const p=Promise.resolve<{x:()=>void}>(a);const use=x=>x;return await use(p);",
        ),
        (
            "const p=Promise.resolve(a);let x=a?p:b;const read=()=>x;return await read();",
            "const p=Promise.resolve<typeof absent>(a);let x=a?p:b;const read=()=>x;return await read();",
        ),
        (
            "const f=async()=>{await a;return b;};const p=f();return await Promise.resolve(p);",
            "const f=async()=>{await a;return b;};const p=f();return await Promise.resolve<Promise<Wrong>>(p);",
        ),
        (
            "async function f(x){return await Promise.resolve(x);}export async function run(a,b){return await f(a);}",
            "async function f(x){return await Promise.resolve<never>(x);}export async function run(a,b){return await f(a);}",
        ),
    ] {
        let (plain, typed) = if plain.starts_with("async function") {
            (plain.to_string(), typed.to_string())
        } else {
            (source_for(plain), source_for(typed))
        };
        compare_typed(&[(&plain, &typed)]);
    }
}

#[test]
fn promise_type_arguments_preserve_surplus_effects_failure_and_adoption() {
    for (plain, typed) in [
        (
            "let n=0;const p=Promise.resolve((n=n*10+1,a),n=n*10+2);await p;return n;",
            "let n=0;const p=Promise.resolve<never,Missing>((n=n*10+1,a),n=n*10+2);await p;return n;",
        ),
        (
            "let n=0;try{await Promise.resolve((n=1,a),await Promise.reject(b),n=2);}catch(e){return n;}",
            "let n=0;try{await Promise.resolve<never>((n=1,a),await Promise.reject<Missing>(b),n=2);}catch(e){return n;}",
        ),
        (
            "try{return Promise.reject(b);}catch(e){return 999;}",
            "try{return Promise.reject<never>(b);}catch(e){return 999;}",
        ),
        (
            "try{return await Promise.reject(false);}catch(e){return e;}",
            "try{return await Promise.reject<number>(false);}catch(e){return e;}",
        ),
        (
            "try{return await Promise.resolve();}finally{await b;}",
            "try{return await Promise.resolve<never>();}finally{await b;}",
        ),
        (
            "try{return await Promise.reject();}catch(e){return e;}",
            "try{return await Promise.reject<{a:Missing}>();}catch(e){return e;}",
        ),
    ] {
        compare_typed(&[(&source_for(plain), &source_for(typed))]);
    }
}

#[test]
fn promise_type_arguments_keep_value_binding_and_source_budget_guards() {
    for types in [
        "never",
        "Missing",
        "typeof absent",
        "[Missing,never]",
        "\"not a number\"",
        "Missing,Other",
        "{value:Missing;call:()=>void}",
    ] {
        let plain = source_for("return await Promise.resolve(null);");
        let typed = source_for(&format!("return await Promise.resolve<{types}>(null);"));
        assert_eq!(
            compile_async(&plain, false).unwrap().wasm,
            compile_async(&typed, true).unwrap().wasm
        );
    }
    let plain = source_for("return await Promise.resolve(a);");
    let thousand_types = vec!["never"; 1001].join(",");
    let typed = source_for(&format!(
        "return await Promise.resolve<{thousand_types}>(a);"
    ));
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
    for count in [32, 33] {
        let plain = source_for(&format!(
            "{}return b;",
            "await Promise.resolve(a);".repeat(count)
        ));
        let typed = plain.replace("Promise.resolve(", "Promise.resolve<never>(");
        if count == 32 {
            compare_typed(&[(&plain, &typed)]);
        } else {
            assert!(
                compile_async(&typed, true)
                    .err()
                    .unwrap()
                    .contains("32 await sites")
            );
        }
    }
    let prefix = "export async function run(a,b){return await Promise.resolve<\"";
    let suffix = "\">(a);}";
    let exact = format!(
        "{prefix}{}{suffix}",
        "x".repeat(16_384 - prefix.len() - suffix.len())
    );
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&exact, true).unwrap().wasm
    );
    assert!(
        compile_async(&exact.replace("\">(a)", "x\">(a)"), true)
            .err()
            .unwrap()
            .contains("16 KiB")
    );
    compare_typed(&[
        (
            &source_for("return Math.abs(a);"),
            &source_for("return Math.abs<never>(a);"),
        ),
        (
            &source_for("const f=x=>x;return await f(a);"),
            &source_for("const f=x=>x;return await f<never>(a);"),
        ),
    ]);
    for body in [
        "const Promise=a;return await Promise.resolve<never>(b);",
        "return await Promise.other<never>(a);",
        "return await Promise.resolve<never>(...a);",
        "return await Promise.resolve?.<never>(a);",
        "return await Promise['resolve']<never>(a);",
        "return await Promise.resolve<never>(absent);",
        "return await Promise.resolve<never>({});",
        "return await Promise.reject<never>(()=>a);",
        "return await Promise.resolve<Missing>(a);type Missing=typeof absent;return absent;",
    ] {
        assert!(compile_async(&source_for(body), true).is_err(), "{body}");
    }
}
