use grunkalunka_compiler::compile;

#[path = "managed/common.rs"]
mod common;

#[test]
fn object_bindings_select_shorthand_renamed_and_literal_keys() {
    for body in [
        "const {x,y:renamed}={x:a,y:b};let {x:other}={x:b};return x===a&&renamed===b&&other===b?1:0;",
        "const {'left-label':left,['right-label']:right,'':empty,'1':one}={'left-label':a,'right-label':b,'':a,'1':b};return left===a&&right===b&&empty===a&&one===b?1:0;",
        r"const {'\ud800\0':surrogate,'\ufffd\0':replacement,['\0']:nul}={'\ud800\0':a,'\ufffd\0':b,'\0':a};return surrogate===a&&replacement===b&&nul===a?1:0;",
        "const {x,missing}={x:a};return x===a&&missing===undefined?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn nested_object_bindings_select_fields_and_keep_proven_leaf_operations() {
    for body in [
        // Preserve the original nested-pattern rejection source as an admission witness.
        "const {child:{x}}={child:{x:a}};return a;",
        "const child={tag:b};const {row:{'display-name':label,inner:{value,ref}},empty:{}}={row:{'display-name':' Ready ',inner:{value:a,ref:child}},empty:{}};return label.trim()==='Ready'&&value===a&&ref===child&&ref.tag===b?1:0;",
    ] {
        compare(body);
    }
    let source = "function identity(value){return value;}export function run(a,b){const record={row:{label:' x ',items:[a,b]}};const alias=identity(record);const {row:{label,items}}=alias;return label.trim()==='x'&&items.length===2&&items.at(-1)===b?1:0;}";
    common::compare_sources(source, source);
}

#[test]
fn nested_object_bindings_finish_each_child_before_reading_the_next_outer_field() {
    compare(
        "let n=0;const root={inner:{last:a}};const make=()=>{n=n*10+1;return root;};const {inner:{first=(n=n*10+2,root.inner={last:b},a),last},inner:{last:later},tail=(n=n*10+3,b)}=make();return n+(first===a?1000:0)+(last===a?10000:0)+(later===b?100000:0)+(tail===b?1000000:0);",
    );
}

#[test]
fn nested_object_bindings_retain_each_original_receiver_during_defaults() {
    compare(
        "const child={second:a};let root={child,tail:a};const original=root;const {child:{first=(root.child={second:b},root={child:{second:9},tail:b},child.second=b,11),second},tail}=root;return first===11&&second===b&&tail===a&&original.child!==child&&root!==original?1:0;",
    );
}

#[test]
fn nested_object_bindings_keep_mutable_and_per_iteration_capture_cells() {
    for body in [
        "let {child:{x=a}}={child:{}};const read=()=>x===b?1:0;x=b;return read();",
        "let first=()=>0;let second=()=>0;for(let i=0;i<2;i++){let {child:{x=i}}={child:{}};const read=()=>x===0?1:2;if(i===0)first=read;else second=read;}return first()*10+second();",
        "let first=()=>0;let last=()=>0;for(let {child:{x=1}}={child:{}},i=0;i<2;i++,x=2){if(i===0)first=()=>x===1?1:10;last=()=>x===2?1:10;}return first()+last();",
    ] {
        compare(body);
    }
}

#[test]
fn nested_object_bindings_check_null_prototype_provenance_at_each_receiver() {
    compare(
        "const dictionary=Object.create(null);dictionary.constructor=a;dictionary.__proto__=b;const {entry:{constructor:value,__proto__:raw,toString:missing}}={entry:dictionary};return value===a&&raw===b&&missing===undefined?1:0;",
    );
}

#[test]
fn nested_object_binding_annotations_are_erased_without_proving_receivers() {
    let js = "export function run(a,b){let {child:{value,label}}={child:{value:a,label:' ab '}};value=b;return value===b&&label.trim()==='ab'?1:0;}";
    let ts = "type Shape=never;export function run(a:string,b:boolean):void{let {child:{value,label}}:Shape=({child:{value:a,label:' ab '}} as unknown);value=b;return (value as number)===b&&(label as number).trim()==='ab'?1:0;}";
    common::compare_sources(js, ts);
    assert_eq!(
        compile(js, false).unwrap().wasm,
        compile(ts, true).unwrap().wasm
    );
    let source = "export function run(a,b){const source:{child:{value:number}}={child:a};const {child:{value}}=source;return value===a?1:0;}";
    let error = compile(source, true)
        .err()
        .unwrap_or_else(|| panic!("admitted {source}"));
    assert!(error.contains("unsupported"), "{source}: {error}");
    assert!(!error.contains("internal IR"), "{source}: {error}");
}

#[test]
fn nested_object_bindings_preserve_empty_receiver_tdz_and_alias_guards() {
    for body in [
        "const {child:{}}={};return a;",
        "const {child:{}}={child:null};return a;",
        "const {child:{}}={child:[]};return a;",
        "const {child:{x}}={child:a?{x:a}:null};return a;",
        "const root={child:{x:a}};const alias=root;alias.child=null;const {child:{x}}=root;return a;",
        "const root={child:{text:'ready'}};const alias=root.child;alias.text=a;const {child:{text}}=root;return text.length;",
        "const {child:{constructor}}={child:{}};return a;",
        "const dictionary=Object.create(null);dictionary.child={};const {child:{constructor}}=dictionary;return a;",
        // Dictionary writes retain a possible missing value in the existing origin proof.
        "const dictionary=Object.create(null);dictionary.constructor={value:a};dictionary.__proto__=b;const {entry:{constructor:{value},__proto__:raw,toString:missing}}={entry:dictionary};return value===a&&raw===b&&missing===undefined?1:0;",
        "const {child:{x=x}}={child:{}};return a;",
        "const {child:{x=later},later=a}={child:{}};return a;",
        "const {first=later,child:{later=a}}={child:{}};return a;",
        "const {child:{f=()=>x,x=a}}={child:{}};return a;",
        "const {child:{x,...rest}}={child:{x:a}};return a;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let error = compile(&source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(error.contains("unsupported"), "{source}: {error}");
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

#[test]
fn nested_object_binding_budgets_count_leaves_and_bound_source_depth() {
    for count in [128, 129] {
        let fields = (0..count)
            .map(|index| format!("value:v{index}"))
            .collect::<Vec<_>>()
            .join(",");
        let source = format!(
            "export function run(a,b){{const {{empty:{{}},child:{{{fields}}}}}={{empty:{{}},child:{{value:a}}}};return v0===a&&v127===a?1:0;}}"
        );
        for typed in [false, true] {
            let result = compile_nested_on_small_stack(source.clone(), typed);
            if count == 128 {
                assert!(result.is_ok(), "{result:?}");
            } else {
                let error = result.unwrap_err();
                assert!(error.contains("at most 128 source locals"), "{error}");
            }
        }
    }
    for depth in [32, 160] {
        let mut pattern = "value".to_owned();
        let mut value = "a".to_owned();
        for _ in 0..depth {
            pattern = format!("{{next:{pattern}}}");
            value = format!("{{next:{value}}}");
        }
        let source =
            format!("export function run(a,b){{const {pattern}={value};return value===a?1:0;}}");
        for typed in [false, true] {
            let result = compile_nested_on_small_stack(source.clone(), typed);
            if depth == 32 {
                assert!(result.is_ok(), "{result:?}");
            } else {
                let error = result.unwrap_err();
                assert!(error.contains("nesting"), "{error}");
                assert!(!error.contains("internal IR"), "{error}");
            }
        }
    }
}

fn compile_nested_on_small_stack(source: String, typed: bool) -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || compile(&source, typed).map(|_| ()))
        .unwrap()
        .join()
        .expect("nested binding traversal must not overflow the native stack")
}

#[test]
fn object_bindings_evaluate_one_rhs_and_publish_declarators_in_order() {
    for body in [
        "let n=0;const make=()=>{n++;return {x:a,y:b};};const {x,y}=make();return n===1&&x===a&&y===b?1:0;",
        "let n=0;const {}=(n=n*10+1,{}),{x}=(n=n*10+2,{x:a}),{}=(n=n*10+3,{});return n===123&&x===a?1:0;",
        "let n=0;const source={x:a};const {x:first}=(n=n*10+1,source),{x:second}=(n=n*10+2,source.x=b,source),{x:third}=(n=n*10+3,{x:first});return n===123&&first===a&&second===b&&third===a?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn object_bindings_save_values_and_mutable_captures_share_the_binding() {
    for body in [
        "let source={x:a};const {x:saved}=source;source={x:b};let {x:changed}=source;changed=a;return saved===a&&changed===a&&source.x===b?1:0;",
        "const source={x:a};let {x}=source;const read=()=>x===b?1:0;x=b;source.x=a;return read()+(source.x===a?2:0);",
        "const child={x:a};const {ref}={ref:child};child.x=b;return ref===child&&ref.x===b?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn object_binding_fields_retain_proven_string_and_container_operations() {
    for source in [
        "export function run(a,b){const {text,items}={text:' abc ',items:[a,b]};return text.trim().endsWith('bc')&&items.length===2&&items.at(-1)===b?1:0;}",
        "function identity(x){return x;}export function run(a,b){const {row}={row:{label:' q ',values:[a,b]}};const take=()=>identity(row);const {label,values}=take();return label.trim()==='q'&&values.at(0)===a?1:0;}",
        "function identity(x){return x;}export function run(a,b){const {text}={text:' abc '};return identity(text).trim().indexOf('b');}",
        "function inspect(o){const {x}=o;return x?1:0;}export function run(a,b){return inspect({x:a})+inspect({x:b});}",
        "export function run(a,b){const {x}=a?{x:a}:{x:b};return x===(a?a:b)?1:0;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn dictionary_bindings_keep_special_keys_and_missing_fields() {
    compare(
        "const d=Object.create(null);d.constructor=a;d.__proto__=b;const {constructor:c,__proto__:p,toString:missing}=d;return c===a&&p===b&&missing===undefined?1:0;",
    );
}

#[test]
fn for_header_object_bindings_preserve_per_iteration_captures() {
    compare(
        "let first=()=>0;let last=()=>0;for(let {x}={x:a},i=0;i<2;i++,x=b){if(i===0)first=()=>x===a?1:0;last=()=>x===b?2:0;}return first()+last();",
    );
}

#[test]
fn object_binding_defaults_select_only_undefined() {
    for body in [
        "let n=0;const {missing=(n=n+1,a),explicit=(n=n+10,b)}={explicit:undefined};return n+(missing===a?100:0)+(explicit===b?1000:0);",
        "let n=0;const {nil=(n=1),flag=(n=2),zero=(n=3),negative=(n=4),nan=(n=5),empty=(n=6)}={nil:null,flag:false,zero:0,negative:-0,nan:NaN,empty:''};return n+(nil===null?10:0)+(flag===false?100:0)+(zero===0?1000:0)+(negative===0?10000:0)+(nan!==nan?100000:0)+(empty===''?1000000:0);",
        "let n=0;const {x=(n=1,b)}={x:a};return n+(x===a?10:0);",
        "const {'left-label':left=a,['right-label']:right=b,'':empty='fallback'}={};return left===a&&right===b&&empty==='fallback'?1:0;",
        "const undefined=7;let n=0;const {x=(n=n*10+1,a),y=(n=n*10+2,b)}={y:void 0};return n+(x===a?100:0)+(y===b?1000:0);",
    ] {
        compare(body);
    }
}

#[test]
fn object_binding_defaults_run_lazily_and_publish_in_source_order() {
    for body in [
        "let n=0;const {x=(n=n*10+1,a),y=(n=n*10+2,b),z=(n=n*10+3,x)}={y:0};return n+(x===a?100:0)+(y===0?1000:0)+(z===x?10000:0);",
        "let n=0;const {x=(n=1,a),y=x}={},{z=(n=n*10+2,y)}={};return n+(z===x?100:0);",
        "let n=0;const make=()=>{n=n*10+1;return {};};const {x=(n=n*10+2,a),y=(n=n*10+3,b)}=make();return n+(x===a?1000:0)+(y===b?10000:0);",
        "const source={};const {x:first=(source.x=b,a),x:second}=source;return first===a&&second===b?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn object_binding_defaults_save_the_rhs_and_read_later_fields_when_reached() {
    for body in [
        "let source={y:a};const old=source;const {x=(source={y:b},a),y}=source;return x===a&&y===a&&source!==old?1:0;",
        "const source={y:a};const alias=source;const {x=(alias.y=b,a),y}=source;return x===a&&y===b?1:0;",
        "let source={y:1};const old=source;const {x=(source={y:9},old.y=3,2),y}=source;return x===2&&y===3&&source!==old?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn object_binding_defaults_compose_with_helpers_blocks_and_live_captures() {
    for source in [
        "function fallback(x){return x;}export function run(a,b){const {x=fallback(a)}={};return x===a?1:0;}",
        "function inspect(o,value){const {x=value}=o;return x===value?1:0;}export function run(a,b){return inspect({},a)+inspect({x:b},a);}",
        "export function run(a,b){const {x=a,y=(()=>{return x;})()}={};return x===y?1:0;}",
        "export function run(a,b){let {x=a}={};const read=()=>x===b?1:0;const change=()=>{x=b;return read();};return change();}",
        "export function run(a,b){const selected={tag:a};const {x=selected}={};return x===selected?1:0;}",
        "export function run(a,b){const {text=' fallback '}={text:' ready '};return text.trim().indexOf('ead');}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn object_binding_defaults_preserve_header_and_body_capture_cells() {
    for body in [
        "let first=()=>0;let last=()=>0;for(let {x=1}={},i=0;i<2;i++,x=2){if(i===0)first=()=>x===1?1:10;last=()=>x===2?1:10;}return first()+last();",
        "let first=()=>0;let second=()=>0;for(let i=0;i<2;i++){let {x=i}={};const read=()=>x===0?1:2;if(i===0)first=read;else second=read;}return first()*10+second();",
    ] {
        compare(body);
    }
}

#[test]
fn object_binding_default_annotations_are_erased_without_proving_values() {
    let js = "export function run(a,b){let {x=a,label='fallback'}={};x=b;return x===b&&label==='fallback'?1:0;}";
    let ts = "type Shape=never;export function run(a:string,b:boolean):void{let {x=a,label='fallback'}:Shape=({} as unknown);x=b;return (x as number)===b&&(label as number)==='fallback'?1:0;}";
    common::compare_sources(js, ts);
    assert_eq!(
        compile(js, false).unwrap().wasm,
        compile(ts, true).unwrap().wasm
    );
    let source = "export function run(a,b){const {text='fallback'}:{text:string}={text:a};return text.trim().length;}";
    let error = compile(source, true)
        .err()
        .unwrap_or_else(|| panic!("admitted {source}"));
    assert!(error.contains("unsupported"), "{source}: {error}");
    assert!(!error.contains("internal IR"), "{source}: {error}");
}

#[test]
fn object_binding_defaults_keep_tdz_kind_and_pattern_guards() {
    for body in [
        "const {x=x}={};return a;",
        "const {x=y,y=a}={};return a;",
        "const x=a;{const {x=x}={};return x===a?1:0;}",
        "const {x=x}={x:a};return a;",
        "const {x=y,y=a}={x:b};return a;",
        "const {x=(y=a),y=b}={};return a;",
        "const {f=()=>x,x=a}={};return a;",
        "const {f=()=>x,x=a}={f:0};return a;",
        "const {x=undefined,undefined=a}={};return a;",
        "const {x=a}={};x=b;return a;",
        "const {x=a}={};return x+1;",
        "const {text='fallback'}={};return text.trim().length;",
        "const {text='fallback'}={text:a};return text.trim().length;",
        "const {child:{x}={x:a}}={};return a;",
        "const key='x';const {[key]:x=a}={};return a;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let error = compile(&source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(error.contains("unsupported"), "{source}: {error}");
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

#[test]
fn object_binding_defaults_keep_the_source_local_limit_on_a_small_native_stack() {
    for count in [128, 129] {
        let fields = (0..count)
            .map(|index| format!("value:v{index}=a"))
            .collect::<Vec<_>>()
            .join(",");
        let source = format!(
            "export function run(a,b){{const {{{fields}}}={{}};return v0===a&&v127===a?1:0;}}"
        );
        for typed in [false, true] {
            let source = source.clone();
            let result = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || compile(&source, typed).map(|_| ()))
                .unwrap()
                .join()
                .expect("default continuations must not overflow the native stack");
            if count == 128 {
                assert!(result.is_ok(), "{result:?}");
            } else {
                let error = result.unwrap_err();
                assert!(error.contains("at most 128 source locals"), "{error}");
            }
        }
    }
}

#[test]
fn object_binding_annotations_are_erased_without_proving_values() {
    let js = "export function run(a,b){let {x:value,text}={x:a,text:' ab '};value=b;return value===b&&text.trim()==='ab'?1:0;}";
    let ts = "type Shape=never;export function run(a:string,b:boolean):void{let {x:value,text}:Shape=({x:a,text:' ab '} as unknown);value=b;return (value as number)===b&&(text as number).trim()==='ab'?1:0;}";
    common::compare_sources(js, ts);
    assert_eq!(
        compile(js, false).unwrap().wasm,
        compile(ts, true).unwrap().wasm
    );
    for source in [
        "export function run(a,b){const source:{x:number}=a;const {x}=source;return x===a?1:0;}",
        "export function run(a,b){const {text}:{text:string}={text:a};return text.length;}",
    ] {
        let error = compile(source, true)
            .err()
            .unwrap_or_else(|| panic!("admitted {source}"));
        assert!(error.contains("unsupported"), "{source}: {error}");
        assert!(!error.contains("internal IR"), "{source}: {error}");
    }
}

#[test]
fn object_bindings_reject_unimplemented_patterns_and_unproven_sources() {
    for body in [
        "const {x,...rest}={x:a,y:b};return a;",
        "const key='x';const {[key]:x}={x:a};return a;",
        "const {[a?'x':'y']:x}={x:a,y:b};return a;",
        "const {1:x}={'1':a};return a;",
        "const {[1]:x}={'1':a};return a;",
        "const {x}=a;return a;",
        "const {x}=1;return a;",
        "const {x}='abc';return a;",
        "const {x}=false;return a;",
        "const {x}=null;return a;",
        "const {x}=undefined;return a;",
        "const {}=null;return a;",
        "const {}='abc';return a;",
        "const {length}=[a,b];return a;",
        "const source=a?{x:a}:null;const {x}=source;return a;",
        "const items=[{x:a}];const {x}=items.at(0);return a;",
        "const {constructor}={};return a;",
        "const {toString}={};return a;",
        "const {__proto__}={};return a;",
        "var {x}={x:a};return a;",
        "let x=a;({x}={x:b});return a;",
        "const {x}={x:a};x=b;return a;",
        "const x=a;{const {x}={x:x};return x===a?1:0;}",
        "const {x}={x:a};return x+1;",
        "const source={text:'abc'};source.text=a;const {text}=source;return text.length;",
        "const source={text:'abc'};const alias=source;alias.text=a;const {text}=source;return text.trim().length;",
        "const d=Object.create(null);d.text='abc';d[a?'text':'other']=a;const {text}=d;return text.length;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let error = compile(&source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(error.contains("unsupported"), "{source}: {error}");
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

#[test]
fn object_binding_source_local_limit_counts_each_leaf_and_empty_patterns_are_free() {
    for count in [128, 129] {
        let fields = (0..count)
            .map(|index| format!("value:v{index}"))
            .collect::<Vec<_>>()
            .join(",");
        let source = format!(
            "export function run(a,b){{const {{}}={{}},{{{fields}}}={{value:a}};return v0===a&&v127===a?1:0;}}"
        );
        for typed in [false, true] {
            let source = source.clone();
            let result = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || compile(&source, typed).map(|_| ()))
                .unwrap()
                .join()
                .expect("flat binding leaves must not overflow the native stack");
            if count == 128 {
                assert!(result.is_ok(), "{result:?}");
            } else {
                let error = result.unwrap_err();
                assert!(error.contains("at most 128 source locals"), "{error}");
            }
        }
    }
}

fn compare(body: &str) {
    let source = format!("export function run(a,b){{{body}}}");
    common::compare_sources(&source, &source);
}
