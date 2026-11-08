#[test]
fn var_identifiers_are_hoisted_and_redeclarations_write_existing_cells() {
    for body in [
        "var x=a;return x+b;",
        "var x=a;return x;",
        "const saved=x;var x=3;return saved;",
        "const before=x===undefined?1:0;var x=a;return before;",
        "var x=1;var x;return x;",
        "var x=1;var x=2;return x;",
        "x=a;var x;return x;",
        "var x; x++;return x;",
        "{var x=a;}return x;",
        "if(a){var x=b;}return x;",
        "for(var i=0;i<3;i++){}return i;",
        "var x='abc';return x.length;",
        "var x={a:a};return x.a;",
        "var x=[a,b];return x[0];",
        "const f=()=>x;var x=3;return f();",
        "const f=()=>{var x=3;return x;};return f();",
        "const f=()=>{const saved=x;var x=3;return saved;};return f();",
        "var a;return a;",
        "var a=null;return a;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let result = crate::compile_value(&source, ts);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
}
#[test]
fn var_helpers_and_default_parameter_body_environments_are_distinct() {
    for source in [
        "function f(x){var y=x;return y;}export function run(a,b){return f(a);}",
        "function f(x){const saved=y;var y=x;return saved;}export function run(a,b){return f(a);}",
        "function f(x){var x;return x;}export function run(a,b){return f(a);}",
        "function f(x=3){var x;return x;}export function run(a,b){return f();}",
        "export function run(a,b){const f=(x=3)=>{var x;return x;};return f();}",
    ] {
        let result = crate::compile_value(source, false);
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
}

#[test]
fn var_number_proofs_follow_initialization_and_account_for_all_writes() {
    for body in [
        "var x=a;return x;",
        "x=a;var x;return x;",
        "var x=a; x=b;return x;",
        "var x; x++;return x;",
        "for(var i=0;i<3;i++){}return i;",
        "var x=1;var x=2;return x;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        let result = crate::compile(&source, false);
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
    for body in [
        "var x;return x;",
        "if(a){var x=b;}return x;",
        "const saved=x;var x=3;return saved;",
        "const change=()=>{x=null;};var x=3;change();return x;",
        "var x=3;({x}={});return x;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(crate::compile(&source, false).is_err(), "{source}");
    }
}

#[test]
fn var_keeps_unreachable_iteration_async_callable_and_lexical_guards() {
    for body in [
        "return x;var x=3;",
        "var f=x=>x;return f(a);",
        "var {x}={x:a};return x;",
        "for(var x of [a]){}return a;",
        "for(var x in {x:a}){}return a;",
        "try{throw a;}catch(e){var e=3;}return 0;",
        "var x=1;let x=2;return x;",
        "var x=1;{const saved=x;let x=2;}return x;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(crate::compile_value(&source, false).is_err(), "{source}");
    }
    assert!(
        crate::compile_async("export async function run(a,b){var x=a;return x;}", false).is_err()
    );
}

#[test]
fn var_annotations_erase_without_changing_hoisting_or_parameter_cells() {
    for (js, ts) in [
        (
            "export function run(a,b){const before=x;var x=a;return before;}",
            "export function run(a:number,b:number):unknown{const before=x;var x:string=a;return before;}",
        ),
        (
            "function f(x=3){var x;return x;}export function run(a,b){return f();}",
            "function f(x:boolean=3):unknown{var x:string;return x;}export function run(a:number,b:number):unknown{return f();}",
        ),
    ] {
        assert_eq!(
            crate::compile_value(js, false).unwrap().wasm,
            crate::compile_value(ts, true).unwrap().wasm
        );
    }
}

#[test]
fn var_source_binding_limits_are_controlled_on_a_small_native_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for count in [128, 129] {
                let names = (0..count)
                    .map(|i| format!("x{i}"))
                    .collect::<Vec<_>>()
                    .join(",");
                let source = format!("export function run(a,b){{var {names};return a;}}");
                for ts in [false, true] {
                    let result = crate::compile_value(&source, ts);
                    if count == 128 {
                        result.unwrap();
                    } else {
                        assert!(result.err().unwrap().contains("at most 128 source locals"));
                    }
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
