use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};

#[test]
fn const_binding_preserves_the_selected_primitive_kind() {
    let source = "export function run(a,b) {const value=a?null:b; return value;}";
    let artifact = compile_value(source, false).unwrap();
    let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
    assert!(matches!(instance.run(1.0, 7.0).unwrap(), Value::Null));
    assert!(matches!(
        instance.run(0.0, 7.0).unwrap(),
        Value::Number(7.0)
    ));
}

fn encoded(value: Value) -> String {
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

#[test]
fn general_const_programs_match_js_in_both_engines() {
    let cases = [
        ("const value=null; const alias=value; return alias;", false),
        ("const value=undefined; return value;", false),
        (
            "const value=a?true:false; const alias=value; return alias;",
            false,
        ),
        (
            "const value=a?null:b; const alias=(value as number); return alias;",
            false,
        ),
        ("const value=(b=a,a?true:null); return value;", false),
        (
            "const first=null; let n=a; const flag=a<b; const second=a?true:undefined; const alias=second; n+=b; return first===null?(alias===true?n:flag?b:a):0;",
            true,
        ),
        (
            "const value=a?null:b; {const value=b?true:undefined; value;} return value===null?a:b;",
            true,
        ),
        (
            "let result=0; for(let i=0;i<3;i++) {const value=i<2?null:undefined; const alias=value; if(alias===null) result+=a; else result+=b;} return result;",
            true,
        ),
        (
            "const undefined=a?null:b; const value=undefined; return value===null?a:b;",
            true,
        ),
        (
            "const value=a?null:b; ((value as number)); return value===null?a:b;",
            true,
        ),
        (
            "const value=a?null:b,alias=value,number=b; return alias===null?a:number;",
            true,
        ),
    ];
    let vectors = [
        (0.0, 0.0),
        (-0.0, -0.0),
        (4.0, 2.0),
        (1.0, -0.0),
        (-1.0, 1.0),
        (f64::NAN, 1.0),
        (1.0, f64::NAN),
        (f64::INFINITY, f64::NEG_INFINITY),
        (f64::from_bits(1), -f64::from_bits(1)),
    ];
    let mut records = Vec::new();
    for (body, numeric) in cases {
        let jsbody = body.replace(" as number", "");
        let js = format!("export function run(a,b) {{{jsbody}}}");
        let ts = format!("export function run(a:number,b:number) {{{body}}}");
        for (source, typescript) in [(&js, false), (&ts, true)] {
            for value in [false, true] {
                if !value && !numeric {
                    continue;
                }
                let artifact = if value {
                    compile_value(source, typescript)
                } else {
                    compile(source, typescript)
                }
                .unwrap();
                let observed = if value {
                    let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
                    vectors
                        .iter()
                        .map(|(a, b)| encoded(instance.run(*a, *b).unwrap()))
                        .collect::<Vec<_>>()
                } else {
                    let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
                    vectors
                        .iter()
                        .map(|(a, b)| encoded(Value::Number(instance.run(*a, *b).unwrap())))
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
const assert=require('node:assert/strict'),cases=[{}];
const vectors=[[0,0],[-0,-0],[4,2],[1,-0],[-1,1],[NaN,1],[1,NaN],[Infinity,-Infinity],[Number.MIN_VALUE,-Number.MIN_VALUE]];
const v=new DataView(new ArrayBuffer(8));
function encode(x){{if(x===undefined)return[0];if(x===null)return[1];if(typeof x==='boolean')return[2,x];assert.equal(typeof x,'number');if(Number.isNaN(x))return[3,'nan'];v.setFloat64(0,x);return[3,v.getBigUint64(0).toString(16).padStart(16,'0')];}}
for(const c of cases){{
 const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();
 const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;
 for(let i=0;i<vectors.length;i++){{
  const expected=encode(original(...vectors[i]));
  assert.deepEqual(c.observed[i],expected,c.source+' Wasmtime');
  let result=run(...vectors[i]);
  if(c.value){{const[tag,payload]=result;result=tag===0?undefined:tag===1?null:tag===2?!!payload:payload;}}
  assert.deepEqual(encode(result),expected,c.source+' Node-Wasm');
 }}
}}
console.log('PASS '+cases.length+' General const modules');
"#,
        records.join(",")
    );
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
    assert_eq!(
        String::from_utf8(result.stdout).unwrap().trim(),
        "PASS 34 General const modules"
    );
}

#[test]
fn general_const_cells_preserve_lexical_and_use_boundaries() {
    for ts in [false, true] {
        for body in [
            "const value=null; return +value;",
            "const value=null; return value<1?1:0;",
            "const value=null; value=1; return a;",
            "const value=null; if(0) value=1; return a;",
            "const value=null; return value++;",
            "const value=value; return a;",
            "const first=value; const value=null; return a;",
            "return value===null?a:b; const value=null;",
            "{const value=null;} return value===null?a:b;",
            "const value=null; {const copy=value; const value=null;} return a;",
        ] {
            let source = format!("export function run(a,b) {{{body}}}");
            let error = compile_value(&source, ts).err().unwrap();
            assert!(error.contains("unsupported"), "{error}");
            assert!(!error.contains("internal IR"), "{error}");
        }
        let source = "export function run(a,b) {const value=null; return value;}";
        assert!(
            compile(source, ts)
                .err()
                .unwrap()
                .contains("General bindings require a primitive value position")
        );
    }
}

#[test]
fn source_local_budget_counts_general_cells_once() {
    for count in [128, 129] {
        let declarations = (0..count)
            .map(|i| format!("const v{i}=null;"))
            .collect::<String>();
        let source = format!("export function run(a,b) {{{declarations} return v0;}}");
        let result = compile_value(&source, false);
        if count == 128 {
            let artifact = result.unwrap();
            assert!(matches!(
                ValueInstance::new(&artifact.wasm)
                    .unwrap()
                    .run(0.0, 0.0)
                    .unwrap(),
                Value::Null
            ));
        } else {
            assert!(result.err().unwrap().contains("at most 128 source locals"));
        }
    }
}

#[test]
fn original_primitive_profile_local_guard_is_now_a_null_result() {
    let source = "export function run(a,b) {const x=null; return x;}";
    for ts in [false, true] {
        let artifact = compile_value(source, ts).unwrap();
        assert!(matches!(
            ValueInstance::new(&artifact.wasm)
                .unwrap()
                .run(4.0, 2.0)
                .unwrap(),
            Value::Null
        ));
    }
}
