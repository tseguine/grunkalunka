#[path = "managed/common.rs"]
mod common;

#[test]
fn local_closure_object_arguments_preserve_mutation_and_snapshots() {
    let js = "export function run(a,b){const o={x:a};const old=o.x;let set=(target,n)=>{target.x=n;return 0;};set(o,b);return old===a&&o.x===b?a:b;}";
    common::compare_sources(js, js);
}

#[test]
fn known_alias_and_selected_object_parameters_keep_actual_kinds() {
    for body in [
        "const o={x:a};const f=x=>x.x===a?a:b;const alias=f;return alias(o);",
        "const o={x:a};const f=a?x=>x.x===a?a:b:x=>x.x===b?b:a;return f(o);",
        "const o={x:a};let f=x=>x.x===a?a:b;f=x=>x.x===b?b:a;return f(o);",
        "const o={x:a};const old=o;let f=x=>{x={x:b};return x!==old?a:b;};f(o);return o===old&&o.x===a?a:b;",
        "const o={x:a};let f=x=>x.x===a?a:b;return f(o,(o.x=b,a++));",
        "const o={x:a};let f=v=>v.x===a?a:b;return f(o);",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&js, &js);
    }
}
#[test]
fn local_array_and_string_parameters_keep_identity_and_content() {
    for body in [
        "const x=[a];let f=v=>v[0]===a?a:b;return f(x);",
        "const x=[a];let f=v=>{v[0]=b;return 0;};f(x);return x[0]===b?a:b;",
        "const x=[];x[0]=x;const f=v=>v[0]===v?a:b;return f(x);",
        "let f=s=>s==='text'?a:b;return f('text');",
        "let f=s=>s==='text'?a:b;f('other');return f('text');",
        "const f=a?s=>s==='text'?a:b:s=>s==='other'?b:a;return f('text');",
        "let f=v=>v.length;return f([a,b]);",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&js, &js);
    }
}
#[test]
fn managed_arguments_flow_through_callbacks_and_factory_results() {
    for body in [
        "const o={x:a};const apply=(fn,x)=>fn(x);return apply(v=>v.x===a?a:b,o);",
        "const o={x:a};const apply=a?(fn,x)=>fn(x):(fn,x)=>fn(x);return apply(v=>v.x===a?a:b,o);",
        "let make=x=>()=>x.x===a?a:b;const o={x:a};const f=make(o);o.x=b;return f();",
        "const factory=a?()=>x=>x.x===a?a:b:()=>x=>x.x===b?b:a;const f=factory();return f({x:a});",
        "const make=a?x=>()=>x.x===a?a:b:x=>()=>x.x===b?b:a;const f=make({x:a});return f();",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&js, &js);
    }
}

#[test]
fn managed_parameter_joins_keep_values_and_call_evaluation_order() {
    for body in [
        "let f=v=>typeof v==='object'?a:b;f({});return f(a);",
        "let f=v=>v===a?a:b;f({});return f(a);",
        "let f=v=>v==='text'?a:b;f({});f([a]);f(null);return f('text');",
        "const x={v:a};let f=v=>{const old=v;v=b;return typeof old==='object'&&v===b?a:b;};return f(x);",
        "const x={v:a};let f=v=>v.x===a?a:b;let alias=x;alias={x:b};return f(alias);",
        "let f=v=>typeof v==='undefined'?a:b;f({});return f(undefined);",
        "const x={v:a};const f=v=>v===x?a:b;return f(x,(x.v=b,a++));",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&js, &js);
    }
}

#[test]
fn managed_parameter_annotations_do_not_override_source_values() {
    common::compare_sources(
        "export function run(a,b){const f=x=>x.x===a?a:b;return f({x:a});}",
        "export function run(a:boolean,b:string):void{const f=(x:number):string=>x.x===a?a:b;return f({x:a});}",
    );
}

#[test]
fn unknown_receivers_managed_results_and_async_storage_still_reject() {
    for source in [
        "export function run(a,b){let f=v=>v.x;f({x:a});return f(a);}",
        "export function run(a,b){let f=v=>v;return f({})===null?a:b;}",
        "export function run(a,b){let f=v=>v[b];return f({x:a});}",
        "export function run(a,b){let f=v=>v.toString();return f({});}",
        "export function run(a,b){let f=v=>v.x===a?a:b;return f(a);}",
        "export function run(a,b){let f=v=>v.x===a?a:b;f({x:a});return f();}",
        "export async function run(a,b){let f=v=>v.x===a?a:b;await 0;return f({x:a});}",
    ] {
        for ts in [false, true] {
            let error = grunkalunka_compiler::compile(source, ts)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(error.contains("unsupported"), "{source}: {error}");
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

#[test]
fn managed_parameter_planning_keeps_small_native_stack_limits() {
    for (depth, admitted) in [(16, true), (140, false)] {
        let source = format!(
            "export function run(a,b){{const f=v=>{}(v.x===a?a:b){};return f({{x:a}});}}",
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
