#[path = "managed/common.rs"]
mod common;
use common::compare_sources;

#[test]
fn string_values_compare_content_and_keep_property_snapshots() {
    let source = "export function run(a,b){const o={x:'same'};const saved=o.x;o.x='other';return saved==='same'&&o.x==='other'&&saved!==o.x?a:b;}";
    compare_sources(source, source);
}

#[test]
fn string_truthiness_preserves_empty_and_nonempty_values() {
    for js in [
        "export function run(a,b){const empty='';const zero='0';return empty?b:zero?a:b;}",
        "export function run(a,b){const o={x:a?'':'nonempty'};return o.x?a:b;}",
    ] {
        compare_sources(js, js);
    }
}

#[test]
fn strings_pass_through_helpers_and_mixed_managed_values() {
    for js in [
        "function identity(v){return v;} export function run(a,b){const text=identity('value');return text==='value'&&identity('other')!==text?a:b;}",
        "function identity(v){return v;} export function run(a,b){const o={};return identity(o)===o&&identity('x')==='x'&&identity(a)===a&&identity(null)===null?a:b;}",
        "function get(o){return o.x;} export function run(a,b){const o={x:'text'};return get(o)==='text'?a:b;}",
    ] {
        compare_sources(js, js);
    }
}

#[test]
fn large_string_literals_and_keys_execute_across_engines() {
    let text = "x".repeat(10_001);
    for js in [
        format!("export function run(a,b){{const text='{text}';return text?a:b;}}"),
        format!("export function run(a,b){{const o={{'{text}':a}};return a;}}"),
    ] {
        common::compare_sources_with_fuel(&js, &js, Some(1_000_000));
        let artifact = grunkalunka_compiler::compile(&js, false).unwrap();
        let mut production = grunkalunka_runtime::NumericInstance::new(&artifact.wasm).unwrap();
        let error = production.run(1.0, 2.0).unwrap_err();
        assert!(format!("{error:?}").contains("fuel"), "{error:?}");
    }
}

#[test]
fn string_equality_is_lossless_utf16_without_normalization() {
    for js in [
        r"export function run(a,b){const x='\ud800\0';return x==='\ud800\x00'&&x!=='\ufffd\0'&&x!=='\ud800'?a:b;}",
        r"export function run(a,b){const x='\ud83d\ude00';return x==='\u{1f600}'&&'\u00e9'!=='e\u0301'?a:b;}",
        "export function run(a,b){return ''!==null&&''!==undefined&&'0'!==0&&'false'!==false&&'x'!=={}?a:b;}",
    ] {
        compare_sources(js, js);
    }
}

#[test]
fn string_values_keep_effects_and_erased_type_annotations() {
    let js = "function identity(v){return v;} export function run(a,b){let n=0;let s='old';const old=s;s=(n++,'new');const o={x:(n++,identity(s))};return old==='old'&&s==='new'&&o.x==='new'&&n===2?a:b;}";
    let ts = "function identity(v:number):boolean{return v;} export function run(a:string,b:object):void{let n=0;let s:number='old';const old:boolean=s;s=(n++,'new');const o={x:(n++,identity(s))};return (old as unknown)==='old'&&s==='new'&&o.x==='new'&&n===2?a:b;}";
    compare_sources(js, ts);
    for js in [
        "export function run(a,b){const f=x=>{const s='text';return s?x:a;};return f(b);}",
        "export function run(a,b){const o={x:'text'};return a;}",
        "export function run(a,b){const o={x:true?a:'text'};return a;}",
    ] {
        compare_sources(js, js);
    }
}

#[test]
fn unimplemented_string_operations_and_reference_frames_reject_explicitly() {
    for body in [
        "return 'text';",
        "const s='text';return s.length;",
        "const s='text';return s[0];",
        "return +'1';",
        "return 1+'x';",
        "return '0'==0?a:b;",
        "const s='x';s[0]='z';return a;",
        "const s='x';return s.toUpperCase();",
        "const s='x';const f=()=>s;return f();",
        "let s='x';s++;return a;",
        "return (a?'x':[])===null?a:b;",
        "if(0){const s='x';s.unknown();}return a;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let error = grunkalunka_compiler::compile(&js, ts)
                .err()
                .unwrap_or_else(|| panic!("admitted {js}"));
            assert!(error.contains("unsupported"), "{js}: {error}");
            assert!(!error.contains("internal IR"), "{js}: {error}");
        }
    }
    let error = grunkalunka_compiler::compile_async(
        "export async function run(a,b){const s='x';return s==='x'?a:b;}",
        false,
    )
    .err()
    .unwrap();
    assert_eq!(error, "managed values require reference-aware async frames");
    assert!(
        grunkalunka_compiler::compile_value("export function run(a,b){return 'x';}", false)
            .is_err()
    );
}

#[test]
fn string_helper_queries_keep_source_depth_off_the_native_stack() {
    for (depth, admitted) in [(16, true), (140, false)] {
        let source = format!(
            "function identity(v){{return {}v{};}} export function run(a,b){{return identity('x')==='x'?a:b;}}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || grunkalunka_compiler::compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(result.is_ok(), admitted);
        if let Err(error) = result {
            assert!(error.contains("nesting"), "{error}");
        }
    }
}
