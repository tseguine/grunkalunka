use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};

#[test]
fn bare_let_starts_with_undefined() {
    let artifact =
        compile_value("export function run(a,b) {let value; return value;}", false).unwrap();
    assert!(matches!(
        ValueInstance::new(&artifact.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap(),
        Value::Undefined
    ));
}

fn number_bits(value: f64) -> String {
    if value.is_nan() {
        "nan".into()
    } else {
        format!("{:016x}", value.to_bits())
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

fn encode(value: &Value) -> String {
    match value {
        grunkalunka_runtime::Value::TypeError { .. }
        | grunkalunka_runtime::Value::Promise { .. } => {
            panic!("primitive-only fixture received an async object")
        }
        Value::Undefined => "[0]".into(),
        Value::Null => "[1]".into(),
        Value::Boolean(b) => format!("[2,{b}]"),
        Value::Number(n) => format!("[3,{:?}]", number_bits(*n)),
    }
}
#[test]
fn implicit_undefined_respects_source_order_shadowing_and_kind_changes() {
    let cases = [
        ("let value; return value;", Value::Undefined, false),
        (
            "let undefined=42; let value; return value;",
            Value::Undefined,
            false,
        ),
        (
            "let value; return value===undefined?1:0;",
            Value::Number(1.0),
            true,
        ),
        (
            "let left,right=left; return right;",
            Value::Undefined,
            false,
        ),
        (
            "let left,right=left; left=null; return right;",
            Value::Undefined,
            false,
        ),
        ("let left=1,right; return right;", Value::Undefined, false),
        (
            "let value; value=a; return +value;",
            Value::Number(4.0),
            true,
        ),
        (
            "let value; value=false; return value;",
            Value::Boolean(false),
            false,
        ),
        ("let value; value=null; return value;", Value::Null, false),
        (
            "let value; value=NaN; return +value;",
            Value::Number(f64::NAN),
            true,
        ),
        (
            "let value; value=-0; return +value;",
            Value::Number(-0.0),
            true,
        ),
        (
            "let value; const old=value++; return old;",
            Value::Number(f64::NAN),
            true,
        ),
        (
            "let value; value++; return value;",
            Value::Number(f64::NAN),
            false,
        ),
        (
            "let value; const saved=value; value=a; return saved;",
            Value::Undefined,
            false,
        ),
        (
            "let value; if(value) return a; return b;",
            Value::Number(2.0),
            true,
        ),
        ("let value; return value||a;", Value::Number(4.0), false),
        ("let value; return value&&a;", Value::Undefined, false),
        (
            "let value; return value==null?1:0;",
            Value::Number(1.0),
            true,
        ),
        ("let x; {let x; x=a;} return x;", Value::Undefined, false),
        (
            "let undefined=42; let value; return value===undefined?1:0;",
            Value::Number(0.0),
            true,
        ),
    ];
    let mut records = Vec::new();
    for (body, expected, numeric) in cases {
        let js = format!("export function run(a,b) {{{body}}}");
        for ts in [false, true] {
            let source = if ts {
                js.replace("run(a,b)", "run(a:number,b:number)")
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
                assert_eq!(encode(&actual), encode(&expected), "{source}");
                records.push(format!(
                    "{{source:{js:?},bytes:{:?},value:{value},expected:{}}}",
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
function decode([tag,payload]){{if(tag<2){{assert.ok(Object.is(payload,0));return tag===0?undefined:null;}}if(tag===2){{assert.ok(Object.is(payload,0)||payload===1);return!!payload;}}assert.equal(tag,3);return payload;}}
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();assert.deepEqual(encode(original(4,2)),c.expected,c.source+' standardgolden');const module=new WebAssembly.Module(Uint8Array.from(c.bytes));assert.deepEqual(WebAssembly.Module.imports(module),[]);const result=new WebAssembly.Instance(module).exports.run(4,2);assert.deepEqual(encode(c.value?decode(result):result),c.expected,c.source+' Node-Wasm');}}console.log('PASS '+cases.length+' implicit let golden modules');
"#,
        records.join(",")
    );
    node(
        &script,
        &format!("PASS {} implicit let golden modules", records.len()),
    );
}

#[test]
fn implicit_let_preserves_order_scope_and_per_iteration_initialization() {
    let bodies = [
        "let value; value=a; return +value;",
        "let left,right=left; left=a; return right===undefined?+left:b;",
        "let count=0; let x,y=(count++,x),z=(count++,a); return count*10+(y===undefined?+z:0);",
        "let sum=0; for(let i=0;i<3;i++){let value; sum+=value===undefined?1:0; value=a;} return sum;",
        "let value; const saved=value; value=null; return saved===undefined?a:b;",
        "let value; value=a; {let value; value=b;} return +value;",
        "let count=0; for(let value; value===undefined;){count++;value=a;} return count;",
        "let value; return +(value=a?null:false)+b;",
        "let value; return value++;",
        "let value; return +(a?(value=b):value);",
        "let value; value; return a;",
        "let value; return value==null?a:b;",
        "let value; return value===undefined?a:b;",
        "let undefined=a; let value; return value===null?1:0;",
        "let value; let other=value; value=b; return other===undefined?+value:a;",
        "let sum=0,i=0; do {let value; if(value===undefined)sum+=a;value=b;i++;}while(i<2); return sum;",
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
                let mut observed = Vec::new();
                for (a, b) in vectors {
                    let n = if value {
                        match ValueInstance::new(&artifact.wasm)
                            .unwrap()
                            .run(a, b)
                            .unwrap()
                        {
                            Value::Number(n) => n,
                            other => panic!("expected Number: {other:?}"),
                        }
                    } else {
                        NumericInstance::new(&artifact.wasm)
                            .unwrap()
                            .run(a, b)
                            .unwrap()
                    };
                    observed.push(format!("{:?}", number_bits(n)));
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
function bits(n){{assert.equal(typeof n,'number');if(Number.isNaN(n))return'nan';view.setFloat64(0,n);return view.getBigUint64(0).toString(16).padStart(16,'0');}}
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;for(let i=0;i<vectors.length;i++){{const expected=bits(original(...vectors[i]));assert.equal(c.observed[i],expected,c.source+' Wasmtime');let actual=run(...vectors[i]);if(c.value){{assert.equal(actual[0],3);actual=actual[1];}}assert.equal(bits(actual),expected,c.source+' Node-Wasm');}}}}console.log('PASS '+cases.length+' implicit let effect modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 64 implicit let effect modules");
}

#[test]
fn implicit_let_keeps_tdz_const_and_source_contracts() {
    for source in [
        "export function run(a,b) {value; let value; return a;}",
        "export function run(a,b) {let value=value; return a;}",
        "export function run(a,b) {let x=y,y; return a;}",
        "export function run(a,b) {let x; {let x=x;} return a;}",
        "export function run(a,b) {let x; {x; let x;} return a;}",
        "export function run(a,b) {const x; return a;}",
        "export function run(a,b) {var x; return a;}",
        "export function run(a,b) {let {x}; return a;}",
        "export function run(a,b) {let x!:number; return a;}",
        "export function run(a,b) {let x; x=\"bad\"; return a;}",
    ] {
        let error = compile_value(source, true).err().unwrap();
        assert!(!error.contains("internal IR"), "{source}: {error}");
    }
    assert!(
        compile("export function run(a,b) {let x; x=a; return x;}", false)
            .err()
            .unwrap()
            .contains("General bindings require a primitive value position")
    );
}

#[test]
fn bare_let_retains_local_and_statement_budgets() {
    for count in [128, 129] {
        let names = (0..count)
            .map(|i| format!("x{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let source = format!("export function run(a,b) {{let {names}; return a;}}");
        let result = compile(&source, false);
        if count == 128 {
            result.unwrap();
        } else {
            assert!(result.err().unwrap().contains("at most 128 source locals"));
        }
    }
    for count in [254, 255] {
        let source = format!(
            "export function run(a,b) {{let value; {} return a;}}",
            "value;".repeat(count)
        );
        let result = compile(&source, false);
        if count == 254 {
            result.unwrap();
        } else {
            assert!(result.err().unwrap().contains("at most 256 statements"));
        }
    }
}
