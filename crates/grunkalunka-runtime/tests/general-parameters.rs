use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};

#[test]
fn parameter_can_receive_null_without_changing_the_number_signature() {
    let source = "export function run(a,b) {a=null; return a;}";
    let artifact = compile_value(source, false).unwrap();
    let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
    assert!(matches!(instance.run(4.0, 2.0).unwrap(), Value::Null));
}

#[test]
fn parameter_layout_propagates_later_writes_and_captures_old_values() {
    let source = "export function run(a,b) {a=b; b=null; return a;}";
    let artifact = compile_value(source, false).unwrap();
    let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
    assert!(matches!(
        instance.run(4.0, 2.0).unwrap(),
        Value::Number(2.0)
    ));
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
fn parameter_write_matrix_preserves_captured_kind_and_payload() {
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
    for (literal, expected) in values {
        for parameter in ["a", "b"] {
            for ts in [false, true] {
                for value in [false, true] {
                    let result = if value {
                        "captured".to_owned()
                    } else {
                        format!("captured===({literal})?1:0")
                    };
                    let js = format!(
                        "export function run(a,b) {{{parameter}={literal}; const captured={parameter}; {parameter}=null; return {result};}}"
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
                    let golden = if value {
                        encode(&expected)
                    } else {
                        encode(&Value::Number(
                            if matches!(expected,Value::Number(n) if n.is_nan()) {
                                0.0
                            } else {
                                1.0
                            },
                        ))
                    };
                    assert_eq!(encode(&actual), golden, "{source}");
                    records.push(format!(
                        "{{source:{js:?},bytes:{:?},value:{value},expected:{golden}}}",
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
console.log('PASS '+cases.length+' parameter write modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 96 parameter write modules");
}

#[test]
fn parameter_snapshots_shadowing_helpers_and_scalar_interleaving_match_js() {
    let bodies = [
        ("a=null; return a;", false),
        ("a=b; b=null; return a;", false),
        ("const old=a; a=null; return old;", false),
        ("let old=a; a=false; return old;", false),
        ("a=a?false:b; return a;", false),
        ("b=a; a=b; b=null; return a;", false),
        ("a=null; return (a=b++)+b;", true),
        ("a=null; return +a+b;", true),
        ("a=null; return +a;", true),
        ("if(false)a=null; return +a;", true),
        ("if(a)a=false; else a=null; return +a;", true),
        ("for(let i=0;i<3;i++){a=i?a:false;} return a;", false),
        ("{let a=null; a=false;} return a;", true),
        ("a=null; {const a=b; return a;}", true),
        ("const before=+(a=b); b=null; return before;", true),
        (
            "a=null; b=false; const first=(a=b); b=true; return first;",
            false,
        ),
        ("a=null; b=undefined; return +(a=b);", true),
    ];
    let mut cases = bodies
        .iter()
        .map(|(body, numeric)| (format!("export function run(a,b) {{{body}}}"), *numeric))
        .collect::<Vec<_>>();
    cases.extend([
 ("function f(x,y){const old=x; x=null; return +old+y;} export function run(a,b){return f(a,b);}".to_owned(),true),
 ("function f(x,y){x=null; return x===null?y:0;} export function run(a,b){return f(a,b);}".to_owned(),true),
 ("function f(x,y,z){y=false; return +y+x+z;} export function run(a,b){return f(a,b,1);}".to_owned(),true),
 ]);
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
    for (js, numeric) in cases {
        for ts in [false, true] {
            let source = if ts {
                js.replace("run(a,b)", "run(a:number,b:number)")
                    .replace("f(x,y)", "f(x:number,y:number)")
                    .replace("f(x,y,z)", "f(x:number,y:number,z:number)")
            } else {
                js.clone()
            };
            for value in [false, true] {
                if !value && !numeric {
                    continue;
                }
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
console.log('PASS '+cases.length+' parameter snapshot modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 64 parameter snapshot modules");
}

#[test]
fn generated_parameter_cells_do_not_consume_source_local_or_parameter_budgets() {
    for count in [128, 129] {
        let locals = (0..count)
            .map(|i| format!("let v{i}=0;"))
            .collect::<String>();
        let source =
            format!("export function run(a,b) {{{locals} a=null; b=false; return +a+(+b)+v0;}}");
        let result = compile(&source, false);
        if count == 128 {
            assert_eq!(
                NumericInstance::new(&result.unwrap().wasm)
                    .unwrap()
                    .run(4.0, 2.0)
                    .unwrap(),
                0.0
            );
        } else {
            assert!(result.err().unwrap().contains("at most 128 source locals"));
        }
    }
    for count in [1000, 1001] {
        let names = (0..count).map(|i| format!("p{i:x}")).collect::<Vec<_>>();
        let args = vec!["a"; count].join(",");
        let source = format!(
            "function f({}){{p0=null; return +p0;}} export function run(a,b){{return f({args});}}",
            names.join(",")
        );
        assert!(source.len() < 16384);
        let result = compile(&source, false);
        if count == 1000 {
            let artifact = result.unwrap();
            assert_eq!(
                NumericInstance::new(&artifact.wasm)
                    .unwrap()
                    .run(4.0, 2.0)
                    .unwrap(),
                0.0
            );
        } else {
            assert!(
                result
                    .err()
                    .unwrap()
                    .contains("at most 1000 function parameters")
            );
        }
    }
    let names = (0..1000).map(|i| format!("p{i:x}")).collect::<Vec<_>>();
    let mut writes = names
        .windows(2)
        .map(|p| format!("{}={}", p[0], p[1]))
        .collect::<Vec<_>>();
    writes.push(format!("{}=null", names.last().unwrap()));
    let source = format!(
        "function f({}){{{},0;return +p0;}} export function run(a,b){{return f({});}}",
        names.join(","),
        writes.join(","),
        vec!["a"; 1000].join(",")
    );
    assert!(source.len() <= 16384, "{}", source.len());
    let artifact = compile(&source, false).unwrap();
    let _instance = NumericInstance::new(&artifact.wasm).unwrap();
    // The trailing scalar zero isolates storage planning from the separately
    // unsupported discarded sequence whose final value is General.
    // Compile/validate this long chain only: executing all 1000 General prologues
    // may exhaust the normal per-call fuel and is not a result comparison.
    node(
        &format!(
            "const assert=require('node:assert/strict');assert.ok(WebAssembly.validate(Uint8Array.from({:?})));console.log('PASS 1000 parameter chain validation');",
            artifact.wasm
        ),
        "PASS 1000 parameter chain validation",
    );
}

#[test]
fn parameter_cells_keep_argument_and_other_write_contracts() {
    for body in [
        "a=null; return run(a,b);",
        "a=null; {let a=a;} return 1;",
        "a=null; return missing;",
        "a='x'; return a;",
    ] {
        let source = format!("export function run(a,b) {{{body}}}");
        let error = compile_value(&source, false).err().unwrap();
        assert!(error.contains("unsupported"), "{source}: {error}");
        assert!(!error.contains("internal IR"));
    }
    for source in [
        "function f(x,y){return x;} export function run(a,b){return f(true,b);}",
        "export function run(a=1,b){return a;}",
        "export function run({a},b){return a;}",
    ] {
        assert!(compile_value(source, false).is_err());
    }
}
