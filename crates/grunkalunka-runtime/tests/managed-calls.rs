#[path = "managed/common.rs"]
mod common;
use common::compare_sources;

#[test]
fn helper_mutation_preserves_argument_and_result_identity() {
    let js = "function set(o,v){o.x=v;return o;} export function run(a,b){const o={x:a};const alias=set(o,b);return alias===o&&o.x===b?a:b;}";
    compare_sources(js, js);
}

#[test]
fn helper_construction_forwarding_and_recursion_keep_references() {
    for js in [
        "function create(v){return {x:v};} function forward(o){return o;} export function run(a,b){const o=create(a);const alias=forward(o);alias.x=b;return o===alias&&o.x===b?a:b;}",
        "function first(o,n){if(n===0)return o;return second(o,n-1);} function second(o,n){if(n===0)return o;return first(o,n-1);} export function run(a,b){const o={};return first(o,3)===o?a:b;}",
        "function choose(flag,o){if(flag)return o;return {x:0};} export function run(a,b){const o={};const yes=choose(1,o);const no=choose(0,o);return yes===o&&no!==o?a:b;}",
    ] {
        compare_sources(js, js);
    }
}

#[test]
fn mixed_helper_values_preserve_tags_and_reference_identity() {
    let js = "function identity(v){return v;} export function run(a,b){const o={};return identity(o)===o&&identity(a)===a&&identity(null)===null&&identity(undefined)===undefined&&identity(false)===false?a:b;}";
    compare_sources(js, js);
    let js = "function choose(flag,o,n){if(flag)return o;return n;} export function run(a,b){const o={};return choose(1,o,a)===o&&choose(0,o,a)===a?a:b;}";
    compare_sources(js, js);
    let js = "function get(o){return o.x;} function put(o,v){return o.x=v;} export function run(a,b){const inner={};const o={x:inner};const saved=get(o);const result=put(o,b);return saved===inner&&get(o)===b&&result===b?a:b;}";
    compare_sources(js, js);
}

#[test]
fn helper_arguments_and_results_keep_evaluation_order_and_snapshots() {
    for js in [
        "function first(x,y){x.x=y;return x;} export function run(a,b){let o={};const saved=o;const result=first(o,(o={}));return result===saved&&saved.x===o&&o!==saved?a:b;}",
        "function identity(o){return o;} export function run(a,b){const o={};let n=0;const alias=identity.call(n=1,(n=2,o),n=3);return alias===o&&n===3?a:b;}",
        "function identity(o){return o;} export function run(a,b){const o={};let n=0;const alias=identity.call(n=1,o);return alias===o&&n===1?a:b;}",
        "function rebind(o,p){const saved=o;o=p;return saved;} export function run(a,b){const o={};const p={};return rebind(o,p)===o?a:b;}",
    ] {
        compare_sources(js, js);
    }
}

#[test]
fn helper_signature_flow_uses_resolved_symbols_not_annotations_or_names() {
    let js = "function identity(o){return o;} function numeric(identity){return identity+1;} export function run(a,b){const o={x:a};const alias=identity(o);return alias===o&&numeric(2)===3?a:b;}";
    let ts = "function identity(o:number):number{return o;} function numeric(identity:{x:string}):object{return identity+1;} export function run(a:string,b:null):boolean{const o={x:a};const alias=identity(o);return alias===o&&numeric(2)===3?a:b;}";
    compare_sources(js, ts);
}

#[test]
fn helper_results_work_as_receivers_values_and_discarded_effects() {
    for js in [
        "function create(v){return {x:v};} export function run(a,b){return create(a).x===a?a:b;}",
        "function set(o,v){o.x=v;return o;} export function run(a,b){const o={x:a};set(o,b);return o.x===b?a:b;}",
        "function get(o){return o.x;} function identity(v){return v;} export function run(a,b){const o={x:a};const copy={x:identity(get(o))};return copy.x===a?a:b;}",
        "function replace(v){v={};return v;} export function run(a,b){return replace(a)!==a?a:b;}",
        "function numeric(v){return v+1;} function identity(o){return o;} export function run(a,b){const o={};identity(o);return numeric(a)+b;}",
    ] {
        compare_sources(js, js);
    }
}

#[test]
fn helper_observations_keep_existing_primitive_export_kinds() {
    use grunkalunka_compiler::compile_value;
    use grunkalunka_runtime::{Value, ValueInstance};
    for ts in [false, true] {
        let artifact=compile_value("function identity(o){return o;} function get(o){return o.x;} export function run(a,b){const o={x:a};return identity(o)===o&&get(o)===a;}",ts).unwrap();
        let mut equality = ValueInstance::new(&artifact.wasm).unwrap();
        let artifact=compile_value("function get(o){return o.x;} export function run(a,b){const o={x:a};return get(o)?null:undefined;}",ts).unwrap();
        let mut nullish = ValueInstance::new(&artifact.wasm).unwrap();
        for a in [0.0_f64, -0.0, 1.0, -7.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(equality.run(a,a).unwrap(),Value::Boolean(v) if v==!a.is_nan()));
            let value = nullish.run(a, 7.0).unwrap();
            assert!(if a != 0.0 && !a.is_nan() {
                matches!(value, Value::Null)
            } else {
                matches!(value, Value::Undefined)
            });
        }
    }
}

#[test]
fn helper_source_queries_remain_bounded_on_a_small_native_stack() {
    for (depth, admitted) in [(16, true), (140, false)] {
        let expression = format!("{}o{}", "(".repeat(depth), ")".repeat(depth));
        let source = format!(
            "function identity(o){{return {expression};}} export function run(a,b){{const o={{}};return identity(o)===o?a:b;}}"
        );
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || grunkalunka_compiler::compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(result.is_ok(), admitted);
        if let Err(e) = result {
            assert!(e.contains("nesting"), "{e}");
        }
    }
}

#[test]
fn helpers_keep_unimplemented_reference_boundaries_explicit() {
    for source in [
        "function read(o){return o.x;} export function run(a,b){read({});return read(a);}",
        "function identity(o){return o;} export function run(a,b){return identity({});}",
        "function identity(o){return o;} export function run(a,b){const o={};return identity();}",
        "function identity(o){return o;} export function run(a,b){const o={};return identity(...o);}",
        "function identity(o){return o;} export function run(a,b){const o={};const f=identity;return f(o);}",
        "function identity(o){return o;} export function run(a,b){const o={};return identity(o)+a;}",
        "function bad(o){if(0)return o.toString;return o;} export function run(a,b){const o={};return bad(o)===o?a:b;}",
        "function capture(o){const f=()=>o;return f();} export function run(a,b){const o={};return capture(o)===o?a:b;}",
    ] {
        for ts in [false, true] {
            let e = grunkalunka_compiler::compile(source, ts)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(e.contains("unsupported"), "{source}: {e}");
            assert!(!e.contains("internal IR"), "{source}: {e}");
        }
    }
}
