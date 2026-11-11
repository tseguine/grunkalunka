//! A TypeScript this parameter has no runtime argument or binding.

#[test]
fn this_parameter_metadata_preserves_numeric_and_value_artifacts() {
    for (js, ts) in [
        (
            "export function run(a,b) { return a; }",
            "export function run(this:void,a:number,b:number):number { return a; }",
        ),
        (
            "export function run(){return 4;}",
            "export function run(this:Missing){return 4;}",
        ),
        (
            "export function run(a){return a+run.length;}",
            "export function run(this:never,a?:unknown){return a+run.length;}",
        ),
        (
            "function helper(x) { return x; } export function run(a,b) { return helper(a); }",
            "function helper(this:number,x:number) { return x; } export function run(a,b) { return helper(a); }",
        ),
        (
            "function f(x,y=3){return x+y;}export function run(a,b){return f.length+f(a)+f.call(b,a,b);}",
            "function f(this:typeof absent,x:number,y=3){return x+y;}export function run(this:void,a,b){return f.length+f(a)+f.call(b,a,b);}",
        ),
        (
            "function add(x,y){return x+y;}export function run(a,b){let n=0;const r=add.call(n++,n++,n++);return r*10+n;}",
            "function add(this:never,x,y){return x+y;}export function run(this:Missing,a,b){let n=0;const r=add.call(n++,n++,n++);return r*10+n;}",
        ),
    ] {
        assert_eq!(
            crate::compile(js, false).unwrap().wasm,
            crate::compile(ts, true).unwrap().wasm,
            "{ts}"
        );
        assert_eq!(
            crate::compile_value(js, false).unwrap().wasm,
            crate::compile_value(ts, true).unwrap().wasm,
            "{ts}"
        );
    }
}

#[test]
fn this_parameter_metadata_preserves_async_and_heap_artifacts() {
    for (parameters, value) in [("", "4"), ("a", "a"), ("a,b", "a+b")] {
        let typed_parameters = if parameters.is_empty() {
            "this:Missing".into()
        } else {
            format!("this:Missing,{parameters}")
        };
        let js = format!("export async function run({parameters}){{return await({value});}}");
        let ts = format!("export async function run({typed_parameters}){{return await({value});}}");
        assert_eq!(
            crate::compile_async(&js, false).unwrap().wasm,
            crate::compile_async(&ts, true).unwrap().wasm,
            "{ts}"
        );
        let js = format!("export function run({parameters}){{return JSON.stringify('ok');}}");
        let ts = format!("export function run({typed_parameters}){{return JSON.stringify('ok');}}");
        assert_eq!(
            crate::compile_heap_value(&js, false).unwrap().wasm,
            crate::compile_heap_value(&ts, true).unwrap().wasm,
            "{ts}"
        );
    }
    assert_eq!(
        crate::compile_async("async function f(x){return await x;}export async function run(a,b){return await f(a)+f.length;}", false).unwrap().wasm,
        crate::compile_async("async function f(this:void,x){return await x;}export async function run(this:never,a,b){return await f(a)+f.length;}", true).unwrap().wasm
    );
}

#[test]
fn this_parameter_metadata_does_not_supply_runtime_values_or_relax_parameters() {
    for parameters in [
        "this:void,a,b,c",
        "this:void,a,...rest",
        "this:void,a,b=1",
        "this:void,{a},b",
    ] {
        let source = format!("export function run({parameters}){{return 4;}}");
        assert!(crate::compile(&source, true).is_err(), "{source}");
        assert!(crate::compile_value(&source, true).is_err(), "{source}");
        assert!(
            crate::compile_heap_value(&source.replace("return 4;", "return 'ok';"), true).is_err(),
            "{source}"
        );
        assert!(
            crate::compile_async(
                &source.replace("export function", "export async function"),
                true
            )
            .is_err(),
            "{source}"
        );
    }
    for value in ["this", "this.value", "()=>this"] {
        let source = format!("export function run(this:{{value:number}},a,b){{return {value};}}");
        assert!(crate::compile(&source, true).is_err(), "{source}");
        assert!(crate::compile_value(&source, true).is_err(), "{source}");
        assert!(
            crate::compile_heap_value(&source, true).is_err(),
            "{source}"
        );
        assert!(
            crate::compile_async(
                &source.replace("export function", "export async function"),
                true
            )
            .is_err(),
            "{source}"
        );
    }
    assert!(crate::compile("export function run(this:void,a,b){return a;}", false).is_err());
    for parameters in [
        "this:void,...xs",
        "this:void,{x}",
        "this:void,x=sideEffect()",
    ] {
        let source = format!(
            "function f({parameters}){{return 1;}}export function run(a,b){{return f(a);}}"
        );
        assert!(crate::compile(&source, true).is_err(), "{source}");
    }
}

#[test]
fn this_parameter_metadata_keeps_ordinary_parameter_and_source_limits() {
    for count in [1000, 1001] {
        let parameters = (0..count)
            .map(|n| format!("p{n}"))
            .collect::<Vec<_>>()
            .join(",");
        let plain = format!(
            "function f({parameters}){{return 0;}}export function run(a,b){{return f.length;}}"
        );
        let typed = plain.replacen("function f(", "function f(this:Missing,", 1);
        assert_eq!(crate::compile(&typed, true).is_ok(), count == 1000);
        if count == 1000 {
            assert_eq!(
                crate::compile(&plain, false).unwrap().wasm,
                crate::compile(&typed, true).unwrap().wasm
            );
        }
    }
    let prefix = "export function run(this:'";
    let suffix = "',a,b){return a;}";
    let source = format!(
        "{prefix}{}{suffix}",
        "x".repeat(16_384 - prefix.len() - suffix.len())
    );
    assert!(crate::compile(&source, true).is_ok());
    assert!(
        crate::compile(&(source + " "), true)
            .err()
            .unwrap()
            .contains("16 KiB")
    );
}
