//! Optional markers are erased metadata, not a new calling convention.
#[test]
fn optional_markers_preserve_entry_helper_and_arrow_artifacts() {
    for (js, ts) in [
        (
            "export function run(a,b){return a;}",
            "export function run(a?:unknown,b?:unknown):unknown{return a;}",
        ),
        (
            "function add(x,y){return x+y;}export function run(a,b){return add(a,b);}",
            "function add(x?:string,y?:unknown):unknown{return x+y;}export function run(a?:number,b?:number){return add(a,b);}",
        ),
        (
            "export function run(a,b){const f=(x,y)=>x+y;return f(a,b);}",
            "export function run(a?:number,b?:number){const f=(x?:string,y?:unknown):unknown=>x+y;return f(a,b);}",
        ),
        (
            "export function run(a,b){let n=a;const f=(x)=>{n+=x;return n;};return f(b)+f(a);}",
            "export function run(a?:number,b?:number){let n=a;const f=(x?:number)=>{n+=x;return n;};return f(b)+f(a);}",
        ),
        (
            "export function run(a,b){const f=(x,y)=>x+y;return f.length+f(a,b);}",
            "export function run(a?:number,b?:number){const f=(x?:unknown,y?:unknown)=>x+y;return f.length+f(a,b);}",
        ),
        (
            "export function run(a,b){const o={f:(x)=>x+a};return o.f(b);}",
            "export function run(a?:number,b?:number){const o={f:(x?:unknown)=>x+a};return o.f(b);}",
        ),
        (
            "export function run(a,b){const apply=(f,x)=>f(x);const id=(x)=>x+a;return apply(id,b);}",
            "export function run(a?:number,b?:number){const apply=(f?:unknown,x?:unknown)=>f(x);const id=(x?:unknown)=>x+a;return apply(id,b);}",
        ),
        (
            "function helper(x) { return x; } export function run(a,b) { return helper(a); }",
            "function helper(x?:number) { return x; } export function run(a,b) { return helper(a); }",
        ),
        (
            "export function run(a,b) { return a; }",
            "export function run<T>(a?:T,b?:T):T { return a; }",
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
fn optional_markers_keep_other_parameter_and_numeric_return_guards() {
    for source in [
        "export function run(a?:number){return a;}",
        "export function run(a?:number,b=1){return a;}",
        "export function run(a?:number,...b:number[]){return a;}",
        "export function run(a?:number,b?:number){const f=(x?:number)=>x;return f();}",
        "export function run(a?:number,b?:number){const f=(x:number=a)=>x;return f(a);}",
        "export function run(a?:number,b?:number){const f=(...x:number[])=>a;return f(a);}",
        "export function run(a?:number,b?:number){const f=({x}:unknown)=>x;return f(a);}",
    ] {
        assert!(crate::compile(source, true).is_err(), "{source}");
    }
    assert!(crate::compile("export function run(a?:number,b?:number){return a;}", false).is_err());
}
#[test]
fn optional_markers_preserve_heap_and_async_artifacts() {
    assert_eq!(
        crate::compile_heap_value("export function run(a,b){return 'ok';}", false)
            .unwrap()
            .wasm,
        crate::compile_heap_value(
            "export function run(a?:unknown,b?:unknown){return 'ok';}",
            true
        )
        .unwrap()
        .wasm
    );
    for (js, ts) in [
        (
            "export async function run(a,b){return a+await b;}",
            "export async function run(a?:unknown,b?:unknown){return a+await b;}",
        ),
        (
            "export async function run(a,b){const f=async(x)=>await x;return await f(a);}",
            "export async function run(a?:unknown,b?:unknown){const f=async(x?:unknown)=>await x;return await f(a);}",
        ),
    ] {
        assert_eq!(
            crate::compile_async(js, false).unwrap().wasm,
            crate::compile_async(ts, true).unwrap().wasm,
            "{ts}"
        );
    }
}

#[test]
fn optional_markers_erase_around_literal_arrow_defaults() {
    let original = "export function run(a?:number,b?:number){const f=(x:number=1)=>x;return f(a);}";
    let js = "export function run(a,b){const f=(x=1)=>x;return f(a);}";
    assert_eq!(
        crate::compile(original, true).unwrap().wasm,
        crate::compile(js, false).unwrap().wasm
    );
}
