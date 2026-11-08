#[test]
fn private_primitive_defaults_admit_omitted_and_supplied_values() {
    for source in [
        "function f(x=3){return x;}export function run(a,b){return f();}",
        "function f(x=3){return x+1;}export function run(a,b){return f(a);}",
        "function f(x=-0){return x;}export function run(a,b){return f();}",
        "function f(x=true){return x;}export function run(a,b){return f();}",
        "function f(x=null){return x;}export function run(a,b){return f();}",
        "function f(x=undefined){return x;}export function run(a,b){return f();}",
        "function f(x='abc'){return x.length;}export function run(a,b){return f();}",
        "function f(x=3){return x;}export function run(a,b){const g=f;return g();}",
        "function f(x=3,y=false){return x;}export function run(a,b){return f(a);}",
        "function f(x=3){return x;}export function run(a,b){return a;}",
        "function f(x=3){return x;}export function run(a,b){return f.length;}",
        "function f(x=3){return x;}export function run(a,b){const g=f;return g.length;}",
    ] {
        for ts in [false, true] {
            let result = crate::compile_value(source, ts);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
}

#[test]
fn helper_default_scope_and_source_guards_remain_explicit() {
    for source in [
        "export function run(a=1,b){return a;}",
        "function f(x=3){return x;}function g(x){return x;}export function run(a,b){const h=a?f:g;return h.length;}",
        "function f(...x){return 1;}export function run(a,b){return f();}",
        "function f({x}=null){return 1;}export function run(a,b){return f();}",
        "function f(x=a){return x;}export function run(a,b){return f();}",
        "function f(x=Math.random()){return x;}export function run(a,b){return f();}",
        "function f(x=1+2){return x;}export function run(a,b){return f();}",
        "function f(undefined,x=undefined){return x;}export function run(a,b){return f(a);}",
        "export function run(a,b){const f=(x=a)=>x;return f();}",
    ] {
        assert!(crate::compile_value(source, false).is_err(), "{source}");
    }
    assert!(
        crate::compile_async(
            "function f(x=3){return x;}export async function run(a,b){return f();}",
            false
        )
        .is_err()
    );
}

#[test]
fn helper_default_wrappers_respect_source_depth_and_erased_annotations() {
    let js = "function f(x=3){return x;}export function run(a,b){return f(a);}";
    let ts = "function f<T>(x:never=3):Missing{return x as never;}export function run(a:unknown,b:unknown):unknown{return f<Wrong>(a);}";
    assert_eq!(
        crate::compile_value(js, false).unwrap().wasm,
        crate::compile_value(ts, true).unwrap().wasm
    );
    for depth in [16, 129] {
        let source = format!(
            "function f(x={}1){{return x;}}export function run(a,b){{return f();}}",
            "+ ".repeat(depth)
        );
        assert_eq!(crate::compile_value(&source, false).is_ok(), depth == 16);
    }
}

#[test]
fn synchronous_arrow_defaults_share_the_checked_primitive_selection() {
    for source in [
        "export function run(a,b){const f=(x=3)=>x;return f();}",
        "export function run(a,b){const f=(x=3)=>x;return f(a);}",
        "export function run(a,b){const f=(x=3)=>x;return f(undefined);}",
        "export function run(a,b){const f=(x=true)=>x;return f();}",
        "export function run(a,b){const f=(x='abc')=>x.length;return f();}",
        "export function run(a,b){const f=(x=3)=>y=>x+y;return f()(a);}",
        "export function run(a,b){const f=(x=3)=>x;return f.length;}",
        "export function run(a,b){const f=(x,y=3,z)=>y;return f.length;}",
        "export function run(a,b){const f=(x=3)=>{x++;return x;};return f();}",
    ] {
        for ts in [false, true] {
            let result = crate::compile_value(source, ts);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
}

#[test]
fn arrow_defaults_keep_effect_scope_async_and_callback_arity_guards() {
    for source in [
        "export function run(a,b){const f=(x=a)=>x;return f();}",
        "export function run(a,b){const f=(x=Math.random())=>x;return f();}",
        "export function run(a,b){const f=(x=3)=>Math.noSuch(x);return a;}",
        "export function run(a,b){const f=(x=1+2)=>x;return f();}",
        "export function run(a,b){const f=(undefined,x=undefined)=>x;return f();}",
        "export function run(a,b){const f=({x}=null)=>x;return f();}",
        "export function run(a,b){const f=(x=3)=>x;const invoke=g=>g();return invoke(f);}",
        "export function run(a,b){const f=(x=3)=>x;const g=x=>x;const h=a?f:g;return h.length;}",
        "export function run(a,b){const f=(x='abc')=>x;return f();}",
    ] {
        assert!(crate::compile_value(source, false).is_err(), "{source}");
    }
    for source in [
        "export async function run(a,b){const f=(x=3)=>x;return f();}",
        "export async function run(a,b){const f=async(x=3)=>x;return await f();}",
    ] {
        assert!(crate::compile_async(source, false).is_err(), "{source}");
    }
}
