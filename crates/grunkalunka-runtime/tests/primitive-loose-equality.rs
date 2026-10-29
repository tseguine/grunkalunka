use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};
#[test]
fn null_and_undefined_compare_loosely_equal() {
    let artifact = compile(
        "export function run(a,b) {return null==undefined?1:0;}",
        false,
    )
    .unwrap();
    assert_eq!(
        NumericInstance::new(&artifact.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap(),
        1.0
    );
}

#[test]
fn null_stays_distinct_from_numeric_zero_despite_equal_payloads() {
    let artifact = compile("export function run(a,b) {return null==0?1:0;}", false).unwrap();
    assert_eq!(
        NumericInstance::new(&artifact.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap(),
        0.0
    );
}

fn encode(value: &Value) -> String {
    match value {
        grunkalunka_runtime::Value::TypeError { .. }
        | grunkalunka_runtime::Value::Promise { .. } => {
            panic!("primitive-only fixture received an async object")
        }
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

fn standard_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Undefined, Value::Undefined)
        | (Value::Null, Value::Null)
        | (Value::Undefined, Value::Null)
        | (Value::Null, Value::Undefined) => true,
        (Value::Boolean(a), Value::Boolean(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::Boolean(b), Value::Number(n)) | (Value::Number(n), Value::Boolean(b)) => {
            *n == if *b { 1.0 } else { 0.0 }
        }
        _ => false,
    }
}
#[test]
fn four_kind_loose_equality_matches_independent_standard_goldens() {
    let values = [
        ("undefined", Value::Undefined),
        ("null", Value::Null),
        ("false", Value::Boolean(false)),
        ("true", Value::Boolean(true)),
        ("0", Value::Number(0.0)),
        ("-0", Value::Number(-0.0)),
        ("NaN", Value::Number(f64::NAN)),
        ("1", Value::Number(1.0)),
        ("-1", Value::Number(-1.0)),
        ("Infinity", Value::Number(f64::INFINITY)),
        ("-Infinity", Value::Number(f64::NEG_INFINITY)),
        ("5e-324", Value::Number(f64::from_bits(1))),
    ];
    let mut records = Vec::new();
    for (left, left_value) in &values {
        for (right, right_value) in &values {
            for negated in [false, true] {
                let expected = standard_equal(left_value, right_value) ^ negated;
                let op = if negated { "!=" } else { "==" };
                for ts in [false, true] {
                    for value in [false, true] {
                        let expression = format!("left{op}right");
                        let result = if value {
                            expression
                        } else {
                            format!("({expression})?1:0")
                        };
                        let js = format!(
                            "export function run(a,b) {{let left=null,right=null; left={left}; right={right}; return {result};}}"
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
                        let golden = encode(&if value {
                            Value::Boolean(expected)
                        } else {
                            Value::Number(if expected { 1.0 } else { 0.0 })
                        });
                        assert_eq!(encode(&actual), golden, "{source}");
                        records.push(format!(
                            "{{source:{js:?},bytes:{:?},value:{value},expected:{golden}}}",
                            artifact.wasm
                        ));
                    }
                }
            }
        }
    }
    let script = format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}],view=new DataView(new ArrayBuffer(8));
function encode(x){{if(typeof x==='boolean')return[2,x];assert.equal(typeof x,'number');if(Number.isNaN(x))return[3,'nan'];view.setFloat64(0,x);return[3,view.getBigUint64(0).toString(16).padStart(16,'0')];}}
function decode([tag,payload]){{if(tag===2){{assert.ok(Object.is(payload,0)||payload===1);return!!payload;}}assert.equal(tag,3);return payload;}}
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();assert.deepEqual(encode(original(4,2)),c.expected,c.source+' standardgolden');const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;const actual=run(4,2);assert.deepEqual(encode(c.value?decode(actual):actual),c.expected,c.source+' Node-Wasm');}}
console.log('PASS '+cases.length+' primitive loose equality modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 1152 primitive loose equality modules");
}

#[test]
fn loose_equality_captures_operands_once_before_later_mutation() {
    let bodies = [
        "let x=0; const left=(x++,null); const result=left==(x++,undefined); return (result?10:0)+x;",
        "let value=a; const equal=value==(value=true); return equal?a:b;",
        "let value=null; const equal=value==(value=0); return equal?1:0;",
        "let value=false; const equal=value==(value=0); return equal?1:0;",
        "const equal=a==(a=true); return equal?b:0;",
        "const result=a?(true==b):(null==undefined); return result?1:0;",
        "let value=null; if(value==undefined){value=a;} return value==a?b:a;",
        "let sum=0; for(let i=0;i<3;i++){if((i<1)==true)sum+=a;} return sum;",
        "const left=a?null:false; const right=b?undefined:0; return left==right?a:b;",
        "let count=0; return ((count++,null)!=(count++,undefined))?count:-count;",
        "let count=0; (a?null:b)==(count++,undefined); return count;",
        "let value=null; return (value=false)!=(value=a)?1:0;",
        "a=false; b=null; return a==b?1:0;",
        "let count=0; while((count<3)==true){count++;} return count;",
        "const saved=a; a=null; return saved==b?1:0;",
        "const left=a?undefined:true; const right=b?null:1; return left!=right?a:b;",
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
console.log('PASS '+cases.length+' loose equality effect modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 64 loose equality effect modules");
}

#[test]
fn exact_former_loose_equality_guards_execute_fixed_results() {
    let cases = [
        ("export function run(a,b) { return true==false?1:0; }", 0.0),
        ("export function run(a,b) { return true!=false?1:0; }", 1.0),
        ("export function run(a,b) { return ~(true==a); }", -1.0),
        ("export function run(a,b) { return true==a?1:0; }", 0.0),
        ("export function run(a,b) { return true==false?1:2; }", 2.0),
        (
            "export function run(a,b) {const value=null; return value==0?1:0;}",
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
                assert_eq!(encode(&actual), encode(&Value::Number(expected)));
                records.push(format!(
                    "{{source:{source:?},bytes:{:?},value:{value},expected:{expected}}}",
                    artifact.wasm
                ));
            }
        }
    }
    let script = format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}];for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();assert.ok(Object.is(original(4,2),c.expected));const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;let actual=run(4,2);if(c.value){{assert.equal(actual[0],3);actual=actual[1];}}assert.ok(Object.is(actual,c.expected));}}console.log('PASS '+cases.length+' exact loose equality guard modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 24 exact loose equality guard modules");
}

#[test]
fn loose_inequality_negates_the_primitive_result() {
    for (body, expected) in [
        ("return null!=undefined?1:0;", 0.0),
        ("return true!=1?1:0;", 0.0),
        ("let value=null; value=NaN; return value!=value?1:0;", 1.0),
    ] {
        let source = format!("export function run(a,b) {{{body}}}");
        let artifact = compile(&source, false).unwrap();
        assert_eq!(
            NumericInstance::new(&artifact.wasm)
                .unwrap()
                .run(4.0, 2.0)
                .unwrap(),
            expected
        );
    }
}

#[test]
fn loose_equality_retains_unrepresented_kind_scope_and_depth_guards() {
    for body in [
        "return a==\"4\"?1:0;",
        "return 1n==a?1:0;",
        "return {}==a?1:0;",
        "return Symbol()==a?1:0;",
        "return true==a;",
        "return value==null?1:0; let value=null;",
        "const value=null; return (value=a)==a?1:0;",
        "return a?1:(Math.random()==null?1:0);",
    ] {
        let source = format!("export function run(a,b) {{{body}}}");
        let error = compile(&source, false).err().unwrap();
        assert!(error.contains("unsupported"), "{source}: {error}");
        assert!(!error.contains("internal IR"));
    }
    for depth in [126, 127] {
        let source = format!(
            "export function run(a,b) {{return {}true==false{}?a:b;}}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        let result = compile(&source, false);
        if depth == 126 {
            result.unwrap();
        } else {
            assert!(result.err().unwrap().contains("expression nesting exceeds"));
        }
    }
}
