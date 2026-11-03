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
fn string_length_counts_utf16_code_units_including_lone_surrogates() {
    for literal in [
        "''",
        "'hello'",
        r"'\u{1f600}'",
        r"'\ud800\0\udfff'",
        r"'e\u0301'",
    ] {
        let source = format!("export function run(a,b){{return {literal}.length;}}");
        compare_sources(&source, &source);
    }
}

#[test]
fn string_length_preserves_receiver_effects_and_saved_values() {
    for source in [
        "export function run(a,b){let n=0;let s='x';const saved=s;const len=(n++,s='long',saved).length;return len*100+n*10+s.length;}",
        "export function run(a,b){const o={text:'old'};const saved=o.text;o.text='new text';return saved.length*10+o.text['length'];}",
        "export function run(a,b){const x=['a','bc'];return x[0].length*10+x[1].length;}",
        "export function run(a,b){const s=a?'x':'';const o={n:s.length};return s.length?(o.n===1?a:b):(o.n===0?b:a);}",
        "export function run(a,b){return (typeof a).length;}",
    ] {
        compare_sources(source, source);
    }
}

#[test]
fn string_length_composes_helpers_closures_and_erased_types() {
    for source in [
        "function measure(s){return s.length;}function text(a){return a?'two':'x';}export function run(a,b){return measure(text(a));}",
        "export function run(a,b){let s='old';const f=()=>s.length;const old=f();s='longer';return old*10+f();}",
        "export function run(a,b){const f=s=>s.length;return f('one')+f('four');}",
        "function text(){return 'three';}export function run(a,b){const f=()=>text();return f().length;}",
    ] {
        compare_sources(source, source);
    }
    compare_sources(
        "function text(s){return s;}export function run(a,b){const s='xy';return text(s)['length'];}",
        "function text(s:number):boolean{return s;}export function run(a:string,b:object):void{const s:number='xy';return (text(s) as unknown)['length' as never];}",
    );
}

#[test]
fn string_length_requires_known_string_receivers_and_static_reads() {
    for source in [
        "export function run(a,b){const s=a?'x':null;return s.length;}",
        "export function run(a,b){const s=a?'x':[];return s.length;}",
        "function length(s){return s.length;}export function run(a,b){return length('x')+length(a);}",
        "export function run(a,b){const s='x';const key='length';return s[key];}",
        "export function run(a,b){const s='x';s.length=2;return s.length;}",
        "export function run(a,b){const s='x';return s?.length;}",
    ] {
        for typed in [false, true] {
            let error = grunkalunka_compiler::compile(source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(error.contains("unsupported"), "{source}: {error}");
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

#[test]
fn unimplemented_string_operations_and_reference_frames_reject_explicitly() {
    for body in [
        "return 'text';",
        "const s='text';return s[0];",
        "return +'1';",
        "return 1+'x';",
        "return '0'==0?a:b;",
        "const s='x';s[0]='z';return a;",
        "const s='x';return s.toUpperCase();",
        "const s='x';const f=()=>s;return f();",
        "let s='x';s++;return a;",
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
    assert!(
        grunkalunka_compiler::compile_value("export function run(a,b){return 'x';}", false)
            .is_err()
    );
}

#[test]
fn string_helper_queries_keep_source_depth_off_the_native_stack() {
    for (depth, admitted) in [(16, true), (140, false)] {
        let source = format!(
            "function identity(v){{return {}v{};}} export function run(a,b){{return identity('x').length;}}",
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
