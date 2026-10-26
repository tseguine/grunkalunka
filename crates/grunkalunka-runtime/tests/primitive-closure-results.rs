use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};

fn encode(value: Value) -> String {
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

fn node(script: String) {
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
fn concise_closure_returns_null() {
    let source = "export function run(a,b){const f=()=>null;return f();}";
    let artifact = compile_value(source, false).unwrap();
    let result = ValueInstance::new(&artifact.wasm)
        .unwrap()
        .run(4.0, 2.0)
        .unwrap();
    assert!(matches!(result, Value::Null));
}

#[test]
fn primitive_closure_results_match_javascript_and_node_wasm() {
    let bodies = [
        "const f=()=>undefined;return f();",
        "const f=()=>null;const g=f;return g();",
        "const f=()=>false;return f();",
        "const f=()=>true;return f();",
        "const f=()=>a?null:false;return f();",
        "const f=x=>x?true:null;return f(a);",
        "const f=()=>a?undefined:b;return f();",
        "const f=()=>a?-0:NaN;return f();",
        "return (()=>null)();",
        "const f=()=>null;const g=()=>false;const h=a?f:g;return h();",
        "const f=()=>null;const g=()=>false;let h=f;if(a)h=g;return h();",
        "const f=()=>null;const g=()=>false;return (a?f:g)();",
        "const f=()=>null;const g=()=>false;return (f??g)();",
        "const f=()=>null;const g=()=>false;return (f&&g)();",
        "let x=a?null:false;const f=()=>x;const g=()=>undefined;let h=a?f:g;x=b;return h();",
        "const f=()=>null;let x=f();x=b;return x;",
        "const f=()=>null;let x=a;x=f();return x;",
        "const f=()=>null;const g=()=>f();return g();",
        "const f=()=>null;return f()??b;",
        "const f=()=>false;return f()||b;",
        "const f=()=>null;f();return b;",
        "let x=0;const f=()=>((x+=1),null);return f()??x;",
        "let x=0;const f=y=>((x+=y),false);const g=y=>null;let h=f;const z=h((h=g,a));return z||x;",
        "const f=()=>{return a+b;};return f();",
        "const id=fn=>fn;const f=id(()=>true);return f();",
        "const make=x=>()=>true;return a;",
        "const pick=()=>{return ()=>true;};const f=pick();return f();",
        "const f=()=>null;let x=a;const g=()=>x;x=f();return g();",
        "let x=false;const f=()=>x;const g=()=>null;let h=a?f:g;return h();",
        "const make=()=>{const x=false;return ()=>x;};const f=make();return f();",
        "const f=x=>(x=null,x);const g=x=>false;let h=a?f:g;return h(b);",
        "const make=()=>()=>null;const f=make();let x=a;x=f();return x;",
        "const make=()=>()=>false;const f=make();let x=f();x=a;return x;",
        "let x=a;const make=()=>()=>x;const f=make();x=null;let y=f();y=b;return y;",
    ];
    let mut cases = Vec::new();
    for body in bodies {
        for ts in [false, true] {
            let js = format!("export function run(a,b){{{body}}}");
            let source = if ts {
                js.replace("run(a,b)", "run(a:number,b:number)")
            } else {
                js.clone()
            };
            let artifact = compile_value(&source, ts).unwrap_or_else(|e| panic!("{source}: {e}"));
            let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
            let expected = [
                (4.0, 2.0),
                (0.0, -0.0),
                (-0.0, -2.0),
                (f64::NAN, f64::INFINITY),
            ]
            .map(|(a, b)| encode(instance.run(a, b).unwrap()))
            .join(",");
            cases.push(format!(
                "{{source:{js:?},bytes:{:?},expected:[{expected}]}}",
                artifact.wasm
            ));
        }
    }
    node(format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}],view=new DataView(new ArrayBuffer(8));
function encode(x){{if(x===undefined)return[0];if(x===null)return[1];if(typeof x==='boolean')return[2,x];assert.equal(typeof x,'number');if(Number.isNaN(x))return[3,'nan'];view.setFloat64(0,x);return[3,view.getBigUint64(0).toString(16).padStart(16,'0')];}}
function decode([tag,payload]){{if(tag===0||tag===1){{assert.ok(Object.is(payload,0));return tag===0?undefined:null;}}if(tag===2){{assert.ok(Object.is(payload,0)||payload===1);return!!payload;}}assert.equal(tag,3);return payload;}}
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;let i=0;for(const args of [[4,2],[0,-0],[-0,-2],[NaN,Infinity]]){{assert.deepEqual(encode(original(...args)),c.expected[i],c.source+' JS');assert.deepEqual(encode(decode(run(...args))),c.expected[i++],c.source+' Wasm');}}}}
"#,
        cases.join(",")
    ));
}

#[test]
fn primitive_closure_coercions_and_predicates() {
    for body in [
        "const f=()=>null;return +f();",
        "const f=()=>undefined;return +f();",
        "const f=()=>false;if(f())return a;return b;",
        "const f=()=>true;const g=()=>null;let h=a?f:g;if(h())return a;return b;",
        "const f=()=>null;const g=()=>false;let h=a?f:g;return +h();",
        "const f=()=>null;return f()===null?1:0;",
        "return +(()=>null)();",
        "const make=x=>()=>++x;const f=make(a);let y=f();y=f();return y;",
        "const make=x=>()=>true;return a;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        let artifact = compile(&source, false).unwrap_or_else(|e| panic!("{source}: {e}"));
        let result = NumericInstance::new(&artifact.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap();
        node(format!("const assert=require('node:assert/strict');{source};assert.ok(Object.is(run(4,2),{}));", if result.is_nan() { "NaN".into() } else { result.to_string() }).replace("export function", "function"));
    }
}

#[test]
fn primitive_results_do_not_expand_other_closure_domains() {
    for body in [
        "const f=()=>{return null;};return f();",
        "const f=async()=>null;return a;",
        "const f=x=>x;return f(null);",
        "const f=()=>null;return f;",
        "const f=()=>null;const g=()=>b;let h=a?f:g;return h();",
        "const f=()=>'';return f();",
        "const f=()=>({});return f();",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(compile_value(&source, false).is_err(), "{source}");
    }
    assert!(
        compile(
            "export function run(a,b){const f=()=>null;return f();}",
            false
        )
        .is_err()
    );
}
