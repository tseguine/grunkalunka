use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};
#[test]
fn null_less_than_one_converts_left_operand() {
    let artifact = compile("export function run(a,b) {return null<1?1:0;}", false).unwrap();
    assert_eq!(
        NumericInstance::new(&artifact.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap(),
        1.0
    );
}

#[test]
fn one_less_than_true_converts_right_operand() {
    let artifact = compile("export function run(a,b) {return 1<true?1:0;}", false).unwrap();
    assert_eq!(
        NumericInstance::new(&artifact.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap(),
        0.0
    );
}

fn number_bits(value: f64) -> String {
    if value.is_nan() {
        "nan".into()
    } else {
        format!("{:016x}", value.to_bits())
    }
}
fn node(script: &str, expected: &str) {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new("node")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(String::from_utf8(result.stdout).unwrap().trim(), expected);
}

#[test]
fn four_kind_relations_match_independent_numeric_goldens() {
    let values = [
        ("undefined", f64::NAN),
        ("null", 0.0),
        ("false", 0.0),
        ("true", 1.0),
        ("-0", -0.0),
        ("NaN", f64::NAN),
        ("Infinity", f64::INFINITY),
        ("1.5", 1.5),
    ];
    let mut records = Vec::new();
    for (left, x) in values {
        for (right, y) in values {
            for op in ["<", "<=", ">", ">="] {
                let expected = match op {
                    "<" => x < y,
                    "<=" => x <= y,
                    ">" => x > y,
                    ">=" => x >= y,
                    _ => unreachable!(),
                };
                for ts in [false, true] {
                    for value in [false, true] {
                        let result = if value {
                            format!("left{op}right")
                        } else {
                            format!("left{op}right?1:0")
                        };
                        let js = format!(
                            "export function run(a,b) {{let left=null,right=null; left={left};right={right};return {result};}}"
                        );
                        let source = if ts {
                            js.replace("run(a,b)", "run(a:number,b:number)")
                        } else {
                            js.clone()
                        };
                        let artifact = if value {
                            compile_value(&source, ts)
                        } else {
                            compile(&source, ts)
                        }
                        .unwrap();
                        if value {
                            let actual = ValueInstance::new(&artifact.wasm)
                                .unwrap()
                                .run(4.0, 2.0)
                                .unwrap();
                            match actual {
                                Value::Boolean(b) => assert_eq!(b, expected, "{source}"),
                                other => panic!("expected Boolean: {other:?}"),
                            }
                        } else {
                            assert_eq!(
                                NumericInstance::new(&artifact.wasm)
                                    .unwrap()
                                    .run(4.0, 2.0)
                                    .unwrap(),
                                if expected { 1.0 } else { 0.0 },
                                "{source}"
                            );
                        }
                        records.push(format!(
                            "{{source:{js:?},bytes:{:?},value:{value},expected:{expected}}}",
                            artifact.wasm
                        ));
                    }
                }
            }
        }
    }
    let script = format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}];
for(const c of cases){{const expected=c.value?c.expected:Number(c.expected);const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();assert.equal(original(4,2),expected,c.source+' standardgolden');const module=new WebAssembly.Module(Uint8Array.from(c.bytes));assert.deepEqual(WebAssembly.Module.imports(module),[]);let actual=new WebAssembly.Instance(module).exports.run(4,2);if(c.value){{assert.equal(actual[0],2);assert.ok(Object.is(actual[1],0)||actual[1]===1);actual=!!actual[1];}}assert.equal(actual,expected,c.source+' Node-Wasm');}}console.log('PASS '+cases.length+' primitive relational modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 1024 primitive relational modules");
}

#[test]
fn relational_operands_preserve_left_to_right_effects_and_snapshots() {
    let bodies = [
        "let value=null; value=a; return value<(value=true)?a:b;",
        "let value=true; return value>(value=null)?a:b;",
        "a=null; return a>=(a=b)?1:0;",
        "a=false; return a<=(a=b)?1:0;",
        "let count=0; const result=(count++,null)<=(count++,false); return (result?10:0)+count;",
        "let left=a?null:false,right=b?undefined:true; return left<right?a:b;",
        "let value=null; for(let i=0;i<3;i++){if(value<=false)value+=true;} return +value;",
        "let value=undefined; return value<=null?1:0;",
        "let value=undefined; return value>=null?1:0;",
        "let value=null; value=a; return (value=b)<a?b:a;",
        "if(a){let value=null;return value<run(0,b)?a:b;}return b;",
        "let value=null; const result=value<a; value=b; return result?a:b;",
        "const flag=a<b; return flag<=true?a:b;",
        "let value; return value>b?a:b;",
        "let value=false; const alias=value; value=a; return alias>=value?a:b;",
        "let count=0; while(count<true){count++;} return count+a;",
    ];
    let vectors = [
        (0.0, 0.0),
        (-0.0, -0.0),
        (4.0, 2.0),
        (0.0, -1.0),
        (1.0, -0.0),
        (f64::NAN, 1.0),
        (0.0, f64::NAN),
        (f64::INFINITY, f64::NEG_INFINITY),
    ];
    let mut records = Vec::new();
    for body in bodies {
        let js = format!("export function run(a,b) {{{body}}}");
        for ts in [false, true] {
            let source = if ts {
                js.replace("run(a,b)", "run(a:number,b:number)")
            } else {
                js.clone()
            };
            for value in [false, true] {
                let artifact = if value {
                    compile_value(&source, ts)
                } else {
                    compile(&source, ts)
                }
                .unwrap();
                let mut observed = Vec::new();
                for (a, b) in vectors {
                    let n = if value {
                        match ValueInstance::new(&artifact.wasm)
                            .unwrap()
                            .run(a, b)
                            .unwrap()
                        {
                            Value::Number(n) => n,
                            other => panic!("expected Number: {other:?}"),
                        }
                    } else {
                        NumericInstance::new(&artifact.wasm)
                            .unwrap()
                            .run(a, b)
                            .unwrap()
                    };
                    observed.push(format!("{:?}", number_bits(n)));
                }
                records.push(format!(
                    "{{source:{js:?},bytes:{:?},value:{value},observed:[{}]}}",
                    artifact.wasm,
                    observed.join(",")
                ));
            }
        }
    }
    let script = format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}],vectors=[[0,0],[-0,-0],[4,2],[0,-1],[1,-0],[NaN,1],[0,NaN],[Infinity,-Infinity]],view=new DataView(new ArrayBuffer(8));
function bits(n){{assert.equal(typeof n,'number');if(Number.isNaN(n))return'nan';view.setFloat64(0,n);return view.getBigUint64(0).toString(16).padStart(16,'0');}}
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;for(let i=0;i<vectors.length;i++){{const expected=bits(original(...vectors[i]));assert.equal(c.observed[i],expected,c.source+' Wasmtime');let actual=run(...vectors[i]);if(c.value){{assert.equal(actual[0],3);actual=actual[1];}}assert.equal(bits(actual),expected,c.source+' Node-Wasm');}}}}console.log('PASS '+cases.length+' primitive relational effect modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 64 primitive relational effect modules");
}

#[test]
fn exact_previous_relational_guards_execute_original_sources() {
    let cases = [
        ("export function run(a,b) { return true<false?1:0; }", 0.0),
        (
            "export function run(a,b) {const value=null; return value<1?1:0;}",
            1.0,
        ),
        ("export function run(a,b) {return true<a?1:0;}", 1.0),
    ];
    let mut records = Vec::new();
    for (source, expected) in cases {
        for ts in [false, true] {
            for value in [false, true] {
                let artifact = if value {
                    compile_value(source, ts)
                } else {
                    compile(source, ts)
                }
                .unwrap();
                let actual = if value {
                    match ValueInstance::new(&artifact.wasm)
                        .unwrap()
                        .run(4.0, 2.0)
                        .unwrap()
                    {
                        Value::Number(n) => n,
                        other => panic!("{other:?}"),
                    }
                } else {
                    NumericInstance::new(&artifact.wasm)
                        .unwrap()
                        .run(4.0, 2.0)
                        .unwrap()
                };
                assert_eq!(actual, expected);
                records.push(format!(
                    "{{source:{source:?},bytes:{:?},value:{value},expected:{expected}}}",
                    artifact.wasm
                ));
            }
        }
    }
    node(
        &format!(
            r#"const assert=require('node:assert/strict'); for(const c of [{}]){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();assert.equal(original(4,2),c.expected);let n=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run(4,2);if(c.value){{assert.equal(n[0],3);n=n[1];}}assert.equal(n,c.expected);}}console.log('PASS 12 exact relational guard modules');"#,
            records.join(",")
        ),
        "PASS 12 exact relational guard modules",
    );
}

#[test]
fn relational_conversions_keep_unrepresented_kind_and_binding_guards() {
    for body in [
        "return a<\"4\"?1:0;",
        "return 1n<=a?1:0;",
        "return ({})>a?1:0;",
        "return Symbol()>=a?1:0;",
        "return a==\"4\"?1:0;",
        "return a===\"4\"?1:0;",
        "return (\"4\" as number)<a?1:0;",
        "return value<null?1:0;let value=null;",
        "const value=null;return (value=a)<a?1:0;",
        "if(false)return Math.random()<null?1:0;return a;",
        "return null<true;",
    ] {
        let source = format!("export function run(a,b) {{{body}}}");
        let error = compile(&source, true).err().unwrap();
        assert!(!error.contains("internal IR"), "{source}: {error}");
    }
}

#[test]
fn primitive_relations_retain_depth_local_and_statement_budgets() {
    for (depth, admitted) in [(125, true), (129, false)] {
        let source = format!(
            "export function run(a,b) {{let value=null;return {}value<true{}?a:b;}}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        let result = compile(&source, false);
        if admitted {
            result.unwrap();
        } else {
            assert!(result.err().unwrap().contains("predicate nesting exceeds"));
        }
    }
    for count in [128, 129] {
        let declarations = (0..count)
            .map(|i| format!("x{i}=null"))
            .collect::<Vec<_>>()
            .join(",");
        let source = format!("export function run(a,b) {{let {declarations};return x0<true?a:b;}}");
        let result = compile(&source, false);
        if count == 128 {
            result.unwrap();
        } else {
            assert!(result.err().unwrap().contains("at most 128 source locals"));
        }
    }
    for count in [254, 255] {
        let source = format!(
            "export function run(a,b) {{let value=null;{}return a;}}",
            "value<true;".repeat(count)
        );
        let result = compile(&source, false);
        if count == 254 {
            result.unwrap();
        } else {
            assert!(result.err().unwrap().contains("at most 256 statements"));
        }
    }
}
