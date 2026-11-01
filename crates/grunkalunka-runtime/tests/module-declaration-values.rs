#[path = "managed/common.rs"]
mod common;

#[test]
fn declaration_aliases_are_callable_and_keep_module_identity() {
    let source = "function f(x){return x;}export function run(a,b){const g=f;const o={f,g};return o.f===o.g&&g(a)===f(a)?1:0;}";
    common::compare_sources(source, source);
}

fn compare(source: &str) {
    common::compare_sources(source, source);
}

#[test]
fn declarations_are_hoisted_and_distinct_from_other_creations() {
    for source in [
        "export function run(a,b){const o={f};return o.f===f?1:0;}function f(x){return x;}",
        "function f(x){return x;}function g(x){return x;}export function run(a,b){const o=[f,g];return o[0]===f&&o[1]===g&&f!==g?1:0;}",
        "function f(x){return x;}export function run(a,b){const g=x=>x;return f!==g&&[f,f][0]===[f][0]?1:0;}",
        "export function run(a,b){const f=run;return f===run&&[f][0]===run?1:0;}",
        "function f(){return 3;}export function run(a,b){const g=f;return g()+f();}",
        "function f(x,y,z){return x+y+z;}export function run(a,b){const g=f;return g(a,b,2);}",
    ] {
        compare(source);
    }
}

#[test]
fn declaration_values_use_existing_callback_and_snapshot_rules() {
    for source in [
        "function f(x){return x;}export function run(a,b){const call=(fn,x)=>fn(x);return call(f,a);}",
        "function f(x){return x;}export function run(a,b){const wrap=fn=>({fn});return wrap(f).fn===f?1:0;}",
        "function f(x){return x;}function g(x){return -x;}export function run(a,b){let h=f;const old=h;const o={h};h=g;return o.h===old&&old===f&&h===g&&old!==h?1:0;}",
        "function f(x){return x;}export function run(a,b){let n=0;const h=(n++,f);const o={h:(n++,h)};return n===2&&o.h===f?h(a):b;}",
        "function f(x){return x;}export function run(a,b){const h=a?f:f;return h===f?h(b):a;}",
        "function f(x){return x;}export function run(a,b){const h=(f.call)(b,a);const g=f;return g.call(b,h);}",
    ] {
        compare(source);
    }
}

#[test]
fn declaration_values_preserve_managed_parameters_and_results() {
    for source in [
        "function make(x){return {x};}export function run(a,b){const f=make;const o=f(a);return o.x===a?1:0;}",
        "function id(x){return x;}export function run(a,b){const o={x:a};const f=id;return f(o)===o?1:0;}",
        "function make(x){return [x];}export function run(a,b){const f=make;const call=(fn,x)=>fn(x);return call(f,a)[0]===a?1:0;}",
        "function make(x){return {x};}export function run(a,b){const f=make;const o={f};return o.f===make&&f(a).x===a?1:0;}",
        "function f(x){return x;}export function run(a,b){const o={};o.f=f;o.self=o;const x=[f,o];o.x=x;return o.f===f&&x[0]===o.f&&o.x===x&&x[1]===o?1:0;}",
    ] {
        compare(source);
    }
}

#[test]
fn declarations_keep_symbol_identity_and_erased_annotations() {
    for source in [
        "function f(x){return x;}function check(x){const o=[f];return o[0]===f?x:0;}export function run(a,b){const f=x=>-x;return [f][0]===f?check(a):b;}",
        "function f(x){return x;}export function run(a,b){return typeof f==='function'?1:0;}",
        "function f(x,y,z){return x+y+z;}export function run(a,b){return f.length;}",
        "function f(x){return x;}export function run(a,b){return ((f))(a);}",
        "function id(x){return x;}export function run(a,b){let f=id;const g=f;return g('text')==='text'?1:0;}",
    ] {
        compare(source);
    }
    common::compare_sources(
        "function f(x){return x;}export function run(a,b){const g=f;const o={g};return o.g===f?g(a):b;}",
        "function f<T>(x:string):boolean{return x;}export function run(a:object,b:string):void{const g=(f as unknown)!;const o={g};return o.g===f?g(a):b;}",
    );
}

#[test]
fn declaration_general_results_keep_the_primitive_public_boundary() {
    for ts in [false, true] {
        let source = "function f(x){if(x)return null;return false;}export function run(a,b){const g=f;return g(a);}";
        let artifact = grunkalunka_compiler::compile_value(source, ts).unwrap();
        let mut instance = grunkalunka_runtime::ValueInstance::new(&artifact.wasm).unwrap();
        for a in [0.0, -0.0, 1.0, -7.0, f64::NAN, f64::INFINITY] {
            let actual = instance.run(a, 0.0).unwrap();
            if a == 0.0 || a.is_nan() {
                assert!(matches!(actual, grunkalunka_runtime::Value::Boolean(false)));
            } else {
                assert!(matches!(actual, grunkalunka_runtime::Value::Null));
            }
        }
    }
}

#[test]
fn declaration_values_keep_unsupported_constructs_explicit() {
    for source in [
        "function f(x){return x;}export function run(a,b){f=x=>x;return a;}",
        "function f(x){return x;}export function run(a,b){const g=f;return g();}",
        "function f(x){return x;}function g(x,y){return x+y;}export function run(a,b){const h=a?f:g;return h(a,b);}",
        "export function run(a,b){function f(){return a;}const g=f;return g();}",
        "function f(){return Symbol();}export function run(a,b){const o={f};return a;}",
        "function make(x){return {x};}export function run(a,b){const f=make;return f(a).x;}",
        "function f(x){return x;}export async function run(a,b){const g=f;const h=async x=>{await b;return x;};const p=h(a);await p;return g(a);}",
    ] {
        for ts in [false, true] {
            let result = if source.contains("export async") {
                grunkalunka_compiler::compile_async(source, ts)
            } else {
                grunkalunka_compiler::compile(source, ts)
            };
            let error = result
                .err()
                .unwrap_or_else(|| panic!("unexpected admission: {source}"));
            assert!(!error.contains("internal IR"), "{source}: {error}");
            assert!(
                error.contains("unsupported") || error.contains("reference-aware async"),
                "{source}: {error}"
            );
        }
    }
}

#[test]
fn declaration_adapters_keep_direct_calls_specialized_and_native_stack_bounded() {
    let source = "function f(x){return x;}export function run(a,b){return f(a);}";
    let artifact = grunkalunka_compiler::compile(source, false).unwrap();
    assert!(!artifact.wat.contains("$gc_module_function"));
    assert!(!artifact.wat.contains("$gc_function"));
    let typeof_source =
        "function f(x){return x;}export function run(a,b){return typeof f==='function'?a:b;}";
    let artifact = grunkalunka_compiler::compile(typeof_source, false).unwrap();
    assert!(!artifact.wat.contains("$gc_module_function"));
    for (depth, admitted) in [(16, true), (140, false)] {
        let source = format!(
            "function f(x){{return x;}}export function run(a,b){{const g={}f{};const o={{g}};return o.g===f?g(a):b;}}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || grunkalunka_compiler::compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(
            result.is_ok(),
            admitted,
            "depth={depth}: {:?}",
            result.as_ref().err()
        );
        if let Err(error) = result {
            assert!(error.contains("nesting"), "{error}");
        }
    }
}

#[test]
fn declaration_adapters_keep_the_existing_thousand_parameter_boundary() {
    let parameters = (0..1000)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let arguments = (0..1000)
        .map(|i| {
            if i == 0 {
                "a"
            } else if i == 999 {
                "b"
            } else {
                "0"
            }
        })
        .collect::<Vec<_>>()
        .join(",");
    let source = format!(
        "function many({parameters}){{return p0+p999;}}export function run(a,b){{const g=many;return g({arguments});}}"
    );
    // Wide typed frames are checked with an explicit engine-test budget; the
    // production NumericInstance per-call fuel policy remains unchanged.
    common::compare_sources_with_fuel(&source, &source, Some(2_000_000));
}
