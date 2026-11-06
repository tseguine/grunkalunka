use grunkalunka_compiler::compile;

#[path = "managed/common.rs"]
mod common;

#[test]
fn object_assignments_preserve_cells_static_keys_rest_and_rhs_identity() {
    for body in [
        "let x='',y='';({x,name:y}={x:'one',name:'two'});return x.length*10+y.length;",
        "let [x,y,z]=[];({x,y,z}={x:a,y:b});return x===a&&y===b&&z===undefined?1:0;",
        "let x='';const read=()=>x;({name:x}={name:'text'});return read().length;",
        "let x='';const source={x:'text'};const result=({x}=source);return result===source&&x==='text'?1:0;",
        "let [x]=[];({first:x,second:x}={first:a,second:b});return x===b?1:0;",
        "let [x,y]=[];({[1]:x,[-0]:y}=[a,b]);return x===b&&y===a?1:0;",
        "let x='';const d=Object.create(null);d.constructor='text';({constructor:x}=d);return x==='text'?1:0;",
        "let [x]=[];let {...rest}={};const child={tag:b};const source={x:a,child};({x,...rest}=source);return x===a&&rest.x===undefined&&rest.child===child&&rest!==source?1:0;",
        "let [x]=[];const source=[a,b];let {...rest}={};({0:x,...rest}=source);return x===a&&rest[1]===b&&rest.length===undefined?1:0;",
        "const source=[a];return ({}=source)===source?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn numeric_binding_keys_select_and_exclude_the_same_property() {
    for body in [
        "const {1:x}={'1':a};return a;",
        "const {[1]:x}={'1':a};return a;",
        "const source={1:{name:' one '},'01':a,[-1.5]:b};const {0x1:{name},[-1.5]:x,...rest}=source;return name.trim()==='one'&&x===b&&rest['01']===a&&rest[1]===undefined&&rest[-1.5]===undefined&&Object.keys(rest).length===1?1:0;",
        "const source={[-0]:a,'-0':b};const {0:x,...rest}=source;return x===a&&rest['-0']===b&&rest[-0]===undefined?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn object_rest_copies_only_remaining_own_fields_into_fresh_slots() {
    for body in [
        "const {x,...rest}={x:a,y:b};return a;",
        "const {child:{x,...rest}}={child:{x:a}};return a;",
        "const [{x,...rest}]=[{x:a,y:b}];return a;",
        "for(const {...rest} of [{value:a}]){}return a;",
        "const child={value:a};const source={drop:a,'2':b,'1':a,child,empty:undefined};const {drop:x,drop:y,...rest}=source;const keys=Object.keys(rest);source.child=null;rest.empty=b;return x===y&&rest!==source&&rest.drop===undefined&&rest.child===child&&source.empty===undefined&&keys.length===4&&keys[0]==='1'&&keys[1]==='2'&&keys[2]==='child'&&keys[3]==='empty'?1:0;",
        r"const {['\ud800']:x,'':y,...rest}={'':a,'\ud800':b,'\ufffd':a,'\0':b};return x===b&&y===a&&rest['\ud800']===undefined&&rest['\ufffd']===a&&rest['\0']===b&&Object.keys(rest).length===2?1:0;",
        "const d=Object.create(null);d.__proto__=a;d.constructor=b;const {constructor:x,...r}=d;const keys=Object.keys(r);return x===b&&keys.length===1&&keys[0]==='__proto__'?1:0;",
        "const source={x:a};const {x,...empty}=source;const {...full}=source;return empty!==full&&full!==source&&full.x===a&&Object.keys(empty).length===0?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn object_rest_observes_defaults_on_the_saved_receiver_and_fresh_loop_cells() {
    for body in [
        "let source={tail:a};const original=source;const {first=(source={other:a},original.tail=b,original.added=a,1),...rest}=original;return first===1&&rest.tail===b&&rest.added===a&&rest.other===undefined&&source!==original?1:0;",
        "const row={name:a,keep:b};const source={row,outer:a};const {row:{name,...inner},...outer}=source;return inner.keep===b&&inner.name===undefined&&outer.outer===a&&outer.row===undefined?1:0;",
        "let first=()=>({}),last=()=>({});for(const {x,...rest} of [{x:a,keep:a},{x:b,keep:b}]){const read=()=>rest;if(x===a)first=read;last=read;}return first()!==last()&&first().keep===a&&last().keep===b?1:0;",
    ] {
        compare(body);
    }
}

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
fn nested_object_pattern_defaults_select_once_and_finish_before_outer_fields() {
    for body in [
        // Preserve the exact formerly rejected whole-pattern default.
        "const {child:{x}={x:a}}={};return a;",
        "let n=0;const make=()=>{n=n*10+1;return {present:{tag:b},explicit:undefined};};const {missing:{label=(n=n*10+3,' x '),tag}=(n=n*10+2,{tag:a}),present:{tag:kept}=(n=9,{tag:a}),explicit:{tag:last}=(n=n*10+4,{tag:b}),empty:{}=(n=n*10+5,{})}=make();return n+label.trim().length*100000+(tag===a?1000000:0)+(kept===b?10000000:0)+(last===b?100000000:0);",
        "const fallback={last:' before '};let source={tail:' root '};const original=source;const {child:{first=(original.child={last:' replacement '},fallback.last=' after ',' x '),last}=(source={tail:' new '},original.child=fallback,fallback),tail}=source;return first.trim().length+last.trim().length*10+tail.trim().length*100+(source!==original?1000:0)+(original.child!==fallback?10000:0);",
    ] {
        compare(body);
    }
    let source = "function fallback(tag){return {row:{label:' ready ',tag}};}export function run(a,b){const {child:{row:{label,tag}}=fallback(a)}={};return label.trim().length+(tag===a?100:0);}";
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
        "const {child:{}={}}={child:null};return a;",
        "const {child:{x}={x:a}}={child:b};return a;",
        "const {child:{}={}}={child:undefined,...{child:a}};return a;",
        "const source={};const alias=source;alias.child=a;const {child:{x}={x:b}}=source;return a;",
        "const {child:{}=null}={};return a;",
        "const {child:{x}={x}}={};return a;",
        "const {child:{x}=later,later={x:a}}={};return a;",
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
fn object_binding_defaults_prove_consumers_after_known_undefined_selections() {
    for body in [
        // Preserve the exact former rejection now justified by a missing own field.
        "const {text='fallback'}={};return text.trim().length;",
        "let n=0;const saved=()=>a;const source=a?{read:saved}:{};const {read=(n=1,saved)}=source;const [kept=(n=2,saved)]=[read];return read===saved&&kept===saved?n:-1;",
        "const {label=' ready ',row={tag:a},items=[a,b]}={label:undefined,row:void 0};return label.trim().length+(row.tag===a?10:0)+items.length*100+(items.at(-1)===b?1000:0);",
        "let missing;const undefined=7;let n=0;const {text=' hi ',other='bye'}={text:void(n++),other:missing};return n*100+text.trim().length*10+other.length;",
        "const source={...{text:a},text:undefined};const {text='ready'}=source;return text.trim().length;",
        "const source=Object.create(null);source.label=' chosen ';const alias=source;const {label=' fallback ',constructor:other=' name ',__proto__:row={tag:a}}=alias;return label.trim().length+other.trim().length*10+(row.tag===a?100:0);",
    ] {
        compare(body);
    }
}

#[test]
fn proven_object_defaults_keep_helper_captures_laziness_and_original_receivers() {
    let source = "function identity(value){return value;}export function run(a,b){let n=0;const original=a?{label:' ready '}:{};const alias=identity(original);let {label=(n++,' fallback ')}=alias;const read=()=>label.trim().length;return read()+n*100;}";
    common::compare_sources(source, source);
    compare(
        "let n=0;let source={tail:' before '};const old=source;const {text=(n=1,source={tail:' other '},old.tail=' after ',' fallback '),tail}=source;return text.trim().length+tail.trim().length*10+n*100+(source!==old?1000:0);",
    );
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
    for source in [
        "export function run(a,b){const {text='fallback'}:{text:string}={text:a};return text.trim().length;}",
        "export function run(a,b){let missing;(missing as number)=a;const {label=' x '}={label:missing};return label.trim().length;}",
    ] {
        let error = compile(source, true)
            .err()
            .unwrap_or_else(|| panic!("admitted {source}"));
        assert!(error.contains("unsupported"), "{source}: {error}");
        assert!(!error.contains("internal IR"), "{source}: {error}");
    }
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
        "const {text='fallback'}={text:a};return text.trim().length;",
        // Default-selected callable values do not establish direct call signatures.
        "const {read=()=>a}={};return +read();",
        "const {read=()=>a}={read:b};return +read();",
        "const {text='fallback'}={text:null};return text.trim().length;",
        "const undefined=a;const {text='fallback'}={text:undefined};return text.trim().length;",
        "let value;value=a;const {text='fallback'}={text:value};return text.trim().length;",
        "let missing;missing++;const {label=' x '}={label:missing};return label.trim().length;",
        "let missing=undefined;missing+=a;const {label=' x '}={label:missing};return label.trim().length;",
        "const source={...{text:a}};const {text='fallback'}=source;return text.trim().length;",
        "const source={text:undefined,...{text:a}};const {text='fallback'}=source;return text.trim().length;",
        "const source={};const alias=source;alias.text=a;const {text='fallback'}=source;return text.trim().length;",
        "let {text='ready'}={};const read=()=>text.length;text=a;return read();",
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
        "const key='x';const {[key]:x}={x:a};return a;",
        "const {[a?'x':'y']:x}={x:a,y:b};return a;",
        "const {x}=a;return a;",
        "const {x}=1;return a;",
        "const {x}='abc';return a;",
        "const {x}=false;return a;",
        "const {x}=null;return a;",
        "const {x}=undefined;return a;",
        "const {}=null;return a;",
        "const {}='abc';return a;",
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

#[test]
fn numeric_key_annotations_erase_in_literals_bindings_and_reads() {
    common::compare_sources(
        "export function run(a,b){const o={[-0]:a,1:b};const {[0]:x,...r}=o;return x===a&&r[1]===b&&r[0]===undefined?1:0;}",
        "export function run(a:number,b:number){const o={[(-0 as unknown)]:a,1:b};const {[(0 as string)]:x,...r}=o;return x===a&&r[(1 as string)]===b&&r[0]===undefined?1:0;}",
    );
}

#[test]
fn object_rest_fields_support_proven_nested_and_callable_consumers() {
    for body in [
        "const source={id:a,label:' ready ',row:{title:' nested '},items:['first'],apply:x=>x+1};const {id,...rest}=source;const {row:{title},items:[first]}=rest;return rest.label.trim()==='ready'&&title.trim()==='nested'&&first.length===5&&rest.apply(1)===2?1:0;",
        "const {title,...first}={title:a,label:' entry '};const {missing,...second}=first;const {title:replacement=' default ',label}=second;return replacement.trim()==='default'&&label.trim()==='entry'?1:0;",
        "const source={id:a,row:{value:b}};const {id,...rest}=source;rest.row.value=a;const same=source.row.value===a;rest.row={value:b};return same&&source.row!==rest.row&&rest.row.value===b?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn object_rest_field_consumers_reject_possible_incompatible_values() {
    for body in [
        "const source={label:'x'};const alias=source;alias.label=a;const {...rest}=source;return rest.label.trim().length;",
        "const source={nested:{label:'x'}};const {...rest}=source;rest.nested.label=a;return source.nested.label.trim().length;",
        "const source={label:'x'};const {...rest}=source;const alias=rest;alias.label=a;return rest.label.trim().length;",
        "const source=a?{label:'x'}:{label:null};const {...rest}=source;return rest.label.trim().length;",
        "const {label,...rest}={label:'x'};return rest.label.trim().length;",
        "const {...rest}={...{label:b}};const {label='fallback'}=rest;return label.trim().length;",
        "let current={label:'x'};for(let i=0;i<2;i++){const {...next}=current;current=next;}return current.label.trim().length;",
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
fn dictionary_binding_keys_run_once_and_exclude_their_saved_strings() {
    for body in [
        "const d=Object.create(null);d.x=a;d.y=b;d.z=undefined;let k='x',n=0;const {[ (n=n*10+1,k) ]:first,[ (n=n*10+2,k='y',k) ]:second,[ (n=n*10+3,k='x',k) ]:again,...rest}=d;const keys=Object.keys(rest);return n===123&&first===a&&second===b&&again===a&&keys.length===1&&keys[0]==='z'?1:0;",
        "let source=Object.create(null);const old=source;source.keep=a;const other=Object.create(null);other.wrong=b;let k='missing';const {[(source=other,k)]:value=(old.keep=b,k='keep',1),...rest}=source;return value===1&&rest.keep===b&&rest.wrong===undefined&&Object.keys(rest).length===1?1:0;",
        r"const d=Object.create(null);const k='\ud800';d[k]=a;d['\ufffd']=b;d.__proto__=a;const {[k]:x,[('__'+'proto__')]:y,...rest}=d;const keys=Object.keys(rest);return x===a&&y===a&&keys.length===1&&keys[0]==='\ufffd'&&rest['\ufffd']===b?1:0;",
        "const d=Object.create(null);d.tag=a;const source=[d];const [{[(source[1]=b,'tag')]:value},...tail]=source;return value===a&&tail.length===1&&tail[0]===b?1:0;",
        "const d=Object.create(null);d.key='value';d.value=a;let n=0;for(const {key='fallback',[key]:value,...rest} of [d,d]){if(value===a&&Object.keys(rest).length===0)n++;}return n;",
    ] {
        compare(body);
    }
}

#[test]
fn object_patterns_select_array_length_indices_and_remaining_enumerable_fields() {
    for body in [
        "const {length}=[a,b];return a;",
        "const {child:{}=[]}={};return a;",
        "const {child:{}}={child:[]};return a;",
        "const source=[a,,b];source.named=a;const {0:first,1:missing=' empty ',length:n,...rest}=source;const keys=Object.keys(rest);source[2]=a;return first===a&&missing.trim()==='empty'&&n===3&&rest[2]===b&&rest.named===a&&rest.length===undefined&&keys.length===2&&keys[0]==='2'&&keys[1]==='named'?1:0;",
        "const source=[a];const {...rest}=source;const {length:label=' absent '}=rest;return label.trim()==='absent'&&rest[0]===a?1:0;",
        "const src=[undefined,b];const {0:first=(src.length=0,src[2]=a,1),length:n,...rest}=src;return first===1&&n===3&&Object.keys(rest).length===1&&rest[2]===a?1:0;",
        "for(const {0:first,length:n,...others} of [[' title ',a]])return first.trim().length+(n===2&&others[1]===a?10:0);return 0;",
    ] {
        compare(body);
    }
}
