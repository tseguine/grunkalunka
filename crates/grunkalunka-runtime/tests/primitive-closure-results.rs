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
fn runtime_factory_returns_boolean_leaf() {
    let source = "export function run(a,b){let make=x=>()=>true;return make(a)();}";
    for typescript in [false, true] {
        let artifact = compile_value(source, typescript).unwrap();
        assert!(matches!(
            ValueInstance::new(&artifact.wasm)
                .unwrap()
                .run(4.0, 2.0)
                .unwrap(),
            Value::Boolean(true)
        ));
        assert!(compile(source, typescript).is_err());
    }
}

#[test]
fn runtime_factory_primitive_leaves_preserve_kinds_captures_and_snapshots() {
    compare_primitive_closure_programs(&[
        "let make=x=>()=>true;return make(a)();",
        "let make=x=>()=>false;return make(a)();",
        "let make=x=>()=>null;return make(a)();",
        "let make=x=>()=>undefined;return make(a)();",
        "let make=x=>()=>void x;return make(a)();",
        "const make=a?x=>()=>null:x=>()=>false;return make(b)();",
        "let make=x=>()=>x?null:false;return make(a)();",
        "let make=x=>()=>{if(x)return b;};return make(a)();",
        "let make=x=>()=>{return;};return make(a)();",
        "let make=x=>()=>{};return make(a)();",
        "let make=x=>()=>{const old=x;x=null;return old;};const f=make(a);return f()??f();",
        "let make=x=>()=>{if(x++===a)return null;return x;};const f=make(a),g=make(b);f();g();return f();",
        "let make=x=>()=>null;const alias=make;make=x=>()=>false;return alias(a)();",
        "let make=x=>()=>null;const f=make((make=x=>()=>false,a));return f();",
        "let make=x=>()=>null;make=x=>()=>false;return make(a)();",
        "let make=x=>()=>null;make&&=x=>()=>false;return make(a)();",
        "let make=x=>()=>null;make??=x=>()=>false;return make(a)();",
        "let make=x=>()=>a?b:null;const f=make(a);a=0;return f();",
        "let n=0;let make=x=>(n++,()=>x?null:false);const f=make(a),g=make(b);return (f()??g())||n;",
    ]);
}

#[test]
fn nested_runtime_factories_keep_primitive_leaf_kinds_and_captures() {
    compare_primitive_closure_programs(&[
        "let make=x=>()=>()=>true;return make(a)()();",
        "let make=x=>()=>()=>null;return make(a)()();",
        "let make=x=>y=>()=>x?null:false;return make(a)(b)();",
        "let make=x=>{return y=>{return ()=>undefined;};};return make(a)(b)();",
        "let make=x=>y=>()=>{if(x)return y;};return make(a)(b)();",
        "let make=x=>{let state=x;return y=>()=>{const old=state;state=null;return old;};};const middle=make(a);const f=middle(b);f();return middle(a)();",
        "let make=x=>y=>()=>null;const middle=make(a);make=x=>y=>()=>false;return middle(b)();",
        "let make=x=>y=>()=>null;let middle=make(a);const f=middle((middle=make(b),a));return f();",
        "let make=x=>()=>()=>true;let middle=make(a);const f=middle();middle=()=>()=>false;return f();",
        "let make=x=>{return ()=>()=>x;};return a;",
    ]);
}

#[test]
fn runtime_block_factories_preserve_primitive_leaf_kinds() {
    compare_primitive_closure_programs(&[
        "let make=x=>{return ()=>null;};return make(a)();",
        "let make=x=>{if(x)return ()=>null;return ()=>false;};return make(a)();",
        "const make=a?(x=>{return ()=>null;}):(x=>{return ()=>false;});return make(b)();",
        "let make=x=>{return ()=>{if(x)return b;};};return make(a)();",
        "let make=x=>{return ()=>{};};return make(a)();",
        "let make=x=>{let y=x;return ()=>{const old=y;y=null;return old;};};const f=make(a);f();return f();",
        "let make=x=>{const y=null;return ()=>y;};return make(a)();",
        "let make=x=>{let i=0;while(i<3){i++;if(x)return ()=>null;}return ()=>false;};return make(a)();",
        "let make=x=>{return ()=>null;};const old=make;make=x=>{return ()=>false;};return old(a)();",
        "let make=x=>{return ()=>null;};const f=make((make=x=>{return ()=>false;},a));return f();",
    ]);
}

#[test]
fn primitive_factory_leaves_keep_wide_argument_frames_and_arity_limits() {
    let body = |creators, leaves| {
        let parameters = |prefix, count| {
            (0..count)
                .map(|i| format!("{prefix}{i}"))
                .collect::<Vec<_>>()
                .join(",")
        };
        let arguments = |name: &str, count| vec![name; count].join(",");
        format!(
            "let make=({})=>({})=>p0===q0?null:false;return make({})({});",
            parameters("p", creators),
            parameters("q", leaves),
            arguments("a", creators),
            arguments("b", leaves)
        )
    };
    compare_primitive_closure_programs(&[&body(1000, 1), &body(1, 1000)]);
    let combined = body(1000, 1000);
    let source = format!("export function run(a,b){{{combined}}}");
    let artifact = compile_value(&source, false).unwrap();
    let error = ValueInstance::new(&artifact.wasm)
        .unwrap()
        .run(4.0, 2.0)
        .unwrap_err();
    assert!(format!("{error:#}").contains("fuel"));
    node(format!(
        "const assert=require('node:assert/strict');const native=Function('a','b',{combined:?});const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({:?}))).exports;assert.deepEqual(e.run(4,2),[2,Number(native(4,2))]);",
        artifact.wasm
    ));
    for (creators, leaves) in [(1001, 1000), (1000, 1001)] {
        let source = format!("export function run(a,b){{{}}}", body(creators, leaves));
        for typescript in [false, true] {
            let error = compile_value(&source, typescript)
                .err()
                .expect("arity limit");
            assert!(error.contains("1000"), "{creators}/{leaves}: {error}");
        }
    }
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
    compare_primitive_closure_programs(&bodies);
}

fn compare_primitive_closure_programs(bodies: &[&str]) {
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
