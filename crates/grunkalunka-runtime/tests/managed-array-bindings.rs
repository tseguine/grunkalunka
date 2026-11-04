use grunkalunka_compiler::compile;

#[path = "managed/common.rs"]
mod common;

#[test]
fn array_bindings_select_positions_holes_and_missing_elements() {
    for body in [
        "const [first,,third,missing]=[a,,b];return first===a&&third===b&&missing===undefined?1:0;",
        "const [,second,,fourth,]=[0,a,0,b];return second===a&&fourth===b?1:0;",
        "const [hole,explicit,last,after]=[,undefined,b];return hole===undefined&&explicit===undefined&&last===b&&after===undefined?1:0;",
        "const [nil,flag,zero,nan,empty]=[null,false,-0,NaN,''];return nil===null&&flag===false&&zero===0&&nan!==nan&&empty===''?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn array_bindings_evaluate_one_rhs_and_publish_declarators_in_order() {
    for body in [
        "let n=0;const make=()=>{n++;return [a,b];};const [x,y]=make();return n===1&&x===a&&y===b?1:0;",
        "let n=0;const []=(n=n*10+1,[]),[,] = (n=n*10+2,[a]),[x]=(n=n*10+3,[b]);return n===123&&x===b?1:0;",
        "let n=0;const source=[a];const [first]=(n=n*10+1,source),[second]=(n=n*10+2,source[0]=b,source),[third]=(n=n*10+3,[first]);return n===123&&first===a&&second===b&&third===a?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn array_bindings_save_values_and_keep_mutable_captures_and_reference_identity() {
    for body in [
        "let source=[a];const [saved]=source;source=[b];let [changed]=source;changed=a;return saved===a&&changed===a&&source[0]===b?1:0;",
        "const source=[a];let [x]=source;const read=()=>x===b?1:0;const change=()=>{x=b;return read();};source[0]=a;return change()+(source[0]===a?2:0);",
        "const child={x:a};const items=[b];const [ref,selected]=[child,items];child.x=b;selected[0]=a;return ref===child&&ref.x===b&&selected===items&&items[0]===a?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn array_binding_positions_retain_proven_string_and_container_operations() {
    for source in [
        "export function run(a,b){const [text,items,row]=[' abc ',[a,b],{tag:b}];return text.trim().endsWith('bc')&&items.length===2&&items.at(-1)===b&&row.tag===b?1:0;}",
        "function identity(x){return x;}export function run(a,b){const [row]=[{label:' q ',values:[a,b]}];const take=()=>identity(row);const {label,values}=take();return label.trim()==='q'&&values.at(0)===a?1:0;}",
        "function identity(x){return x;}export function run(a,b){const [text]=[' abc '];return identity(text).trim().indexOf('b');}",
        "function inspect(items){const [x]=items;return x?1:0;}export function run(a,b){return inspect([a])+inspect([b]);}",
        "export function run(a,b){const [x]=a?[a]:[b];return x===(a?a:b)?1:0;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn array_bindings_preserve_header_and_body_capture_cells() {
    for body in [
        "let first=()=>0;let last=()=>0;for(let [x]=[a],i=0;i<2;i++,x=b){if(i===0)first=()=>x===a?1:0;last=()=>x===b?2:0;}return first()+last();",
        "let first=()=>0;let second=()=>0;for(let i=0;i<2;i++){let [x]=[i];const read=()=>x===0?1:2;if(i===0)first=read;else second=read;}return first()*10+second();",
    ] {
        compare(body);
    }
}

#[test]
fn array_binding_annotations_are_erased_without_proving_values() {
    let js = "export function run(a,b){let [x,text]=[a,' ab '];x=b;return x===b&&text.trim()==='ab'?1:0;}";
    let ts = "type Tuple=never;export function run(a:string,b:boolean):void{let [x,text]:Tuple=([a,' ab '] as unknown);x=b;return (x as number)===b&&(text as number).trim()==='ab'?1:0;}";
    common::compare_sources(js, ts);
    assert_eq!(
        compile(js, false).unwrap().wasm,
        compile(ts, true).unwrap().wasm
    );
    for source in [
        "export function run(a,b){const source:number[]=a;const [x]=source;return x===a?1:0;}",
        "export function run(a,b){const source:[string]=[a];const [text]=source;return text.trim().length;}",
    ] {
        let error = compile(source, true)
            .err()
            .unwrap_or_else(|| panic!("admitted {source}"));
        assert!(error.contains("unsupported"), "{source}: {error}");
        assert!(!error.contains("internal IR"), "{source}: {error}");
    }
}

#[test]
fn array_bindings_keep_pattern_tdz_and_value_proof_boundaries() {
    for body in [
        "const [x,...rest]=[a,b];return a;",
        "const [[x]]=[[a]];return a;",
        "const [{x}]=[{x:a}];return a;",
        "const [x=a]=[];return a;",
        "const [x]=a;return a;",
        "const [x]={0:a,length:1};return a;",
        "const [x]='abc';return a;",
        "const []=null;return a;",
        "const [,]=undefined;return a;",
        "const source=a?[a]:null;const [x]=source;return a;",
        "var [x]=[a];return a;",
        "let x=a;[x]=[b];return x;",
        "const [x]=[a];x=b;return a;",
        "const x=a;{const [x]=[x];return x===a?1:0;}",
        "const [x]=[a];return x+1;",
        "const source=['ready'];source[0]=a;const [text]=source;return text.length;",
        "const source=['ready'];const alias=source;alias.pop();const [text]=source;return text.trim().length;",
        "const source=[{value:a}];source.length=0;const [row]=source;return row.value===a?1:0;",
        "let [text]=['ready'];const read=()=>text.length;text=a;return read();",
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
    let source = "function take([x]){return x;}export function run(a,b){return take([a]);}";
    for typed in [false, true] {
        let error = compile(source, typed)
            .err()
            .unwrap_or_else(|| panic!("admitted {source}"));
        assert!(error.contains("unsupported"), "{source}: {error}");
        assert!(!error.contains("internal IR"), "{source}: {error}");
    }
}

#[test]
fn array_binding_local_budget_counts_leaves_not_elisions_on_a_small_native_stack() {
    for count in [128, 129] {
        let leaves = (0..count)
            .map(|index| format!("v{index}"))
            .collect::<Vec<_>>()
            .join(",,");
        let source = format!(
            "export function run(a,b){{const []=[],[,,,,]=[],[{leaves}]=[a];return v0===a&&v127===undefined?1:0;}}"
        );
        for typed in [false, true] {
            let result = compile_on_small_stack(source.clone(), typed);
            if count == 128 {
                assert!(result.is_ok(), "{result:?}");
            } else {
                let error = result.unwrap_err();
                assert!(error.contains("at most 128 source locals"), "{error}");
            }
        }
    }
    let source = format!(
        "export function run(a,b){{const [{}value]=[];return value===undefined?1:0;}}",
        ",".repeat(2048)
    );
    for typed in [false, true] {
        let result = compile_on_small_stack(source.clone(), typed);
        assert!(result.is_ok(), "{result:?}");
    }
}

fn compile_on_small_stack(source: String, typed: bool) -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || compile(&source, typed).map(|_| ()))
        .unwrap()
        .join()
        .expect("flat array bindings must not overflow the native stack")
}

fn compare(body: &str) {
    let source = format!("export function run(a,b){{{body}}}");
    common::compare_sources(&source, &source);
}
