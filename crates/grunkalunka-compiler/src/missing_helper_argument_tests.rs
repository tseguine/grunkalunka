#[test]
fn omitted_named_helper_arguments_use_undefined_without_numeric_laundering() {
    for source in [
        "function flag(x){return x===undefined?1:0;}export function run(a,b){return flag();}",
        "function flag(x){return typeof x==='undefined'?1:0;}export function run(a,b){return flag();}",
        "function add(x,y){return x+ +y;}export function run(a,b){return add(a);}",
        "function choose(x,y){return y??x;}export function run(a,b){return choose(a);}",
        "function flag(x){return x===undefined?1:0;}export function run(a,b){return flag()*100+flag(a)*10+flag(NaN);}",
        "function f(x){x=3;return x;}export function run(a,b){return +f();}",
        "function f(x){if(x===undefined)return 7;return f();}export function run(a,b){return f(a);}",
        "function f(x,y){return y===undefined?x:0;}export function run(a,b){let n=0;const x=f((n++,a));return x+n;}",
        "function flag(x){return x===undefined?1:0;}export function run(a,b){let n=0;const value=flag.call((n++,a));return value*10+n;}",
        "function flag(x){return x===undefined?1:0;}export function run(a,b){return flag.call();}",
        "function f(x){const g=()=>x===undefined?1:0;return g();}export function run(a,b){return f();}",
        "function choose(x,y){return y??x;}export function run(a,b){const text=choose('x');return text==='x'?a:b;}",
        "function choose(x,y){return y??x;}export function run(a,b){const value=choose({v:a});return +value.v;}",
        "function flag(x){return x===undefined?1:0;}export function run(a,b){const alias=flag;return flag()*100+alias(a);}",
        "function flag(x,y){return typeof y==='undefined'?1:0;}export function run(a,b){return flag.length*100+flag(a);}",
    ] {
        for typed in [false, true] {
            let result = crate::compile(source, typed);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
}
#[test]
fn missing_argument_support_keeps_spread_async_and_numeric_return_guards() {
    for source in [
        "export function run(a,b){const f=(x)=>x;return f();}",
        "function f(x,y){return x;}export function run(a,b){return f(Math.random());}",
        "async function f(x){return x;}export async function run(a,b){return await f();}",
    ] {
        let result = if source.contains("export async") {
            crate::compile_async(source, false)
        } else {
            crate::compile(source, false)
        };
        assert!(result.is_err(), "{source}");
    }
}

#[test]
fn numeric_exports_do_not_turn_an_omitted_undefined_return_into_nan() {
    assert!(
        crate::compile(
            "function f(x){return x;}export function run(a,b){return f();}",
            false
        )
        .is_err()
    );
    assert!(
        crate::compile_value(
            "function f(x){return x;}export function run(a,b){return f();}",
            false
        )
        .is_ok()
    );
}

#[test]
fn omitted_formal_synthesis_keeps_declared_parameter_budget() {
    for count in [1000, 1001] {
        let parameters = (0..count)
            .map(|i| format!("p{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let source = format!(
            "function f({parameters}){{return +p0;}}export function run(a,b){{return f();}}"
        );
        let result = crate::compile(&source, false);
        if count == 1000 {
            assert!(result.is_ok(), "{:?}", result.err());
        } else {
            assert!(result.is_err(), "above declared parameter limit");
        }
    }
}

#[test]
fn omitted_named_arguments_ignore_optional_parameter_annotations() {
    let js = "function choose(x,y){return y??x;}export function run(a,b){return choose(a);}";
    let ts = "function choose(x?:unknown,y?:unknown){return y??x;}export function run(a?:number,b?:number){return choose(a);}";
    assert_eq!(
        crate::compile(js, false).unwrap().wasm,
        crate::compile(ts, true).unwrap().wasm
    );
}
