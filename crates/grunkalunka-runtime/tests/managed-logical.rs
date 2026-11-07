#[path = "managed/common.rs"]
mod common;

#[test]
fn final_switch_arms_resume_suffix_without_testing_another_case() {
    for body in [
        "switch(a){case 0:a=b;}return a;",
        "switch(a){case 0:return b;default:a=b;}return a;",
        "switch(a){default:return b;case 0:a=b;}return a;",
        "let n=0;switch(a){case(n++,0):return n;case(n=n*10+2,1):n+=100;}return n;",
        "switch(a){case 0:;}return a;",
        "switch(a){default:;}return b;",
        "switch(a){case 0:if(b)return b;}return a;",
        "const x=a;switch(b){case 0:{const x=b;}}return x;",
        "let n=0;for(let i=0;i<2;i++){switch(a){case 0:return b;default:n++;}}return n;",
        "let n=0;try{switch(a){case 0:return b;default:n++;}}finally{n+=10;}return n;",
        "try{switch(a){case 0:if(b)throw b;}return a;}catch(e){return e+1;}",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn nonexhaustive_switch_runs_suffix_only_after_every_case_misses() {
    for body in [
        "switch(a){case 0:return b;}return a;",
        "let n=0;switch((n++,a)){}return n+a;",
        "let n=0;switch(a){case(n++,0):return n;case(n=n*10+2,1):return n;}return n+100;",
        "switch(a){case(a=b,b):return 10;}return a;",
        "switch(a){case NaN:return 99;case -0:return b;}return a;",
        "switch(a?'a':'b'){case 'a':return a;}return b;",
        "const f=x=>{switch(x){case 0:return b;}return x;};return f(a);",
        "let n=0;for(let i=0;i<2;i++){switch(a){case 0:return b;}n++;}return n;",
        "try{switch(a){case 0:throw b;}return a;}catch(e){return e+1;}",
        "let n=0;try{switch(a){case 0:return b;}}finally{n++;}return n;",
        "switch(a){case 0:switch(b){case 0:return 1;default:return 2;}}return 3;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
    let js = "export function run(a,b){switch(a){case 0:return b;}return a;}";
    let ts = "export function run(a:number,b:number){switch(a as boolean){case (0 as string):return b;}return a;}";
    common::compare_sources(js, ts);
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm
    );
}

#[test]
fn returning_switch_executes_the_exact_original_gap_and_goldens() {
    let source = "export function run(a,b) { switch(a) { case 4: return b; default: return a; } }";
    common::compare_sources_with_fuel_and_goldens(source, source, None, &[[4, 2, 2], [3, 2, 3]]);
}

#[test]
fn returning_switch_dispatch_preserves_strict_kinds_source_order_and_defaults() {
    for body in [
        "switch(a){case 4:return b;default:return a;}",
        "switch(a){case NaN:return 99;case -0:return b;default:return a;}",
        "switch(a?true:null){case 1:return 99;case true:return a;case null:return b;default:return 98;}",
        "switch(a?'x':'y'){case 'x':return a;case ('y'+''):return b;default:return 99;}",
        "let n=0;switch(a){case (n++,0):return n;default:return n;case (n=n*10+2,1):return n;case (n=99,2):return n;}",
        "switch(a){case (a=b,b):return a+10;case b:return 99;default:return a;}",
        "let n=0;switch((n++,a)){case a:return n;case (n=99,b):return n;default:return n;}",
        "switch(a){default:{let x=b;return x;}case 0:{let x=a;return x;}}",
        "switch(a){case 0:switch(b){case 0:return 1;default:return 2;}default:return 3;}",
        "const f=x=>{switch(x){case 0:return b;default:return x;}};return f(a);",
        "try{switch(a){case 0:throw b;default:return a;}}catch(e){return e+1;}",
        "let n=0;try{switch(a){case 0:return b;default:return a;}}finally{n++;}",
        "switch(a){default:return b;}",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
    let js = "export function run(a,b){switch(a){case 4:return b;default:return a;}}";
    let ts = "export function run(a:number,b:number){switch(a as boolean){case (4 as string):return b;default:return a;}}";
    common::compare_sources(js, ts);
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm
    );
}

#[test]
fn boxed_primitive_alternatives_use_scalar_coercion_and_saved_values() {
    for body in [
        "const o={v:a?true:b};return +o.v;",
        "const o={v:a?null:b};return o.v/b;",
        "const o={v:a?undefined:false};return ~o.v;",
        "const o={v:a?true:b};return (o.v<<1)|~o.v;",
        "const o={v:a?null:b};return o.v<(o.v=false,b)?1:0;",
        "const o={v:a?true:b};const value=o.v==(o.v=null,b);return (value?1:0)+ +o.v;",
        "const o={v:a?true:b};return o.v===true?1:0;",
        "const {v}={v:a?undefined:true};return +v;",
        "let n=0;for(const v of [true,null,undefined,b])n+=~v;return n;",
        "const read=x=>x.v;const o={v:a?null:b};return +read(o);",
        "const o={v:a?false:b};const read=()=>o.v;return +read();",
        // Exact former compiler guards now execute with native source oracles.
        "const o={flag:a?true:b};return +o.flag;",
        "const o={flag:true};delete o.flag;return +o.flag;",
        "const o={flag:true||a};return o.flag==true?1:0;",
        "const o={flag:false&&a};return o.flag==false?1:0;",
        "const o={flag:true};return o.flag==1?1:0;",
        "const o={flag:a?true:b};return o.flag==true?1:0;",
        "const o={flag:true};delete o.flag;return o.flag==true?1:0;",
        "const o={n:a?b:false};return o.n==b?1:0;",
        "const o={n:a};delete o.n;return o.n==b?1:0;",
        "const o={x:a};delete o.x;return o.x+b;",
        "const o={x:a};const change=t=>{t.x=null;return 0;};change(o);return o.x+b;",
        "const o={x:a};const key='x';delete o[key];return o.x+b;",
        "const xs=[a,,b];return xs[1]+b;",
        "const o={};return o.x+b;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
    let js = "export function run(a,b){const o={flag:true};return o.flag==a?1:0;}";
    let ts = "export function run(a:number,b:number){const o={flag:true};return o.flag==(a as unknown as boolean)?1:0;}";
    common::compare_sources(js, ts);
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm
    );
    let js = "export function run(a,b){const o={x:a};delete o.x;return o.x+b;}";
    let ts = "export function run(a:number,b:number){const o={x:a};delete o.x;return (o.x as number)+b;}";
    common::compare_sources(js, ts);
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm
    );
}

#[test]
fn boolean_logical_cells_keep_selection_snapshots_and_captures() {
    for body in [
        "let [flag]=[true];const value=flag&&=false;return value==false&&flag==false?1:0;",
        "let {flag}={flag:false};return +(flag||=true);",
        "let [flag]=[false];let n=0;const value=flag??=(n++,true);return value==false&&flag==false&&n===0?1:0;",
        "let [flag]=[a<b];let n=0;const old=flag;const value=flag&&=(n++,false);return value==false&&flag==false&&n==+old?1:0;",
        "let [flag]=[false];const read=()=>flag;flag||=true;return read()==true?1:0;",
        "let [flag]=[false];const set=()=>flag||=true;const value=set();return value==true&&flag==true?1:0;",
        "let [flag]=[true];const value=flag&&=(flag=false,true);return value==true&&flag==true?1:0;",
        "let count=0;for(let flag of [true,false]){flag||=true;count+=+flag;}return count;",
        "let [flag]=[true];let count=0;while(flag){count++;flag&&=false;}return count+ +flag;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
    let js = "export function run(a,b){let [flag]=[a<b];flag||=true;return +flag;}";
    let ts = "export function run(a:number,b:number){let [flag]:number=[a<b];flag||=(true as number);return +(flag as string);}";
    common::compare_sources(js, ts);
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm
    );
}

#[test]
fn boolean_fields_project_as_booleans_before_scalar_coercion() {
    for body in [
        "const o={flag:a<b};return +o.flag;",
        "const xs=[true,false];return +xs[0]*10+ +xs[1];",
        "const o={flag:false};return 1/-o.flag;",
        "const o={flag:true};const value=(+o.flag)+(o.flag=false,+o.flag);return value;",
        "const o={flag:a<b};const take=value=>+value;return take(o.flag);",
        "const o={flag:true};return (o.flag?1:0)+ +(o.flag==true);",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
    let js = "export function run(a,b){const o={flag:a<b};return +o.flag;}";
    let ts = "export function run(a:number,b:number){const o:string={flag:a<b};return +((o as boolean).flag as number);}";
    common::compare_sources(js, ts);
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm
    );
}

#[test]
fn boxed_boolean_loose_equality_keeps_tags_and_saved_values() {
    for body in [
        "const o={flag:false??a};return o.flag==false?1:0;",
        "const o={flag:true};return o.flag==true&&o.flag!=false?1:0;",
        "const xs=[a<b,false];return xs[0]!=xs[1]?1:0;",
        "const o={flag:!a};return o.flag==!!b?1:0;",
        "const o={flag:a==b};return o.flag==(a==b)?1:0;",
        "const o={x:a};const flags={removed:delete o.x,present:'x' in o};return flags.removed==true&&flags.present==false?1:0;",
        "const o={flag:true};const value=o.flag==(o.flag=false,true);return value&&o.flag==false?1:0;",
        "let [flag]=[true];const same=flag==(flag=false,true);return same&&flag==false?1:0;",
        "let n=0;for(const flag of [true,false,true]){if(flag==true)n++;}return n;",
        "const same=(x,y)=>x.flag==y.flag;return same({flag:a<b},{flag:a<b})?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
    let js = "export function run(a,b){const o={flag:a<b};return o.flag==true?1:0;}";
    let ts = "export function run(a:number,b:number){const o:number={flag:a<b};return (o as string).flag==(true as number)?1:0;}";
    common::compare_sources(js, ts);
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm
    );
}

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

#[test]
fn switch_references_preserve_aliases_and_fresh_function_identity() {
    for body in [
        "const x={};switch(x){case {}:return 99;case x:return a;default:return b;}",
        "const x=[];switch(x){case []:return 99;case x:return a;default:return b;}",
        "const x=Object.create(null);const y=x;switch(x){case y:return a;default:return b;}",
        "let x={};const old=x;switch(x){case (x={},x):return 99;case old:return a;default:return b;}",
        "const x={};let n=0;switch(x){case(n++,x):return n;case(n=99,{}):return n;}return n;",
        "const f=()=>a;const g=f;switch(f){case g:return a;default:return b;}",
        "const f=()=>a;const g=()=>a;switch(f){case g:return 99;case f:return a;default:return b;}",
        "const make=()=>()=>a;const f=make();const g=make();switch(f){case g:return 99;case f:return a;default:return b;}",
        "const f=async()=>a;const g=f;switch(f){case g:return a;default:return b;}",
        "const make=()=>async()=>a;const f=make();const g=make();switch(f){case g:return 99;case f:return a;default:return b;}",
        "const x={};switch(a){case x:return 99;default:return b;}",
        "switch({}){case null:return b;default:return a;}",
        "switch(a){case {}:return b;default:return a;}",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn mixed_switch_values_preserve_tags_identity_and_snapshots() {
    for body in [
        "switch(a?'x':null){case null:return b;case 'x':return a;default:return 99;}",
        "switch(a?'1':1){case 1:return b;case '1':return a;default:return 99;}",
        "const x={};switch(a?x:false){case false:return b;case {}:return 99;case x:return a;}return 98;",
        "const x={};switch(x){case a?x:null:return a;}return b;",
        "const f=()=>a;switch(a?f:null){case null:return b;case f:return a;}return 99;",
        "const make=()=>()=>a;const f=make();const g=make();switch(a?f:0){case a?g:0:return b;case f:return a;}return 99;",
        "const xs=[{},'x',null,a];switch(xs[0]){case xs[1]:return 99;case xs[0]:return a;}return b;",
        "const x=a?undefined:[];switch(x){case undefined:return a;default:return b;}",
        "let x=a?{}:'x';const old=x;switch(x){case(x=null,x):return 99;case old:return a;}return b;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn switch_breaks_exit_selection_and_preserve_enclosing_control() {
    for body in [
        "let n=0;switch(a){case 0:n=b;break;default:n=a;}return n;",
        "let n=0;switch(a){default:n=3;break;case 0:n=4;break;case 1:n=5;}return n;",
        "let n=0;switch(a){case 0:if(b)break;else return a;default:n=b;}return n+1;",
        "let n=0;for(let i=0;i<3;i++){switch(i){case 0:n++;break;default:n+=10;}n+=100;}return n;",
        "let n=0;for(let i=0;i<3;i++){switch(i){case 0:n++;break;default:n+=10;}if(a)continue;n+=100;}return n;",
        "let n=0;while(n<3){switch(n){case 0:n++;break;default:n+=2;break;}if(a)break;}return n;",
        "let n=0;switch(a){case 0:while(n<3){n++;if(n===2)break;}n+=10;break;default:n=b;}return n;",
        "let n=0;switch(a){case 0:switch(b){case 0:n=1;break;default:n=2;}n+=10;break;default:n=3;}return n;",
        "let n=0;switch(a){case(n++,0):n+=10;break;case(n+=100,1):n+=1000;break;}return n;",
        "let n=0;switch(a){case 0:try{n=1;break;}finally{n+=10;}default:n=2;}return n;",
        "let n=0;switch(a){case 0:try{throw b;}catch(e){n=+e;break;}default:n=2;}return n;",
        "let n=0;switch(a){case 0:try{break;}finally{return b;}default:n=2;}return n;",
        "const f=x=>{switch(x){case 0:break;default:return x;}return b;};return f(a);",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn switch_break_executes_the_exact_former_rejection_source() {
    let source = "export function run(a,b) { switch(a) { default: break; } return b; }";
    common::compare_sources(source, source);
}

#[test]
fn switch_fallthrough_keeps_selection_order_and_abrupt_completions() {
    for body in [
        "let n=0;switch(a){case 0:n+=1;case 1:n+=10;default:n+=100;}return n;",
        "let n=0;switch(a){case 0:case 1:n=b;break;default:n=a;}return n;",
        "let n=0;switch(a){default:n+=100;case 0:n+=1;case 1:n+=10;}return n;",
        "let n=0;switch(a){case 0:n+=1;default:n+=100;case 1:n+=10;}return n;",
        "let n=0;switch(a){case(n=1,0):n+=10;default:n+=100;case(n=n*10+2,1):n+=1000;}return n;",
        "let n=0;switch(a){case(n++,0):n++;case(n=99,1):n+=10;}return n;",
        "let n=0;switch(a){case 0:if(b)break;n++;case 1:n+=10;}return n;",
        "let n=0;switch(a){case 0:if(b)return b;n++;default:n+=10;}return n;",
        "switch(a){case 0:;default:return b;}",
        "const f=x=>{switch(x){case 0:;default:return b;}};return f(a);",
        "let n=0;for(let i=0;i<3;i++){switch(i){case 0:n++;case 1:n+=10;break;default:n+=100;}n+=1000;}return n;",
        "let n=0;switch(a){case 0:try{n++;}finally{n+=10;}case 1:n+=100;break;default:n=2;}return n;",
        "let n=0;switch(a){case 0:switch(b){case 0:n++;default:n+=10;}case 1:n+=100;}return n;",
        "let n=0;switch(a){default:case 0:n++;case 1:;}return n;",
        "const x={};let n=0;switch(a?x:null){case x:n++;case null:n+=10;}return n;",
        "switch(a){case 0:{const x=b;return x;}default:{const x=a;return x;}}",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn optional_ts_parameters_preserve_javascript_call_behavior() {
    for (js, ts) in [
        (
            "export function run(a,b){return a;}",
            "export function run(a?:unknown,b?:unknown):unknown{return a;}",
        ),
        (
            "function add(x,y){return x+y;}export function run(a,b){return add(a,b);}",
            "function add(x?:string,y?:unknown):unknown{return x+y;}export function run(a?:number,b?:number){return add(a,b);}",
        ),
        (
            "export function run(a,b){const f=(x,y)=>x+y;return f(a,b);}",
            "export function run(a?:number,b?:number){const f=(x?:string,y?:unknown):unknown=>x+y;return f(a,b);}",
        ),
        (
            "export function run(a,b){let n=a;const f=(x)=>{n+=x;return n;};return f(b)+f(a);}",
            "export function run(a?:number,b?:number){let n=a;const f=(x?:number)=>{n+=x;return n;};return f(b)+f(a);}",
        ),
        (
            "export function run(a,b){const f=(x,y)=>x+y;return f.length+f(a,b);}",
            "export function run(a?:number,b?:number){const f=(x?:unknown,y?:unknown)=>x+y;return f.length+f(a,b);}",
        ),
        (
            "export function run(a,b){const o={f:(x)=>x+a};return o.f(b);}",
            "export function run(a?:number,b?:number){const o={f:(x?:unknown)=>x+a};return o.f(b);}",
        ),
        (
            "export function run(a,b){const apply=(f,x)=>f(x);const id=(x)=>x+a;return apply(id,b);}",
            "export function run(a?:number,b?:number){const apply=(f?:unknown,x?:unknown)=>f(x);const id=(x?:unknown)=>x+a;return apply(id,b);}",
        ),
    ] {
        common::compare_sources(js, ts);
    }
}

#[test]
fn optional_parameters_execute_exact_former_typed_rejection_sources() {
    common::compare_sources(
        "export function run(a,b) { return a; }",
        "export function run(a?:number,b?:number):number { return a; }",
    );
    common::compare_sources(
        "export function run(a,b) { return a; }",
        "export function run(a?:unknown,b?:unknown):unknown { return a; }",
    );
    common::compare_sources(
        "function helper(x) { return x; } export function run(a,b) { return helper(a); }",
        "function helper(x?:number) { return x; } export function run(a,b) { return helper(a); }",
    );
    common::compare_sources(
        "export function run(a,b) { return a; }",
        "export function run<T>(a?:T,b?:T):T { return a; }",
    );
}

#[test]
fn omitted_named_helper_arguments_preserve_undefined_and_effects() {
    for source in [
        "function flag(x){return x===undefined?1:0;}export function run(a,b){return flag();}",
        "function flag(x){return typeof x==='undefined'?1:0;}export function run(a,b){return flag();}",
        "function add(x,y){return x+ +y;}export function run(a,b){return add(a);}",
        "function choose(x,y){return y??x;}export function run(a,b){return choose(a);}",
        "function flag(x){return x===undefined?1:0;}export function run(a,b){return flag()*100+flag(a)*10+flag(NaN);}",
        "function f(x){x=3;return x;}export function run(a,b){return +f();}",
        "function f(x){if(x===undefined)return 7;return f();}export function run(a,b){return f(a);}",
        "function f(x,y){return y===undefined?x:0;}export function run(a,b){let n=0;const x=f((n++,a));return x+n;}",
        "function flag(x){return x===undefined?1:0;}export function run(a,b){let n=0;const value=flag.call((n++,a));return value*10+n;}",
        "function flag(x){return x===undefined?1:0;}export function run(a,b){return flag.call();}",
        "function f(x){const g=()=>x===undefined?1:0;return g();}export function run(a,b){return f();}",
        "function choose(x,y){return y??x;}export function run(a,b){const text=choose('x');return text==='x'?a:b;}",
        "function choose(x,y){return y??x;}export function run(a,b){const value=choose({v:a});return +value.v;}",
        "function flag(x){return x===undefined?1:0;}export function run(a,b){const alias=flag;return flag()*100+alias(a);}",
        "function flag(x,y){return typeof y==='undefined'?1:0;}export function run(a,b){return flag.length*100+flag(a);}",
    ] {
        common::compare_sources(source, source);
    }
}
