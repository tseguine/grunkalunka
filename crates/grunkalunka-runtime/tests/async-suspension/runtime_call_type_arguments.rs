use super::*;

fn compare_typed(plain: &str, typed: &str) {
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true)
            .unwrap_or_else(|error| panic!("{typed}: {error}"))
            .wasm,
        "type metadata changed guest bytes: {typed}"
    );
    compare_async_sources(
        &[plain.to_string()],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-2.0, -4.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn runtime_call_type_arguments_erase_on_async_closures() {
    compare_typed(
        &source_for("const f=async x=>await x;return await f(a);"),
        &source_for("const f=async x=>await x;return await f<Missing>(a);"),
    );
    for body in [
        "const f=async x=>await x;const alias=f;return await alias(a);",
        "const f=async x=>{await a;return x;};return await f(null);",
        "const f=async x=>{await a;throw x;};try{return await f(b);}catch(e){return e;}",
        "let x=a;const f=async()=>{const old=x;await b;x=b;return old;};const p=f();return (await p)+x;",
        "const f=async()=>{await a;return b;};f();await a;return b;",
    ] {
        let plain = source_for(body);
        let typed = plain
            .replace("f(", "f<never>(")
            .replace("alias(", "alias<typeof absent>(");
        compare_typed(&plain, &typed);
    }
    compare_typed(
        "async function id(x){return await x;}export async function run(a,b){return await id(a);}",
        "async function id(x){return await x;}export async function run(a,b){return await id<Missing>(a);}",
    );
}

#[test]
fn runtime_call_type_arguments_keep_local_and_selected_call_snapshots() {
    for body in [
        "const f=x=>x;return await f(a);",
        "let x=a;const f=(y,z)=>y+z;return f(x,x=b)+x;",
        "let x=null;const f=y=>{const read=()=>y;return read();};return f(x,x=b);",
        "const left=async x=>{await a;return x;},right=async x=>{await b;return x+1;};const f=a?left:right;return await f(b);",
        "const make=x=>async y=>{await b;return x+y;};const f=make(a);return await f(b);",
        "let n=0;const make=x=>(n=n+1,async y=>{await y;return x+n;});const f=make(a);return await f(b,n=n+10);",
        "const f=async x=>x;try{return await f(a,await Promise.reject(b));}catch(e){return e;}",
        "const f=x=>x;return f(a,await(a=b))+a;",
        "const left=async x=>{await a;return x;},right=async x=>{await b;return x+10;};let f=left;return await f(b,f=right);",
    ] {
        let plain = source_for(body);
        let typed = plain.replace("f(", "f<Missing,Other>(");
        compare_typed(&plain, &typed);
    }
    compare_typed(
        &source_for("const make=x=>async y=>{await y;return x+y;};return await make(a)(b);"),
        &source_for("const make=x=>async y=>{await y;return x+y;};return await make(a)<never>(b);"),
    );
    compare_typed(
        &source_for("const make=x=>()=>x;return make(a)();"),
        &source_for("const make=x=>()=>x;return make<never>(a)();"),
    );
    compare_typed(
        &source_for(
            "const apply=async(fn,x)=>{await a;return fn(x);};return await apply(x=>x+b,a);",
        ),
        &source_for(
            "const apply=async(fn,x)=>{await a;return fn<Missing>(x);};return await apply<never>(x=>x+b,a);",
        ),
    );
}

#[test]
fn runtime_call_type_arguments_do_not_prove_runtime_kinds_or_bindings() {
    for body in [
        "const f=async x=>x;return await f<never>();",
        "const f=async x=>x;return await f<never>(...a);",
        "const f=async x=>x;return await f?.<never>(a);",
        "const f=async x=>x;return await f<number>('x');",
        "const f=async x=>x;return await f<number>({});",
        "const f=async x=>x;return await f<number>(1n);",
        "const f=async x=>x;return await f<never>(missing);",
        "const f=a;return await f<never>(b);",
        "type f=()=>number;return await f<never>(a);",
    ] {
        assert!(compile_async(&source_for(body), true).is_err(), "{body}");
    }
    let direct = "function id(x){return x;}export async function run(a,b){return id<never>(a);}";
    assert!(compile_async(direct, true).is_err());
}

#[test]
fn runtime_call_type_arguments_keep_metadata_and_source_budgets() {
    let plain = source_for("const f=async x=>await x;return await f(a);");
    for metadata in ["Missing", "never", "typeof absent", "{call:()=>void}"] {
        let typed = plain.replace("f(", &format!("f<{metadata}>("));
        compare_typed(&plain, &typed);
    }
    let types = vec!["never"; 1001].join(",");
    let typed = plain.replace("f(", &format!("f<{types}>("));
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
    for count in [1000, 1001] {
        let arguments = vec!["a"; count].join(",");
        let plain = source_for(&format!("const f=async x=>x;return await f({arguments});"));
        let typed = plain.replace("f(", "f<never>(");
        if count == 1000 {
            assert_eq!(
                compile_async(&plain, false).unwrap().wasm,
                compile_async(&typed, true).unwrap().wasm
            );
        } else {
            assert!(compile_async(&typed, true).err().unwrap().contains("1000"));
        }
    }
    let prefix = "export async function run(a,b){const f=async x=>x;return await f<\"";
    let suffix = "\">(a);}";
    let exact = format!(
        "{prefix}{}{suffix}",
        "x".repeat(16_384 - prefix.len() - suffix.len())
    );
    let plain = source_for("const f=async x=>x;return await f(a);");
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&exact, true).unwrap().wasm
    );
    assert!(
        compile_async(&(exact + " "), true)
            .err()
            .unwrap()
            .contains("16 KiB")
    );
}
