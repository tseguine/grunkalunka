use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};

#[test]
fn nullable_conditional_compares_kind_and_payload() {
    let source = "export function run(a,b) {return (a?null:1)===null?1:0;}";
    let artifact = compile(source, false).unwrap();
    let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
    assert_eq!(instance.run(0.0, 7.0).unwrap(), 0.0);
    assert_eq!(instance.run(1.0, 7.0).unwrap(), 1.0);
}

fn equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Undefined, Value::Undefined) | (Value::Null, Value::Null) => true,
        (Value::Boolean(a), Value::Boolean(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a == b,
        _ => false,
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
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), expected);
}

#[test]
fn primitive_kind_matrix_matches_js_in_both_profiles_and_engines() {
    let operands = [
        ("undefined", Value::Undefined),
        ("null", Value::Null),
        ("false", Value::Boolean(false)),
        ("true", Value::Boolean(true)),
        ("0", Value::Number(0.0)),
        ("-0", Value::Number(-0.0)),
        ("1", Value::Number(1.0)),
        ("-1", Value::Number(-1.0)),
        ("NaN", Value::Number(f64::NAN)),
        ("Infinity", Value::Number(f64::INFINITY)),
        ("-Infinity", Value::Number(f64::NEG_INFINITY)),
        ("5e-324", Value::Number(f64::from_bits(1))),
    ];
    let mut cases = Vec::new();
    for (left, a) in &operands {
        for (right, b) in &operands {
            for negated in [false, true] {
                let operator = if negated { "!==" } else { "===" };
                let expression = format!("({left}){operator}({right})");
                let expected = equal(a, b) ^ negated;
                let oracle = format!("export function run(a,b) {{return {expression};}}");
                for value_profile in [false, true] {
                    let source = if value_profile {
                        oracle.clone()
                    } else {
                        format!("export function run(a,b) {{return {expression}?1:0;}}")
                    };
                    let artifact = if value_profile {
                        compile_value(&source, false)
                    } else {
                        compile(&source, false)
                    }
                    .unwrap();
                    if value_profile {
                        let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
                        let Value::Boolean(actual) = instance.run(4.0, 2.0).unwrap() else {
                            panic!("Boolean return kind");
                        };
                        assert_eq!(actual, expected, "{source}");
                    } else {
                        let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
                        assert_eq!(
                            instance.run(4.0, 2.0).unwrap(),
                            f64::from(u8::from(expected)),
                            "{source}"
                        );
                    }
                    cases.push(format!("{{bytes:{:?},source:{oracle:?},expected:{expected},value:{value_profile}}}",artifact.wasm));
                }
            }
        }
    }
    let script = format!(
        r#"
'use strict';
const assert=require('node:assert/strict');
const cases=[{}];
for(const c of cases) {{
 const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();
 assert.equal(original(4,2),c.expected,c.source+' reference');
 const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;
 const actual=run(4,2);
 if(c.value) {{assert.equal(actual[0],2);assert.equal(actual[1],Number(c.expected));}}
 else assert.equal(actual,Number(c.expected));
}}
console.log('PASS '+cases.length+' primitive matrix modules');
"#,
        cases.join(",")
    );
    node(&script, "PASS 576 primitive matrix modules");
}

#[test]
fn general_snapshots_and_effects_preserve_comparison_semantics() {
    for ts in [false, true] {
        let source = "export function run(a,b) {return (a?b:null)===(a?b:undefined)?1:0;}";
        let artifact = compile(source, ts).unwrap();
        let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
        for b in [
            0.0,
            -0.0,
            1.0,
            -1.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::from_bits(1),
        ] {
            assert_eq!(instance.run(0.0, b).unwrap(), 0.0);
            assert_eq!(
                instance.run(1.0, b).unwrap(),
                f64::from(u8::from(!b.is_nan()))
            );
        }
        for body in [
            "return ((b=a,null)===(b=b+1,null))?b:999;",
            "return ((b=a,true)!==(b=b+1,false))?b:999;",
        ] {
            let source = format!("export function run(a,b) {{{body}}}");
            let artifact = compile(&source, ts).unwrap();
            let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
            assert_eq!(instance.run(4.0, 0.0).unwrap(), 5.0);
        }
        // Exact former Rust-only guards now have actual runtime outcomes.
        for (body, expected) in [
            (" return true===a?1:0; ", 0.0),
            (" return a!==false?1:0; ", 1.0),
            (" return +(true===a); ", 0.0),
            (" return true===a?1:2; ", 2.0),
        ] {
            let source = format!("export function run(a,b) {{{body}}}");
            let artifact = compile(&source, ts).unwrap();
            let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
            for a in [0.0, -0.0, 1.0, f64::NAN] {
                assert_eq!(instance.run(a, 2.0).unwrap(), expected);
            }
        }
    }
    let artifact =
        compile_value("export function run(a,b) {return null===undefined;}", false).unwrap();
    assert!(matches!(
        ValueInstance::new(&artifact.wasm)
            .unwrap()
            .run(0.0, 0.0)
            .unwrap(),
        Value::Boolean(false)
    ));
}

#[test]
fn original_curated_references_keep_their_fixed_golden() {
    let cases = [
        (
            include_str!(
                "../../../tests/compatibility/fixtures/primitive-strict-equality/gap-null.js"
            ),
            include_str!(
                "../../../tests/compatibility/fixtures/primitive-strict-equality/gap-null.ts"
            ),
        ),
        (
            include_str!(
                "../../../tests/compatibility/fixtures/primitive-strict-equality/gap-undefined.js"
            ),
            include_str!(
                "../../../tests/compatibility/fixtures/primitive-strict-equality/gap-undefined.ts"
            ),
        ),
    ];
    let mut observations = Vec::new();
    for (js, ts) in cases {
        for (source, typescript) in [(js, false), (ts, true)] {
            for value in [false, true] {
                let artifact = if value {
                    compile_value(source, typescript)
                } else {
                    compile(source, typescript)
                }
                .unwrap();
                if value {
                    let Value::Number(actual) = ValueInstance::new(&artifact.wasm)
                        .unwrap()
                        .run(4.0, 2.0)
                        .unwrap()
                    else {
                        panic!("Number kind");
                    };
                    assert_eq!(actual, 4.0);
                } else {
                    assert_eq!(
                        NumericInstance::new(&artifact.wasm)
                            .unwrap()
                            .run(4.0, 2.0)
                            .unwrap(),
                        4.0
                    );
                }
                observations.push(format!(
                    "{{source:{js:?},bytes:{:?},value:{value}}}",
                    artifact.wasm
                ));
            }
        }
    }
    let script = format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}];
for(const c of cases){{
 const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();
 assert.equal(original(4,2),4);
 const counterfactual=Function('"use strict";'+c.source.replace('===','!==').replace('export function','function')+';return run;')();
 assert.equal(counterfactual(4,2),2,'fixed golden distinguishes inequality');
 const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;
 const actual=run(4,2);
 if(c.value) assert.deepEqual(actual,[3,4]);else assert.equal(actual,4);
}}
console.log('PASS 8 original curated modules and fixed golden');
"#,
        observations.join(",")
    );
    node(&script, "PASS 8 original curated modules and fixed golden");
}
