use super::*;

fn compare_typed(plain: &str, typed: &str) {
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true)
            .unwrap_or_else(|error| panic!("{typed}: {error}"))
            .wasm,
        "{typed}"
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
fn direct_call_type_arguments_preserve_async_caller_snapshots_and_jobs() {
    let helpers = "function id(x){return x;}";
    for body in [
        "return id(await a);",
        "return await id(a);",
        "let x=a;return id(x,await(x=b))+(+x);",
        "try{return id(a,await Promise.reject(b));}catch(e){return e;}",
        "try{return id(await Promise.reject(b),a=b);}catch(e){return a;}",
        "try{return await id(a);}finally{await b;}",
        "return id(a)<b;",
    ] {
        let plain = format!("{helpers}{}", source_for(body));
        let typed = format!(
            "{helpers}{}",
            source_for(&body.replace("id(", "id<Missing,Other>("))
        );
        compare_typed(&plain, &typed);
    }
}

#[test]
fn direct_call_type_arguments_preserve_nested_and_recursive_helpers() {
    for (plain, typed) in [
        (
            "function first(x){return second(x)+1;}function second(x){return x*2;}export async function run(a,b){return first(await a)+b;}",
            "function first(x){return second<never>(x)+1;}function second(x){return x*2;}export async function run(a,b){return first<Missing>(await a)+b;}",
        ),
        (
            "function fold(n,x){if(n<=0)return x;return fold(n-1,x+1);}export async function run(a,b){return await fold(3,a);}",
            "function fold(n,x){if(n<=0)return x;return fold<never>(n-1,x+1);}export async function run(a,b){return await fold<Missing>(3,a);}",
        ),
        (
            "function zero(){return 7;}export async function run(a,b){let n=0;return zero(n=n+1,await(n=n+10))+n;}",
            "function zero(){return 7;}export async function run(a,b){let n=0;return zero<never>(n=n+1,await(n=n+10))+n;}",
        ),
        (
            "export async function run(a,b){const n=start(a);await b;return n;}function start(x){const child=async()=>{await x;return x;};child();return x+1;}",
            "export async function run(a,b){const n=start<Missing>(a);await b;return n;}function start(x){const child=async()=>{await x;return x;};child();return x+1;}",
        ),
        (
            "function undefined(x){return x+1;}export async function run(a,b){return undefined(a);}",
            "function undefined(x){return x+1;}export async function run(a,b){return undefined<typeof absent>(a);}",
        ),
    ] {
        compare_typed(plain, typed);
    }
}

#[test]
fn direct_call_type_arguments_keep_async_helper_number_and_binding_domains() {
    for body in [
        "return id<never>();",
        "return id<never>(...a);",
        "return id?.<never>(a);",
        "return id<number>(null);",
        "return id<number>(true);",
        "return id<number>(missing);",
        "const id=a;return id<never>(b);",
    ] {
        let source = format!("function id(x){{return x;}}{}", source_for(body));
        assert!(compile_async(&source, true).is_err(), "{source}");
    }
    let plain = "function id(x){return x;}export async function run(a,b){return id(a);}";
    for metadata in ["never", "Missing", "typeof absent", "{call:()=>void}"] {
        let typed = plain.replace("return id(", &format!("return id<{metadata}>("));
        compare_typed(plain, &typed);
    }
    compare_typed(
        "function id(x){return x;}export async function run(a,b){return (id)(a);}",
        "function id(x){return x;}export async function run(a,b){return (id)<never>(a);}",
    );
}
