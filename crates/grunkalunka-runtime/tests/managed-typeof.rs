#[path = "managed/common.rs"]
mod common;

#[test]
fn typeof_property_values_returns_language_kind_strings() {
    for (value, kind) in [
        ("undefined", "undefined"),
        ("null", "object"),
        ("false", "boolean"),
        ("true", "boolean"),
        ("a", "number"),
        ("''", "string"),
        ("'text'", "string"),
        ("{}", "object"),
        ("[]", "object"),
    ] {
        let source = format!(
            "export function run(a,b){{const x={{v:{value}}};return typeof x.v==='{kind}'?a:b;}}"
        );
        common::compare_sources(&source, &source);
    }
}

#[test]
fn typeof_values_and_mutations_preserve_operand_order_and_snapshots() {
    for source in [
        "export function run(a,b){const x={};return typeof x.missing==='undefined'?a:b;}",
        "export function run(a,b){const x=[,a];return typeof x[0]==='undefined'&&typeof x[1]==='number'&&typeof x.length==='number'?a:b;}",
        "export function run(a,b){const x={v:a};const before=typeof x.v;x.v='s';const after=typeof x.v;return before==='number'&&after==='string'?a:b;}",
        "export function run(a,b){let n=0;const x={v:a};const kind=typeof(n++,x.v=(n++,b));return n===2&&kind==='number'?a:b;}",
        "export function run(a,b){let n=0;typeof(n++,{});return n;}",
        "export function run(a,b){let n=0;const kind=typeof(n++);return kind==='number'&&n===1?a:b;}",
        "export function run(a,b){const kind=typeof(a?{}:'');return kind===(a?'object':'string')?a:b;}",
        "export function run(a,b){return typeof typeof a==='string'?a:b;}",
        "export function run(a,b){return typeof undefined==='undefined'&&typeof NaN==='number'&&typeof Infinity==='number'?a:b;}",
        "export function run(undefined,b){return typeof undefined==='number'?undefined:b;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn typeof_results_flow_through_helpers_and_erase_metadata() {
    let js = "function kind(x){return typeof x;}export function run(a,b){const o={};const first=kind(a);const second=kind('');const third=kind(o);return first==='number'&&second==='string'&&third==='object'?a:b;}";
    let ts = "function kind(x:number):number{return typeof(x as object);}export function run(a:string,b:null):number{const o={};const first=kind(a);const second=kind('');const third=kind(o);return first==='number'&&second==='string'&&third==='object'?a:b;}";
    common::compare_sources(js, ts);
    let discarded =
        "function kind(x){return typeof x;}export function run(a,b){let n=0;kind(n++);return n;}";
    common::compare_sources(discarded, discarded);
}

#[test]
fn typeof_callable_values_does_not_call_them() {
    for source in [
        "function f(x){return x+1;}export function run(a,b){return typeof f==='function'?a:b;}",
        "function f(x){return x;}export function run(f,b){return typeof f==='number'?f:b;}",
        "export function run(a,b){let n=0;const f=()=>++n;const kind=typeof f;return kind==='function'&&n===0&&f()===1?a:b;}",
        "export function run(a,b){let n=0;const kind=typeof(n++,()=>a);return n===1&&kind==='function'?a:b;}",
        "export function run(a,b){const kind=typeof(a?(()=>a):(()=>b));return kind==='function'?a:b;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn typeof_keeps_global_tdz_and_unrepresented_operand_boundaries() {
    for source in [
        "export function run(a,b){return typeof unknownGlobal==='undefined'?a:b;}",
        "export function run(a,b){return typeof Object==='undefined'?a:b;}",
        "export function run(a,b){const kind=typeof x;const x=a;return kind==='undefined'?a:b;}",
        "function f(x){return x;}export function run(a,b){const kind=typeof f;const f=()=>a;return kind==='function'?a:b;}",
        "export function run(a,b){return typeof null.x==='undefined'?a:b;}",
        "export function run(a,b){return typeof 1n==='bigint'?a:b;}",
        "export function run(a,b){return typeof Symbol()==='symbol'?a:b;}",
        "export function run(a,b){let n=0;const kind=typeof Function((n++,'x'),(n++,'return x;'));return n===2&&kind==='function'?a:b;}",
        "export function run(a,b){return typeof new TypeError()==='object'?a:b;}",
        "function f(x){return missing(x);}export function run(a,b){return typeof f==='function'?a:b;}",
        "export function run(a,b){return typeof Promise.resolve(a)==='object'?a:b;}",
        "export function run(a,b){return typeof(a?()=>a:b)==='function'?a:b;}",
        "export function run(a,b){const kind=typeof a;return kind;}",
        "export async function run(a,b){const kind=typeof a;await b;return kind==='number'?a:b;}",
    ] {
        for ts in [false, true] {
            assert!(
                grunkalunka_compiler::compile(source, ts).is_err(),
                "{source}"
            );
        }
    }
}

#[test]
fn typeof_traversals_keep_expression_limits_on_a_small_stack() {
    for (depth, admitted) in [(16, true), (128, true), (129, false)] {
        let source = format!(
            "export function run(a,b){{const kind={}a;return kind==='string'?a:b;}}",
            "typeof ".repeat(depth)
        );
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                for ts in [false, true] {
                    let result = grunkalunka_compiler::compile(&source, ts);
                    assert_eq!(
                        result.is_ok(),
                        admitted,
                        "{source}: {:?}",
                        result.as_ref().err()
                    );
                    if let Err(error) = result {
                        assert!(error.contains("expression nesting exceeds"), "{error}");
                    }
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
    for (depth, admitted) in [(16, true), (127, true), (128, false)] {
        let source = format!(
            "function f(x){{return x;}}export function run(a,b){{const kind=typeof {}f{};return kind==='function'?a:b;}}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                for ts in [false, true] {
                    let result = grunkalunka_compiler::compile(&source, ts);
                    assert_eq!(
                        result.is_ok(),
                        admitted,
                        "{source}: {:?}",
                        result.as_ref().err()
                    );
                    if let Err(error) = result {
                        assert!(error.contains("expression nesting exceeds"), "{error}");
                    }
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}

#[test]
fn typeof_comparisons_preserve_primitive_export_kinds() {
    use grunkalunka_compiler::compile_value;
    use grunkalunka_runtime::{Value, ValueInstance};
    for ts in [false, true] {
        let artifact = compile_value(
            "export function run(a,b){const o={v:a};return typeof o.v==='number';}",
            ts,
        )
        .unwrap();
        let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
        for a in [0.0, -0.0, 1.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                instance.run(a, 2.0).unwrap(),
                Value::Boolean(true)
            ));
        }
    }
}

#[test]
fn typeof_does_not_admit_untraced_async_reference_frames() {
    for source in [
        "export async function run(a,b){const kind=typeof a;await b;return kind==='number'?a:b;}",
        "function kind(x){return typeof x==='number'?1:0;}export async function run(a,b){await a;return kind(b)?a+b:a;}",
    ] {
        for ts in [false, true] {
            let result = grunkalunka_compiler::compile_async(source, ts);
            assert!(result.is_err(), "{source}");
            let error = result.err().unwrap();
            assert!(error.contains("managed") || error.contains("GC"), "{error}");
        }
    }
}

#[test]
fn typeof_combines_with_managed_defaults_and_nullable_helper_inputs() {
    for source in [
        "export function run(a,b){const o={v:a?null:''};return typeof(o.v??[])===(a?'object':'string')?a:b;}",
        "export function run(a,b){const o={v:a};return typeof(o.v&&'s')===(a?'string':'number')?a:b;}",
        "function kind(x){return typeof x;}export function run(a,b){return kind(null)==='object'?a:b;}",
        "function choose(x,y){return x??y;}export function run(a,b){return typeof choose(a?null:'',[])===(a?'object':'string')?a:b;}",
    ] {
        common::compare_sources(source, source);
    }
}
