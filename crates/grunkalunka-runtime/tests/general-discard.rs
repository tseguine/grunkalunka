use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};

#[test]
fn discarded_null_does_not_change_the_returned_number() {
    let artifact = compile("export function run(a,b) {null; return a;}", false).unwrap();
    assert_eq!(
        NumericInstance::new(&artifact.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap(),
        4.0
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

fn check_cases(cases: Vec<String>, label: &str) {
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
    for js in cases {
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
                if label == "discarded primitive" {
                    assert_eq!(
                        observed,
                        vectors
                            .iter()
                            .map(|(a, _)| encode(&Value::Number(*a)))
                            .collect::<Vec<_>>(),
                        "{source}"
                    );
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
function encode(x){{assert.equal(typeof x,'number');if(Number.isNaN(x))return[3,'nan'];view.setFloat64(0,x);return[3,view.getBigUint64(0).toString(16).padStart(16,'0')];}}
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;for(let i=0;i<vectors.length;i++){{const expected=encode(original(...vectors[i]));assert.deepEqual(c.observed[i],expected,c.source+' Wasmtime');let actual=run(...vectors[i]);if(c.value){{assert.equal(actual[0],3);actual=actual[1];}}assert.deepEqual(encode(actual),expected,c.source+' Node-Wasm');}}}}
console.log('PASS '+cases.length+' {label} modules');
"#,
        records.join(",")
    );
    node(&script, &format!("PASS {} {label} modules", records.len()));
}

#[test]
fn discarded_primitive_forms_preserve_number_identity() {
    let literals = [
        "undefined",
        "null",
        "false",
        "true",
        "0",
        "-0",
        "NaN",
        "1",
        "-1",
        "Infinity",
        "-Infinity",
        "5e-324",
    ];
    let mut cases = Vec::new();
    for value in literals {
        for expression in [
            value.to_owned(),
            format!("(0,{value})"),
            format!("a?{value}:b"),
            format!("a&&{value}"),
            format!("a||{value}"),
            format!("(a?{value}:b,{value})"),
        ] {
            cases.push(format!(
                "export function run(a,b) {{{expression}; return a;}}"
            ));
        }
    }
    assert_eq!(cases.len(), 72);
    check_cases(cases, "discarded primitive");
}

#[test]
fn discarded_general_sequences_and_branches_evaluate_effects_once() {
    let bodies = [
        "({}); return a;",
        "let x=0; (x++,null); return x;",
        "let x=0; a?(x++,null):(x+=2,undefined); return x;",
        "let x=0; for(let i=0;i<3;(i++,null))x+=1; return x;",
        "let x=0; for((x++,null);x<3;(x++,undefined)){} return x;",
        "let i=0; for(null;i<2;undefined){i++;} return i;",
        "let x=0; false&&(x++,null); true||(x++,undefined); return x;",
        "let x=0; a&&(x++,null); a||(x+=2,undefined); return x;",
        "a=null,b=false; return +a+(+b);",
        "let x=null; (x=false,x=null); return x===null?1:0;",
        "let x=0; if(a){x++,null;}else{x+=2,undefined;} return x;",
        "let i=0; while(i<3){i++,null;} return i;",
        "let i=0; do{i++,undefined;}while(i<2); return i;",
        "let x=0; const flag=true; flag?(x++,null):(x+=2,undefined); return x;",
        "const captured=a; (a=null,b=false); return +captured;",
        "let x=0; ((x++,null),(x+=2,undefined)); return x;",
        "let x=0; (x=a,undefined,x=b,null); return x;",
        "a??null; return a;",
        "void a; return a;",
    ];
    check_cases(
        bodies
            .map(|body| format!("export function run(a,b) {{{body}}}"))
            .to_vec(),
        "discarded effect",
    );
}

#[test]
fn discarded_general_values_keep_scope_source_and_budget_guards() {
    for body in [
        "value; let value=null; return a;",
        "missing; return a;",
        "if(false){const x=1; x=2;} return a;",
        "null; return Math.random();",
        "a?null:Math.random(); return a;",
        "a??Math.random(); return a;",
        "const value=null; value++; return a;",
        "return a; null;",
    ] {
        let source = format!("export function run(a,b) {{{body}}}");
        let error = compile(&source, false).err().unwrap();
        assert!(error.contains("unsupported"), "{source}: {error}");
        assert!(!error.contains("internal IR"));
    }
    for depth in [128, 129] {
        let source = format!(
            "export function run(a,b) {{{}null{}; return a;}}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        let result = compile(&source, false);
        if depth == 128 {
            assert_eq!(
                NumericInstance::new(&result.unwrap().wasm)
                    .unwrap()
                    .run(4.0, 2.0)
                    .unwrap(),
                4.0
            );
        } else {
            assert!(result.err().unwrap().contains("expression nesting exceeds"));
        }
    }
    for count in [255, 256] {
        let source = format!(
            "export function run(a,b) {{{}return a;}}",
            "null;".repeat(count)
        );
        let result = compile(&source, false);
        if count == 255 {
            assert_eq!(
                NumericInstance::new(&result.unwrap().wasm)
                    .unwrap()
                    .run(4.0, 2.0)
                    .unwrap(),
                4.0
            );
        } else {
            assert!(result.err().unwrap().contains("at most 256 statements"));
        }
    }
    for count in [128, 129] {
        let declarations = (0..count)
            .map(|i| format!("let v{i}=null;"))
            .collect::<String>();
        let names = (0..count)
            .map(|i| format!("v{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let source = format!("export function run(a,b) {{{declarations} {names}; return a;}}");
        let result = compile(&source, false);
        if count == 128 {
            assert_eq!(
                NumericInstance::new(&result.unwrap().wasm)
                    .unwrap()
                    .run(4.0, 2.0)
                    .unwrap(),
                4.0
            );
        } else {
            assert!(result.err().unwrap().contains("at most 128 source locals"));
        }
    }
}

#[test]
fn exact_former_discard_guards_execute_their_original_sources() {
    let cases = [
        (
            "export function run(a,b) { const flag=true; flag; a; !a; (a=1,flag); a?flag:!b; flag&&true; return a; }",
            Value::Number(1.0),
            true,
        ),
        (
            "export function run(a,b) { const flag=true; a?flag:flag; return a; }",
            Value::Number(4.0),
            true,
        ),
        (
            "export function run(a,b) { const flag=true; a,flag; return a; }",
            Value::Number(4.0),
            true,
        ),
        (
            "export function run(a,b) {a=null,b=false; return a;}",
            Value::Null,
            false,
        ),
    ];
    let mut records = Vec::new();
    for (source, expected, numeric) in cases {
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
                let golden = encode(&expected);
                assert_eq!(encode(&actual), golden);
                records.push(format!(
                    "{{source:{source:?},bytes:{:?},value:{value},expected:{golden}}}",
                    artifact.wasm
                ));
            }
        }
    }
    let script = format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}],view=new DataView(new ArrayBuffer(8));
function encode(x){{if(x===undefined)return[0];if(x===null)return[1];if(typeof x==='boolean')return[2,x];assert.equal(typeof x,'number');if(Number.isNaN(x))return[3,'nan'];view.setFloat64(0,x);return[3,view.getBigUint64(0).toString(16).padStart(16,'0')];}}
function decode([tag,payload]){{if(tag===0||tag===1){{assert.ok(Object.is(payload,0));return tag===0?undefined:null;}}if(tag===2){{assert.ok(Object.is(payload,0)||payload===1);return!!payload;}}assert.equal(tag,3);return payload;}}
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();assert.deepEqual(encode(original(4,2)),c.expected);const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;let actual=run(4,2);assert.deepEqual(encode(c.value?decode(actual):actual),c.expected);}}
console.log('PASS '+cases.length+' exact discarded guard modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 14 exact discarded guard modules");
}

#[test]
fn original_general_parameter_chain_needs_no_trailing_scalar() {
    let names = (0..1000).map(|i| format!("p{i:x}")).collect::<Vec<_>>();
    let mut writes = names
        .windows(2)
        .map(|p| format!("{}={}", p[0], p[1]))
        .collect::<Vec<_>>();
    writes.push(format!("{}=null", names.last().unwrap()));
    let source = format!(
        "function f({}){{{};return +p0;}} export function run(a,b){{return f({});}}",
        names.join(","),
        writes.join(","),
        vec!["a"; 1000].join(",")
    );
    assert!(source.len() <= 16384);
    assert_eq!(&source[14193..14197], "null");
    let artifact = compile(&source, false).unwrap();
    let _instance = NumericInstance::new(&artifact.wasm).unwrap();
    // Validate the exact formerly failing source in both engines. Its many prologue
    // operations are not executed or counted as a fuel-bounded result comparison.
    node(
        &format!(
            "const assert=require('node:assert/strict');assert.ok(WebAssembly.validate(Uint8Array.from({:?})));console.log('PASS original discarded-General parameter chain');",
            artifact.wasm
        ),
        "PASS original discarded-General parameter chain",
    );
}
