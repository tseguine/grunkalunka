#[test]
fn checked_factory_closures_preserve_managed_result_consumers() {
    for body in [
        "const f=x=>()=>x;const g=f();return g();",
        "const f=x=>()=>x;const g=f(false);return g();",
        "const f=x=>()=>x;const g=f(null);return g();",
        "const f=x=>()=>x;const g=f(a?null:b);return g();",
        "const f=x=>()=>x;const g=f();const alias=g;return alias();",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let result = crate::compile_value(&source, typed);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
    for body in [
        "const f=x=>()=>x;const g=f();return +g();",
        "const f=x=>()=>x;const g=f('x');return g()==='x'?a:b;",
        "const f=x=>()=>x;const g=f({v:a});return +g().v;",
        "const f=x=>()=>x;const g=f([a]);return +g()[0];",
        "const f=x=>()=>x;const g=f();return typeof g()==='undefined'?a:b;",
        "let n=0;const f=x=>()=>{n++;return x;};const g=f();return +(g()??b)+n;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let result = crate::compile(&source, typed);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
}

#[test]
fn managed_factory_results_keep_numeric_return_and_definition_guards() {
    for source in [
        "export function run(a,b){const f=x=>()=>x;const g=f();return g();}",
        "export function run(a,b){const f=x=>()=>Math.noSuch(x);const g=f(a);return a;}",
        "export function run(a,b){const f=x=>()=>x;const g=f(unknown());return a;}",
    ] {
        assert!(crate::compile(source, false).is_err(), "{source}");
    }
}

#[test]
fn number_result_facts_do_not_require_number_only_logical_operands() {
    for typed in [false, true] {
        for source in [
            "export function run(a,b) {return null||a;}",
            "export function run(a,b) {const value=a?null:b;return value||a;}",
            "export function run(a,b) {return false??'x';}",
            "export function run(a,b){const f=x=>x??b;return f(null);}",
        ] {
            let result = crate::compile_value(source, typed);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
        let source = "export function run(a,b) {return ((undefined)||(0))===(0)?1:0;}";
        let result = crate::compile(source, typed);
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
}

#[test]
fn logical_function_writes_preserve_nested_receiver_provenance() {
    let source = "export function run(a,b){const left={child:{x:a}},right={child:{x:b}};let pick=()=>left.child;pick&&=()=>right.child;const o={child:pick()};return o.child.x===b?1:0;}";
    for typed in [false, true] {
        let result = crate::compile(source, typed);
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
}
