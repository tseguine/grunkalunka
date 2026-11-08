use grunkalunka_compiler::compile;

#[path = "managed/common.rs"]
mod common;

#[test]
fn numeric_for_of_result_executes_the_exact_former_guard_source() {
    let source = "export function run(a,b){for(const item of [a])return item+1;return a;}";
    common::compare_sources(source, source);
}

#[test]
fn boxed_number_iteration_cells_support_updates_and_captures() {
    for body in [
        "for(let item of [a])return item++;return b;",
        "let read=()=>a;for(let item of [a]){read=()=>item;item+=b;}return read();",
        "for(let [item] of [[a]])return ++item;return b;",
    ] {
        compare(body);
    }
}

#[test]
fn boxed_number_iteration_cells_support_logical_assignments() {
    for body in [
        "for(let item of [a])return item&&=b;return b;",
        "let read=()=>a;for(let item of [a]){read=()=>item;item||=b;}return read();",
        "for(let [item] of [[a]])return item??=b;return b;",
    ] {
        compare(body);
    }
}

#[test]
fn array_for_of_visits_values_holes_and_empty_arrays() {
    for body in [
        "let n=0;for(const item of [a,,b,undefined])n=n*10+(item===undefined?3:item===a?1:2);return n;",
        "let n=0;for(const item of [])n+=100;for(const item of [,,,])n+=item===undefined?1:10;return n;",
        "let n=0;for(const item of [null,false,0,-0,NaN,''])n=n*2+(item?2:1);return n;",
        "let n=0;for(const item of [null,false,a,'value',{},[]])n+=item?1:0;return n;",
    ] {
        compare(body);
    }
}

#[test]
fn for_in_enumerates_keys_in_ecmascript_order() {
    for body in [
        "const obj={b:a,'1':b,a:3,'0':a};let n=0;for(const key in obj)n=n*10+(key==='0'?1:key==='1'?2:key==='b'?3:4);return n;",
        "const obj=Object.create(null);obj.z=a;obj.a=b;let n=0;for(const key in obj)n=n*10+(key==='z'?1:2);return n;",
        "const items=[a,,b];items.foo=a;let n=0;for(let key in items)n=n*10+(key==='0'?1:key==='2'?2:3);return n;",
        "let n=0;for(const key in 'A💠B')n+=key==='0'?1:key==='1'?2:key==='2'?4:key==='3'?8:0;return n;",
        "let n=0;for(const key in {})n++;for(const key in a)n+=10;return n;",
    ] {
        compare(body);
    }
}

#[test]
fn for_in_retains_rhs_snapshot_and_skips_removed_keys() {
    for body in [
        "const source=[a,b,b];let n=0;for(const key in source){n=n*10+(key==='0'?1:key==='1'?2:3);source.pop();}return n;",
        "const obj={a:1};let n=0;for(const key in obj){n++;obj.b=2;}return n;",
        "let current={x:a,y:b},count=0;const original=current;for(const key in current){count++;current={z:a};}return count*10+(current!==original?1:0);",
        "let n=0;for(const key in {a:1,b:2,c:3}){if(key==='b')continue;n++;if(key==='c')break;}return n;",
    ] {
        compare(body);
    }
}

#[test]
fn for_in_keys_remain_strings_through_helpers_captures_and_let_writes() {
    for body in [
        "let n=0;for(const key in {a:1,bb:2})n=n*10+key.length;return n;",
        "let n=0;for(let key in {' a ':1,' bb ':2}){key=key.trim();n=n*10+key.length;}return n;",
        "let first=()=>'',last=()=>'';for(const key in {a:1,bc:2}){const read=()=>key;if(key==='a')first=read;last=read;}return first().length*10+last().length;",
    ] {
        compare(body);
    }
}

#[test]
fn iteration_assignment_heads_update_existing_mutable_cells() {
    for body in [
        "let key='';let n=0;for(key in {a:1,bb:2})n=n*10+key.length;return n+key.length;",
        "let key='initial';let n=0;for(key in {})n++;return n+key.length;",
        "let key='';const items=[a,b,b];let n=0;for(key in items){n++;items.length=1;}return n*10+(key==='0'?1:0);",
        "let item='';let n=0;for(item of [' a ',' bb '])n=n*10+item.trim().length;return n+item.length;",
        "let item='';for(item of []){}return item.length;",
        "let key='';let n=0;const source={a:1,b:2};for(key in source){n++;if(key==='a')continue;}return n*10+key.length;",
        "let item='';let n=0;for(item of ['a','bb']){n+=item.length;if(item==='a')continue;}return n*10+item.length;",
    ] {
        compare(body);
    }
}

#[test]
fn iteration_assignment_heads_preserve_shared_capture_cells_and_ts_erasure() {
    for body in [
        "let key='';const read=()=>key;let n=0;for(key in {a:1,bb:2})n+=read().length;return n*10+read().length;",
        "let item='';const read=()=>item;let n=0;for(item of ['a','bb'])n+=read().length;return n*10+read().length;",
    ] {
        compare(body);
    }
    common::compare_sources(
        "export function run(a,b){let key='';let n=0;for(key in {a:1,bb:2})n+=key.length;return n;}",
        "export function run(a:number,b:number):number{let key:string='';let n:number=0;for(key in {a:1,bb:2})n+=key.length;return n;}",
    );
    common::compare_sources(
        "export function run(a,b){let item='';let n=0;for(item of ['a','bb'])n+=item.length;return n;}",
        "export function run(a:number,b:number):number{let item:string='';let n:number=0;for(item of ['a','bb'])n+=item.length;return n;}",
    );
}

#[test]
fn iteration_assignment_heads_reject_const_unbound_or_incompatible_slots() {
    for body in [
        "const key='';for(key in {a:1}){}return a;",
        "const item='';for(item of ['a']){}return a;",
        "for(key in {a:1}){}return a;",
        "for(item of ['a']){}return a;",
        "let item=a;for(item of ['a']){}return a;",
        "let key=0;for(key in {a:1}){}return a;",
        "let key='';for(key in null){}return a;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            assert_rejected(&source, typed);
        }
    }
}

#[test]
fn array_for_of_evaluates_one_rhs_and_keeps_the_original_array_and_selection() {
    for body in [
        "let calls=0;const make=()=>{calls++;return [a,b];};let n=0;for(const item of make())n++;return calls*10+n;",
        "let source=[a,b];const original=source;let n=0;for(const item of source){if(n===0){source=[b];original[1]=a;}n=n*10+(item===a?1:2);}return n+(source!==original?100:0);",
        "const source=[a,b];let n=0;for(const item of source){source[0]=b;n=n*10+(item===a?1:2);}return n;",
        "const row={tag:a};let n=0;for(const item of [row,row]){if(item===row)n++;row.tag=b;}return n+(row.tag===b?10:0);",
    ] {
        compare(body);
    }
}

#[test]
fn array_for_of_observes_live_growth_shrinkage_and_sparse_positions() {
    for body in [
        "const source=[a];let n=0,holes=0;for(const item of source){n++;if(item===undefined)holes++;if(n===1)source[2]=b;}return n*10+holes;",
        "const source=[a,b,a];let n=0;for(const item of source){n++;source.length=1;}return n;",
        "const source=[a,b];let n=0;for(const item of source){n++;source.pop();}return n;",
        "const source=[a,b,b];let n=0,holes=0;for(const item of source){n++;if(item===undefined)holes++;if(n===1){source.length=0;source[2]=b;}}return n*10+holes;",
    ] {
        compare(body);
    }
}

#[test]
fn array_for_of_preserves_continue_break_nested_loops_and_return() {
    for body in [
        "let n=0;for(const item of [0,1,2,3]){if(item===1)continue;if(item===3)break;n=n*10+(item===0?1:3);}return n;",
        "let n=0;for(const outer of [0,1]){for(const inner of [0,1,2]){if(inner===1)continue;if(inner===2)break;n++;}if(outer===0)continue;n+=10;}return n;",
        "let n=0;for(const item of [0,1,2]){for(let i=0;i<2;i++){if(i===0)continue;n++;}if(item===1)break;}return n;",
        "for(const item of [a,b]){if(item===a)return 1;else return 2;}return 3;",
    ] {
        compare(body);
    }
}

#[test]
fn array_for_of_allocates_fresh_const_and_mutable_let_capture_cells() {
    for body in [
        "let first=()=>0,last=()=>0;let n=0;for(const item of [0,1]){const read=()=>item===0?1:2;if(n===0)first=read;last=read;n++;}return first()*10+last();",
        "let first=()=>0,last=()=>0,change=()=>0;for(let item of [0,1]){const read=()=>item===0?1:item===1?2:3;if(item===0){first=read;change=()=>{item=9;return 0;};continue;}last=read;}change();return first()*10+last();",
        "let first=()=>0,second=()=>0;let n=0;for(const item of [a,b]){const read=()=>item===a?1:item===b?2:3;if(n===0)first=read;else second=read;n++;}return first()*10+second();",
    ] {
        compare(body);
    }
}

#[test]
fn array_for_of_composes_with_helpers_and_lexical_shadowing() {
    for source in [
        "function count(items){let n=0;for(const item of items)if(item)n++;return n;}export function run(a,b){return count([a,b]);}",
        "function select(items,needle){for(const item of items){if(item===needle)return 1;}return 0;}export function run(a,b){return select([a,b],b);}",
        "function identity(x){return x;}export function run(a,b){const row={tag:a};let n=0;for(const item of [row,b])if(identity(item)===row)n++;return n;}",
        "export function run(a,b){const items=[a,b];const count=()=>{let n=0;for(const item of items)n+=item?1:0;return n;};return count();}",
        "export function run(a,b){const item=9;let n=0;for(let item of [a,b]){item=7;n+=item===7?1:0;}return item+n;}",
        "export function run(a,b){const item=7;let n=0;for(const item of [a]){{const item=b;n+=item===b?1:0;}n+=item===a?2:0;}return n+(item===7?10:0);}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn array_for_of_retains_proven_string_record_and_nested_array_operations() {
    for body in [
        "for(const item of ['ready'])return item.trim().length;return a;",
        "for(const item of [{value:a}])return item.value===a?1:0;return a;",
        "let n=0;for(const item of [' a ',' bb '])n=n*10+item.trim().length;return n;",
        "const row={label:' q ',values:[a,b]};let n=0;for(const item of [row,{label:' zz ',values:[b,a]}]){const {label,values}=item;n=n*10+label.trim().length+(values.at(0)===a?3:0);}return n;",
        "let n=0;for(const values of [[' a '],[],[' bb ',' ccc ']]){n+=values.length;for(const item of values)n=n*10+item.trim().length;}return n;",
    ] {
        compare(body);
    }
}

#[test]
fn array_for_of_proves_values_through_helpers_captures_and_empty_array_branches() {
    for source in [
        "function values(){return [' a ',' bb '];}function identity(x){return x;}export function run(a,b){let n=0;for(const item of values())n+=identity(item).trim().length;return n;}",
        "function inspect(items){let n=0;for(const item of items)n+=item.trim().length;return n;}export function run(a,b){return inspect(a?[' x ',' yy ']:[' z ']);}",
        "export function run(a,b){let n=0;for(const item of (a?[]:[' x ']))n+=item.trim().length;return n;}",
        "export function run(a,b){const items=[{label:' ab ',value:a}];const select=()=>items;let n=0;for(const row of select())n+=row.label.trim().length+(row.value===a?1:0);return n;}",
        "export function run(a,b){let n=0;for(const item of Object.keys({ready:a,other:b}))n=n*10+(item==='ready'?1:item==='other'?2:9);return n;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn array_for_of_keeps_proven_values_in_fresh_and_mutable_capture_cells() {
    for body in [
        "let first=()=>0,last=()=>0;let n=0;for(const item of [' a ',' bb ']){const read=()=>item.trim().length;if(n===0)first=read;last=read;n++;}return first()*10+last();",
        "let first=()=>0,last=()=>0,change=()=>0;let n=0;for(let item of [' a ',' bb ']){const read=()=>item.trim().length;if(n===0){first=read;change=()=>{item=' ccc ';return 0;};n++;continue;}last=read;}change();return first()*10+last();",
        "let first=()=>0,last=()=>0,change=()=>0;let n=0;for(let item of [{label:' a '},{label:' bb '}]){const read=()=>item.label.trim().length;if(n===0){first=read;change=()=>{item={label:' ccc '};return 0;};}last=read;n++;}change();return first()*10+last();",
        "let n=0;for(let values of [[a],[b,a]]){values=[b];n+=values.length+(values.at(0)===b?10:0);}return n;",
    ] {
        compare(body);
    }
}

#[test]
fn array_for_of_keeps_proofs_for_safe_replacements_and_named_properties() {
    for source in [
        "export function run(a,b){const items=[' a ',' bb '];items[0]=' ccc ';let n=0;for(const item of items){items[1]=' dddd ';n=n*10+item.trim().length;}return n;}",
        "export function run(a,b){const original={label:' a '};const items=[original,{label:' bb '}];const alias=items;let n=0;for(const item of items){alias[0]={label:' ccc '};alias[1]={label:' dddd '};n=n*10+item.label.trim().length+(item===original?100:0);}return n;}",
        "function replace(items){items[1]=' ccc ';return items;}export function run(a,b){const items=[' a ',' bb '];const alias=replace(items);let n=0;for(const item of alias)n=n*10+item.trim().length;return n;}",
        "export function run(a,b){const items=[' a ',' bb '];items.named=a;let n=0;for(const item of items)n=n*10+item.trim().length;return n;}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn array_for_of_preserves_proven_member_callbacks_and_their_signatures() {
    for source in [
        "function identity(x){return x;}export function run(a,b){let n=0;for(const row of [{f:identity},{f:x=>x}])n+=row.f(a)===a?1:0;return n;}",
        "export function run(a,b){let n=0;for(const callbacks of [[()=>a],[()=>b]])n+=callbacks[0]()===a?1:2;return n;}",
        "export function run(a,b){let first=()=>0,last=()=>0;let n=0;for(const row of [{f:()=>1},{f:()=>2}]){const read=()=>row.f();if(n===0)first=read;last=read;n++;}return first()*10+last();}",
        "export function run(a,b){let n=0;for(const item of [()=>a,()=>b]){const box={f:item};n=n*10+(box.f()===a?1:2);}return n;}",
    ] {
        common::compare_sources(source, source);
    }
    for body in [
        "let n=0;for(const row of [{f:()=>a},{f:x=>x}])n+=row.f();return n;",
        "let n=0;for(const row of [{f:()=>a},{f:b}])n+=row.f();return n;",
        "for(const row of [{f:()=>a}]){row.f=b;return row.f();}return 0;",
        "for(const callbacks of [[()=>a],[]])return callbacks[0]();return 0;",
        "for(const item of [()=>a])return item();return 0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            assert_rejected(&source, typed);
        }
    }
}

#[test]
fn array_for_of_rejects_tainted_or_unproven_specialized_values() {
    for body in [
        "let n=0;for(const item of ['ready',a])n+=item.trim().length;return n;",
        "let n=0;for(const item of ['ready',,])n+=item.trim().length;return n;",
        "let n=0;for(const item of ['ready',undefined])n+=item.trim().length;return n;",
        "let n=0;for(const item of [{label:' x '},{}])n+=item.label.trim().length;return n;",
        "const items=['ready'];items[0]=a;for(const item of items)return item.trim().length;return 0;",
        "const items=['ready'];const alias=items;alias[0]=a;for(const item of items)return item.trim().length;return 0;",
        "const items=['ready'];items[2]='later';let n=0;for(const item of items)n+=item.trim().length;return n;",
        "const items=['ready'];items.length=2;let n=0;for(const item of items)n+=item.trim().length;return n;",
        "const items=['ready'];items[a]='later';let n=0;for(const item of items)n+=item.trim().length;return n;",
        "for(let item of ['ready']){item=a;return item.trim().length;}return 0;",
        "for(let item of ['ready']){const read=()=>item.trim().length;item=a;return read();}return 0;",
        "for(let item of [{label:'ready'}]){item=a;return item.label.trim().length;}return 0;",
        "const items=[{label:' x '}];const row=items[0];row.label=a;for(const item of items)return item.label.trim().length;return 0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            assert_rejected(&source, typed);
        }
    }
    for source in [
        "function identity(x){return x;}export function run(a,b){const items=['ready'];identity(items)[0]=a;for(const item of items)return item.trim().length;return 0;}",
        "function replace(items,value){items[0]=value;return items;}export function run(a,b){for(const item of replace(['ready'],a))return item.trim().length;return 0;}",
    ] {
        for typed in [false, true] {
            assert_rejected(source, typed);
        }
    }
}

#[test]
fn array_for_of_erases_annotations_without_proving_runtime_values() {
    let js = "export function run(a,b){let n=0;for(let item of [a,b]){item=a;n+=item===a?1:0;}return n;}";
    let ts = "type Items=never;export function run(a:string,b:boolean):void{let n:number=0;for(let item of ([a,b] as Items)){item=a;n+=(item as number)===a?1:0;}return n;}";
    common::compare_sources(js, ts);
    assert_eq!(
        compile(js, false).unwrap().wasm,
        compile(ts, true).unwrap().wasm
    );
    let js = "export function run(a,b){let n=0;for(const item of [{label:' x '},{label:' yy '}])n+=item.label.trim().length;return n;}";
    let ts = "type Items=never;export function run(a:string,b:boolean):void{let n:number=0;for(const item of ([{label:' x '},{label:' yy '}] as Items))n+=(item.label as number).trim().length;return n;}";
    common::compare_sources(js, ts);
    assert_eq!(
        compile(js, false).unwrap().wasm,
        compile(ts, true).unwrap().wasm
    );
    for source in [
        "export function run(a,b){const items:number[]=a;for(const item of items){return item===a?1:0;}return 0;}",
        "export function run(a,b){for(const item of ([a] as string[])){return item.trim().length;}return 0;}",
    ] {
        assert_rejected(source, true);
    }
}

#[test]
fn array_for_of_keeps_header_tdz_and_managed_value_admission_boundaries() {
    for body in [
        "let item=a;for(item of [b]){}return a;",
        "const row={value:a};for(row.value of [b]){}return a;",
        "for(const item of a){}return a;",
        "const source=a?'abc':[b];for(const item of source){}return a;",
        "for(const [first] of 'abc'){}return a;",
        "for(let item of 'abc'){item=a;return item.trim().length;}return 0;",
        "for(const item of {0:a,length:1}){}return a;",
        "for(const item of null){}return a;",
        "for(const item of undefined){}return a;",
        "const items=a?[a]:null;for(const item of items){}return a;",
        "for(const item of [item]){}return a;",
        "const items=[a];for(const items of items){}return a;",
        "let saved=()=>0;for(const item of (saved=()=>item,[a])){}return saved()===a?1:0;",
        "for(const item of [a]){}return item;",
        "for(const item of [a])item=b;return a;",
        "outer:for(const item of [a]){break outer;}return a;",
        "for(var key in {item:a}){}return a;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            assert_rejected(&source, typed);
        }
    }
}

#[test]
fn array_for_of_keeps_source_local_and_nesting_limits_on_a_small_native_stack() {
    for count in [128, 129] {
        let locals = (0..count - 1)
            .map(|index| format!("v{index}=a"))
            .collect::<Vec<_>>()
            .join(",");
        let source = format!(
            "export function run(a,b){{const {locals};for(const item of [a])return item===a?1:0;return b;}}"
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
    for (depth, admitted) in [(1, true), (16, true), (64, true), (65, false)] {
        let mut body = String::from("break;");
        for level in 0..depth {
            body = format!("for(const item{level} of [a]){body}");
        }
        let source = format!("export function run(a,b){{{body}return b;}}");
        for typed in [false, true] {
            let result = compile_on_small_stack(source.clone(), typed);
            if admitted {
                assert!(result.is_ok(), "{depth}: {result:?}");
            } else {
                let error = result.unwrap_err();
                assert!(error.contains("statement nesting exceeds 64"), "{error}");
            }
        }
    }
}

fn compile_on_small_stack(source: String, typed: bool) -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || compile(&source, typed).map(|_| ()))
        .unwrap()
        .join()
        .expect("array for-of must not overflow the native stack")
}

fn assert_rejected(source: &str, typed: bool) {
    let error = compile(source, typed)
        .err()
        .unwrap_or_else(|| panic!("admitted {source}"));
    assert!(error.contains("unsupported"), "{source}: {error}");
    assert!(!error.contains("internal IR"), "{source}: {error}");
}

fn compare(body: &str) {
    let source = format!("export function run(a,b){{{body}}}");
    common::compare_sources(&source, &source);
}

#[test]
fn destructuring_for_of_reads_records_arrays_and_nested_values() {
    for body in [
        "for(const [item] of [[a]]){}return a;",
        "for(const {item} of [{item:a}]){}return a;",
        "let n=0;for(const {label,value} of [{label:' a ',value:a},{label:' bb ',value:b}])n=n*10+label.trim().length+(value===a?3:0);return n;",
        "let n=0;for(const [label,,value] of [[' a ',0,a],[' bb ',0,b]])n=n*10+label.trim().length+(value===a?3:0);return n;",
        "let n=0;for(const {row:{label},flags:[flag]} of [{row:{label:' a '},flags:[a]},{row:{label:' bb '},flags:[b]}])n=n*10+label.trim().length+(flag===a?3:0);return n;",
        "let n=0;for(const {} of [{},{}])n++;for(const [] of [[],[a]])n+=10;return n;",
    ] {
        compare(body);
    }
    let source = "function count(rows){let n=0;for(const {label} of rows)n+=label.trim().length;return n;}export function run(a,b){return count([{label:' a '},{label:' bb '}]);}";
    common::compare_sources(source, source);
}

#[test]
fn destructuring_for_of_orders_defaults_and_keeps_original_selections() {
    for body in [
        "const inner=[];const rows=[inner,inner];let n=0;for(const [x=(inner[0]=7,1),y] of rows)n=n*10+(x===7?2:1)+(y===undefined?0:5);return n;",
        "const rows=[{first:undefined,next:a},{first:undefined,next:b}];let n=0;for(const {first=(rows[1].next=a,1),next} of rows)n=n*10+(first===1?1:0)+(next===a?2:0);return n;",
        "let rows=[{value:a},{value:b}];const original=rows;let calls=0,n=0;for(const {value} of (calls++,rows)){rows=[{value:0}];n=n*10+(value===a?1:2);}return calls*100+n+(rows!==original?1000:0);",
        "let n=0;for(const {first=a,next=first} of [{},{first:null},{first:false}])n=n*10+(first===next?1:2);return n;",
        "const row={inner:{first:undefined,next:a},tail:a};let n=0;for(const {inner:{first=(row.inner={next:b},row.tail=b,1),next},tail} of [row])n=(first===1?100:0)+(next===a?10:0)+(tail===b?1:0);return n;",
    ] {
        compare(body);
    }
}

#[test]
fn whole_pattern_defaults_in_for_of_heads_keep_laziness_and_fresh_captures() {
    for body in [
        "let calls=0,n=0;for(const [{label}=(calls++,{label:' x '})] of [[],[{label:' yy '}]])n=n*10+label.trim().length;return calls*100+n;",
        "let first=()=>0,last=()=>0,change=()=>0;let n=0;for(let {row:{label}=(n++,{label:n===1?' a ':' bb '}),unused=(last=()=>label.trim().length,0)} of [{},{}]){if(n===1){first=last;change=()=>{label=' ccc ';return 0;};}}change();return first()*10+last();",
        "let first=()=>0,last=()=>0;for(let {row:{label}={label:' a '}}={},i=0;i<2;i++,label=' bb '){if(i===0)first=()=>label.trim().length;last=()=>label.trim().length;}return first()*10+last();",
    ] {
        compare(body);
    }
}

#[test]
fn array_rest_heads_keep_fresh_captures_and_support_identifier_iteration() {
    for body in [
        // Preserve the exact former rejection for an array-rest loop head.
        "for(const [first,...rest] of [[a,b]]){}return a;",
        "const [,...rest]=[a,b,undefined];let n=0;for(const value of rest)n=n*10+(value===undefined?2:1);return n;",
        "let first=()=>0,last=()=>0,change=()=>0;let n=0;for(let {items:[head,...rest]=[a,b],after=(last=()=>+rest.length,0)} of [{},{items:[a,b,a]}]){if(n===0){first=last;change=()=>{rest=[a,a,a];return 0;};}n++;}change();return +first()*10+(+last());",
        "let first=()=>0,last=()=>0;for(let [...tail]=[a],i=0;i<2;i++,tail=[a,b]){if(i===0)first=()=>+tail.length;last=()=>+tail.length;}return +first()*10+(+last());",
    ] {
        compare(body);
    }
}

#[test]
fn destructuring_for_of_allocates_fresh_leaf_cells_and_preserves_loop_control() {
    for body in [
        "let first=()=>0,last=()=>0,change=()=>0;let n=0;for(let {label,unused=(last=()=>label.trim().length,0)} of [{label:' a '},{label:' bb '}]){if(n===0){first=last;change=()=>{label=' ccc ';return 0;};}n++;}change();return first()*10+last();",
        "let first=()=>0,last=()=>0;let n=0;for(const {label} of [{label:' a '},{label:' bb '}]){const read=()=>label.trim().length;if(n===0)first=read;last=read;n++;}return first()*10+last();",
        "let first=()=>0,last=()=>0,change=()=>0;let n=0;for(let [label] of [[' a '],[' bb ']]){const read=()=>label.trim().length;if(n===0){first=read;change=()=>{label=' ccc ';return 0;};n++;continue;}last=read;}change();return first()*10+last();",
        "let n=0;for(const {value} of [{value:0},{value:1},{value:2}]){try{if(value===0)continue;break;}finally{n=n*10+(value===0?1:2);}}return n;",
        "for(const [value] of [[a],[b]]){if(value===a)return 1;return 2;}return 3;",
    ] {
        compare(body);
    }
}

#[test]
fn destructuring_for_of_erases_types_and_keeps_unknowns_and_tdz_rejected() {
    let js = "export function run(a,b){let n=0;for(const {row:{label},values:[value]} of [{row:{label:' x '},values:[a]}])n+=label.trim().length+(value===a?1:0);return n;}";
    let ts = "type Rows=never;export function run(a:string,b:boolean):void{let n:number=0;for(const {row:{label},values:[value]} of ([{row:{label:' x '},values:[a]}] as Rows))n+=(label as number).trim().length+((value as string)===a?1:0);return n;}";
    common::compare_sources(js, ts);
    assert_eq!(
        compile(js, false).unwrap().wasm,
        compile(ts, true).unwrap().wasm
    );
    for body in [
        "for(const {value} of [a]){}return a;",
        "for(const [value] of [[a],null]){}return a;",
        "for(const {nested:{value}} of [{nested:{value:a}},{}]){}return a;",
        "const rows=[{nested:{value:a}}];const alias=rows;alias[0].nested=b;for(const {nested:{value}} of rows){}return a;",
        "const rows=[{value:a}];rows.length=2;for(const {value} of rows){}return a;",
        "for(const {value} of [{value}]){}return a;",
        "for(const {value=value} of [{}]){}return a;",
        "for(const {first=next,next=a} of [{}]){}return a;",
        "for(const [value] of [[a]]){}return value;",
        "for(const {value} of [{value:a}])value=b;return a;",
        "for(var {value} of [{value:a}]){}return a;",
        "for(const [...rest] of [[a],null]){}return a;",
        "for(const [...rest] of [[a]])rest=[];return a;",
        "for(const [head=rest,...rest] of [[]]){}return a;",
        "for(const {row:{x}={x:a}} of [{row:null}]){}return a;",
        "for(const [[x]=[a]] of [[b]]){}return a;",
        "for(const {row:{x}={x}} of [{}]){}return a;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            assert_rejected(&source, typed);
        }
    }
}

#[test]
fn destructuring_for_of_counts_each_leaf_against_the_source_local_budget() {
    for count in [128, 129] {
        let names = (0..count)
            .map(|index| format!("v{index}"))
            .collect::<Vec<_>>();
        let pattern = names.join(",");
        let fields = names
            .iter()
            .map(|name| format!("{name}:a"))
            .collect::<Vec<_>>()
            .join(",");
        let source = format!(
            "export function run(a,b){{for(const {{{pattern}}} of [{{{fields}}}])return v0===a?1:0;return b;}}"
        );
        for typed in [false, true] {
            let result = compile_on_small_stack(source.clone(), typed);
            if count == 128 {
                assert!(result.is_ok(), "{result:?}");
            } else {
                assert!(result.unwrap_err().contains("at most 128 source locals"));
            }
        }
    }
}

#[test]
fn array_rest_iteration_preserves_proven_strings_records_and_inner_arrays() {
    for body in [
        "const [...rest]=['x'];for(const item of rest)return item.trim().length;return 0;",
        "const source=[a,' one ',' two '];source[0]=b;const [,...rest]=source;let n=0;for(const item of rest)n+=item.trim().length;return n;",
        "const source=[a,{label:' one '},{label:' two '}];const [header,...rows]=source;let n=0;for(const {label} of rows)n+=label.trim().length;return n+(header===a?10:0);",
        "const source=[,a,[a,b],[b,a]];const [,...middle]=source;const [,...tail]=middle;let n=0;for(const [x,y] of tail)n+=(x===a?1:0)+(y===b?10:0);return n;",
        "const source=a?[0,' first ',' next ']:[1];const [,...tail]=source;let n=0;for(const item of tail)n+=item.trim().length;return n;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn tail_iteration_rejects_holes_incompatible_writes_and_opaque_sources() {
    for body in [
        "const [,...tail]=[0,,'x'];for(const item of tail)return item.trim().length;return 0;",
        "const source=[a,'x'];const alias=source;alias[1]=b;const [,...tail]=source;for(const item of tail)return item.trim().length;return 0;",
        "const [,...tail]=[a,'x'];tail.length=3;for(const item of tail)return item.trim().length;return 0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            assert_rejected(&source, typed);
        }
    }
}

#[test]
fn array_rest_pattern_loop_heads_allocate_fresh_captured_leaves() {
    for body in [
        "for(const [...[tail]] of [[a]]){}return a;",
        "let first=()=>0,last=()=>0,n=0;for(let [,...[label]] of [[a,' first '],[b,' last ']]){const read=()=>label.trim().length;if(n===0)first=read;last=read;n++;}return first()*10+last();",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn object_keys_iterations_use_string_names_and_guard_indexed_misses() {
    for body in [
        "for(const item of Object.keys({ready:a}))return item.trim().length;return 0;",
        "const [...tail]=Object.keys('x');for(const item of tail)return item.trim().length;return 0;",
        "const record={first:a,second:b};let n=0;for(const key of Object.keys(record))n+=key.trim().length;return n;",
        "const k=a?' first ':' second ';const o={[k]:b};const [key=' empty ']=Object.keys(o);return key.trim().length;",
        "const [missing=' fallback ']=Object.keys({});return missing.trim().length;",
        "const keys=Object.keys({x:a});const alias=keys;alias.named=b;let n=0;for(const key of alias)n+=key.trim().length;return n;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn object_keys_consumers_reject_overwrites_holes_and_unchecked_indices() {
    for body in [
        "const keys=Object.keys({x:a});const alias=keys;alias[0]=b;for(const name of keys)return name.trim().length;return 0;",
        "const keys=Object.keys({x:a});keys.length=2;for(const name of keys)return name.trim().length;return 0;",
        "const keys=Object.keys({});return keys[0].trim().length;",
        "const keys=Object.keys({x:a});keys[0]=b;const [key='fallback']=keys;return key.trim().length;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            assert_rejected(&source, ts);
        }
    }
}

#[test]
fn string_for_of_yields_code_points_and_preserves_unpaired_surrogates() {
    for body in [
        "for(const item of 'abc'){}return a;",
        r"let n=0;for(const ch of 'A\u{1f600}\ud800x\udc00'){if(ch==='A')n=n*10+1;else if(ch==='\u{1f600}')n=n*10+2;else if(ch==='\ud800')n=n*10+3;else if(ch==='x')n=n*10+4;else if(ch==='\udc00')n=n*10+5;else return -1;}return n;",
        r"let n=0;for(const ch of '\ud800\ud801\udc01\udc02\udbff\udfff\ud800')n=n*3+ch.length;return n;",
        r"let text='A\u{1f600}B',n=0;for(const ch of text){text='rebound';if(ch==='A')continue;n=n*10+ch.length;}return n;",
        "let n=0;for(const ch of (n=1,''))n=2;return n;",
        r"let first=()=>0,last=()=>0,n=0;for(let ch of 'x\u{1f600}'){const get=()=>ch.length;if(n===0)first=get;last=get;n++;}return first()*10+last();",
        r"const obj={label:a?'A\u{1f600}':' B '};let n=0;for(const ch of obj.label.trim()){if(ch==='B')break;n+=ch.length;}return n;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn var_for_of_assigns_one_hoisted_cell_and_retains_zero_visit_values() {
    common::compare_sources(
        "export function run(a,b){for(var item of [a]){}return a;}",
        "export function run(a,b){for(var item of [a]){}return a;}",
    );
    for body in [
        "let n=0;for(var item of [a,b])n+=+item;return n;",
        "const before=item;for(var item of []){}return before===undefined?1:0;",
        "var item=3;for(var item of []){}return +item;",
        "for(var item of [a,b]){}var item;return +item;",
        "var a;for(var a of [null,b]){}return +a;",
        "const f=()=>a;for(var a of [null,b]){}return +f();",
        "let n=0;for(var item of [a,b]){if(item===a)continue;n++;}return n;",
        "for(var item of [a,b])break;return +item;",
        "let n=0;for(var item of [a,b]){for(var item of [1,2])n+=+item;}return n+(+item);",
        "let n=0;for(var item of 'a\u{1f600}b')n+=item==='a'?1:item==='b'?2:3;return n;",
        "const f={a:()=>item,b:()=>item};let n=0;for(var item of [1,2]){if(n++===0)f.a=()=>item;else f.b=()=>item;}return +f.a()+(+f.b());",
    ] {
        compare(body);
    }
}
