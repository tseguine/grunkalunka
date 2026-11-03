#[path = "managed/common.rs"]
mod common;
use common::compare_sources;

#[test]
fn string_search_clamps_number_positions_and_matches_utf16_units() {
    for body in [
        "const s='abcabc';return (s.startsWith('abc',a)?100:0)+(s.includes('abc',a)?10:0)+(s.includes('bc',b)?1:0);",
        "const s='abcabc';return (s.startsWith('bc',a+0.75)?10:0)+(s.includes('bc',b+1.75)?1:0);",
        "return (''.startsWith('',a)?100:0)+(''.includes('',b)?10:0)+(''.includes('a',a)?1:0);",
        "return 'abc'.startsWith('',Infinity)&&'abc'.includes('',4294967296)&&!'abc'.startsWith('abcd')&&!'abc'.includes('abcd')?1:0;",
        "return 'abababc'.includes('ababc')&&!'abababc'.startsWith('ababc')&&'aaaaab'.includes('aaab')&&!''.startsWith('a')?1:0;",
        r"const s='\0A\u{1f600}\ud800e\u0301';return s.startsWith('\ud83d',2)&&s.startsWith('\ude00',3)&&s.includes('\ud800e')&&s.includes('\0')&&!s.includes('\u00e9')?1:0;",
        r"return '\ud800x\udfff'.includes('\udfff',a)&&'\u{1f600}'.includes('\ude00',b)?1:0;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        compare_sources(&js, &js);
    }
}

#[test]
fn string_search_keeps_defaults_argument_effects_and_operand_snapshots() {
    for body in [
        "return 'abc'.startsWith('a')&&'abc'.startsWith('a',undefined)&&'abc'.includes('a',undefined)&&'abc'.includes('b')?1:0;",
        "let n=0;const found=(n=1,'abc').startsWith((n=n*10+2,'b'),(n=n*10+3,1),(n=n*10+4,0));return found?n:-1;",
        "let n=0;const found=(n=1,'abc').includes((n=n*10+2,'a'),void(n=n*10+3),(n=n*10+4,0));return found?n:-1;",
        "let s='abc';const found=s['startsWith']((s='changed','a'));return found&&s==='changed'?1:0;",
        "let needle='b';const found='abc'['includes'](needle,(needle='missing',0));return found&&needle==='missing'?1:0;",
        "let s='abc';let needle='b';const found=s.includes(needle,0,(s='changed',needle='missing'));return found&&s==='changed'&&needle==='missing'?1:0;",
        "const undefined=2;return 'abc'.startsWith('c',undefined)&&!'abc'.includes('b',undefined)?1:0;",
        "let n=0;'abc'.startsWith((n=1,'a'),(n=n*10+2,0),n=n*10+3);return n;",
        "let n=0;'abc'.includes((n=1,'a'),void(n=n*10+2),n=n*10+3);return n;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        compare_sources(&js, &js);
    }
}

#[test]
fn string_search_composes_boolean_values_helpers_and_ordinary_methods() {
    for source in [
        "function has(s,n){return s.includes(n);}export function run(a,b){return has('Heading: Notes','Notes')&&!has('Heading: Notes','Other')?1:0;}",
        "function text(){return 'abc';}export function run(a,b){const needle={text:'b'};const has=()=>text().includes(needle.text);return has()?1:0;}",
        "export function run(a,b){const found='abc'.includes('a');let flag=found;flag='abc'.startsWith('b');const box={found};return box.found===true&&flag===false&&found!==1?1:0;}",
        "export function run(a,b){const found='abc'.includes('a');return +found+ +'abc'.startsWith('a')+('abc'.includes('b')&&a?10:0);}",
        "export function run(a,b){const text=('abc'+'def').slice(1,5);const xs=[text];return xs.at(0)===text&&text.includes('cd')&&text.startsWith('bc')?text.length:0;}",
        "export function run(a,b){const o={startsWith:(x,y)=>x+y,includes:x=>x};return o.startsWith(a,b)+o.includes(a);}",
        "function text(x){return 'ordinary';}export function run(a,b){const o={startsWith:text,includes:text};return o.startsWith(a).length+o.includes(b).length;}",
        "export function run(a,b){const o={startsWith:x=>x>0,includes:x=>x<0};return o.startsWith(a)&&o.includes(b)?1:0;}",
    ] {
        compare_sources(
            source,
            &source.replace("run(a,b)", "run(a: number,b: number)"),
        );
    }
    compare_sources(
        "export function run(a,b){const s='abc';return s.startsWith('a')?1:0;}",
        "export function run(a: number,b: number){const s: number='abc';return (s as boolean).startsWith('a' as number)?1:0;}",
    );
}

#[test]
fn string_search_rejects_unproven_strings_and_unrepresented_coercions() {
    for body in [
        "return 'abc'.startsWith()?1:0;",
        "return 'abc'.includes(undefined)?1:0;",
        "return 'abc'.includes(a)?1:0;",
        "return 'abc'.startsWith(null)?1:0;",
        "return 'abc'.includes(/a/)?1:0;",
        "return 'abc'.includes('a','1')?1:0;",
        "return 'abc'.startsWith('a',true)?1:0;",
        "return 'abc'.includes('a',null)?1:0;",
        "return 'abc'.startsWith('a',{})?1:0;",
        "const s=a?'abc':null;return s.includes('a')?1:0;",
        "const needle=a?'a':null;return 'abc'.includes(needle)?1:0;",
        "let needle='a';const test=()=> 'abc'.includes(needle);needle=a;return test()?1:0;",
        "const xs=['abc'];return xs.at(0).includes('a')?1:0;",
        "const xs=['a'];return 'abc'.startsWith(xs.at(0))?1:0;",
        "const o={needle:'a'};o.needle=a;return 'abc'.includes(o.needle)?1:0;",
        "const s='abc';return s?.startsWith('a')?1:0;",
        "const s='abc';return s.includes?.('a')?1:0;",
        "return 'abc'.startsWith.call('abc','a')?1:0;",
        "return 'abc'.includes(...['a'])?1:0;",
        "const key='includes';return 'abc'[key]('a')?1:0;",
        "return 'abc'.includes('a').length;",
        "return ('abc'.startsWith('a')+'!').length;",
        "return 'abc'.includes('a');",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let error = grunkalunka_compiler::compile(&source, ts)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(error.contains("unsupported"), "{source}: {error}");
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
    let typed = "export function run(a: number,b: number){const s: string=a;return (s as string).includes('a')?1:0;}";
    assert!(grunkalunka_compiler::compile(typed, true).is_err());
}

#[test]
fn string_search_preserves_the_existing_small_stack_boundary() {
    for (depth, admitted) in [(24, true), (140, false)] {
        let source = format!(
            "export function run(a,b){{return 'abcdef'{}.includes('bc')?1:0;}}",
            ".slice(0)".repeat(depth)
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

#[test]
fn string_characters_use_utf16_units_and_number_index_normalization() {
    for body in [
        r"const s='\0\u{1f600}\ud800\uffff';return s.charCodeAt(a);",
        r"const s='\0\u{1f600}\ud800\uffff';return s.charCodeAt(a+1.75);",
        "const s='abc';const c=s.charAt(a-0.75);return c.length*10+(c==='a'?1:0)+(c==='b'?2:0);",
        r"const s='\u{1f600}';return s.charAt(0)==='\ud83d'&&s['charAt'](1)==='\ude00'&&s.charAt(0)+s.charAt(1)===s?1:0;",
        r"return '\0\u8000\uffff'.charCodeAt(1)+'\0\u8000\uffff'.charCodeAt(2);",
        "return ''.charCodeAt(a);",
        "return ''.charAt(a).length;",
        r"const s='\0A';return (s.charAt(a)?10:0)+(s.charCodeAt(a)?1:0);",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        compare_sources(&js, &js);
    }
}

#[test]
fn string_characters_keep_default_indices_effects_and_snapshots() {
    for body in [
        "return 'abc'.charAt()==='a'&&'abc'.charAt(undefined)==='a'&&'abc'.charCodeAt()===97&&'abc'.charCodeAt(undefined)===97?1:0;",
        "let n=0;const c=(n=1,'abc').charAt((n=n*10+2,1),(n=n*10+3,2),(n=n*10+4,0));return c==='b'?n:-1;",
        "let n=0;const c=(n=1,'abc').charCodeAt(void(n=n*10+2),(n=n*10+3,2));return c===97?n:-1;",
        "let s='abc';const c=s.charAt((s='xyz',1));return c==='b'&&s==='xyz'?1:0;",
        "let s='abc';const c=s.charCodeAt((s='xyz',1));return c===98&&s==='xyz'?1:0;",
        "const undefined=2;return 'abc'.charAt(undefined)==='c'&&'abc'.charCodeAt(undefined)===99?1:0;",
        "let n=0;'abc'.charCodeAt((n=1,0),(n=n*10+2));'abc'.charAt((n=n*10+3,0));return n;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        compare_sources(&js, &js);
    }
}

#[test]
fn string_characters_compose_numeric_helpers_and_captured_string_results() {
    for source in [
        "function code(s,i){return s.charCodeAt(i);}export function run(a,b){return code('abc',a);}",
        "function first(s){return s.charAt(0);}export function run(a,b){const text=first('Heading');const use=()=>text+'!';return use()==='H!'?text.length:0;}",
        "export function run(a,b){let s='abc';const code=()=>s.charCodeAt(a);const character=()=>s.charAt(a);return code()+character().length;}",
        "export function run(a,b){const o={text:'abc'.charAt(1)};return o.text.charCodeAt(0);}",
        "export function run(a,b){const o={charAt:(x)=>x,charCodeAt:()=>true};const f=()=>o.charCodeAt();return f()===true?o.charAt(a):b;}",
        "export function run(a,b){const code='abc'.charCodeAt(a);return typeof code==='number'?code:b;}",
    ] {
        compare_sources(
            source,
            &source.replace("run(a,b)", "run(a: number,b: number)"),
        );
    }
    compare_sources(
        "export function run(a,b){const s='abc';return s.charCodeAt(a)+s.charAt(b).length;}",
        "export function run(a: number,b: number){const s: number='abc';return (s as boolean).charCodeAt(a as never)+s.charAt(b).length;}",
    );
}

#[test]
fn string_characters_preserve_receiver_and_result_kind_guards() {
    for body in [
        "return 'abc'.charAt('1').length;",
        "return 'abc'.charCodeAt(null);",
        "return 'abc'.charAt(true).length;",
        "return 'abc'.charCodeAt({});",
        "const s=a?'abc':null;return s.charAt(0).length;",
        "const s=a?'abc':{charCodeAt:()=>1};return s.charCodeAt(0);",
        "return 'abc'.charCodeAt(a).length;",
        "return 'abc'.charCodeAt(a)+'x';",
        "return 'abc'.charAt?.(0).length;",
        "return 'abc'.charCodeAt(...[0]);",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let error = grunkalunka_compiler::compile(&source, ts)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(error.contains("unsupported"), "{source}: {error}");
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

#[test]
fn string_character_queries_keep_small_native_stack_limits() {
    for (depth, admitted) in [(24, true), (140, false)] {
        for source in [
            format!(
                "export function run(a,b){{return 'abc'{}.length;}}",
                ".charAt(0)".repeat(depth)
            ),
            format!(
                "export function run(a,b){{return {}a{};}}",
                "'abc'.charCodeAt(".repeat(depth),
                ")".repeat(depth)
            ),
        ] {
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
}

#[test]
fn string_ranges_clamp_number_endpoints_and_copy_utf16_units() {
    for body in [
        "const s='abcdef';const t=s.slice(a,b);return t.length*100+(t==='abcdef'?10:0)+(t==='a'?1:0);",
        "const s='abcdef';const t=s.substring(a,b);return t.length*100+(t==='abcdef'?10:0)+(t==='a'?1:0);",
        "const s='abcdef';const t=s.slice(a+0.75,b+2.25);return t.length*10+(t==='bc'?1:0);",
        "const s='abcdef';const t=s.substring(a+0.75,b+2.25);return t.length*10+(t==='bc'?1:0);",
        "return 'abcdef'.slice(4,1)===''&&'abcdef'.substring(4,1)==='bcd'?1:0;",
        r"const s='\0A\u{1f600}\ud800e\u0301';return s.slice(2,3)==='\ud83d'&&s.substring(4,3)==='\ude00'&&s.slice(-3)==='\ud800e\u0301'?1:0;",
        "return ''.slice(a,b)===''&&''.substring(a,b)===''?1:0;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        compare_sources(&js, &js);
    }
}

#[test]
fn string_ranges_keep_defaults_argument_effects_and_receiver_snapshots() {
    for body in [
        "const s='abcdef';return s.slice()===s&&s.substring()===s&&s.slice(undefined,undefined)===s&&s.substring(undefined,undefined)===s?1:0;",
        "let n=0;const t=(n=1,'abcdef').slice((n=n*10+2,1),(n=n*10+3,4),(n=n*10+4,0));return t==='bcd'?n:-1;",
        "let n=0;const t=(n=1,'abcdef').substring(void(n=n*10+2),void(n=n*10+3),(n=n*10+4,0));return t==='abcdef'?n:-1;",
        "let s='abcdef';const t=s['slice']((s='changed',1),3);return t==='bc'&&s==='changed'?1:0;",
        "const undefined=2;return 'abcdef'.substring(undefined)==='cdef'?1:0;",
        "let n=0;'abcdef'.slice((n=1,2),(n=n*10+2,4),n=n*10+3);return n;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        compare_sources(&js, &js);
    }
}

#[test]
fn string_ranges_compose_helpers_captures_fields_and_ordinary_method_calls() {
    for source in [
        "function preview(s,n){return s.slice(0,n);}export function run(a,b){const text=preview('Heading: Notes',8);const use=()=>text+'!';return use()==='Heading:!'?text.length:0;}",
        "export function run(a,b){const source={text:'abcdef'};const selected=source.text.substring(1,4);const box={text:selected};source.text='changed';return box.text.length+(box.text==='bcd'?10:0);}",
        "export function run(a,b){const o={slice:(x,y)=>x+y,substring:(x)=>x};return o.slice(a,b)+o.substring(a);}",
        "function text(n){return 'ordinary';}export function run(a,b){const o={slice:text};return o.slice(a).length;}",
    ] {
        compare_sources(
            source,
            &source.replace("run(a,b)", "run(a: number,b: number)"),
        );
    }
    compare_sources(
        "export function run(a,b){const s='abcdef';return s.slice(1,3).length;}",
        "export function run(a: number,b: number){const s: number='abcdef';return (s as boolean).slice(1,3).length;}",
    );
}

#[test]
fn string_ranges_reject_unproven_receivers_and_unrepresented_coercions() {
    for body in [
        "return 'abc'.slice('1').length;",
        "return 'abc'.substring(null).length;",
        "return 'abc'.slice(true).length;",
        "return 'abc'.substring({}).length;",
        "const s=a?'abc':null;return s.slice(0).length;",
        "let s='abc';s=a;return s.substring(0).length;",
        "const preview=()=>a?'abc'.slice(0,2):null;const label=preview();const use=()=>label+'!';return use().length;",
        "const o={text:'abc'};o.text=a?'abc':null;return o.text.substring(0).length;",
        "const s='abc';return s?.slice(0).length;",
        "const s='abc';return s.slice?.(0).length;",
        "const s='abc';return s.slice(...[0]).length;",
        "const key='slice';return 'abc'[key](0).length;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let error = grunkalunka_compiler::compile(&source, ts)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(error.contains("unsupported"), "{source}: {error}");
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

#[test]
fn string_range_chains_keep_the_existing_small_stack_boundary() {
    for (depth, admitted) in [(24, true), (140, false)] {
        let source = format!(
            "export function run(a,b){{return 'abcdef'{}.length;}}",
            ".slice(0)".repeat(depth)
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
        "function s(x){if(x)return 'x';}export function run(a,b){return s(a).length;}",
        "function s(x){return x?'x':null;}export function run(a,b){return s(a).length;}",
        "export function run(a,b){let s='x';const length=()=>s.length;s=a;return length();}",
    ] {
        for typed in [false, true] {
            let error = grunkalunka_compiler::compile(source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(error.contains("unsupported"), "{source}: {error}");
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
    assert!(grunkalunka_compiler::compile(
        "export function run(a:number,b:object){const text:string=a;return (text as string).length;}",
        true,
    ).is_err());
}

#[test]
fn string_concat_keeps_lossless_utf16_contents_and_empty_values() {
    for source in [
        r"export function run(a,b){return ('\ud800'+'\udfff')==='\ud800\udfff'?a:b;}",
        r"export function run(a,b){return ('\u{1f600}'+'\0x').length;}",
        r"export function run(a,b){return ('e'+'\u0301')!=='\u00e9'?a:b;}",
        "export function run(a,b){const s=a?'x':'';return (''+s)===(s+'')&&(''+'').length===0?a:b;}",
    ] {
        compare_sources(source, source);
    }
}

#[test]
fn string_concat_snapshots_left_before_right_effects() {
    for source in [
        "export function run(a,b){let n=0;let s='left';const text=(n=n*10+1,s)+(n=n*10+2,s='right');return text==='leftright'&&s==='right'&&n===12?a:b;}",
        "export function run(a,b){const o={s:'old'};const text=o.s+(o.s='new');return text==='oldnew'&&o.s==='new'?a:b;}",
        "export function run(a,b){let s='';for(let i=0;i<3;i++){s=s+'x';}return s.length;}",
        "export function run(a,b){let n=0;(n++,'a')+(n++,'b');return n;}",
    ] {
        compare_sources(source, source);
    }
}

#[test]
fn string_concat_composes_helpers_captures_and_erased_types() {
    for source in [
        "function join(x,y){return x+y;}export function run(a,b){return join('prefix:','body').length;}",
        "export function run(a,b){const suffix='!';const f=x=>x+suffix;return f('hello')==='hello!'?a:b;}",
        "export function run(a,b){const parts=['one','two'];const o={label:parts[0]+parts[1]};return o.label.length;}",
    ] {
        compare_sources(source, source);
    }
    compare_sources(
        "function join(x,y){return x+y;}export function run(a,b){const label=join('a','bc');return label.length;}",
        "function join(x:number,y:boolean):object{return x+y;}export function run(a:string,b:object):void{const label:number=join('a','bc');return (label as never).length;}",
    );
}

#[test]
fn string_concat_keeps_mixed_coercion_and_compound_assignment_outside_this_slice() {
    for body in [
        "return ('x'+a).length;",
        "return (a+'x').length;",
        "return ('x'+null).length;",
        "return ({}+'x').length;",
        "const value=a?'x':b;return (value+'x').length;",
        "let text='x';text+='y';return text.length;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let error = grunkalunka_compiler::compile(&source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(error.contains("unsupported"), "{source}: {error}");
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

#[test]
fn string_concat_length_growth_traps_before_wasm32_wraparound() {
    let plain =
        grunkalunka_compiler::compile("export function run(a,b){return 'plain'.length;}", false)
            .unwrap();
    assert!(!plain.wat.contains("$gc_string_concat"));
    let artifact =
        grunkalunka_compiler::compile("export function run(a,b){return ('a'+'b').length;}", false)
            .unwrap();
    // Exercise the actual private allocation guard without allocating multi-GB
    // arrays. Overflow is a resource trap, not an admitted JS thrown value.
    let base = artifact.wat.trim_end().strip_suffix(')').unwrap();
    let wat = format!("{base}(export \"length\" (func $gc_concat_length)))");
    let engine = wasmtime::Engine::default();
    let wasm = wat::parse_str(wat).unwrap();
    let module = wasmtime::Module::from_binary(&engine, &wasm).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let length = instance
        .get_typed_func::<(u32, u32), u32>(&mut store, "length")
        .unwrap();
    for (left, right, expected) in [
        (0, 0, 0),
        (2, 3, 5),
        (u32::MAX, 0, u32::MAX),
        (1 << 31, (1 << 31) - 1, u32::MAX),
    ] {
        assert_eq!(length.call(&mut store, (left, right)).unwrap(), expected);
    }
    for (left, right) in [(u32::MAX, 1), (1 << 31, 1 << 31), (u32::MAX, u32::MAX)] {
        assert!(length.call(&mut store, (left, right)).is_err());
    }
}

#[test]
fn string_concat_nesting_uses_the_existing_bounded_cursor() {
    for (terms, admitted) in [(64, true), (140, false)] {
        let expression = vec!["'x'"; terms].join("+");
        let source = format!("export function run(a,b){{return ({expression}).length;}}");
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
