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
fn block_closure_returns_null() {
    let source = "export function run(a,b){const f=()=>{return null;};return f();}";
    let artifact = compile_value(source, false).unwrap();
    let result = ValueInstance::new(&artifact.wasm)
        .unwrap()
        .run(4.0, 2.0)
        .unwrap();
    assert!(matches!(result, Value::Null));
}

#[test]
fn empty_local_closure_returns_undefined() {
    let artifact = compile_value(
        "export function run(a,b){const f=()=>{};return f();}",
        false,
    )
    .unwrap();
    let result = ValueInstance::new(&artifact.wasm)
        .unwrap()
        .run(4.0, 2.0)
        .unwrap();
    assert!(matches!(result, Value::Undefined));
}

#[test]
fn numeric_branch_and_implicit_undefined_keep_distinct_result_kinds() {
    let source = "export function run(a,b){const f=()=>{if(a)return b;};return f();}";
    let artifact = compile_value(source, false).unwrap();
    let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
    assert!(matches!(
        instance.run(4.0, 2.0).unwrap(),
        Value::Number(2.0)
    ));
    assert!(matches!(instance.run(0.0, 2.0).unwrap(), Value::Undefined));
}

#[test]
fn local_undefined_completion_keeps_entry_and_function_value_boundaries() {
    for source in [
        "export function run(a,b){}",
        "export function run(a,b){return;}",
        "export function run(a,b){{return;}}",
        "export function run(a,b){if(a)return null;}",
        "function helper(){return;}export function run(a,b){return helper();}",
        "export function run(a,b){const f=()=>{if(a)return ()=>b;};return a;}",
        "export function run(a,b){const f=()=>{return;a++;};return a;}",
    ] {
        assert!(compile_value(source, false).is_err(), "{source}");
    }
}

#[test]
fn primitive_closure_results_match_javascript_and_node_wasm() {
    let bodies = [
        "const f=()=>{};return f();",
        "const f=()=>{};return f?a:b;",
        "const f=()=>{}; return a;",
        "const f=()=>{return;};return f();",
        "const f=()=>{return;};return a;",
        "const f=()=>{if(a)return null;};return a;",
        "const pick=()=>{return;};return a;",
        "const f=()=>{if(a)return b;};return f();",
        "const f=()=>{if(a)return;return b;};return f();",
        "const f=()=>{let x=b;if(a){return x;}x=a;};return f();",
        "const f=()=>{if(a){if(b)return true;}else{return null;}};return f();",
        "const f=()=>{let i=0;while(i<3){i++;if(a)return b;}};return f();",
        "const f=()=>{for(let i=0;i<3;i++){if(a)continue;if(b)break;return null;}};return f();",
        "const f=()=>{let i=0;do{i++;if(a)return false;}while(i<3);};return f();",
        "const f=()=>{const g=()=>null;};return f();",
        "const f=()=>{};const g=()=>{return false;};let h=a?f:g;return h();",
        "const f=()=>{if(a)return b;};const g=()=>undefined;let h=a?f:g;return h();",
        "let x=0;const f=()=>{x++;return;};const g=()=>{x+=10;};let h=a?f:g;const old=h();return old??x;",
        "const undefined=b;const f=()=>{return;};return f();",
        "const undefined=b;const f=()=>{};return f();",
        "const make=x=>()=>{if(x)return b;};const f=make(a);return f();",
        "const f=()=>{};let x=a;x=f();return x;",
        "const f=()=>{return undefined;};return f();",
        "const f=()=>{return null;};return f();",
        "const f=()=>{return false;};return f();",
        "const f=()=>{return true;};return f();",
        "const f=()=>{return true;}; return a;",
        "const f=()=>{if(a)return null;return false;};return f();",
        "const f=()=>{if(a){return b;}else{return true;}};return f();",
        "const f=()=>{let x=null;for(let i=0;i<3;i++){if(a)return x;x=b;}return x;};return f();",
        "const f=()=>{let i=0;while(i<3){i++;if(a)return false;}return null;};return f();",
        "const f=()=>{let i=0;do{i++;if(a)return undefined;}while(i<3);return null;};return f();",
        "const f=()=>{const g=()=>null;return g();};return f();",
        "const f=()=>{const g=()=>a;return g();};return f();",
        "const f=()=>{let x=a;x=null;return x;};return f();",
        "let x=a?null:false;const f=()=>{const old=x;x=b;return old;};return f();",
        "const make=x=>()=>{if(x)return null;return false;};const f=make(a);return f();",
        "const f=()=>{return null;};const g=()=>{return false;};let h=a?f:g;return h();",
        "const f=()=>{return null;};const g=()=>false;let h=f;if(a)h=g;return h();",
        "let x=0;const f=y=>{x+=y;return false;};const g=y=>null;let h=f;const old=h((h=g,a));return old||x;",
        "const f=x=>{x=null;return x;};const g=x=>false;let h=a?f:g;return h(b);",
        "const f=()=>{return -0;};const g=()=>{return NaN;};let h=a?f:g;return h();",
        "const f=()=>{return null;};let x=a;x=f();return x;",
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
        "const f=()=>{};return +f();",
        "const f=()=>{return;};if(f())return a;return b;",
        "const f=()=>{if(a)return b;};return +f();",
        "const f=x=>{if(x)return a;else return b;};return f(a);",
        "const f=()=>{do{return a;}while(b);};return f();",
        "const f=()=>{}; return a;",
        "const pick=()=>{return;};return a;",
        "const f=()=>{return null;};return +f();",
        "const f=()=>{return false;};if(f())return a;return b;",
        "const f=()=>{const g=()=>a;return g();};let x=f();x=f();return x;",
        "const f=()=>{let x=a;x+=b;return x;};return f();",
        "const f=x=>{x=null;return +x;};return f(a);",
        "const f=()=>{return true;}; return a;",
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
        "const f=()=>{if(a)return ()=>b;return null;};return a;",
        "const f=()=>{return '';};return a;",
        "const f=()=>{return {};};return a;",
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
