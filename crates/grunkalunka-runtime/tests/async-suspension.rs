use grunkalunka_compiler::compile_async;
use grunkalunka_runtime::{AsyncInstance, AsyncState, Value};
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

fn primitive_bits(value: Value) -> String {
    match value {
        Value::Undefined => "undefined".into(),
        Value::Null => "null".into(),
        Value::Boolean(value) => format!("boolean:{value}"),
        Value::Number(value) if value.is_nan() => "nan".into(),
        Value::Number(value) => format!("{:x}", value.to_bits()),
    }
}

#[test]
fn runtime_closures_survive_suspension_and_keep_native_job_order() {
    compare_promises(
        &[
            "const f=a?()=>a:()=>b;return f();",
            "const f=a?()=>1:()=>2;await true;return f();",
            "const f=a?()=>a:()=>b;await null;return f();",
            "const f=a?()=>a:()=>b;const alias=f;await false;a=b;return alias();",
            "let x=a;const f=a?()=>++x:()=>--x;f();await true;return f();",
            "const f=(await a)?()=>a:()=>b;await null;return f();",
            "const f=a?(await null,()=>a):(await false,()=>b);return f();",
            "const make=x=>()=>++x;const f=a?make(await a):make(await b);await false;return f()*10+f();",
            "const f=a?x=>x+a:x=>x-b;return f(await b);",
            "const f=a?(x,y)=>x*10+y+a:(x,y)=>x-y;return f(a++,await a++);",
            "const f=a?()=>a:()=>b;return f()+await(a=b)+f();",
            "const f=a?()=>a:()=>b;const g=a?()=>f()+1:()=>f()-1;await true;return g();",
            "const read=()=>a;const f=a?()=>read()+b:()=>read()-b;await true;a=b;return f();",
            "const f=()=>{const g=a?()=>a:()=>b;return g();};return f();",
            "const f=a?x=>{const g=x?()=>x+b:()=>x-b;return g();}:x=>x;await false;return f(a);",
            "const flag=a<b;const f=a?()=>flag?b:a:()=>flag?a:b;await null;return f();",
            "let x=null;const f=a?()=>+(x=true):()=>+x;await false;f();await null;return x;",
            "const f=a?()=>+a:()=>+b;await null;a=await true;return f();",
            "const f=a?()=>-0:()=>b;return await f();",
            "const f=a?x=>x%a:x=>x%b;await false;return f(b);",
            "const f=a?()=>a:()=>b;let x=0;for(let i=0;i<3;i++){x+=await f();}return x;",
            "const f=a?()=>a:()=>b;await Promise.reject(null);return f();",
            "const f=a?()=>a:()=>b;return await Promise.reject(f());",
            "const f=a?()=>a:()=>b;return a?f():true;",
        ],
        &[
            (4.0, 2.0),
            (0.0, 2.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn mutable_function_writes_preserve_snapshots_across_jobs() {
    compare_promises(
        &[
            "let f=()=>a;await null;f=()=>b;return f();",
            "let f=()=>a;const alias=f;await false;f=()=>b;await true;return f()*10+alias();",
            "let f=x=>a+x;return f((await(f=x=>b-x,3)));",
            "let f=(x,y)=>a+x+y;return f(a++,await(f=(x,y)=>b-x-y,a++));",
            "let f=()=>a;f=(await a)?()=>a+b:()=>a-b;return f();",
            "let f=()=>a;let g=()=>b;const alias=(f=g=(await a)?()=>a+b:()=>a-b);await null;f=()=>b;return alias()+g()+f();",
            "let f=()=>a;const read=a?()=>f():()=>f()+1;await false;f=()=>b;return read();",
            "let f=()=>a;const set=()=>{f=()=>b;return f();};await false;set();await true;return f();",
            "const make=x=>()=>++x;let f=make(a);const alias=f;f=a?make(await b):make(await a);await null;return f()*100+alias()*10+f();",
            "const make=x=>()=>++x;let f=make(a);f=make(await f());return f();",
            "let f=()=>a;let x=f();await null;f=()=>b;x=f();return x;",
            "let f=()=>a;let x=f();await null;x=true;f=()=>+x;return f();",
            "let f=()=>a;for(let i=0;i<3;i++){let x=await i;f=()=>x;}return f();",
            "let f=()=>a;f=(await null,()=>b);return await f();",
            "let f=()=>a;await Promise.reject(null);f=()=>b;return f();",
            "let f=()=>a;f=()=>b;return await Promise.reject(f());",
            "let f=x=>x;f=x=>{if(x<=0)return b;return x+f(x-1);};await null;return f(4);",
        ],
        &[
            (4.0, 2.0),
            (0.0, 2.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    let plain = "export async function run(a,b){let f=x=>x+a;await null;return f((f=x=>x-b,a));}";
    let typed = "export async function run(a:never,b:Missing):Wrong {let f:(x:unknown)=>string=(x=>x+a) as unknown as ((x:number)=>number);await (null as never);return (f as any)((f=((x:number)=>x-b) as any,a));}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
}

#[test]
fn async_closure_frames_keep_wide_calls_and_activation_ownership() {
    let parameters = (0..1000)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let arguments = vec!["a"; 1000].join(",");
    let body = format!(
        "const f=a?({parameters})=>p0+p999+a:({parameters})=>p0-p999+b;await null;return f({arguments});"
    );
    compare_promises(&[&body], &[(4.0, 2.0), (0.0, 2.0)]);

    let source = source_for(
        "const make=x=>()=>++x;const f=a?make(a):make(b);await null;const x=f();await true;return x*10+f();",
    );
    let artifact = compile_async(&source, false).unwrap();
    // Logical closure IDs start at one. Physical function one is start; the
    // table must point to the named lifted body instead of that numeric index.
    assert!(
        artifact
            .wat
            .contains("(elem (i32.const 1) func $closure_body1)")
    );
    let mut first = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
    let mut second = AsyncInstance::new(&artifact.wasm, 0.0, 2.0).unwrap();
    for call in [&mut first, &mut second] {
        assert!(matches!(call.state().unwrap(), AsyncState::Pending));
        assert!(call.run_one_job().unwrap());
        assert!(matches!(call.state().unwrap(), AsyncState::Pending));
    }
    assert!(second.run_one_job().unwrap());
    assert!(first.run_one_job().unwrap());
    assert!(matches!(
        first.state().unwrap(),
        AsyncState::Fulfilled(Value::Number(56.0))
    ));
    assert!(matches!(
        second.state().unwrap(),
        AsyncState::Fulfilled(Value::Number(34.0))
    ));

    for body in [
        "const f=a?()=>{for(let i=0;i<10000;i++)a+=b;return a;}:()=>b;await null;return f();",
        "const f=a?()=>a:()=>b;for(let i=0;i<33;i++)await f();return a;",
    ] {
        let wasm = compile_async(&source_for(body), false).unwrap().wasm;
        let mut call = AsyncInstance::new(&wasm, 4.0, 2.0).unwrap();
        while matches!(call.state().unwrap(), AsyncState::Pending) {
            if call.run_one_job().is_err() {
                break;
            }
        }
        assert!(call.state().is_err());
        assert!(call.run_one_job().is_err());
        let mut independent = AsyncInstance::new(&wasm, 0.0, 2.0).unwrap();
        // The first body takes its short alternate; the second still has its
        // own 32-job budget instead of inheriting the failed call's state.
        assert!(matches!(independent.state().unwrap(), AsyncState::Pending));
        assert!(independent.run_one_job().unwrap());
    }
}

#[test]
fn async_argument_frames_grow_without_resetting_captures_between_jobs() {
    use wasmtime::{Engine, Instance, Module, Store};
    let parameters = (0..1000)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let arguments = vec!["a"; 1000].join(",");
    let body = format!(
        "const f=a?({parameters})=>p0+p999+a:({parameters})=>p0-p999+b;let x=0;for(let i=0;i<32;i++){{await null;x+=f({arguments});}}return x;"
    );
    compare_promises(&[&body], &[(4.0, 2.0), (0.0, 2.0)]);
    let artifact = compile_async(&source_for(&body), false).unwrap();
    let mut wat = artifact.wat.trim_end().to_string();
    assert_eq!(wat.pop(), Some(')'));
    wat.push_str("(export \"top\" (global $cell_top))(export \"arena\" (memory 0)))");
    let wasm = wat::parse_str(wat).unwrap();
    let mut config = wasmtime::Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config).unwrap();
    let module = Module::new(&engine, &wasm).unwrap();
    let mut store = Store::new(&engine, ());
    store.set_fuel(10000).unwrap();
    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    let start = instance
        .get_typed_func::<(f64, f64), (i32, i32, f64)>(&mut store, "start")
        .unwrap();
    let resume = instance
        .get_typed_func::<(i32, f64), (i32, i32, f64)>(&mut store, "resume")
        .unwrap();
    let top = instance.get_global(&mut store, "top").unwrap();
    let memory = instance.get_memory(&mut store, "arena").unwrap();
    let mut state = start.call(&mut store, (4.0, 2.0)).unwrap();
    let initial = top.get(&mut store).i32().unwrap();
    assert!(initial > 0);
    for job in 1..=32 {
        assert_eq!(state.0, 1);
        store.set_fuel(10000).unwrap();
        state = resume.call(&mut store, (state.1, state.2)).unwrap();
        assert_eq!(top.get(&mut store).i32(), Some(initial + 8000 * job));
    }
    assert_eq!(state, (0, 3, 384.0));
    assert_eq!(memory.size(&store), 4);
    run_node(&format!(
        "import assert from 'node:assert/strict';const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({wasm:?}))).exports;let s=e.start(4,2);const initial=e.top.value;for(let j=1;j<=32;j++){{assert.equal(s[0],1);s=e.resume(s[1],s[2]);assert.equal(e.top.value,initial+8000*j);}}assert.deepEqual(s,[0,3,384]);assert.equal(e.arena.buffer.byteLength,4*65536);assert.throws(()=>e.start(4,2),WebAssembly.RuntimeError);assert.throws(()=>e.resume(3,0),WebAssembly.RuntimeError);"
    ));
}

#[test]
fn primitive_awaits_keep_kind_reasons_and_job_order() {
    let bodies = [
        "return await true;",
        "return await false;",
        "return await null;",
        "return await undefined;",
        "return true;",
        "return null;",
        "return undefined;",
        "return await Promise.resolve(true);",
        "return await Promise.reject(null);",
        "return await Promise.reject(undefined);",
        "return await Promise.reject(false);",
        "return +(await true === 1);",
        "return +(await null === 0);",
        "return +(await true == 1);",
        "return +(await null == undefined);",
        "return +(await false === 0);",
        "return (await true)?a:b;",
        "let x=await false;if(!x)x=await null;return x;",
        "let x=await undefined;x=await a;return x;",
        "let x=await a;x=await true;return x;",
        "const x=await false;return x||(await true);",
        "return (await null)&&a;",
        "return (await undefined)||(await true);",
        "return (await true)+a;",
        "return (await null)+b;",
        "return +(await undefined);",
        "return await Promise.reject(await false);",
        "return await(a?true:null);",
        "const Promise=a;return await null;",
        "const undefined=a;return await undefined;",
        "let x=null;const f=()=>{x=true;return +a;};await f();return x;",
        "let x=await null;const f=()=>{return +x;};return await f();",
        "let x=a;for(let i=0;i<3;i++){x=(await true)?x+b:x;}return x;",
        "let x=true;for(let i=0;i<32;i++)x=await x;return x;",
        "let x=a;const read=()=>+x;const set=()=>+(x=true);await null;set();await false;return read();",
        "const flag=a<b;const f=()=>flag?a:b;await true;return f();",
        "const f=x=>{const g=()=>x;return g();};const x=f(a);await null;return x+f(b);",
        "let x=await null;const f=()=>+x;x=await true;return f();",
        "const f=()=>+a;await null;a=await true;return f();",
        "const make=x=>()=>++x;const f=make(a),g=make(b);await true;return f()*100+f()*10+g();",
        "const make=x=>()=>++x;const f=make(await a);await false;const g=make(await b);return f()*100+f()*10+g();",
        "const make=x=>()=>+(x=true);const f=make(a);await null;return f();",
    ];
    compare_promises(
        &bodies,
        &[
            (4.0, 2.0),
            (-0.0, 0.0),
            (f64::NAN, f64::INFINITY),
            (f64::from_bits(1), f64::MAX),
        ],
    );
    for (body, kind, expected) in [
        ("return await true;", "fulfilled", "boolean:true"),
        ("return await null;", "fulfilled", "null"),
        ("return await undefined;", "fulfilled", "undefined"),
        (
            "return await Promise.reject(false);",
            "rejected",
            "boolean:false",
        ),
        ("return await Promise.reject(null);", "rejected", "null"),
        (
            "return await Promise.reject(undefined);",
            "rejected",
            "undefined",
        ),
    ] {
        let wasm = compile_async(&source_for(body), false).unwrap().wasm;
        let mut call = AsyncInstance::new(&wasm, 4.0, 2.0).unwrap();
        assert!(matches!(call.state().unwrap(), AsyncState::Pending));
        assert!(call.run_one_job().unwrap());
        let (actual, value) = match call.state().unwrap() {
            AsyncState::Fulfilled(value) => ("fulfilled", value),
            AsyncState::Rejected(value) => ("rejected", value),
            _ => panic!("not settled"),
        };
        assert_eq!(actual, kind);
        assert_eq!(primitive_bits(value), expected);
        assert!(!call.run_one_job().unwrap());
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
    compare_promises(
        &bodies,
        &[
            (4.0, 2.0),
            (-0.0, 0.0),
            (f64::NAN, f64::INFINITY),
            (f64::from_bits(1), f64::MAX),
        ],
    );
}

#[test]
fn suspended_control_flow_matches_native_results_and_job_order() {
    let bodies = [
        "return a ? await b : a;",
        "if(a)return b;return await a;",
        "if(await a){return await b;}else{return await a;}",
        "{let x=await a; a=await b; return x+a;}",
        "let x=a; if(b){x=await b;}else{x=await a;}return x;",
        "let x=0;for(let i=0;i<3;i++){x+=await(a+i);}return x;",
        "let x=0;for(let i=0;i<3;i=await(i+1)){if(i===1)continue;x+=a;}return x;",
        "let x=a;let i=0;while(await(i++<3?1:0)){x+=b;}return x;",
        "let x=a;let i=0;do{i++;if(i<3)continue;x+=b;}while(await(i<3?1:0));return x;",
        "let x=a;for(let i=0;i<3;i++){if(i===1)break;x+=await b;}return x;",
        "let x=a;for(let i=0;i<2;i++){for(let j=0;j<3;j++){if(j===1)continue;if(j===2)break;x+=await b;}x+=1;}return x;",
        "let x=a;for(let i=0;i<3;i++){if(i===1)return x;x+=await b;}return b;",
        "return a && await b;",
        "return a || await b;",
        "let x=a;{let x=await b;x+=1;}return x;",
        "for(let i=0;i<3;i++){let x=await i;const f=()=>x;a+=f();}return a;",
        "let x=a;for(let i=0;i<3;i++){if(i===1)return await Promise.reject(b);x+=await 1;}return x;",
        "let x=a?await b:await a;return +x;",
        "const x=a?!(await b):!(await a);if(x)return a;return b;",
        "let x=a?(await b,true):null;return +x;",
        "return a+(b?await(a=b):await(a+1));",
        "return await(a?await b:await a);",
        "if(a)return b;else return a;",
        "let x=a;for(let i=0;i<3;i++){if(i<2){await b;continue;}x+=1;}return x;",
        "let x=a;for(let i=await 0;i<2;i++){x+=await b;}return x;",
        "do{return await a;}while(await b);",
        "if((await a)?await b:await a)return a;return b;",
        "const f=()=>{if(a)return b;return a;};return (await f())+a;",
        "const f=x=>{const g=()=>{if(x)return x;return b;};return g()+1;};let x=await a;return f(x)+f(b);",
        "const f=()=>{for(let i=0;i<3;i++){if(i===1)return a;}return b;};return (await f())+b;",
    ];
    compare_promises(
        &bodies,
        &[
            (4.0, 2.0),
            (-0.0, 0.0),
            (f64::NAN, f64::INFINITY),
            (f64::from_bits(1), f64::MAX),
        ],
    );
    // The exact previously rejected loop terminates for these inputs.
    compare_promises(
        &["while(a){a=await b;}return a;"],
        &[(4.0, 0.0), (-0.0, 0.0), (f64::NAN, 0.0)],
    );
}

fn compare_promises(bodies: &[&str], vectors: &[(f64, f64)]) {
    let mut script = String::from(
        r#"
import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
const {createAsyncFunction}=await import(pathToFileURL(process.argv[2]));
const buffer=new ArrayBuffer(8), view=new DataView(buffer);
function bits(value){if(value===undefined)return 'undefined';if(value===null)return 'null';if(typeof value==='boolean')return 'boolean:'+value;assert.equal(typeof value,'number');if(Number.isNaN(value))return 'nan';view.setFloat64(0,value);return view.getBigUint64(0).toString(16);}
async function observe(run,a,b){
 const trace=[];const promise=run(a,b);assert(promise instanceof Promise);trace.push('returned');
 const done=promise.then(value=>{trace.push('fulfilled:'+bits(value));},reason=>{trace.push('rejected:'+bits(reason));});
 let count=0;const markers=new Promise(resolve=>{function tick(){trace.push('tick'+(++count));if(count<34)queueMicrotask(tick);else resolve();}queueMicrotask(tick);});
 await Promise.all([done,markers]);const outcome=trace.find(entry=>entry.startsWith('fulfilled:')||entry.startsWith('rejected:'));
 return {trace,outcome,jobs:trace.slice(0,trace.indexOf(outcome)).filter(entry=>entry.startsWith('tick')).length};
}
function fromBits(hex){view.setBigUint64(0,BigInt('0x'+hex));return view.getFloat64(0);}
assert.equal(createHash('sha256').update('export async function run(a,b) { return a + await Promise.resolve(b); }').digest('hex'),'735913a592f91c980a71bac74f112102d572af6f6dc989f6b571ef11877e786e');
"#,
    );
    let inputs: Vec<_> = vectors
        .iter()
        .map(|(a, b)| {
            format!(
                "[fromBits('{:x}'),fromBits('{:x}')]",
                a.to_bits(),
                b.to_bits()
            )
        })
        .collect();
    script.push_str(&format!("const vectors=[{}];\n", inputs.join(",")));
    for body in bodies {
        let source = source_for(body);
        for typescript in [false, true] {
            let artifact = compile_async(&source, typescript)
                .unwrap_or_else(|error| panic!("{source}: {error}"));
            let mut rust = Vec::new();
            for &(a, b) in vectors {
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
                let bits = primitive_bits(value);
                rust.push(format!("{{outcome:'{kind}:{bits}',jobs:{jobs}}}"));
            }
            script.push_str(&format!("{{const native=({});const wasm=createAsyncFunction(new Uint8Array({:?}));const rust=[{}];let i=0;for(const [a,b] of vectors){{const expected=await observe(native,a,b);assert.deepEqual(await observe(wasm,a,b),expected,{:?});assert.equal(rust[i].outcome,expected.outcome);assert.equal(rust[i].jobs,expected.jobs);i++;}}const [a,b]=vectors[0], [c,d]=vectors.at(-1);assert.deepEqual(await Promise.allSettled([wasm(a,b),wasm(c,d)]),await Promise.allSettled([native(a,b),native(c,d)]));}}\n",source.trim_start_matches("export "),artifact.wasm,rust.join(","),source));
        }
    }
    run_node(&script);
}

fn run_node(script: &str) {
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
        "export async function run(a,b){return await run(a,b);}",
        "export async function run(a,b){return await 'string';}",
        "export async function run(a,b){return await {};}",
        "export async function run(a,b){const Promise=a;return await Promise.resolve(true);}",
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
    assert!(matches!(
        call.state().unwrap(),
        AsyncState::Fulfilled(Value::Number(6.0))
    ));
    let typed = "export async function run(a:number,b:never):number{const x:number=await(null as unknown as number);return x;}";
    let artifact = compile_async(typed, true).unwrap();
    let mut call = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
    call.run_one_job().unwrap();
    assert!(matches!(
        call.state().unwrap(),
        AsyncState::Fulfilled(Value::Null)
    ));
    run_node(&format!(
        r#"
import assert from 'node:assert/strict';import {{pathToFileURL}} from 'node:url';
const {{createAsyncFunction}}=await import(pathToFileURL(process.argv[2]));
assert.equal(await createAsyncFunction(new Uint8Array({:?}))(4,2),null);
"#,
        artifact.wasm
    ));
    for (count, admitted) in [(32, true), (33, false)] {
        let source = format!(
            "export async function run(a,b){{let x=a;{}return x;}}",
            "x=await x;".repeat(count)
        );
        assert_eq!(compile_async(&source, false).is_ok(), admitted);
    }
    for (per_branch, admitted) in [(16, true), (17, false)] {
        let branch = "a=await a;".repeat(per_branch);
        let source =
            format!("export async function run(a,b){{if(b){{{branch}}}else{{{branch}}}return a;}}");
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
        (
            "let x=0;for(let i=0;i<3;i=await(i+1)){if(i===1)continue;x+=a;}return x;",
            3,
            8.0,
        ),
        (
            "let x=a;let i=0;do{i++;if(i<3)continue;x+=b;}while(await(i<3?1:0));return x;",
            3,
            6.0,
        ),
        (
            "let x=a;for(let i=0;i<3;i++){if(i===1)break;x+=await b;}return x;",
            1,
            6.0,
        ),
        (
            "let x=a;for(let i=0;i<2;i++){for(let j=0;j<3;j++){if(j===1)continue;if(j===2)break;x+=await b;}x+=1;}return x;",
            2,
            10.0,
        ),
        (
            "let x=a;for(let i=0;i<3;i++){if(i===1)return x;x+=await b;}return b;",
            1,
            6.0,
        ),
        ("return a || await b;", 0, 4.0),
        ("return a && await b;", 1, 2.0),
        (
            "let x=a;let i=0;while(await(i++<3?1:0)){x+=b;}return x;",
            4,
            10.0,
        ),
        ("let x=a?(await b,true):null;return +x;", 1, 1.0),
        (
            "const x=a?!(await b):!(await a);if(x)return a;return b;",
            1,
            2.0,
        ),
        ("do{return await a;}while(await b);", 1, 4.0),
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
            let AsyncState::Fulfilled(Value::Number(actual)) = call.state().unwrap() else {
                panic!("{source}: not fulfilled");
            };
            assert_eq!(actual.to_bits(), expected.to_bits(), "{source}");
            assert!(!call.run_one_job().unwrap());
        }
    }
}

#[test]
fn source_loop_budgets_and_continuation_graph_size_are_bounded() {
    let source = "export async function run(a,b){for(let i=0;i<b;i++)a=await(a+1);return a;}";
    for typescript in [false, true] {
        let artifact = compile_async(source, typescript).unwrap();
        for count in [32, 33] {
            let mut activation = AsyncInstance::new(&artifact.wasm, 4.0, count as f64).unwrap();
            for _ in 0..31 {
                assert!(activation.run_one_job().unwrap());
            }
            if count == 32 {
                assert!(activation.run_one_job().unwrap());
                assert!(matches!(
                    activation.state().unwrap(),
                    AsyncState::Fulfilled(Value::Number(36.0))
                ));
                assert!(!activation.run_one_job().unwrap());
            } else {
                assert!(activation.run_one_job().is_err());
                assert!(activation.state().is_err());
            }
        }
        run_node(&format!(
            r#"
import assert from 'node:assert/strict';
import {{pathToFileURL}} from 'node:url';
const {{createAsyncFunction}}=await import(pathToFileURL(process.argv[2]));
const run=createAsyncFunction(new Uint8Array({:?}));
assert.equal(await run(4,32),36);
await assert.rejects(run(4,33),RangeError);
assert.equal(await run(7,1),8);
"#,
            artifact.wasm
        ));
        let source =
            "export async function run(a,b){a=await a;for(let i=0;i<10000;i++)a+=b;return a;}";
        let wasm = compile_async(source, typescript).unwrap().wasm;
        let mut activation = AsyncInstance::new(&wasm, 4.0, 2.0).unwrap();
        assert!(activation.run_one_job().is_err());
        assert!(activation.state().is_err());
        assert!(activation.run_one_job().is_err());
    }
    let mut expression = String::from("x");
    for _ in 0..20 {
        expression = format!("x?x:({expression})");
    }
    let source = format!(
        "export async function run(a,b){{const f=x=>{expression};{}return a;}}",
        "a=f(a);".repeat(80)
    );
    for typescript in [false, true] {
        let error = compile_async(&source, typescript)
            .err()
            .expect("graph must be bounded");
        assert!(
            error.contains("continuation graph exceeds 4096 states"),
            "{error}"
        );
    }
    let body = format!(
        "const f=x=>{expression};{}return await a;",
        "a=f(a);".repeat(32)
    );
    for typescript in [false, true] {
        let artifact = compile_async(&source_for(&body), typescript).unwrap();
        assert!(artifact.wat.matches("block $state").count() > 3000);
    }
    compare_promises(&[&body], &[(4.0, 2.0)]);
}

#[test]
fn rejection_and_signed_zero_survive_the_job_boundary() {
    let source = "export async function run(a,b){return await Promise.reject(a);}";
    let wasm = compile_async(source, false).unwrap().wasm;
    let mut call = AsyncInstance::new(&wasm, -0.0, 2.0).unwrap();
    assert!(matches!(call.state().unwrap(), AsyncState::Pending));
    call.run_one_job().unwrap();
    assert!(
        matches!(call.state().unwrap(),AsyncState::Rejected(Value::Number(value)) if value.to_bits()==(-0.0f64).to_bits())
    );
    assert!(!call.run_one_job().unwrap());
}

#[test]
fn malformed_protocol_traps_and_excess_jobs_are_terminal() {
    let invalid=wat::parse_str("(module(func(export \"start\")(param f64 f64)(result i32 i32 f64)i32.const 99 i32.const 3 f64.const 0)(func(export \"resume\")(param i32 f64)(result i32 i32 f64)unreachable))").unwrap();
    assert!(AsyncInstance::new(&invalid, 1.0, 2.0).is_err());
    for body in [
        "unreachable",
        "i32.const 1 i32.const 3 f64.const 1",
        "(loop $forever br $forever) unreachable",
    ] {
        let wasm=wat::parse_str(format!("(module(func(export \"start\")(param f64 f64)(result i32 i32 f64)i32.const 1 i32.const 3 f64.const 1)(func(export \"resume\")(param i32 f64)(result i32 i32 f64){body}))")).unwrap();
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

#[test]
fn noncanonical_primitive_tags_fail_at_both_async_boundaries() {
    let mut script = String::from(
        "import assert from 'node:assert/strict';import {pathToFileURL} from 'node:url';const {createAsyncFunction}=await import(pathToFileURL(process.argv[2]));\n",
    );
    for (tag, payload) in [(0, "-0"), (1, "1"), (2, "2"), (2, "-0"), (4, "0")] {
        for status in [0, 1, 2] {
            let wasm=wat::parse_str(format!(r#"(module
(func(export "start")(param f64 f64)(result i32 i32 f64)i32.const {status} i32.const {tag} f64.const {payload})
(func(export "resume")(param i32 f64)(result i32 i32 f64)unreachable))"#)).unwrap();
            assert!(AsyncInstance::new(&wasm, 4.0, 2.0).is_err());
            script.push_str(&format!("await assert.rejects(createAsyncFunction(new Uint8Array({wasm:?}))(4,2),TypeError);\n"));
        }
        let wasm=wat::parse_str(format!(r#"(module
(func(export "start")(param f64 f64)(result i32 i32 f64)i32.const 1 i32.const 2 f64.const 1)
(func(export "resume")(param i32 f64)(result i32 i32 f64)i32.const 0 i32.const {tag} f64.const {payload}))"#)).unwrap();
        let mut call = AsyncInstance::new(&wasm, 4.0, 2.0).unwrap();
        assert!(call.run_one_job().is_err());
        assert!(call.state().is_err());
        assert!(call.run_one_job().is_err());
        script.push_str(&format!(
            "await assert.rejects(createAsyncFunction(new Uint8Array({wasm:?}))(4,2),TypeError);\n"
        ));
    }
    run_node(&script);
}
