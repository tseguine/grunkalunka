use grunkalunka_compiler::compile_async;
use grunkalunka_runtime::{AsyncInstance, AsyncState};
use std::{
    io::Write,
    process::{Command, Stdio},
};

const ORIGINAL: &str = "export async function run(a,b) { return a + await Promise.resolve(b); }";

fn source_for(body: &str) -> String {
    if body == "return a + await Promise.resolve(b);" {
        ORIGINAL.into()
    } else {
        format!("export async function run(a,b){{{body}}}")
    }
}

#[test]
fn real_promises_match_js_results_and_fifo_job_order() {
    let maximum = format!("let x=a;{}return x;", "x=await x;".repeat(32));
    let bodies = [
        "return a + await Promise.resolve(b);",
        "return a + await Promise.resolve(a=b);",
        "let x=await a; x+=await b; return x;",
        "return await await b;",
        "const read=()=>a; let x=await read(); a=b; return x+read();",
        "return (await a) % b;",
        "return a+b;",
        "return await Promise.reject(a);",
        "let x=await a; return x+await Promise.reject(b);",
        "return (await (Promise.resolve(b)));",
        "const Promise=a; return await b;",
        "let x=await a; const f=y=>x+y; return f(b);",
        "let x=await a; for(let i=0;i<3;i++)x+=b; return x;",
        "return await (a?b:a);",
        maximum.as_str(),
    ];
    let mut script = String::from(
        r#"
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
const {createAsyncFunction}=await import(pathToFileURL(process.argv[2]));
const buffer=new ArrayBuffer(8), view=new DataView(buffer);
function bits(value){assert.equal(typeof value,'number');if(Number.isNaN(value))return 'nan';view.setFloat64(0,value);return view.getBigUint64(0).toString(16);}
async function observe(run,a,b){
 const trace=[];const promise=run(a,b);assert(promise instanceof Promise);trace.push('returned');
 const done=promise.then(value=>{trace.push('fulfilled:'+bits(value));},reason=>{trace.push('rejected:'+bits(reason));});
 let count=0;const markers=new Promise(resolve=>{function tick(){trace.push('tick'+(++count));if(count<34)queueMicrotask(tick);else resolve();}queueMicrotask(tick);});
 await Promise.all([done,markers]);const outcome=trace.find(entry=>entry.startsWith('fulfilled:')||entry.startsWith('rejected:'));
 return {trace,outcome,jobs:trace.slice(0,trace.indexOf(outcome)).filter(entry=>entry.startsWith('tick')).length};
}
const vectors=[[4,2],[-0,0],[NaN,Infinity],[Number.MIN_VALUE,Number.MAX_VALUE]];
assert.equal(createHash('sha256').update('export async function run(a,b) { return a + await Promise.resolve(b); }').digest('hex'),'735913a592f91c980a71bac74f112102d572af6f6dc989f6b571ef11877e786e');
"#,
    );
    for body in bodies {
        let source = source_for(body);
        for typescript in [false, true] {
            let artifact = compile_async(&source, typescript)
                .unwrap_or_else(|error| panic!("{source}: {error}"));
            let mut rust = Vec::new();
            for (a, b) in [
                (4.0, 2.0),
                (-0.0, 0.0),
                (f64::NAN, f64::INFINITY),
                (f64::from_bits(1), f64::MAX),
            ] {
                let mut activation = AsyncInstance::new(&artifact.wasm, a, b).unwrap();
                let mut jobs = 0;
                while matches!(activation.state().unwrap(), AsyncState::Pending) {
                    activation.run_one_job().unwrap();
                    jobs += 1;
                    assert!(jobs <= 32);
                }
                let (kind, value) = match activation.state().unwrap() {
                    AsyncState::Fulfilled(value) => ("fulfilled", value),
                    AsyncState::Rejected(value) => ("rejected", value),
                    _ => unreachable!(),
                };
                let bits = if value.is_nan() {
                    "nan".into()
                } else {
                    format!("{:x}", value.to_bits())
                };
                rust.push(format!("{{outcome:'{kind}:{bits}',jobs:{jobs}}}"));
            }
            script.push_str(&format!("{{const native=({});const wasm=createAsyncFunction(new Uint8Array({:?}));const rust=[{}];let i=0;for(const [a,b] of vectors){{const expected=await observe(native,a,b);assert.deepEqual(await observe(wasm,a,b),expected,{:?});assert.equal(rust[i].outcome,expected.outcome);assert.equal(rust[i].jobs,expected.jobs);i++;}}assert.deepEqual(await Promise.allSettled([wasm(4,2),wasm(7,3)]),await Promise.allSettled([native(4,2),native(7,3)]));}}\n",source.trim_start_matches("export "),artifact.wasm,rust.join(","),source));
        }
    }
    let adapter = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("js/async.mjs");
    let mut child = Command::new("node")
        .args(["--input-type=module", "-"])
        .arg(adapter)
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
}

#[test]
fn profile_boundaries_and_ordinary_async_rejections_remain_explicit() {
    for source in [
        "export function run(a,b){return a+b;}",
        "export async function run(Promise,b){return await Promise.resolve(b);}",
        "export async function run(a,b){return await Promise.resolve(b,a);}",
        "export async function run(a,b){return await Promise.resolve(...a);}",
        "export async function run(a,b){return await Promise.all(a);}",
        "export async function run(a,b){return a ? await b : a;}",
        "export async function run(a,b){while(a){a=await b;}return a;}",
        "export async function run(a,b){if(a)return b;return await a;}",
        "export async function run(a,b){return await run(a,b);}",
        "export async function run(a,b){return await true;}",
    ] {
        assert!(compile_async(source, false).is_err(), "{source}");
    }
    let original = ORIGINAL;
    assert!(grunkalunka_compiler::compile(original, false).is_err());
    assert!(grunkalunka_compiler::compile_value(original, false).is_err());
    let typed = "export async function run(a:Missing,b:never):string {return (a as number) + await (Promise.resolve((b as unknown as number)));}";
    let artifact = compile_async(typed, true).unwrap();
    let mut call = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
    call.run_one_job().unwrap();
    assert!(matches!(call.state().unwrap(), AsyncState::Fulfilled(6.0)));
    for (count, admitted) in [(32, true), (33, false)] {
        let source = format!(
            "export async function run(a,b){{let x=a;{}return x;}}",
            "x=await x;".repeat(count)
        );
        assert_eq!(compile_async(&source, false).is_ok(), admitted);
    }
}

#[test]
fn async_values_are_saved_until_a_separate_continuation_job() {
    for (body, jobs, expected) in [
        ("return a + await Promise.resolve(b);", 1, 6.0f64),
        ("return a + await Promise.resolve(a=b);", 1, 6.0),
        ("let x=await a; x+=await b; return x;", 2, 6.0),
        ("return await await b;", 2, 2.0),
        (
            "const read=()=>a; let x=await read(); a=b; return x+read();",
            1,
            6.0,
        ),
        ("return (await a) % b;", 1, 0.0),
        ("return a+b;", 0, 6.0),
    ] {
        for typescript in [false, true] {
            let source = format!("export async function run(a,b){{{body}}}");
            let artifact =
                compile_async(&source, typescript).unwrap_or_else(|e| panic!("{source}: {e}"));
            let mut call = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
            for job in 0..jobs {
                assert!(
                    matches!(call.state().unwrap(), AsyncState::Pending),
                    "{source}: before job {job}"
                );
                assert!(call.run_one_job().unwrap());
            }
            let AsyncState::Fulfilled(actual) = call.state().unwrap() else {
                panic!("{source}: not fulfilled");
            };
            assert_eq!(actual.to_bits(), expected.to_bits(), "{source}");
            assert!(!call.run_one_job().unwrap());
        }
    }
}

#[test]
fn rejection_and_signed_zero_survive_the_job_boundary() {
    let source = "export async function run(a,b){return await Promise.reject(a);}";
    let wasm = compile_async(source, false).unwrap().wasm;
    let mut call = AsyncInstance::new(&wasm, -0.0, 2.0).unwrap();
    assert!(matches!(call.state().unwrap(), AsyncState::Pending));
    call.run_one_job().unwrap();
    assert!(
        matches!(call.state().unwrap(),AsyncState::Rejected(value) if value.to_bits()==(-0.0f64).to_bits())
    );
    assert!(!call.run_one_job().unwrap());
}

#[test]
fn malformed_protocol_traps_and_excess_jobs_are_terminal() {
    let invalid=wat::parse_str("(module(func(export \"start\")(param f64 f64)(result i32 f64)i32.const 99 f64.const 0)(func(export \"resume\")(param f64)(result i32 f64)unreachable))").unwrap();
    assert!(AsyncInstance::new(&invalid, 1.0, 2.0).is_err());
    for body in [
        "unreachable",
        "i32.const 1 f64.const 1",
        "(loop $forever br $forever) unreachable",
    ] {
        let wasm=wat::parse_str(format!("(module(func(export \"start\")(param f64 f64)(result i32 f64)i32.const 1 f64.const 1)(func(export \"resume\")(param f64)(result i32 f64){body}))")).unwrap();
        let mut call = AsyncInstance::new(&wasm, 1.0, 2.0).unwrap();
        if body.starts_with("i32") {
            for _ in 0..31 {
                assert!(call.run_one_job().unwrap());
            }
        }
        assert!(call.run_one_job().is_err());
        assert!(call.state().is_err());
        assert!(call.run_one_job().is_err());
    }
}
