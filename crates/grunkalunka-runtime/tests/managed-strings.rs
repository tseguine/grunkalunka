#[path = "managed/common.rs"]
mod common;

#[test]
fn string_relational_operators_use_utf16_units_and_equal_prefix_lengths() {
    for (left, right) in [
        ("''", "''"),
        ("''", "'a'"),
        ("'a'", "''"),
        ("'ab'", "'abc'"),
        ("'abc'", "'ab'"),
        ("'a'", "('a'+'')"),
        ("'2'", "'10'"),
        ("'Z'", "'a'"),
        (r"'\0a'", r"'\0b'"),
        (r"'\ud800'", r"'\udfff'"),
        (r"'\u{10000}'", r"'\ue000'"),
        (r"'\uffff'", r"'\u0001'"),
        (r"'\u00e9'", r"'e\u0301'"),
    ] {
        let source = format!(
            "export function run(a,b){{const x={left};const y={right};return (x<y?1:0)+(x<=y?2:0)+(x>y?4:0)+(x>=y?8:0);}}"
        );
        common::compare_sources(&source, &source);
    }
}

#[test]
fn string_relational_operands_keep_source_order_snapshots_and_captures() {
    for operator in ["<", "<=", ">", ">="] {
        let source = format!(
            "export function run(a,b){{let text='a';let n=0;const value=(n=n+1,text){operator}(n=n*10+2,text='z','b');return (value?100:0)+n+text.length;}}"
        );
        common::compare_sources(&source, &source);
    }
    for body in [
        "const o={s:a?'a':'z'};const read=()=>o.s;return read()<'m'?1:0;",
        "let n=0;for(const s of ['a','b','z']){if(s<'m')n++;}return n;",
        "const o={s:'a'};const value=o.s<(o.s='z','b');return value&&o.s==='z'?1:0;",
        "const xs=['b'];const value=xs[0]>(xs[0]='','a');return value&&xs[0]===''?1:0;",
        "const less=(x,y)=>x<y;return less('a','b')?1:0;",
        "const flags=['a'<'b','z'>='z'];return flags[0]==true&&flags[1]==true?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
    let js = "export function run(a,b){const o={s:'a'};return o.s<'b'?1:0;}";
    let ts = "export function run(a:number,b:number){const o:number={s:'a'};return (o as boolean).s<('b' as number)?1:0;}";
    common::compare_sources(js, ts);
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm
    );
}

#[test]
fn discarded_template_executes_the_exact_former_rejection_source() {
    let source = "export function run(a,b) { `hint`; return a; }";
    common::compare_sources(source, source);
}
use common::compare_sources;

#[test]
fn string_loose_equality_uses_contents_and_saved_left_values() {
    for body in [
        "const o={x:'text'};return o.x=='text'?1:0;",
        "return ''==''&&'a'!='b'&&('x'+'')=='x'?1:0;",
        "let text='old';const value=text==(text='new','old');return value&&text==='new'?1:0;",
        "const o={s:'old'};const value=o.s==(o.s='new','old');return value&&o.s==='new'?1:0;",
        "let n=0;for(const text of ['a','b']){if(text=='a')n++;if(text!='b')n+=10;}return n;",
        "const o={s:'x'};const equal=()=>o.s=='x';o.s+='';return equal()?1:0;",
        "const flags=['x'=='x','x'!='y'];return flags[0]===true&&flags[1]===true?1:0;",
        r"return '\ud800\0'=='\ud800\x00'&&'\ud83d\ude00'=='\u{1f600}'&&'\u00e9'!='e\u0301'?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        compare_sources(&source, &source);
    }
    let js = "export function run(a,b){const o={s:'text'};return o.s=='text'?1:0;}";
    let ts = "export function run(a:number,b:number){const o:number={s:'text'};return (o as boolean).s==('text' as number)?1:0;}";
    compare_sources(js, ts);
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm
    );
}

#[test]
fn string_property_compounds_keep_reference_snapshots_and_utf16() {
    for body in [
        "const o={s:'x'};return (o.s+='y').length;",
        "const xs=['x'];const value=xs[0]+='y';return value==='xy'&&xs[0]===value&&xs.length===1?1:0;",
        "const o={inner:{s:'x'}};o.inner['s']+='y';return o.inner.s==='xy'?1:0;",
        "let o={s:'old'};const alias=o;const value=o.s+=(o={s:'other'},'new');return alias.s==='oldnew'&&o.s==='other'&&value===alias.s?1:0;",
        "const o={s:'old'};const value=o.s+=(o.s='changed','new');return value==='oldnew'&&o.s===value?1:0;",
        "let n=0;const o={s:'x'};const value=(n=n+1,o).s+=(n=n+10,'y');return n===11&&value==='xy'&&o.s===value?1:0;",
        "const o={s:''};for(const value of ['a','b'])o.s+=value;const read=()=>o.s;return read()==='ab'?1:0;",
        "const o={s:'x'};const add=()=>o.s+='y';return add()==='xy'&&o.s==='xy'?1:0;",
        "const o={s:''};if(o.s+='')return -1;if(o.s+='x')return o.s==='x'?1:0;return -2;",
        "const o={s:'a'};const xs=['b'];const value=o.s+=(xs[0]+='c');return value==='abc'&&o.s===value&&xs[0]==='bc'?1:0;",
        r"const o={s:'\ud800'};o.s+='\udfff\0';return o.s==='\ud800\udfff\0'?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        compare_sources(&source, &source);
    }
    let js = "export function run(a,b){const o={s:'x'};const value=o['s']+='y';return value===o.s?value.length:0;}";
    let ts = "export function run(a:number,b:number){const o:number={s:'x'};const value=(o as boolean)[('s' as number)]+=('y' as never);return value===o.s?value.length:0;}";
    compare_sources(js, ts);
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm,
    );
}

#[test]
fn string_local_compounds_preserve_snapshots_captures_and_utf16() {
    for body in [
        "let text='x';text+='y';return text.length;",
        "let [text]=['x'];return (text+='y').length;",
        "for(let text of ['x'])return (text+='y').length;return a;",
        "let text='old';const value=text+=(text='changed','new');return text==='oldnew'&&value===text?1:0;",
        "let text='x';const read=()=>text;text+='y';return read()==='xy'?1:0;",
        "let text='x';const add=()=>text+='y';return add()==='xy'&&text==='xy'?1:0;",
        "let text='';for(let value of ['a','b'])text+=value;return text==='ab'?1:0;",
        r"let text='\ud800';text+='\udfff\0';return text==='\ud800\udfff\0'?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        compare_sources(&source, &source);
    }
}

#[test]
fn string_indices_clamp_number_positions_and_return_numeric_offsets() {
    for body in [
        // The first input selects the method; the second still exercises every
        // Number position in each method, retaining exact result-bit comparisons.
        "return a>0?'abcabc'.indexOf('bc',b):'abcabc'.lastIndexOf('bc',b);",
        "return a>0?'abcabc'.indexOf('bc',b+1.75):'abcabc'.lastIndexOf('bc',b+1.75);",
        "return a>0?''.indexOf('',b):''.lastIndexOf('',b);",
        "return a>0?'abc'.indexOf('',b):'abc'.lastIndexOf('',b);",
        "return 'abcabc'.indexOf('abc')===0&&'abcabc'.indexOf('abc',undefined)===0&&'abcabc'.indexOf('abc',NaN)===0&&'abcabc'.lastIndexOf('abc')===3&&'abcabc'.lastIndexOf('abc',undefined)===3&&'abcabc'.lastIndexOf('abc',NaN)===3?1:0;",
        "return 'abcabc'.indexOf('abc',1)===3&&'abcabc'.indexOf('abc',4)===-1&&'abcabc'.lastIndexOf('abc',4)===3&&'abcabc'.lastIndexOf('abc',1)===0?1:0;",
        "return ''.indexOf('a')===-1&&''.lastIndexOf('a')===-1&&'a'.indexOf('abc')===-1&&'a'.lastIndexOf('abc')===-1&&'ab'.lastIndexOf('b',-Infinity)===-1&&'ab'.lastIndexOf('a',-Infinity)===0?1:0;",
        "return 'abc'.indexOf('c',4294967296)===-1&&'abc'.lastIndexOf('c',4294967296)===2&&'abc'.indexOf('',1e300)===3&&'abc'.lastIndexOf('',1e300)===3&&'abc'.indexOf('a',-Infinity)===0?1:0;",
        "return 'ababa'.indexOf('aba',1)===2&&'ababa'.lastIndexOf('aba',1)===0&&'aaaa'.indexOf('aa',1)===1&&'aaaa'.lastIndexOf('aa',Infinity)===2?1:0;",
        r"const s='\0A\u{1f600}\ud800e\u0301\u{1f600}';return s.indexOf('\ude00')*100+s.lastIndexOf('\ude00')*10+s.indexOf('\ud800');",
        r"return '\ud800x\udfff'.indexOf('\udfff')===2&&'\ud800x\udfff'.lastIndexOf('\ud800')===0&&'e\u0301'.indexOf('\u00e9')===-1&&'e\u0301'.lastIndexOf('\u0301')===1?1:0;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        compare_sources(&js, &js);
    }
}

#[test]
fn string_indices_keep_defaults_argument_order_and_operand_snapshots() {
    for body in [
        "let n=0;const pos=(n=1,'abcabc').indexOf((n=n*10+2,'bc'),(n=n*10+3,2),(n=n*10+4,{}));return pos===4?n:-1;",
        "let n=0;const pos=(n=1,'abcabc').lastIndexOf((n=n*10+2,'bc'),(n=n*10+3,3),(n=n*10+4,null));return pos===1?n:-1;",
        "let s='abcabc';const pos=s['indexOf']((s='changed','bc'),2);return pos===4&&s==='changed'?1:0;",
        "let needle='bc';const pos='abcabc'.lastIndexOf(needle,(needle='missing',3));return pos===1&&needle==='missing'?1:0;",
        "let s='abcabc';let needle='bc';const pos=s.indexOf(needle,2,(s='changed',needle='missing'));return pos===4&&s==='changed'&&needle==='missing'?1:0;",
        "let n=0;const first='abcabc'.indexOf((n=1,'bc'),void(n=n*10+2),(n=n*10+3));const last='abcabc'.lastIndexOf('bc',void(n=n*10+4));return first===1&&last===4?n:-1;",
        "const undefined=2;return 'abcabc'.indexOf('bc',undefined)===4&&'abcabc'.lastIndexOf('bc',undefined)===1?1:0;",
        "let n=0;'abc'.indexOf((n=1,'a'),(n=n*10+2,0),n=n*10+3);'abc'.lastIndexOf((n=n*10+4,'c'),void(n=n*10+5));return n;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        compare_sources(&js, &js);
    }
}

#[test]
fn string_indices_compose_numeric_helpers_and_source_label_handlers() {
    for source in [
        "function first(s,n,p){return s.indexOf(n,p);}function last(s,n,p){return s.lastIndexOf(n,p);}export function run(a,b){return first('abcabc','bc',a)+last('abcabc','bc',b);}",
        "export function run(a,b){const first='abc'.indexOf('b');const last='abc'.lastIndexOf('missing');const box={first,last};return typeof box.first==='number'&&box.first!==true&&box.first===1&&box.last===-1?1:0;}",
        "function label(s){const text=s.trim();const start=text.indexOf(':');const name=text.slice(start+1).trim();const end=name.lastIndexOf('.');return end<0?name:name.slice(0,end);}export function run(a,b){const title=a>0?' Heading: Notes.md ':' Heading: Notes ';const handler=()=>label(title)==='Notes'?a:b;return handler();}",
        "export function run(a,b){const o={indexOf:(x,y)=>x+y,lastIndexOf:x=>x>0};return o.lastIndexOf(a)?o.indexOf(a,b):b;}",
        "function text(x){return 'ordinary';}export function run(a,b){const o={indexOf:text,lastIndexOf:text};return o.indexOf(a).length+o.lastIndexOf(b).length;}",
    ] {
        compare_sources(
            source,
            &source.replace("run(a,b)", "run(a: number,b: number)"),
        );
    }
    let js = "export function run(a,b){const s='abcabc';return s.indexOf('bc',a)*10+s['lastIndexOf']('bc',b);}";
    let ts = "export function run(a: number,b: number){const s: number='abcabc';return (s as boolean).indexOf('bc' as never,a as unknown)*10+s[('lastIndexOf' as number)]('bc',b);}";
    compare_sources(js, ts);
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm
    );
}

#[test]
fn string_indices_reject_unproven_strings_and_unrepresented_coercions() {
    for body in [
        "return 'abc'.indexOf();",
        "return 'abc'.lastIndexOf(undefined);",
        "return 'abc'.indexOf(a);",
        "return 'abc'.lastIndexOf(/c/);",
        "return 'abc'.indexOf('c','1');",
        "return 'abc'.lastIndexOf('c',null);",
        "return 'abc'.indexOf('c',true);",
        "const s=a?'abc':null;return s.lastIndexOf('c');",
        "const needle=a?'c':null;return 'abc'.indexOf(needle);",
        "let needle='c';const find=()=> 'abc'.lastIndexOf(needle);needle=a;return find();",
        "const xs=['abc'];return xs.at(0).indexOf('c');",
        "const xs=['c'];return 'abc'.lastIndexOf(xs.at(0));",
        "const s='abc';return s?.indexOf('c');",
        "const s='abc';return s.lastIndexOf?.('c');",
        "return 'abc'.indexOf.call('abc','c');",
        "return 'abc'.lastIndexOf(...['c']);",
        "const key='indexOf';return 'abc'[key]('c');",
        "return 'abc'.lastIndexOf('c').length;",
        "return ('abc'.indexOf('a')+'!').length;",
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
    assert!(
        grunkalunka_compiler::compile(
            "export function run(a: number,b: number){const s: string=a;return (s as string).lastIndexOf('c');}",
            true,
        )
        .is_err()
    );
}

#[test]
fn string_ends_with_clamps_endpoints_and_matches_utf16_units() {
    for body in [
        "const s='abcabc';return (s.endsWith('abc',a)?100:0)+(s.endsWith('bc',b)?10:0)+(s.endsWith('b',a+1.75)?1:0);",
        "return 'abc'.endsWith('bc')&&'abc'.endsWith('bc',undefined)&&!'abc'.endsWith('bc',NaN)&&'abc'.endsWith('',NaN)?1:0;",
        "return 'abc'.endsWith('c',Infinity)&&'abc'.endsWith('c',4294967296)&&'abc'.endsWith('c',1e300)&&!'abc'.endsWith('a',-Infinity)&&!'abc'.endsWith('a',-0.75)?1:0;",
        "return 'abcabc'.endsWith('bc',3.9)&&'abcabc'.endsWith('b',2.9)&&!'abcabc'.endsWith('bc',2.9)&&!'abcabc'.endsWith('abc',-3)?1:0;",
        "return (''.endsWith('',a)?100:0)+('abc'.endsWith('',b)?10:0)+(''.endsWith('a',a)?1:0);",
        "return !'abc'.endsWith('abcd')&&!'abc'.endsWith('bc',1)&&!'abc'.endsWith('a')&&'abcabc'.endsWith('abc',3)&&'abc'.endsWith('abc')?1:0;",
        r"const s='\0A\u{1f600}\ud800e\u0301';return s.endsWith('\ud83d',3)&&s.endsWith('\ude00',4)&&s.endsWith('\ud800',5)&&s.endsWith('\0',1)&&s.endsWith('e\u0301')&&!s.endsWith('\u00e9')?1:0;",
        r"return ('\ud800x\udfff'.endsWith('\udfff',a)?10:0)+('\u{1f600}'.endsWith('\ude00',b+1)?1:0);",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        compare_sources(&js, &js);
    }
}

#[test]
fn string_ends_with_keeps_defaults_argument_effects_and_operand_snapshots() {
    for body in [
        "let n=0;const found=(n=1,'abc').endsWith((n=n*10+2,'bc'),(n=n*10+3,3),(n=n*10+4,{}));return found?n:-1;",
        "let n=0;const found='abc'.endsWith((n=1,'bc'),void(n=n*10+2),(n=n*10+3,null));return found?n:-1;",
        "let s='abc';const found=s['endsWith']((s='changed','bc'));return found&&s==='changed'?1:0;",
        "let needle='bc';const found='abc'.endsWith(needle,(needle='missing',3));return found&&needle==='missing'?1:0;",
        "let s='abc';let needle='bc';const found=s.endsWith(needle,3,(s='changed',needle='missing'));return found&&s==='changed'&&needle==='missing'?1:0;",
        "const undefined=2;return 'abc'.endsWith('b',undefined)&&!'abc'.endsWith('c',undefined)?1:0;",
        "let n=0;'abc'.endsWith((n=1,'bc'),(n=n*10+2,3),(n=n*10+3));return n;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        compare_sources(&js, &js);
    }
}

#[test]
fn string_ends_with_composes_booleans_helpers_and_ordinary_method_names() {
    for source in [
        "function suffix(s,n){return s.endsWith(n);}export function run(a,b){return suffix('Heading: Notes','Notes')&&!suffix('Heading: Notes','Other')?1:0;}",
        "function text(){return 'abc';}export function run(a,b){const needle={text:'bc'};const matches=()=>text().endsWith(needle.text);return matches()?1:0;}",
        "export function run(a,b){const found='abc'.endsWith('bc');const box={found};let flag=found;flag='abc'.endsWith('ab');return box.found===true&&found!==1&&flag===false?1:0;}",
        "export function run(a,b){const label=(' Notes '+'!').slice(0,-1).trim();return label.endsWith('tes')?label.length:0;}",
        "export function run(a,b){const o={endsWith:(x,y)=>x+y};return o.endsWith(a,b);}",
        "function text(x){return 'ordinary';}export function run(a,b){const o={endsWith:text};return o.endsWith(a).length;}",
        "export function run(a,b){const o={endsWith:x=>x>0};return o.endsWith(a)?1:0;}",
    ] {
        compare_sources(
            source,
            &source.replace("run(a,b)", "run(a: number,b: number)"),
        );
    }
    let js = "export function run(a,b){const s='abc';return s.endsWith('bc',a)?1:0;}";
    let ts = "export function run(a: number,b: number){const s: number='abc';return (s as boolean).endsWith('bc' as never,a as unknown)?1:0;}";
    compare_sources(js, ts);
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm
    );
}

#[test]
fn string_ends_with_rejects_unproven_strings_and_unrepresented_coercions() {
    for body in [
        "return 'abc'.endsWith()?1:0;",
        "return 'abc'.endsWith(undefined)?1:0;",
        "return 'abc'.endsWith(a)?1:0;",
        "return 'abc'.endsWith(/c/)?1:0;",
        "return 'abc'.endsWith('c','3')?1:0;",
        "return 'abc'.endsWith('c',null)?1:0;",
        "return 'abc'.endsWith('c',true)?1:0;",
        "const s=a?'abc':null;return s.endsWith('c')?1:0;",
        "const needle=a?'c':null;return 'abc'.endsWith(needle)?1:0;",
        "let needle='c';const matches=()=> 'abc'.endsWith(needle);needle=a;return matches()?1:0;",
        "const xs=['abc'];return xs.at(0).endsWith('c')?1:0;",
        "const xs=['c'];return 'abc'.endsWith(xs.at(0))?1:0;",
        "const s='abc';return s?.endsWith('c')?1:0;",
        "const s='abc';return s.endsWith?.('c')?1:0;",
        "return 'abc'.endsWith.call('abc','c')?1:0;",
        "return 'abc'.endsWith(...['c'])?1:0;",
        "const key='endsWith';return 'abc'[key]('c')?1:0;",
        "return 'abc'.endsWith('c').length;",
        "return 'abc'.endsWith('c');",
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
    assert!(
        grunkalunka_compiler::compile(
            "export function run(a: number,b: number){const s: string=a;return (s as string).endsWith('c')?1:0;}",
            true,
        )
        .is_err()
    );
}

#[test]
fn string_trim_uses_ecmascript_whitespace_and_line_terminators() {
    for body in [
        r"const ws='\t\v\f \u00a0\ufeff\u1680\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u202f\u205f\u3000\n\r\u2028\u2029';const s=ws+'Heading'+ws;return s.trim()==='Heading'&&s.trimStart()==='Heading'+ws&&s.trimEnd()===ws+'Heading'?1:0;",
        r"const ws='\t\v\f \u00a0\ufeff\u1680\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u202f\u205f\u3000\n\r\u2028\u2029';return ws.trim()===''&&ws.trimStart()===''&&ws.trimEnd()===''?1:0;",
        "return ''.trim()===''&&''.trimStart()===''&&''.trimEnd()===''&&'text'.trim()==='text'&&'text'.trimStart()==='text'&&'text'.trimEnd()==='text'?1:0;",
        "const s='  a b  ';return s.trim()==='a b'&&s.trimStart()==='a b  '&&s.trimEnd()==='  a b'&&s==='  a b  '?1:0;",
        r"const s=' \ud800\u{1f600}\0\udfff\t\u00a0e\u0301\r\n';return s.trim()==='\ud800\u{1f600}\0\udfff\t\u00a0e\u0301'&&s.trimStart()==='\ud800\u{1f600}\0\udfff\t\u00a0e\u0301\r\n'&&s.trimEnd()===' \ud800\u{1f600}\0\udfff\t\u00a0e\u0301'?1:0;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        compare_sources(&js, &js);
    }
    // These are deliberately not ECMAScript whitespace, despite resembling it
    // or appearing in older Unicode whitespace definitions.
    for unit in [r"\u0085", r"\u180e", r"\u200b"] {
        let js = format!(
            "export function run(a,b){{const edge='{unit}';const s=edge+' '+edge;return s.trim()===s&&s.trimStart()===s&&s.trimEnd()===s&&(' '+s+' ').trim()===s?1:0;}}"
        );
        compare_sources(&js, &js);
    }
}

#[test]
fn string_trim_snapshots_receivers_and_evaluates_every_ignored_argument() {
    for body in [
        "let n=0;const text=(n=1,' x ').trim((n=n*10+2,null),(n=n*10+3,{}),(n=n*10+4,'ignored'));return text==='x'?n:-1;",
        "let s=' x ';const text=s['trimStart']((s='changed',false),undefined);return text==='x '&&s==='changed'?1:0;",
        "let s=' x ';const text=s.trimEnd((s='changed',[]),()=>a);return text===' x'&&s==='changed'?1:0;",
        "let n=0;' x '.trim(n=1,n=n*10+2);' x '.trimStart(n=n*10+3);' x '.trimEnd(n=n*10+4);return n;",
        "const o={text:' x '};const text=o.text.trim((o.text='changed'));return text==='x'&&o.text==='changed'?1:0;",
        "const s=' x ';return s.trim(a,b)==='x'&&s.trimStart(a,b)==='x '&&s.trimEnd(a,b)===' x'?1:0;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        compare_sources(&js, &js);
    }
}

#[test]
fn string_trim_composes_helpers_captures_fields_and_ordinary_method_names() {
    for source in [
        "function clean(s){return s.trim();}export function run(a,b){const text=clean(' Heading ');const use=()=>text+'!';return use()==='Heading!'?text.length:0;}",
        "export function run(a,b){let text=' first ';const clean=()=>text.trimStart().trimEnd();const first=clean();text=' next ';return first==='first'&&clean()==='next'?1:0;}",
        "export function run(a,b){const o={text:' Heading '.trim()};return o.text.startsWith('Head')&&o.text.includes('ing')?o.text.charCodeAt(0):0;}",
        "export function run(a,b){const o={trim:(x,y)=>x+y,trimStart:x=>x,trimEnd:()=>true};return o.trimEnd()?o.trim(a,b)+o.trimStart(a):b;}",
        "function text(x){return ' ordinary ';}export function run(a,b){const o={trim:text,trimStart:text,trimEnd:text};return o.trim(a).length+o.trimStart(b).length+o.trimEnd(a).length;}",
    ] {
        compare_sources(
            source,
            &source.replace("run(a,b)", "run(a: number,b: number)"),
        );
    }
    let js = "export function run(a,b){const text=' x ';return text['trim'](a).length+text.trimStart().length+text.trimEnd().length;}";
    let ts = "export function run(a: number,b: number){const text: number=' x ';return (text as boolean)[('trim' as number)](a as never).length+text.trimStart().length+text.trimEnd().length;}";
    compare_sources(js, ts);
    assert_eq!(
        grunkalunka_compiler::compile(js, false).unwrap().wasm,
        grunkalunka_compiler::compile(ts, true).unwrap().wasm
    );
}

#[test]
fn string_trim_rejects_unproven_receivers_and_unrepresented_call_forms() {
    for body in [
        "return a.trim().length;",
        "const s=a?' x ':null;return s.trimStart().length;",
        "let s=' x ';s=a;return s.trimEnd().length;",
        "const o={text:' x '};o.text=a;return o.text.trim().length;",
        "const xs=[' x '];return xs.at(0).trim().length;",
        "const s=' x ';return s?.trim().length;",
        "const s=' x ';return s.trimStart?.().length;",
        "return ' x '.trimEnd(...[a]).length;",
        "const key='trim';return ' x '[key]().length;",
        "return ' x '.trim.call(' x ').length;",
        "return ' x '.trim();",
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
    assert!(
        grunkalunka_compiler::compile(
            "export function run(a: number,b: number){const s: string=a;return (s as string).trim().length;}",
            true,
        )
        .is_err()
    );
}

#[test]
fn string_trim_cursors_keep_small_stack_and_argument_count_bounds() {
    for (count, admitted, nested) in [
        (24, true, true),
        (140, false, true),
        (1000, true, false),
        (1001, false, false),
    ] {
        let expression = if nested {
            format!("' x '{}", ".trim()".repeat(count))
        } else {
            format!("' x '.trim({})", vec!["a"; count].join(","))
        };
        let source = format!("export function run(a,b){{return {expression}.length;}}");
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || grunkalunka_compiler::compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(
            result.is_ok(),
            admitted,
            "count={count}, nested={nested}: {:?}",
            result.as_ref().err()
        );
        if let Err(error) = result {
            assert!(
                error.contains(if nested { "nesting" } else { "1000" }),
                "{error}"
            );
        }
    }
}

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
fn optional_string_length_executes_the_exact_former_guard_source() {
    for source in [
        "export function run(a,b){const s='x';return s?.length;}",
        "export function run(a,b){const s=a?'x':null;return +(s?.length);}",
        "export function run(a,b){const s=a?'A💠':undefined;return +(s?.length);}",
    ] {
        compare_sources(source, source);
    }
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
fn string_concat_keeps_mixed_coercion_outside_this_slice() {
    for body in [
        "return ('x'+a).length;",
        "return (a+'x').length;",
        "return ('x'+null).length;",
        "return ({}+'x').length;",
        "const value=a?'x':b;return (value+'x').length;",
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

#[test]
fn has_own_string_receivers_have_length_and_canonical_code_unit_positions() {
    for body in [
        "const s='abc';return Object.hasOwn(s,'length')&&Object.hasOwn(s,0)&&Object.hasOwn(s,-0)&&Object.hasOwn(s,'2')&&!Object.hasOwn(s,3)&&!Object.hasOwn(s,'charAt')?1:0;",
        "const s='abc';return !Object.hasOwn(s,'-0')&&!Object.hasOwn(s,'01')&&!Object.hasOwn(s,'+1')&&!Object.hasOwn(s,0.5)&&!Object.hasOwn(s,4294967295)?1:0;",
        r"const s='\u{1f600}\ud800';return Object.hasOwn(s,'2')&&Object.hasOwn(s,1)&&!Object.hasOwn(s,3)&&Object.hasOwn('', 'length')&&!Object.hasOwn('',0)?1:0;",
        "let text='abc',key='2';const found=Object.hasOwn(text,key,(text='',key='missing'));return found&&text.length===0&&!Object.hasOwn(text,key)?1:0;",
        "const row={label:' title '};const {label}=row;return Object.hasOwn(label.trim(),4)&&!Object.hasOwn(label.trim(),5)?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&source, &source);
    }
    let source = "function last(text){return Object.hasOwn(text,1);}export function run(a,b){return last('ab')&&!last('a')?1:0;}";
    common::compare_sources(source, source);
}

#[test]
fn has_own_string_receiver_proofs_reject_incompatible_aliases_and_annotations() {
    for (source, ts) in [
        (
            "export function run(a,b){let s='abc';s=null;return Object.hasOwn(s,0)?1:0;}",
            false,
        ),
        (
            "export function run(a,b){const d={s:'abc'};const alias=d;alias.s=null;return Object.hasOwn(d.s,0)?1:0;}",
            false,
        ),
        (
            "export function run(a:number,b:number){return Object.hasOwn((a?{}:null) as string,0)?1:0;}",
            true,
        ),
    ] {
        let err = grunkalunka_compiler::compile(source, ts)
            .err()
            .unwrap_or_else(|| panic!("admitted {source}"));
        assert!(
            err.contains("unsupported") && !err.contains("internal IR"),
            "{source}: {err}"
        );
    }
}

#[test]
fn string_at_selects_relative_utf16_units_and_preserves_missing_results() {
    for body in [
        "const c='abc'.at(a);return c===undefined?0:c==='a'?1:c==='b'?2:c==='c'?3:-1;",
        "const c='abc'.at(a+1.75);return c===undefined?0:c==='a'?1:c==='b'?2:c==='c'?3:-1;",
        "return 'abc'.at(-1)==='c'&&'abc'.at(-3)==='a'&&'abc'.at(-4)===undefined&&'abc'.at(3)===undefined&&''.at(0)===undefined?1:0;",
        "return 'abc'.at()==='a'&&'abc'.at(undefined)==='a'&&'abc'.at(NaN)==='a'&&'abc'.at(-0)==='a'&&'abc'.at(-0.5)==='a'?1:0;",
        "return 'abc'.at(1e300)===undefined&&'abc'.at(-1e300)===undefined&&'abc'.at(Infinity)===undefined&&'abc'.at(-Infinity)===undefined?1:0;",
        r"return 'A\u{1f600}\ud800'.at(1)==='\ud83d'&&'A\u{1f600}\ud800'.at(-2)==='\ude00'&&'A\u{1f600}\ud800'.at(-1)==='\ud800'&&'\0'.at(0)==='\0'?1:0;",
        "let n=0;let s='abc';const c=(n=1,s)['at']((s='xyz',n=n*10+2,-1),(n=n*10+3));return c==='c'&&s==='xyz'?n:-1;",
        "let n=0;const c='abc'.at(void(n=1),n=n*10+2);return c==='a'?n:-1;",
        "const undefined=-1;return 'abc'.at(undefined)==='c'?1:0;",
        "let n=0;'abc'.at((n=1,-1),n=n*10+2);return n;",
        "const c='abc'.at(a);const box={c};return box.c===undefined?0:typeof box.c==='string'?1:-1;",
        "const x=[1,2,3];return x.at(-1)===3&&'abc'.at(-1)==='c'?1:0;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        compare_sources(&js, &js.replace("run(a,b)", "run(a: number,b: number)"));
    }
    for source in [
        "function last(s){return s.at(-1);}export function run(a,b){return last('menu/')==='/'&&last('')===undefined?1:0;}",
        "export function run(a,b){const text='menu/';const handler=()=>text.at(-1)==='/'?a:b;return handler();}",
        "export function run(a,b){const o={at:x=>x+1};return o.at(a);}",
    ] {
        compare_sources(source, source);
    }
}

#[test]
fn code_point_at_decodes_pairs_and_returns_undefined_outside_the_string() {
    for body in [
        r"const c='A\u{1f600}Z'.codePointAt(a);return c===undefined?0:c===65?1:c===128512?2:c===56832?3:c===90?4:-1;",
        r"const c='A\u{1f600}Z'.codePointAt(a+1.75);return c===undefined?0:c===65?1:c===128512?2:c===56832?3:c===90?4:-1;",
        r"return '\ud800\udc00'.codePointAt(0)===65536&&'\udbff\udfff'.codePointAt(0)===1114111&&'\ud800\udc00'.codePointAt(1)===56320?1:0;",
        r"return '\ud800'.codePointAt(0)===55296&&'\udbffx'.codePointAt(0)===56319&&'\udfff'.codePointAt(0)===57343&&'\udc00\ud800'.codePointAt(0)===56320&&'\ud800\ud800'.codePointAt(0)===55296?1:0;",
        r"return '\0\uffff'.codePointAt(0)===0&&'\0\uffff'.codePointAt(1)===65535&&'e\u0301'.codePointAt(1)===769?1:0;",
        "return 'abc'.codePointAt()===97&&'abc'.codePointAt(undefined)===97&&'abc'.codePointAt(NaN)===97&&'abc'.codePointAt(-0)===97&&'abc'.codePointAt(-0.5)===97?1:0;",
        "return ''.codePointAt(0)===undefined&&'abc'.codePointAt(3)===undefined&&'abc'.codePointAt(-1)===undefined&&'abc'.codePointAt(Infinity)===undefined&&'abc'.codePointAt(-Infinity)===undefined&&'abc'.codePointAt(1e300)===undefined?1:0;",
        "let n=0;let s='abc';const c=(n=1,s)['codePointAt']((s='xyz',n=n*10+2,1),(n=n*10+3));return c===98&&s==='xyz'?n:-1;",
        "let n=0;const c='abc'.codePointAt(void(n=1),n=n*10+2);return c===97?n:-1;",
        "const undefined=1;return 'abc'.codePointAt(undefined)===98?1:0;",
        "let n=0;'abc'.codePointAt((n=1,1),n=n*10+2);return n;",
        "const c='abc'.codePointAt(a);const box={c};return box.c===undefined?0:typeof box.c==='number'?1:-1;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        compare_sources(&js, &js.replace("run(a,b)", "run(a: number,b: number)"));
    }
    for source in [
        "function point(s){return s.codePointAt(0);}export function run(a,b){return point('A')===65&&point('')===undefined?1:0;}",
        "function echo(x){return x;}export function run(a,b){const c='A'.codePointAt(a);return echo(c)===65?1:0;}",
        "export function run(a,b){const text='A';const point=()=>text.codePointAt(a);return point()===65?1:0;}",
        "export function run(a,b){const o={codePointAt:x=>x+1};return o.codePointAt(a);}",
        r"export function run(a,b){let count=0;for(const c of 'A\u{1f600}B'){if(c.codePointAt(0)===128512)count++;}return count;}",
    ] {
        compare_sources(source, source);
    }
}

#[test]
fn string_at_defaults_prove_present_text_and_keep_lazy_fallback_effects() {
    for body in [
        "const {c='?'}={c:'abc'.at(a)};return c.length+c.charCodeAt(0);",
        "const [c='?']=['abc'.at(a)];return c.trim().length+c.charCodeAt(0);",
        "let n=0;const {c=(n=1,'?')}={c:'abc'.at(a)};return n*1000+c.charCodeAt(0);",
        r"const {c='?'}={c:'\u{1f600}'.at(a)};return c.charCodeAt(0);",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        compare_sources(&js, &js.replace("run(a,b)", "run(a: number,b: number)"));
    }
    for source in [
        "function last(s){return s.at(-1);}export function run(a,b){const {c='?'}={c:last(a>0?'menu/':'')};return c.charCodeAt(0);}",
        "function unit(s,i){const {c='?'}={c:s.at(i)};return c;}export function run(a,b){return unit('abc',a).charCodeAt(0);}",
        "export function run(a,b){const text=a>0?'menu/':'';const handle=()=>{const {c='?'}={c:text.at(-1)};return c.charCodeAt(0);};return handle();}",
    ] {
        compare_sources(source, source);
    }
}

#[test]
fn is_well_formed_validates_utf16_and_preserves_effects() {
    for text in [
        r"",
        r"abc\0\uffff",
        r"\ud800",
        r"\udfff",
        r"a\ud800b",
        r"\ud800\udc00",
        r"\udbff\udfff",
        r"\udc00\ud800",
        r"\ud800\ud800\udc00",
        r"\ud800\udc00\udc00",
    ] {
        let s = format!("export function run(a,b){{return '{text}'.isWellFormed()?a:b;}}");
        compare_sources(&s, &s);
    }
    for s in [
        "export function run(a,b){let s='abc';let n=0;const ok=s['isWellFormed']((s='changed',n=1),n=n*10+2);return ok?n:-1;}",
        "function valid(s){return s.isWellFormed();}export function run(a,b){const o={ok:valid('abc')};return o.ok===true?1:0;}",
        "export function run(a,b){const o={isWellFormed:()=>a>0};return o.isWellFormed()?a:b;}",
    ] {
        compare_sources(s, &s.replace("run(a,b)", "run(a: number,b: number)"));
    }
}

#[test]
fn to_well_formed_repairs_lone_surrogates_and_preserves_sources() {
    for (text, expected) in [
        (r"", r""),
        (r"abc\0\uffff", r"abc\0\uffff"),
        (r"\ud800", r"\ufffd"),
        (r"\udfff", r"\ufffd"),
        (r"a\ud800b", r"a\ufffdb"),
        (r"\ud800\udc00", r"\ud800\udc00"),
        (r"\udbff\udfff", r"\udbff\udfff"),
        (r"\udc00\ud800", r"\ufffd\ufffd"),
        (r"\ud800\ud800\udc00", r"\ufffd\ud800\udc00"),
        (r"\ud800\udc00\udc00", r"\ud800\udc00\ufffd"),
    ] {
        let s = format!(
            "export function run(a,b){{const s='{text}';const t=s.toWellFormed();return t==='{expected}'&&s==='{text}'&&t.isWellFormed()&&t.length===s.length?1:0;}}"
        );
        compare_sources(&s, &s);
    }
    for s in [
        r"export function run(a,b){let s='\ud800';let n=0;const t=s['toWellFormed']((s='abc',n=1),n=n*10+2);return t==='\ufffd'&&s==='abc'?n:-1;}",
        r"function repair(s){return s.toWellFormed();}export function run(a,b){const box={label:repair(' \ud800 ')};return box.label.trim()==='\ufffd'?1:0;}",
        "export function run(a,b){const o={toWellFormed:()=>a};return o.toWellFormed();}",
    ] {
        compare_sources(s, &s.replace("run(a,b)", "run(a: number,b: number)"));
    }
}

#[test]
fn concat_method_saves_all_fragments_and_preserves_utf16() {
    for body in [
        "return ''.concat().length+'abc'.concat().length;",
        "return 'a'.concat('b','','c','d')==='abcd'?1:0;",
        r"return '\ud800'.concat('\udc00','\0','\udfff')==='\ud800\udc00\0\udfff'?1:0;",
        "let n=0;const s=(n=1,'a').concat((n=n*10+2,'b'),(n=n*10+3,'c'),(n=n*10+4,'d'));return s==='abcd'?n:-1;",
        "let s='a';let t='b';const result=s['concat'](t,(s='x',t='y','c'));return result==='abc'&&s==='x'&&t==='y'?1:0;",
        "let n=0;'a'.concat((n=1,'b'),(n=n*10+2,'c'));return n;",
        "const o={concat:(x,y)=>x+y};return o.concat(a,b);",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        compare_sources(&js, &js.replace("run(a,b)", "run(a: number,b: number)"));
    }
    compare_sources(
        "function label(s,t){return s.concat(': ',t);}export function run(a,b){const box={label:label('Menu',a>0?'Open':'Close')};return box.label.startsWith('Menu: ')?1:0;}",
        "function label(s: number,t: boolean){return s.concat(': ',t);}export function run(a: number,b: number){const box={label:label('Menu',a>0?'Open':'Close')};return box.label.startsWith('Menu: ')?1:0;}",
    );
}

#[test]
fn template_literals_use_cooked_utf16_and_ordered_string_substitutions() {
    for body in [
        "return `hello`.length+``.length;",
        r"return `a\n\t\u{1f600}\ud800\0`==='a\n\t\u{1f600}\ud800\0'?1:0;",
        r"return `\`\${x}\\`==='`${x}\\'?1:0;",
        "let x='first';const text=`${x}:${(x='second')}:${x}`;return text==='first:second:second'?1:0;",
        "let n=0;const text=`pre${(n=1,'a')}mid${(n=n*10+2,'b')}post`;return text==='preamidbpost'?n:-1;",
        "const name=a>0?'Open':'Close';return `Menu: ${name}`.startsWith('Menu: ')?1:0;",
        "return `x${`y${'z'}`}q`==='xyzq'?1:0;",
        r"return `\ud800${'\udc00'}`==='\ud800\udc00'?1:0;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        compare_sources(&js, &js.replace("run(a,b)", "run(a: number,b: number)"));
    }
    let s = "function label(s){return `Menu: ${s}`;}export function run(a,b){const box={text:label(a>0?'Open':'Close')};return box.text.startsWith('Menu: ')?1:0;}";
    compare_sources(s, s);
}

#[test]
fn numeric_string_index_preserves_property_key_semantics_and_snapshots() {
    for body in [
        "const c='abc'[a];return c===undefined?0:c==='a'?1:c==='b'?2:c==='c'?3:-1;",
        "const c='abc'[a+1.5];return c===undefined?0:c==='a'?1:c==='b'?2:c==='c'?3:-1;",
        "return 'abc'[-0]==='a'&&'abc'[-1]===undefined&&'abc'[1.5]===undefined&&'abc'[NaN]===undefined&&'abc'[Infinity]===undefined&&'abc'[1e300]===undefined&&''[0]===undefined?1:0;",
        r"return '\u{1f600}\ud800\0'[0]==='\ud83d'&&'\u{1f600}\ud800\0'[1]==='\ude00'&&'\u{1f600}\ud800\0'[2]==='\ud800'&&'\u{1f600}\ud800\0'[3]==='\0'?1:0;",
        "let s='abc';const c=s[(s='xyz',1)];return c==='b'&&s==='xyz'?1:0;",
        "let n=0;const c=(n=1,'abc')[(n=n*10+2,1)];return c==='b'?n:-1;",
        "return 'abc'['length']===3&&'abc'.length===3?1:0;",
        "let n=0;'abc'[(n=1,0)];return n;",
    ] {
        let s = format!("export function run(a,b){{{body}}}");
        compare_sources(&s, &s.replace("run(a,b)", "run(a: number,b: number)"));
    }
    for s in [
        "function character(s,i){return s[i];}export function run(a,b){return character('abc',a)===undefined?0:1;}",
        "export function run(a,b){const text='abc';const handle=()=>text[a];return handle()==='a'?1:0;}",
    ] {
        compare_sources(s, s);
    }
}
