use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};
use wasmtime::Trap;

#[path = "managed/common.rs"]
mod common;

#[test]
fn original_finally_gap_sources_execute_without_rewriting() {
    for source in [
        "export function run(a,b) { try { return a; } finally { return b; } }",
        "export function run(a,b) { while(0) { try { continue; } finally { a=b; } } return b; }",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn normal_finalizers_preserve_returns_throws_and_ordered_effects() {
    for body in [
        "let n=0;try{n=1;}finally{n=n*10+2;}return n;",
        "try{return a;}finally{b;}",
        "let x=a;try{return x;}finally{x=b;}",
        "let n=0;const f=()=>{try{return(n++,a);}finally{n=n*10+2;}};const value=f();return value+n;",
        "let n=0;try{try{n=1;throw a;}finally{n=n*10+2;}}catch(e){return n+(+e);}",
        "let x=a;try{try{throw x;}finally{x=b;}}catch(e){return +e;}",
        "let n=0;try{try{throw(n++,a);}finally{n=n*10+2;}}catch(e){return n+(+e);}",
        "let n=0;const f=()=>{try{throw a;}catch(e){n=1;return +e;}finally{n=n*10+2;}};const value=f();return value+n;",
        "let n=0;try{n=1;}catch(e){n=9;}finally{n=n*10+2;}return n;",
    ] {
        compare(body);
    }
}

#[test]
fn finalizer_completions_preserve_or_override_each_pending_completion() {
    // The enclosing source catch authorizes every throw in this finite 5x5
    // matrix. Break and continue always target the bounded loop in the helper.
    let pending = [
        "n=n*10+1;",
        "return(n=n*10+1,a);",
        "throw(n=n*10+1,a);",
        "n=n*10+1;break;",
        "n=n*10+1;continue;",
    ];
    let cleanup = [
        "n=n*10+2;",
        "return(n=n*10+2,b);",
        "throw(n=n*10+2,b);",
        "n=n*10+2;break;",
        "n=n*10+2;continue;",
    ];
    for before in pending {
        for after in cleanup {
            compare(&format!(
                "let n=0;const f=()=>{{try{{for(let i=0;i<2;i++){{try{{{before}}}finally{{{after}}}}}return n;}}catch(e){{return n+(+e);}}}};const value=f();return value+n;"
            ));
        }
    }
    for body in [
        "try{return a;}finally{if(b)return b;}",
        "try{try{throw a;}finally{if(b)throw b;}}catch(e){return +e;}",
        "let n=0;for(let i=0;i<3;i++){try{n++;continue;}finally{if(i===1)break;n+=10;}}return n;",
        "let n=0;const f=()=>{for(let i=0;i<2;i++){try{return a;}finally{n++;if(i===0)continue;}}return b;};const value=f();return value+n;",
    ] {
        compare(body);
    }
}

#[test]
fn nested_finalizers_and_handlers_restore_the_right_pending_completion() {
    for body in [
        "let n=0;const f=()=>{try{try{return(n=1,a);}finally{n=n*10+2;}}finally{n=n*10+3;}};const value=f();return n+value;",
        "let n=0;try{try{try{throw a;}catch(e){n=1;throw e;}}finally{n=n*10+2;}}catch(e){return n+(+e);}",
        "let n=0;const f=()=>{try{return a;}finally{try{throw b;}catch(e){n=+e;}}};const value=f();return value+n;",
        "try{try{throw a;}finally{try{throw b;}catch(e){b=(+e)+1;}}}catch(e){return(+e)+b;}",
        "let n=0;try{try{throw a;}catch(e){n=1;throw e;}finally{n=n*10+2;}}catch(e){n=n*10+3;return n+(+e);}",
        "let n=0;try{try{return a;}catch(e){n=9;return +e;}finally{n=1;throw b;}}catch(e){return n+(+e);}",
        "try{try{try{return a;}finally{throw b;}}finally{return a;}}catch(e){return +e;}",
        "let n=0;try{try{throw a;}finally{try{n=1;}finally{n=n*10+2;}}}catch(e){return n+(+e);}",
        "let n=0;try{try{throw a;}finally{try{throw b;}catch(e){n=1;throw e;}finally{n=n*10+2;}}}catch(e){return n+(+e);}",
    ] {
        compare(body);
    }
}

#[test]
fn finalizers_keep_loop_updates_and_nested_loop_destinations() {
    for body in [
        "let n=0,updates=0;for(let i=0;i<3;i++,updates++){try{n=n*10+i;continue;}finally{n=n*10+5;}}return n+updates;",
        "let n=0;while(n<3){try{n++;break;}finally{n+=10;}}return n;",
        "let n=0,cleanups=0;do{try{n++;continue;}finally{cleanups++;}}while(n<3);return n*10+cleanups;",
        "let n=0;for(let i=0;i<3;i++){try{if(i===1)continue;if(i===2)break;n++;}finally{for(let j=0;j<2;j++){n+=10;break;}n+=100;}}return n;",
        "let n=0;for(let i=0;i<2;i++){try{break;}finally{for(let j=0;j<2;j++){n++;continue;}n+=10;}}return n;",
        "try{return a;}finally{for(let j=0;j<2;j++){if(j===0)continue;break;}}",
        "let n=0;for(let i=0;i<2;i++){try{try{n++;break;}finally{n+=10;}}finally{n+=100;}}return n;",
        "let n=0;for(const item of [a,b]){try{if(item===a)continue;n++;}finally{n+=10;}}return n;",
        "let n=0;for(let i=0;i<2;i++){for(let j=0;j<3;j++){try{if(j===1)continue;if(j===2)break;n++;}finally{n+=10;}}n+=100;}return n;",
        "let n=0;while(n<2){n++;try{break;}finally{continue;}}return n;",
    ] {
        compare(body);
    }
}

#[test]
fn finalizers_preserve_named_helper_and_closure_result_contracts() {
    for source in [
        "function keep(x){try{return x;}finally{x=x+1;}}export function run(a,b){return keep(a)+keep(b);}",
        "function leaf(x){try{return x;}finally{x++;}}function middle(x){try{return leaf(x);}finally{x++;}}export function run(a,b){return middle(a);}",
        "function recover(x){try{try{throw x;}finally{x=9;}}catch(e){return +e;}}export function run(a,b){return recover(a);}",
        "export function run(a,b){let n=0;const leaf=()=>{try{return a;}finally{n=n*10+1;}};const middle=()=>{try{return leaf();}finally{n=n*10+2;}};const value=middle();return value+n;}",
        "export function run(a,b){const make=x=>{try{return()=>x;}finally{x=b;}};const f=make(a);return f();}",
        "export function run(a,b){let make=x=>{try{return()=>x;}finally{x=b;}};const f=make(a);return f();}",
        "export function run(a,b){const make=()=>{try{return()=>a;}finally{return()=>b;}};return make()();}",
        "export function run(a,b){let make=()=>{try{return()=>a;}finally{return()=>b;}};return make()();}",
        "export function run(a,b){const f=a?(()=>{try{return a;}finally{b;}}):(()=>{try{return b;}finally{a;}});return f();}",
    ] {
        common::compare_sources(source, source);
    }
}

#[test]
fn finalizers_preserve_managed_and_mixed_private_results() {
    for body in [
        "const make=()=>{try{return{value:a};}finally{b;}};return make().value===a?1:0;",
        "let make=()=>{try{return[a,b];}finally{a=b;}};const result=make();return result.length===2&&result[1]===b?1:0;",
        "let make=()=>{try{return ' abc ';}finally{a=b;}};return make().trim().length;",
        "const make=()=>{const row={value:a};try{return row;}finally{row.value=b;}};return make().value===b?1:0;",
        "const first={value:a},second={value:b};const make=()=>{try{return first;}finally{return second;}};const result=make();return result===second&&result.value===b?1:0;",
        "let make=()=>{try{return a?{value:b}:null;}finally{a=0;}};const result=make();return result===null?1:2;",
        "const f=a?(()=>{try{return{x:a};}finally{b;}}):(()=>{try{return b;}finally{a;}});return typeof f()==='object'?1:2;",
        "let make=()=>{try{if(a)return{x:b};}finally{b;}};return typeof make()==='undefined'?1:2;",
        "let make=()=>{const row={value:a};try{return row;}finally{row.self=row;}};const result=make();return result.self===result?1:0;",
        "const apply=fn=>{try{return fn();}finally{a=b;}};const result=apply(()=>({value:a}));return result.value===b?1:2;",
    ] {
        compare(body);
    }
}

#[test]
fn finalizer_cells_are_fresh_and_pending_values_are_snapshots() {
    for body in [
        "let first=()=>0,last=()=>0;for(let i=0;i<2;i++){try{if(i===0)continue;}finally{let value=i;const read=()=>value;if(i===0)first=read;last=read;}}return first()*10+last();",
        "let first=()=>0,last=()=>0,write=()=>0;for(let i=0;i<2;i++){try{if(i===0)continue;}finally{let value=i;const read=()=>value;if(i===0){first=read;write=()=>{value=9;return 0;};}last=read;}}write();return first()*10+last();",
        "let x=a;const f=()=>{try{return x;}finally{let x=b;x++;}};return f()+x;",
        "let read=()=>0,write=()=>0;const f=()=>{try{try{throw a;}catch(e){read=()=>+e;write=()=>{e=b;return 0;};return read();}}finally{write();}};const value=f();return value+read();",
        "let read=()=>0;const f=()=>{let value=a;try{return value;}finally{read=()=>value;value=b;}};const saved=f();return saved+read();",
    ] {
        compare(body);
    }
}

#[test]
fn finalizers_keep_primitive_result_tags_and_number_payloads() {
    let mut sources = Vec::new();
    for value in ["undefined", "null", "false", "true", "a", "b", "-0", "NaN"] {
        sources.push(format!(
            "export function run(a,b){{let saved={value};try{{return saved;}}finally{{saved=b;}}}}"
        ));
        sources.push(format!(
            "export function run(a,b){{try{{try{{throw {value};}}finally{{a=b;}}}}catch(e){{return e;}}}}"
        ));
        sources.push(format!(
            "export function run(a,b){{try{{return a;}}finally{{return {value};}}}}"
        ));
    }
    for source in [
        "export function run(a,b){const f=()=>{try{return null;}finally{if(a)return false;}};return f();}",
        "export function run(a,b){let f=()=>{try{return a?undefined:b;}finally{a=0;}};return f();}",
        "export function run(a,b){const recover=x=>{try{try{throw x;}finally{a=0;}}catch(e){return e;}};return recover(a?undefined:b);}",
        "function keep(x){try{return x;}finally{x++;}}export function run(a,b){try{return keep(a);}finally{b;}}",
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
        "export function run(a,b){try{return a?null:b;}finally{a=b;}}".into(),
        "type Reason=never;export function run(a:string,b:boolean):void{try{return(a as number)?null:(b as Reason);}finally{a=b as Reason;}}".into(),
    ));
    compare_primitive_sources(&cases);
}

#[test]
fn one_cleanup_body_serves_return_throw_break_continue_and_normal_paths() {
    let source = "export function run(a,b){let n=0;const f=x=>{try{for(let i=0;i<3;i++){try{if(x===1)return a;if(x===-7)throw b;if(i===1)continue;if(i===2)break;n++;}finally{n+=104729;}}}catch(e){return +e;}return n;};const value=f(a);return value+n;}";
    common::compare_sources(source, source);
    for typed in [false, true] {
        for value_profile in [false, true] {
            let artifact = if value_profile {
                compile_value(source, typed)
            } else {
                compile(source, typed)
            }
            .unwrap();
            assert_eq!(
                artifact.wat.matches("f64.const 104729\n").count(),
                1,
                "the one source finalizer must be emitted once"
            );
        }
    }
}

#[test]
fn finalizers_keep_erasure_and_source_limits_on_a_small_native_stack() {
    let js = "export function run(a,b){let n=0;try{try{if(a)throw b;return a;}catch(e){return +e;}}finally{n++;}}";
    let ts = "type Result=never;export function run(a:string,b:boolean):void{let n:number=0;try{try{if(a as Result)throw(b as Result);return a as Result;}catch(e:unknown){return +(e as number);}}finally{n++;}}";
    common::compare_sources(js, ts);
    assert_eq!(
        compile(js, false).unwrap().wasm,
        compile(ts, true).unwrap().wasm
    );
    for count in [127, 128] {
        let locals = (0..count)
            .map(|index| format!("v{index}=a"))
            .collect::<Vec<_>>()
            .join(",");
        let source = format!(
            "export function run(a,b){{const {locals};try{{return a;}}finally{{const cleanup=b;}}}}"
        );
        for typed in [false, true] {
            let result = compile_on_small_stack(source.clone(), typed);
            if count == 127 {
                assert!(result.is_ok(), "{result:?}");
            } else {
                assert!(result.unwrap_err().contains("at most 128 source locals"));
            }
        }
    }
    // Try, body block, if, its return, the following return and cleanup block
    // contribute six source statements; cleanup effects are charged once.
    for count in [250, 251] {
        let source = format!(
            "export function run(a,b){{try{{if(a)return a;return b;}}finally{{{}}}}}",
            "b;".repeat(count)
        );
        for typed in [false, true] {
            let result = compile_on_small_stack(source.clone(), typed);
            if count == 250 {
                assert!(result.is_ok(), "{result:?}");
            } else {
                assert!(result.unwrap_err().contains("at most 256 statements"));
            }
        }
    }
    for (count, admitted) in [(1, true), (8, true), (32, true), (33, false)] {
        let mut body = "return a;".to_owned();
        for _ in 0..count {
            body = format!("try{{{body}}}finally{{b;}}");
        }
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let result = compile_on_small_stack(source.clone(), typed);
            if admitted {
                assert!(result.is_ok(), "{count}: {result:?}");
            } else {
                assert!(
                    result.unwrap_err().contains("statement nesting exceeds 64"),
                    "{count}"
                );
            }
        }
        if count == 8 {
            common::compare_sources(&source, &source);
        }
    }
}

#[test]
fn host_fuel_failures_remain_terminal_in_the_body_and_finalizer() {
    // Infinite paths are deliberately executed only by the fuel-bounded engine.
    for source in [
        "export function run(a,b){try{while(a){a=a;}}finally{return b;}}",
        "export function run(a,b){try{return b;}finally{while(a){a=a;}}}",
        "export function run(a,b){try{try{while(a){a=a;}}finally{return b;}}catch(e){return 9;}}",
    ] {
        for typed in [false, true] {
            let artifact = compile(source, typed).unwrap();
            let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
            let error = instance.run(1.0, 42.0).unwrap_err();
            assert_eq!(error.downcast_ref::<Trap>(), Some(&Trap::OutOfFuel));
            assert_eq!(instance.run(0.0, 42.0).unwrap(), 42.0);
            let artifact = compile_value(source, typed).unwrap();
            let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
            let error = instance.run(1.0, 42.0).unwrap_err();
            assert_eq!(error.downcast_ref::<Trap>(), Some(&Trap::OutOfFuel));
            assert!(matches!(
                instance.run(0.0, 42.0).unwrap(),
                Value::Number(42.0)
            ));
        }
    }
}

#[test]
fn finalizers_do_not_authorize_escaping_throws_or_unsupported_reason_values() {
    for body in [
        "try{throw a;}finally{return b;}",
        "try{return a;}finally{throw b;}",
        "try{try{throw 'text';}finally{b;}}catch(e){return 0;}",
        "try{try{throw {value:a};}finally{b;}}catch(e){return 0;}",
        "try{try{throw [a];}finally{b;}}catch(e){return 0;}",
        "const fail=()=>{try{throw a;}finally{return b;}};try{return fail();}catch(e){return +e;}",
        "const fail=()=>{try{if(a)throw b;return 0;}finally{a;}};try{return fail();}catch(e){return +e;}",
        "try{throw a;}catch({value}){return 0;}finally{b;}",
        "outer:for(let i=0;i<2;i++){try{break outer;}finally{b;}}return a;",
        "try{return a;}finally{b;}return b;",
        "try{return a;}finally{return b;a;}",
        "try{throw a;}catch(e){return +e;}finally{return +e;}",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            for value_profile in [false, true] {
                let error = if value_profile {
                    compile_value(&source, typed)
                } else {
                    compile(&source, typed)
                }
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
                assert!(error.contains("unsupported"), "{source}: {error}");
                assert!(!error.contains("internal IR"), "{source}: {error}");
            }
        }
    }
}

fn compare(body: &str) {
    let source = format!("export function run(a,b){{{body}}}");
    common::compare_sources(&source, &source);
}

fn compile_on_small_stack(source: String, typed: bool) -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || compile(&source, typed).map(|_| ()))
        .unwrap()
        .join()
        .expect("synchronous finally must not overflow the native stack")
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
                panic!("synchronous primitive finally produced an async object")
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
                "synchronous primitive finally Node timeout: {}",
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
