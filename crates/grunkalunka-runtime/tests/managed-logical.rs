#[path = "managed/common.rs"]
mod common;

#[test]
fn logical_values_return_the_selected_operand() {
    for source in [
        "export function run(a,b){const x='';const y=x||'fallback';return y==='fallback'?a:b;}",
        "export function run(a,b){const x='left';const y=x||'fallback';return y===x?a:b;}",
        "export function run(a,b){const x='';const y=x&&'right';return y===x?a:b;}",
        "export function run(a,b){const x='left';const y=x&&'right';return y==='right'?a:b;}",
        "export function run(a,b){const x={v:a};const y=x||{v:b};return y===x?a:b;}",
        "export function run(a,b){const x={v:a};const other={v:b};const y=x&&other;return y===other?a:b;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn nullish_defaults_distinguish_absence_from_every_falsy_value() {
    for source in [
        "export function run(a,b){const x={v:null};const y=x.v??'fallback';return y==='fallback'?a:b;}",
        "export function run(a,b){const x={};const y=x.v??'fallback';return y==='fallback'?a:b;}",
        "export function run(a,b){const x={v:undefined};const y=x.v??'fallback';return y==='fallback'?a:b;}",
        "export function run(a,b){const x={v:false};const y=x.v??'fallback';return y===false?a:b;}",
        "export function run(a,b){const x={v:0};const y=x.v??'fallback';return y===0?a:b;}",
        "export function run(a,b){const x={v:0/0};const y=x.v??'fallback';return y!==y?a:b;}",
        "export function run(a,b){const x={v:''};const y=x.v??'fallback';return y===''?a:b;}",
        "export function run(a,b){const x={v:a};const y=x.v??'fallback';return (a!==a?y!==y:y===a)?a:b;}",
        "export function run(a,b){const ref={};const x={v:ref};const y=x.v??'fallback';return y===ref?a:b;}",
        "export function run(a,b){let n=0;const x={v:a?null:'v'};const y=(n++,x.v)??(n+=10,'f');return (n===11&&y==='f')||(n===1&&y==='v')?a:b;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn logical_values_keep_snapshots_and_rhs_effects_once() {
    for source in [
        "export function run(a,b){let n=0;let x='left';const old=x;const y=(n++,x)||(n+=10,x='right');return n===1&&y===old&&x===old?a:b;}",
        "export function run(a,b){let n=0;let x='';const old=x;const y=(n++,x)&&(n+=10,x='right');return n===1&&y===old&&x===old?a:b;}",
        "export function run(a,b){let n=0;let x='left';const y=(n++,x)&&(n+=10,x='right');return n===11&&y===x&&x==='right'?a:b;}",
        "export function run(a,b){let x={v:a};const old=x;const y=x&&(x={v:b});return y===x&&old!==y?a:b;}",
        "export function run(a,b){const x={v:''};const y=x.v||(x.v='changed','fallback');return y==='fallback'&&x.v==='changed'?a:b;}",
        "export function run(a,b){let n=0;const x={v:null};const y=(n++,x.v)??(n+=10,x.v='changed','fallback');return n===11&&y==='fallback'&&x.v==='changed'?a:b;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn managed_logical_results_flow_through_ordinary_helpers() {
    let js = "function choose(x,y){return x??y;}export function run(a,b){const ref={};const x=choose(a?'':null,ref);return a?(x===''?a:b):(x===ref?a:b);}";
    let ts = "function choose(x:number,y:number):number{return (x as number)??(y as number);}export function run(a:number,b:number):number{const ref={};const x=choose(a?'':null,ref);return a?(x===''?a:b):(x===ref?a:b);}";
    common::compare_sources(js, ts);
    for source in [
        "function choose(x,y){return x||y;}export function run(a,b){const ref={};const x=choose(a,ref);return (a?x===a:x===ref)?a:b;}",
        "function choose(x,y){return x&&y;}export function run(a,b){const ref={};const x=choose(a,ref);return (a?x===ref:(a!==a?x!==x:x===a))?a:b;}",
        "function choose(x,y){return x??y;}export function run(a,b){const ref={};let n=0;const x=choose.call(n++,null,ref,n++);return n===2&&x===ref?a:b;}",
        "function choose(x,y){return x??y;}export function run(a,b){const x=choose(!a,{});return x===!a?a:b;}",
        "function choose(x,y){return x??y;}export function run(a,b){const x=choose(a===b,{});return x===(a===b)?a:b;}",
        "function flag(x){return !x;}function choose(x,y){return x??y;}export function run(a,b){const x=choose(flag(a),{});return x===!a?a:b;}",
        "function id(x){return x;}export function run(a,b){const o={v:id(null)};const x=o.v??'fallback';return x==='fallback'?a:b;}",
        "function id(x){return x;}export function run(a,b){const o={v:id(!a)};const x=o.v??'fallback';return x===!a?a:b;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn discarded_managed_defaults_keep_only_selected_effects() {
    for source in [
        "export function run(a,b){let n=0;const x={v:null};x.v??(n++,'yes');return n;}",
        "export function run(a,b){let n=0;const x={v:''};x.v??(n++,'yes');return n;}",
        "export function run(a,b){let n=0;const x={v:a};x.v||(n++,'yes');return n;}",
        "export function run(a,b){let n=0;const x={v:a};x.v&&(n++,'yes');return n;}",
        "export function run(a,b){let n=0;({})??(n++,'yes');return n;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn unselected_managed_branches_still_require_supported_source() {
    for source in [
        "export function run(a,b){const x={}||missing();return a;}",
        "export function run(a,b){const x={}||{[a]:b};return a;}",
        "export function run(a,b){const x={}||(()=>a);return a;}",
        "export function run(a,b){let x='';x||='fallback';return a;}",
        "export function run(a,b){const x={}??{};return x;}",
        "export async function run(a,b){const x='v'||'fallback';await a;return a;}",
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
fn managed_logical_queries_and_lowering_have_a_small_stack_bound() {
    for operator in ["&&", "||", "??"] {
        for (depth, admitted) in [(16, true), (128, true), (129, false)] {
            let expression = format!("{}'v'", format!("'v'{operator}").repeat(depth));
            let source =
                format!("export function run(a,b){{const x={expression};return x==='v'?a:b;}}");
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
}

#[test]
fn managed_defaults_preserve_existing_primitive_export_profiles() {
    use grunkalunka_compiler::compile_value;
    use grunkalunka_runtime::{Value, ValueInstance};
    let source = "function flag(x){return !x;}function choose(x,y){return x??y;}export function run(a,b){const ref={};const x=choose(flag(a),ref);return x===!a;}";
    for ts in [false, true] {
        let artifact = compile_value(source, ts).unwrap();
        let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
        for a in [0.0, -0.0, 1.0, -7.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                instance.run(a, 2.0).unwrap(),
                Value::Boolean(true)
            ));
        }
    }
}

#[test]
fn array_logical_values_keep_identity_and_element_snapshots() {
    for source in [
        "export function run(a,b){const x=[];const y=x&&'yes';return y==='yes'?a:b;}",
        "export function run(a,b){const x=[];const y=x||[b];return y===x?a:b;}",
        "export function run(a,b){const x=[];const y=x??[b];return y===x?a:b;}",
        "export function run(a,b){const x=[null];const y=x[0]??[];return y!==x?a:b;}",
        "export function run(a,b){const x=[a];const y=x[0]||'fallback';return (a?y===a:y==='fallback')?a:b;}",
        "export function run(a,b){let n=0;let x=[];const old=x;const y=(n++,x)||(n+=10,x=[b]);return n===1&&y===old&&x===old?a:b;}",
        "function choose(x,y){return x??y;}export function run(a,b){const x=[];const y=choose(null,x);return y===x?a:b;}",
    ] {
        common::compare_sources(source, source);
    }
}
