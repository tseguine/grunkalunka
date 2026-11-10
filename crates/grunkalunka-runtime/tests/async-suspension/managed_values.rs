use super::*;

#[test]
fn unsigned_has_own_keys_preserve_suspended_expression_order_and_errors() {
    compare_promises(
        &[
            "const xs=[a];return Object.hasOwn(xs,(await b,0)>>>0)?1:0;",
            "const xs=[a];return Object.hasOwn(xs,(await b,1)>>>0)?1:0;",
            "const xs=[a];return Object.hasOwn(xs,(await b,xs.pop(),0)>>>0)?1:0;",
            "return Object.hasOwn('💠',(await a,1)>>>0)?1:0;",
            "const d=Object.create(null);d[0]=b;return Object.hasOwn(d,(await a,0)>>>0)?1:0;",
            "const xs=[a];return Object.hasOwn(xs,(await a,0)>>>0,xs.pop())?xs.length:0;",
            "let n=0;try{Object.hasOwn([a],(await Promise.reject(b),0)>>>0,n++);}catch(e){await a;return n;}return 99;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn final_switch_arms_keep_selected_awaits_rejection_and_suffix_order() {
    compare_promises(
        &[
            "let n=0;for(let i=0;i<3;i++){switch(i){case 1:try{continue;}finally{n+=await 1;}default:break;}n+=10;}return n;",
            "switch(a){case 0:a=await b;}return await a;",
            "switch(a){case 0:return await b;default:a=await b;}return a;",
            "switch(a){default:return b;case 0:a=await b;}return a;",
            "let n=0;switch(a){case(await b,0):n++;}return n+await b;",
            "try{switch(a){case 0:await Promise.reject(b);}return a;}catch(e){return e;}",
            "switch(a){case 0:if(b)return await b;}return await a;",
            "const o={x:a};switch(a){case 0:await b;o.x=b;}return +o.x;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn nonexhaustive_switch_preserves_await_order_rejection_and_suffix() {
    compare_promises(
        &[
            "switch(await a){case await b:return a;}return await b;",
            "switch(await 'x'){case(await b,'y'):return a;}return b;",
            "switch(a){case(a=b,await b):return a;}return await a;",
            "try{switch(a){case(await Promise.reject(b),b):return 99;}return a;}catch(e){return e;}",
            "switch(a){case a:return await b;case(await Promise.reject(b),b):return 99;}return a;",
            "let n=0;switch((await b,n++,a)){}return n;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn returning_switch_dispatch_keeps_awaited_tests_and_saved_discriminants() {
    compare_promises(
        &[
            "switch(await a){case await b:return a;default:return b;}",
            "switch(await 'x'){case await 'y':return a;case await 'x':return b;default:return 99;}",
            "switch(a){case (a=b,await b):return a;default:return a+b;}",
            "switch(a){case a:return await b;case (await Promise.reject(b),b):return 99;default:return a;}",
            "let n=0;switch(a){case (n++,await 0):return n;default:return n;case (n++,await 1):return n;}",
            "try{switch(a){case (await Promise.reject(b),b):return 99;default:return a;}}catch(e){return e;}",
            "switch(a?null:undefined){case await null:return a;case await undefined:return b;default:return 99;}",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn dictionary_primitive_origins_keep_awaited_keys_receivers_and_results() {
    compare_promises(
        &[
            "const d=Object.create(null);d.x=a;return +(await d[await 'x']);",
            "const d=Object.create(null);d.x=a;const k='x';const value=d[k]+(d.x=b,await b);return value;",
            "let d=Object.create(null);d.x=a;const other=Object.create(null);other.x=b;const value=d[(d=other,await b,'x')];return +value+ +d['x'];",
            "const d=Object.create(null);d.x=false;const value=d[(d.x=true,await b,'x')];return value==1?1:0;",
            "const d=Object.create(null);const k=await 'missing';return +(await d[k]);",
            "const d=Object.create(null);const k='x';const value=d[k]??=await b;return +value+ +d[k];",
            "const d=Object.create(null);d.x=false;const k='x';const value=d[k]&&=(await Promise.reject(b),b);return +value;",
            "const d=Object.create(null);d.x=a;let n=0;try{const value=d[(n++,await Promise.reject(b),'x')];return +value;}catch(e){return n===1&&e===b?1:0;}",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn boxed_primitive_unions_preserve_await_tags_snapshots_and_rejections() {
    compare_promises(
        &[
            "const o={v:await (a?null:b)};return +o.v;",
            "const o={v:a?null:undefined};return +(await o.v);",
            "const o={v:await null};return o.v===null?1:0;",
            "const o={v:await undefined};return o.v===undefined?1:0;",
            "const o={flag:await (a?true:b)};return o.flag==true?1:0;",
            "const o={v:a?true:b};return o.v==(o.v=false,await b)?1:0;",
            "const o={v:a?null:b};const value=o.v<(o.v=false,await b);return (value?10:0)+ +o.v;",
            "const o={v:false};const value=o.v==(o.v=true,await b,0);return value&&o.v===true?1:0;",
            "let n=0;const o={v:null};try{const value=(n++,o.v)==(await Promise.reject(b));return value?9:8;}catch(e){return e===b&&n===1?1:0;}",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn string_relational_values_keep_snapshots_rejections_and_await_order() {
    compare_promises(
        &[
            "return (await 'a')<(await 'b')?1:0;",
            "return (await 'a')<=(await 'a')?1:0;",
            "return (await 'b')>(await 'a')?1:0;",
            "return (await 'a')>=(await 'a')?1:0;",
            "let text='a';const value=text<(await b,text='z','b');return value&&text==='z'?1:0;",
            "const o={s:'b'};const read=()=>o.s;const value=read()>(o.s='',await b,'a');return value&&o.s===''?1:0;",
            "let n=0;try{const value=(n++,'a')<(await Promise.reject(b),'b');return value?9:8;}catch(e){return e===b&&n===1?1:0;}",
            "const flags=[(await(await 'a'))<'b'];await b;return flags[0]==true?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn boolean_logical_cells_keep_origins_through_selected_awaits() {
    compare_promises(
        &[
            "let [flag]=[false];const value=flag||=await true;return value==true&&flag==true?1:0;",
            "let [flag]=[true];flag&&=(await b,false);return +flag;",
            "let [flag]=[false];flag&&=(await Promise.reject(b),true);return flag==false?1:0;",
            "let [flag]=[false];flag??=(await Promise.reject(b),true);return +flag;",
            "let [flag]=[false];try{flag||=(await Promise.reject(b),true);}catch(e){return flag==false&&e===b?1:0;}return -1;",
            "let [flag]=[true];const read=()=>flag;const value=flag&&=(flag=false,await b,true);return read()==value&&value==true?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn awaited_boolean_origins_compose_with_storage_and_scalar_consumers() {
    compare_promises(
        &[
            "const o={flag:await true};return o.flag==true?1:0;",
            "const xs=[await false,await true];return +xs[0]*10+ +xs[1];",
            "const o={flag:await(await(a<b))};return +o.flag;",
            "const o={flag:a<b};return +(await o.flag);",
            "const o={flag:true};const value=(await o.flag)==(o.flag=false,true);return value&&o.flag==false?1:0;",
            "const o={flag:true};try{o.flag=await (await Promise.reject(b),false);}catch(e){return o.flag==true&&e===b?1:0;}return -1;",
            "const o={flag:a<b};if(await o.flag)return a;return b;",
            "const o={flag:true};await o.flag;return +o.flag;",
            "return await true;",
            "return await false;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn boolean_loose_equality_saves_operands_through_rhs_awaits() {
    compare_promises(
        &[
            "const o={flag:true};const same=o.flag==(await b,o.flag=false,true);return same&&o.flag==false?1:0;",
            "const xs=[a<b];return xs[0]!=(await a,false)?1:0;",
            "const o={flag:true};try{const same=o.flag==(await Promise.reject(b),true);return +same;}catch(e){return e;}",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn function_loose_equality_keeps_captured_identity_through_awaits() {
    compare_promises(
        &[
            "let f=()=>a;const old=f;const same=f==(await b,f=()=>b,old);return same&&f!=old?1:0;",
            "const make=()=>()=>a;const f=make();await b;return f!=make()&&f==f?1:0;",
            "const f=()=>a;const o={f};return o.f==(await b,f)?1:0;",
            "const f=()=>a;try{const same=f==(await Promise.reject(b),f);return +same;}catch(e){return e;}",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn boxed_number_loose_equality_keeps_snapshots_through_rhs_awaits() {
    compare_promises(
        &[
            "const o={n:a};const value=o.n==(await b,o.n=b,a);return value==(a==a)?1:0;",
            "const xs=[a,b];return xs[0]!=(await a,xs[0]=b,b)?1:0;",
            "const o={n:await a};return o.n==(await b,a)?1:0;",
            "const o={n:a};try{const value=o.n==(await Promise.reject(b),a);return +value;}catch(e){return e;}",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn reference_loose_equality_keeps_identity_across_rhs_suspension() {
    compare_promises(
        &[
            "let o={};const old=o;const value=o==(await a,o={},old);return value&&o!=old?1:0;",
            "const xs=[{}];const old=xs[0];const value=xs[0]==(await b,xs[0]={},old);return value&&xs[0]!=old?1:0;",
            "const o={};try{const value=o==(await Promise.reject(b),{});return +value;}catch(e){return e;}",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn string_loose_equality_saves_operands_through_await_and_rejection() {
    compare_promises(
        &[
            "const o={s:'old'};const value=o.s==(await a,o.s='new','old');return value&&o.s==='new'?1:0;",
            "const left=await 'x';return left!=(await b,'y')?1:0;",
            "const o={s:'old'};try{const value=o.s==(await Promise.reject(b),'old');return +value;}catch(e){return e;}",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn loose_null_checks_keep_left_snapshots_and_rhs_reactions() {
    compare_promises(
        &[
            "const o={x:a?null:'text'};return o.x==(await b,null)?(a?1:0):(a?0:1);",
            "const o={x:null};const value=o.x==(await a,o.x='changed',null);return value&&o.x==='changed'?1:0;",
            "const o={x:null};const value=(await a,void(o.x='changed'))!=o.x;return value?1:0;",
            "const o={x:'text'};try{const value=o.x==(await Promise.reject(b),null);return +value;}catch(e){return e;}",
            "const d=Object.create(null);const key=await 'x';d[key]=null;return d[key]!=null?0:1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn awaited_strings_keep_contents_snapshots_and_native_microtask_order() {
    compare_promises(
        &[
            "const text=await 'abc';return text.length;",
            "return (await (await 'abc')).length;",
            "let text='x';const value=text+=await 'y';return value==='xy'&&text===value?1:0;",
            "const o={s:'x'};const value=o.s+=await 'y';return value==='xy'&&o.s===value?1:0;",
            "let text='old';const value=await (text='new',text);text='later';return value==='new'&&text==='later'?1:0;",
            "let n=0;const text=await (n++,'abc');return text.length+n;",
            "if(await '')return -1;if(await 'x')return a;return b;",
            "const o={s:await 'abc'};const {s}=o;await 'unused';return s.length;",
            "try{const text=await (await Promise.reject(b),'abc');return text.length;}catch(e){return e;}",
            "let text='x';try{text+=await (await Promise.reject(b),'y');}catch(e){return text==='x'&&e===b?1:0;}return -1;",
            r"const text=await '\ud800\udfff\0';return text==='\ud800\udfff\0'?text.length:0;",
            "const text=await (a?'first':'second');return text.length;",
            "const get=()=> 'abc';return (await get()).length;",
            "await 'one';await 'two';await 'three';return a;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
    let js =
        "export async function run(a,b){const o={s:await 'x'};o.s+=await 'y';return o.s.length;}";
    let ts = "export async function run(a:number,b:number){const o:number={s:await ('x' as boolean)};(o as boolean).s+=await ('y' as never);return o.s.length;}";
    assert_eq!(
        compile_async(js, false).unwrap().wasm,
        compile_async(ts, true).unwrap().wasm
    );
}

#[test]
fn string_property_compounds_keep_references_across_await_and_rejection() {
    compare_promises(
        &[
            "let o={s:'old'};const alias=o;const value=o.s+=(await b,o={s:'other'},'new');return value==='oldnew'&&alias.s===value&&o.s==='other'?1:0;",
            "const xs=['old'];const value=xs[0]+=(await a,xs[0]='changed','new');return value==='oldnew'&&xs[0]===value?1:0;",
            "const o={s:'x'};const value=(await a,o).s+=(await b,'y');return value==='xy'&&o.s===value?1:0;",
            "const o={s:'old'};try{o.s+=(await Promise.reject(b),'new');}catch(e){return o.s==='old'&&e===b?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn string_local_compounds_keep_snapshots_and_commit_after_await_success() {
    compare_promises(
        &[
            "let text='old';const value=text+=(await b,text='changed','new');return value==='oldnew'&&text===value?1:0;",
            "let [text]=['x'];text+=(await a,'y');return text==='xy'?1:0;",
            "let text='old';try{text+=(await Promise.reject(b),'new');}catch(e){return text==='old'&&e===b?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn runtime_dictionary_logical_writes_keep_snapshots_across_awaits() {
    compare_promises(
        &[
            "let o={x:3,y:5};const old=o,k=a?'x':'y';const v=o[k]+=(o={x:7},await b);return old[k]===v&&o.x===7?1:0;",
            "let o={x:0};const old=o,k=a?'x':'y';const v=o[k]||=(o={},await b);return old[k]===v&&o[k]===undefined?1:0;",
            "const o={x:0};const k=a?'x':'y';try{o[k]??=await Promise.reject(b);}catch(e){return o[k]===undefined&&e===b?1:0;}return o[k]===0?1:0;",
            "const d=Object.create(null);const k='x';const value=d[k]??=(await b,'new');return d.x===value?1:0;",
            "const d=Object.create(null);const k='x';d[k]='old';d[k]||=(await Promise.reject(b),'new');return d.x==='old'?1:0;",
            "const d=Object.create(null);const k='x';try{d[k]??=(await Promise.reject(b),'new');}catch(e){return d.x===undefined&&e===b?1:0;}return -1;",
            "let d=Object.create(null);const old=d;let k='x';const value=d[(await b,d=Object.create(null),k)]??=(await a,k='y','new');return old.x===value&&old.y===undefined&&d.x===undefined?1:0;",
            "const d=Object.create(null);let n=0;try{d[(await Promise.reject(b),'x')]??=(n++,a);}catch(e){return n===0&&d.x===undefined&&e===b?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn mixed_property_logical_assignments_preserve_selection_across_awaits() {
    compare_promises(
        &[
            "const o={x:null};const value=o.x??=(await b,'new');return value==='new'&&o.x===value?1:0;",
            "const o={x:'old'};o.x||=(await Promise.reject(b),'new');return o.x==='old'?1:0;",
            "const xs=[false];xs[0]&&=(await Promise.reject(b),'new');return xs[0]===false?1:0;",
            "const o={x:null};try{o.x??=(await Promise.reject(b),'new');}catch(e){return o.x===null&&e===b?1:0;}return -1;",
            "let current={x:null};const old=current;const value=current.x??=(await b,current={x:false},'new');return old.x===value&&current.x===false?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn mixed_boxed_local_logical_assignments_select_suspension_and_store() {
    compare_promises(
        &[
            "let [x]=[null];const value=x??=(await b,'new');return value==='new'&&x===value?1:0;",
            "let [x]=['old'];x||=(await Promise.reject(b),'new');return x==='old'?1:0;",
            "let [x]=[false];x&&=(await Promise.reject(b),'new');return x===false?1:0;",
            "let [x]=[null];try{x??=(await Promise.reject(b),'new');}catch(e){return x===null&&e===b?1:0;}return -1;",
            "const other={n:a};let [x]=[null];const value=x??=(await b,other);return x===other&&value===other?1:0;",
            "let [x]=[null];const read=()=>x;const value=x??=(x=false,await b,'new');return read()===value?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn awaited_number_origins_compose_with_boxed_storage_and_writes() {
    compare_promises(
        &[
            "const o={x:await a};return o.x+b;",
            "let [x]=[a];return x+=await b;",
            "let [x]=[a];const result=x&&=await b;return +result;",
            "const o={x:a};const result=o.x||=await b;return +result;",
            "const o={x:await (await a)};return o.x+b;",
            "let n=0;const o={x:await (n++,a)};return o.x+n;",
            "let [x]=[a];const result=x+=await (x=b,b);return result+x;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn number_property_writes_save_old_values_and_commit_only_after_await_success() {
    compare_promises(
        &[
            "let current={n:a};const old=current;const value=current.n+=(await b,current={n:b},b);return old.n===value&&current.n===b?1:0;",
            "const o={n:a};try{o.n+=await Promise.reject(b);}catch(e){await a;return o.n===a&&e===b?1:0;}return -1;",
            "const o={n:a};await b;return o.n++;",
            "const o={n:a};await b;return o.n+=b;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn number_property_logical_writes_select_awaits_and_commit_on_success() {
    compare_promises(
        &[
            "const o={n:a};const value=o.n&&=(await b,b);return +value;",
            "const values=[a];const value=values[0]||=(await b,b);return +value;",
            "const o={n:a};const value=o.n??=(await Promise.reject(b),b);return +value;",
            "const o={n:a};try{o.n||=(await Promise.reject(b),b);}catch(e){return e;}return +o.n;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn boxed_number_local_writes_save_old_values_and_commit_after_await_success() {
    compare_promises(
        &[
            "let [x]=[a];const value=x+=(await b,x=b,b);return x===value?value:NaN;",
            "let {x}={x:a};try{x-=await Promise.reject(b);}catch(e){await a;return x===a&&e===b?1:0;}return -1;",
            "let [x]=[a];await b;return x++;",
            "let {x}={x:a};const value=x-=await b;return x===value?value:NaN;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn boxed_number_local_logical_writes_select_awaits_and_commit_on_success() {
    compare_promises(
        &[
            "let [x]=[a];const value=x&&=(await b,b);return +value;",
            "let {x}={x:a};const value=x||=(await b,b);return +value;",
            "let [x]=[a];const value=x??=(await Promise.reject(b),b);return +value;",
            "let {x}={x:a};try{x||=(await Promise.reject(b),b);}catch(e){return e;}return +x;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn typeof_unresolvable_names_keep_text_and_control_flow_across_awaits() {
    compare_promises(
        &[
            "const kind=typeof missingFeature;await a;return kind==='undefined'?b:a;",
            "await a;let n=0;const kind=()=>typeof missingFeature;return kind()==='undefined'?(n++,n):-1;",
            "const kind=typeof missingFeature;try{await Promise.reject(b);}catch(e){return kind==='undefined'&&e===b?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn boxed_numbers_keep_snapshots_and_numeric_results_across_awaits() {
    compare_promises(
        &[
            "const o={x:a};return o.x+(await b,o.x=b);",
            "const {x}={x:a};await b;return x+b;",
            "const o={x:a};const get=t=>t.x;await b;return get(o)+b;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn primitive_and_value_delete_keep_await_order_and_rejections() {
    compare_promises(
        &[
            "let n=0;const removed=delete (n++,a)[(await b,n++,'x')];await a;return removed?n:-1;",
            "const o={x:a};const removed=delete (await b,o.x);await a;return removed&&o.x===a?1:0;",
            "const o={x:a};try{delete (await Promise.reject(b),o.x);}catch(e){await a;return o.x===a&&e===b?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn runtime_delete_keys_keep_saved_objects_across_waits_and_rejections() {
    compare_promises(
        &[
            "let current={x:a};const original=current;delete current[(await a,current={x:b},'x')];await b;return original.x===undefined&&current.x===b?1:0;",
            "const o={x:a};let n=0;const removed=delete o[(n++,await a,'x')];await b;return removed&&n===1&&o.x===undefined?1:0;",
            "const o={x:a};try{delete o[(await Promise.reject(b),'x')];}catch(e){await a;return o.x===a&&e===b?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn property_deletes_preserve_references_and_effects_across_suspension() {
    compare_promises(
        &[
            "const record={x:a};const saved=record;await b;delete record.x;await a;return saved.x===undefined&&!Object.hasOwn(saved,'x')?1:0;",
            "const xs=[a,b];delete (await a,xs)[0];await b;return xs.length===2&&xs[0]===undefined&&xs[1]===b?1:0;",
            "const record={x:a};const remove=()=>delete record.x;await a;const removed=remove();await b;return removed&&record.x===undefined?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn string_indices_save_operands_and_numeric_results_across_awaits() {
    compare_promises(
        &[
            "let s='abcabc';const pos=s.indexOf((await a,s='changed','bc'),(await b,2));await a;return pos===4&&s==='changed'?1:0;",
            "let needle='bc';const pos='abcabc'.lastIndexOf(needle,(await a,needle='changed',3));await b;return pos===1&&needle==='changed'?1:0;",
            "let n=0;const pos='abcabc'.lastIndexOf('bc',void(await a,n=1),(await b,n=n*10+2));await a;return pos===4?n:-1;",
            "const first='abcabc'.indexOf('bc',(await a,NaN));const last='abcabc'.lastIndexOf('bc',(await b,NaN));await a;return first*10+last;",
            "const pos='abcabc'.indexOf('bc',(await a,2));await b;return pos;",
            "let n=0;'abc'.indexOf((await a,n=1,'a'),(await b,n=n*10+2,0),(await a,n=n*10+3));'abc'.lastIndexOf('c',void(await b,n=n*10+4));return n;",
            "let n=0;try{'abc'.lastIndexOf((await a,'c'),await Promise.reject(b),n=1);}catch(e){return n;}return -1;",
            "const label=' Notes.md '.trim();await a;const end=label.lastIndexOf('.');await b;return label.slice(0,end)==='Notes'?end:-1;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn string_ends_with_saves_arguments_and_boolean_results_across_awaits() {
    compare_promises(
        &[
            "let s='abc';const found=s.endsWith((await a,s='changed','bc'),(await b,3));await a;return found&&s==='changed'?1:0;",
            "let needle='bc';const found='abc'.endsWith(needle,void(await a,needle='changed'));await b;return found&&needle==='changed'?1:0;",
            "let n=0;const found=(n=1,'abc').endsWith((await a,n=n*10+2,'bc'),void(await b,n=n*10+3),(await a,n=n*10+4));await b;return found?n:-1;",
            "await a;return 'abc'.endsWith('bc',void(await b));",
            "const found='abc'.endsWith('missing');await a;return found;",
            "let n=0;'abc'.endsWith((await a,n=1,'bc'),(await b,n=n*10+2,3),(await a,n=n*10+3));return n;",
            "let n=0;try{'abc'.endsWith((await a,'bc'),await Promise.reject(b),n=1);}catch(e){return n;}return -1;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn string_trim_preserves_snapshots_and_ignored_argument_effects_across_awaits() {
    compare_promises(
        &[
            "let s=' x ';const text=s.trim((await a,s='changed'),(await b,s='later'));await a;return text==='x'&&s==='later'?1:0;",
            "let s=' x ';const text=s.trimStart((await a,s='changed'));await b;return text==='x '&&s==='changed'?1:0;",
            "let n=0;const text=(n=1,' x ').trimEnd(void(await a,n=n*10+2),(await b,n=n*10+3));await a;return text===' x'?n:-1;",
            r"const text=' \ud800\u{1f600}\t'.trim();await a;return text==='\ud800\u{1f600}'?text.length:0;",
            "let n=0;' x '.trim((await a,n=1),(await b,n=n*10+2));return n;",
            "let s='original';try{s=' x '.trim(await Promise.reject(a));}catch(e){return s==='original'&&e===a?1:0;}return 0;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn string_search_saves_operands_and_boolean_results_across_awaits() {
    compare_promises(
        &[
            "let s='abc';const found=s.startsWith((await a,s='changed','a'),(await b,0));await a;return found&&s==='changed'?1:0;",
            "let needle='b';const found='abc'.includes(needle,(await a,needle='changed',0),(await b,0));await a;return found&&needle==='changed'?1:0;",
            "let n=0;const found='abc'.includes((await a,n=1,'a'),void(await b,n=n*10+2),(await a,n=n*10+3));await b;return found?n:-1;",
            "await a;return 'abc'.startsWith('a',(await b,0));",
            "const found='abc'.includes('missing');await a;return found;",
            "let n=0;'abc'.startsWith('a',(await a,n=1,0),(await b,n=n*10+2));return n;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn string_characters_preserve_inputs_and_results_across_suspension() {
    compare_promises(
        &[
            "let s='abc';const c=s.charAt((await a,s='xyz',1),(await b,s='later'));await a;return c==='b'&&s==='later'?1:0;",
            "let s='abc';const n=s.charCodeAt((await a,s='xyz',1),(await b,s='later'));await a;return n===98&&s==='later'?1:0;",
            r"const c='\u{1f600}'.charAt((await a,1));await b;return c==='\ude00'&&c.length===1?1:0;",
            "let n=0;const code='abc'.charCodeAt(void(await a,n=1),(await b,n=n*10+2));await a;return code===97?n:-1;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn string_ranges_save_receiver_and_endpoints_across_awaits() {
    compare_promises(
        &[
            "let s='abcdef';const text=s.slice((await a,s='changed',1),(await b,3));await a;return text==='bc'&&s==='changed'?1:0;",
            "let n=0;const text='abcdef'.substring((await a,n=1,4),(await b,n=n*10+2,1),(await a,n=n*10+3));await b;return text==='bcd'?n:-1;",
            "let n=0;const text='abcdef'.slice(void(await a,n=1),void(await b,n=n*10+2));await a;return text==='abcdef'?n:-1;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn dictionary_writes_preserve_receiver_and_key_snapshots_across_await() {
    compare_promises(
        &[
            "let o={x:1,y:2};const old=o;const key=a?'x':'y';o[key]=(o={},await b);return old[key]===b&&o[key]===undefined?1:0;",
            "let d=Object.create(null);const old=d;let key='x';d[key]=(await a,key='y',d=Object.create(null),b);await b;return old.x===b&&old.y===undefined&&d.x===undefined?1:0;",
            "let d=Object.create(null);const old=d;d[(await a,d=Object.create(null),'x')]=(await b,a);return old.x===a&&d.x===undefined?1:0;",
            "const d=Object.create(null);const key='__proto__';const value=d[key]=(await b,a);await a;return d.__proto__===a&&value===a?1:0;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn dictionary_reads_preserve_receiver_and_value_snapshots_across_await() {
    compare_promises(
        &[
            "let n=0;const x=(n++,await a)?[b]:{};return (x instanceof Array?n+1:n);",
            "let o=a?{x:b}:null;const key='x';let n=0;const v=o?.[(n++,o=null,await a,key)];return n*10+(v===b?1:0);",
            "let d=Object.create(null);const old=d;d.x=a;const value=d[(await a,d=Object.create(null),'x')];await b;return value===a&&d!==old?1:0;",
            "const d=Object.create(null);d.x=a;const key='x';const value=d[(await b,d.x=b,key)];await a;d.x=a;return value===b?1:0;",
            "const d=Object.create(null);d.__proto__=a;const key=(await b,'__proto__');const value=d[key];await a;return value===a?1:0;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn dictionary_static_values_and_aliases_survive_suspension() {
    compare_promises(
        &[
            "const d=Object.create(null);d.__proto__=a;const saved=d.__proto__;await b;d.__proto__=b;await a;return saved===a&&d.__proto__===b?1:0;",
            "let d=Object.create(null);const alias=d;d.constructor=a;await b;d=Object.create(null);d.constructor=b;await a;return alias!==d&&alias.constructor===a&&d.constructor===b?1:0;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_pop_preserves_live_length_and_saved_results_across_suspension() {
    compare_promises(
        &[
            "let x=[a,b];const old=x;const saved=x.pop((await a,x=[b]),(await b,old.length=1));await a;return saved===a&&old.length===0&&x.length===1?1:0;",
            "const o={x:a};const x=[o];const saved=x.pop();await b;x[0]=null;await a;return saved===o?1:0;",
            "const x=[a,b];const take=()=>x.pop();await a;const saved=take();await b;return saved===b&&x.length===1?1:0;",
            "const x=[a,b];try{x.pop(await Promise.reject(a));}catch(e){return x.length===2&&x[1]===b?1:0;}return 0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_at_preserves_snapshots_through_suspended_arguments_and_results() {
    compare_promises(
        &[
            "const x=[a,b];const selected=x.at((await a,0),(await b,x[0]=b));await a;return selected===b?1:0;",
            "let x=[a,b];const old=x;const selected=x.at((await a,x=[b],-1),(await b,old.length=1));await a;return selected===a&&old!==x?1:0;",
            "const o={x:a};const x=[o];const saved=x.at(0);await b;x[0]=null;await a;return saved===o?1:0;",
            "const x=Object.keys({...{first:a,second:b}});await a;return x.at(-1)==='second'?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_at_defaults_keep_element_aliases_and_lazy_defaults_across_awaits() {
    compare_promises(
        &[
            "let n=0;const rows=[{label:' first '},{label:' second '}];const {item=(await b,n++,{label:' missing '})}={item:rows.at(await a)};await b;return item.label.trim().length*10+n;",
            "let rows=[{label:' first '},{label:' second '}];const old=rows;const {item={label:' missing '}}={item:rows.at((await b,rows=[{label:' new '}],a))};await b;old[0].label=' changed ';await a;return item.label.trim().length;",
        ],
        &[
            (0.0, 2.0),
            (-1.0, 0.0),
            (4.0, 3.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn nullish_defaults_keep_selected_receivers_and_lazy_awaits() {
    compare_promises(
        &[
            "let n=0;const rows=[{label:' first '},{label:' second '}];const item=rows.at(await a)??(await b,n++,{label:' missing '});await b;return item.label.trim().length*10+n;",
            "const source={label:' first '};const rows=[source];const item=rows.at(await a)??(await b,{label:' missing '});await b;source.label=' changed ';return item.label.trim().length;",
        ],
        &[
            (0.0, 2.0),
            (-1.0, 0.0),
            (4.0, 3.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn logical_defaults_keep_selected_receivers_and_lazy_awaits() {
    compare_promises(
        &[
            "let n=0;const receiver=a?{}:null;const item=(receiver&&(await b,n++,{label:' enabled '}))??{label:' disabled '};await a;return item.label.trim().length*10+n;",
            "let n=0;const text=(a?'':' ready ')||(await b,n++,' fallback ');await a;return text.trim().length*10+n;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn string_concat_retains_left_snapshots_across_await() {
    compare_promises(
        &[
            "let s='left';const text=s+(await a,s='right');await b;return text==='leftright'&&s==='right'?1:0;",
            r"const left='\ud800';const text=left+(await a,'\udfff');await b;return text==='\ud800\udfff'&&text.length===2?1:0;",
            "let s='';const add=()=>s=s+'x';await a;add();await b;add();return s.length;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn string_length_keeps_utf16_snapshots_across_suspension() {
    compare_promises(
        &[
            r"const s='\ud800\u{1f600}';await b;return s.length;",
            "let s='old';const saved=s;const f=()=>s.length;await a;s='longer';await b;return saved['length']*10+f();",
            "const o={text:a?'one':''};const saved=o.text;await b;o.text='replacement';return saved.length*100+o.text.length;",
            "let n=0;const s='three';return (await a,n++,s).length+n;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn object_keys_nullable_input_uses_a_catchable_typeerror() {
    compare_promises(
        &[
            "const source=a?{x:b}:null;try{const keys=Object.keys(source);return keys.length;}catch(e){return typeof e==='object'?-1:0;}",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn object_keys_fallible_inputs_preserve_finalizers_and_argument_order() {
    compare_promises(
        &[
            "const source=a?'xy':null;try{const k=Object.keys(source);await b;return k.length;}catch(e){await b;return typeof e==='object'&&e===e?-1:0;}",
            "const source=a?[a,,b]:true;const k=Object.keys(source);await b;return a?(k.length===2&&k[1]==='2'?1:0):(k.length===0?1:0);",
            "const f=()=>b;const source={...{f}};const k=Object.keys(source.f);await b;return k.length===0?1:0;",
            "const source=a?a:null;try{return Object.keys(source).length;}catch(e){return -1;}",
            "let n=0;try{try{Object.keys();}catch(e){await a;throw e;}finally{await b;n++;}}catch(e){return n===1&&typeof e==='object'?1:0;}return 0;",
            "try{Object.keys(undefined);}finally{await a;return b;}",
            "let n=0;try{Object.keys.call((n++,null),null,(n++,await Promise.reject(b)));}catch(e){return n===2&&e===b?1:0;}return 0;",
            "let n=0;try{Object.keys(null,(n++,await a));}catch(e){return n===1&&typeof e==='object'?1:0;}return 0;",
            "const keys=Object.keys({one:a,two:b});await a;let n=0;for(const key of keys){await b;n+=key.trim().length;}return n;",
            "const o={x:a};const k=Object.keys(o,(await b,o.y=b));await a;return k.length===2&&k[1]==='y'?1:0;",
            "try{Object.keys(null);}catch(e){const k=Object.keys(e);const o={e};await a;return k.length===0&&o.e===e&&typeof e==='object'&&!!e&&+(e)!==+(e)?1:0;}return 0;",
            "let saved=null;try{Object.keys(null);}catch(e){saved=e;}try{Object.keys(undefined);}catch(e){await a;return e!==saved&&!(e<saved)&&e<=saved&&!(e>saved)&&e>=saved?1:0;}return 0;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn object_keys_uses_js_error_string_order_across_intrinsic_causes() {
    compare_promises(
        &[
            "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();try{await p;}catch(cycle){try{Object.keys(null);}catch(toObject){await b;return toObject<cycle&&toObject<=cycle&&!(toObject>cycle)&&!(toObject>=cycle)&&cycle>toObject&&cycle>=toObject&&!(cycle<toObject)&&!(cycle<=toObject)?1:0;}}return 0;",
        ],
        &[(4.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn object_spread_composes_suspended_values_and_preserves_source_order() {
    compare_promises(
        &[
            "const source={x:a};let n=0;const copy={before:(n++,a),...(await b,n++,source),after:(n++,source.x=b)};await a;return n===3&&copy.x===a&&copy.after===b?1:0;",
            "const source={x:a};const f=async()=>{await b;return 1;};const p=f();const copy={...source,...(await p,source)};await b;return copy.x===a?1:0;",
            "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();try{await p;}catch(e){const copy={...e,x:a};await b;return copy.x===a&&copy.message===undefined?1:0;}return 0;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn large_managed_literals_keep_construction_scratch_separate_from_async_roots() {
    use wasmtime::{Collector, Config, Engine, Instance, Module, Store};

    let text = "x".repeat(10_001);
    for source in [
        format!("export async function run(a,b){{const s='{text}';await b;return s?a:b;}}"),
        format!("export async function run(a,b){{await b;const s='{text}';await a;return s?a:b;}}"),
        format!("export async function run(a,b){{const o={{'{text}':a}};await b;return o?a:b;}}"),
        format!(
            "export async function run(a,b){{const o={{}};await b;return o['{text}']===undefined?a:b;}}"
        ),
        format!(
            "function make(){{return '{text}';}}export async function run(a,b){{const s=make();await b;return s?a:b;}}"
        ),
    ] {
        for ts in [false, true] {
            let artifact = compile_async(&source, ts).unwrap();
            // Large construction measures semantics with a bounded core-engine
            // budget; the ordinary production budget must still exhaust cleanly.
            let mut config = Config::new();
            config.consume_fuel(true).collector(Collector::Copying);
            let engine = Engine::new(&config).unwrap();
            let module = Module::new(&engine, &artifact.wasm).unwrap();
            for (a, b) in [(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)] {
                let mut store = Store::new(&engine, ());
                store.set_fuel(1_000_000).unwrap();
                let instance = Instance::new(&mut store, &module, &[]).unwrap();
                let start = instance
                    .get_typed_func::<(f64, f64), (i32, i32, f64)>(&mut store, "start")
                    .unwrap();
                let resume = instance
                    .get_typed_func::<(i32, f64), (i32, i32, f64)>(&mut store, "resume")
                    .unwrap();
                let mut state = start.call(&mut store, (a, b)).unwrap();
                let mut suspensions = 0;
                while state.0 == 1 {
                    suspensions += 1;
                    assert!(suspensions <= 2);
                    store.gc(None).unwrap();
                    state = resume.call(&mut store, (state.1, state.2)).unwrap();
                }
                assert!(suspensions > 0);
                assert_eq!((state.0, state.1), (0, 3));
                assert!(state.2.is_nan() && a.is_nan() || state.2.to_bits() == a.to_bits());
            }
            let error = match AsyncInstance::new(&artifact.wasm, 4.0, 2.0) {
                Err(error) => error,
                Ok(mut activation) => loop {
                    match activation.run_one_job() {
                        Err(error) => break error,
                        Ok(true) => (),
                        Ok(false) => {
                            panic!("large construction escaped the production fuel budget")
                        }
                    }
                },
            };
            assert!(format!("{error:?}").contains("fuel"), "{error:?}");
            run_node(&format!(
                r#"
import assert from 'node:assert/strict';import {{pathToFileURL}} from 'node:url';
const {{createAsyncFunction}}=await import(pathToFileURL(process.argv[2]));
const native=(()=>{{{};return run;}})(),wasm=createAsyncFunction(new Uint8Array({:?}));
async function observe(run,a,b){{const trace=['returned'];const done=run(a,b).then(v=>trace.push(['fulfilled',v]));let n=0;const markers=new Promise(resolve=>{{function tick(){{trace.push(++n);if(n<12)queueMicrotask(tick);else resolve();}}queueMicrotask(tick);}});await Promise.all([done,markers]);return trace;}}
for(const [a,b] of [[4,2],[-0,0],[NaN,Infinity]])assert.deepEqual(await observe(wasm,a,b),await observe(native,a,b));
assert.deepEqual(await Promise.all([wasm(4,2),wasm(-0,0)]),await Promise.all([native(4,2),native(-0,0)]));
"#,
                source.replacen("export async function run", "async function run", 1),
                artifact.wasm
            ));
        }
    }
}

#[test]
fn managed_objects_keep_identity_and_alias_mutation_across_await() {
    compare_promises(
        &["const o={x:a};const p=o;await b;p.x=b;return o===p&&o.x===b?1:0;"],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn managed_arrays_strings_and_saved_values_survive_suspension() {
    compare_promises(
        &[
            "const x=[a,,b];await b;x[1]=b;x.length=5;return x.length+(x[1]===b?1:0);",
            "const xs=[a,b];await a;xs[0]=b;return xs[0]===b?1:0;",
            "const o={child:{x:a}};await b;o.child.x=b;return o.child.x===b?1:0;",
            "const s=a?'x':'';await b;return s==='x'?a:b;",
            "const o={x:a?undefined:null};const saved=o.x;await b;o.x=b;return saved===null?b:a;",
            "let o={x:a};const saved=o;await b;o={x:b};return saved!==o&&saved.x===a&&o.x===b?1:0;",
            "const o={};o.self=o;await a;return o.self===o?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn managed_values_follow_existing_jobs_handlers_and_loop_completions() {
    compare_promises(
        &[
            "const o={x:a};for(let i=0;i<3;i++){await i;o.x=b;}return o.x===b?1:0;",
            "const o={x:a};try{await Promise.reject(b);}catch(e){o.x=e;}finally{await a;}return o.x===b?1:0;",
            "const o={x:a};const n=o?a:await Promise.reject(b);return n;",
            "const o={x:a};await b;return o?null:undefined;",
            "const o={x:a};await b;return Promise.resolve(o.x?a:b);",
            "const o={x:a};await b;return Promise.reject(o.x?a:b);",
            "const o={};const f=async x=>x;await f(a);return o?1:0;",
            "const f=async x=>{const o={x};await b;return o.x===x?1:0;};return await f(a);",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn synchronous_managed_helpers_keep_reference_signatures_in_async_modules() {
    compare_async_sources(
        &[
            "export async function run(a,b){const kind=typeof a;await b;return kind==='number'?a:b;}".into(),
            "function kind(x){return typeof x==='number'?1:0;}export async function run(a,b){await a;return kind(b)?a+b:a;}".into(),
            "function identity(o){return o;} export async function run(a,b){const o={};return identity(o)===o?a:b;}".into(),
            "function identity(o){return o;}export async function run(a,b){const o={x:a};const p=identity(o);await b;p.x=b;return p===o&&o.x===b?1:0;}".into(),
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn public_managed_values_and_unrepresented_promise_payloads_still_reject() {
    for body in [
        "const o={};await a;return o;",
        "const o={};await o;return a;",
        "const o={};await a;throw o;",
        "const p=Promise.resolve(a);const o={p};await b;return o.p===p?1:0;",
        "const p=Promise.resolve(a);try{await Promise.reject(p);}catch(e){const o={x:e};await b;return o.x===e?1:0;}return a;",
    ] {
        let source = format!("export async function run(a,b){{{body}}}");
        for ts in [false, true] {
            let result = std::panic::catch_unwind(|| compile_async(&source, ts))
                .unwrap_or_else(|_| panic!("compiler panic: {source}"));
            let error = result
                .err()
                .unwrap_or_else(|| panic!("unexpected admission: {source}"));
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

#[test]
fn former_managed_async_guards_execute_the_original_sources() {
    compare_async_sources(
        &[
            "function identity(o){return o;} export async function run(a,b){const o={};return identity(o)===o?a:b;}".into(),
            "export async function run(a,b){const x=[a];return x.length;}".into(),
            "export async function run(a,b){const s='x';return s==='x'?a:b;}".into(),
            "export async function run(a,b){const o={x:a};return o.x?a:b;}".into(),
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn managed_async_values_keep_erased_types_and_small_stack_limits() {
    let js = "export async function run(a,b){const xs=[a,b];await a;xs[0]=b;return xs[0]===b?1:0;}";
    let ts = "export async function run(a:string,b:boolean):void{const xs=[a,b];await(a as unknown);(xs as never)[0]=b;return(xs[0]as Missing)===b?1:0;}";
    assert_eq!(
        compile_async(js, false).unwrap().wasm,
        compile_async(ts, true).unwrap().wasm
    );
    for (count, admitted) in [(24, true), (129, false)] {
        let aliases: String = (1..=count)
            .map(|i| format!("const o{i}=o{};", i - 1))
            .collect();
        for ts in [false, true] {
            let source = format!(
                "export async function run(a,b){{const o0={{x:a}};{aliases}await b;return o{count}===o0?1:0;}}"
            );
            let result = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || compile_async(&source, ts))
                .unwrap()
                .join()
                .unwrap();
            assert_eq!(result.is_ok(), admitted, "{:?}", result.as_ref().err());
            if let Err(error) = result {
                assert!(error.contains("unsupported"), "{error}");
            }
        }
    }
}

#[test]
fn has_own_preserves_receiver_and_key_across_awaits() {
    compare_promises(
        &[
            "let o={},k='x';const old=o;const present=Object.hasOwn(o,(await a,k),(await b,k='y',o={},old.x=undefined));await a;return present&&!Object.hasOwn(o,'x')?1:0;",
            "const x=[a];const present=Object.hasOwn(x,'0',(await a,x.length=0));await b;return !present&&Object.hasOwn(x,'length')?1:0;",
            "let n=0;try{Object.hasOwn({},(await Promise.reject(a),'x'),n=1);}catch(e){return n===0&&e===a?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn has_own_runs_void_key_effects_before_ignored_arguments() {
    compare_promises(
        &[
            "let n=0;const o={};const present=Object.hasOwn(o,void (await a,n=1),(await b,n=n*10+2,o.undefined=a));return present&&n===12?1:0;",
            "let n=0;try{Object.hasOwn({},void (await Promise.reject(a)),n=1);}catch(e){return n===0&&e===a?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn has_own_string_receivers_keep_their_utf16_snapshot_across_await() {
    compare_promises(
        &[
            "let s='abc',key='2';const present=Object.hasOwn(s,(await a,s='',key),(await b,key='missing'));return present&&s.length===0?1:0;",
            r"const s='\u{1f600}';await b;return Object.hasOwn(s,1)&&!Object.hasOwn(s,2)?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn dictionary_in_saves_the_key_across_receiver_awaits() {
    compare_promises(
        &[
            "const d=Object.create(null);let k='old',n=0;const present=(await a,n=1,k) in (await b,k='new',n=n*10+2,d.old=undefined,d);return present&&n===12?1:0;",
            "const d=Object.create(null);let n=0;try{(await Promise.reject(a),'x') in (n=1,d);}catch(e){return n===0&&e===a?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn inherited_in_keys_preserve_awaits_in_known_receivers() {
    compare_promises(
        &[
            "let n=0;const present='constructor' in (await a,n=1,{});const method='map' in (await b,n=n*10+2,[]);return present&&method&&n===12?1:0;",
            "let n=0;try{'map' in (await Promise.reject(a),n=1,[]);}catch(e){return n===0&&e===a?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn dynamic_in_keeps_the_saved_name_across_receiver_suspension() {
    compare_promises(
        &[
            "let k='map';const list=[];const yes=(await a,k) in (await b,k='missing',list);return yes&&!(k in list)?1:0;",
            "let n=0;const k='toString';try{k in (await Promise.reject(a),n=1,{});}catch(e){return n===0&&e===a?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn computed_literal_keys_survive_value_awaits_and_key_rejections() {
    compare_promises(
        &[
            "let k='first',n=0;const obj={[(await a,n=1,k)]:(await b,k='second',n=n*10+2,a),[k]:b};await a;return n===12&&obj.first===a&&obj.second===b?1:0;",
            "let n=0;try{const obj={[(await Promise.reject(a),'x')]:(n=1,b),tail:(n=2,a)};n=3;}catch(e){return n===0&&e===a?1:0;}return -1;",
            "let n=0;try{const obj={[(n=1,'x')]:(await Promise.reject(a),b),tail:(n=2,a)};n=3;}catch(e){return n===1&&e===a?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn object_values_keeps_snapshot_order_and_routes_nullish_typeerrors() {
    compare_promises(
        &[
            "let o={x:a};const saved=o;const vs=Object.values(o,(await b,saved.x=b,o={y:a}));await a;saved.x=a;return vs.length===1&&vs[0]===b?1:0;",
            "let n=0;try{Object.values(null,(await a,n=1));}catch(e){await b;return n===1&&typeof e==='object'?1:0;}return -1;",
            "try{Object.values();}catch(first){try{Object.values(undefined);}catch(second){return first!==second&&typeof first==='object'?1:0;}}return -1;",
            "const input=a?{x:b}:null;try{const values=Object.values(input);await b;return values[0]===b?1:0;}catch(e){return typeof e==='object'?2:0;}",
        ],
        &[(4.0, 2.0), (0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn object_entries_retains_pairs_over_await_and_catches_nullable_receivers() {
    compare_promises(
        &[
            "const src={label:a};const rows=Object.entries(src,(await a,src.label=b));src.label=a;let n=0;for(const [key,value] of rows){await b;if(key.trim()==='label'&&value===b)n++;}return n;",
            "try{Object.entries(null,(await a,b));}catch(err){await b;return typeof err==='object'?1:0;}return -1;",
            "const src=a?{x:b}:null;try{const rows=Object.entries(src);for(const [key,v] of rows)return key==='x'&&v===b?1:0;return 0;}catch(err){return typeof err==='object'?2:0;}",
        ],
        &[(4.0, 2.0), (0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_is_array_retains_its_argument_over_await_and_abrupt_extras() {
    compare_promises(
        &[
            "const row={value:[a]};const result=Array.isArray(row.value,(await b,row.value={}));await a;return result&&!Array.isArray(row.value)?1:0;",
            "let n=0;try{Array.isArray((n=1,[]),(await Promise.reject(b),n=2));}catch(e){return n===1&&e===b?1:0;}return -1;",
            "try{Object.keys(null);}catch(error){await a;return Array.isArray(error)?-1:1;}return 0;",
        ],
        &[(4.0, 2.0), (0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn enumeration_keeps_label_values_and_child_records_across_awaits() {
    compare_promises(
        &[
            "const src={x:' one ',y:' two '};const labels=Object.values(src);await a;src.x='replaced';let n=0;for(const label of labels){await b;n+=label.trim().length;}return n;",
            "const child={label:' old '};const rows=Object.entries({item:child});await a;child.label=' changed ';let n=0;for(const [key,{label}] of rows){await b;n+=key.trim().length+label.trim().length;}return n;",
        ],
        &[(4.0, 2.0), (0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn string_for_of_keeps_the_saved_text_and_code_unit_offset_across_awaits() {
    compare_promises(
        &[
            r"let text=a?'A\u{1f600}B':'\ud800Z';let n=0;for(const ch of (await b,text)){await a;text='new';if(ch==='A')continue;n=n*10+ch.length;}return n;",
            r"let n=0;for(const ch of 'A\u{1f600}B'){try{await a;if(ch.length===2)break;n++;}finally{await b;n+=10;}}return n;",
        ],
        &[(4.0, 2.0), (0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn copy_spread_observes_the_resumed_source_and_retains_fresh_slots() {
    compare_promises(
        &[
            "const src=[,a];const copy=[...(await b,src[1]=b,src)];await a;src[1]=a;return copy!==src&&Object.hasOwn(copy,0)&&copy[1]===b?1:0;",
            "let n=0;try{const copy=[...(await Promise.reject(a),n=1,[b])];n=2;}catch(e){await b;return n===0&&e===a?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn string_copy_spread_keeps_code_point_slots_through_awaits() {
    compare_promises(
        &[
            r"let text='A\u{1f600}';const chars=[...(await b,text)];await a;text='new';return chars.length===2&&chars[1]==='\u{1f600}'?1:0;",
            r"let n=0;try{const chars=[...(await Promise.reject(a),n=1,'x')];n=2;}catch(e){return n===0&&e===a?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_includes_saves_receiver_and_needle_before_later_awaits() {
    compare_promises(
        &[
            "let list=[a],needle='x';const saved=list;const found=list.includes((await a,needle),(await b,list=[],needle='y',saved[0]='x',0));return found&&!list.includes(needle)?1:0;",
            "let n=0;try{[a].includes((n=1,b),(await Promise.reject(a),n=2));}catch(e){return n===1&&e===a?1:0;}return -1;",
            "try{Object.keys(null);}catch(e){const xs=[e];await a;try{Object.keys(undefined);}catch(other){return xs.includes(e)&&!xs.includes(other)?1:0;}}return -1;",
        ],
        &[(4.0, 2.0), (0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_index_search_retains_operands_and_uses_the_live_contents_after_awaits() {
    compare_promises(
        &[
            "const xs=['x','y','x'];let needle='x';const i=xs.lastIndexOf(needle,(await a,needle='y',1),(await b,xs[1]='x'));return i*10+xs.indexOf(needle);",
            "const xs=['x','x'];let n=0;const i=xs.lastIndexOf('x',void(await a,n++),(await b,xs[0]='y'));return n*10+i;",
            "let n=0;try{[a].indexOf((n=1,b),(await Promise.reject(a),n=2));}catch(e){return n===1&&e===a?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_slice_saves_inputs_across_awaits_and_stops_on_rejection() {
    compare_promises(
        &[
            "let src=[a,b];const old=src;const xs=src.slice((await a,0),(await b,src=[],1),(old[0]=b));await b;return xs.length===1&&xs[0]===b&&old.length===2?1:0;",
            "let n=0;const xs=[a,b].slice(void(await a,n++),void(await b,n++));return xs.length+n;",
            "let n=0;try{[a].slice((n=1,0),(await Promise.reject(a),n=2),n=3);}catch(e){return n===1&&e===a?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_slice_record_origins_compose_with_iteration_and_suspension() {
    compare_promises(
        &[
            "const child={label:' x '};const xs=[child].slice(0);await a;child.label=' changed ';let n=0;for(const {label} of xs){await b;n+=label.trim().length;}return n;",
            "const xs=[' x ',' y '].slice((await a,1));const [label='empty']=xs;await b;return label.trim().length;",
        ],
        &[(4.0, 2.0), (0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_reverse_observes_contents_after_awaits_and_preserves_return_aliases() {
    compare_promises(
        &[
            "let xs=[a,b];const old=xs;const alias=xs.reverse((await a,xs=[]),(await b,old[0]=b));await a;alias[0]=a;return old[0]===a&&old[1]===b&&xs.length===0?1:0;",
            "const xs=[a,b];let n=0;try{xs.reverse((n=1,await Promise.reject(a)),n=2);}catch(e){return xs[0]===a&&n===1&&e===a?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_shift_observes_current_contents_after_awaits_and_stops_on_rejection() {
    compare_promises(
        &[
            "let xs=[a,b];const old=xs;const first=xs.shift((await a,xs=[]),(await b,old[0]=b));await a;return first===b&&old.length===1&&old[0]===b&&xs.length===0?1:0;",
            "const xs=[a,b];let n=0;try{xs.shift((n=1,await Promise.reject(a)),n=2);}catch(e){return xs.length===2&&n===1&&e===a?1:0;}return -1;",
        ],
        &[(4.0, 2.0), (0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn reversed_iteration_origins_survive_suspension() {
    compare_promises(
        &[
            "const xs=[{label:' x '},{label:' yy '}];xs.reverse(await a);let n=0;for(const {label} of xs){await b;n=n*10+label.trim().length;}return n;",
        ],
        &[(1.0, 2.0), (0.0, 0.0)],
    );
}

#[test]
fn to_reversed_saves_receiver_and_observes_later_argument_effects() {
    compare_promises(
        &[
            "let xs=[a,b];const old=xs;const ys=xs.toReversed((await a,xs=[]),(await b,old[1]=a));return ys!==old&&ys[0]===a&&old[0]===a&&xs.length===0?1:0;",
            "const xs=[a,b];let n=0;try{xs.toReversed((n=1,await Promise.reject(a)),n=2);}catch(e){return n===1&&xs.length===2&&e===a?1:0;}return -1;",
        ],
        &[(1.0, 2.0), (0.0, 0.0)],
    );
}

#[test]
fn string_at_saves_receiver_index_and_missing_results_across_awaits() {
    compare_promises(
        &[
            "let s='abc';const c=s.at((await a,s='xyz',-1),(await b));await a;return c==='c'&&s==='xyz'?1:0;",
            "const c='abc'.at((await a,Infinity));await b;return c===undefined?1:0;",
            "let n=0;const c='abc'.at(void(await a,n=1),(await b,n=n*10+2));await a;return c==='a'?n:-1;",
            "let n=0;try{'abc'.at(await Promise.reject(b),n=1);}catch(e){return n;}return -1;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn code_point_at_saves_receiver_index_and_boxed_results_across_awaits() {
    compare_promises(
        &[
            r"let s='A\u{1f600}';const c=s.codePointAt((await a,s='xyz',1),(await b));await a;return c===128512&&s==='xyz'?1:0;",
            "const c='abc'.codePointAt((await a,-1));await b;return c===undefined?1:0;",
            "let n=0;const c='abc'.codePointAt(void(await a,n=1),(await b,n=n*10+2));await a;return c===97?n:-1;",
            "let n=0;try{'abc'.codePointAt(await Promise.reject(b),n=1);}catch(e){return n;}return -1;",
            "const c='abc'.codePointAt(await a);await b;return c===undefined?0:c===97?1:c===98?2:c===99?3:-1;",
        ],
        &[
            (1.0, 2.0),
            (0.0, -0.0),
            (-1.0, 0.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn string_at_defaults_remain_lazy_across_suspension() {
    compare_promises(
        &[
            "let n=0;const {c=(await b,n=1,'?')}={c:'abc'.at(await a)};await b;return n*1000+c.charCodeAt(0);",
            "let s='abc';const {c='?'}={c:s.at((await a,s='',-1))};await b;return c.charCodeAt(0);",
            "const [c=(await b,'?')]=['abc'.at(await a)];await b;return c.charCodeAt(0);",
        ],
        &[
            (1.0, 2.0),
            (-1.0, 0.0),
            (4.0, 3.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn is_well_formed_keeps_receiver_and_boolean_across_awaits() {
    compare_promises(
        &[
            r"let s='\ud800';const ok=s.isWellFormed((await a,s='abc'));await b;return ok?1:0;",
            "let n=0;const ok='abc'.isWellFormed((await a,n=1),(await b,n=n*10+2));await a;return ok?n:-1;",
        ],
        &[(1.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn to_well_formed_keeps_receiver_and_repaired_text_across_awaits() {
    compare_promises(
        &[
            r"let s='\ud800';const t=s.toWellFormed((await a,s='abc'));await b;return t==='\ufffd'&&s==='abc'?1:0;",
            "let n=0;const t='abc'.toWellFormed((await a,n=1),(await b,n=n*10+2));await a;return t==='abc'?n:-1;",
        ],
        &[(1.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn concat_method_saves_fragments_across_awaits_and_stops_on_rejection() {
    compare_promises(
        &[
            "let s='a';let t='b';const result=s.concat((await a,t),(await b,s='x',t='y','c'));await a;return result==='abc'&&s==='x'&&t==='y'?1:0;",
            "let n=0;const s='a'.concat((await a,n=1,'b'),(await b,n=n*10+2,'c'));await a;return s==='abc'?n:-1;",
            "let n=0;try{'a'.concat((await a,'b'),(await Promise.reject(b),'c'),(n=1,'d'));}catch(e){return n;}return -1;",
        ],
        &[(1.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn templates_keep_prefixes_and_effect_order_across_awaits() {
    compare_promises(
        &[
            "let s='old';const text=`${s}:${(await a,s='new')}:${(await b,s)}`;await a;return text==='old:new:new'?1:0;",
            "let n=0;try{const text=`${(await a,'a')}${(await Promise.reject(b),'b')}${(n=1,'c')}`;}catch(e){return n;}return -1;",
        ],
        &[(1.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn numeric_string_index_saves_receiver_across_awaits() {
    compare_promises(
        &[
            "let s='abc';const c=s[(await a,s='xyz',1)];await b;return c==='b'&&s==='xyz'?1:0;",
            "const c='abc'[await a];await b;return c===undefined?0:c==='a'?1:c==='b'?2:3;",
        ],
        &[
            (0.0, 2.0),
            (1.0, 2.0),
            (-1.0, 0.0),
            (f64::NAN, 0.0),
            (1.5, 0.0),
        ],
    );
}

#[test]
fn switch_references_keep_saved_values_across_suspension() {
    compare_promises(
        &[
            "const x={};switch(x){case(await a,{}):return 99;case(await b,x):return a;default:return b;}",
            "let x=[];const old=x;switch(x){case(await a,x=[],x):return 99;case(await b,old):return a;}return b;",
            "const f=()=>a;switch(f){case(await a,()=>a):return 99;case(await b,f):return a;}return b;",
            "const f=async()=>a;switch(f){case(await a,f):return await b;}return a;",
            "const x={};let n=0;try{switch(x){case(await Promise.reject(b),x):n=99;}}catch(e){return n;}return a;",
            "const x={};switch(x){case x:return await a;case(await Promise.reject(b),{}):return 99;}return b;",
        ],
        &[(0.0, 2.0), (1.0, 3.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn mixed_switch_values_survive_awaited_case_effects() {
    compare_promises(
        &[
            "const x={};switch(a?x:null){case(await b,null):return b;case x:return a;}return 99;",
            "let x=a?{}:'x';const old=x;switch(x){case(await b,x=null,x):return 99;case old:return a;}return b;",
            "const f=()=>a;switch(a?f:0){case(await b,()=>a):return 99;case(await b,f):return a;default:return b;}",
            "let n=0;try{switch(a?'x':{}){case(await Promise.reject(b),'x'):n=99;}}catch(e){return n;}return a;",
        ],
        &[(0.0, 2.0), (1.0, 3.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn switch_breaks_preserve_awaited_cleanup_and_suffix() {
    compare_promises(
        &[
            "let n=0;switch(a){case 0:n=await b;break;default:n=a;}return n;",
            "let n=0;switch(a){case(await b,0):n=await b;break;default:n=a;}return n;",
            "let n=0;switch(a){case 0:try{await b;break;}finally{n=await b;}default:n=a;}return n;",
            "let n=0;switch(a){case 0:try{await Promise.reject(b);break;}catch(e){n=+e;break;}default:n=a;}return n;",
            "let n=0;for(let i=0;i<2;i++){switch(i){case 0:n=await b;break;default:n+=await a;}n+=10;}return n;",
            "let n=0;switch(a){case 0:try{break;}finally{await Promise.reject(b);}default:n=a;}return n;",
        ],
        &[(0.0, 2.0), (1.0, 3.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn switch_fallthrough_preserves_suspension_selection_and_cleanup_order() {
    compare_promises(
        &[
            "let n=0;switch(a){case 0:n=await b;case 1:n+=await a;default:n+=10;}return n;",
            "let n=0;switch(a){case(await b,0):n++;default:n+=10;case(await b,1):n+=100;}return n;",
            "let n=0;switch(a){case 0:await b;n++;case(await Promise.reject(b),1):n+=10;}return n;",
            "let n=0;switch(a){case 0:try{await b;n++;}finally{n+=await b;}case 1:n+=10;}return n;",
            "let n=0;switch(a){case 0:await Promise.reject(b);case 1:n++;}return n;",
            "const o={x:a};switch(a){case 0:await b;o.x=b;default:await a;o.x=+o.x+1;}return +o.x;",
        ],
        &[(0.0, 2.0), (1.0, 3.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn optional_ts_parameters_preserve_async_artifacts_and_reactions() {
    for (js, ts) in [
        (
            "export async function run(a,b){return a+await b;}",
            "export async function run(a?:unknown,b?:unknown){return a+await b;}",
        ),
        (
            "export async function run(a,b){const f=async(x)=>await x;return await f(a);}",
            "export async function run(a?:unknown,b?:unknown){const f=async(x?:unknown)=>await x;return await f(a);}",
        ),
    ] {
        assert_eq!(
            compile_async(js, false).unwrap().wasm,
            compile_async(ts, true).unwrap().wasm
        );
        compare_async_sources(
            &[js.to_owned()],
            &[(0.0, 2.0), (1.0, 3.0), (f64::NAN, f64::INFINITY)],
        );
    }
}

#[test]
fn omitted_synchronous_helper_parameters_compose_with_await() {
    compare_async_sources(&[
        "function f(x){return x===undefined?1:0;}export async function run(a,b){await a;return f()+await b;}".to_owned(),
        "function f(x,y){return y??x;}export async function run(a,b){return await f(await a);}".to_owned(),
        "function choose(x,y){return y??x;}export async function run(a,b){const selected=choose({v:a});await b;return +selected.v;}".to_owned(),
    ],&[(0.0,2.0),(1.0,3.0),(f64::NAN,f64::INFINITY)]);
}

#[test]
fn omitted_synchronous_arrow_parameters_compose_with_await() {
    compare_promises(
        &[
            "await a;const f=x=>x===undefined?1:0;return f()+await b;",
            "const f=(x,y)=>y??x;return await f(await a);",
            "const f=(x,y)=>y??x;const selected=f({v:a});await b;return +selected.v;",
        ],
        &[(0.0, 2.0), (1.0, 3.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn omitted_runtime_closure_parameters_compose_with_suspension() {
    compare_promises(
        &[
            "const f=a?(x=>x===undefined?1:0):(x=>x===undefined?2:0);await b;return f();",
            "let f=(x,y)=>y??x;return +f(await a);",
            "let f=(x,y)=>y===undefined?x:0;const r=f((await b,f=(x,y)=>0,a));return +r;",
            "let choose=(x,y)=>y??x;const selected=choose({v:a});await b;return +selected.v;",
        ],
        &[(0.0, 2.0), (1.0, 3.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn factory_created_local_results_survive_suspension() {
    compare_promises(
        &[
            "const f=x=>()=>x;const g=f();await a;return typeof g()==='undefined'?a:b;",
            "const f=x=>()=>x;const g=f({v:a});await b;return +g().v;",
            "const f=x=>()=>x;const g=f([a]);const saved=g();await b;return +saved[0];",
        ],
        &[(0.0, 2.0), (1.0, 3.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn synchronous_bare_helper_returns_keep_async_tags_and_reactions() {
    compare_async_sources(&[
        "function helper(x){return;}export async function run(a,b){return helper(a);}".to_owned(),
        "function f(x){if(x)return null;return;}export async function run(a,b){const saved=f(a);await b;return saved;}".to_owned(),
        "function f(x){if(x)return null;return;}export async function run(a,b){await b;return +f(a);}".to_owned(),
    ],&[(0.0,2.0),(1.0,3.0),(f64::NAN,f64::INFINITY)]);
}

#[test]
fn side_effect_only_helpers_keep_undefined_and_state_across_suspension() {
    compare_async_sources(&[
        "function helper(x){x++;}export async function run(a,b){return helper(a);}".to_owned(),
        "function helper(x){if(x)return;}export async function run(a,b){await b;return helper(a);}".to_owned(),
        "function helper(o){o.x++;}export async function run(a,b){const o={x:a};const result=helper(o);await b;return result??+o.x;}".to_owned(),
        "function unused(x){x++;}export async function run(a,b){unused(a);const f=async x=>{await x;return x;};const make=p=>async()=>{await a;return await p;};const get=make(f(b));return await get();}".to_owned(),
    ],&[(0.0,2.0),(1.0,3.0),(f64::NAN,f64::INFINITY)]);
}

#[test]
fn conditional_helper_completions_keep_tags_across_suspension() {
    compare_async_sources(&[
        "function f(x){if(x)return x;}export async function run(a,b){const saved=f(a);await b;return saved;}".to_owned(),
        "function f(x){if(x)return null;}export async function run(a,b){await b;return f(a);}".to_owned(),
        "function f(x){if(x)return x;}export async function run(a,b){await b;return +f(a);}".to_owned(),
    ],&[(0.0,2.0),(1.0,3.0),(f64::NAN,f64::INFINITY)]);
}

#[test]
fn helper_loop_switch_and_finalizer_results_survive_suspension() {
    compare_async_sources(&[
        "function f(x){while(x){return x;}}export async function run(a,b){const saved=f(a);await b;return saved;}".to_owned(),
        "function f(x){switch(x){case 0:break;default:return 3;}}export async function run(a,b){await b;return f(a);}".to_owned(),
        "function f(x){try{if(x)return x;}finally{x++;}}export async function run(a,b){const saved=f(a);await b;return saved;}".to_owned(),
        "function f(x){try{throw x;}catch(e){if(e)return e;}}export async function run(a,b){await b;return +f(a);}".to_owned(),
    ],&[(0.0,2.0),(1.0,3.0),(f64::NAN,f64::INFINITY)]);
}

#[test]
fn primitive_object_bindings_preserve_suspension_and_fresh_rest_objects() {
    compare_promises(
        &[
            "const {...rest}=await a;return Object.keys(rest).length+await b;",
            "const {...rest}=await (a>b);await b;return Object.keys(rest).length;",
            "const {value:{...rest}}={value:await a};return Object.keys(rest).length+await b;",
            "let n=0;for(const {...rest} of [a,false]){await b;n+=Object.keys(rest).length;}return n;",
        ],
        &[(0.0, 2.0), (1.0, 3.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn primitive_object_assignments_preserve_suspension_rhs_and_results() {
    compare_promises(
        &[
            "let rest={};({...rest}=await a);return Object.keys(rest).length+await b;",
            "let rest={};({...rest}=await (a>b));await b;return Object.keys(rest).length;",
            "const value=({}=await a);return value+await b;",
            "let rest={};const value=({...rest}=await a);await b;return value;",
        ],
        &[(0.0, 2.0), (1.0, 3.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn primitive_object_numeric_field_defaults_survive_suspension() {
    compare_promises(
        &[
            "const {0:x=(b=7)}=await a;return b;",
            "let x={},rest={};({[b>>>0]:x=(b=9),...rest}=await a);await b;return b+Object.keys(rest).length;",
        ],
        &[(0.0, 2.0), (1.0, 3.0), (f64::NAN, f64::INFINITY)],
    );
}
