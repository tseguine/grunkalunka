use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};

#[test]
fn condition_uses_the_selected_general_payload() {
    let source = "export function run(a,b) {const value=a?null:b; return value?1:0;}";
    let artifact = compile(source, false).unwrap();
    let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
    assert_eq!(instance.run(1.0, 7.0).unwrap(), 0.0);
    assert_eq!(instance.run(0.0, 7.0).unwrap(), 1.0);
    assert_eq!(instance.run(0.0, f64::NAN).unwrap(), 0.0);
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
fn four_kind_truthiness_matches_standard_goldens_in_both_engines() {
    let representatives = [
        ("undefined", false),
        ("null", false),
        ("false", false),
        ("true", true),
        ("0", false),
        ("-0", false),
        ("NaN", false),
        ("1", true),
        ("-1", true),
        ("Infinity", true),
        ("-Infinity", true),
        ("5e-324", true),
    ];
    let mut records = Vec::new();
    for (expression, truthy) in representatives {
        for (form, negated) in [("value", false), ("!value", true), ("!!value", false)] {
            for typescript in [false, true] {
                for value in [false, true] {
                    let body = format!(
                        "const value=a?{expression}:undefined; return {form}?{}:{};",
                        if value { "true" } else { "1" },
                        if value { "false" } else { "0" }
                    );
                    let oracle = format!("export function run(a,b) {{{body}}}");
                    let source = if typescript {
                        oracle.replace("run(a,b)", "run(a:number,b:number)")
                    } else {
                        oracle.clone()
                    };
                    let artifact = if value {
                        compile_value(&source, typescript)
                    } else {
                        compile(&source, typescript)
                    }
                    .unwrap();
                    let golden = [negated, truthy ^ negated];
                    if value {
                        let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
                        for (a, expected) in golden.iter().enumerate() {
                            assert!(
                                matches!(instance.run(a as f64,0.0).unwrap(),Value::Boolean(actual) if actual==*expected),
                                "{source}"
                            );
                        }
                    } else {
                        let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
                        for (a, expected) in golden.iter().enumerate() {
                            assert_eq!(
                                instance.run(a as f64, 0.0).unwrap(),
                                f64::from(u8::from(*expected)),
                                "{source}"
                            );
                        }
                    }
                    records.push(format!(
                        "{{source:{oracle:?},bytes:{:?},value:{value},golden:{golden:?}}}",
                        artifact.wasm
                    ));
                }
            }
        }
    }
    let script = format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}];
for(const c of cases){{
 const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();
 const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;
 for(let a=0;a<2;a++){{
  const expected=c.value?c.golden[a]:Number(c.golden[a]);assert.equal(original(a,0),expected);
  const actual=run(a,0);if(c.value)assert.deepEqual(actual,[2,Number(c.golden[a])]);else assert.equal(actual,expected);
 }}
}}
console.log('PASS '+cases.length+' truthiness modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 144 truthiness modules");
}

#[test]
fn direct_nullish_conditions_and_effects_match_original_js() {
    let cases = [
        "if('x') return a; return b;",
        "if({}) return a; return b;",
        "const value=null; if(value) return 1; return 0;",
        "if(null) return a; return b;",
        "if(undefined) return a; return b;",
        "let n=0; if(null&&(n=1)) n+=10; return n;",
        "let n=0; if(undefined||(n=2)) n+=10; return n;",
        "let n=0; if((n+=1,a?null:b)) n+=10; return n;",
        "while(null) {a+=1;} return a;",
        "for(;undefined;) {a+=1;} return a;",
        "do {a+=b;} while(null); return a;",
        "const undefined=a?null:b; return undefined?1:0;",
        "return +!null;",
        "return !(a?null:b)?1:0;",
    ];
    let vectors = [
        (0.0, 0.0),
        (1.0, 2.0),
        (0.0, -0.0),
        (0.0, -1.0),
        (0.0, f64::NAN),
        (f64::NAN, 1.0),
        (0.0, f64::INFINITY),
    ];
    let mut records = Vec::new();
    for body in cases {
        for typescript in [false, true] {
            for value in [false, true] {
                let js = format!("export function run(a,b) {{{body}}}");
                let source = if typescript {
                    js.replace("run(a,b)", "run(a:number,b:number)")
                } else {
                    js.clone()
                };
                let artifact = if value {
                    compile_value(&source, typescript)
                } else {
                    compile(&source, typescript)
                }
                .unwrap();
                let mut observed = Vec::new();
                if value {
                    let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
                    for (a, b) in vectors {
                        let Value::Number(result) = instance.run(a, b).unwrap() else {
                            panic!("Number")
                        };
                        observed.push(result.to_bits());
                    }
                } else {
                    let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
                    for (a, b) in vectors {
                        observed.push(instance.run(a, b).unwrap().to_bits());
                    }
                }
                records.push(format!(
                    "{{source:{js:?},bytes:{:?},value:{value},observed:{:?}}}",
                    artifact.wasm,
                    observed
                        .iter()
                        .map(|bits| format!("{bits:016x}"))
                        .collect::<Vec<_>>()
                ));
            }
        }
    }
    let script = format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}],vectors=[[0,0],[1,2],[0,-0],[0,-1],[0,NaN],[NaN,1],[0,Infinity]],v=new DataView(new ArrayBuffer(8));
const bits=x=>{{if(Number.isNaN(x))return'nan';v.setFloat64(0,x);return v.getBigUint64(0).toString(16).padStart(16,'0')}};
const number=hex=>{{v.setBigUint64(0,BigInt('0x'+hex));return v.getFloat64(0)}};
for(const c of cases){{
 const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();
 const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;
 for(let i=0;i<vectors.length;i++){{
  const expected=bits(original(...vectors[i]));assert.equal(bits(number(c.observed[i])),expected,c.source+' Wasmtime');
  let actual=run(...vectors[i]);if(c.value){{assert.equal(actual[0],3);actual=actual[1];}}
  assert.equal(bits(actual),expected,c.source+' Node-Wasm');
 }}
}}
console.log('PASS '+cases.length+' effect modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 56 effect modules");
}

#[test]
fn general_entry_call_result_can_supply_a_condition() {
    let source = "export function run(a,b) {return a?(run(0,b)?true:false):null;}";
    let artifact = compile_value(source, false).unwrap();
    let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
    assert!(matches!(
        instance.run(1.0, 0.0).unwrap(),
        Value::Boolean(false)
    ));
    assert!(matches!(instance.run(0.0, 0.0).unwrap(), Value::Null));
    let script = format!(
        "const assert=require('node:assert/strict');const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from({:?}))).exports.run;assert.deepEqual(run(1,0),[2,0]);assert.deepEqual(run(0,0),[1,0]);console.log('PASS General call condition');",
        artifact.wasm
    );
    node(&script, "PASS General call condition");
}

#[test]
fn truthiness_preserves_remaining_scope_kind_and_depth_guards() {
    for ts in [false, true] {
        for body in [
            "if(1n) return a; return b;",
            "if(null&&Math.random()) return a; return b;",
            "const value=null; if(value) value=1; return a;",
            "if(value) return a; const value=null; return b;",
            "const value=value?null:a; return b;",
        ] {
            let error = compile_value(&format!("export function run(a,b) {{{body}}}"), ts)
                .err()
                .unwrap();
            assert!(error.contains("unsupported"), "{error}");
            assert!(!error.contains("internal IR"), "{error}");
        }
    }
    for depth in [127, 128] {
        let source = format!(
            "export function run(a,b) {{return {}null?1:0;}}",
            "!".repeat(depth)
        );
        let result = compile(&source, false);
        if depth == 127 {
            result.unwrap();
        } else {
            assert!(result.err().unwrap().contains("predicate nesting exceeds"));
        }
    }
}
