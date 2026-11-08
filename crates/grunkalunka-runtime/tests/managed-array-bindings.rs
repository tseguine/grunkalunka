use grunkalunka_compiler::compile;

#[path = "managed/common.rs"]
mod common;

#[test]
fn numeric_array_binding_results_execute_the_exact_former_guard_sources() {
    for body in [
        "let x=a;[x]=[b];return x;",
        "const [x]=[a];return x+1;",
        "const [x=a]=[];return x+1;",
    ] {
        compare(body);
    }
}

#[test]
fn array_assignment_defaults_keep_order_live_length_and_sticky_exhaustion() {
    for body in [
        "let x=0,y=0,n=0;[x=(n=n*10+1,a),y=(n=n*10+2,b)]=[];return n*100+(x===a?10:0)+(y===b?1:0);",
        "let x=0,y=0,n=0;[x=(n++,a),y=(n++,b)]=[null,false];return n===0&&x===null&&y===false?1:0;",
        "let x=0,y=0;const source=[undefined,a];[x=(source[1]=b,a),y]=source;return x===a&&y===b?1:0;",
        "let x=0,y=0;const source=[];[x=(source[1]=b,a),y]=source;return x===a&&y===undefined?1:0;",
        "let x=0,y=0,z=0;const source=[undefined,a];[x=(source.length=0,a),y=(source[2]=b,b),z]=source;return x===a&&y===b&&z===undefined?1:0;",
        "let x=0,y=0;let source=[undefined,a];const original=source;const result=([x=(source=[b],b),y]=source);return result===original&&source!==original&&x===b&&y===a?1:0;",
        "let x=0,y=0;[x=a,y=x]=[];return x===a&&y===a?1:0;",
        "let x=0,tail=[];const source=[];[x=(source[1]=b,a),...tail]=source;return tail.length;",
        "let x=0,tail=[];const source=[undefined,a];[x=(source[2]=b,a),...tail]=source;return tail.length*10+(tail[0]===a&&tail[1]===b?1:0);",
        "let x='';const undefined=a;[x='text']=[];return x.length;",
        "let x='';const read=()=>x;[x='text']=[];return read().length;",
    ] {
        compare(body);
    }
}

#[test]
fn assignment_patterns_accept_numeric_and_implicitly_undefined_locals() {
    for body in [
        "let x=0,y=0;[x,y]=[a,b];return x===a&&y===b?1:0;",
        "let x,y;const before=x===undefined&&y===undefined;[x,y]=[a,b];return before&&x===a&&y===b?1:0;",
        "let x=0;const read=()=>x;[x]=['text'];return read()==='text'?1:0;",
        "let x;const read=()=>x;const before=read()===undefined;[x]=[b];return before&&read()===b?1:0;",
        "let tail=[];[,...tail]=[a,,b];return tail.length*10+(tail[0]===undefined&&tail[1]===b?1:0);",
        "let x=0,rest={};const source={x:a,last:b};({x,...rest}=source);return x===a&&rest.last===b&&rest.x===undefined?1:0;",
        "let x=0;{let x=0;[x]=[b];}return x;",
    ] {
        compare(body);
    }
    common::compare_sources(
        "export function run(a,b){let x=0;[x]=['text'];return x==='text'?1:0;}",
        "export function run(a:number,b:number):number{let x:number=0;[x]=['text'];return x==='text'?1:0;}",
    );
}

#[test]
fn array_assignments_write_existing_cells_and_return_the_original_rhs() {
    for body in [
        "let x='',y='';[x,y]=['first','second'];return x.length*10+y.length;",
        "let [x,y,z]=[];[x,,y,z]=[a,,b];return x===a&&y===b&&z===undefined?1:0;",
        "let x='';const source=['text'];const result=([x]=source);return result===source&&x==='text'?1:0;",
        "let x='';const read=()=>x;[x]=['saved'];return read().length;",
        "let [x]=[a];[x,x]=[a,b];return x===b?1:0;",
        "let [tail]=[[]];const source=[a,,b];[,...tail]=source;return tail.length*10+(tail[0]===undefined&&tail[1]===b&&tail!==source?1:0);",
        "let x='';let n=0;const make=()=>{n++;return ['text'];};[x]=make();return n*10+x.length;",
        "let x='';if([x]=['text'])return x.length;return 0;",
    ] {
        compare(body);
    }
}

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
fn array_rest_bindings_admit_original_sources_and_compose_with_patterns() {
    for body in [
        // Preserve the three exact formerly rejected identifier-rest sources.
        "const [x,...rest]=[a,b];return a;",
        "const [[x,...rest]]=[[a,b]];return a;",
        "const [x=a,...rest]=[];return a;",
        "let n=0;const {group:[,,...rest]=(n++,[a,,b])}={};const [...again]=rest;return n*100+rest.length*10+again.length+(again!==rest&&again[0]===b?1000:0);",
    ] {
        compare(body);
    }
    // Rest provenance must not depend on an Array or Object expression in the source.
    let source = "function size(items){return +items.length;}export function run(a,b){const [...rest]=Object.keys('ab');return +size(rest)+(rest[0]==='0'?10:0)+(rest[1]==='1'?100:0);}";
    common::compare_sources(source, source);
}

#[test]
fn array_rest_bindings_copy_dense_shallow_values_and_keep_slots_independent() {
    compare(
        "const child={tag:a};const values=[b];const fn=()=>a;const source=[a,,undefined,child,values,fn];source.named=b;const [first,...rest]=source;const keys=Object.keys(rest);const copied=rest[0]===undefined&&rest[1]===undefined&&rest[2]===child&&rest[3]===values&&rest[4]===fn;const replacement={tag:b};source[3]=replacement;source[4]=[];source[5]=()=>b;const retained=rest[2]===child&&rest[3]===values&&rest[4]===fn;rest[0]=b;rest[2]=null;return copied&&retained&&rest!==source&&rest.length===5&&keys.length===5&&keys[0]==='0'&&keys[1]==='1'&&keys[4]==='4'&&source[1]===undefined&&source[3]===replacement&&rest.named===undefined?1:0;",
    );
}

#[test]
fn array_rest_bindings_keep_original_sources_and_permanent_exhaustion() {
    for body in [
        "let source=[undefined,a,b];const original=source;const [first=(source=[b,b,b],original[1]=b,a),...rest]=source;const copied=rest[0]===b&&rest[1]===b;original[2]=a;rest[0]=a;return first===a&&copied&&rest.length===2&&rest[0]===a&&rest[1]===b&&original[1]===b&&original[2]===a&&source!==original&&rest!==original?1:0;",
        "let [...rest]=[a];const original=rest;const grown=Object.keys(rest,rest[2]=b);const saved=Object.keys(rest,rest=[b,b]);return grown.length*100+saved.length*10+rest.length+(grown[0]==='0'&&grown[1]==='2'&&saved[1]==='2'&&rest!==original?1000:0);",
        "const source=[];const [first=(source[1]=b,a),...rest]=source;const [...other]=[];return rest.length*100+(rest.at(0)===b?10:0)+(rest!==source&&rest!==other?1:0);",
        "const source=[,];const [first=(source[1]=b,a),...rest]=source;const [...other]=[];return rest.length*100+(rest.at(0)===b?10:0)+(rest!==source&&rest!==other?1:0);",
        // A zero-leaf descendant default can grow the ancestor before an elision.
        "const source=[];const [[]=(source[2]=9,[]),,...rest]=source;return rest.length*10+(rest[0]===9?1:0);",
        "const source=[,];const [[]=(source[2]=9,[]),,...rest]=source;return rest.length*10+(rest[0]===9?1:0);",
    ] {
        compare(body);
    }
}

#[test]
fn array_rest_bindings_keep_receiver_and_leaf_proofs_conservative() {
    for body in [
        "const [...rest]=a;return a;",
        "const [...rest]=null;return a;",
        "const [...rest]='abc';return a;",
        "const {items:[...rest]}={items:a};return a;",
        "const [x=rest,...rest]=[];return a;",
        "const [...rest]=rest;return a;",
        "const [f=()=>rest,...rest]=[];return a;",
        "const [...rest]=[a];rest=[];return a;",
        "let [...rest]=[a];const read=()=>+rest.length;rest=b;return +read();",
        "const [...rest]=Object.keys('ab');const [text]=rest;return text.trim().length;",
        "const [...rest]=[a];const [text='fallback']=rest;return text.trim().length;",
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
fn nested_array_bindings_admit_original_patterns_and_select_mixed_records() {
    for body in [
        // These three exact sources previously documented rejected patterns.
        "const [[x]]=[[a]];return a;",
        "const [{x}]=[{x:a}];return a;",
        "const {items:[x]}={items:[a]};return a;",
        "const child={tag:b};const [{row:[label,{values:[first,,last]},ref]},[]]=[{row:[' Ready ',{values:[a,,b]},child]},[]];return label.trim()==='Ready'&&first===a&&last===b&&ref===child?1:0;",
        "let n=0;const make=()=>{n++;return [[],{empty:[]}];};const [[],{empty:[]}]=make();return n;",
    ] {
        compare(body);
    }
    let source = "function identity(value){return value;}export function run(a,b){const source=[{row:{items:[' x ',a,b]}}];const alias=identity(source);const [{row:{items:[label,first,last]}}]=alias;return label.trim()==='x'&&first===a&&last===b?1:0;}";
    common::compare_sources(source, source);
}

#[test]
fn nested_pattern_defaults_admit_original_sources_and_select_mixed_receivers() {
    for body in [
        // Preserve the three exact formerly rejected whole-pattern defaults.
        "const [[x]=[a]]=[];return a;",
        "const [{x}={x:a}]=[];return a;",
        "const {items:[x]=[a]}={};return a;",
        "let n=0;const [{items:[label,tag]=(n=n*10+2,[' x ',a])}=(n=n*10+1,{}),[]=(n=n*10+3,[])]=[];return n+label.trim().length*1000+(tag===a?10000:0);",
    ] {
        compare(body);
    }
}

#[test]
fn whole_array_defaults_keep_ancestor_exhaustion_and_fresh_inner_iterators() {
    for body in [
        // Even a child with no leaves can grow an ancestor before its next step.
        "const outer=[];const [[]=(outer[1]=9,[]),tail]=outer;return (tail===undefined?10:0)+(tail===9?1:0);",
        "const outer=[,];const [[]=(outer[1]=9,[]),tail]=outer;return (tail===undefined?10:0)+(tail===9?1:0);",
        "const inner=[];const outer=[undefined,undefined];const [[x=(inner[0]=7,1),y]=inner,[z,w]=inner]=outer;return (x===1?1000:0)+(y===undefined?100:0)+(z===7?10:0)+(w===undefined?1:0);",
    ] {
        compare(body);
    }
}

#[test]
fn nested_array_bindings_retain_original_receivers_and_read_after_nested_defaults() {
    compare(
        "const inner=[undefined,a];let outer=[inner,b];const original=outer;const [[x=(outer[0]=[b,b],outer=[[],a],inner[1]=b,a),y],z]=outer;return x===a&&y===b&&z===b&&original[0]!==inner&&outer!==original?1:0;",
    );
}

#[test]
fn nested_array_binding_defaults_keep_ancestor_iterators_live() {
    for body in [
        "const inner=[];const outer=[inner];const [[x=(inner[1]=7,outer[1]=9,1),y],z]=outer;return (x===1?100:0)+(y===7?10:0)+(z===9?1:0);",
        "const inner=[,];const outer=[inner];const [[x=(inner[1]=7,outer[1]=9,1),y],z]=outer;return (x===1?100:0)+(y===7?10:0)+(z===9?1:0);",
    ] {
        compare(body);
    }
}

#[test]
fn nested_array_bindings_distinguish_elisions_from_permanent_exhaustion() {
    for body in [
        "const inner=[,];const outer=[inner];const [[x=(inner[1]=7,1),y],,seed=(outer[3]=9,2),tail]=outer;return (x===1?1000:0)+(y===7?100:0)+(seed===2?10:0)+(tail===9?1:0);",
        "const inner=[,];const outer=[inner,,,];const [[x=(inner[1]=7,1),y],,seed=(outer[3]=9,2),tail]=outer;return (x===1?1000:0)+(y===7?100:0)+(seed===2?10:0)+(tail===9?1:0);",
    ] {
        compare(body);
    }
}

#[test]
fn nested_array_bindings_own_exhaustion_per_iterator_not_per_receiver() {
    for body in [
        "const empty=[];const hole=[,];const outer=[empty,hole];const [[x=(empty[1]=7,1),y],[z=(hole[1]=8,2),w]]=outer;return (x===1?1000:0)+(y===undefined?100:0)+(z===2?10:0)+(w===8?1:0);",
        "const inner=[];const outer=[inner,inner];const [[x=(inner[0]=7,1),y],[z,w]]=outer;return (x===1?1000:0)+(y===undefined?100:0)+(z===7?10:0)+(w===undefined?1:0);",
    ] {
        compare(body);
    }
}

#[test]
fn nested_array_binding_leaves_retain_fresh_header_and_body_capture_cells() {
    for body in [
        "let first=()=>0;let second=()=>0;for(let i=0;i<2;i++){let [{items:[x=i]}]=[{items:[]}];const read=()=>x===0?1:2;if(i===0)first=read;else second=read;}return first()*10+second();",
        "let first=()=>0;let last=()=>0;for(let [{items:[x=1]}]=[{items:[]}],i=0;i<2;i++,x=2){if(i===0)first=()=>x===1?1:10;last=()=>x===2?1:10;}return first()+last();",
    ] {
        compare(body);
    }
}

#[test]
fn nested_array_binding_annotations_are_erased_without_proving_receivers() {
    let js = "export function run(a,b){let [{row:[value,label]}]=[{row:[a,' ab ']}];value=b;return value===b&&label.trim()==='ab'?1:0;}";
    let ts = "type Shape=never;export function run(a:string,b:boolean):void{let [{row:[value,label]}]:Shape=([{row:[a,' ab ']}] as unknown);value=b;return (value as number)===b&&(label as number).trim()==='ab'?1:0;}";
    common::compare_sources(js, ts);
    assert_eq!(
        compile(js, false).unwrap().wasm,
        compile(ts, true).unwrap().wasm
    );
    let source = "export function run(a,b){const source:[{items:number[]}]=[{items:a}];const [{items:[x]}]=source;return x===a?1:0;}";
    let error = compile(source, true)
        .err()
        .unwrap_or_else(|| panic!("admitted {source}"));
    assert!(error.contains("unsupported"), "{source}: {error}");
    assert!(!error.contains("internal IR"), "{source}: {error}");
}

#[test]
fn nested_array_bindings_preserve_receiver_proof_tdz_and_pattern_limits() {
    for body in [
        "const [[]]=[];return a;",
        "const [[]]=[null];return a;",
        "const [[]]=[{}];return a;",
        "const [{items:[]}]=[{items:undefined}];return a;",
        "const {items:[]}={items:a?[a]:null};return a;",
        "const [[x]]=[a];return a;",
        "const [{x}]=[a?{x:a}:null];return a;",
        "const source=[[a]];const alias=source;alias[0]=a;const [[x]]=source;return a;",
        "const source=[['ready']];const alias=source[0];alias[0]=a;const [[text]]=source;return text.length;",
        "const source=[[a]];source.length=0;const [[x]]=source;return a;",
        "const [[x=x]]=[[]];return a;",
        "const [[x=y],y=a]=[[]];return a;",
        "const [x=y,[y=a]]=[undefined,[]];return a;",
        "const [[]=[]]=[null];return a;",
        "const [[x]=[a]]=[b];return a;",
        "const [[]={}]=[];return a;",
        "const source=[undefined];const alias=source;alias[0]=a;const [[x]=[b]]=source;return a;",
        "const {items:[x]=[a]}={...{items:b}};return a;",
        "const [[x]=[x]]=[];return a;",
        "const [[x]=later,later=[a]]=[];return a;",
        // The write taints index zero even though this native path first exhausts.
        "const outer=[];const [[x]=(outer[0]=7,outer[1]=9,outer),tail]=outer;return (x===7?100:0)+(tail===9?10:0)+(tail===undefined?1:0);",
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
fn nested_array_binding_budgets_count_leaves_and_bound_mixed_source_depth() {
    for count in [128, 129] {
        let leaves = (0..count)
            .map(|index| format!("v{index}"))
            .collect::<Vec<_>>()
            .join(",,");
        let source = format!(
            "export function run(a,b){{const [[],{{items:[{leaves}]}}]=[[],{{items:[a]}}];return v0===a&&v127===undefined?1:0;}}"
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
    for depth in [32, 160] {
        let mut pattern = "value".to_owned();
        let mut value = "a".to_owned();
        for index in 0..depth {
            if index % 2 == 0 {
                pattern = format!("[{pattern}]");
                value = format!("[{value}]");
            } else {
                pattern = format!("{{next:{pattern}}}");
                value = format!("{{next:{value}}}");
            }
        }
        let source =
            format!("export function run(a,b){{const {pattern}={value};return value===a?1:0;}}");
        for typed in [false, true] {
            let result = compile_on_small_stack(source.clone(), typed);
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
    let original = "export function run(a,b){var [x]=[a];return a;}";
    common::compare_sources(original, original);
    for body in [
        "const [x]=a;return a;",
        "const [x]={0:a,length:1};return a;",
        "const [x]='abc';return a;",
        "const []=null;return a;",
        "const [,]=undefined;return a;",
        "const source=a?[a]:null;const [x]=source;return a;",
        "for(var [x] of [[a]]){}return a;",
        "const [x]=[a];x=b;return a;",
        "const x=a;{const [x]=[x];return x===a?1:0;}",
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

#[test]
fn array_binding_defaults_select_only_intrinsic_undefined() {
    for body in [
        "let n=0;const [missing=(n=n*10+1,a),explicit=(n=n*10+2,b)]=[,undefined];return n+(missing===a?100:0)+(explicit===b?1000:0);",
        "let n=0;const [nil=(n=1),flag=(n=2),zero=(n=3),negative=(n=4),nan=(n=5),empty=(n=6)]=[null,false,0,-0,NaN,''];return n+(nil===null?10:0)+(flag===false?100:0)+(zero===0?1000:0)+(negative===0?10000:0)+(nan!==nan?100000:0)+(empty===''?1000000:0);",
        "const undefined=7;let n=0;const [x=(n=n*10+1,a),y=(n=n*10+2,b)]=[,void 0];return n+(x===a?100:0)+(y===b?1000:0);",
        "let n=0;const [x=(n=1,b)]=[a];return n+(x===a?10:0);",
    ] {
        compare(body);
    }
}

#[test]
fn array_binding_defaults_prove_consumers_after_known_undefined_selections() {
    for body in [
        // Preserve exact former rejections whose defaults replace known Undefined.
        "const [text='fallback']=[];return text.trim().length;",
        "const [items=[a]]=[];return items.length;",
        "const source=a?[]:[undefined,'ready'];const [x=(source[1]='grown',a),text='fallback']=source;return text.trim().length;",
        "const [text=' x ',row={tag:a},items=[a,b],past=' end ']=[undefined,void 0,,];return text.trim().length+(row.tag===a?10:0)+items.length*100+past.trim().length*1000;",
        "const selected={tag:a};const [row=selected]=[];row.tag=b;return row===selected&&selected.tag===b?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn array_binding_defaults_observe_growth_until_exhaustion_and_advance_elisions() {
    for body in [
        "const source=[];const [x=(source[1]=b,a),y=9]=source;return x===a&&y===9&&source[1]===b?1:0;",
        "const source=[];const [x=(source[1]=b,a),y]=source;return x===a&&y===undefined?1:0;",
        "const source=[,];const [x=(source[1]=b,a),y=9]=source;return x===a&&y===b?1:0;",
        "const source=[undefined];const [x=(source[2]=b,a),middle=7,last=9]=source;return x===a&&middle===7&&last===b?1:0;",
        "const source=[a];const [x=(source[1]=b,1),y=9]=source;return x===a&&y===9&&source.length===1?1:0;",
        "const source=[,];const [,x=(source[2]=b,a),y=9]=source;return x===a&&y===9?1:0;",
        "const source=[,];const [x=(source[2]=b,a),,y=9]=source;return x===a&&y===b?1:0;",
        "const source=[];const [empty,,x=(source[3]=b,a),y=9]=source;return empty===undefined&&x===a&&y===9?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn array_binding_defaults_distinguish_regrowth_before_and_after_iterator_exhaustion() {
    for body in [
        "const source=[undefined,8,9];const [x=(source.length=0,a),y=(source[2]=b,2),z=3]=source;return x===a&&y===2&&z===3&&source[2]===b?1:0;",
        "const source=[undefined,8,9];const [x=(source.length=0,source[2]=b,a),y=2,z=3]=source;return x===a&&y===2&&z===b?1:0;",
        "const source=[undefined,8];const [x=(source.pop(),source.pop(),a),y=(source[2]=b,2),z=3]=source;return x===a&&y===2&&z===3?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn array_binding_defaults_save_the_rhs_and_publish_each_completed_binding() {
    for body in [
        "let source=[undefined,a];const old=source;const [x=(source=[b,b],old[1]=b,a),y]=source;return x===a&&y===b&&source!==old?1:0;",
        "const source=[undefined,a];const alias=source;const [x=(alias[1]=b,a),y=9]=source;return x===a&&y===b?1:0;",
        "const [x=a,y=x,z=(()=>{return y;})()]=[];return x===a&&y===x&&z===y?1:0;",
        "let n=0;const make=()=>{n=n*10+1;return [];};const [x=(n=n*10+2,a),y=(n=n*10+3,x)]=make(),[z=(n=n*10+4,y)]=[];return n+(z===a?10000:0);",
    ] {
        compare(body);
    }
}

#[test]
fn array_binding_defaults_compose_with_helpers_identity_and_capture_cells() {
    for source in [
        "function fallback(x){return x;}export function run(a,b){const [x=fallback(a)]=[];return x===a?1:0;}",
        "function inspect(items,value){const [x=value]=items;return x===value?1:0;}export function run(a,b){return inspect([],a)+inspect([b],a);}",
        "export function run(a,b){const selected={tag:a};const [x=selected]=[];return x===selected?1:0;}",
        "export function run(a,b){let [x=a]=[];const read=()=>x===b?1:0;const change=()=>{x=b;return read();};return change();}",
        "export function run(a,b){const [text=' fallback ']=[' ready '];return text.trim().indexOf('ead');}",
        "export function run(a,b){let first=()=>0;let last=()=>0;for(let [x=1]=[],i=0;i<2;i++,x=2){if(i===0)first=()=>x===1?1:10;last=()=>x===2?1:10;}return first()+last();}",
        "export function run(a,b){let first=()=>0;let second=()=>0;for(let i=0;i<2;i++){let [x=i]=[];const read=()=>x===0?1:2;if(i===0)first=read;else second=read;}return first()*10+second();}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn array_binding_default_annotations_are_erased_without_proving_values() {
    let js = "export function run(a,b){let [x=a,label='fallback']=[];x=b;return x===b&&label==='fallback'?1:0;}";
    let ts = "type Tuple=never;export function run(a:string,b:boolean):void{let [x=a,label='fallback']:Tuple=([] as unknown);x=b;return (x as number)===b&&(label as number)==='fallback'?1:0;}";
    common::compare_sources(js, ts);
    assert_eq!(
        compile(js, false).unwrap().wasm,
        compile(ts, true).unwrap().wasm
    );
    let source =
        "export function run(a,b){const [text='fallback']:[string]=[a];return text.trim().length;}";
    let error = compile(source, true)
        .err()
        .unwrap_or_else(|| panic!("admitted {source}"));
    assert!(error.contains("unsupported"), "{source}: {error}");
    assert!(!error.contains("internal IR"), "{source}: {error}");
}

#[test]
fn array_binding_defaults_keep_tdz_and_conservative_origin_guards() {
    for body in [
        "const [x=x]=[];return a;",
        "const [x=x]=[a];return a;",
        "const [x=y,y=a]=[];return a;",
        "const [x=y,y=a]=[a,b];return a;",
        "const x=a;{const [x=x]=[];return x===a?1:0;}",
        "const [x=(y=a),y=b]=[];return a;",
        "const [f=()=>x,x=a]=[];return a;",
        "const [f=()=>x,x=a]=[0,b];return a;",
        "const [x=undefined,undefined=a]=[];return a;",
        "const [x=a]=[];x=b;return a;",
        "const [text='fallback']=[a];return text.trim().length;",
        "const [text='fallback']=[null];return text.trim().length;",
        "const source=[];const alias=source;alias[0]=a;const [text='fallback']=source;return text.trim().length;",
        "let [text='ready']=[];const read=()=>text.length;text=a;return read();",
        "const source=[undefined,'ready'];const [x=(source.length=0,a),text='fallback']=source;return text.trim().length;",
        "const source=[undefined,'ready'];const alias=source;const [x=(alias.pop(),a),text='fallback']=source;return text.trim().length;",
        "const source=[undefined,8,'ready'];const [x=(source.length=0,a),y=(source[2]='grown',b),text='fallback']=source;return text.trim().length;",
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
    let source = "function drop(items){items.pop();return 0;}export function run(a,b){const source=[undefined,'ready'];const [x=(drop(source),a),text='fallback']=source;return text.trim().length;}";
    for typed in [false, true] {
        let error = compile(source, typed)
            .err()
            .unwrap_or_else(|| panic!("admitted {source}"));
        assert!(error.contains("unsupported"), "{source}: {error}");
        assert!(!error.contains("internal IR"), "{source}: {error}");
    }
}

#[test]
fn array_binding_defaults_keep_local_and_native_stack_bounds_with_elisions() {
    for count in [128, 129] {
        let leaves = (0..count)
            .map(|index| format!("v{index}=a"))
            .collect::<Vec<_>>()
            .join(",,");
        let source = format!(
            "export function run(a,b){{const []=[],[,,,,]=[],[{leaves}]=[];return v0===a&&v127===a?1:0;}}"
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
        "export function run(a,b){{const [{}value=a]=[];return value===a?1:0;}}",
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
        .expect("array binding traversal must not overflow the native stack")
}

fn compare(body: &str) {
    let source = format!("export function run(a,b){{{body}}}");
    common::compare_sources(&source, &source);
}

#[test]
fn array_rest_indices_follow_the_offset_and_keep_undefined_defaults() {
    for body in [
        "const [...rest]=[[a]];const [[x]]=rest;return a;",
        "const src=[a,' one ',{label:' two '}];const [,...mid]=src;const [,...rows]=mid;return mid[0].trim().length+rows[0].label.trim().length;",
        "const src=[a,,'x'];src.named='excluded';const [,...tail]=src;const [first=' fallback ',second]=tail;return first.trim().length+second.length+(tail.named===undefined?10:0);",
        "const src=[a,'x'];src[4294967295]='named';const [,...tail]=src;const [value='default']=[];return tail[4294967294]===undefined&&value==='default'?1:0;",
        "const src=[a,{label:' x '}];const [,...tail]=src;tail[0].label='new';const same=src[1].label.trim()==='new';tail[0]={label:' next '};return same&&src[1]!==tail[0]&&src[1].label.trim()==='new'?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn array_rest_indices_reject_unstable_or_incompatible_projections() {
    for body in [
        "const src=[];const [first=(src[1]='text',a),...rest]=src;return rest[0].trim().length;",
        "const src=[a,'x'];const [,...rest]=src;src.length=0;src[1]='new';return rest[0].trim().length;",
        "const src=[a,'x'];const [,...rest]=src;const alias=src;alias[1]=b;return rest[0].trim().length;",
        "const src=[a,'x'];const [,...rest]=src;rest[0]=b;return rest[0].trim().length;",
        "const src=a?[a,'x']:[a];const [,...rest]=src;return rest[0].trim().length;",
        "const src=[a,'x'];const [,...rest]=src;rest.pop();return rest[0].trim().length;",
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
fn array_rest_pattern_targets_destructure_the_fresh_copy() {
    for body in [
        "const [...[x]]=[a];return a;",
        "const src=[a,undefined,' old '];const [,...[first=(src[2]='new',b),second]]=src;return first===b&&second.trim()==='old'&&src[2]==='new'?1:0;",
        "const [,...[,...[{title},...tail]]]=[a,b,{title:' title '},,a];return title.trim().length+tail.length+(tail[0]===undefined&&tail[1]===a?10:0);",
        "let n=0;const [...[]]=(n++,[a,b]);const [...[x='fallback']]=[];return n+x.length;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn object_rest_targets_read_the_fresh_dense_tail_as_an_array() {
    for body in [
        "const [{x}=[]]=[];return a;",
        "const [...{length}]=[a];return a;",
        "const src=[a,,{label:' heading '}];const [,...{0:first=(src[2]={label:'changed'},1),1:row,length:size,...other}]=src;return first===1&&row.label.trim()==='heading'&&size===2&&Object.keys(other).length===0?1:0;",
        "const src=[,a];src.named=b;const [...{...rest}]=src;const keys=Object.keys(rest);return rest[0]===undefined&&rest[1]===a&&rest.named===undefined&&keys[0]==='0'&&keys[1]==='1'?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}
