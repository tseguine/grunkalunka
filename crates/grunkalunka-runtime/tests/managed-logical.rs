#[path = "managed/common.rs"]
mod common;

#[test]
fn boxed_number_loose_equality_keeps_number_semantics_and_snapshots() {
    for body in [
        "const o={n:a};return o.n==b?1:0;",
        "const xs=[a,b];return xs[0]!=xs[1]?1:0;",
        "const o={n:NaN};return o.n!=o.n&&!(o.n==o.n)?1:0;",
        "const o={n:-0};return o.n==0&&0==o.n&&!(o.n!=0)?1:0;",
        "const o={n:Infinity};const xs=[-Infinity];return o.n==Infinity&&xs[0]!=o.n?1:0;",
        "const o={n:a};const value=o.n==(o.n=b,a);return value==(a==a)?1:0;",
        "let [x]=[a];const same=x==(x=b,a);return same==(a==a)?1:0;",
        "const same=(x,y)=>x.n==y.n;const first={n:a};const second={n:b};return same(first,second)?1:0;",
        "let n=0;const o={n:a};const value=(n=1,o.n)==(n=n*10+2,a);return n===12&&value==(a==a)?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
    let js = "export function run(a,b){const o={n:a};return o.n!=b?1:0;}";
    let ts = "export function run(a:number,b:number){const o:string={n:a};return (o as boolean).n!=(b as string)?1:0;}";
    common::compare_sources(js, ts);
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm
    );
}

#[test]
fn reference_loose_equality_preserves_identity_and_saved_operands() {
    for body in [
        "const make=()=>({});const first=make();const second=make();return first!=second&&first==first?1:0;",
        "const o={};const alias=o;return o==alias&&o!={}?1:0;",
        "const xs=[];const alias=xs;return xs==alias&&xs!=[]?1:0;",
        "return {}!=[]&&[]!={}?1:0;",
        "const d=Object.create(null);const alias=d;return d==alias&&d!={}?1:0;",
        "const o={x:{}};return o.x==o.x&&o.x!={}?1:0;",
        "let o={};const old=o;const value=o==(o={},old);return value&&o!=old?1:0;",
        "const o={x:{}};const old=o.x;const value=o.x==(o.x={},old);return value&&o.x!=old?1:0;",
        "let n=0;const o={};const value=(n=1,o)==(n=n*10+2,o);return value&&n===12?1:0;",
        "const xs=[{},{}];let n=0;for(const value of xs){if(value==value)n++;if(value!=xs[0])n+=10;}return n;",
        "const o={};const same=(x,y)=>x==y;return same(o,o)&&!same(o,{})?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
    let js = "export function run(a,b){const xs=[];const alias=xs;return xs==alias?1:0;}";
    let ts = "export function run(a:number,b:number){const xs:number=[];const alias:boolean=xs;return (xs as string)==(alias as number)?1:0;}";
    common::compare_sources(js, ts);
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm
    );
}

#[test]
fn loose_null_checks_cover_managed_tags_and_keep_operand_snapshots() {
    for body in [
        "const o={x:null};return o.x==null?1:0;",
        "const o={};return undefined==o.x?1:0;",
        "const o={x:a?'text':null};return (o.x!=null)==!!a?1:0;",
        "let count=0;for(const value of [null,undefined,false,0,NaN,'','x',{},[],()=>a]){if(value==null)count++;if(undefined!=value)count+=10;}return count;",
        "const d=Object.create(null);const k='x';d[k]=a?null:'v';return (d[k]==null)==!!a?1:0;",
        "const o={x:null};let n=0;const value=(n=1,o.x)==void(n=n*10+2,o.x='changed');return value&&n===12&&o.x==='changed'?1:0;",
        "const o={x:null};let n=0;const value=void(n=1,o.x='changed')===(n=n*10+2,undefined);return value&&n===12?1:0;",
        "const o={x:null};let n=0;const value=void(n=1,o.x='changed')==(n=n*10+2,o.x);return !value&&n===12?1:0;",
        "const o={x:'text'};const sentinel=a?null:undefined;return sentinel!=o.x?1:0;",
        "const undefined=a;const o={x:null};return o.x==undefined?1:0;",
        "const left={};const right=[];return left==null||undefined==right?0:1;",
        "const o={x:null};return {flag:o.x==null}.flag===true?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
    let js = "export function run(a,b){const o={x:a?'text':null};return o.x!=null?1:0;}";
    let ts = "export function run(a:number,b:number){const o:number={x:a?'text':null};return (o as boolean).x!=(null as number)?1:0;}";
    common::compare_sources(js, ts);
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm
    );
}

#[test]
fn logical_values_return_the_selected_operand() {
    for source in [
        "export function run(a,b){let x='';x||='fallback';return a;}",
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

#[test]
fn nullish_defaults_compose_selected_strings_and_lazy_effects() {
    for source in [
        "export function run(a,b){const text=[' ready ',null,'',undefined].at(a)??' missing ';return text.trim().length;}",
        "export function run(a,b){const text='abc'.at(a)??'?';return text.trim().length;}",
        "function choose(x,y){return x??y;}export function run(a,b){const text=choose(a?null:' ready ',' missing ');return text.trim().length;}",
        "export function run(a,b){let n=0;const box={label:a?null:' '};const text=(n++,box.label)??(n+=10,box.label=' changed ',' fallback ');return text.trim().length*100+n;}",
        "export function run(a,b){let n=0;const text=''??(n++,a);return text.trim().length+n;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn nullish_defaults_keep_selected_container_aliases_and_helper_captures() {
    for source in [
        "export function run(a,b){const original={label:' old '};const row=[original].at(a)??{label:' fallback '};original.label=' changed ';return row.label.trim().length;}",
        "function choose(x,y){return x??y;}export function run(a,b){const source=a?null:{label:' ready '};const read=()=>choose(source,{label:' fallback '});return read().label.trim().length;}",
        "export function run(a,b){const fallback=[b];const row=[[a,b],null].at(a)??fallback;return row===fallback?row.length:row.length+10;}",
        "export function run(a,b){let n=0;const row={label:' keep '}??(n++,a);return row.label.trim().length+n;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn logical_defaults_compose_nullable_strings_and_short_circuit_effects() {
    for source in [
        "export function run(a,b){const text=[' ready ',null,'',undefined].at(a)||' missing ';return text.trim().length;}",
        "function choose(x,y){return x||y;}export function run(a,b){const text=choose(a?null:' ready ',' missing ');return text.trim().length;}",
        "export function run(a,b){let n=0;const text=(n++,a?null:'')||(n+=10,' fallback ');return text.trim().length*100+n;}",
        "export function run(a,b){let n=0;const text=((a?{}:null)&&(n++,' kept '))??' fallback ';return text.trim().length*10+n;}",
        "export function run(a,b){let n=0;const text=(null&&(n++,a))??' fallback ';return text.trim().length+n;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn logical_selection_retains_reference_identity_and_compatible_receivers() {
    for source in [
        "export function run(a,b){let n=0;const original={label:' old '};const row=original||(n++,{label:' ignored '});original.label=' changed ';return row.label.trim().length+n;}",
        "export function run(a,b){let n=0;const original={label:' left '};const row=(n++,original)&&(n+=10,{label:' right '});return row.label.trim().length*10+n;}",
        "export function run(a,b){const source={label:' first '};const row=[]&&source;const read=()=>row.label.trim().length;source.label=' changed ';return read();}",
        "export function run(a,b){const row=Object.create(null);row.constructor=' own ';const selected=row||a;return selected.constructor===' own '?1:0;}",
        "export function run(a,b){const values=[a,b]||a;return values.length;}",
        "function marker(x){return x;}export function run(a,b){const row=marker&&{label:' present '};return row.label.trim().length;}",
    ] {
        common::compare_sources(source, source);
    }
}
