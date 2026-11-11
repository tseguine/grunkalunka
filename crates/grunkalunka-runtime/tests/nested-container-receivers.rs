#[path = "managed/common.rs"]
mod common;

fn compare(body: &str) {
    let source = format!("export function run(a,b){{{body}}}");
    common::compare_sources(&source, &source);
}

#[test]
fn loop_tests_guard_receivers_in_the_body_and_for_update() {
    for body in [
        "const row=a?{child:{v:b}}:null;let i=0,n=0;while(row&&i<2){n+=+row.child.v;i++;}return n;",
        "const xs=a?[b,b]:null;let n=0;for(;xs&&n<3;n+=xs.length){continue;}return n;",
        "const xs=a?[b,b]:{n:b};let n=0;for(;Array.isArray(xs);){n=xs.length;break;}return n;",
    ] {
        compare(body);
    }
    for body in [
        "const row=a?{v:b}:null;do{return +row.v;}while(row);return 0;",
        "const row=a?{v:b}:null;for(let n=+row.v;row;){return n;}return 0;",
        "const row=a?{v:b}:null;while(row){break;}return +row.v;",
        "let row=a?{v:b}:null;while(row){row=null;return +row.v;}return 0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(
            grunkalunka_compiler::compile(&source, false).is_err(),
            "{source}"
        );
    }
}

#[test]
fn switch_cases_narrow_receivers_only_when_the_label_decides_entry() {
    for body in [
        "const v=a?'abc':{n:b};switch(typeof v){case 'string':return v.length;default:return +v.n;}",
        "const v=a?{n:b}:(b?null:undefined);switch(v){default:return +v.n;case null:return 2;case undefined:return 3;}",
        "const v=a?'xy':b;let n=0;switch(typeof v){case 'number':n=v+1;break;case 'string':n=v.length;break;}return n;",
    ] {
        compare(body);
    }
    for body in [
        "const v=a?'x':b;switch(typeof v){case 'number':case 'string':return v.length;}return 0;",
        "const v=a?{n:b}:null;switch(v){case null:default:return +v.n;}",
        "const v=a?{n:b}:null;switch(typeof v){case 'object':return +v.n;default:return 7;}",
        "let v=a?'x':b;switch(typeof v){case 'string':v=b;return v.length;default:return 0;}",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(
            grunkalunka_compiler::compile(&source, false).is_err(),
            "{source}"
        );
    }
}

#[test]
fn const_guard_aliases_preserve_runtime_facts_about_unchanged_receivers() {
    for body in [
        "const row=a?{n:b}:null,ready=row!=null;if(!ready)return 7;return +row.n;",
        "const v=a?[a,b]:{n:b},isArray=Array.isArray(v),other=!isArray;return other?+v.n:v.length;",
        "const row=a?{n:b}:null,xs=b?[a,b]:null;const hasRow=!!row,ready=hasRow&&xs;return ready?+row.n+xs.length:7;",
    ] {
        compare(body);
    }
    for body in [
        "let row=a?{n:b}:null;const ok=!!row;row=null;if(ok)return +row.n;return 7;",
        "const row=a?{n:b}:null;let ok=!!row;ok=true;if(ok)return +row.n;return 7;",
        "const row=a?{n:b}:null,ok=!!row;{const ok=!!b;if(ok)return +row.n;}return 7;",
        "const row=a?{n:b}:null,ok=!!row||b;if(ok)return +row.n;return 7;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(
            grunkalunka_compiler::compile(&source, false).is_err(),
            "{source}"
        );
    }
}

#[test]
fn strict_literal_matches_establish_only_the_matched_kind() {
    for body in [
        "const v=a?'ok':{n:b};if(v==='ok')return v.charCodeAt(0);return 3;",
        "const v=a?'x':{n:b};return 'x'!==v?7:v.length;",
        "const v=a?b:{n:b};if(-7===v)return v+1;return 0;",
        "const v=a?true:{n:b};if(v===true)return +v;return 0;",
        "const v=a?'ok':{n:b};switch(v){case 'ok':return v.length;default:return b;}",
    ] {
        compare(body);
    }
    for body in [
        "const v=a?'x':null;if(v!=='y')return v.length;return 0;",
        "const v=a?'x':null;if(v=='x')return v.length;return 0;",
        "const v=a?'12':{n:b};switch(v){case 'ok':return 0;default:return v.length;}",
        "const v=a?2:{n:b};if(v!==1)return v+1;return 0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(
            grunkalunka_compiler::compile(&source, false).is_err(),
            "{source}"
        );
    }
}

#[test]
fn nested_object_fields_can_act_as_receivers() {
    let source = "export function run(a,b){const o={child:{x:a}};return o.child.x===a?1:0;}";
    common::compare_sources(source, source);
}

#[test]
fn abrupt_guard_arms_narrow_only_the_rest_of_the_current_statement_list() {
    for body in [
        "const xs=a?[b,b]:null;let n=0;for(let i=0;i<2;i++){if(!xs)continue;n+=xs.length;}return n;",
        "const row=a?{v:b}:null;let n=0;while(n<2){if(row){}else{break;}return +row.v;}return n;",
        "const row=a?{v:b}:null;switch(b){case 0:if(!row)break;return +row.v;default:return 7;}return 3;",
    ] {
        compare(body);
    }
    for body in [
        "const row=a?{v:b}:null;for(let i=0;i<2;i++){if(!row)break;}return +row.v;",
        "const row=a?{v:b}:null;switch(b){case 0:if(!row)break;case 1:return +row.v;}return 3;",
        "const row=a?{v:b}:null;while(a){if(!row){while(b){break;}}return +row.v;}return 7;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(
            grunkalunka_compiler::compile(&source, false).is_err(),
            "{source}"
        );
    }
}

#[test]
fn nested_object_array_combinations_keep_original_values() {
    for body in [
        "const o={child:[a,b]};return o.child[0]===a?1:0;",
        "const o=[{x:a}];return o[0].x===a?1:0;",
        "const o=[[a,b]];return o[0][0]===a?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
}

#[test]
fn writes_keep_aliases_saved_children_and_rhs_receiver_snapshots() {
    for body in [
        "const o={child:{x:a}};const p=o.child;p.x=b;return o.child.x===b?1:0;",
        "const o={child:[a]};const p=o.child;p[0]=b;return o.child[0]===b?1:0;",
        "const old={x:a},next={x:a},o={child:old};o.child.x=(o.child=next,b);return old.x===b&&next.x===a&&o.child===next?1:0;",
        "const old=[a],next=[a],o={child:old};o.child[0]=(o.child=next,b);return old[0]===b&&next[0]===a&&o.child===next?1:0;",
        "const o={child:{x:a}};const old=o.child;o.child={x:b};return old!==o.child&&old.x===a&&o.child.x===b?1:0;",
        "const o={child:{x:a}};return(o.child={x:b}).x===b?1:0;",
        "const o={child:{}};o.child.parent=o;return o.child.parent===o?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn nested_references_pass_through_helpers_factories_and_captures() {
    for body in [
        "const use=x=>x.x===a?1:0;const o={child:{x:a}};return use(o.child);",
        "const id=x=>x;const o={child:{x:a}};const p=id(o.child);return p.x===a?1:0;",
        "const make=x=>()=>x;const o={child:[a,b]};const f=make(o.child);const p=f();return p[0]===a&&p===o.child?1:0;",
        "const o={child:{x:a}};const f=()=>o.child.x===a?1:0;const holder={f};return f();",
        "const make=n=>({child:{x:n}});const o=make(a);return o.child.x===a?1:0;",
    ] {
        compare(body);
    }
    common::compare_sources(
        "function read(x,n){return x[0]===n?1:0;}export function run(a,b){const o={child:[a,b]};return read(o.child,a);}",
        "function read<T>(x:string,n:boolean):never{return x[0]===n?1:0;}export function run(a:string,b:boolean):void{const o={child:[a,b]};return read<Missing>(o.child as never,a);}",
    );
}

#[test]
fn nested_array_lengths_and_function_fields_remain_live() {
    for body in [
        "const o={child:[a,b]};return o.child.length;",
        "const o=[[a,b]];o[0].length=1;return o[0].length+(o[0][0]===a?1:0);",
        "const o={child:[()=>a]};o.child.length=3;return o.child[0]();",
        "const o={child:{f:()=>a}};return o.child.f();",
    ] {
        compare(body);
    }
}

#[test]
fn nested_receivers_and_callees_preserve_effect_order() {
    for body in [
        "let n=0;const o={child:{x:a}};(n++,o.child).x=(n++,b);return n===2&&o.child.x===b?1:0;",
        "let n=0;const o={child:{f:x=>x}};return (n++,o.child).f((n++,a),(n++,b))+n;",
        "const o={child:{f:x=>x}};return o.child.f((o.child={f:x=>b},a));",
        "let n=0;const o={child:{f:x=>x}};return o.child.f.call((n++,b),(n++,a),(n++,b))+n;",
        "const o={child:{f:x=>x}};const apply=(fn,x)=>fn(x);const value={};return apply(o.child.f,value)===value?1:0;",
        "let n=0;const pick=x=>x;const o={child:{x:a}};return pick.call((n++,b),(n++,o.child)).x===a?n:0;",
        "const o={'😀':{x:a}};return o['😀'].x===a?1:0;",
        "let n=0;const o={child:(n++,null),child:{x:(n++,a)}};return n===2&&o.child.x===a?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn nested_receivers_erase_types_without_changing_representation() {
    let js = "export function run(a,b){const o={child:[{x:a}]};o.child[0].x=b;return o.child[0].x===b?1:0;}";
    let ts = "type Unused=Missing;export function run(a:boolean,b:string):void{const o={child:[{x:a}]};(o.child as unknown)[(0 as never)]!.x=b;return(o.child[0]as Unknown).x===b?1:0;}";
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm
    );
    common::compare_sources(js, ts);
    let js = "export function run(a,b){const left={child:{x:a}},right={child:{x:b}};let pick=()=>left.child;pick&&=()=>right.child;const o={child:pick()};return o.child.x===b?1:0;}";
    let ts = "export function run(a,b){const left={child:{x:a}},right={child:{x:b}};let pick=()=>left.child;(pick as unknown)&&=()=>right.child;const o={child:pick()};return o.child.x===b?1:0;}";
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm
    );
    common::compare_sources(js, ts);
}

#[test]
fn nested_receiver_queries_keep_source_and_two_megabyte_stack_bounds() {
    for (aliases, admitted) in [(24, true), (129, false)] {
        let names = (1..=aliases)
            .map(|i| format!("const o{i}=o{};", i - 1))
            .collect::<String>();
        let literal = format!("{}{{x:a}}{}", "{child:".repeat(8), "}".repeat(8));
        let path = "child.".repeat(8);
        let source = format!(
            "export function run(a,b){{const o0={literal};{names}return o{aliases}.{path}x===a?1:0;}}"
        );
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || grunkalunka_compiler::compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(result.is_ok(), admitted, "{:?}", result.as_ref().err());
        if let Err(error) = result {
            assert!(error.contains("unsupported"), "{error}");
        }
    }
}

#[test]
fn logical_binding_updates_cannot_hide_non_container_alternatives() {
    let source = "export function run(a,b){const left={child:{x:a}},right={child:[b]};let pick=()=>left.child;pick&&=()=>right.child;const o={child:pick()};return o.child.x===a?1:0;}";
    for ts in [false, true] {
        assert!(
            grunkalunka_compiler::compile(source, ts).is_err(),
            "unexpected admission: {source}"
        );
    }
    for body in [
        "const left={child:{x:a}},right={child:{x:b}};let pick=()=>left.child;pick&&=()=>right.child;const o={child:pick()};return o.child.x===b?1:0;",
        "const left={child:[a]},right={child:[b]};let pick=()=>left.child;pick&&=()=>right.child;const o={child:pick()};return o.child[0]===b?1:0;",
        "const left={child:{x:a}},right={child:{x:b}};let pick=()=>left.child;pick||=()=>right.child;const o={child:pick()};return o.child.x===a?1:0;",
        "const left={child:{x:a}},right={child:{x:b}};let pick=()=>left.child;pick??=()=>right.child;const o={child:pick()};return o.child.x===a?1:0;",
        "const left={child:{x:a}},right={child:{x:b}};let pick=()=>left.child;const o={child:(pick&&=()=>right.child)()};return o.child.x===b?1:0;",
    ] {
        compare(body);
    }
}

#[test]
fn former_unknown_alias_receiver_guard_executes_unchanged() {
    compare("const o={x:{}};const x=o.x;return x.y?a:b;");
}

#[test]
fn assignment_target_type_erasure_keeps_known_runtime_and_ts_guards() {
    // A later Array store supersedes the earlier record. Accessing its missing
    // "x" property yields Undefined, not the original record's Number.
    let js = "export function run(a,b){const initial={child:{x:a}};let child=initial.child;child=[b];const o={child};return o.child.x===a?1:0;}";
    common::compare_sources(js, js);

    // The retained TS assignment-target wrapper remains a separate unsupported
    // source form; do not mistake JS admission for TS erasure support.
    let ts = "export function run(a,b){const initial={child:{x:a}};let child=initial.child;(child as unknown)=[b];const o={child};return o.child.x===a?1:0;}";
    let error = grunkalunka_compiler::compile(ts, true)
        .err()
        .unwrap_or_else(|| panic!("unexpected TS assignment-target admission: {ts}"));
    assert!(error.contains("unsupported"), "{ts}: {error}");
    assert!(!error.contains("internal IR"), "{ts}: {error}");
}

#[test]
fn nested_record_number_keys_match_java_script() {
    // Number property keys cannot spell the record's "x" field, but these
    // dynamic lookups are supported and still need real result verification.
    compare("const o={child:{x:a}};return o.child[a]===a?1:0;");
}

#[test]
fn unknown_or_mixed_nested_receiver_kinds_reject_before_execution() {
    for body in [
        "const o={child:a?{}:[]};return o.child[0]===a?1:0;",
        "const o={child:a?{x:a}:null};return o.child.x===a?1:0;",
        "const o={child:{x:a}};o.child=null;return o.child.x===a?1:0;",
        "let child={x:a};child&&=null;const o={child};return o.child.x===a?1:0;",
        "const o={};o.child={x:a};return o.child.x===a?1:0;",
        "const use=x=>x.x;const o={child:{x:a}};use(null);return use(o.child)===a?1:0;",
        "const o=[{x:a}];o.length=0;return o[0].x===a?1:0;",
        "const o={child:[()=>a]};o.child.length=0;return o.child[0]();",
        "const o={child:{x:a}};return o.child.__proto__===o?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let result = std::panic::catch_unwind(|| grunkalunka_compiler::compile(&source, ts))
                .unwrap_or_else(|_| panic!("compiler panic: {source}"));
            assert!(result.is_err(), "unexpected admission: {source}");
        }
    }
}
