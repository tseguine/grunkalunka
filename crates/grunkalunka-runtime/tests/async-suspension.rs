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
fn primitive_throw_rejects_without_an_extra_reaction_job() {
    compare_promises(
        &[
            "throw a;",
            "throw undefined;",
            "throw null;",
            "throw true;",
            "throw false;",
            "throw -0;",
            "throw a+b;",
            "throw (a?null:true);",
            "try{throw a;}catch(e){return e+b;}",
            "try{throw a;}catch(e){return e;}",
            "try{throw undefined;}catch(e){return e;}",
            "try{throw null;}catch(e){return e;}",
            "try{throw true;}catch(e){return e;}",
            "try{throw false;}catch(e){return e;}",
            "try{throw a;}catch{return b;}",
            "try{throw (a=b,a++);}catch(e){return e*10+a;}",
            "let x=0;try{x=1;throw (x+=a);}catch(e){return e+x;}",
            "try{throw a?null:true;}catch(e){if(e)return a;return b;}",
            "if(a)throw b;return a;",
            "if(a)return b;throw a;",
            "try{if(a)throw b;return a;}catch(e){return e;}",
            "await a;throw b;",
            "throw await a;",
            "throw await Promise.reject(a);",
            "try{throw await Promise.resolve(a);}catch(e){return e;}",
            "try{throw await Promise.reject(a);}catch(e){return e;}",
            "try{throw a;}catch(e){throw e+b;}",
            "try{try{throw a;}catch(e){throw e+b;}}catch(e){return e;}",
            "try{await null;try{throw a;}catch(e){throw await b;}}catch(e){return e;}",
            "try{throw a;}catch(e){return await e;}",
            "try{await a;throw b;}catch(e){return await e;}",
            "try{throw null;}catch(e){e=a;return await e;}",
            "try{await Promise.reject(a);return b;}catch(e){throw e;}",
            "try{try{await Promise.reject(a);return b;}catch(e){throw e+b;}}catch(e){return await e;}",
            "let x=0;for(let i=0;i<3;i++){try{throw i;}catch(e){x+=e;}}return x;",
            "let x=0;for(let i=0;i<3;i++){try{throw i;}catch(e){if(e===1)continue;x+=e;}}return x;",
            "let x=0;for(let i=0;i<3;i++){try{throw i;}catch(e){if(e===1)break;x+=e;}}return x;",
            "try{for(let i=0;i<3;i++){if(i===1)throw b;a+=1;}}catch(e){return a+e;}return a;",
            "try{while(a){throw b;}return a;}catch(e){return e;}",
            "do{throw a;}while(await b);",
            "let f=()=>a;try{throw b;}catch(e){f=()=>+e;}await null;return f();",
            "let f=()=>a;let old=()=>b;for(let i=0;i<2;i++){try{throw i;}catch(e){old=f;f=()=>+e;}}return old()*10+f();",
            "for(let i=0;i<32;i++)await null;throw a;",
            "const pick=flag=>{if(flag)return (fn,x)=>fn(x);return (fn,x)=>fn(fn(x));};const h=pick(a);try{throw h(x=>x+b,a);}catch(reason){await null;return h(x=>x+ +reason,b);}",
            "try{throw a;}catch(reason){const make=()=>{return (fn,x)=>fn(x)+ +reason;};const h=make();await null;throw h(x=>x+b,b);}",
        ],
        &[
            (4.0, 2.0),
            (0.0, -2.0),
            (-0.0, 0.0),
            (f64::NAN, f64::INFINITY),
            (f64::INFINITY, f64::NEG_INFINITY),
            (f64::from_bits(1), f64::MAX),
        ],
    );
}

#[test]
fn throw_keeps_profile_protocol_and_host_failure_boundaries() {
    for body in [
        "throw 'text';",
        "throw {};",
        "throw (()=>a);",
        "const f=()=>{throw a;};return f();",
        "try{throw a;}finally{return b;}",
        "throw a;return b;",
    ] {
        assert!(compile_async(&source_for(body), false).is_err(), "{body}");
    }
    for value_profile in [false, true] {
        let source = "export function run(a,b){throw a;}";
        let error = if value_profile {
            grunkalunka_compiler::compile_value(source, false)
        } else {
            grunkalunka_compiler::compile(source, false)
        }
        .err()
        .unwrap();
        assert!(
            error.contains("statement is outside the numeric experiment"),
            "{error}"
        );
    }
    let plain = "export async function run(a,b){try{throw a;}catch(e){throw await e;}}";
    let typed = "export async function run(a:never,b:Missing):Wrong{try{throw(a as unknown);}catch(e:unknown){throw await(e as number);}}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    let wasm = compile_async(&source_for("throw a;"), false).unwrap().wasm;
    let mut direct = AsyncInstance::new(&wasm, -0.0, 2.0).unwrap();
    assert!(
        matches!(direct.state().unwrap(),AsyncState::Rejected(Value::Number(value)) if value.to_bits()==(-0.0f64).to_bits())
    );
    assert!(!direct.run_one_job().unwrap());
    let wasm = compile_async(
        &source_for("try{const f=()=>{for(;;)a+=1;return a;};throw f();}catch(e){return e;}"),
        false,
    )
    .unwrap()
    .wasm;
    assert!(AsyncInstance::new(&wasm, 4.0, 2.0).is_err());
    let wasm = compile_async(
        &source_for("try{for(let i=0;i<33;i++)await null;throw a;}catch(e){throw b;}"),
        false,
    )
    .unwrap()
    .wasm;
    let mut call = AsyncInstance::new(&wasm, 4.0, 2.0).unwrap();
    for _ in 0..31 {
        assert!(call.run_one_job().unwrap());
    }
    assert!(call.run_one_job().is_err());
    assert!(call.state().is_err());
    assert!(call.run_one_job().is_err());
    run_node(&format!(
        r#"import assert from 'node:assert/strict';import {{pathToFileURL}} from 'node:url';const {{createAsyncFunction}}=await import(pathToFileURL(process.argv[2]));await assert.rejects(createAsyncFunction(new Uint8Array({:?}))(4,2),/32-job budget/);"#,
        wasm
    ));
}

#[test]
fn rejected_await_enters_lexical_catch_on_the_same_reaction_job() {
    compare_promises(
        &[
            "try{return await Promise.reject(a);}catch(e){return e+b;}",
            "try{return await Promise.reject(null);}catch(e){return e;}",
            "try{return await Promise.reject(undefined);}catch(e){return e;}",
            "try{return await Promise.reject(true);}catch(e){return e;}",
            "try{return await Promise.reject(false);}catch(e){return e;}",
            "try{return await Promise.reject(a);}catch(e){return e;}",
            "try{return await a;}catch(e){return b;}",
            "try{return a;}catch(e){return b;}",
            "try{await Promise.reject(a);a=99;}catch(e){return e;}return b;",
            "let x=0;try{x=await Promise.reject(a);x=99;}catch(e){x=e;}return x;",
            "let x=0;try{x=await a;}catch(e){x=e;}return await x;",
            "try{await Promise.reject(a);}catch(e){e+=b;return await e;}return b;",
            "try{await Promise.reject(null);}catch(e){e=true;await false;return e;}return b;",
            "let e=a;try{await Promise.reject(b);}catch(e){e=a;await null;}return e;",
            "try{await Promise.reject(a);}catch(a){a=b;await null;}return a;",
            "try{await Promise.reject(a);}catch(undefined){return await undefined;}return b;",
            "try{await Promise.reject(null);}catch{return await b;}return b;",
            "try{if(a)return await Promise.reject(b);return await a;}catch(e){return e;}",
            "try{return a?await Promise.reject(b):await a;}catch(e){return e;}",
            "try{return (await Promise.reject(a))+(await b);}catch(e){return e;}",
            "try{await a;return await Promise.reject(b);}catch(e){return e;}",
            "try{await Promise.reject(a);}catch(e){return (await e)+(await b);}return b;",
            "try{await Promise.reject(a);}catch(e){return await Promise.reject(e);}return b;",
            "try{try{await Promise.reject(a);}catch(e){return await Promise.reject(e+b);}}catch(e){return e;}return b;",
            "try{try{return await Promise.reject(a);}catch(e){return await e;}}catch(e){return b;}",
            "try{try{return await a;}catch(e){return await Promise.reject(b);}}catch(e){return e;}",
            "let x=0;try{try{await Promise.reject(a);}catch(e){x=e;await Promise.reject(b);}}catch(e){return x+e;}return b;",
            "let x=0;for(let i=0;i<3;i++){try{await Promise.reject(i);}catch(e){x+=e;}}return x;",
            "let x=0;for(let i=0;i<3;i++){try{await Promise.reject(i);}catch(e){if(e===1)continue;x+=e;}}return x;",
            "let x=0;for(let i=0;i<3;i++){try{await Promise.reject(i);}catch(e){if(e===1)break;x+=e;}}return x;",
            "try{for(let i=0;i<3;i++){if(i===1)await Promise.reject(b);a+=1;}}catch(e){return a+e;}return a;",
            "let f=()=>a;try{await Promise.reject(b);}catch(e){f=()=>+e;}await null;return f();",
            "let f=()=>a;for(let i=0;i<2;i++){try{await Promise.reject(i);}catch(e){f=()=>+e;}}return f();",
            "let f=()=>a;let old=()=>b;for(let i=0;i<2;i++){try{await Promise.reject(i);}catch(e){old=f;f=()=>+e;}}return old()*10+f();",
            "try{await Promise.reject(a);}catch(e){const f=()=>{e=b;return +e;};await null;return f();}return b;",
        ],
        &[
            (4.0, 2.0),
            (0.0, -2.0),
            (-0.0, 0.0),
            (f64::NAN, f64::INFINITY),
            (f64::INFINITY, f64::NEG_INFINITY),
            (f64::from_bits(1), f64::MAX),
        ],
    );
}

#[test]
fn catch_keeps_profile_limits_and_resource_failures_terminal() {
    for body in [
        "try{return await a;}finally{return b;}",
        "try{return await a;}catch(e){return e;}finally{a=b;}",
        "try{return await a;}catch({x}){return x;}",
        "try{await a;}catch(e){a=+e;}return e;",
        "const f=()=>{try{return a;}catch(e){return +e;}};return f();",
    ] {
        assert!(compile_async(&source_for(body), false).is_err(), "{body}");
    }
    let plain = "export async function run(a,b){try{return await Promise.reject(a);}catch(e){e=b;return await e;}}";
    let typed = "export async function run(a:never,b:Missing):Wrong{try{return await Promise.reject((a as unknown));}catch(e:unknown){e=b;return await(e as number);}}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    for count in [127, 128] {
        let locals = (0..count)
            .map(|i| format!("const x{i}=a;"))
            .collect::<String>();
        let source = source_for(&format!(
            "{locals}try{{return await a;}}catch(e){{return e;}}"
        ));
        let result = compile_async(&source, false);
        if count == 127 {
            result.unwrap();
        } else {
            assert!(result.err().unwrap().contains("128 source locals"));
        }
    }
    for count in [16, 17] {
        let source = source_for(&format!(
            "try{{{}return a;}}catch(e){{{}return e;}}",
            "await a;".repeat(count),
            "await b;".repeat(16)
        ));
        let result = compile_async(&source, false);
        if count == 16 {
            result.unwrap();
        } else {
            assert!(result.err().unwrap().contains("32 await sites"));
        }
    }
    let ordinary = "export function run(a,b){try{return a;}catch(e){return +e;}}";
    assert!(
        grunkalunka_compiler::compile(ordinary, false)
            .err()
            .unwrap()
            .contains("statement is outside the numeric experiment")
    );
    assert!(
        grunkalunka_compiler::compile_value(ordinary, false)
            .err()
            .unwrap()
            .contains("statement is outside the numeric experiment")
    );
    for count in [32, 33] {
        let body = format!(
            "let x=0;for(let i=0;i<{count};i++){{try{{await Promise.reject(i);}}catch(e){{x+=e;}}}}return x;"
        );
        let artifact = compile_async(&source_for(&body), false).unwrap();
        if count == 32 {
            compare_promises(&[&body], &[(4.0, 2.0)]);
        }
        let mut call = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
        let mut failed = false;
        while matches!(call.state().unwrap(), AsyncState::Pending) {
            if call.run_one_job().is_err() {
                failed = true;
                break;
            }
        }
        if count == 32 {
            assert!(!failed);
            assert!(matches!(
                call.state().unwrap(),
                AsyncState::Fulfilled(Value::Number(496.0))
            ));
        } else {
            assert!(failed);
            assert!(call.state().is_err());
            assert!(call.run_one_job().is_err());
            // Host job-budget errors cannot enter a source catch in either adapter.
            run_node(&format!(
                r#"import assert from 'node:assert/strict';import {{pathToFileURL}} from 'node:url';const {{createAsyncFunction}}=await import(pathToFileURL(process.argv[2]));await assert.rejects(createAsyncFunction(new Uint8Array({:?}))(4,2), /32-job budget/);"#,
                artifact.wasm
            ));
        }
    }
    let wasm = compile_async(
        &source_for("try{await null;for(;;)a+=1;}catch(e){return e;}return a;"),
        false,
    )
    .unwrap()
    .wasm;
    let mut call = AsyncInstance::new(&wasm, 4.0, 2.0).unwrap();
    assert!(matches!(call.state().unwrap(), AsyncState::Pending));
    assert!(call.run_one_job().is_err());
    assert!(call.state().is_err());
    assert!(call.run_one_job().is_err());
}

#[test]
fn runtime_block_factories_keep_synchronous_bodies_and_async_caller_jobs() {
    compare_promises(
        &[
            "let make=x=>{return ()=>x+b;};const f=make(await a);await null;return f();",
            "const make=a?(x=>{return ()=>x+a;}):(x=>{return ()=>x-b;});return make(await b)();",
            "let make=x=>{let y=x;return ()=>++y;};const f=make(a);await null;const g=make(b);return f()+g()+f();",
            "let make=x=>{let i=0;while(i<3){i++;if(x)return ()=>i+b;}return ()=>i;};const f=make(await a);await false;return f();",
            "let make=x=>{return ()=>null;};const f=make(a);await null;return f();",
            "let make=x=>{if(x)return ()=>null;return ()=>false;};const f=make(await a);await true;return f();",
            "let make=x=>{return ()=>{if(x)return b;};};return await make(await a)();",
            "let make=x=>{return ()=>null;};const old=make;await null;make=x=>{return ()=>false;};return old(a)();",
            "let make=x=>{return ()=>x+a;};const f=make((await null,make=x=>{return ()=>x-b;},a));return f();",
            "let n=0;let make=x=>{n++;return ()=>x+n;};const f=make(a);await null;const g=make(b);return f()+g()+n;",
            "let make=x=>{return ()=>null;};make&&=(await null,x=>{return ()=>false;});return make(a)();",
            "let make=x=>{return ()=>null;};make??=(await Promise.reject(a),x=>{return ()=>false;});return make(a)();",
            "let make=(fn,x)=>{return ()=>fn(x);};const f=make(y=>y+b,await a);await null;return f();",
            "let make=x=>{return ()=>null;};const f=make(a);try{return await Promise.reject(f());}catch(e){return e;}",
        ],
        &[
            (4.0, 2.0),
            (0.0, 2.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    let plain = "export async function run(a,b){let make=x=>{if(x)return ()=>null;return ()=>false;};const f=make(await a);return f();}";
    let typed = "export async function run(a:never,b:Missing):Wrong{let make:Wrong=((x:Missing)=>{if(x)return (()=>null) as any;return (()=>false) as any;}) as any;const f=make(await(a as never));return (f as any)();}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
}

#[test]
fn primitive_factory_leaves_keep_async_pairs_values_and_jobs() {
    compare_promises(
        &[
            "let make=x=>()=>true;return make(a)();",
            "let make=x=>()=>null;const f=make(a);await false;return f();",
            "const make=a?x=>()=>false:x=>()=>undefined;const f=make(await b);await null;return f();",
            "let make=x=>()=>{if(x)return b;};return await make(await a)();",
            "let make=x=>()=>null;const f=make((await null,make=x=>()=>false,a));await true;return f();",
            "let make=x=>()=>null;const old=make;await null;make=x=>()=>false;return old(a)();",
            "let make=x=>()=>null;make&&=(await null,x=>()=>false);return make(a)();",
            "let make=x=>()=>null;make??=(await Promise.reject(a),x=>()=>false);return make(a)();",
            "let make=x=>()=>{const old=x;x=null;return old;};const f=make(a);await null;f();return f();",
            "let make=x=>()=>a?null:false;const f=make(a);await null;a=0;return f();",
            "let make=x=>()=>null;try{const f=make(await Promise.reject(a));return f();}catch(e){return e;}",
            "let make=x=>()=>null;const f=make(a);try{return await Promise.reject(f());}catch(e){return e;}",
            "let make=x=>()=>false;const f=make(a);return await Promise.reject(f());",
            "let make=x=>()=>undefined;const f=make(a);return await f();",
        ],
        &[
            (4.0, 2.0),
            (0.0, 2.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    let plain = "export async function run(a,b){let make=x=>()=>null;const f=make(await a);await false;return f();}";
    let typed = "export async function run(a:never,b:Missing):Wrong{let make:(x:number)=>()=>number=(x=>()=>null) as any;const f=make(await(a as never));await(false as number);return f();}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
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
fn function_logical_assignments_keep_skipped_jobs_and_abrupt_stores() {
    compare_promises(
        &[
            "let f=()=>a;f&&=(await null,()=>b);return f();",
            "let f=()=>a;f||=(await null,()=>b);return f();",
            "let f=()=>a;f??=(await null,()=>b);return f();",
            "let f=()=>a;const old=f;const selected=f&&=(await false,()=>b);await true;f=()=>a+b;return selected()+old()+f();",
            "let f=()=>a;const selected=f||=(await Promise.reject(b),()=>b);await null;f=()=>b;return selected()+f();",
            "let f=()=>a;const selected=f??=(await Promise.reject(b),()=>b);await null;f=()=>b;return selected()+f();",
            "let n=0;const make=x=>{n++;return ()=>x;};let f=()=>a;f&&=make(await b);return f()+n;",
            "let n=0;const make=x=>{n++;return ()=>x;};let f=()=>a;f||=make(await b);return f()+n;",
            "let n=0;const make=x=>{n++;return ()=>x;};let f=()=>a;f??=make(await b);return f()+n;",
            "let f=()=>a;try{f&&=(await Promise.reject(b),()=>b);}catch(e){return f()+e;}return 99;",
            "let f=()=>a;try{f&&=(f=()=>b,await Promise.reject(a),()=>a+b);}catch(e){return f()+e;}return 99;",
            "let f=()=>a;f&&=(f=()=>a+b,await null,()=>b);return f();",
            "let f=()=>a;const read=()=>f();await null;f&&=(await false,()=>b);return read();",
            "let f=x=>x+a;return (f&&=(await null,x=>x-b))(await a);",
            "let f=()=>null;f&&=(await true,()=>false);return f();",
            "let f=()=>null;f||=(await true,()=>false);return f();",
            "let f=()=>null;f??=(await true,()=>false);return f();",
        ],
        &[
            (4.0, 2.0),
            (0.0, 2.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    let plain = "export async function run(a,b){let f=()=>a;f&&=(await null,()=>b);return f();}";
    let typed = "export async function run(a:never,b:Missing):Wrong{let f:(()=>string)=(()=>a) as any;f&&=(await(null as never),(()=>b) as any);return f();}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
}

#[test]
fn local_callback_arguments_keep_order_and_captures_across_await() {
    compare_promises(
        &[
            "const apply=(fn,x)=>fn(x);return apply(x=>x+a,await b);",
            "const apply=(fn,x)=>fn(x);let f=x=>x+a;return apply(f,await(f=x=>x-b,a))+f(a);",
            "const apply=(fn,x)=>fn(x);return apply((await null,x=>x+a),b);",
            "const apply=(fn,x)=>fn(x);return apply(a?x=>x+a:x=>x-b,await b);",
            "const apply=(fn,x)=>fn(x);const relay=(f,y)=>apply(f,y);await null;return relay(x=>x+a,b);",
            "const apply=(fn,x)=>{const alias=fn;return alias(x);};return await apply(x=>x+a,b);",
            "const make=fn=>x=>fn(x);const f=make(x=>x+a);await null;a=b;return f(b);",
            "const make=fn=>()=>fn();const f=a?make(()=>a):make(()=>b);await false;return f();",
            "const make=fn=>()=>fn();const f=make((await null,()=>a));await true;return f();",
            "const apply=(fn,x)=>fn(x);await null;return await Promise.reject(apply(x=>x+a,b));",
            "const apply=(fn,x)=>{let y=fn(x);y|=1;return y;};return await apply(x=>x+a,b);",
        ],
        &[
            (4.0, 2.0),
            (0.0, 2.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    let plain =
        "export async function run(a,b){const apply=(fn,x)=>fn(x);return apply(x=>x+a,await b);}";
    let typed = "export async function run(a:never,b:Missing):Wrong {const apply=<T>(fn:T,x:never):string=>(fn as any)(x);return apply(((x:number)=>x+a) as any,await(b as unknown as number));}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
}

#[test]
fn local_function_results_keep_pairs_effects_and_jobs_across_await() {
    compare_promises(
        &[
            "const id=fn=>fn;const f=id(()=>a);await null;return f();",
            "const id=fn=>fn;return id(x=>x+a)(await b);",
            "const id=fn=>fn;return id((await null,x=>x+a))(await b);",
            "const choose=(f,g,c)=>c?f:g;const h=choose(()=>a,()=>b,await a);await true;return h();",
            "const id=x=>x;const forward=fn=>id(fn);const f=forward(()=>a);await false;return id(b)+f();",
            "let current=()=>a;const get=()=>current;const f=get();await false;current=()=>b;return f()*10+current();",
            "let current=()=>a;const get=x=>current;const f=get(await(current=()=>b,1));await true;current=()=>a;return f();",
            "const pass=fn=>(fn(),fn);const f=pass(()=>++a);await null;return f()+a;",
            "const id=fn=>fn;let f=x=>x+a;return id(f)(await(f=x=>x-b,b))+f(a);",
            "const make=x=>()=>++x;const id=fn=>fn;const f=id(make(await a));await null;return f()*10+f();",
            "const get=()=>a?()=>a:()=>b;await null;return get()();",
            "const replace=fn=>(fn=()=>b);const f=replace(()=>a);await null;return f();",
            "const id=fn=>fn;const f=id(()=>a);return await Promise.reject(f());",
            "const id=fn=>fn;const f=id(()=>+a);await null;a=await true;return f();",
        ],
        &[
            (4.0, 2.0),
            (0.0, 2.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    let plain = "export async function run(a,b){const id=fn=>fn;return id(x=>x+a)(await b);}";
    let typed = "export async function run(a:never,b:Missing):Wrong {const id=<T>(fn:T):T=>(fn as T);return ((id as any)(((x:number)=>x+a) as any) as any)(await(b as unknown as number));}";
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
    let sources = bodies
        .iter()
        .map(|body| source_for(body))
        .collect::<Vec<_>>();
    compare_async_sources(&sources, vectors);
}

fn compare_async_sources(sources: &[String], vectors: &[(f64, f64)]) {
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
    for source in sources {
        for typescript in [false, true] {
            let artifact = compile_async(source, typescript)
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
            script.push_str(&format!("{{const native=(()=>{{{};return run;}})();const wasm=createAsyncFunction(new Uint8Array({:?}));const rust=[{}];let i=0;for(const [a,b] of vectors){{const expected=await observe(native,a,b);assert.deepEqual(await observe(wasm,a,b),expected,{:?});assert.equal(rust[i].outcome,expected.outcome);assert.equal(rust[i].jobs,expected.jobs);i++;}}const [a,b]=vectors[0], [c,d]=vectors.at(-1);assert.deepEqual(await Promise.allSettled([wasm(a,b),wasm(c,d)]),await Promise.allSettled([native(a,b),native(c,d)]));}}\n",source.replacen("export async function run", "async function run", 1),artifact.wasm,rust.join(","),source));
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
        for status in [0, 1, 2, 3] {
            let wasm=wat::parse_str(format!(r#"(module
(func(export "start")(param f64 f64)(result i32 i32 f64)i32.const {status} i32.const {tag} f64.const {payload})
(func(export "resume")(param i32 f64)(result i32 i32 f64)unreachable))"#)).unwrap();
            assert!(AsyncInstance::new(&wasm, 4.0, 2.0).is_err());
            script.push_str(&format!("await assert.rejects(createAsyncFunction(new Uint8Array({wasm:?}))(4,2),TypeError);\n"));
        }
        for status in [0, 1, 2, 3] {
            let wasm=wat::parse_str(format!(r#"(module
(func(export "start")(param f64 f64)(result i32 i32 f64)i32.const 1 i32.const 2 f64.const 1)
(func(export "resume")(param i32 f64)(result i32 i32 f64)i32.const {status} i32.const {tag} f64.const {payload}))"#)).unwrap();
            let mut call = AsyncInstance::new(&wasm, 4.0, 2.0).unwrap();
            assert!(call.run_one_job().is_err());
            assert!(call.state().is_err());
            assert!(call.run_one_job().is_err());
            script.push_str(&format!(
            "await assert.rejects(createAsyncFunction(new Uint8Array({wasm:?}))(4,2),TypeError);\n"
        ));
        }
    }
    run_node(&script);
}

#[test]
fn selected_higher_order_calls_preserve_pairs_across_await() {
    compare_promises(
        &[
            "const apply=(fn,x)=>fn(x);const twice=(fn,x)=>fn(fn(x));const h=a?apply:twice;await null;return h(x=>x+b,a);",
            "let h=(fn,x)=>fn(x);let cb=x=>x+a;return h(cb,await(h=(fn,x)=>fn(x)*10,cb=x=>x-b,a))+h(cb,b);",
            "const h=a?(fn,x)=>fn(x):(fn,x)=>fn(fn(x));return h((await null,x=>x+b),await a);",
            "const h=a?(fn,x)=>{const invoke=a?()=>fn(x):()=>fn(x)+1;fn=y=>y+b;return invoke();}:(fn,x)=>fn(x);await true;return h(x=>x+a,b);",
            "const make=x=>()=>++x;const h=a?fn=>fn()*10+fn():fn=>fn();return h((await null,make(b)));",
            "const h=a?(fn,x)=>fn(x):(fn,x)=>fn(fn(x));return await Promise.reject(h(x=>x+b,a));",
            "let h=(fn,x)=>fn(x);try{await Promise.reject(a);}catch(reason){const cb=x=>x+ +reason;await null;return h(cb,b);}return 0;",
            "const h=a?(fn,x)=>fn(x):(fn,x)=>fn(fn(x));try{return h(x=>x+b,await Promise.reject(a));}catch(reason){return h(x=>x+ +reason,b);}",
            "let h=(fn,x)=>fn(x);const saved=h;try{await Promise.reject(b);}catch(reason){h=(fn,x)=>fn(x)+ +reason;await null;return saved(x=>x+a,b)+h(x=>x+b,a);}return 0;",
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
fn block_function_results_survive_async_suspension() {
    compare_promises(
        &[
            "const pick=flag=>{if(flag)return (fn,x)=>fn(x);return (fn,x)=>fn(fn(x));};const h=pick(a);try{await Promise.reject(b);}catch(reason){await null;return h(x=>x+ +reason,a);}return 0;",
            "try{await Promise.reject(a);}catch(reason){const make=()=>{return (fn,x)=>fn(x)+ +reason;};const h=make();await null;return h(x=>x+b,b);}return 0;",
            "const pick=flag=>{if(flag)return x=>x+a;return x=>x-b;};const f=pick(a);await null;return f(b);",
            "const pick=x=>{let value=x;return ()=>++value;};const f=pick(await a);const g=pick(await b);return f()*100+g()*10+f();",
            "let current=a?()=>a:()=>b;const pick=()=>{return current;};const f=pick();await false;current=()=>b*10;return f()+current();",
            "const pick=flag=>{for(let i=0;i<3;i++){if(flag&&i===1)return ()=>i+b;}return ()=>a;};const f=pick(a);await true;return await f();",
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
fn runtime_factory_results_keep_cells_and_snapshots_across_async_jobs() {
    compare_promises(
        &[
            "const one=x=>()=>++x;const two=x=>()=>x+b;const make=a?one:two;const f=make(await a);await null;return f()+f();",
            "let make=x=>()=>x+a;const f=make(await(make=x=>()=>x-b,a));await null;return f()+make(b)();",
            "let leaf=x=>x+a;let make=()=>leaf;const f=make();await null;leaf=x=>x-b;return f(a)+make()(a);",
            "const factory=a?(fn,x)=>()=>fn(x):(fn,x)=>()=>fn(x)+b;const f=factory(x=>x+a,await b);await null;return f()+f();",
            "let make=x=>()=>x;make=x=>x<=0?()=>b:make(x-1);await null;return make(3)();",
            "let make=x=>()=>x;try{await Promise.reject(a);}catch(reason){make=x=>()=>x+ +reason;await null;return make(b)();}return 0;",
            "let make=x=>()=>x+a;const saved=make;try{return make(await Promise.reject(a))();}catch(reason){return saved(+reason)();}",
            "let make=x=>()=>x;try{throw a;}catch(reason){make=x=>()=>x+ +reason;await null;return make(b)();}",
            "let make=x=>()=>x+a;try{throw make(await b)();}catch(reason){await null;return +reason;}",
            "const pick=c=>{if(c)return x=>()=>x+a;return x=>()=>x-b;};const make=pick(a);await null;return make(b)();",
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
fn primitive_coalescing_preserves_nullish_selection_jobs_and_catches() {
    compare_promises(
        &[
            "return a??await b;",
            "return a??await Promise.reject(b);",
            "return false??await Promise.reject(b);",
            "return NaN??await Promise.reject(b);",
            "return null??await b;",
            "return undefined??await null;",
            "return (await (a?null:false))??await b;",
            "try{return null??await Promise.reject(b);}catch(reason){return reason;}",
            "try{return (await Promise.reject(a))??b;}catch(reason){await null;return reason;}",
            "let x=a?null:false;const result=x??(await null,x=b);x=null;await null;return result;",
            "if((a?null:false)??await b)return a;return b;",
            "let n=0;(n++,a?undefined:a)??(await null,n++);return n;",
            "let x=a?null:false;x??(x=await b);return x;",
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
fn nullish_function_results_skip_rhs_jobs_and_keep_left_failures_and_snapshots() {
    compare_promises(
        &[
            "const f=()=>a;const g=()=>b;const h=f??(await null,g);return h();",
            "const f=()=>a;const g=()=>b;const h=f??(await Promise.reject(b),g);return h();",
            "const f=()=>a;const g=()=>b;const h=(await null,f)??(await Promise.reject(b),g);return h();",
            "const f=()=>a;const g=()=>b;try{const h=(await Promise.reject(b),f)??g;return h();}catch(reason){await null;return +reason;}",
            "let f=()=>a;const g=()=>b;const h=(await null,f)??(f=g);f=g;await null;return h()*10+f();",
            "const f=(fn,x)=>fn(x);const g=(fn,x)=>fn(fn(x));const h=f??(await null,g);const relay=(cb,x)=>h(cb,x);return relay(x=>x+b,a);",
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
fn logical_function_results_preserve_effects_snapshots_and_skipped_jobs() {
    compare_promises(
        &[
            "const f=()=>a;const g=()=>b;const h=f||(await null,g);return h();",
            "const f=()=>a;const g=()=>b;const h=f&&(await null,g);return h();",
            "const f=()=>a;const g=()=>b;const h=f||(await Promise.reject(b),g);return h();",
            "const f=()=>a;const g=()=>b;try{const h=f&&(await Promise.reject(b),g);return h();}catch(reason){await null;return +reason;}",
            "let f=()=>a;const old=f;const h=(await null,f)||(f=()=>b);f=()=>b;await null;return h()*10+old()+f();",
            "let calls=0;const f=()=>++calls;const g=()=>--calls;const h=(calls+=10,f)&&(await null,calls*=2,g);return h()*1000+calls;",
            "const f=()=>a;const g=()=>b;const h=(f||(await null,g))&&(await null,g);return h();",
            "const apply=(fn,x)=>fn(x);const twice=(fn,x)=>fn(fn(x));const h=apply&&(await null,twice);return h(x=>x+b,a);",
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
fn function_truthiness_preserves_async_conditions_effects_and_catches() {
    compare_promises(
        &[
            "let make=x=>()=>x;try{if(make(await a))throw b;}catch(reason){await null;return +reason;}return 99;",
            "let calls=0;let make=x=>(calls++,()=>++calls);try{if(make(await a))throw b;}catch(reason){await null;return calls*10+ +reason;}return 99;",
            "let calls=0;const f=a?()=>++calls:()=>--calls;await null;return f?calls:b;",
            "const make=x=>()=>x;if(make(await a))return b;return a;",
            "const get=()=>{a++;return ()=>b++;};return get()?await a:b;",
            "const f=()=>a;return !f;",
            "let calls=0;const f=()=>++calls;if(f||(await Promise.reject(a)))return calls;return 99;",
            "let calls=0;const f=()=>++calls;try{if(f&&(await Promise.reject(a)))return 99;}catch(reason){return calls+ +reason;}return 0;",
            "const h=a?(fn,x)=>fn(x):(fn,x)=>fn(fn(x));try{await Promise.reject(b);}catch(reason){await null;return h?+reason:a;}return 0;",
            "let f=()=>a;const old=f;if(f=(await null,()=>b))return (old?10:20)+(f?1:2);return 99;",
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
fn primitive_closure_results_preserve_async_values_and_job_order() {
    compare_promises(
        &[
            "const f=()=>null;return await f();",
            "const f=()=>undefined;return await f();",
            "const f=()=>false;return await f();",
            "const f=()=>a?null:false;return await f();",
            "const f=()=>null;const g=()=>false;let h=a?f:g;return await h();",
            "const f=()=>null;return f()??await b;",
            "const f=()=>false;return f()??await b;",
            "let x=a?null:false;const f=()=>x;const g=()=>undefined;let h=a?f:g;const old=h();await b;x=true;return old;",
            "const f=()=>null;try{throw f();}catch(reason){await b;return reason;}",
            "const f=()=>undefined;try{await Promise.reject(f());}catch(reason){return reason;}return null;",
            "let x=0;const f=y=>((x+=y),false);const g=y=>null;let h=f;const old=h((h=g,await a));await b;return old||x;",
            "const f=()=>null;let x=a;x=f();await b;return x;",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn primitive_block_results_preserve_async_values_and_job_order() {
    compare_promises(
        &[
            "const f=()=>{return null;};return await f();",
            "const f=()=>{if(a)return undefined;return false;};return await f();",
            "const f=()=>{return null;};const g=()=>{return false;};let h=a?f:g;return await h();",
            "const f=()=>{return null;};return f()??await b;",
            "const f=()=>{return false;};return f()??await b;",
            "let x=a?null:false;const f=()=>{const old=x;x=b;return old;};const old=f();await b;x=true;return old;",
            "const f=()=>{return null;};try{throw f();}catch(reason){await b;return reason;}",
            "let x=0;const f=y=>{x+=y;return false;};const g=y=>null;let h=f;const old=h((h=g,await a));await b;return old||x;",
            "const f=()=>{return null;};let x=a;x=f();await b;return x;",
            "const f=()=>{let i=0;while(i<3){i++;if(a)return null;}return false;};const g=()=>undefined;let h=a?f:g;await b;return h();",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn implicit_local_returns_preserve_async_kinds_effects_and_job_order() {
    compare_promises(
        &[
            "const f=()=>{};return await f();",
            "const f=()=>{return;};return await f();",
            "const f=()=>{if(a)return b;};return await f();",
            "const f=()=>{if(a)return b;};return f()??await b;",
            "let x=0;const f=()=>{x++;return;};const old=f();await b;return old??x;",
            "const f=x=>{if(x)return;};const g=x=>false;let h=f;const old=h((h=g,await a));await b;return old;",
            "const f=()=>{};try{throw f();}catch(reason){await b;return reason;}",
            "const f=()=>{};try{await Promise.reject(f());}catch(reason){return reason;}return null;",
            "const make=()=>()=>{};const f=make();await b;return f();",
            "const f=()=>{};let x=a;x=f();await b;return x;",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn known_async_arrows_fulfill_primitives_without_extra_jobs() {
    compare_promises(
        &[
            "const f=async()=>a;return await f();",
            "const f=async()=>-0;return await f();",
            "const f=async()=>NaN;return await f();",
            "const f=async()=>null;return await f();",
            "const f=async()=>false;return await f();",
            "const f=async()=>undefined;return await f();",
            "const f=async()=>{};return await f();",
            "const f=async()=>{return;};return await f();",
            "const f=async()=>{if(a)return b;};return await f();",
            "const f=async x=>x?null:false;return await f(a);",
            "const f=async()=>a;const alias=f;return await alias();",
            "return await (async x=>x+a)(b);",
            "const f=async()=>a;return await Promise.resolve(f());",
            "const f=async()=>a;return await Promise.resolve(((f())));",
            "const f=async x=>a;return await f(await b);",
            "const f=async x=>x;return await f(await a);",
            "const f=async(fn,x)=>fn(x);return await f(x=>x+a,b);",
            "const f=async()=>a;const g=async()=>b;const x=await f();return x+await g();",
            "let count=0;const f=async(x,y)=>{count+=10;return x+y;};const out=await f((count++,a),(count+=2,b));return out+count;",
            "let value=a;const f=async()=>value;const old=await f();value=b;return old;",
            "const f=async x=>a;return await f((a=b,a));",
            "const f=async()=>a?null:false;if(a)return await f();return b;",
            "const f=async()=>a?null:false;try{await Promise.reject(await f());}catch(reason){await null;return reason;}return b;",
            "const f=async()=>a;return a?await f():await f();",
            "const f=async()=>a;return b;",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn known_async_arrows_keep_promise_object_and_body_boundaries() {
    for body in [
        "const f=async()=>a;return f();",
        "const f=async()=>a;f();return a;",
        "const f=async()=>a;return f;",
        "const f=a?async()=>a:async()=>b;return await f();",
        "let f=async()=>a;return await f();",
        "const f=async()=>Promise.reject(b);return await f();",
        "const f=async()=>()=>a;return await f();",
        "const f=async()=>{return ()=>a;};return await f();",
        "const f=async()=>'';return await f();",
        "const f=async()=>({});return await f();",
        "const f=async x=>x;return await f(null);",
        "const f=async()=>a;return await f?.();",
        "const f=async()=>a;return await f(...a);",
    ] {
        let source = source_for(body);
        assert!(compile_async(&source, false).is_err(), "{source}");
    }
    let source = source_for(
        "const f=async()=>a;try{await Promise.reject(f());}catch(reason){return reason;}return b;",
    );
    assert!(
        compile_async(&source, false)
            .err()
            .unwrap()
            .contains("Promise-valued reason")
    );
    let source = "export function run(a,b){const f=async()=>a;return a;}";
    assert!(grunkalunka_compiler::compile(source, false).is_err());
    assert!(grunkalunka_compiler::compile_value(source, false).is_err());
}

#[test]
fn known_async_arrow_type_claims_do_not_override_runtime_primitives() {
    let js = "export async function run(a,b){const f=async x=>x?null:false;return await f(a);}";
    let ts = "export async function run(a:number,b:number):Promise<unknown>{const f=async(x:number):Promise<number>=>(x?null:false) as unknown as number;return await f(a);}";
    assert_eq!(
        compile_async(js, false).unwrap().wasm,
        compile_async(ts, true).unwrap().wasm
    );
}

#[test]
fn known_concise_async_bodies_preserve_body_and_caller_jobs() {
    compare_promises(
        &[
            "const f=async()=>await a;return await f();",
            "const f=async()=>await null;return await f();",
            "const f=async()=>await false;return await f();",
            "const f=async()=>await undefined;return await f();",
            "const f=async()=>await -0;return await f();",
            "const f=async()=>await NaN;return await f();",
            "const f=async()=>await Promise.resolve(a);return await f();",
            "const f=async()=>await (await a);return await f();",
            "const f=async()=>a?await null:await false;return await f();",
            "const f=async x=>x+await b;return await f(await a);",
            "const f=async()=>a+await(a=b);return await f();",
            "let value=a;const f=async x=>(value+=x,value+await b);const out=await f((value=b,a));return out+value;",
            "const g=async()=>await b;const f=async()=>a+await g();return await f();",
            "const f=async()=>await(async()=>await a)();return await f();",
            "const f=async()=>await a;const alias=f;return await alias();",
            "const f=async()=>await false;if(await f())return a;return b;",
            "const f=async()=>await undefined;return +(await f());",
            "const f=async()=>await a;try{await Promise.reject(await f());}catch(reason){return reason;}return b;",
            "const f=async()=>await a;try{await Promise.reject(b);}catch(reason){return await f();}return b;",
            "const f=async(fn,x)=>fn(await x);return await f(x=>x+a,b);",
            "let total=0;for(let i=0;i<3;i++){const f=async()=>i+await b;total+=await f();}return total;",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn concise_async_promise_object_guards_do_not_change_root_rejection() {
    for body in [
        "const f=async()=>await a;return f();",
        "const f=async()=>await a;const p=f();return await p;",
        "const f=async()=>await({});return await f();",
    ] {
        let source = source_for(body);
        assert!(compile_async(&source, false).is_err(), "{source}");
    }
    compare_promises(
        &["try{await Promise.reject(a);}catch(reason){return reason;}return b;"],
        &[(4.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn concise_async_body_type_claims_preserve_general_wait_values() {
    let js =
        "export async function run(a,b){const f=async x=>await(x?null:false);return await f(a);}";
    let ts = "export async function run(a:number,b:number):Promise<unknown>{const f=async(x:number):Promise<number>=>await((x?null:false) as unknown as number);return await f(a);}";
    assert_eq!(
        compile_async(js, false).unwrap().wasm,
        compile_async(ts, true).unwrap().wasm
    );
}

#[test]
fn known_async_blocks_preserve_body_and_caller_jobs() {
    compare_promises(
        &[
            "const f=async()=>{return await a;};return await f();",
            "const f=async()=>{return await null;};return await f();",
            "const f=async()=>{return await false;};return await f();",
            "const f=async()=>{return await undefined;};return await f();",
            "const f=async()=>{return await -0;};return await f();",
            "const f=async()=>{return await NaN;};return await f();",
            "const f=async()=>{if(a)return b;return await null;};return await f();",
            "const f=async()=>{let x=a;await b;return x;};return await f();",
            "const f=async()=>{await a;return;};return await f();",
            "const f=async()=>{await a;};return await f();",
            "const f=async()=>{let x=0;for(let i=0;i<3;i++){if(i===1)continue;x+=await b;if(a)break;}return x;};return await f();",
            "const f=async()=>{let i=0;while(i<3){i++;await b;if(a)return null;}return false;};return await f();",
            "let x=a;const f=async()=>{const old=x;await b;x=b;return old;};const out=await f();return out+x;",
            "const f=async x=>{const old=x;await b;x=a;return old;};return await f(await b);",
            "const f=async(fn,x)=>{const y=await x;return fn(y);};return await f(x=>x+a,b);",
            "const g=async()=>{return await b;};const f=async()=>{const x=await a;return x+await g();};return await f();",
            "const f=async()=>{return await(async()=>{await b;return a;})();};return await f();",
            "const f=async()=>{return await a;};const alias=f;return await alias();",
            "const f=async()=>{return await a;};try{await Promise.reject(b);}catch(reason){return reason+await f();}return b;",
            "const f=async()=>{return await a;};try{throw b;}catch(reason){return reason+await f();}",
            "const f=async()=>{try{return await a;}catch(reason){return b;}};return await f();",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn async_blocks_keep_promise_object_and_sync_guards() {
    for body in [
        "const f=async()=>{return await a;};return f();",
        "const f=async()=>{return await a;};const p=f();return await p;",
        "const f=()=>{return await a;};return await f();",
        "const f=async()=>{return ()=>a;};return await f();",
    ] {
        let source = source_for(body);
        assert!(compile_async(&source, false).is_err(), "{source}");
    }
}

#[test]
fn async_block_type_claims_preserve_general_wait_values() {
    let js = "export async function run(a,b){const f=async x=>{return await(x?null:false);};return await f(a);}";
    let ts = "export async function run(a:number,b:number):Promise<unknown>{const f=async(x:number):Promise<number>=>{return await((x?null:false) as unknown as number);};return await f(a);}";
    assert_eq!(
        compile_async(js, false).unwrap().wasm,
        compile_async(ts, true).unwrap().wasm
    );
}

#[test]
fn async_root_normal_completion_fulfills_undefined() {
    compare_promises(
        &[
            "",
            "return;",
            "await a;",
            "await a;return;",
            "if(a)return b;",
            "if(a)return;return b;",
            "if(a)throw b;",
            "if(a)return await b;await null;",
            "if(a)return await null;",
            "if(a)throw await b;await a;",
            "let undefined=a;await b;return;",
            "let undefined=a;await b;",
            "let x=a;const f=()=>++x;await f();",
            "for(let i=0;i<3;i++){await b;if(a)return;}",
            "for(let i=0;i<3;i++){await b;if(i===1)continue;if(a)break;}",
            "while(a){await b;return b;}",
            "do{await b;if(a)return null;}while(false);",
            "try{await Promise.reject(a);}catch(reason){await reason;}",
            "try{await Promise.reject(a);}catch(reason){if(reason)return b;}",
            "try{throw a;}catch(reason){await b;return;}",
            "try{if(a)throw b;await a;}catch(reason){throw reason;}",
            "const f=async()=>{await a;};await f();",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn async_root_completion_keeps_other_profile_and_abrupt_guards() {
    for body in ["", "return;", "if(a)return b;"] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(grunkalunka_compiler::compile(&source, false).is_err());
        assert!(grunkalunka_compiler::compile_value(&source, false).is_err());
    }
    for source in [
        "function helper(x){return;}export async function run(a,b){return helper(a);}",
        "function helper(x){x++;}export async function run(a,b){return helper(a);}",
        "export async function run(a,b){return;await a;}",
        "export async function run(a,b){break;}",
        "export async function run(a,b){continue;}",
        "export async function run(a,b){try{await a;}finally{a++;}}",
    ] {
        assert!(compile_async(source, false).is_err(), "{source}");
    }
}

#[test]
fn async_root_completion_ignores_type_claims() {
    let js = "export async function run(a,b){if(a)return;await b;}";
    let ts = "export async function run(a:number,b:number):Promise<number>{if(a)return;await(b as number);}";
    assert_eq!(
        compile_async(js, false).unwrap().wasm,
        compile_async(ts, true).unwrap().wasm
    );
}

#[test]
fn awaited_async_callee_rejection_keeps_its_caller_reaction() {
    compare_promises(
        &[
            "const f=async()=>{throw b;};return await f();",
            "const f=async()=>await Promise.reject(a);return await f();",
            "const f=async()=>(await Promise.reject(a),b);return await f();",
            "const f=async()=>a?await Promise.reject(b):await a;return await f();",
            "const f=async()=>await(async()=>await Promise.reject(a))();return await f();",
            "const f=async()=>await(async()=>{throw a;})();return await f();",
            "const f=async()=>{return await Promise.reject(a);};return await f();",
            "const f=async()=>{if(a)return await Promise.reject(b);return await a;};return await f();",
            "const f=async()=>{try{await Promise.reject(a);}catch(reason){return reason;}};return await f();",
            "const f=async()=>{throw a;};return await f();",
            "const f=async()=>{await b;throw a;};return await f();",
            "const f=async()=>{if(a)throw b;return await a;};return await f();",
            "const f=async()=>{try{throw a;}catch(reason){return await reason;}};return await f();",
            "const f=async()=>{throw null;};return await f();",
            "const f=async()=>{throw undefined;};return await f();",
            "const f=async()=>{throw false;};return await f();",
            "const f=async()=>{throw -0;};return await f();",
            "const f=async()=>{throw NaN;};return await f();",
            "const f=async()=>await Promise.reject(null);return await f();",
            "const f=async()=>await Promise.reject(false);return await f();",
            "const f=async()=>await Promise.reject(undefined);return await f();",
            "const f=async()=>{throw a;};try{return await f();}catch(reason){return reason;}",
            "const f=async()=>{await a;throw b;};try{return await f();}catch(reason){await null;return reason;}",
            "const f=async()=>{await a;throw b;};try{return await f();}catch(reason){throw await reason;}",
            "const f=async()=>{try{await Promise.reject(a);}catch(reason){throw reason+b;}};try{return await f();}catch(reason){return reason;}",
            "const g=async()=>{throw a;};const f=async()=>{return await g();};return await f();",
            "const g=async()=>{throw a;};const f=async()=>{try{return await g();}catch(reason){throw await b;}};try{return await f();}catch(reason){return reason;}",
            "const g=async()=>await Promise.reject(a);const f=async()=>{try{return await g();}catch(reason){return await reason;}};return await f();",
            "let x=0;const f=async y=>{x+=10;throw y;};try{await f(await Promise.reject(a));}catch(reason){return x+ +reason;}",
            "let x=a;const f=async()=>{x=b;await null;throw x++;};try{await f();}catch(reason){return reason+x;}",
            "const f=async(fn,x)=>{const y=await x;throw fn(y);};try{return await f(x=>x+a,b);}catch(reason){return reason;}",
            "const f=async()=>{for(let i=0;i<3;i++){await b;if(a)throw i;}return null;};try{return await f();}catch(reason){return reason;}",
            "const f=async()=>{throw a;};try{await Promise.reject(await f());}catch(reason){return reason;}",
            "const f=async()=>{try{await Promise.reject(a);}catch(reason){await reason;}};return await f();",
            "const f=async()=>{try{if(a)throw b;await a;}catch(reason){throw reason;}};return await f();",
            "const f=async()=>{throw a;};return b;",
            "const f=async()=>await(async()=>{return null;})();return await f();",
            "const f=async()=>await(async()=>{})();return await f();",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn awaited_callee_settlements_share_one_await_site_and_keep_job_limits() {
    for (callee, accepted_calls) in [
        ("async()=>a", 32),
        ("async()=>await a", 16),
        ("async()=>{throw a;}", 32),
    ] {
        for count in [accepted_calls, accepted_calls + 1] {
            let body = format!(
                "const f={callee};{}return b;",
                "try{await f();}catch(reason){a=+reason;}".repeat(count)
            );
            let result = compile_async(&source_for(&body), false);
            if count == accepted_calls {
                result.unwrap();
                compare_promises(&[&body], &[(4.0, 2.0)]);
            } else {
                assert!(result.err().unwrap().contains("32 await sites"));
            }
        }
    }
    for count in [16, 17] {
        let body =
            format!("const f=async()=>await a;for(let i=0;i<{count};i++)await f();return b;");
        let artifact = compile_async(&source_for(&body), false).unwrap();
        let mut call = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
        let mut failed = false;
        while matches!(call.state().unwrap(), AsyncState::Pending) {
            if call.run_one_job().is_err() {
                failed = true;
                break;
            }
        }
        if count == 16 {
            assert!(!failed);
            assert!(matches!(
                call.state().unwrap(),
                AsyncState::Fulfilled(Value::Number(2.0))
            ));
        } else {
            assert!(failed);
            assert!(call.state().is_err());
        }
    }
}

#[test]
fn awaited_callee_type_claims_do_not_coerce_rejection_or_fulfillment() {
    let js = "export async function run(a,b){const f=async x=>{if(x)throw null;return await false;};return await f(a);}";
    let ts = "export async function run(a:number,b:number):Promise<number>{const f=async(x:number):Promise<number>=>{if(x)throw null;return await(false as unknown as number);};return await f(a);}";
    assert_eq!(
        compile_async(js, false).unwrap().wasm,
        compile_async(ts, true).unwrap().wasm
    );
}

#[test]
fn synchronous_number_helpers_preserve_async_values_and_jobs() {
    let programs = [
        "function helper(a){return a;}export async function run(a,b){return await b;}",
        "function helper(x){return x;}export async function run(a,b){return helper(a);}",
        "function helper(x){return x;}export async function run(a,b){return await helper(a);}",
        "function helper(x){return x;}export async function run(a,b){return helper(await a);}",
        "function helper(x){return x;}export async function run(a,b){await b;return helper(a);}",
        "function helper(x,y){return x-y;}export async function run(a,b){return helper(a,await(a=b));}",
        "function zero(){return -0;}function one(x){return x;}function two(x,y){return x+y;}export async function run(a,b){return two(zero(),await one(a));}",
        "export async function run(a,b){return helper(await a,b);}function helper(x,y){return x-y;}",
        "function first(x){return second(x)+1;}function second(x){return x*2;}export async function run(a,b){return first(await a);}",
        "function factorial(x){if(x<=1)return 1;return x*factorial(x-1);}export async function run(a,b){await b;return factorial(4);}",
        "function rem(x,y){return x%y;}export async function run(a,b){return rem(await a,b);}",
        "function helper(x){const f=()=>++x;return f()+f();}export async function run(a,b){return helper(a)+await helper(b);}",
        "function helper(x){const f=y=>x+y;const g=y=>x-y;let h=x?f:g;return h(x);}export async function run(a,b){await b;return helper(a);}",
        "function helper(x){return x*2;}export async function run(a,b){const f=()=>helper(a);const g=()=>helper(b)+1;let h=a?f:g;await null;return h();}",
        "function helper(x){return x*2;}export async function run(a,b){const f=async()=>{const old=helper(a);await b;return old;};return await f();}",
        "function helper(x){return x*2;}export async function run(a,b){const f=async()=>{throw helper(await a);};try{return await f();}catch(reason){return helper(+reason);}}",
        "function helper(x){return x;}export async function run(a,b){try{return await Promise.reject(helper(a));}catch(reason){await b;return helper(+reason);}}",
        "function helper(x){return x;}export async function run(a,b){let x=a;for(let i=0;i<3;i++){x=helper(x)+await b;}return x;}",
        "function helper(x){return x;}export async function run(a,b){const f=()=>helper(a);await b;return helper(f());}",
    ].map(String::from);
    compare_async_sources(
        &programs,
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn helper_resource_traps_remain_terminal_outside_js_catches() {
    for delayed in [false, true] {
        let source = format!(
            "function helper(x){{for(let i=0;i<70000;i++){{const y=x;const f=()=>y;x=f();}}return x;}}export async function run(a,b){{{}try{{return helper(a);}}catch(reason){{return 0;}}}}",
            if delayed { "await b;" } else { "" }
        );
        let artifact = compile_async(&source, false).unwrap();
        if delayed {
            let mut call = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
            assert!(call.run_one_job().is_err());
            assert!(call.state().is_err());
        } else {
            assert!(AsyncInstance::new(&artifact.wasm, 4.0, 2.0).is_err());
        }
        run_node(&format!(
            "import assert from 'node:assert/strict';import {{pathToFileURL}} from 'node:url';const {{createAsyncFunction}}=await import(pathToFileURL(process.argv[2]));const run=createAsyncFunction(new Uint8Array({:?}));await assert.rejects(run(4,2),Error);",
            artifact.wasm
        ));
    }
}

#[test]
fn synchronous_helper_arity_limit_keeps_its_ordinary_call_abi() {
    let params = (0..1000)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let args = std::iter::once("await a")
        .chain(std::iter::repeat_n("b", 999))
        .collect::<Vec<_>>()
        .join(",");
    let source = format!(
        "function wide({params}){{return p0+p999;}}export async function run(a,b){{return wide({args});}}"
    );
    compare_async_sources(&[source], &[(4.0, 2.0)]);
    let source = format!(
        "function wide({params},extra){{return p0;}}export async function run(a,b){{return a;}}"
    );
    assert!(
        compile_async(&source, false)
            .err()
            .unwrap()
            .contains("1000 function parameters")
    );
}

#[test]
fn void_preserves_async_jobs_effects_and_rejection() {
    compare_promises(
        &[
            "return void(await a);",
            "return await(void a);",
            "return void await Promise.reject(a);",
            "try{return void await Promise.reject(null);}catch(reason){return reason;}",
            "let x=a;const f=async()=>{x=b;return await false;};const out=void await f();return out??x;",
            "return (void await a)&&await b;",
            "return (void await a)??await b;",
            "const f=()=>void a;return await f();",
            "const f=async()=>void(await a);return await f();",
            "const f=async()=>{throw a;};return void await f();",
            "const f=async()=>{throw a;};try{return void await f();}catch(reason){return void(await reason);}",
            "if(void(await a))return a;return b;",
            "let x=0;for(let i=0;i<3;i++){void(await(x++));}return x;",
            "const f=()=>a;return void(a?f:()=>b);",
            "let x=a;const f=()=>{x=b;return ()=>x;};void f();await null;return x;",
            "let undefined=a;return void await b;",
            "const f=async x=>void(await x);return await f(await a);",
            "void(await a);return b;",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    for body in [
        "const f=async()=>a;return void f();",
        "const f=async()=>a;return void f;",
        "const f=async()=>Promise.reject(a);return void await f();",
    ] {
        assert!(compile_async(&source_for(body), false).is_err(), "{body}");
    }
}

#[test]
fn nullish_assignment_preserves_selected_values_cells_and_jobs() {
    compare_promises(
        &[
            "let value=null;return value??=await a;",
            "let value;return value??=await false;",
            "let value=false;return value??=await a;",
            "let value=0;return value??=await a;",
            "let value=-0;return value??=await a;",
            "let value=NaN;return value??=await a;",
            "let value=a?null:false;return value??=await b;",
            "let value=null;const old=value??=await(value=false,b);return +old+(+value);",
            "let value=null;const read=()=>value;const old=value??=await(value=false,b);return +old+(+read());",
            "let value=a?false:0;return value??=await(value=null,b);",
            "let value=null;if(value??=await a)return b;return false;",
            "let value=NaN;return value??=await Promise.reject(b);",
            "let value=null;return value??=await Promise.reject(b);",
            "let value=null;try{value??=await Promise.reject(b);}catch(reason){return value===null?reason:false;}return 0;",
            "let value=null;try{value??=await Promise.reject((value=false,b));}catch(reason){return value;}return b;",
            "let value;for(let i=0;i<2;i++){value??=await undefined;}return value;",
            "a=null;return a??=await b;",
            "const undefined=a;let value=undefined;return value??=await b;",
            "let value=null;const set=async()=>{value=false;return await b;};const old=value??=await set();return +old+(+value);",
            "const f=async()=>{throw b;};a=false;return a??=await f();",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}
