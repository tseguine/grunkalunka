use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};

const ORIGINAL: &str = "export function run(a,b) { return void a === undefined ? 1 : 0; }";

#[test]
fn original_void_gap_returns_its_unchanged_number_golden() {
    let artifact = compile(ORIGINAL, false).unwrap();
    assert_eq!(
        NumericInstance::new(&artifact.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap(),
        1.0
    );
}

#[test]
fn void_yields_undefined_without_widening_the_numeric_return_contract() {
    let source = "export function run(a,b){return void a;}";
    let artifact = compile_value(source, false).unwrap();
    assert!(matches!(
        ValueInstance::new(&artifact.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap(),
        Value::Undefined
    ));
    assert!(compile(source, false).is_err());
}

fn encode(value: Value) -> String {
    match value {
        grunkalunka_runtime::Value::TypeError { .. }
        | grunkalunka_runtime::Value::Promise { .. } => {
            panic!("primitive-only fixture received an async object")
        }
        Value::Undefined => "[0]".into(),
        Value::Null => "[1]".into(),
        Value::Boolean(value) => format!("[2,{value}]"),
        Value::Number(value) if value.is_nan() => "[3,\"nan\"]".into(),
        Value::Number(value) => format!("[3,\"{:016x}\"]", value.to_bits()),
    }
}

#[test]
fn void_keeps_operand_effects_and_primitive_results_in_both_engines() {
    let cases = [
        ("return void a === undefined ? 1 : 0;", true),
        ("let x=a;void(x++);return x;", true),
        ("let x=a;const y=void(x=b);return +(y===undefined)+x;", true),
        ("return +(void a);", true),
        ("if(void(a=b))return a;return b;", true),
        ("let x=0;while(void x++){x+=100;}return x;", true),
        ("return +((void(a=b))??a);", true),
        ("return void(a?null:false);", false),
        ("return void({});", false),
        ("let undefined=a;return void b;", false),
        ("const f=x=>void(x+a);return f(b);", false),
        (
            "let x=0;const f=y=>void(x+=y);const g=y=>null;let h=a?f:g;return h(b);",
            false,
        ),
        ("return void(()=>a);", false),
        (
            "let x=a;const make=()=>{x=b;return ()=>x;};void make();return x;",
            true,
        ),
        ("let x=a;const f=()=>x++;void f();return x;", true),
        ("return void(null,false,a);", false),
        (
            "const f=()=>void a;const g=()=>false;let h=f;const old=h();h=g;return old;",
            false,
        ),
        (
            "let x=a;const f=()=>void(x=b);let y=a;y=f();return y;",
            false,
        ),
        ("return !(void a);", false),
        ("return (void a)||false;", false),
        ("return void void a;", false),
        ("void(void a);return b;", true),
        ("let x=a;x=void b;return +x;", true),
    ];
    let vectors = [
        (4.0, 2.0),
        (0.0, -0.0),
        (-0.0, -2.0),
        (f64::NAN, f64::INFINITY),
    ];
    let mut records = Vec::new();
    for (body, numeric) in cases {
        let js = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let source = if ts {
                js.replace("run(a,b)", "run(a:never,b:Missing):unknown")
            } else {
                js.clone()
            };
            for general in [false, true] {
                if !general && !numeric {
                    continue;
                }
                let artifact = if general {
                    compile_value(&source, ts)
                } else {
                    compile(&source, ts)
                }
                .unwrap_or_else(|error| panic!("{source}: {error}"));
                let observed = if general {
                    let mut run = ValueInstance::new(&artifact.wasm).unwrap();
                    vectors
                        .iter()
                        .map(|(a, b)| encode(run.run(*a, *b).unwrap()))
                        .collect::<Vec<_>>()
                } else {
                    let mut run = NumericInstance::new(&artifact.wasm).unwrap();
                    vectors
                        .iter()
                        .map(|(a, b)| encode(Value::Number(run.run(*a, *b).unwrap())))
                        .collect::<Vec<_>>()
                };
                records.push(format!(
                    "{{source:{js:?},bytes:{:?},general:{general},observed:[{}]}}",
                    artifact.wasm,
                    observed.join(",")
                ));
            }
        }
    }
    let script = format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}],vectors=[[4,2],[0,-0],[-0,-2],[NaN,Infinity]],view=new DataView(new ArrayBuffer(8));
function encode(x){{if(x===undefined)return[0];if(x===null)return[1];if(typeof x==='boolean')return[2,x];assert.equal(typeof x,'number');if(Number.isNaN(x))return[3,'nan'];view.setFloat64(0,x);return[3,view.getBigUint64(0).toString(16).padStart(16,'0')];}}
function decode([tag,payload]){{if(tag===0||tag===1){{assert.ok(Object.is(payload,0));return tag===0?undefined:null;}}if(tag===2){{assert.ok(Object.is(payload,0)||payload===1);return!!payload;}}assert.equal(tag,3);return payload;}}
for(const c of cases){{const native=Function('"use strict";'+c.source.replace('export function','function')+';return run;')(),run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;for(let i=0;i<vectors.length;i++){{const [a,b]=vectors[i],expected=encode(native(a,b));assert.deepEqual(c.observed[i],expected,c.source);const result=run(a,b);assert.deepEqual(encode(c.general?decode(result):result),expected,c.source);}}}}
"#,
        records.join(",")
    );
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
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
}

#[test]
fn void_does_not_hide_invalid_operands_and_keeps_depth_limits() {
    for body in [
        "return void missing;",
        "void x;let x=a;return a;",
        "const x=a;return void(x=b);",
        "return void [];",
        "return void 'text';",
        "return void(()=>[]);",
        "const f=()=>a;return void f?.();",
    ] {
        assert!(
            compile_value(&format!("export function run(a,b){{{body}}}"), false).is_err(),
            "{body}"
        );
    }
    for depth in [128, 129] {
        let source = format!(
            "export function run(a,b){{return {}a;}}",
            "void ".repeat(depth)
        );
        let result = compile_value(&source, false);
        if depth == 128 {
            let artifact = result.unwrap();
            assert!(matches!(
                ValueInstance::new(&artifact.wasm)
                    .unwrap()
                    .run(4.0, 2.0)
                    .unwrap(),
                Value::Undefined
            ));
        } else {
            assert!(result.err().unwrap().contains("nesting"));
        }
    }
}
