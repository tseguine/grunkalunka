#[test]
fn synchronous_private_helpers_complete_undefined_without_value_returns() {
    for source in [
        "function f(x){}export function run(a,b){return f(a);}",
        "function f(x){x++;}export function run(a,b){return f(a);}",
        "function f(x){if(x)return;}export function run(a,b){return f(a);}",
        "function f(x){const g=()=>x;}export function run(a,b){return f(a);}",
        "function f(o){o.x++;}export function run(a,b){const o={x:a};const result=f(o);return result??o.x;}",
    ] {
        for ts in [false, true] {
            let result = crate::compile_value(source, ts);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
    for body in ["", "x++;", "if(x)return;"] {
        let source = format!("function f(x){{{body}}}export function run(a,b){{return +f(a);}}");
        let result = crate::compile(&source, false);
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
}

#[test]
fn implicit_helper_results_keep_number_and_source_guards() {
    for source in [
        "function f(x){}export function run(a,b){return f(a);}",
        "function f(x){x++;}export function run(a,b){return f(a);}",
        "function f(x){if(x)return x;}export function run(a,b){return f(a);}",
    ] {
        assert!(crate::compile(source, false).is_err(), "{source}");
    }
    for source in [
        "function f(x){if(x)return null;}export function run(a,b){return +f(a);}",
        "function f(x){Math.random();}export function run(a,b){return f(a);}",
        "function f(x){return;Math.random();}export function run(a,b){return f(a);}",
    ] {
        assert!(crate::compile_value(source, false).is_err(), "{source}");
    }
}

#[test]
fn side_effect_only_helpers_do_not_change_unrelated_async_signatures() {
    for prefix in ["function unused(x){x++;}", "function unused(x){}"] {
        for call in ["", "unused(a);"] {
            let source = format!(
                "{prefix}export async function run(a,b){{{call}const f=async x=>{{await x;return x;}};const make=p=>async()=>{{await a;return await p;}};const get=make(f(b));return await get();}}"
            );
            let result = crate::compile_async(&source, false);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
}
