#[path = "managed/common.rs"]
mod common;

fn compare(body: &str) {
    let source = format!("export function run(a,b){{{body}}}");
    common::compare_sources(&source, &source);
}

#[test]
fn nested_object_fields_can_act_as_receivers() {
    let source = "export function run(a,b){const o={child:{x:a}};return o.child.x===a?1:0;}";
    common::compare_sources(source, source);
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
fn erased_assignment_targets_preserve_all_receiver_alternatives() {
    let js = "export function run(a,b){const initial={child:{x:a}};let child=initial.child;child=[b];const o={child};return o.child.x===a?1:0;}";
    let ts = "export function run(a,b){const initial={child:{x:a}};let child=initial.child;(child as unknown)=[b];const o={child};return o.child.x===a?1:0;}";
    assert!(grunkalunka_compiler::compile(js, false).is_err());
    assert!(
        grunkalunka_compiler::compile(ts, true).is_err(),
        "TS erasure lost an assignment alternative"
    );
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
        "const o={child:{x:a}};return o.child[a]===a?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let result = std::panic::catch_unwind(|| grunkalunka_compiler::compile(&source, ts))
                .unwrap_or_else(|_| panic!("compiler panic: {source}"));
            assert!(result.is_err(), "unexpected admission: {source}");
        }
    }
}
