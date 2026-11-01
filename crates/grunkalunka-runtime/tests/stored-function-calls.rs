#[path = "managed/common.rs"]
mod common;

#[test]
fn functions_in_own_properties_remain_callable() {
    let source = "export function run(a,b){const f=()=>a;const o={f};return o.f();}";
    common::compare_sources(source, source);
}

fn compare(body: &str) {
    let source = format!("export function run(a,b){{{body}}}");
    common::compare_sources(&source, &source);
}

#[test]
fn field_calls_keep_aliases_arguments_and_snapshots() {
    for body in [
        "const o=[()=>a];return o[0]();",
        "const f=x=>x;const o={f};return o.f(a);",
        "const f=x=>x;const o={f};const g=o.f;return g(a);",
        "const o={f:()=>a};const p=o;p.f=()=>b;return o.f();",
        "const o={f:()=>a};const old=o.f;o.f=()=>b;return old()+o.f();",
        "const o={f:x=>x};return o.f((o.f=x=>b,a));",
        "let n=0;const o={f:x=>x};return (n++,o).f((n++,a),(n++,b))+n;",
        "const o={f:()=>a};const p={f:()=>b};return (a?o:p).f();",
    ] {
        compare(body);
    }
}

#[test]
fn field_calls_transport_managed_arguments_and_results() {
    for body in [
        "const f=x=>x;const o={f};const value={};return o.f(value)===value?1:0;",
        "const f=x=>x;const o={f};const value=[a,b];return o.f(value)===value?1:0;",
        "const f=x=>x;const o={f};return o.f('text')==='text'?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn field_calls_follow_factory_and_helper_origins() {
    for body in [
        "const make=()=>({f:()=>a});const o=make();return o.f();",
        "const make=fn=>({f:fn});const o=make(()=>a);return o.f();",
        "const make=x=>()=>x;const o={f:make(a)};return o.f();",
        "const apply=o=>o.f();const o={f:()=>a};return apply(o);",
    ] {
        compare(body);
    }
}

#[test]
fn array_resize_cannot_turn_a_proven_function_into_a_missing_element() {
    let source = "export function run(a,b){const o=[()=>a];o.length=0;return o[0]();}";
    for ts in [false, true] {
        assert!(grunkalunka_compiler::compile(source, ts).is_err());
    }
    compare("const o=[()=>a];o.length=3;return o[0]();");
}

#[test]
fn stored_callbacks_and_general_values_use_complete_signatures() {
    for body in [
        "const o={f:x=>x};return o.f(true)===true?1:0;",
        "const o={f:x=>x};return o.f(null)===null?1:0;",
        "const o={f:x=>x};return o.f(undefined)===undefined?1:0;",
        "const o={f:fn=>fn(a)};const g=x=>x;return o.f(g);",
        "const o={f:()=>()=>a};const g=o.f();return g();",
        "const make=n=>({f:()=>n});const o=make(a),p=make(b);return o.f()+p.f();",
    ] {
        compare(body);
    }
    common::compare_sources(
        "function id(x){return x;}export function run(a,b){const o={id};const x={};return o.id(x)===x?1:0;}",
        "function id<T>(x:boolean):string{return x;}export function run(a:string,b:never):void{const o={id};const x={};return o.id<Missing>(x as number)===x?1:0;}",
    );
}

#[test]
fn receivers_keys_and_explicit_call_receivers_preserve_effects() {
    for body in [
        "let n=0;const o={f:()=>a};const yes=typeof(n++,o).f==='function';return yes?n:a;",
        "let n=0;const o={f:x=>x};return o.f.call((n++,b),(n++,a),(n++,b))+n;",
        "const o={'😀':x=>x};return o['😀'](a);",
        "const o={f:()=>b,f:()=>a};return o.f();",
        "const o={f:()=>a};const change=x=>{x.f=()=>b;return x;};change(o);return o.f();",
    ] {
        compare(body);
    }
}

#[test]
fn missing_mixed_opaque_and_async_fields_remain_explicit_rejections() {
    for body in [
        "const o={};return o.f();",
        "const o={};o.f=()=>a;return o.f();",
        "const o=a?{f:()=>a}:{};return o.f();",
        "const o={f:a?()=>a:b};return o.f();",
        "const o={f:()=>a};o.f=b;return o.f();",
        "const o={f:()=>a};o.f=b;const g=o.f;return g();",
        "const o={f:()=>a};o.f=b;const apply=fn=>fn();return apply(o.f);",
        "const o={f:()=>a};o.f=(x,y)=>x+y;return o.f();",
        "const o={f:()=>a};o.f=()=>true;return o.f();",
        "const o={f:()=>a};o.f=()=>({});return o.f();",
        "const o=[()=>a];const p=o;p.length=0;return o[0]();",
        "const o=[,()=>a];return o[0]();",
        "const o={f:()=>a};return o[a]();",
        "const o=[()=>a];return o[a]();",
        "const o={f:async()=>a};return o.f();",
        "const o={constructor:()=>a};return o.constructor();",
        "const o={f:()=>a};return o.f;",
        "const o={child:{f:()=>a}};return o.child.f();",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let result = std::panic::catch_unwind(|| grunkalunka_compiler::compile(&source, ts))
                .unwrap_or_else(|_| panic!("compiler panicked for {source}"));
            assert!(result.is_err(), "unexpected admission: {source}");
        }
    }
}

#[test]
fn field_calls_retain_returned_reference_environments_and_surplus_effects() {
    for body in [
        "const value={};const make=x=>({f:()=>x});const o=make(value);return o.f()===value?1:0;",
        "let n=0;const make=fn=>({f:fn});const o=make.call((n++,b),(n++,()=>a),(n++,b));return o.f()+n;",
        "const o={f:()=>a};return o.f((o.f=()=>b,b))+o.f();",
        "let o={f:x=>x};return o.f((o={f:x=>b},a));",
    ] {
        compare(body);
    }
    common::compare_sources(
        "function make(n){return {f:()=>n};}export function run(a,b){const o=make(a);return o.f();}",
        "function make<T>(n:boolean):string{return {f:()=>n};}export function run(a:string,b:boolean):never{const o=make<Missing>(a);return o.f();}",
    );
    let js = "export function run(a,b){const o=[()=>a];o.length=3;return o[0]();}";
    let ts = "type Unused='metadata';export function run(a:string,b:boolean):void{const o=[()=>a];o.length=(3 as unknown)!;return o[(0 as never)]();}";
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm
    );
    common::compare_sources(js, ts);
}

#[test]
fn field_origin_queries_preserve_two_megabyte_stacks_and_source_bounds() {
    for (aliases, admitted) in [(24, true), (129, false)] {
        let names = (1..=aliases)
            .map(|i| format!("const o{i}=o{};", i - 1))
            .collect::<String>();
        let source = format!(
            "export function run(a,b){{const o0={{f:()=>a}};{names}return o{aliases}.f();}}"
        );
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || grunkalunka_compiler::compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(result.is_ok(), admitted, "{:?}", result.as_ref().err());
        if let Err(error) = result {
            assert!(error.contains("unsupported"), "{error}");
        }
    }
}

#[test]
fn former_opaque_field_guards_execute_the_original_programs() {
    for source in [
        "function f(x){return x;}export function run(a,b){const o={f};return o.f(a);}",
        "function id(x){return x;}export function run(a,b){const o={id};const apply=(fn,x)=>fn(x);return apply(o.id,{})===o?1:0;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn primitive_entry_field_calls_preserve_value_kinds() {
    use grunkalunka_runtime::{Value, ValueInstance};
    for (input, expected) in [
        ("undefined", Value::Undefined),
        ("null", Value::Null),
        ("true", Value::Boolean(true)),
        ("a", Value::Number(-0.0)),
    ] {
        let source = format!("export function run(a,b){{const o={{f:x=>x}};return o.f({input});}}");
        for ts in [false, true] {
            let artifact = grunkalunka_compiler::compile_value(&source, ts).unwrap();
            let actual = ValueInstance::new(&artifact.wasm)
                .unwrap()
                .run(-0.0, 7.0)
                .unwrap();
            match (&actual, &expected) {
                (Value::Number(a), Value::Number(b)) => assert_eq!(a.to_bits(), b.to_bits()),
                (Value::Undefined, Value::Undefined) | (Value::Null, Value::Null) => (),
                (Value::Boolean(a), Value::Boolean(b)) => assert_eq!(a, b),
                _ => panic!("expected {expected:?}, got {actual:?}"),
            }
        }
    }
}
