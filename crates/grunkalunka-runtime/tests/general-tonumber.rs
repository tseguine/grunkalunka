use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};

#[test]
fn null_local_converts_to_positive_zero_in_unary_plus() {
    let source = "export function run(a,b) {let value=null; return +value;}";
    let artifact = compile(source, false).unwrap();
    let actual = NumericInstance::new(&artifact.wasm)
        .unwrap()
        .run(4.0, 2.0)
        .unwrap();
    assert_eq!(actual.to_bits(), 0.0_f64.to_bits());
}

#[test]
fn undefined_local_converts_to_nan_instead_of_its_zero_payload() {
    let source = "export function run(a,b) {let value=undefined; return +value;}";
    let artifact = compile(source, false).unwrap();
    let actual = NumericInstance::new(&artifact.wasm)
        .unwrap()
        .run(4.0, 2.0)
        .unwrap();
    assert!(actual.is_nan(), "{actual}");
}

fn encode(value: &Value) -> String {
    match value {
        Value::Undefined => "[0]".into(),
        Value::Null => "[1]".into(),
        Value::Boolean(b) => format!("[2,{b}]"),
        Value::Number(n) => format!(
            "[3,\"{}\"]",
            if n.is_nan() {
                "nan".into()
            } else {
                format!("{:016x}", n.to_bits())
            }
        ),
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
fn canonical_primitive_arithmetic_matches_fixed_numeric_goldens() {
    let inputs = [
        ("undefined", f64::NAN),
        ("null", 0.0),
        ("false", 0.0),
        ("true", 1.0),
        ("0", 0.0),
        ("-0", -0.0),
        ("NaN", f64::NAN),
        ("1", 1.0),
        ("-1", -1.0),
        ("Infinity", f64::INFINITY),
        ("-Infinity", f64::NEG_INFINITY),
        ("5e-324", f64::from_bits(1)),
    ];
    let mut cases = Vec::new();
    for (literal, number) in inputs {
        for operator in ["+", "-"] {
            let expected = if operator == "+" { number } else { -number };
            cases.push((
                format!("let value=null; value={literal}; return {operator}value;"),
                expected,
            ));
        }
        for (right, right_number) in inputs {
            for operator in ["+", "-", "*", "/", "%"] {
                let expected = match operator {
                    "+" => number + right_number,
                    "-" => number - right_number,
                    "*" => number * right_number,
                    "/" => number / right_number,
                    _ => number % right_number,
                };
                cases.push((format!("let left=null,right=null; left={literal}; right={right}; return left{operator}right;"),expected));
            }
        }
    }
    let mut records = Vec::new();
    for (body, expected) in cases {
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
                let actual = if value {
                    ValueInstance::new(&artifact.wasm)
                        .unwrap()
                        .run(4.0, 2.0)
                        .unwrap()
                } else {
                    Value::Number(
                        NumericInstance::new(&artifact.wasm)
                            .unwrap()
                            .run(4.0, 2.0)
                            .unwrap(),
                    )
                };
                let golden = encode(&Value::Number(expected));
                assert_eq!(encode(&actual), golden, "{source}");
                records.push(format!(
                    "{{source:{js:?},bytes:{:?},value:{value},expected:{golden}}}",
                    artifact.wasm
                ));
            }
        }
    }
    let script = format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}],view=new DataView(new ArrayBuffer(8));
function encode(x){{assert.equal(typeof x,'number');if(Number.isNaN(x))return[3,'nan'];view.setFloat64(0,x);return[3,view.getBigUint64(0).toString(16).padStart(16,'0')];}}
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();assert.deepEqual(encode(original(4,2)),c.expected,c.source+' golden');const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;let actual=run(4,2);if(c.value){{assert.equal(actual[0],3);actual=actual[1];}}assert.deepEqual(encode(actual),c.expected,c.source+' Node-Wasm');}}
console.log('PASS '+cases.length+' primitive arithmetic modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 2976 primitive arithmetic modules");
}

#[test]
fn arithmetic_coercion_preserves_effect_order_and_operand_snapshots() {
    let bodies = [
        "let value=null; return +(value=a?true:undefined);",
        "let value=true; return value+(value=false);",
        "let value=null; value=a; return value-(value=b);",
        "let value=null; return (value=a?true:null)/(value=b?false:true);",
        "const value=a?null:b; return value*b;",
        "let value=null; const before=+(value=b++); return before+b;",
        "let value=null; return +(a?(value=true):(value=false))+(value?b:a);",
        "const value=a&&false; return value+b;",
        "const value=a||true; return -value;",
        "let value=undefined; return (value=a&&null)+b;",
        "let value=null; return (value=a,b)+(value=true);",
        "let value=null; value=a; const alias=value; value=null; return +alias+b;",
        "let result=0; for(let i=0;i<3;i++){let value=null; value=i?false:a; result=result+(+value);} return result;",
        "let value=null; return (value=b++)*(value=b++);",
        "let value=a; {let value=null; value=true; +value;} return value;",
        "let value=null; const left=(value=true); return left+(value=false);",
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
                let observed = if value {
                    let mut i = ValueInstance::new(&artifact.wasm).unwrap();
                    vectors
                        .iter()
                        .map(|(a, b)| encode(&i.run(*a, *b).unwrap()))
                        .collect::<Vec<_>>()
                } else {
                    let mut i = NumericInstance::new(&artifact.wasm).unwrap();
                    vectors
                        .iter()
                        .map(|(a, b)| encode(&Value::Number(i.run(*a, *b).unwrap())))
                        .collect::<Vec<_>>()
                };
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
function encode(x){{assert.equal(typeof x,'number');if(Number.isNaN(x))return[3,'nan'];view.setFloat64(0,x);return[3,view.getBigUint64(0).toString(16).padStart(16,'0')];}}
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;for(let i=0;i<vectors.length;i++){{const expected=encode(original(...vectors[i]));assert.deepEqual(c.observed[i],expected,c.source+' Wasmtime');let actual=run(...vectors[i]);if(c.value){{assert.equal(actual[0],3);actual=actual[1];}}assert.deepEqual(encode(actual),expected,c.source+' Node-Wasm');}}}}
console.log('PASS '+cases.length+' primitive arithmetic effect modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 64 primitive arithmetic effect modules");
}

#[test]
fn exact_previous_guard_sources_now_execute_fixed_results() {
    let cases = [
        ("export function run(a,b) {return null+1;}", 1.0),
        (
            "export function run(a,b) { const flag=true; return +(a?flag:false); }",
            1.0,
        ),
        (
            "export function run(a,b) { let flag=true; return +flag; }",
            1.0,
        ),
        ("export function run(a,b) { return +(true&&false); }", 0.0),
        ("export function run(a,b) { return -(a?true:false); }", -1.0),
        ("export function run(a,b) { return +(0,true); }", 1.0),
        ("export function run(a,b) { return (a?true:false)+1; }", 2.0),
        (
            "export function run(a,b) { const flag=a?(b?true:false):!a; return +flag; }",
            1.0,
        ),
        (
            "export function run(a,b) {const value=null; return +value;}",
            0.0,
        ),
        (
            "export function run(a,b) {let value=null; return +value;}",
            0.0,
        ),
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
                    ValueInstance::new(&artifact.wasm)
                        .unwrap()
                        .run(4.0, 2.0)
                        .unwrap()
                } else {
                    Value::Number(
                        NumericInstance::new(&artifact.wasm)
                            .unwrap()
                            .run(4.0, 2.0)
                            .unwrap(),
                    )
                };
                let expected = encode(&Value::Number(expected));
                assert_eq!(encode(&actual), expected);
                records.push(format!(
                    "{{source:{source:?},bytes:{:?},value:{value},expected:{expected}}}",
                    artifact.wasm
                ));
            }
        }
    }
    let script = format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}],view=new DataView(new ArrayBuffer(8));
function encode(x){{assert.equal(typeof x,'number');if(Number.isNaN(x))return[3,'nan'];view.setFloat64(0,x);return[3,view.getBigUint64(0).toString(16).padStart(16,'0')];}}
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();assert.deepEqual(encode(original(4,2)),c.expected);const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;let actual=run(4,2);if(c.value){{assert.equal(actual[0],3);actual=actual[1];}}assert.deepEqual(encode(actual),c.expected);}}
console.log('PASS '+cases.length+' exact former arithmetic guards');
"#,
        records.join(",")
    );
    node(&script, "PASS 40 exact former arithmetic guards");
}

#[test]
fn primitive_coercion_does_not_widen_other_source_contracts() {
    for body in [
        "return null;",
        "return true;",
        "return +\"1\";",
        "return {}+a;",
        "return a&true;",
        "const value=null; return ~value;",
        "let value=null; value&&=a; return a;",
        "return +value; let value=null;",
        "const value=null; value=1; return +value;",
    ] {
        let source = format!("export function run(a,b) {{{body}}}");
        let error = compile(&source, false).err().unwrap();
        assert!(error.contains("unsupported"), "{source}: {error}");
        assert!(!error.contains("internal IR"));
    }
    let recursive = compile_value("export function run(a,b) {return run(0,b)+1;}", false).unwrap();
    let error = ValueInstance::new(&recursive.wasm)
        .unwrap()
        .run(0.0, 0.0)
        .unwrap_err();
    assert!(format!("{error:#}").contains("fuel"));
    for ts in [false, true] {
        let source = "export function run(a,b) {return a?null:run(1,b)+1;}";
        let artifact = compile_value(source, ts).unwrap();
        let mut i = ValueInstance::new(&artifact.wasm).unwrap();
        assert!(matches!(i.run(0.0, 0.0).unwrap(), Value::Number(1.0)));
        assert!(matches!(i.run(1.0, 0.0).unwrap(), Value::Null));
    }
}

#[test]
fn general_number_payload_preserves_negative_zero_and_non_nan_bits() {
    let artifact = compile(
        "export function run(a,b) {let value=null; value=a; return +value;}",
        false,
    )
    .unwrap();
    let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
    for value in [
        -0.0,
        0.0,
        -1.0,
        f64::from_bits(1),
        f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ] {
        let actual = instance.run(value, 0.0).unwrap();
        if value.is_nan() {
            assert!(actual.is_nan());
        } else {
            assert_eq!(actual.to_bits(), value.to_bits());
        }
    }
}
