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
        "const {child:{x}}={child:{x:a}};return a;",
        "const {items:[x]}={items:[a]};return a;",
        "const [x]=[a];return a;",
        "const {x=b}={x:a};return a;",
        "const {x:y=b}={x:a};return a;",
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
