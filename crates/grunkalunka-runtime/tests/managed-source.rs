use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{Value, ValueInstance};

#[path = "managed/common.rs"]
mod common;

#[test]
fn dictionary_numeric_updates_preserve_primitive_conversions_and_key_snapshots() {
    for body in [
        "const d=Object.create(null);const key='x';d[key]++;return a;",
        "const d=Object.create(null);const key='x';d[key]+=a;return a;",
        "const d=Object.create(null);d.x=a;const k='x';return d[k]+=b;",
        "const d=Object.create(null);d.x=a;const k='x';return d[k]-=b;",
        "const d=Object.create(null);d.x=a;const k='x';return d[k]*=b;",
        "const d=Object.create(null);d.x=a;const k='x';return d[k]/=b;",
        "const d=Object.create(null);d.x=a;const k='x';return d[k]%=b;",
        "const d=Object.create(null);d.x=a;const k='x';return d[k]&=b;",
        "const d=Object.create(null);d.x=a;const k='x';return d[k]|=b;",
        "const d=Object.create(null);d.x=a;const k='x';return d[k]^=b;",
        "const d=Object.create(null);d.x=a;const k='x';return d[k]<<=b;",
        "const d=Object.create(null);d.x=a;const k='x';return d[k]>>=b;",
        "const d=Object.create(null);d.x=a;const k='x';return d[k]>>>=b;",
        "const d=Object.create(null);d.x=a;const k='x';return d[k]++;",
        "const d=Object.create(null);d.x=a;const k='x';return --d[k];",
        "const d=Object.create(null);const k='missing';return d[k]++;",
        "const d=Object.create(null);d.x=a?false:null;const k='x';return ++d[k];",
        "const d=Object.create(null);const k='__proto__';d[k]|=b;return Object.hasOwn(d,k)?+d[k]:99;",
        "const d=Object.create(null);d.x=a;let k='x';const v=d[k]-=(k='y',b);return Object.hasOwn(d,'y')?99:v;",
        "let d=Object.create(null);d.x=a;const old=d;const k='x';const v=d[k]-=(d=Object.create(null),b);return old!==d&&!Object.hasOwn(d,k)?v:99;",
        "let d=Object.create(null);d.x=a;const old=d;const v=d[(d=Object.create(null),'x')]++;return old!==d&&!Object.hasOwn(d,'x')?v:99;",
        "const d=Object.create(null);d.x=a;const k='x';return d[k]*=(d[k]=b,b);",
        "const d=Object.create(null);d.x=a;const k='x';const update=()=>d[k]++;return update();",
        "const d=Object.create(null);d.x=a;const alias=d;const k='x';d[k]++;return +alias[k];",
        "const d=Object.create(null);const k='\\ud800😀';d[k]|=b;return +d[k];",
    ] {
        compare(body);
    }
}

#[test]
fn unsigned_numeric_updates_preserve_old_values_and_saved_targets() {
    for body in [
        "const xs=[];xs[a>>>0]+=b;return a;",
        "const xs=[a];return xs[0>>>0]*=b;",
        "const xs=[a];return xs[0>>>0]/=b;",
        "const xs=[a];return xs[0>>>0]%=b;",
        "const xs=[a];return xs[0>>>0]&=b;",
        "const xs=[a];return xs[0>>>0]|=b;",
        "const xs=[a];return xs[0>>>0]^=b;",
        "const xs=[a];return xs[0>>>0]<<=b;",
        "const xs=[a];return xs[0>>>0]>>=b;",
        "const xs=[a];return xs[0>>>0]++;",
        "const xs=[a];return --xs[0>>>0];",
        "const xs=[];const n=xs[a>>>0]++;return n;",
        "const xs=[false,null,undefined];return xs[a>>>0]+=b;",
        "const o={0:a};return o[0>>>0]-=b;",
        "const d=Object.create(null);return ++d[a>>>0];",
        "const xs=[];xs[2>>>0]|=b;return xs.length===3&&Object.hasOwn(xs,2)&&!Object.hasOwn(xs,0)?1:0;",
        "const xs=[];xs[(-1)>>>0]&=b;return xs.length===0&&Object.hasOwn(xs,4294967295)?1:0;",
        "const xs=[];xs[(-2)>>>0]^=b;return xs.length;",
        "let xs=[a];const old=xs;let key=0;const v=xs[key>>>0]-=(key=1,xs=[b],b);return old!==xs&&!Object.hasOwn(old,1)?v:99;",
        "let xs=[a];const old=xs;const v=xs[(xs=[b],0>>>0)]++;return old!==xs?v:99;",
        "const xs=[a];return xs[0>>>0]*=(xs[0>>>0]=b,b);",
        "const xs=[a];return xs[0>>>0]>>>=b;",
        "const xs=[a];xs[0>>>0]+=b;xs[0>>>0]-=b;xs[0>>>0]*=b;xs[0>>>0]/=b;xs[0>>>0]%=b;xs[0>>>0]&=b;xs[0>>>0]|=b;xs[0>>>0]^=b;xs[0>>>0]<<=b;return xs[0>>>0]>>=b;",
        "const xs=[a];const update=()=>xs[0>>>0]++;return update();",
        "const xs=[a];const alias=xs;xs[0>>>0]++;return +alias[0>>>0];",
    ] {
        compare(body);
    }
}

#[test]
fn unsigned_logical_property_writes_preserve_short_circuit_and_snapshots() {
    for body in [
        "const xs=[a];return +(xs[0>>>0]||=b);",
        "const xs=[a];return +(xs[0>>>0]&&=b);",
        "const xs=[,null,false];return +(xs[a>>>0]??=b);",
        "const o={0:a};return +(o[0>>>0]||=b);",
        "const d=Object.create(null);return +(d[a>>>0]??=b);",
        "const xs=[];xs[2>>>0]??=b;return xs.length===3&&Object.hasOwn(xs,2)?1:0;",
        "const xs=[];xs[(-1)>>>0]||=b;return xs.length===0&&Object.hasOwn(xs,4294967295)?1:0;",
        "const xs=[];let n=0;const result=xs[0>>>0]&&=(n=1,b);return n===0&&result===undefined&&xs.length===0?1:0;",
        "const xs=[false,0];let n=0;xs[0>>>0]??=(n=1,b);xs[1>>>0]??=(n=2,b);return n;",
        "let xs=[0];const saved=xs;let key=0,n=0;const result=xs[(n=1,key>>>0)]||=(n=n*10+2,key=1,xs=[a],b);return n===12&&saved!==xs&&Object.hasOwn(saved,0)&&!Object.hasOwn(saved,1)&&result===b?1:0;",
        "const child={v:a};const xs=[child];return (xs[0>>>0]??={v:b})===child?1:0;",
        "const xs=[0];const apply=()=>+(xs[0>>>0]||=b);return apply();",
    ] {
        compare(body);
    }
}

#[test]
fn unsigned_property_reads_preserve_missing_values_and_snapshots() {
    for body in [
        "const xs=[a,b];return +xs[a>>>0];",
        "const xs=[,a];return xs[a>>>0]===undefined?1:0;",
        "const o={0:a,1:b};return +o[a>>>0];",
        "const d=Object.create(null);d[0]=a;return +d[b>>>0];",
        "const xs=[];xs[a>>>0]=b;return +xs[a>>>0];",
        "const xs=[];xs[4294967295]=b;return +xs[(-1)>>>0];",
        "const xs=[];xs[4294967294]=b;return +xs[(-2)>>>0];",
        "let xs=[a,b];const saved=xs;let n=0;const v=(n=1,xs)[(n=n*10+2,xs=[b],0>>>0)];return n===12&&saved!==xs&&v===a?1:0;",
        "const child={v:a};const xs=[child];return xs[0>>>0]===child?1:0;",
        "const xs=[a,b];delete xs[a>>>0];return xs[a>>>0]===undefined?1:0;",
        "const xs=[a,b];const read=()=>+xs[a>>>0];return read();",
        "const xs=[a,b];return +xs[a?(0>>>0):(1>>>0)];",
    ] {
        compare(body);
    }
}

#[test]
fn unsigned_property_writes_preserve_growth_values_and_snapshots() {
    for body in [
        "const xs=[];xs[a>>>0]=b;return Object.keys(xs).length*10+(Object.hasOwn(xs,0)?1:0)+(Object.hasOwn(xs,1)?2:0);",
        "const o={};o[a>>>b]=b;return Object.keys(o).length*10+(Object.hasOwn(o,0)?1:0);",
        "const d=Object.create(null);d[a>>>0]=b;return Object.keys(d).length;",
        "const xs=[];for(let i=0;i<3;i++)xs[i>>>0]=i;return xs.length*10+Object.keys(xs).length;",
        "const xs=[];xs[(-1)>>>0]=b;return xs.length===0&&Object.hasOwn(xs,4294967295)?1:0;",
        "const xs=[];xs[(-2)>>>0]=a;return xs.length===4294967295&&Object.hasOwn(xs,4294967294)?1:0;",
        "const xs=[];xs[a>>>0]=b;xs.length=0;return Object.keys(xs).length;",
        "let xs=[];const old=xs;let n=0;const result=(n=1,xs)[(n=n*10+2,0>>>0)]=(n=n*10+3,xs=[],b);return n===123&&old!==xs&&Object.hasOwn(old,0)&&!Object.hasOwn(xs,0)&&result===b?1:0;",
        "let key=0;const o={};o[key>>>0]=(key=1,b);return Object.hasOwn(o,0)&&!Object.hasOwn(o,1)?1:0;",
        "const o={[0>>>0]:a};o[0>>>0]=b;return Object.values(o)[0]===b?1:0;",
        "const xs=[];const child={v:a};return (xs[a>>>0]=child)===child?1:0;",
        "const xs=[];xs[a?(0>>>0):(1>>>0)]=b;return Object.hasOwn(xs,0)?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn unsigned_computed_object_keys_preserve_order_values_and_snapshots() {
    for body in [
        "const o={[a>>>0]:b};return Object.keys(o).length*10+(Object.hasOwn(o,0)?1:0)+(Object.hasOwn(o,1)?2:0);",
        "const o={[a>>>b]:undefined};return Object.keys(o).length*10+(Object.hasOwn(o,0)?1:0);",
        "let n=0;const o={[(n=1,a)>>>0]:(n=n*10+2,b)};return n;",
        "let key=0;const o={[key>>>0]:(key=1,b)};return Object.hasOwn(o,0)&&!Object.hasOwn(o,1)&&key===1?1:0;",
        "const o={[a>>>0]:1,[a>>>0]:2};return Object.keys(o).length===1&&Object.values(o)[0]===2?1:0;",
        "const o={z:a,[2>>>0]:b,[1>>>0]:a,y:b};const keys=Object.keys(o);return keys[0]==='1'&&keys[1]==='2'&&keys[2]==='z'&&keys[3]==='y'?1:0;",
        "const o={z:a,[(-1)>>>0]:b,[(-2)>>>0]:a};const keys=Object.keys(o);return keys[0]==='4294967294'&&keys[1]==='z'&&keys[2]==='4294967295'?1:0;",
        "const base={x:a};const o={...base,[a>>>0]:b};return Object.keys(o).length===2&&Object.hasOwn(o,'x')?1:0;",
        "const child={v:a};const o={[0>>>0]:child};return Object.values(o)[0]===child?1:0;",
        "const make=()=>({[a>>>0]:b});const o=make();return Object.keys(o).length;",
        "const o={[a?(0>>>0):(1>>>0)]:b};return Object.hasOwn(o,0)?1:0;",
        "let key=0;const o={[key=a>>>0]:b};return Object.keys(o).length;",
    ] {
        compare(body);
    }
}

#[test]
fn unsigned_in_keys_preserve_presence_boundaries_and_key_snapshots() {
    for body in [
        "const xs=[a,,b];return (a>>>0) in xs?1:0;",
        "const o={0:undefined,1:b};return (a>>>0) in o?1:0;",
        "const d=Object.create(null);d[0]=a;return (a>>>b) in d?1:0;",
        "const o=a?{}:Object.create(null);o[0]=b;return (0>>>0) in o?1:0;",
        "const xs=[a];xs[4294967295]=b;return ((-1)>>>0) in xs&&xs.length===1?1:0;",
        "const xs=[a];xs[4294967294]=b;return ((-2)>>>0) in xs&&xs.length===4294967295?1:0;",
        "let key=0,n=0;const xs=[a];const found=(n=1,key>>>0) in (n=n*10+2,key=1,xs);return found&&n===12&&key===1?1:0;",
        "let key=a;const found=(key>>>0) in (key=b,{0:a});return found===((a>>>0)===0)?1:0;",
        "const xs=[a,b];const before=(a>>>0) in xs;delete xs[a>>>0];return before?((a>>>0) in xs?-1:1):0;",
        "const xs=[a];const has=()=>(a>>>0) in xs;return has()?1:0;",
        "const xs=[a,b];return (a?(0>>>0):(1>>>0)) in xs?1:0;",
        "const xs=[a];let key=0;return (key=a>>>0) in xs?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn loose_null_equality_executes_the_exact_former_object_rejection() {
    compare("const o={};return o==null?a:b;");
}

#[test]
fn runtime_dictionary_logical_writes_save_keys_receivers_and_values() {
    for body in [
        "const d=Object.create(null);const key='x';d[key]??=a;return a;",
        "const d=Object.create(null);const k='x';return (d[k]??=a)===a?1:0;",
        "const d=Object.create(null);const k='x';d[k]||='ok';return d[k]==='ok'?1:0;",
        "const d=Object.create(null);const k='x';d[k]=false;let n=0;const value=d[k]&&=(n++,'new');return value===false&&n===0?1:0;",
        "let d=Object.create(null);const old=d;let k='x';const value=d[(d=Object.create(null),k)]??=(k='y','new');return old.x===value&&old.y===undefined&&d.x===undefined?1:0;",
        "const d=Object.create(null);let n=0;const key=()=>{n++;return 'x';};d.x='old';const value=d[key()]||=(n+=10,'new');d.x='later';return value==='old'&&n===1?1:0;",
        "const d=Object.create(null);const k='__proto__';const other={n:a};const value=d[k]??=other;return d[k]===other&&value===other?1:0;",
        "const d=Object.create(null);const k='x';if(d[k]??=a)return b;return a;",
        "const d=Object.create(null);const k='x';d.x=null;const value=d[(d.x='old',k)]??='new';return value==='old'&&d.x==='old'?1:0;",
        "const d=Object.create(null);const k='x';d.x=true;const value=d[k]&&='new';return value==='new'&&d.x===value?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn mixed_property_logical_assignments_keep_values_and_saved_receivers() {
    for body in [
        "const o={};return o.n||=b;",
        "const o={x:null};return (o.x??='ok')==='ok'?1:0;",
        "const o={};return (o.x??=a)===a?1:0;",
        "const xs=[false];return (xs[0]&&='skip')===false?1:0;",
        "const o={x:''};if(o.x||='ok')return a;return b;",
        "const o=Object.create(null);return (o.__proto__??='ok')==='ok'?1:0;",
        "let current={x:null};const old=current;const value=current.x??=(current={x:false},'new');return old.x===value&&current.x===false?1:0;",
        "const other={n:a};const o={x:null};const value=o.x||=other;return o.x===other&&value===other?1:0;",
        "const o={x:'old'};let n=0;const value=o.x||=(n++,'new');o.x='later';return value==='old'&&n===0?1:0;",
        "const xs=[];xs[2]??='new';return xs.length===3&&xs[0]===undefined&&xs[2]==='new'?1:0;",
        "const o={x:null};const read=()=>o.x;o.x??='new';return read()==='new'?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn boxed_local_logical_assignments_preserve_kinds_identity_and_effects() {
    for body in [
        "let [x]=[null];return (x??='ok')==='ok'?1:0;",
        "let {x}={x:''};return (x||=a)===a?1:0;",
        "let [x]=[false];return (x&&='skip')===false?1:0;",
        "const other={n:a};let [x]=[null];return (x??=other)===other?1:0;",
        "let [x]=[undefined];if(x??=a)return b;return a;",
        "let [x]=[a?null:''];let calls=0;const value=x??=(calls++,b);return calls===(a?1:0)&&value===x?1:0;",
        "let [x]=[null];const read=()=>x;const value=x??=(x=false,'new');return read()===value?1:0;",
        "let [x]=['old'];let n=0;const value=x||=(n++,'new');x='later';return value==='old'&&n===0?1:0;",
        "let [x]=[null];let n=0;x??=(n++,a);return n;",
        "const other=[a];let [x]=[null];const value=x||=other;return value===other&&x===other?1:0;",
        "for(let [x] of [[null]]){const value=x??='ok';return value==='ok'?1:0;}return -1;",
        "const other={n:a};for(let x of [null]){const read=()=>x;x||=other;return read()===other?1:0;}return -1;",
    ] {
        compare(body);
    }
}

#[test]
fn number_property_updates_preserve_old_new_values_and_receiver_snapshots() {
    for body in [
        "const o={n:a};return o.n++;",
        "const o={n:a};return ++o.n;",
        "const o={n:a};return o.n--;",
        "const o={n:a};return --o.n;",
        "const xs=[a,b];xs[0]++;return xs[0]+xs[1];",
        "const o={n:a};const alias=o;const old=o.n++;return alias.n===old+1?1:0;",
        "let n=0;const o={n:a};const get=()=>{n++;return o;};const old=get().n++;return n===1&&o.n===old+1?1:0;",
        "let current={n:a};const original=current;const value=current.n+=(current={n:b},b);return original.n===value&&current.n===b?1:0;",
        "const o={n:a};return o.n+=(o.n=b,a);",
        "const o={n:a};const box={x:o.n++};return box.x+b;",
        "const o={n:a};o.n+=undefined;return o.n;",
        "const o={n:a};const increment=state=>state.n++;return increment(o);",
        "const o={n:a};const increment=()=>++o.n;return increment();",
        "const o={'-1':a};return o[-1]++;",
    ] {
        compare(body);
    }
    for operator in [
        "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=", ">>>=",
    ] {
        compare(&format!("const o={{n:a}};return o.n{operator}b;"));
        compare(&format!("const o={{n:a}};o.n{operator}b;return o.n;"));
    }
    compare_typed(
        "const o={n:a};return o.n+=b;",
        "const o:any={n:a};return (o as {n:number}).n+=(b as number);",
    );
}

#[test]
fn number_property_logical_assignments_save_receivers_and_short_circuit() {
    for body in [
        "let current={n:a};const original=current;const value=current.n&&=(current={n:b},b);return original.n===value?value:NaN;",
        "const values=[a];const value=values[0]||=b;return values[0]===value?value:NaN;",
        "const o={n:a};let calls=0;const value=o.n??=(calls++,b);return calls===0&&value===a&&o.n===a?1:0;",
        "const o={n:a};const read=()=>o.n;const value=o.n||=b;return read()===value?value:NaN;",
    ] {
        compare(body);
    }
}

#[test]
fn boxed_number_local_updates_preserve_old_new_values_and_cells() {
    for body in [
        "let [x]=[a];return x++;",
        "let {x}={x:a};return ++x;",
        "let [x]=[a];return x--;",
        "let {x}={x:a};return --x;",
        "let [x]=[a];const old=x++;return old*100+x;",
        "let {x}={x:a};const read=()=>x;const old=x++;return read()===old+1?1:0;",
        "let [x]=[a];const value=x+=(x=b,b);return value===x?value:NaN;",
        "let {x}={x:a};const add=()=>x+=b;add();return x;",
    ] {
        compare(body);
    }
    for operator in [
        "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=", ">>>=",
    ] {
        compare(&format!(
            "let {{x}}={{x:a}};const value=x{operator}b;return x===value?value:NaN;"
        ));
    }
    compare_typed(
        "let [x]=[a];return x+=b;",
        "let [x]:any=[a];return (x as number)+=(b as number);",
    );
}

#[test]
fn boxed_number_local_logical_assignments_preserve_selection_and_cells() {
    for body in [
        "let [x]=[a];const value=x&&=(x=b,b);return value===x?value:NaN;",
        "let {x}={x:a};const value=x||=(x=b,b);return value===x?value:NaN;",
        "let [x]=[a];let calls=0;const value=x??=(calls++,b);return calls===0&&value===a&&x===a?1:0;",
        "let {x}={x:a};const read=()=>x;const value=x||=b;return read()===value?value:NaN;",
    ] {
        compare(body);
    }
}

#[test]
fn typeof_unresolvable_names_preserves_scope_and_expression_effects() {
    for body in [
        "return typeof missingFeature==='undefined'?1:0;",
        "const kind=typeof ((missingFeature));return kind.length;",
        "let n=0;const kind=typeof missingFeature;return kind==='undefined'?(n++,a):b;",
        "let undefined=a;return typeof missingFeature==='undefined'&&typeof undefined==='number'?1:0;",
        "{const missingFeature=a;if(typeof missingFeature!=='number')return -1;}return typeof missingFeature==='undefined'?1:0;",
        "for(let missingFeature=0;missingFeature<1;missingFeature++){}return typeof missingFeature==='undefined'?1:0;",
        "const kind=()=>typeof missingFeature;return kind()==='undefined'?a:b;",
        r"return typeof \u006dissingFeature==='undefined'?1:0;",
        "typeof missingFeature;return a;",
    ] {
        compare(body);
    }
    let original = "export function run(a,b) { return typeof __grunkalunka_typeof_missing__ === \"undefined\" ? 1 : 0; }";
    common::compare_sources(original, original);
    compare_typed(
        "return typeof missingFeature==='undefined'?1:0;",
        "return typeof (missingFeature as number)==='undefined'?1:0;",
    );
}

#[test]
fn boxed_number_fields_and_patterns_compose_with_arithmetic_and_calls() {
    for body in [
        "return ({x:a}).x+b;",
        "const o={x:a};return o.x;",
        "const xs=[a,b];return xs[1];",
        "const {x}={x:a};return x+b;",
        "const [x,y]=[a,b];return x*y;",
        "let x=a;[x]=[b];return x;",
        "const {x=b}={x:a};return x+b;",
        "const {x=b}={};return x+b;",
        "const o={x:a};const alias=o;alias.x=b;return o.x+b;",
        "const get=()=>a+b;const o={x:get()};return o.x+b;",
        "const o={x:a};const get=t=>t.x+b;return get(o);",
        "const o={x:a};return o.x+(o.x=b);",
        "const o={x:a};const n=o.x;o.x=b;return n;",
        "const o={x:a};return o.x<b?o.x:b;",
        "const o={x:a};return o.x===b?1:0;",
        "const o={x:-0};return o.x;",
        "const o={x:NaN};return o.x;",
        "const o={x:Infinity};return o.x;",
        "const o={x:{value:a}};return o.x.value+b;",
        "let total=0;for(const x of [a,b])total+=x;return total;",
        "let total=0;for(const {x} of [{x:a},{x:b}])total+=x;return total;",
    ] {
        compare(body);
    }
    common::compare_sources(
        "function get(o){return o.x;}export function run(a,b){const o={x:a};return get(o);}",
        "function get(o){return o.x;}export function run(a,b){const o={x:a};return get(o);}",
    );
    compare_typed(
        "const o={x:a};const x=o.x;return x+b;",
        "const o:boolean={x:a};const x:string=o.x as unknown as string;return x+b;",
    );
}

#[test]
fn boxed_numbers_execute_the_unchanged_original_rejection_sources() {
    for source in [
        "export function run(a,b) { return ({x:a}).x + b; }",
        "export function run(a,b) { const items = [a,b]; return items[1]; }",
        "export function run(a,b) { const {x}={x:a}; return x+b; }",
        "export function run(a,b) { let x=a; [x]=[b]; return x; }",
        "export function run(a,b) { for(const i of [a]) { return i; } return b; }",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn primitive_property_deletion_preserves_receiver_and_key_effects() {
    for body in [
        "return delete a.x?1:0;",
        "return delete (a<b).length?1:0;",
        "let n=0;const number=()=>{n=n*10+1;return a;};const key=()=>{n=n*10+2;return 'x';};return delete number()[key()]?n:-1;",
        "let n=0;return delete (n++,a)[(n++,b)]?n:-1;",
        "return delete a[null]&&delete a[undefined]&&delete a[false]?1:0;",
        "const flag=a<b;return delete flag.x?1:0;",
    ] {
        compare(body);
    }
    let original = "export function run(a,b) { return delete a.x ? 1 : 0; }";
    common::compare_sources(original, original);
}

#[test]
fn nonreference_delete_evaluates_values_without_deleting_selected_properties() {
    for body in [
        "const o={x:a};return delete (0,o.x)&&o.x===a?1:0;",
        "const o={x:a,y:b};return delete (a?o.x:o.y)&&Object.hasOwn(o,'x')&&Object.hasOwn(o,'y')?1:0;",
        "let n=0;const make=()=>{n++;return {x:a};};return delete make()?n:-1;",
        "return delete (a=b)?a:0;",
        "const o={x:a};return delete (o.x=b)&&o.x===b?1:0;",
        "let n=0;return delete [n++,n++]?n:-1;",
        "let n=0;return delete {x:n++,y:n++}?n:-1;",
        "let n=0;return delete void(n++)?n:-1;",
        "const o={x:a};return delete delete o.x&&o.x===undefined?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn runtime_delete_keys_keep_receiver_snapshots_and_source_order() {
    for body in [
        "const d=Object.create(null);const key='x';delete d[key];return a;",
        "const o={x:a,y:b};const key=a?'x':'y';return delete o[key]&&!Object.hasOwn(o,key)&&Object.keys(o).length===1?1:0;",
        "let current={x:a};const original=current;const key=()=>{current={x:b};return 'x';};const removed=delete current[key()];return removed&&original.x===undefined&&current.x===b?1:0;",
        "let key='old',n=0;const o={old:a,next:b};const receiver=()=>{n++;key='next';return o;};delete receiver()[key];return n===1&&o.old===a&&o.next===undefined?1:0;",
        "const o={x:a};let n=0;delete o[(n=n*10+1,'missing')];delete o[(n=n*10+2,'x')];return n*10+(o.x===undefined?1:0);",
        "const d=Object.create(null);d.__proto__=a;const key='__proto__';return delete d[key]&&d.__proto__===undefined?1:0;",
        "const o={};const key='constructor';return delete o[key]&&('constructor' in o)?1:0;",
        "const o={x:'text'};const alias=o;const key='x';delete alias[key];const {x='fallback'}=o;return x.length;",
        "const o={x:a,y:b};const remove=(target,key)=>delete target[key];return remove(o,'x')&&o.x===undefined&&o.y===b?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn delete_own_properties_preserves_presence_aliases_and_insertion_order() {
    for body in [
        "const o={x:a,y:b};const alias=o;const removed=delete alias.x;return removed&&o.x===undefined&&!Object.hasOwn(o,'x')&&o.y===b?1:0;",
        "const o={a:1,b:2,c:3};delete o.b;o.b=4;const keys=Object.keys(o);return keys.length===3&&keys[0]==='a'&&keys[1]==='c'&&keys[2]==='b'?1:0;",
        "const o={};return delete o.missing&&delete o.constructor&&('constructor' in o)?1:0;",
        "const o=Object.create(null);o.__proto__=a;o.constructor=b;delete o.__proto__;return o.__proto__===undefined&&o.constructor===b?1:0;",
        "const xs=[a,b];xs.named=a;delete xs[0];delete xs.named;return xs.length===2&&!Object.hasOwn(xs,0)&&xs[0]===undefined&&xs[1]===b&&xs.named===undefined?1:0;",
        "const xs=[a,,b];delete xs[2];const copy=[...xs];return xs.length===3&&copy.length===3&&copy[2]===undefined&&Object.hasOwn(copy,2)?1:0;",
        "const o={x:'text'};delete o.x;const {x='fallback'}=o;return x.length;",
        "const o={a:1,b:2,c:3};let count=0;for(const key in o){count++;delete o.b;}return count;",
        "let n=0;const o={x:a};const receiver=()=>{n++;return o;};return delete receiver().x?n*10+(o.x===undefined?1:0):-1;",
        "const xs=[a,b];delete xs[-0];return !Object.hasOwn(xs,'0')&&xs.length===2?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn delete_large_literal_key_uses_the_bounded_text_constructor() {
    let source = format!(
        "export function run(a,b){{const o={{}};return delete o['{}']?1:0;}}",
        "k".repeat(10_001)
    );
    // Exercise literal construction beyond array.new_fixed's V8 limit with a
    // bounded semantic-oracle budget; the production per-call quota is unchanged.
    common::compare_sources_with_fuel(&source, &source, Some(1_000_000));
}

#[test]
fn dictionary_static_keys_have_no_inherited_properties() {
    for body in [
        "const d=Object.create(null);return d.constructor===undefined&&d.__proto__===undefined&&d.toString===undefined&&Object.keys(d).length===0?a:b;",
        "const d=Object.create(null);d.toString=a;d.constructor=b;d.length=3;return d.toString===a&&d.constructor===b&&d.length===3?a:b;",
        "const d=Object.create(null);const child={x:a};d.__proto__=child;const saved=d.__proto__;d.__proto__=null;const keys=Object.keys(d);return saved===child&&d.__proto__===null&&keys.length===1&&keys[0]==='__proto__'?a:b;",
        r"const d=Object['create']((null));d['\ud800\0']=a;d['\ufffd\0']=b;return d['\ud800\0']===a&&d['\ufffd\0']===b?a:b;",
        "const d=Object.create(null);d.__proto__=a;const copy={...d};return Object.keys(copy)[0]==='__proto__'?a:b;",
    ] {
        compare(body);
    }
}

#[test]
fn dictionary_provenance_survives_helpers_aliases_and_erased_types() {
    for source in [
        "function make(){return Object.create(null);}function set(d,v){d.constructor=v;return d;}function get(d){return d.constructor;}export function run(a,b){const d=set(make(),a);return get(d)===a?a:b;}",
        "export function run(a,b){let d=Object.create(null);const saved=d;d=Object.create(null);saved.__proto__=a;d.__proto__=b;return saved!==d&&saved.__proto__===a&&d.__proto__===b?a:b;}",
        "export function run(a,b){const wrapper={d:Object.create(null)};const get=()=>wrapper.d;get().constructor=a;return wrapper.d.constructor===a?a:b;}",
        "export function run(a,b){const Object={create:x=>x};return Object.create(a);}",
    ] {
        common::compare_sources(source, source);
    }
    common::compare_sources(
        "export function run(a,b){const d=Object.create(null);d.__proto__=a;return d.__proto__===a?a:b;}",
        "type Object=never;export function run(a:string,b:object):void{const d:number=(Object as unknown)['create'](null as number);d['__proto__' as never]=a;return d['__proto__' as never]===a?a:b;}",
    );
}

#[test]
fn dictionary_admission_requires_a_proven_null_prototype() {
    for body in [
        "const d={};return d.constructor===undefined?a:b;",
        "const d=a?Object.create(null):{};return d.__proto__===undefined?a:b;",
        "let d=Object.create(null);d={};d.__proto__=null;return a;",
        "const d=Object.create(null);const copy={...d};return copy.constructor===undefined?a:b;",
        "const d=Object.create({});return d.x===undefined?a:b;",
        "const d=Object.create();return a;",
        "const d=Object.create(null,{});return a;",
        "const d=Object.create.call(null,null);return a;",
        "const d=Object.create(...[null]);return a;",
        "const d=Object?.create(null);return a;",
        "const d={};const key='x';return d[key]===undefined?a:b;",
    ] {
        for typed in [false, true] {
            let source = format!("export function run(a,b){{{body}}}");
            let error = compile(&source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(error.contains("unsupported"), "{source}: {error}");
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
    for (depth, admitted) in [(16, true), (140, false)] {
        let source = format!(
            "export function run(a,b){{const d=Object.create({}null{});return d.constructor===undefined?a:b;}}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(result.is_ok(), admitted, "{:?}", result.as_ref().err());
    }
}

#[test]
fn dictionary_reads_accept_runtime_string_keys_and_snapshot_values() {
    for body in [
        "const d=Object.create(null);d.left=a;d.right=b;const key=a?'left':'right';return d[key]===(a?a:b)?a:b;",
        "const d=Object.create(null);d.sta=a;d.stb=b;const key='st'+(a?'a':'b');return d[key]===(a?a:b)?a:b;",
        "const d=Object.create(null);d.number=a;return d[typeof b]===a?a:b;",
        "const d=Object.create(null);d.x=a;let key='x';const saved=d[key];d.x=b;key='missing';return saved===a&&d[key]===undefined?a:b;",
        "const d=Object.create(null);const child={};const f=()=>a;d.child=child;d.f=f;const key=a?'child':'f';return d[key]===(a?child:f)?a:b;",
        "const d=Object.create(null);d.__proto__=a;d.constructor=b;const key=a?'__proto__':'constructor';return d[key]===(a?a:b)?a:b;",
        r"const d=Object.create(null);d['\ud800\0']=a;d['']=b;const key=a?'\ud800\0':'';return d[key]===(a?a:b)?a:b;",
    ] {
        compare(body);
    }
}

#[test]
fn dictionary_reads_evaluate_receiver_then_key_and_keep_both_effects() {
    for body in [
        "let n=0;let d=Object.create(null);const old=d;d.x=a;const saved=(n=n*10+1,d)[(n=n*10+2,d=Object.create(null),'x')];return saved===a&&n===12&&d!==old?a:b;",
        "const d=Object.create(null);d.x=a;const value=d[(d.x=b,'x')];return value===b?a:b;",
        "const d=Object.create(null);d.x=a;let key='x';const next=()=>{key='missing';return 'x';};return d[next()]===a&&key==='missing'?a:b;",
    ] {
        compare(body);
    }
    let source = "function key(a){return a?'x':'y';}function get(d,k){return d[k];}export function run(a,b){const d=Object.create(null);d.x=a;d.y=b;return get(d,key(a))===(a?a:b)?a:b;}";
    common::compare_sources(source, source);
    common::compare_sources(
        "export function run(a,b){const d=Object.create(null);d.x=a;const key='x';return d[key]===a?a:b;}",
        "export function run(a:string,b:object):void{const d:number=Object.create(null);d.x=a;const key:boolean='x';return (d as unknown)[key as number]===a?a:b;}",
    );
}

#[test]
fn dictionary_reads_keep_unknown_values_and_receivers_unproven() {
    for body in [
        "const d=Object.create(null);return d[a]===undefined?a:b;",
        "const d=Object.create(null);const key=a?'x':b;return d[key]===undefined?a:b;",
        "const d=Object.create(null);const key=['x'].at(a);return d[key]===undefined?a:b;",
        "const d=a?Object.create(null):{};const key='x';return d[key]===undefined?a:b;",
        "const d=Object.create(null);const key='x';return d?.[key]===undefined?a:b;",
        "const d=Object.create(null);d.x=a;const key='x';const s=b?'ok':d[key];return s.length;",
        "const d=Object.create(null);const key='x';const o=a?{}:d[key];return o.x===undefined?a:b;",
        "const d=Object.create(null);const key='x';const f=a?()=>a:d[key];return f();",
        // The source admits this named array property, although key provenance
        // recognizes only canonical array indices below u32::MAX.
        "const xs=[a];const value=b?'s':xs[4294967295];return value.length;",
        "const d=Object.create(null);const key='x';const s:string=a;return d[s]===undefined?a:b;",
    ] {
        let typed_only = body.contains(":string");
        for typed in [false, true] {
            if typed_only && !typed {
                continue;
            }
            let source = format!("export function run(a,b){{{body}}}");
            let error = compile(&source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(error.contains("unsupported"), "{source}: {error}");
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
    for (depth, admitted) in [(16, true), (140, false)] {
        let source = format!(
            "export function run(a,b){{const d=Object.create(null);const key='x';return d[{}key{}]===undefined?a:b;}}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(result.is_ok(), admitted, "{:?}", result.as_ref().err());
    }
}

#[test]
fn dictionary_writes_create_replace_and_return_rhs_snapshots() {
    for body in [
        "const d=Object.create(null);const key=a?'left':'right';const assigned=d[key]=a;const saved=d[key];d[key]=b;return assigned===a&&saved===a&&d[key]===b?a:b;",
        "const d=Object.create(null);for(let i=0;i<2;i++){const key=i?'constructor':'__proto__';d[key]=i?b:a;}const keys=Object.keys(d);return d.__proto__===a&&d.constructor===b&&keys.length===2&&keys[0]==='__proto__'&&keys[1]==='constructor'?a:b;",
        "const d=Object.create(null);const key='__proto__';const child={x:a};d[key]=child;return d[key]===child&&d.x===undefined?a:b;",
        "const d=Object.create(null);let key='4294967295';d[key]=a;key='0';d[key]=b;const keys=Object.keys(d);return keys.length===2&&keys[0]==='0'&&keys[1]==='4294967295'?a:b;",
        r"const d=Object.create(null);let key='\ud800\0';d[key]=a;key='';d[key]=b;return d['\ud800\0']===a&&d['']===b?a:b;",
        "const d=Object.create(null);const key='self';d[key]=d;const saved=d[key];d[key]=null;return saved===d&&d[key]===null?a:b;",
        "const d=Object.create(null);const key='label';const label=d[key]='abc';const child=d[key]={x:a};return label.length===3&&child.x===a?a:b;",
        "const d=Object.create(null);const key='x';d[key]=a;const xs=['safe'];xs[4294967295]=b;return xs[0].length===4&&d[key]===a?a:b;",
    ] {
        compare(body);
    }
}

#[test]
fn dictionary_writes_snapshot_receiver_and_key_before_rhs_effects() {
    for body in [
        "let n=0;let d=Object.create(null);const old=d;let key='first';(n=n*10+1,d)[(n=n*10+2,key)]=(n=n*10+3,key='second',d=Object.create(null),b);return old.first===b&&d.first===undefined&&old.second===undefined&&n===123?a:b;",
        "const d=Object.create(null);let key='x';const rhs=()=>{key='y';return a;};const assigned=d[key]=rhs();return assigned===a&&d.x===a&&d.y===undefined&&key==='y'?a:b;",
        "const old=Object.create(null);const next=Object.create(null);let d=old;const key=()=>{d=next;return 'x';};d[key()]=a;return old.x===a&&next.x===undefined?a:b;",
        "const d=Object.create(null);d[(d.x=b,'x')]=(d.x=a,b);return d.x===b?a:b;",
    ] {
        compare(body);
    }
    let source = "function make(){return Object.create(null);}function set(d,k,v){return d[k]=v;}export function run(a,b){const d=make();const saved=set(d,'x',a);const write=k=>d[k]=b;write('y');return saved===a&&d.x===a&&d.y===b?a:b;}";
    common::compare_sources(source, source);
    common::compare_sources(
        "export function run(a,b){const d=Object.create(null);const key='x';d[key]=a;return d[key]===a?a:b;}",
        "export function run(a:string,b:object):void{const d:number=Object.create(null);const key:boolean='x';(d as never)[key as number]=a;return d[key]===a?a:b;}",
    );
}

#[test]
fn dictionary_writes_reject_unsupported_targets_keys_and_projections() {
    for body in [
        "const d={};const key='x';d[key]=a;return a;",
        "const d=a?Object.create(null):{};const key='x';d[key]=a;return a;",
        "const d=Object.create(null);d[a]=b;return a;",
        "const d=Object.create(null);const key=a?'x':null;d[key]=b;return a;",
        "const d=Object.create(null);const key=['x'].at(0);d[key]=b;return a;",
        "const d=Object.create(null);const key='x';d.x='old';const alias=d;alias[key]=a;return d.x.length;",
        "const d=Object.create(null);const key='x';d.x={};const alias=d;alias[key]=a;return d.x.y===undefined?a:b;",
        "const d=Object.create(null);const key='x';d.x=()=>a;const alias=d;alias[key]=b;return d.x();",
    ] {
        for typed in [false, true] {
            let source = format!("export function run(a,b){{{body}}}");
            let error = compile(&source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(error.contains("unsupported"), "{source}: {error}");
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
    for (depth, admitted) in [(16, true), (140, false)] {
        let source = format!(
            "export function run(a,b){{const d=Object.create(null);const key='x';d[{}key{}]=a;return d.x===a?a:b;}}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(result.is_ok(), admitted, "{:?}", result.as_ref().err());
    }
}

#[test]
fn source_object_aliases_preserve_identity() {
    compare("const o={};const alias=o;return o===alias?a:b;");
    compare("const o={};const fresh={};return o!==fresh?a:b;");
    compare("return {}==={}?a:b;");
    compare("const o={x:{}};return o.x!==a&&o.x!==null?a:b;");
    compare("const o={x:false,y:0,z:null};return o.x!==o.y&&o.z!==undefined?a:b;");
}

#[test]
fn object_keys_orders_indexes_before_insertion_order_strings() {
    compare(
        "const o={'2':a,'1':b,z:a,a:b};const keys=Object.keys(o);return keys.length===4&&keys[0]==='1'&&keys[1]==='2'&&keys[2]==='z'&&keys[3]==='a'?a:b;",
    );
}

#[test]
fn object_keys_preserves_index_boundaries_and_sparse_array_enumerability() {
    for body in [
        "const o={'4294967295':a,'01':a,'4294967294':a,'0':a,'2':a};o['01']=b;const k=Object.keys(o);return k.length===5&&k[0]==='0'&&k[1]==='2'&&k[2]==='4294967294'&&k[3]==='4294967295'&&k[4]==='01'?a:b;",
        "const xs=[a,,b];xs.named=a;const k=Object.keys(xs);return k.length===3&&k[0]==='0'&&k[1]==='2'&&k[2]==='named'?a:b;",
        "const xs=[a,b];xs.length=1;const k=Object.keys(xs);return k.length===1&&k[0]==='0'?a:b;",
        "const xs=[];xs[4294967294]=a;const k=Object.keys(xs);return k.length===1&&k[0]==='4294967294'?a:b;",
    ] {
        compare(body);
    }
}

#[test]
fn object_keys_handles_primitive_values_utf16_and_fresh_snapshots() {
    for body in [
        "const k=Object.keys('😀');return k.length===2&&k[0]==='0'&&k[1]==='1'?a:b;",
        "const k=Object.keys('0123456789x');return k.length===11&&k[10]==='10'?a:b;",
        "return Object.keys('').length===0&&Object.keys(a).length===0&&Object.keys(false).length===0&&Object.keys(true).length===0?a:b;",
        "const f=()=>a;return Object.keys(f).length===0&&Object.keys(typeof a).length===6?a:b;",
        r#"const o={'\ud800':a,'\ufffd':b};const k=Object.keys(o);return k[0]==='\ud800'&&k[1]==='\ufffd'?a:b;"#,
        "const o={x:a};const k=Object.keys(o);o.y=b;const later=Object.keys(o);k[0]='z';return k!==later&&k.length===1&&later.length===2&&later[0]==='x'&&later[1]==='y'&&o.x===a?a:b;",
    ] {
        compare(body);
    }
}

#[test]
fn object_keys_evaluates_call_arguments_before_taking_the_snapshot() {
    for body in [
        "let n=0;const o={x:a};const k=Object.keys((n++,o),(n++,o.y=b));return n===2&&k.length===2&&k[1]==='y'?a:b;",
        "let n=0;const o={x:a};const k=Object.keys.call((n++,null),(n++,o),(n++,o.y=b));return n===3&&k.length===2&&k[1]==='y'?a:b;",
        "const k=Object['keys']({x:a});return k.length===1&&k[0]==='x'?a:b;",
        "const Object={keys:x=>x};return Object.keys(a);",
    ] {
        compare(body);
    }
    common::compare_sources(
        "export function run(a,b){const o={x:a};return Object.keys(o)[0]==='x'?a:b;}",
        "type Object=never;export function run(a:string,b:boolean):void{const o:Missing={x:a};return ((Object as unknown).keys(o as never)!)[0]==='x'?a:b;}",
    );
    let source = "function keys(o){return Object.keys(o);}function count(xs){return xs.length;}export function run(a,b){const k=keys({x:a,y:b});return count(k)===2&&k[1]==='y'?a:b;}";
    common::compare_sources(source, source);
}

#[test]
fn object_keys_keeps_unimplemented_semantics_and_depth_explicit() {
    for body in [
        "Object.keys(null);return a;",
        "Object.keys();return a;",
        "const o=a?{}:null;Object.keys(o);return a;",
        "Object.keys({get x(){return a;}});return a;",
        "Object.keys(...[{}]);return a;",
        "Object?.keys({});return a;",
        "Object.keys(1n);return a;",
    ] {
        for ts in [false, true] {
            let error = compile(&format!("export function run(a,b){{{body}}}"), ts)
                .err()
                .unwrap_or_else(|| panic!("admitted {body}"));
            assert!(!error.contains("internal IR"), "{body}: {error}");
        }
    }
    for (depth, admitted) in [(16, true), (140, false)] {
        let source = format!(
            "export function run(a,b){{return Object.keys({}{{x:a}}{})[0]==='x'?a:b;}}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(result.is_ok(), admitted, "{:?}", result.as_ref().err());
    }
}

#[test]
fn source_object_spreads_copy_values_without_cloning_references() {
    compare(
        "const child={x:a};const source={child,x:a};const copy={...source};source.x=b;return copy!==source&&copy.child===child&&copy.x===a?a:b;",
    );
    compare("const o={...{x:a}};return a;");
}

#[test]
fn source_object_spreads_copy_string_utf16_index_properties() {
    compare(
        "const copy={...'0123456789x'};return copy['0']==='0'&&copy['9']==='9'&&copy['10']==='x'&&copy.length===undefined?a:b;",
    );
    compare(
        r#"const copy={...'😀'};return copy['0']==='\ud83d'&&copy['1']==='\ude00'&&copy['2']===undefined?a:b;"#,
    );
}

#[test]
fn source_object_spreads_preserve_order_overwrites_and_shallow_snapshots() {
    for body in [
        "let n=0;const source={x:a};const copy={x:(n++,b),...(n++,source),y:(n++,source.x=b),...(n++,{x:a})};return n===4&&copy.x===a&&copy.y===b&&source.x===b?a:b;",
        "const source={x:a};const copy={...source,...(source.x=b,source)};return copy.x===b?a:b;",
        "const child={};const source={child};const copy={...source};source.child={};return copy.child===child&&source.child!==child?a:b;",
        "const f=()=>a;const copy={...{f}};return copy.f===f?a:b;",
        "const source=a?{x:a}:{x:b};const make=x=>({...x});const copy=make(source);return copy!==source&&copy.x===source.x?a:b;",
        "const o={...{f:a},f:()=>b};return o.f();",
        "const source={f:b};const copy={f:()=>a,...source};return typeof copy.f==='number'?a:b;",
    ] {
        compare(body);
    }
}

#[test]
fn source_object_spreads_observe_array_enumerability_and_ignore_other_primitives() {
    for body in [
        "const xs=[a,,b];xs.named=a;const copy={...xs};return copy['0']===a&&copy['1']===undefined&&copy['2']===b&&copy.named===a&&copy.length===undefined?a:b;",
        "const xs=[a,b];xs.length=1;const copy={...xs};return copy['0']===a&&copy['1']===undefined?a:b;",
        "const copy={...null,...undefined,...false,...true,...a,...'',x:b};return copy.x===b&&copy.absent===undefined?a:b;",
        "const f=()=>a;const copy={...f,x:b};return copy.x===b&&copy.length===undefined?a:b;",
        "const source=a?'ab':null;const copy={...source};return a?(copy['1']==='b'?a:b):(copy['0']===undefined?a:b);",
        "let n=0;const copy={...(n++,null),...(n++,a)};return n===2&&copy.absent===undefined?a:b;",
    ] {
        compare(body);
    }
    let source = "function f(x){return x;}export function run(a,b){const copy={...f,x:a};return copy.x===a&&copy.name===undefined?a:b;}";
    common::compare_sources(source, source);
}

#[test]
fn source_object_spreads_preserve_annotations_limits_and_callable_field_guards() {
    compare_typed(
        "const source={x:a};const copy={...source};return copy.x===a?a:b;",
        "const source:number={x:a};const copy:never={...(source as Missing)};return copy.x===a?a:b;",
    );
    for body in [
        "const source={f:b};const copy={f:()=>a,...source};return copy.f();",
        "const copy={f:()=>a,...(a?{}:{f:b})};return copy.f();",
        "const copy={...{get x(){return a;}}};return a;",
        "const copy={...Promise.resolve(a)};return a;",
        "const copy={...1n};return a;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let error = compile(&source, ts)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
    for (depth, admitted) in [(16, true), (140, false)] {
        let source = format!(
            "export function run(a,b){{const source={{x:a}};const copy={{...{}source{}}};return copy.x===a?a:b;}}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        for ts in [false, true] {
            let source = source.clone();
            let result = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || compile(&source, ts))
                .unwrap()
                .join()
                .unwrap();
            assert_eq!(result.is_ok(), admitted, "{:?}", result.as_ref().err());
            if let Err(error) = result {
                assert!(error.contains("limit"), "{error}");
            }
        }
    }
}

#[test]
fn source_properties_preserve_values_and_saved_reads() {
    compare("const o={x:a};const old=o.x;o.x=b;return old===a&&o.x===b?a:b;");
}

#[test]
fn source_managed_truthiness_keeps_primitive_kinds_and_truthy_objects() {
    for value in ["a", "false", "true", "null", "undefined", "{}"] {
        compare(&format!("const o={{x:{value}}};return o.x?a:b;"));
    }
    compare("const o={};return o?a:b;");
}

#[test]
fn source_object_rebinding_preserves_saved_references() {
    compare("let o={x:a};const old=o;o={x:b};return old.x===a&&o.x===b?a:b;");
}

#[test]
fn source_managed_observations_preserve_primitive_export_kinds() {
    for ts in [false, true] {
        let equality = compile_value(
            "export function run(a,b){const o={x:a};return o.x===b;}",
            ts,
        )
        .unwrap();
        let mut equality = ValueInstance::new(&equality.wasm).unwrap();
        let nullish = compile_value(
            "export function run(a,b){const o={x:a};return o.x?null:undefined;}",
            ts,
        )
        .unwrap();
        let mut nullish = ValueInstance::new(&nullish.wasm).unwrap();
        let number =
            compile_value("export function run(a,b){const o={x:a};return o.x;}", ts).unwrap();
        let mut number = ValueInstance::new(&number.wasm).unwrap();
        for a in [0.0_f64, -0.0, 1.0, -7.0, f64::NAN, f64::INFINITY] {
            let Value::Number(actual) = number.run(a, 7.0).unwrap() else {
                panic!("projected Number changed its primitive export kind");
            };
            assert!(actual.to_bits() == a.to_bits() || (actual.is_nan() && a.is_nan()));
            assert!(matches!(equality.run(a,a).unwrap(),Value::Boolean(v) if v==!a.is_nan()));
            let v = nullish.run(a, 7.0).unwrap();
            assert!(if a != 0.0 && !a.is_nan() {
                matches!(v, Value::Null)
            } else {
                matches!(v, Value::Undefined)
            });
        }
    }
}

#[test]
fn source_object_initializers_and_assignments_keep_effect_order() {
    for body in [
        "let n=0;const o={x:(n++,a),y:(n++,b),x:(n++,b)};return n===3&&o.x===b&&o.y===b?a:b;",
        "const o={a,b};return o.a===a&&o.b===b?a:b;",
        "let o={x:a};const old=o;const out=(o.x=(o={x:b},b));return old.x===b&&o.x===b&&out===b?a:b;",
        "let n=0;let o={x:a};const old=o;(n++,o).x=(n++,o={x:b},a);return n===2&&old.x===a&&o.x===b?a:b;",
        "const o={x:a,y:b};o.x=o.y=a;return o.x===a&&o.y===a?a:b;",
        "const o={x:a};let v=o.x;v=b;return v===b&&o.x===a?a:b;",
        "let n=a;const o={x:n=b};return o.x===b&&n===b?a:b;",
        "const f=(x,y)=>x+y;const o={x:f.length};return o.x===2?a:b;",
        "let i=0;const o={x:a};while(i<2){o.x=i++;}return o.x===1?a:b;",
        "const o={};for(let i=0;i<2;i++){o.x=i?b:a;}return o.x===b?a:b;",
        "let n=0;const f=()=>++n;const o={x:f(),y:f()};return o.x===1&&o.y===2?a:b;",
    ] {
        compare(body);
    }
}

#[test]
fn source_objects_keep_selected_branches_and_nested_reference_values() {
    for body in [
        "const o=a?{x:a}:{x:b};return o.x===a?a:b;",
        "let n=0;const o=a?(n++,{x:a}):(n+=2,{x:b});return (a?n===1:n===2)?a:b;",
        "const child={x:a};const o={child,other:{x:b}};return o.child===child&&o.other!==child?a:b;",
        "const v=a?{}:false;return v?a:b;",
        "const o={x:a?{}:b};return o.x?a:b;",
    ] {
        compare(body);
    }
}

#[test]
fn source_property_names_preserve_utf16_and_absent_values() {
    for body in [
        r#"const o={"\ud800\0":a,"\ufffd\0":b};return o["\ud800\0"]===a&&o["\ufffd\0"]===b?a:b;"#,
        r#"const o={"":a,"\0":b};return o[""]===a&&o["\0"]===b?a:b;"#,
        "const o={['x']:a,length:b};o['x']=b;return o.x===b&&o.length===b?a:b;",
        "const o={x:a};return o.absent===undefined&&o.x!==null?a:b;",
    ] {
        compare(body);
    }
}

#[test]
fn numeric_literal_keys_use_ecmascript_names_in_records_and_dictionaries() {
    for body in [
        "const o={1:a};return a;",
        "let n=0;const o={1:(n++,a),[+1]:(n++,b),[-0]:a,'01':b};const keys=Object.keys(o);return o[1]===b&&o[0]===a&&o['01']===b&&n===2&&keys.length===3&&keys[0]==='0'&&keys[1]==='1'&&keys[2]==='01'?1:0;",
        "const o={0x10:a,1_000:b,1e-7:a,0.000001:b,1e20:a,1e21:b,9007199254740993:a,[1e999]:b};return o['16']===a&&o['1000']===b&&o['1e-7']===a&&o['0.000001']===b&&o['100000000000000000000']===a&&o['1e+21']===b&&o['9007199254740992']===a&&o.Infinity===b?1:0;",
        "const d=Object.create(null);d[-1.5]=a;d[2]=b;const keys=Object.keys(d);return d['-1.5']===a&&d[2]===b&&keys[0]==='2'&&keys[1]==='-1.5'?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn source_object_annotations_do_not_change_runtime_identity() {
    compare_typed(
        "const o={x:a};const alias=o;return o===alias?a:b;",
        "const o:number={x:a};const alias:boolean=o;return (o as unknown)===(alias!)?a:b;",
    );
}

#[test]
fn source_managed_boundaries_explicitly_reject_unimplemented_semantics() {
    for body in [
        "const o={};return o[a]?a:b;",
        "const o={get x(){return a;}};return o.x?a:b;",
        "const o={x(){return a;}};return a;",
        "const o={x:Promise.resolve(a)};return o.x===a?a:b;",
        "const p=Promise.resolve(a);const o={x:p};return o.x===a?a:b;",
        "const o={x:1n};return a;",
        "const o=true?{}:{get x(){return a;}};return a;",
        "const o={};return o;",
        "let o={};o=a;return a;",
        "const o={};o={};return a;",
        "const o={};Object.setPrototypeOf(o,{});return a;",
    ] {
        for ts in [false, true] {
            assert!(
                compile(&format!("export function run(a,b){{{body}}}"), ts).is_err(),
                "{body}"
            );
        }
    }
}

#[test]
fn source_proto_guards_cover_actual_native_intrinsic_names() {
    let output = std::process::Command::new("node")
        .args([
            "-e",
            "console.log(Object.getOwnPropertyNames(Object.prototype).join('\\n'))",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let names = String::from_utf8(output.stdout).unwrap();
    assert!(names.lines().any(|n| n == "__proto__"));
    for name in names.lines() {
        for body in [
            format!("const o={{}};return o[{name:?}]?a:b;"),
            format!("const o={{{name:?}:a}};return a;"),
            format!("const o={{}};o[{name:?}]=a;return a;"),
        ] {
            for ts in [false, true] {
                let error = compile(&format!("export function run(a,b){{{body}}}"), ts)
                    .err()
                    .expect("intrinsic prototype property must reject");
                assert!(error.contains("prototype"), "{body}: {error}");
            }
        }
    }
}

#[test]
fn source_object_depth_checks_run_on_a_small_native_stack() {
    for (depth, admitted) in [(16, true), (140, false)] {
        let source = format!(
            "export function run(a,b){{return ({0}a{1})?a:b;}}",
            "{x:".repeat(depth),
            "}".repeat(depth)
        );
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(result.is_ok(), admitted);
        if let Err(error) = result {
            assert!(error.contains("nesting"), "{error}");
        }
    }
}

fn compare(body: &str) {
    compare_typed(body, body);
}

fn compare_typed(body: &str, typed: &str) {
    let js = format!("export function run(a,b){{{body}}}");
    let ts = format!("export function run(a,b){{{typed}}}");
    common::compare_sources(&js, &ts);
}

#[test]
fn has_own_tests_presence_independently_of_the_stored_value() {
    for body in [
        "const o={enabled:undefined,zero:0};return Object.hasOwn(o,'enabled')&&Object.hasOwn(o,'zero')&&!Object.hasOwn(o,'missing')&&!Object.hasOwn(o,'constructor')?1:0;",
        "const d=Object.create(null);d.__proto__=undefined;d.constructor=null;return Object.hasOwn(d,'__proto__')&&Object.hasOwn(d,'constructor')&&!Object.hasOwn(d,'toString')?1:0;",
        "const x=[,undefined];x.named=false;const before=Object.hasOwn(x,'1');x.length=1;return before&&Object.hasOwn(x,'length')&&Object.hasOwn(x,'named')&&!Object.hasOwn(x,'0')&&!Object.hasOwn(x,'1')&&!Object.hasOwn(x,'pop')?1:0;",
        "const x=[a,,];const [...dense]=x;return !Object.hasOwn(x,'1')&&Object.hasOwn(dense,'1')&&Object.hasOwn([], 'length')&&!Object.hasOwn({},'length')?1:0;",
        r"const d=Object.create(null);const key='\ud800\0';d[key]=a;return Object['hasOwn'](d,key)&&!Object.hasOwn(d,'\ufffd\0')?1:0;",
        "const Object={hasOwn:(x,y)=>x+y};return Object.hasOwn(a,b);",
    ] {
        compare(body);
    }
}

#[test]
fn has_own_saves_operands_but_tests_presence_after_all_argument_effects() {
    for body in [
        "let n=0,key='x',o={};const old=o;const found=Object.hasOwn((n=n*10+1,o),(n=n*10+2,key),(n=n*10+3,old.x=undefined,key='y',o={}));return found&&n===123&&!Object.hasOwn(o,'x')&&key==='y'?1:0;",
        "let n=0;const x=[a];const found=Object.hasOwn.call((n=1,null),(n=n*10+2,x),(n=n*10+3,'0'),(n=n*10+4,x.length=0));return !found&&n===1234?1:0;",
        "const o={yes:true};let flag=Object.hasOwn(o,'yes');const box={flag};flag=Object.hasOwn(o,'no');return box.flag===true&&flag===false&&box.flag!==1?1:0;",
        "let n=0;const d={};Object.hasOwn(d,'x',n++);return n;",
    ] {
        compare(body);
    }
    let source = "function check(items,key){return Object.hasOwn(items,key);}export function run(a,b){const x={value:undefined};return check(x,'value')&&!check(x,'absent')?1:0;}";
    common::compare_sources(source, source);
}

#[test]
fn has_own_rejects_unproven_receivers_keys_and_spread_calls() {
    for body in [
        "return Object.hasOwn(null,'x')?1:0;",
        "return Object.hasOwn(a?{}:null,'x')?1:0;",
        "return Object.hasOwn({x:a},b)?1:0;",
        "return Object.hasOwn({},...['x'])?1:0;",
        "return Object.hasOwn({},'x',...[])?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let error = compile(&source, ts)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(
                error.contains("unsupported") && !error.contains("internal IR"),
                "{source}: {error}"
            );
        }
    }
}

#[test]
fn has_own_uses_the_same_numeric_names_as_property_reads_and_writes() {
    for body in [
        "const x=[,a];x[-1]=undefined;x[1.5]=b;return !Object.hasOwn(x,-0)&&Object.hasOwn(x,+1)&&Object.hasOwn(x,-1)&&Object.hasOwn(x,1.5)&&!Object.hasOwn(x,'-0')?1:0;",
        "const o={1e21:a,'01':b,0:a,[1e999]:b};return Object.hasOwn(o,1e21)&&Object.hasOwn(o,-0)&&Object.hasOwn(o,1e999)&&Object.hasOwn(o,'01')&&!Object.hasOwn(o,1)?1:0;",
        "const d=Object.create(null);d[4294967295]=a;return Object.hasOwn(d,0xffffffff)&&!Object.hasOwn(d,4294967294)?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn has_own_uses_primitive_and_missing_keys_without_losing_effects() {
    for body in [
        "const o={true:undefined,false:b,null:a,undefined:b};return Object.hasOwn(o,true)&&Object.hasOwn(o,false)&&Object.hasOwn(o,null)&&Object.hasOwn(o,undefined)&&Object.hasOwn(o)?1:0;",
        "let n=0;const o={};const has=Object.hasOwn(o,void (n=1), (n=n*10+2,o.undefined=a));return has&&n===12?1:0;",
        "const undefined='real';const o={real:a};return Object.hasOwn(o,undefined)&&!Object.hasOwn(o)?1:0;",
        "const o={undefined:a};return Object.hasOwn.call(b,o)&&!Object.hasOwn.call(a,o,null)?1:0;",
    ] {
        compare(body);
    }
    let js = "export function run(a,b){const o={1:a};return Object.hasOwn(o,-0)?a:Object.hasOwn(o,1)?b:a;}";
    let ts = "export function run(a:number,b:number){const o={1:a};return Object.hasOwn(o,(-0 as string))?a:Object.hasOwn(o,(1 as boolean))?b:a;}";
    common::compare_sources(js, ts);
}

#[test]
fn has_own_returns_a_boolean_through_the_primitive_export() {
    for ts in [false, true] {
        let source =
            "export function run(a,b){const o={undefined:a};return Object.hasOwn(o,void b);}";
        let artifact = compile_value(source, ts).unwrap();
        let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
        assert!(matches!(
            instance.run(4.0, 2.0).unwrap(),
            Value::Boolean(true)
        ));
    }
}

#[test]
fn has_own_known_scalar_receivers_still_evaluate_keys_and_extra_arguments() {
    for body in [
        "return !Object.hasOwn(0,'constructor')&&!Object.hasOwn(a,'toString')&&!Object.hasOwn(true,'valueOf')&&!Object.hasOwn(false,0)?1:0;",
        "let n=0;const key='x';const found=Object.hasOwn(++n,key,n=n*10+2);return !found&&n===12?1:0;",
        "let n=0;const found=Object.hasOwn((++n)>0,void(n=n*10+2),n=n*10+3);return !found&&n===123?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn dictionary_in_checks_presence_for_static_and_runtime_keys() {
    for body in [
        "const d=Object.create(null);d.x=undefined;const k='x';const flag=k in d;const box={flag};return box.flag===true&&flag!==1&&!('missing' in d)&&!('constructor' in d)?1:0;",
        "const d=Object.create(null);d.__proto__=null;d[1.5]=b;d.undefined=a;return '__proto__' in d&&1.5 in d&&undefined in d&&!(false in d)?1:0;",
        "const d=Object.create(null);const has=k=>k in d;d.y=a;return has('y')&&!has('x')?1:0;",
        r"const d=Object.create(null);d['\ud800']=a;return '\ud800' in d&&!('\ufffd' in d)?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn dictionary_in_saves_the_key_before_evaluating_the_receiver() {
    for body in [
        "const d=Object.create(null);let n=0,k='first';const present=(n=n*10+1,k) in (n=n*10+2,k='other',d.first=undefined,d);return present&&n===12&&!('other' in d)?1:0;",
        "const d=Object.create(null);let n=0;const present=void(n=1) in (n=n*10+2,d.undefined=a,d);return present&&n===12?1:0;",
        "const d=Object.create(null);let n=0;(n++,'unused') in (n++,d);return n;",
    ] {
        compare(body);
    }
}

#[test]
fn dictionary_in_rejects_unproven_prototypes_and_key_coercions() {
    for body in [
        "return 'x' in null?1:0;",
        "return a in Object.create(null)?1:0;",
        "return {} in Object.create(null)?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let err = compile(&source, ts)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(
                err.contains("unsupported") && !err.contains("internal IR"),
                "{source}: {err}"
            );
        }
    }
}

#[test]
fn static_in_distinguishes_own_fields_and_intrinsic_prototypes() {
    for body in [
        "return 'length' in [a]?1:0;",
        "return 'x' in (a?Object.create(null):{})?1:0;",
        "const o={own:undefined};return 'own' in o&&'constructor' in o&&'__proto__' in o&&!('map' in o)&&!('missing' in o)?1:0;",
        "const items=[,undefined];return 'map' in items&&'toString' in items&&'length' in items&&!(0 in items)&&1 in items?1:0;",
        "const d=Object.create(null);return !('toString' in d)&&'toString' in {}&&'toString' in []?1:0;",
        "let n=0;const yes='toString' in (n++,{});const also='map' in (n=n*10+2,[]);return yes&&also&&n===12?1:0;",
        "let n=0;const result=void(n=1) in (n=n*10+2,{undefined:0});return result&&n===12?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn static_in_agrees_with_native_intrinsic_string_names() {
    let output=std::process::Command::new("node").args(["-e","for(const obj of [Object.prototype,Array.prototype]) console.log(Object.getOwnPropertyNames(obj).join(','))"]).output().unwrap();
    assert!(output.status.success());
    let names = String::from_utf8(output.stdout).unwrap();
    for (receiver, keys) in ["{}", "[]"].iter().zip(names.lines()) {
        let clauses = keys
            .split(',')
            .map(|key| format!("({key:?} in obj)"))
            .collect::<Vec<_>>()
            .join("&&");
        compare(&format!("const obj={receiver};return {clauses}?1:0;"));
    }
}

#[test]
fn dynamic_in_uses_the_receivers_own_table_and_real_prototype_names() {
    for body in [
        "const k='constructor';return k in {}?1:0;",
        "const key=a?'map':'extra';const obj={extra:undefined};const arr=[,undefined];return ((key in obj)?100:0)+((key in arr)?10:0)+((''+(a?'1':'0')) in arr?1:0);",
        "const obj={owned:undefined};const key=a?'toString':'owned';return key in obj&&(Object.hasOwn(obj,key)===(key==='owned'))?1:0;",
        "let key='map',n=0;const arr=[];const found=(n=1,key) in (key='missing',n=n*10+2,arr);return found&&n===12&&!(key in arr)?1:0;",
        r"const key=a?'toStrin\0':'toStrin\ud800';return !(key in {})&&!(key in [])?1:0;",
        "const k='map';const record={};const list=[];const a1=()=>k in record;const a2=()=>k in list;return !a1()&&a2()?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn dynamic_in_matches_all_native_prototype_names_and_near_misses() {
    let out=std::process::Command::new("node").args(["-e","const o=Object.getOwnPropertyNames(Object.prototype);console.log(o.join(','));console.log([...new Set([...o,...Object.getOwnPropertyNames(Array.prototype)])].join(','));"]).output().unwrap();
    assert!(out.status.success());
    let lines = String::from_utf8(out.stdout).unwrap();
    for (receiver, keys) in ["{}", "[]"].iter().zip(lines.lines()) {
        let mut source = format!(
            "function check(key){{const receiver={receiver};return key in receiver;}}export function run(a,b){{"
        );
        for key in keys.split(',') {
            let mut near = key.to_owned();
            near.pop();
            near.push('!');
            source.push_str(&format!("if(!check({key:?})||check({near:?}))return 0;"));
        }
        source.push_str("return 1;}");
        common::compare_sources_with_fuel(&source, &source, Some(3_000_000));
    }
}

#[test]
fn computed_record_keys_are_saved_before_values_and_keep_definition_order() {
    for body in [
        "let k='first',n=0;const obj={[(n=n*10+1,k)]:(n=n*10+2,k='second',a),[(n=n*10+3,k)]:(n=n*10+4,b)};const keys=Object.keys(obj);return n===1234&&keys[0]==='first'&&keys[1]==='second'&&obj.first===a&&obj.second===b?1:0;",
        "const key=a?'shown':'hidden';const row={[key]:undefined,tail:b};return Object.hasOwn(row,key)&&key in row&&Object.keys(row).length===2&&row.tail===b?1:0;",
        "const k='x';const row={before:a,x:0,[k]:1,...{middle:b},[k]:2,end:0};const keys=Object.keys(row);return row.x===2&&keys[0]==='before'&&keys[1]==='x'&&keys[2]==='middle'&&keys[3]==='end'?1:0;",
        "const k='2';const row={z:a,[k]:b,['1']:a};const keys=Object.keys(row);return keys[0]==='1'&&keys[1]==='2'&&keys[2]==='z'&&row[2]===b?1:0;",
        "const key='x';const obj={[key]:a,label:' confirmed '};return obj.label.trim()==='confirmed'&&obj.x===a?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn computed_proto_names_create_own_data_fields() {
    for body in [
        "const key='__proto__';const proto={marker:a};const obj={[key]:proto};const keys=Object.keys(obj);return Object.hasOwn(obj,key)&&!('marker' in obj)&&keys.length===1&&keys[0]===key?1:0;",
        "const obj={['__proto__']:null,['constructor']:undefined};const {...rest}=obj;return Object.hasOwn(rest,'__proto__')&&Object.hasOwn(rest,'constructor')&&Object.keys(rest).length===2?1:0;",
        r"const key='\ud800\0';const obj={[key]:a,['\ufffd\0']:b};return obj['\ud800\0']===a&&obj['\ufffd\0']===b?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn computed_literal_keys_reject_unknown_coercions_and_invalidated_field_kinds() {
    for body in [
        "const key=a?'label':b;const obj={[key]:a};return a;",
        "const key={};const obj={[key]:a};return a;",
        "const key='label';const obj={label:'text',[key]:a};return obj.label.trim().length;",
        "const key='fn';const obj={fn:x=>x+1,[key]:a};return obj.fn(b);",
        "const obj={__proto__:{marker:a}};return a;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let err = compile(&source, ts)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(
                err.contains("unsupported") && !err.contains("internal IR"),
                "{source}: {err}"
            );
        }
    }
}

#[test]
fn object_values_snapshots_enumerable_data_in_own_key_order() {
    for body in [
        "const child={x:a};const o={z:child,10:b,2:a,empty:undefined};o.z=child;const vs=Object.values(o);o.z=null;vs[0]=b;return vs.length===4&&o[2]===a&&vs[1]===b&&vs[2]===child&&vs[3]===undefined?1:0;",
        "const arr=[,a,,undefined];arr.named=b;const vs=Object.values(arr);return vs.length===3&&vs[0]===a&&Object.hasOwn(vs,1)&&vs[1]===undefined&&vs[2]===b?1:0;",
        "const d=Object.create(null);d.__proto__=a;d.constructor=undefined;const vs=Object['values'](d);return vs.length===2&&vs[0]===a&&vs[1]===undefined?1:0;",
        r"const vs=Object.values('A\u{1f600}\ud800');return vs.length===4&&vs[0]==='A'&&vs[1]==='\ud83d'&&vs[2]==='\ude00'&&vs[3]==='\ud800'?1:0;",
        "const a1=Object.values(a),a2=Object.values(a);return a1!==a2&&a1.length===0&&Object.values(true).length===0?1:0;",
        "const Object={values:(x)=>x};return Object.values(a);",
    ] {
        compare(body);
    }
    let source = "function chars(s){return Object.values(s);}export function run(a,b){const chars1=chars('ab');return chars1.length===2&&chars1[0]==='a'&&chars1[1]==='b'?1:0;}";
    common::compare_sources(source, source);
}

#[test]
fn object_values_observes_saved_receivers_after_all_argument_effects() {
    for body in [
        "let o={x:a};const original=o;let n=0;const vs=Object.values.call((n=1,null),(n=n*10+2,o),(n=n*10+3,original.x=b,o={y:a}));return n===123&&vs.length===1&&vs[0]===b?1:0;",
        "const k='enabled';const o={ [k]:undefined, reload:a };const [flag,value]=Object.values(o);return flag===undefined&&value===a?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn object_entries_yields_fresh_dense_pairs_with_ordered_keys_and_shared_values() {
    for body in [
        "const child={x:a};const o={z:child,10:b,2:a,empty:undefined};const rows=Object.entries(o);o.z=null;const order=Object.keys(o);let i=0;for(const row of rows){const [k,v]=row;if(k!==order.at(i)||row.length!==2)return -1;if(k==='z'&&v!==child)return -2;if(k==='empty'&&v!==undefined)return -3;i++;}return i;",
        "const src=[,a,,undefined];src.named=b;const rows=Object.entries(src);let n=0;for(const [key,value] of rows){if(key==='1'&&value===a)n+=1;if(key==='3'&&value===undefined)n+=10;if(key==='named'&&value===b)n+=100;}return rows.length===3?n:-1;",
        "const d=Object.create(null);d.constructor=a;d.__proto__=b;let n=0;for(const [key,value] of Object['entries'](d))n+=key.trim().length+(value===a?1:0);return n;",
        "const o={x:a,y:b};const first=Object.entries(o);const next=Object.entries(o);return first!==next&&first[0]!==first[1]&&first[0]!==next[0]?1:0;",
        "const child={value:a};const o={x:child};const rows=Object.entries(o);let ok=0;for(const pair of rows){if(pair[0].trim()==='x'&&pair[1]===child)ok=1;pair[1]=null;}return ok&&o.x===child?1:0;",
        "const Object={entries:x=>x};return Object.entries(a);",
    ] {
        compare(body);
    }
}

#[test]
fn object_entries_exposes_each_utf16_unit_and_observes_surplus_argument_effects() {
    for body in [
        r"let n=0;for(const [k,v] of Object.entries('A\u{1f600}\ud800')){if(k==='0'&&v==='A')n+=1;if(k==='1'&&v==='\ud83d')n+=10;if(k==='2'&&v==='\ude00')n+=100;if(k==='3'&&v==='\ud800')n+=1000;}return n;",
        "let o={x:a};const saved=o;let n=0;const rows=Object.entries.call((n=1,null),(n=n*10+2,o),(n=n*10+3,saved.x=b,o={}));let ok=0;for(const [key,value] of rows)if(key==='x'&&value===b)ok++;return n===123&&ok===1?1:0;",
        "return Object.entries(a).length+Object.entries(false).length;",
    ] {
        compare(body);
    }
}

#[test]
fn object_entries_rejects_unsound_pair_or_key_projections_after_mutation() {
    for body in [
        "const rows=Object.entries({x:a});rows[0]=a;for(const [key,value] of rows)return key.trim().length;return 0;",
        "const rows=Object.entries({x:a});for(const pair of rows)pair[0]=b;for(const [key] of rows)return key.trim().length;return 0;",
        "const rows=Object.entries({x:a});for(const pair of rows)pair.length=0;for(const [key] of rows)return key.trim().length;return 0;",
        "for(const [key,value] of Object.entries({x:a}))return value.trim().length;return 0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let err = compile(&source, ts)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(
                err.contains("unsupported") && !err.contains("internal IR"),
                "{source}: {err}"
            );
        }
    }
}

#[test]
fn homogeneous_enumerations_support_typed_values_and_destructured_records() {
    for body in [
        "const key=a?'first':'second';const src={fixed:' yes ',[key]:' no '};let n=0;for(const v of Object.values(src))n+=v.trim().length;return n;",
        "const src=Object.create(null);src[a?'one':'two']=' x ';let n=0;for(const [k,v] of Object.entries(src))n+=k.length+v.trim().length;return n;",
        "const src=[,' x '];src.named=' yy ';let n=0;for(const v of Object.values(src))n+=v.trim().length;return n;",
        "const src={one:{label:' first '},two:{label:' second '}};let n=0;for(const {label} of Object.values(src))n+=label.trim().length;return n;",
        "const record={one:{label:' x '}};let n=0;for(const [key,{label}] of Object.entries(record))n+=key.length+label.trim().length;return n;",
        "const src=a?{x:'one'}:{};const [v=' fallback ']=Object.values(src);return v.trim().length;",
        "let n=0;for(const row of Object.entries('ab'))for(const item of row)n+=item.trim().length;return n;",
    ] {
        compare(body);
    }
}

#[test]
fn enumerated_values_refuse_unknown_or_replaced_elements() {
    for body in [
        "const src={x:'x'};const alias=src;alias.y=b;for(const v of Object.values(src))return v.trim().length;return 0;",
        "const src={x:'x',...{y:b}};for(const [k,v] of Object.entries(src))return v.trim().length;return 0;",
        "const list=Object.values({x:'x'});return list[0].trim().length;",
        "const list=Object.values({x:'x'});list.length=2;for(const v of list)return v.trim().length;return 0;",
        "const rows=Object.entries({x:'x'});for(const r of rows)r[1]=b;for(const [k,v] of rows)return v.trim().length;return 0;",
        "const src={label:'x'};const record={x:src};src.label=b;for(const [k,v] of Object.entries(record))return v.label.trim().length;return 0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let error = compile(&source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(
                error.contains("unsupported") && !error.contains("internal IR"),
                "{source}: {error}"
            );
        }
    }
}
#[test]
fn runtime_dictionary_primitive_reads_keep_missing_tags_and_operand_snapshots() {
    for body in [
        "const d=Object.create(null);d.x=a;const k='x';return d[k]+b;",
        "const d=Object.create(null);d.x=a;const k=a?'x':'missing';return +d[k];",
        "const d=Object.create(null);d.x=false;d.y=null;const k=a?'x':'y';return d[k]==0?1:0;",
        "const d=Object.create(null);const k='missing';return +d[k];",
        "const d=Object.create(null);const k='x';d[k]=a?true:b;return +d[k];",
        "const d=Object.create(null);d.x=a;const k='x';delete d[k];return +d[k];",
        "const d=Object.create(null);d.x=a;const k='x';const saved=d[k];d.x=b;return saved+d[k];",
        "let d=Object.create(null);d.x=a;const other=Object.create(null);other.x=b;const k='x';return d[(d=other,k)]+d[k];",
        "const d=Object.create(null);d.x=a;const key=()=>{d.x=b;return 'x';};return +d[key()];",
        "const d=Object.create(null);d.x=a;const get=(o,k)=>o[k];return +get(d,'x');",
        "const d=Object.create(null);d.x=a;const k='x';const read=()=>d[k];return +read();",
        "const d=Object.create(null);d['__proto__']=a;const k='__proto__';return +d[k];",
        "const d=Object.create(null);const k='x';const value=d[k]??=a;return +value+ +d[k];",
        "const d=Object.create(null);d.x=false;const k='x';const value=d[k]||=b;return +value+ +d[k];",
        "const d=Object.create(null);const k='x';const value=d[k]&&=b;return +value;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
    let js =
        "export function run(a,b){const d=Object.create(null);d.x=a;const k='x';return +d[k];}";
    let ts = "export function run(a:number,b:number){const d:number=Object.create(null);d.x=a;const k:boolean='x';return +((d as string)[k] as boolean);}";
    common::compare_sources(js, ts);
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm
    );
}
