use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{Value, ValueInstance};

#[path = "managed/common.rs"]
mod common;

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
        "const d=Object.create(null);const key='x';d[key]+=a;return a;",
        "const d=Object.create(null);const key='x';d[key]??=a;return a;",
        "const d=Object.create(null);const key='x';d[key]++;return a;",
        "const d=Object.create(null);const key='x';delete d[key];return a;",
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
        for a in [0.0_f64, -0.0, 1.0, -7.0, f64::NAN, f64::INFINITY] {
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
        "const o={1:a};return a;",
        "const o={x:Promise.resolve(a)};return o.x===a?a:b;",
        "const p=Promise.resolve(a);const o={x:p};return o.x===a?a:b;",
        "const o={x:1n};return a;",
        "const o=true?{}:{get x(){return a;}};return a;",
        "const o={x:a};return o.x;",
        "const o={};return o;",
        "let o={};o=a;return a;",
        "const o={};o={};return a;",
        "const o={};return o==null?a:b;",
        "const o={};Object.setPrototypeOf(o,{});return a;",
    ] {
        for ts in [false, true] {
            assert!(
                compile(&format!("export function run(a,b){{{body}}}"), ts).is_err(),
                "{body}"
            );
        }
    }
    for ts in [false, true] {
        assert!(compile("function get(o){return o.x;}export function run(a,b){const o={x:a};return get(o);}",ts).is_err());
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
