use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{Value, ValueInstance};

#[path = "managed/common.rs"]
mod common;

#[test]
fn synchronous_catches_preserve_primitive_kinds_and_number_bits() {
    for reason in ["undefined", "null", "false", "true", "a", "b", "-0", "NaN"] {
        compare(&format!(
            "try{{throw {reason};}}catch(reason){{return reason===undefined?1:reason===null?2:reason===false?3:reason===true?4:5;}}"
        ));
        compare(&format!(
            "try{{throw {reason};}}catch(reason){{return +reason;}}"
        ));
    }
    for body in [
        "try{throw a;}catch(reason){return 1/(+reason);}",
        "try{throw(a?undefined:null);}catch(reason){return reason===undefined?1:reason===null?2:9;}",
        "try{throw(a?false:b);}catch(reason){return reason?11:12;}",
        "try{throw(a?true:b);}catch(reason){return(+reason)+b;}",
    ] {
        compare(body);
    }
}

#[test]
fn synchronous_catches_return_unchanged_reasons_through_the_primitive_profile() {
    let mut sources = Vec::new();
    for reason in ["undefined", "null", "false", "true", "a", "b", "-0", "NaN"] {
        sources.push(format!(
            "export function run(a,b){{try{{throw {reason};}}catch(reason){{return reason;}}}}"
        ));
    }
    for source in [
        "export function run(a,b){try{try{throw(a?null:b);}catch(inner){throw inner;}}catch(outer){return outer;}}",
        "export function run(a,b){const recover=x=>{try{throw x;}catch(e){return e;}};return recover(a?null:b);}",
        "export function run(a,b){let recover=x=>{try{throw x;}catch(e){return e;}};return recover(a?undefined:false);}",
        "export function run(a,b){const recover=a?(x=>{try{throw x;}catch(e){return e;}}):(x=>{try{throw x;}catch(e){return e;}});return recover(b?true:null);}",
        "export function run(a,b){const recover=()=>{try{throw(a?null:b);}catch(e){const read=()=>e;e=false;return read();}};return recover();}",
        // Named helpers retain their Number result contract in the value profile.
        "function recover(x){try{throw x;}catch(e){return +e;}}export function run(a,b){try{throw recover(a);}catch(e){return e;}}",
    ] {
        sources.push(source.to_owned());
    }
    let mut cases = sources
        .into_iter()
        .map(|js| {
            let ts = js.replace("run(a,b)", "run(a:number,b:number)");
            (js, ts)
        })
        .collect::<Vec<_>>();
    cases.push((
        "export function run(a,b){try{throw(a?null:b);}catch(reason){return reason;}}".into(),
        "type Reason=never;export function run(a:string,b:boolean):void{try{throw((a as number)?null:(b as Reason));}catch(reason:unknown){return reason as Reason;}}".into(),
    ));
    compare_primitive_sources(&cases);
}

#[test]
fn synchronous_throw_evaluates_its_operand_once_and_preserves_effect_order() {
    for body in [
        "let n=0;try{n=1;if(b)throw(n=n*10+2,a?null:b);n=99;}catch(e){n=n*10+3;return n+(e===null?1000:2000);}return n;",
        "let calls=0;const choose=()=>{calls++;return a;};try{throw choose();}catch(e){return calls*10+(e===a?1:e!==e&&a!==a?2:3);}",
        "let n=0;try{throw(n=1,n=n*10+2,b=n*10+3,a);}catch{n=n*10+4;}return n*1000+b;",
        "let n=0;try{n=1;}catch(e){n=99;}return n;",
        "let n=0;try{if(a)throw b;n=7;}catch(e){n=9;}return n;",
        "try{if(a)return b;throw a;}catch(e){return +e;}",
        "const row={value:a};let found=0;try{throw b;}catch(e){found=row.value===a?1:2;}return found;",
    ] {
        compare(body);
    }
}

#[test]
fn synchronous_nested_catches_route_rethrows_to_the_outer_handler() {
    for body in [
        "let n=0;try{try{throw a;}catch(first){n=1;throw(+first+b);}}catch(second){return n*10+(second===a+b?1:second!==second?2:3);}",
        "try{try{throw(a?null:b);}catch(inner){throw inner;}}catch(outer){return outer===null?11:+outer;}",
        "let n=0;try{try{throw a;}catch(e){try{throw b;}catch(inner){n=inner===b?1:2;}throw e;}}catch(e){return n*10+(e===a?1:e!==e?2:3);}",
        "try{try{throw a;}catch(e){e=b;throw e;}}catch(e){return +e;}",
        "let n=0;try{try{throw a;}catch(e){n=1;}}catch(e){n=9;}return n;",
        "try{for(let i=0;i<3;i++){try{if(i===1)throw a;}catch(e){throw e;}}}catch(e){return +e;}return b;",
    ] {
        compare(body);
    }
}

#[test]
fn synchronous_catch_bindings_preserve_shadowing_and_mutable_captures() {
    for body in [
        "const reason=7;let n=0;try{throw a;}catch(reason){n=+reason;reason=b;}return reason*10+n;",
        "let saved=()=>0;try{throw a;}catch(e){saved=()=>+e;e=b;}return saved();",
        "let read=()=>0;try{throw null;}catch(e){read=()=>e===null?1:e===false?2:3;e=false;}return read();",
        "const e=9;let n=0;try{throw a;}catch(e){{const e=b;n+=e===b?1:2;}n+=e===a?10:20;}return n+e;",
        "let saved=()=>0;try{try{throw a;}catch(e){saved=()=>+e;throw b;}}catch(e){e=7;}return saved();",
    ] {
        compare(body);
    }
}

#[test]
fn synchronous_catches_allocate_fresh_cells_for_each_dynamic_handler_entry() {
    for body in [
        "let first=()=>0,last=()=>0;for(let i=0;i<2;i++){try{throw i;}catch(e){const read=()=>+e;if(i===0)first=read;last=read;}}return first()*10+last();",
        "let first=()=>0,last=()=>0,change=()=>0;for(let i=0;i<2;i++){try{throw i;}catch(e){const read=()=>+e;if(i===0){first=read;change=()=>{e=9;return 0;};continue;}last=read;}}change();return first()*10+last();",
        "let first=()=>0,last=()=>0;let i=0;while(i<2){try{throw i++;}catch(e){const read=()=>+e;if(e===0)first=read;last=read;}}return first()*10+last();",
    ] {
        compare(body);
    }
}

#[test]
fn synchronous_catches_preserve_loop_control_and_returns() {
    for body in [
        "let n=0;for(let i=0;i<4;i++){try{if(i===1)continue;if(i===3)break;throw i;}catch(e){n=n*10+(+e)+1;}}return n;",
        "let n=0;for(let i=0;i<4;i++){try{throw i;}catch(e){if(e===1)continue;if(e===3)break;n=n*10+(+e)+1;}}return n;",
        "let i=0,n=0;while(i<4){try{throw i++;}catch(e){if(e===1)continue;n++;if(e===2)break;}}return n*10+i;",
        "let i=0,n=0;do{try{throw i++;}catch(e){if(e===0)continue;n++;}}while(i<3);return n;",
        "let n=0;for(const item of [a,b]){try{throw(item===a?1:2);}catch(e){n=n*10+(+e);}}return n;",
        "let n=0;for(let i=0;i<2;i++){for(let j=0;j<3;j++){try{throw j;}catch(e){if(e===1)continue;if(e===2)break;n++;}}n+=10;}return n;",
        "for(let i=0;i<2;i++){try{if(i===0)throw a;return b;}catch(e){if(a)return +e;}}return 3;",
    ] {
        compare(body);
    }
}

#[test]
fn synchronous_helpers_and_closures_handle_their_own_throws() {
    for source in [
        // Original canonical gap-catch source, preserved verbatim.
        "export function run(a,b) { try { throw a; } catch (e) { return e+b; } }",
        "function recover(x){try{throw x;}catch(e){return +e;}}export function run(a,b){return recover(a)+recover(b);}",
        "function recover(x){try{if(x)return x;throw false;}catch(e){return +e;}}export function run(a,b){try{return recover(a);}catch(e){return 99;}}",
        "export function run(a,b){const recover=x=>{try{throw x;}catch(e){return e===null?7:+e;}};return recover(a?null:b);}",
        "export function run(a,b){let calls=0;const recover=x=>{try{calls++;throw x;}catch(e){return(+e)+calls;}};return recover(a)+recover(b);}",
        "export function run(a,b){try{const recover=()=>{try{throw a;}catch(e){return +e;}};return recover();}catch(e){return 9;}}",
        "export function run(a,b){const recover=()=>{try{throw a;}catch(e){return +e;}};const relay=()=>recover();return relay();}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn synchronous_catches_erase_types_without_adding_reason_proofs() {
    let js = "export function run(a,b){try{throw(a?null:b);}catch(reason){const saved=reason;reason=a;return(saved===null?10:+saved)+(+reason);}}";
    let ts = "type Reason=never;export function run(a:string,b:boolean):void{try{throw((a as number)?null:(b as Reason));}catch(reason:unknown){const saved:Reason=reason as Reason;reason=a as Reason;return((saved as number)===null?10:+(saved as number))+(+(reason as number));}}";
    common::compare_sources(js, ts);
    assert_eq!(
        compile(js, false).unwrap().wasm,
        compile(ts, true).unwrap().wasm,
    );
    for source in [
        "export function run(a,b){try{throw('text' as number);}catch(e){return 0;}}",
        "export function run(a,b){try{throw({value:a} as never);}catch(e){return 0;}}",
        "export function run(a,b){try{throw a;}catch(e:unknown){return(e as string).trim().length;}}",
    ] {
        assert_rejected(source, true);
    }
}

#[test]
fn synchronous_catches_keep_uncaught_cross_function_and_reason_boundaries() {
    for body in [
        "throw a;",
        "try{throw a;}catch(e){throw e;}",
        "try{throw a;}catch(e){if(a)throw b;return 0;}",
        "try{throw 'text';}catch(e){return 0;}",
        "try{throw {value:a};}catch(e){return 0;}",
        "try{throw [a];}catch(e){return 0;}",
        "try{throw ()=>a;}catch(e){return 0;}",
        "const fail=()=>{if(a)throw b;return 0;};try{return fail();}catch(e){return +e;}",
        "try{const fail=()=>{if(a)throw b;return 0;};return fail();}catch(e){return +e;}",
        "try{const fail=()=>{try{throw a;}catch(e){if(a)throw e;return 0;}};return fail();}catch(e){return +e;}",
        "let n=0;try{n=1;throw(n=n*10+2,a?null:b);n=99;}catch(e){n=n*10+3;return n+(e===null?1000:2000);}",
        "try{throw null;}catch({value}){return 0;}",
        "try{throw a;}catch([value]){return 0;}",
        "outer:try{throw a;}catch(e){break outer;}return b;",
        "try{throw a;}catch(e){}return +e;",
        "for(const item of [a]){try{throw item;}catch(e){return +e;}}return b;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            assert_rejected(&source, typed);
        }
    }
    for source in [
        "function fail(x){if(x)throw x;return 0;}export function run(a,b){try{return fail(a);}catch(e){return +e;}}",
        "function fail(x){try{throw x;}catch(e){if(x)throw e;return 0;}}export function run(a,b){try{return fail(a);}catch(e){return +e;}}",
    ] {
        for typed in [false, true] {
            assert_rejected(source, typed);
        }
    }
}

#[test]
fn synchronous_catches_compose_with_finalizers() {
    for body in [
        "try{return a;}finally{}",
        "try{throw a;}catch(e){return +e;}finally{b;}",
        "try{try{throw a;}finally{b;}}catch(e){return +e;}",
    ] {
        compare(body);
    }
}

#[test]
fn synchronous_catches_keep_source_local_and_nesting_limits_on_a_small_stack() {
    for count in [127, 128] {
        let locals = (0..count)
            .map(|index| format!("v{index}=a"))
            .collect::<Vec<_>>()
            .join(",");
        let source = format!(
            "export function run(a,b){{const {locals};try{{throw a;}}catch(reason){{return +reason;}}}}"
        );
        for typed in [false, true] {
            let result = compile_on_small_stack(source.clone(), typed);
            if count == 127 {
                assert!(result.is_ok(), "{result:?}");
            } else {
                let error = result.unwrap_err();
                assert!(error.contains("at most 128 source locals"), "{error}");
            }
        }
        if count == 128 {
            let source = format!(
                "export function run(a,b){{const {locals};try{{throw a;}}catch{{return b;}}}}"
            );
            for typed in [false, true] {
                assert!(compile_on_small_stack(source.clone(), typed).is_ok());
            }
        }
    }
    // Each source try and its block consume two statement nesting levels.
    for (count, admitted) in [(1, true), (8, true), (32, true), (33, false)] {
        let mut body = String::from("throw a;");
        for level in 0..count {
            let action = if level + 1 == count {
                format!("return +e{level};")
            } else {
                format!("throw e{level};")
            };
            body = format!("try{{{body}}}catch(e{level}){{{action}}}");
        }
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let result = compile_on_small_stack(source.clone(), typed);
            if admitted {
                assert!(result.is_ok(), "{count}: {result:?}");
            } else {
                let error = result.unwrap_err();
                assert!(error.contains("statement nesting exceeds 64"), "{error}");
            }
        }
        if count == 8 {
            common::compare_sources(&source, &source);
        }
    }
}

fn compile_on_small_stack(source: String, typed: bool) -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || compile(&source, typed).map(|_| ()))
        .unwrap()
        .join()
        .expect("synchronous catch must not overflow the native stack")
}

fn assert_rejected(source: &str, typed: bool) {
    let error = compile(source, typed)
        .err()
        .unwrap_or_else(|| panic!("admitted {source}"));
    assert!(error.contains("unsupported"), "{source}: {error}");
    assert!(!error.contains("internal IR"), "{source}: {error}");
}

fn compare(body: &str) {
    let source = format!("export function run(a,b){{{body}}}");
    common::compare_sources(&source, &source);
}

fn compare_primitive_sources(cases: &[(String, String)]) {
    use std::{
        io::Write,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };

    fn encode(value: Value) -> String {
        match value {
            Value::Undefined => "[0]".into(),
            Value::Null => "[1]".into(),
            Value::Boolean(value) => format!("[2,{value}]"),
            Value::Number(value) => format!(
                "[3,\"{}\"]",
                if value.is_nan() {
                    "nan".into()
                } else {
                    format!("{:016x}", value.to_bits())
                }
            ),
            Value::TypeError { .. } | Value::Promise { .. } => {
                panic!("synchronous primitive catch produced an async object")
            }
        }
    }

    let vectors = [
        (4.0, 2.0),
        (0.0, -0.0),
        (-0.0, 0.0),
        (1.0, -0.0),
        (-1.0, 1.0),
        (f64::NAN, 1.0),
        (1.0, f64::NAN),
        (f64::INFINITY, f64::NEG_INFINITY),
        (f64::from_bits(1), -f64::from_bits(1)),
    ];
    let mut records = Vec::new();
    for (js, ts) in cases {
        let mut original_bytes = None;
        for (source, typed) in [(js, false), (ts, true)] {
            let artifact =
                compile_value(source, typed).unwrap_or_else(|error| panic!("{source}: {error}"));
            if let Some(bytes) = &original_bytes {
                assert_eq!(&artifact.wasm, bytes, "erased TypeScript: {source}");
            } else {
                original_bytes = Some(artifact.wasm.clone());
            }
            let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
            let observed = vectors
                .iter()
                .map(|(a, b)| {
                    encode(
                        instance
                            .run(*a, *b)
                            .unwrap_or_else(|error| panic!("{source}: {a},{b}: {error:#}")),
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            records.push(format!(
                "{{source:{js:?},bytes:{:?},observed:[{observed}]}}",
                artifact.wasm
            ));
        }
    }
    let script = format!(
        r#"const assert=require('node:assert/strict'),cases=[{}],view=new DataView(new ArrayBuffer(8));
const vectors=[[4,2],[0,-0],[-0,0],[1,-0],[-1,1],[NaN,1],[1,NaN],[Infinity,-Infinity],[Number.MIN_VALUE,-Number.MIN_VALUE]];
function encode(x){{if(x===undefined)return[0];if(x===null)return[1];if(typeof x==='boolean')return[2,x];assert.equal(typeof x,'number');if(Number.isNaN(x))return[3,'nan'];view.setFloat64(0,x);return[3,view.getBigUint64(0).toString(16).padStart(16,'0')];}}
function decode([tag,payload]){{if(tag===0||tag===1){{assert.ok(Object.is(payload,0));return tag===0?undefined:null;}}if(tag===2){{assert.ok(Object.is(payload,0)||payload===1);return!!payload;}}assert.equal(tag,3);return payload;}}
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;for(let i=0;i<vectors.length;i++){{const expected=encode(original(...vectors[i]));assert.deepEqual(c.observed[i],expected,c.source+' Wasmtime');assert.deepEqual(encode(decode(run(...vectors[i]))),expected,c.source+' Node-Wasm');}}}}
"#,
        records.join(",")
    );
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
    let started = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() > Duration::from_secs(30) {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!(
                "synchronous primitive catch Node timeout: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
