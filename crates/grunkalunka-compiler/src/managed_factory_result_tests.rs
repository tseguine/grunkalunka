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
