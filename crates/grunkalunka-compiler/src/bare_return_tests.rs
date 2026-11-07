#[test]
fn bare_returns_use_canonical_undefined_in_represented_result_profiles() {
    for source in [
        "export function run(a,b) {return;}",
        "export function run(a,b) {if(a)return;return b;}",
        "export function run(a,b) {const undefined=b;return;}",
        "export function run(a,b) {try{return;}finally{a=b;}}",
        "export function run(a,b) {try{if(a)return;return b;}finally{b++;}}",
        "export function run(a,b) {try{return b;}finally{if(a)return;}}",
        "export function run(a,b) {for(let i=0;i<2;i++){if(a)return;}return b;}",
        "function f(x){if(x)return null;return;}export function run(a,b){return f(a);}",
    ] {
        for typed in [false, true] {
            let result = crate::compile_value(source, typed);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
}

#[test]
fn bare_return_support_keeps_numeric_and_source_validation_guards() {
    for source in [
        "export function run(a,b) {return;}",
        "export function run(a,b) {if(a)return;return b;}",
        "function f(x){if(x)return x;return;}export function run(a,b){return f(a);}",
    ] {
        assert!(crate::compile(source, false).is_err(), "{source}");
    }
    for source in [
        "export function run(a,b) {if(a)return;}",
        "export function run(a,b) {return;Math.random();}",
        "export function run(a,b) {if(a)Math.random();return;}",
    ] {
        assert!(crate::compile_value(source, false).is_err(), "{source}");
    }
    assert_eq!(
        crate::compile_value("export function run(a,b) {return;}", false)
            .unwrap()
            .wasm,
        crate::compile_value(
            "export function run(a:number,b:number):void {return;}",
            true
        )
        .unwrap()
        .wasm
    );
}

#[test]
fn bare_helper_return_composes_with_explicit_numeric_conversion() {
    let source = "function f(x){if(x)return null;return;}export function run(a,b){return +f(a);}";
    for typed in [false, true] {
        let result = crate::compile(source, typed);
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
}

#[test]
fn unused_bare_closure_does_not_change_async_call_signatures() {
    for body in [
        "const f=x=>()=>x;let p=f;const keep=p(null);const g=x=>()=>x;const h=(a?f:g)(await b);await a;return h();",
        "const f=async x=>{await x;return x;};const make=p=>async()=>{await a;return await p;};const get=make(f(b));return await get();",
        "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();try{await p;}catch(e){const cmp=(x,y)=>x<=y;cmp(p,e);return cmp(e,p);}",
    ] {
        let source =
            format!("export async function run(a,b){{const unused=()=>{{return;}};{body}}}");
        let result = crate::compile_async(&source, false);
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
}

#[test]
fn unused_bare_named_helper_preserves_pending_async_calls() {
    let source = "function unused(){return;}export async function run(a,b){const f=async x=>{await x;return x;};const make=p=>async()=>{await a;return await p;};const get=make(f(b));return await get();}";
    let result = crate::compile_async(source, false);
    assert!(result.is_ok(), "{source}: {:?}", result.err());
    let source = "function f(x){return;}export function run(a,b){const g=f;return +g(a);}";
    let result = crate::compile(source, false);
    assert!(result.is_ok(), "{source}: {:?}", result.err());
}

#[test]
fn called_bare_closures_do_not_change_pending_async_calls() {
    for declaration in ["const unused=()=>{return;};unused();", ""] {
        let source = format!(
            "export async function run(a,b){{{declaration}const f=async x=>{{await x;return x;}};const make=p=>async()=>{{await a;return await p;}};const get=make(f(b));return await get();}}"
        );
        let result = crate::compile_async(&source, false);
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
    let source = "function unused(){return;}export async function run(a,b){unused();const f=async x=>{await x;return x;};const make=p=>async()=>{await a;return await p;};const get=make(f(b));return await get();}";
    let result = crate::compile_async(source, false);
    assert!(result.is_ok(), "{source}: {:?}", result.err());
}
