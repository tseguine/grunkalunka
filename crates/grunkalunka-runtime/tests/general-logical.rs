use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};
#[test]
fn logical_and_returns_the_selected_kind_and_payload() {
    let source = "export function run(a,b) {const left=a?null:true; return left&&b;}";
    let artifact = compile_value(source, false).unwrap();
    let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
    assert!(matches!(instance.run(1.0, 7.0).unwrap(), Value::Null));
    assert!(
        matches!(instance.run(0.0,-0.0).unwrap(),Value::Number(n) if n.to_bits()==(-0.0_f64).to_bits())
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
fn truthy(value: &Value) -> bool {
    match value {
        grunkalunka_runtime::Value::TypeError { .. }
        | grunkalunka_runtime::Value::Promise { .. } => {
            panic!("primitive-only fixture received an async object")
        }
        Value::Undefined | Value::Null => false,
        Value::Boolean(b) => *b,
        Value::Number(n) => *n != 0.0 && !n.is_nan(),
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
fn logical_matrix_preserves_selected_kind_and_payload() {
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
    for (left, a) in &values {
        for (right, b) in &values {
            for op in ["||", "&&", "??"] {
                let select_right = match op {
                    "&&" => truthy(a),
                    "||" => !truthy(a),
                    _ => matches!(a, Value::Undefined | Value::Null),
                };
                let selected = if select_right {
                    (*right, b)
                } else {
                    (*left, a)
                };
                for ts in [false, true] {
                    for value in [false, true] {
                        let body = if value {
                            format!("return ({left}){op}({right});")
                        } else {
                            format!("return (({left}){op}({right}))===({})?1:0;", selected.0)
                        };
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
                        let expected = if value {
                            encode(selected.1)
                        } else {
                            encode(&Value::Number(
                                if matches!(selected.1,Value::Number(n) if n.is_nan()) {
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
                                    .run(0.0, 0.0)
                                    .unwrap(),
                            )
                        } else {
                            encode(&Value::Number(
                                NumericInstance::new(&artifact.wasm)
                                    .unwrap()
                                    .run(0.0, 0.0)
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
    }
    let script = format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}],view=new DataView(new ArrayBuffer(8));
function encode(x){{if(x===undefined)return[0];if(x===null)return[1];if(typeof x==='boolean')return[2,x];assert.equal(typeof x,'number');if(Number.isNaN(x))return[3,'nan'];view.setFloat64(0,x);return[3,view.getBigUint64(0).toString(16).padStart(16,'0')];}}
function decode([tag,payload]){{if(tag===0||tag===1){{assert.ok(Object.is(payload,0));return tag===0?undefined:null;}}if(tag===2){{assert.ok(Object.is(payload,0)||payload===1);return!!payload;}}assert.equal(tag,3);return payload;}}
for(const c of cases){{
 const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();
 assert.deepEqual(encode(original(0,0)),c.expected,c.source+' standard golden');
 const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;
 const actual=run(0,0);assert.deepEqual(encode(c.value?decode(actual):actual),c.expected,c.source+' Node-Wasm');
}}
console.log('PASS '+cases.length+' logical modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 1728 logical modules");
}

#[test]
fn general_logical_effects_and_discard_paths_match_js() {
    let cases = [
        ("return a??b;", true),
        ("if(a??b)return a;return b;", true),
        ("if(false??true)return a;return b;", true),
        ("if((a?null:false)??true)return a;return b;", true),
        ("a??(b+=1);return b;", true),
        ("null??(b+=1);return b;", true),
        ("const left=a?null:b;return left??(b=a);", false),
        (
            "let x=a?null:false;const old=(x??(x=b));x=null;return old;",
            false,
        ),
        ("return ((a?null:b)??false)||undefined;", false),
        ("return (a===4?null:a)??b;", false),
        ("return null??b;", false),
        (
            "const left=a?false:b; const chosen=left&&((b=a),null); return (chosen===null?10:0)+b;",
            true,
        ),
        (
            "const chosen=(b=a,null)||(b+=1,true); return chosen===true?b:-99;",
            true,
        ),
        ("null&&(b=a); return b;", true),
        ("undefined||(b=a); return b;", true),
        ("const flag=true; flag&&(a=b); return a;", true),
        ("const flag=true; return (flag&&true)===false?1:0;", true),
        ("const flag=true&&a; return a;", true),
        (
            "const value=(a?true:null)||(b?null:5); return value;",
            false,
        ),
        ("const value=a?null:b; return value&&(b=a);", false),
        ("return null||b;", false),
        ("return null||a;", false),
        (
            "const left=a?undefined:false; return (left||b)&&(a?null:true);",
            false,
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
                let js = format!("export function run(a,b) {{ {body} }}");
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
                    let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
                    vectors
                        .iter()
                        .map(|(a, b)| encode(&instance.run(*a, *b).unwrap()))
                        .collect::<Vec<_>>()
                } else {
                    let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
                    vectors
                        .iter()
                        .map(|(a, b)| encode(&Value::Number(instance.run(*a, *b).unwrap())))
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
for(const c of cases){{
 const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();
 const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;
 for(let i=0;i<vectors.length;i++){{const expected=encode(original(...vectors[i]));assert.deepEqual(c.observed[i],expected,c.source+' Wasmtime');const actual=run(...vectors[i]);assert.deepEqual(encode(c.value?decode(actual):actual),expected,c.source+' Node-Wasm');}}
}}
console.log('PASS '+cases.length+' logical effect modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 72 logical effect modules");
}

#[test]
fn general_logical_entry_calls_and_depth_guards_remain_bounded() {
    let source = "export function run(a,b) {return a?(run(0,b)||true):null;}";
    let artifact = compile_value(source, false).unwrap();
    let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
    assert!(matches!(
        instance.run(1.0, 0.0).unwrap(),
        Value::Boolean(true)
    ));
    assert!(matches!(instance.run(0.0, 0.0).unwrap(), Value::Null));
    for operator in ["false||", "undefined??"] {
        for count in [128, 129] {
            let source = format!(
                "export function run(a,b) {{return {}null;}}",
                operator.repeat(count)
            );
            let result = compile_value(&source, false);
            if count == 128 {
                let a = result.unwrap();
                assert!(matches!(
                    ValueInstance::new(&a.wasm).unwrap().run(0.0, 0.0).unwrap(),
                    Value::Null
                ));
            } else {
                assert!(result.err().unwrap().contains("expression nesting exceeds"));
            }
        }
    }
    for body in [
        "return null??Math.random();",
        "return false??'x';",
        "return undefined??({});",
        "const value=null;return true??(value=1);",
        "return false&&Math.random();",
        "const value=null; return true||(value=1);",
        "return value||a; const value=null;",
        "return null||'x';",
        "return true&&({});",
    ] {
        let error = compile_value(&format!("export function run(a,b) {{{body}}}"), false)
            .err()
            .unwrap();
        assert!(error.contains("unsupported"), "{error}");
        assert!(!error.contains("internal IR"), "{error}");
    }
}

#[test]
fn former_value_logical_guards_keep_exact_sources_and_fixed_results() {
    for (source, expected) in [
        ("export function run(a,b) {return null||a;}", 4.0),
        ("export function run(a,b) {return null||b;}", 2.0),
    ] {
        for ts in [false, true] {
            let artifact = compile_value(source, ts).unwrap();
            let Value::Number(actual) = ValueInstance::new(&artifact.wasm)
                .unwrap()
                .run(4.0, 2.0)
                .unwrap()
            else {
                panic!("Number kind")
            };
            assert_eq!(actual, expected);
            let script = format!(
                "const assert=require('node:assert/strict');const original=Function('\"use strict\";'+{:?}.replace('export function','function')+';return run;')();assert.equal(original(4,2),{expected});const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from({:?}))).exports.run;assert.deepEqual(run(4,2),[3,{expected}]);console.log('PASS original logical guard');",
                source, artifact.wasm
            );
            node(&script, "PASS original logical guard");
        }
    }
}
