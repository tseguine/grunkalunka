use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};
#[test]
fn later_plain_write_can_change_a_number_initialized_local_kind() {
    let source = "export function run(a,b) {let value=a; value=null; return value;}";
    let artifact = compile_value(source, false).unwrap();
    let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
    assert!(matches!(instance.run(4.0, 2.0).unwrap(), Value::Null));
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
fn local_plain_write_matrix_preserves_the_rhs_kind_and_payload() {
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
    for (initial, _) in &values {
        for (right, expected_value) in &values {
            for ts in [false, true] {
                for value in [false, true] {
                    let result = if value {
                        "value".to_owned()
                    } else {
                        format!("value===({right})?1:0")
                    };
                    let js = format!(
                        "export function run(a,b) {{let value={initial}; value={right}; return {result};}}"
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
                    let expected = if value {
                        encode(expected_value)
                    } else {
                        encode(&Value::Number(
                            if matches!(expected_value,Value::Number(n) if n.is_nan()) {
                                0.0
                            } else {
                                1.0
                            },
                        ))
                    };
                    let observed = if value {
                        encode(
                            &ValueInstance::new(&artifact.wasm)
                                .unwrap()
                                .run(4.0, 2.0)
                                .unwrap(),
                        )
                    } else {
                        encode(&Value::Number(
                            NumericInstance::new(&artifact.wasm)
                                .unwrap()
                                .run(4.0, 2.0)
                                .unwrap(),
                        ))
                    };
                    assert_eq!(observed, expected, "{source}");
                    records.push(format!(
                        "{{source:{js:?},bytes:{:?},value:{value},expected:{expected}}}",
                        artifact.wasm
                    ));
                }
            }
        }
    }
    let script = format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}],view=new DataView(new ArrayBuffer(8));
function encode(x){{if(x===undefined)return[0];if(x===null)return[1];if(typeof x==='boolean')return[2,x];assert.equal(typeof x,'number');if(Number.isNaN(x))return[3,'nan'];view.setFloat64(0,x);return[3,view.getBigUint64(0).toString(16).padStart(16,'0')];}}
function decode([tag,payload]){{if(tag===0||tag===1){{assert.ok(Object.is(payload,0));return tag===0?undefined:null;}}if(tag===2){{assert.ok(Object.is(payload,0)||payload===1);return!!payload;}}assert.equal(tag,3);return payload;}}
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();assert.deepEqual(encode(original(4,2)),c.expected,c.source+' golden');const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;const actual=run(4,2);assert.deepEqual(encode(c.value?decode(actual):actual),c.expected,c.source+' Node-Wasm');}}
console.log('PASS '+cases.length+' mutable write modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 576 mutable write modules");
}

#[test]
fn write_planning_aliases_and_assignment_snapshots_match_js() {
    let cases = [
        (
            "let value=a; let alias=value; value=null; return alias;",
            false,
        ),
        (
            "let first=a,second=b; first=second; second=null; return first;",
            false,
        ),
        (
            "let first=a,second=b; first=second; second=first; second=null; return first;",
            false,
        ),
        ("let flag=true; flag=false; return flag;", false),
        (
            "let value=null; const old=value; value=a; const result=(value=false); return old===null?result:undefined;",
            false,
        ),
        (
            "let value=null; const result=(value=b); return result;",
            true,
        ),
        (
            "let left=null,right=false; return left=(right=a?true:b);",
            false,
        ),
        ("let value=null; if(value=b) return a; return b;", true),
        (
            "let value=0,n=a; value=null; n+=b; value=n; return value===n?1:0;",
            true,
        ),
        (
            "let value=null,i=0; while(i<2){value=i?false:a;i++;} return value;",
            false,
        ),
        (
            "let sum=0; for(let i=0;i<3;i++){let value=a; if(i<2)value=null; if(value===null)sum+=b; else sum+=a;} return sum;",
            true,
        ),
        (
            "let value=0; value=null; const old=(value=b++); return old+b;",
            true,
        ),
        ("let value=null; return (value=(value=a,b));", false),
        (
            "let value=a; {let value=null; value=false;} return value;",
            true,
        ),
        ("let value=0; if(false)value=null; return value;", false),
        (
            "let value=null; const first=(value=a),second=(value=b); return first+second;",
            true,
        ),
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
    for (body, numeric) in cases {
        for ts in [false, true] {
            for value in [false, true] {
                if !value && !numeric {
                    continue;
                }
                let js = format!("export function run(a,b) {{{body}}}");
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
function encode(x){{if(x===undefined)return[0];if(x===null)return[1];if(typeof x==='boolean')return[2,x];assert.equal(typeof x,'number');if(Number.isNaN(x))return[3,'nan'];view.setFloat64(0,x);return[3,view.getBigUint64(0).toString(16).padStart(16,'0')];}}
function decode([tag,payload]){{if(tag===0||tag===1){{assert.ok(Object.is(payload,0));return tag===0?undefined:null;}}if(tag===2){{assert.ok(Object.is(payload,0)||payload===1);return!!payload;}}assert.equal(tag,3);return payload;}}
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;for(let i=0;i<vectors.length;i++){{const expected=encode(original(...vectors[i]));assert.deepEqual(c.observed[i],expected,c.source+' Wasmtime');const actual=run(...vectors[i]);assert.deepEqual(encode(c.value?decode(actual):actual),expected,c.source+' Node-Wasm');}}}}
console.log('PASS '+cases.length+' mutable snapshot modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 46 mutable snapshot modules");
}

#[test]
fn mutable_layout_planning_does_not_publish_bindings_or_relax_other_operators() {
    for body in [
        "let value=value; value=null; return a;",
        "let value=other; let other=0; other=null; return a;",
        "let value=a; {let value=value;} return a;",
        "{let value=null;} return value;",
        "const value=null; value=a; return b;",
        "let value; value=null; return value;",
        "let value=null; value+=1; return value;",
        "let value=null; value++; return value;",
        "let value=null; value&&=a; return value;",
        "let value=null; value||=a; return value;",
        "a=null; return a;",
        "let value=null; return +value;",
    ] {
        let source = format!("export function run(a,b) {{{body}}}");
        let error = compile_value(&source, false).err().unwrap();
        assert!(
            error.contains("unsupported") || error.contains("initializer"),
            "{error}"
        );
        assert!(!error.contains("internal IR"), "{error}");
    }
}

#[test]
fn former_mutable_guards_retain_exact_sources_and_fixed_results() {
    let cases = [
        (
            "export function run(a,b) { let flag=true; flag; return a; }",
            true,
            Value::Number(4.0),
        ),
        (
            "export function run(a,b) { const flag=true; let copy=flag; return a; }",
            true,
            Value::Number(4.0),
        ),
        (
            "export function run(a,b) {let value=null; return a;}",
            true,
            Value::Number(4.0),
        ),
        (
            "export function run(a,b) {let value=null; return value||a;}",
            false,
            Value::Number(4.0),
        ),
        (
            "export function run(a,b) {let x=undefined; return x;}",
            false,
            Value::Undefined,
        ),
    ];
    let mut records = Vec::new();
    for (source, numeric, expected) in cases {
        for ts in [false, true] {
            for value in [false, true] {
                if !value && !numeric {
                    continue;
                }
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
                assert_eq!(encode(&actual), encode(&expected));
                records.push(format!(
                    "{{source:{source:?},bytes:{:?},value:{value},expected:{}}}",
                    artifact.wasm,
                    encode(&expected)
                ));
            }
        }
    }
    let script = format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}],view=new DataView(new ArrayBuffer(8));
function encode(x){{if(x===undefined)return[0];if(x===null)return[1];if(typeof x==='boolean')return[2,x];assert.equal(typeof x,'number');if(Number.isNaN(x))return[3,'nan'];view.setFloat64(0,x);return[3,view.getBigUint64(0).toString(16).padStart(16,'0')];}}
function decode([tag,payload]){{if(tag===0||tag===1){{assert.ok(Object.is(payload,0));return tag===0?undefined:null;}}if(tag===2){{assert.ok(Object.is(payload,0)||payload===1);return!!payload;}}assert.equal(tag,3);return payload;}}
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();assert.deepEqual(encode(original(4,2)),c.expected);const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;const actual=run(4,2);assert.deepEqual(encode(c.value?decode(actual):actual),c.expected);}}
console.log('PASS '+cases.length+' exact mutable guard modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 16 exact mutable guard modules");
}

#[test]
fn mutable_general_cells_keep_the_source_local_budget() {
    for count in [128, 129] {
        let declarations = (0..count)
            .map(|i| format!("let v{i}=null;"))
            .collect::<String>();
        let source = format!("export function run(a,b) {{{declarations} v0=a; return v0;}}");
        let result = compile_value(&source, false);
        if count == 128 {
            let artifact = result.unwrap();
            assert!(matches!(
                ValueInstance::new(&artifact.wasm)
                    .unwrap()
                    .run(4.0, 2.0)
                    .unwrap(),
                Value::Number(4.0)
            ));
        } else {
            assert!(result.err().unwrap().contains("at most 128 source locals"));
        }
    }
}
