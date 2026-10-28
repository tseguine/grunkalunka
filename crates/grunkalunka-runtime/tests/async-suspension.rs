use grunkalunka_compiler::compile_async;
use grunkalunka_runtime::{AsyncInstance, AsyncState, PreparedAsyncModule, Value};
use std::{
    io::Write,
    process::{Command, Stdio},
};

const ORIGINAL: &str = "export async function run(a,b) { return a + await Promise.resolve(b); }";

#[path = "async-suspension/pending.rs"]
mod pending;

#[path = "async-suspension/pending_inputs.rs"]
mod pending_inputs;

#[path = "async-suspension/adoption.rs"]
mod adoption;

#[path = "async-suspension/self_resolution.rs"]
mod self_resolution;

#[path = "async-suspension/relational.rs"]
mod relational;

#[path = "async-suspension/promise_relational.rs"]
mod promise_relational;

#[path = "async-suspension/promise_equality.rs"]
mod promise_equality;

#[path = "async-suspension/settled_errors.rs"]
mod settled_errors;

#[path = "async-suspension/source_identities.rs"]
mod source_identities;

#[path = "async-suspension/mixed_returns.rs"]
mod mixed_returns;

#[path = "async-suspension/logical_returns.rs"]
mod logical_returns;

#[path = "async-suspension/value_joins.rs"]
mod value_joins;

#[path = "async-suspension/sync_async_starts.rs"]
mod sync_async_starts;

#[path = "async-suspension/selection_equality.rs"]
mod selection_equality;

#[path = "async-suspension/const_values.rs"]
mod const_values;

#[path = "async-suspension/mutable_values.rs"]
mod mutable_values;

#[path = "async-suspension/mixed_inputs.rs"]
mod mixed_inputs;

#[path = "async-suspension/private_results.rs"]
mod private_results;

#[path = "async-suspension/finally.rs"]
mod finally;

#[path = "async-suspension/number_coercion.rs"]
mod number_coercion;

#[path = "async-suspension/named_async.rs"]
mod named_async;

#[path = "async-suspension/promise_type_arguments.rs"]
mod promise_type_arguments;

#[path = "async-suspension/math_type_arguments.rs"]
mod math_type_arguments;

#[path = "async-suspension/runtime_call_type_arguments.rs"]
mod runtime_call_type_arguments;

#[path = "async-suspension/factory_type_arguments.rs"]
mod factory_type_arguments;

#[test]
fn selected_async_await_operands_keep_the_selected_reactions() {
    compare_promises(
        &[
            "let f=async()=>{await a;return b;};return await(a?f():b);",
            "let f=async()=>{await a;return b;};return await(a?null:f());",
            "let f=async()=>{await a;return false;};return await(a?f():void 0);",
            "let f=async()=>{await a;return void 0;};return await(a?f():-0);",
            "let f=async()=>{await a;return b;};return await(a?f():Promise.resolve(b));",
            "let p=Promise.resolve(b);let f=async()=>{await a;return b;};return await(a?p:f());",
            "let f=async()=>{await a;return b;};let g=async()=>{await b;await a;return a;};return await(a?f():g());",
            "let f=async()=>{await a;return b;};const g=async()=>a;return await(a?f():g());",
            "let f=async()=>{await a;return b;};return await Promise.resolve(a?f():b);",
            "let f=async x=>{await a;return x;};return await(a?f(null):false);",
            "let f=async x=>{await a;return x;};return await(a?false:f(void 0));",
            "return await(a?Promise.resolve(a):b);",
            "const p=Promise.resolve(b);return await(a?null:p);",
            "let f=async()=>{throw b;};try{return await(a?f():false);}catch(e){return e;}",
            "try{return await(a?Promise.reject(null):false);}catch(e){return e;}",
            "try{return await(a?Promise.reject(a):Promise.reject(b));}catch(e){return e;}",
            "return await(a?Promise.resolve(a):(b?false:null));",
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
fn selected_async_await_operands_preserve_prefixes_and_snapshots() {
    compare_promises(
        &[
            "let x=a;let f=async()=>{await a;return x;};return await((x+=b)?(x=b,f()):(x=a,x));",
            "let x=a;let f=async()=>{await a;return x;};return await(x=b,(a?(x+=a,f()):x));",
            "let f=async()=>{await a;return b;};return await((await a)?f():b);",
            "let f=async()=>{await a;return b;};return await(a?(b?f():a):(a?b:f()));",
            "let f=async x=>{await a;return x+a;};const g=async x=>{await b;return x-b;};return await(a?f((f=g,b)):a);",
            "let x=a;let f=async(x,y)=>{await a;return x-y;};return await(a?f(x,x=b):x);",
            "let f=async x=>{await a;return x;};return await(a?f(b,await a):b);",
            "const make=x=>async y=>{await y;return x+y;};let f=make(a);return await(a?f(b):a);",
            "let f=async x=>{await a;return x;};const apply=async(fn,x)=>await(a?fn(x):b);return await(a?apply:apply)(f,a);",
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
fn selected_async_await_operands_keep_rejection_boundaries() {
    compare_promises(
        &[
            "let f=async()=>{await a;throw b;};return await(a?f():b);",
            "let f=async()=>{await a;throw b;};try{return await(a?f():b);}catch(e){return e;}",
            "let f=async()=>{return await Promise.reject(null);};try{return await(a?false:f());}catch(e){return e;}",
            "let f=async()=>{await a;return b;};return await(a?f():Promise.reject(b));",
            "let f=async()=>{await a;return b;};try{return await(a?f():Promise.reject(false));}catch(e){return e;}",
            "let f=async()=>{await a;return b;};const g=async()=>{throw null;};return await(a?g():f());",
            "let f=async()=>{await a;return b;};let g=async()=>{throw void 0;};try{return await(a?g():f());}catch(e){return e;}",
            "let f=async()=>{await a;return b;};try{return await(await Promise.reject(a),a?f():b);}catch(e){return e;}",
            "let f=async()=>{await a;throw b;};let g=async()=>{try{return await(a?f():a);}catch(e){await b;return e;}};return await g();",
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
fn selected_async_await_operands_keep_limits_and_erased_types() {
    let prefix = "let f=async()=>{await a;return b;};let x=0;let k=0;";
    let site = "x+=+(await(++k===1&&a?f():b));";
    let boundary = format!("{prefix}{}await a;return x;", site.repeat(30));
    compare_promises(&[&boundary], &[(4.0, 2.0), (0.0, -0.0)]);
    let over = format!("{prefix}{}await a;await b;return x;", site.repeat(30));
    for ts in [false, true] {
        assert!(
            compile_async(&source_for(&over), ts)
                .err()
                .expect("33 sites reject")
                .contains("32 await sites")
        );
    }
    let mut nested = "f()".to_string();
    for _ in 0..120 {
        nested = format!("a?({nested}):b");
    }
    let deep = format!("let f=async()=>{{await a;return b;}};return await({nested});");
    // Parentheses also consume the existing source depth budget.
    for ts in [false, true] {
        assert!(compile_async(&source_for(&deep), ts).is_err());
    }
    let mut nested = "f()".to_string();
    for _ in 0..48 {
        nested = format!("a?({nested}):b");
    }
    let deep = format!("let f=async()=>{{await a;return b;}};return await({nested});");
    compare_promises(&[&deep], &[(4.0, 2.0), (0.0, -0.0)]);
    let plain = "export async function run(a,b){let f=async x=>{await a;return x;};return await(a?f(b):null);}";
    let typed = "export async function run(a:never,b:Missing):Wrong{let f=async(x:unknown):Promise<string>=>{await(a as string);return x as never;};return await((a as any)?(f as never)(b as Wrong):null);}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    compare_async_sources(
        &[plain.to_string()],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn selected_async_await_operands_keep_pending_and_domain_guards() {
    compare_promises(
        &[
            "let f=async()=>{await a;return b;};const p=a?f():b;return await p;",
            "let f=async()=>{await a;return b;};return await(f()||b);",
            "let f=async()=>{await a;return b;};return await(f()&&b);",
            "let f=async()=>{await a;return b;};return await(f()??b);",
            "let f=async()=>{await a;return b;};return await(a?(f()||b):b);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
    for body in [
        "let f=async()=>{await a;return b;};return await Promise.reject(a?f():b);",
        "let f=async()=>{await a;return b;};return await Promise.resolve(a?f():b,await a);",
        "let f=async()=>{await a;return b;};return await(a?f():{});",
        "let f=async()=>{await a;return b;};return await(a?f():'x');",
        "let f=async()=>{await a;return b;};return await(a?f():(()=>b));",
        "let f=async()=>{await a;return b;};return await(a?f():missing());",
        "let f=async()=>{await a;return b;};return await(a?f():Promise.resolve({}));",
    ] {
        for ts in [false, true] {
            assert!(compile_async(&source_for(body), ts).is_err(), "{body}");
        }
    }
}

#[test]
fn suspending_async_closures_preserve_body_and_caller_reactions() {
    compare_promises(
        &[
            "const f=a?async()=>{await a;return b;}:async()=>b;return await f();",
            "const f=a?async()=>{await a;throw b;}:async()=>b;return await f();",
            "const f=a?async()=>{await a;return b;}:async()=>{await b;return a;};return await f();",
            "const f=a?async()=>{await a;await b;return null;}:async()=>{await b;return false;};return await f();",
            "const f=a?async()=>{await a;}:async()=>{await b;return void 0;};return await f();",
            "const f=a?async()=>{await a;return -0;}:async()=>{await b;return 0/0;};return await f();",
            "let f=async()=>{await a;return b;};return await Promise.resolve(f());",
            "let f=async()=>{await a;return b;};return await(0,f());",
            "let f=async()=>{await a;return b;};return await(f=async()=>{await b;return a;},f());",
            "let x=a;let f=async()=>{await a;return x;};return await(x=b,(x+=a,f()));",
            "let x=a;let f=async()=>{await a;return x;};return await(x=await b,f());",
            "let f=async()=>{await a;return b;};try{return await(await Promise.reject(a),f());}catch(e){return e;}",
            "let f=async()=>{await a;return b;};return await(f||(async()=>a))();",
            "let f=async()=>{await a;return b;};return await(f??(async()=>a))();",
            "let f=async()=>{await a;return b;};f&&=async()=>a;return await f();",
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
fn suspending_async_closures_keep_argument_and_capture_snapshots() {
    compare_promises(
        &[
            "let x=a;const f=a?async y=>{x=y;await b;return x;}:async y=>{await a;return x+y;};await a;x=b;return await f(a);",
            "let f=async x=>{await a;return x+a;};const g=async x=>{await b;return x-b;};return await f((f=g,b));",
            "const f=a?async(x,y)=>{await a;return x-y;}:async(x,y)=>{await b;return y-x;};let x=a;return await f(x,x=b);",
            "const f=a?async x=>{await a;return x;}:async x=>{await b;return x;};return await f(b,await a);",
            "const f=a?async x=>{const before=x;await b;x=b;return before-x;}:async x=>{await a;return x;};return await f(a);",
            "const f=a?async x=>{const g=()=>++x;await a;g();await b;return g();}:async x=>{await b;return x;};return await f(a);",
            "const f=a?async x=>{await b;return x;}:async x=>{await a;return x;};return await f(null);",
            "const f=a?async x=>{await b;return x;}:async x=>{await a;return x;};return await f(false);",
            "const f=a?async x=>{await b;return +x;}:async x=>{await a;return +x;};return await f(void 0);",
            "const p=Promise.resolve(a);const f=a?async p=>{await b;return await p;}:async p=>{return await p;};return await f(p);",
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
fn suspending_async_closures_keep_nested_frames_factories_and_callbacks() {
    compare_promises(
        &[
            "const make=x=>async y=>{await y;return x+y;};const f=make(a);return await(a?f:f)(b);",
            "const make=x=>{return async y=>{await y;return x+y;};};const f=make(a);return await(a?f:f)(b);",
            "const g=a?async x=>{await x;return x+b;}:async x=>{await b;return x-b;};const f=async x=>{const y=await g(x);return y+(await b);};return await(a?f:f)(a);",
            "const apply=async(fn,x)=>await fn(x);const f=a?async x=>{await b;return x;}:async x=>-x;return await apply(f,a);",
            "const apply=a?async(fn,x)=>await fn(x):async(fn,x)=>await fn(-x);const f=async x=>{await b;return x;};return await apply(f,a);",
            "const invoke=async(fn,x)=>await fn(x);const relay=async(fn,x)=>await invoke(fn,x);return await(a?relay:relay)(async x=>{await a;return x;},b);",
            "const make=fn=>async x=>await fn(x);const f=make(async x=>{await a;return x+b;});return await(a?f:f)(a);",
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
fn suspending_async_closures_keep_lexical_rejection_boundaries() {
    compare_promises(
        &[
            "const f=a?async()=>{await a;throw b;}:async()=>b;try{return await f();}catch(e){return e;}",
            "let f=async()=>{return await Promise.reject(b);};return await f();",
            "let f=async()=>{return await Promise.reject(b);};try{return await f();}catch(e){return e;}",
            "let f=async()=>{try{return await Promise.reject(null);}catch(e){await b;return e;}};return await f();",
            "let f=async()=>{await b;try{throw false;}catch(e){return e;}};return await f();",
            "let g=async()=>{await a;throw b;};let f=async()=>{return await g();};try{return await f();}catch(e){return e;}",
            "let g=async()=>{await a;throw void 0;};let f=async()=>{try{return await g();}catch(e){await b;throw e;}};return await f();",
            "let f=async x=>{let i=0;while(i<3){i++;try{await Promise.reject(x);}catch(e){x=+e+b;}}return x;};return await f(a);",
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
fn suspending_async_closures_preserve_source_limits_and_erased_types() {
    let parameters = (0..1000)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let arguments = (0..1000)
        .map(|i| if i == 999 { "b" } else { "a" })
        .collect::<Vec<_>>()
        .join(",");
    let wide = format!(
        "const f=a?async({parameters})=>{{await a;return p999;}}:async({parameters})=>{{await b;return p0;}};return await f({arguments});"
    );
    // The 1,000-argument boundary measures admission/transport, not whether
    // that much work fits the ordinary 10,000-fuel invocation budget.
    use wasmtime::{Config, Engine, Instance, Module, Store};
    for ts in [false, true] {
        let source = source_for(&wide);
        let artifact = compile_async(&source, ts).unwrap();
        let mut config = Config::new();
        config.consume_fuel(true);
        let engine = Engine::new(&config).unwrap();
        let module = Module::new(&engine, &artifact.wasm).unwrap();
        for (a, b) in [(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)] {
            let mut store = Store::new(&engine, ());
            store.set_fuel(10_000_000).unwrap();
            let instance = Instance::new(&mut store, &module, &[]).unwrap();
            let start = instance
                .get_typed_func::<(f64, f64), (i32, i32, f64)>(&mut store, "start")
                .unwrap();
            let resume = instance
                .get_typed_func::<(i32, f64), (i32, i32, f64)>(&mut store, "resume")
                .unwrap();
            let child = start.call(&mut store, (a, b)).unwrap();
            assert_eq!(child.0, 1);
            let caller = resume.call(&mut store, (child.1, child.2)).unwrap();
            assert_eq!(caller.0, 1);
            let done = resume.call(&mut store, (caller.1, caller.2)).unwrap();
            assert_eq!((done.0, done.1), (0, 3));
            let expected = if a != 0.0 && !a.is_nan() { b } else { a };
            assert!(done.2.is_nan() && expected.is_nan() || done.2.to_bits() == expected.to_bits());
        }
        run_node(&format!(
            r#"
import assert from 'node:assert/strict';import {{pathToFileURL}} from 'node:url';
const {{createAsyncFunction}}=await import(pathToFileURL(process.argv[2]));
const native=(()=>{{{};return run;}})(),wasm=createAsyncFunction(new Uint8Array({:?}));
async function observe(run,a,b){{const trace=['returned'];const done=run(a,b).then(v=>trace.push(['fulfilled',v]));let n=0;const markers=new Promise(resolve=>{{function tick(){{trace.push(++n);if(n<34)queueMicrotask(tick);else resolve();}}queueMicrotask(tick);}});await Promise.all([done,markers]);return trace;}}
for(const [a,b] of [[4,2],[0,-0],[NaN,Infinity]])assert.deepEqual(await observe(wasm,a,b),await observe(native,a,b));
"#,
            source.replacen("export async function run", "async function run", 1),
            artifact.wasm
        ));
    }
    let too_wide = wide.replace(&parameters, &format!("{parameters},extra"));
    for ts in [false, true] {
        assert!(compile_async(&source_for(&too_wide), ts).is_err());
    }
    let boundary = format!(
        "const f=a?async()=>{{{}return b;}}:async()=>b;return await f();",
        "await a;".repeat(31)
    );
    compare_promises(&[&boundary], &[(4.0, 2.0), (0.0, -0.0)]);
    let over = boundary.replacen("return b;", "await a;return b;", 1);
    for ts in [false, true] {
        assert!(
            compile_async(&source_for(&over), ts)
                .err()
                .expect("33 logical awaits reject")
                .contains("32 await sites")
        );
    }
    let plain = "export async function run(a,b){const f=a?async x=>{await b;return x;}:async x=>{await a;return null;};return await f(b);}";
    let typed = "export async function run(a:never,b:Missing):Wrong{const f=(a?async(x:unknown):Promise<string>=>{await(b as string);return x as never;}:async(x:unknown):Promise<number>=>{await(a as any);return null;}) as any;return await(f as never)(b as unknown);}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    compare_async_sources(
        &[plain.to_string()],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn suspending_async_closures_keep_pending_and_signature_guards() {
    for body in [
        "let f=async()=>{await a;return b;};return await Promise.reject(f());",
        "const f=a?async()=>{await a;return b;}:()=>b;return await f();",
        "const f=a?async x=>{await a;return x;}:async(x,y)=>{await b;return x+y;};return await f(a,b);",
        "const f=a?async x=>{await a;return x;}:async x=>{await b;return x;};return await f();",
        "let f=async x=>{await a;return x;};return await f?.(b);",
        "let f=async x=>{await a;return x;};return await f(...a);",
        "let f=async()=>{await a;throw Promise.resolve(b);};return await f();",
        "let f=async()=>{await a;return 'x';};return await f();",
        "let f=async()=>{await a;return {};};return await f();",
        "let f=async()=>{await a;return ()=>b;};return await f();",
    ] {
        for ts in [false, true] {
            assert!(compile_async(&source_for(body), ts).is_err(), "{body}");
        }
    }
}

#[test]
fn throwing_async_closures_keep_settlement_and_caller_reactions() {
    compare_promises(
        &[
            "const f=a?async()=>{throw a;}:async()=>b;return await f();",
            "const f=a?async()=>{throw b;}:async()=>{throw a;};return await f();",
            "const f=a?async()=>{throw undefined;}:async()=>{throw null;};return await f();",
            "const f=a?async()=>{throw true;}:async()=>{throw false;};return await f();",
            "const f=a?async()=>{if(a)throw b;}:async()=>false;return await f();",
            "const f=a?async()=>{try{throw b;}catch(e){return e;}}:async()=>null;return await f();",
            "const f=a?async()=>{try{throw a;}catch(e){throw b;}}:async()=>b;try{return await f();}catch(e){return e;}",
            "let n=0;const f=a?async()=>{n++;throw b;}:async()=>{n+=2;return false;};try{return +(await f())+n;}catch(e){return +e+n;}",
            "let f=async()=>a;f=async()=>{throw b;};try{return await f();}catch(e){return e;}",
            "let f=async()=>{throw a;};const old=f;f=async()=>b;try{return await old();}catch(e){return +e+(+(await f()));}",
            "let x=a;let f=async y=>{x++;throw y;};try{return await f((f=async y=>y,await b));}catch(e){return +e+x;}",
            "let n=0;const f=a?async x=>{n++;throw x;}:async x=>x;try{return await f(await Promise.reject(b));}catch(e){return +e+n;}",
            "let n=0;const f=a?async x=>{n++;throw x;}:async x=>x;try{return +(await f(b,n+=4))+n;}catch(e){return +e+n;}",
            "const f=a?async()=>{let x=3;while(x){if(--x===1)throw b;}return x;}:async()=>a;return await f();",
            "const make=x=>async()=>{throw x;};const f=make(b);return await f();",
            "const make=a?x=>async()=>{throw x;}:x=>async()=>-x;const f=make(b);try{return await f();}catch(e){return e;}",
            "let x=a;const make=()=>{return async()=>{throw x;};};const f=make();await b;x=b;return await f();",
            "const f=a?async()=>{const g=()=>b;throw g();}:async()=>a;try{return await f();}catch(e){try{throw e;}catch(x){return x;}}",
            "const f=a?async()=>{throw b;}:()=>Promise.resolve(a);return await f();",
            "let f=async()=>{throw a;};f=()=>{return Promise.resolve(b);};return await f();",
            "let f=async()=>{throw a;};return await(a?f():Promise.resolve(b));",
            "let f=async()=>{throw a;};try{return await(a?Promise.resolve(b):f());}catch(e){return e;}",
            "const f=async x=>{throw x;};const h=()=>f(null);return await(a?h:h)();",
            "const f=async x=>{throw x;};const h=()=>f(false);try{return await(a?h:h)();}catch(e){return e;}",
            "let f=async()=>{throw b;};return await Promise.resolve(f());",
            "let f=async()=>{throw b;};return await(0,f());",
            "let f=async()=>{throw b;};return await(f||(async()=>a))();",
            "let f=async()=>{throw b;};return await(f??(async()=>a))();",
            "let f=async()=>{throw a;};f&&=async()=>b;return await f();",
            "const f=async()=>{const fail=async()=>{throw a;};return b;};const h=a?f:f;return await h();",
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
fn throwing_async_closures_preserve_source_budgets_and_erased_types() {
    let parameters = (0..1000)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let arguments = (0..1000)
        .map(|i| if i == 999 { "b" } else { "a" })
        .collect::<Vec<_>>()
        .join(",");
    let wide = format!(
        "const f=a?async({parameters})=>{{throw p999;}}:async({parameters})=>p0;return await f({arguments});"
    );
    compare_promises(
        &[&wide],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
    let too_wide = wide.replace(&parameters, &format!("{parameters},extra"));
    for ts in [false, true] {
        assert!(compile_async(&source_for(&too_wide), ts).is_err());
    }
    let prefix = "let f=async x=>{if(a)throw x;return x;};let n=0;";
    let site = "try{n+=+(await f(b));}catch(e){n+=+e;}";
    let repeated = format!("{prefix}{}return n;", site.repeat(32));
    compare_promises(
        &[&repeated],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
    for ts in [false, true] {
        let over = source_for(&format!("{prefix}{}return n;", site.repeat(33)));
        assert!(
            compile_async(&over, ts)
                .err()
                .expect("33 logical awaits must reject")
                .contains("32 await sites")
        );
    }
    let plain = "export async function run(a,b){const f=a?async x=>{throw x;}:async x=>null;try{return await f(b);}catch(e){return e;}}";
    let typed = "export async function run(a:never,b:Missing):Wrong{const f=(a?async(x:unknown):Promise<string>=>{throw(x as number);}:async(x:unknown):Promise<number>=>null) as any;try{return await(f as never)(b as unknown);}catch(e:unknown){return e as string;}}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    compare_async_sources(
        &[plain.to_string()],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn throwing_async_closures_forward_through_private_callbacks() {
    compare_promises(
        &[
            "const apply=(fn,x)=>fn(x);return await apply(async x=>{throw x;},b);",
            "const apply=a?(fn,x)=>fn(x):(fn,x)=>fn(-x);return await apply(async x=>{throw x;},b);",
            "const apply=(fn,x)=>fn(x);const relay=(fn,x)=>apply(fn,x);return await relay(async x=>{throw x;},null);",
            "const f=a?async x=>{throw x;}:async x=>false;const apply=(fn,x)=>fn(x);try{return await apply(f,b);}catch(e){return e;}",
            "const apply=(fn,x)=>{return fn(x);};return await apply(async x=>{throw x;},false);",
            "const invoke=fn=>fn(async x=>{throw x;},b);const apply=(fn,x)=>fn(x);return await invoke(apply);",
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
fn throwing_async_closures_keep_unobserved_rejections_guarded() {
    for body in [
        "let f=async()=>{throw a;};return await Promise.reject(f());",
        "const f=a?async()=>{throw 'x';}:async()=>b;return await f();",
        "const f=a?async()=>{throw {};}:async()=>b;return await f();",
        "const f=a?async()=>{throw ()=>a;}:async()=>b;return await f();",
        "const f=a?async()=>{throw Promise.resolve(a);}:async()=>b;return await f();",
        "let f=async()=>{throw a;};return await f?.();",
        "let f=async()=>{throw a;};return await f(...a);",
    ] {
        for ts in [false, true] {
            assert!(compile_async(&source_for(body), ts).is_err(), "{body}");
        }
    }
}

#[test]
fn eager_async_closures_preserve_values_and_jobs() {
    compare_promises(
        &[
            "const f=a?async x=>x:async x=>-x;return await f(b);",
            "const f=a?async()=>a:async()=>b;return await f();",
            "let f=async x=>x;const p=f(b);await a;return await p;",
            "const f=async x=>x;const p=f(b);return await p;",
            "const f=a?async()=>undefined:async()=>null;return await f();",
            "const f=a?async()=>true:async()=>false;return await f();",
            "const f=a?async x=>{if(x)return null;return false;}:async x=>{x++;};return await f(b);",
            "let x=a;const f=a?async()=>x++:async()=>++x;const p=f();x=b;return (+(await p))+x;",
            "let f=async x=>x;return await f((f=async x=>-x,await b));",
            "let f=async x=>x;const old=f;f=async x=>-x;return (+(await old(a)))+(+(await f(b)));",
            "const apply=(fn,x)=>fn(x);return await apply(async x=>x,b);",
            "const apply=a?(fn,x)=>fn(x):(fn,x)=>fn(-x);return await apply(async x=>x,b);",
            "const apply=(fn,x)=>fn(x);const relay=(fn,x)=>apply(fn,x);return await relay(async x=>x,null);",
            "const make=x=>async()=>x;const f=make(a);await b;return await f();",
            "const make=a?x=>async()=>x:x=>async()=>-x;const f=make(b);return await f();",
            "const make=()=>{return async()=>a;};const f=make();return await f();",
            "const make=async x=>{const id=y=>y;return id(x);};const p=make(false);return await p;",
            "const f=a?async x=>x:x=>Promise.resolve(x);return await f(b);",
            "let n=0;const f=a?async x=>(n++,x):async x=>(n+=2,x);const p=f(b,n+=4);return (+(await p))+n;",
            "let n=0;const f=a?async x=>(n++,x):async x=>(n+=2,x);try{return await f(await Promise.reject(b));}catch(e){return +e+n;}",
            "const f=a?async x=>x:async x=>x;const take=a?p=>p:p=>p;return await take(f(false));",
            "const f=async x=>x;const take=p=>p;return await take(f(null));",
            "const f=async()=>a;f();return a;",
            "let f=async()=>a;return await f();",
            "const f=async()=>a;return void f();",
            "const f=async()=>a;return void f;",
            "const f=async()=>a;return await Promise.resolve(f(),b);",
            "let n=0;const f=async()=>{n++;return a;};const out=await Promise.resolve(f(),n+=2);return out+n;",
            "const f=async()=>a;return await Promise.resolve(f(),await b);",
            "const f=async()=>a;const relay=a?()=>f():()=>f();return await relay();",
            "const f=async()=>false;const apply=(fn,x)=>fn(x);const relay=x=>f();return await apply(relay,b);",
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
fn eager_async_closures_keep_completion_and_domain_guards() {
    for body in [
        "const f=a?async()=>()=>a:async()=>()=>b;return await f();",
        "const f=a?async()=>({}):async()=>({});return await f();",
        "const f=a?async x=>x:async x=>x;return await f();",
        "const f=a?async x=>x:async x=>x;return await f(...a);",
        "const f=a?async x=>x:async x=>x;return await f?.(b);",
        "const f=a?async(x=a)=>x:async(x=a)=>x;return await f(b);",
        "let f=async x=>x;f=x=>x;return await f(b);",
    ] {
        for ts in [false, true] {
            assert!(compile_async(&source_for(body), ts).is_err(), "{body}");
        }
    }
}

#[test]
fn known_local_primitive_inputs_preserve_async_values_and_jobs() {
    compare_promises(
        &[
            "const f=x=>+x;return f(await null);",
            "const f=x=>+x;return f(await true);",
            "const f=async x=>+x;return await f(null);",
            "const f=async x=>{await b;return +x;};return await f(true);",
            "let x=null;const f=async(x,y)=>+x+(+y);return (await f(x,x=a))+x;",
            "const f=x=>+x;try{return f(await Promise.reject(null));}catch(e){return e;}",
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
fn known_factory_primitive_inputs_preserve_cells_and_jobs() {
    compare_promises(
        &[
            "const make=x=>()=>+x;const f=make(await null);await b;return f();",
            "const make=x=>()=>+x;const f=make(await true);return await f();",
            "const make=x=>{return ()=>+x;};const f=make(await null);await b;return f();",
            "const make=x=>{if(a)return ()=>+x;return ()=>-x;};const f=make(await true);return await f();",
            "let x=null;const make=(x,y)=>()=>+x+(+y);const f=make(x,x=await a);await b;return f()+x;",
            "const make=x=>()=>+x;try{const f=make(await Promise.reject(null));return f();}catch(e){return e;}",
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
fn known_local_input_result_facts_preserve_async_kinds_and_jobs() {
    compare_promises(
        &[
            "const f=x=>x;return f(await null);",
            "const f=x=>x;return f(await true);",
            "const f=x=>x;const relay=y=>f(y);return relay(await false);",
            "const f=async x=>x;return await f(null);",
            "const f=async x=>{await b;return x;};return await f(undefined);",
            "const make=x=>()=>x;const f=make(await null);await b;return f();",
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
fn runtime_primitive_inputs_keep_callee_snapshots_and_async_jobs() {
    compare_promises(
        &[
            "const f=a?x=>+x:x=>-x;return f(await null);",
            "const f=a?x=>x:x=>x;return f(await true);",
            "let x=null;const f=a?(x,y)=>+x+y:(x,y)=>-x-y;return f(x,(x=await a,a))+(+x);",
            "let f=x=>+x;return 1/f((f=x=>-x,await null));",
            "let make=x=>()=>x;const f=make(await null);await b;return f();",
            "let n=0;let f=x=>(n++,+x);f(null);try{return f(await Promise.reject(null));}catch(e){return +e+n;}",
            "let f=x=>+x;try{throw a?null:false;}catch(e){return f(e);}",
            "let f=x=>x;try{throw a?null:false;}catch(e){return f(e);}",
            "let f=x=>x;try{throw a;}catch(e){return f(e);}",
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
fn callback_primitive_inputs_preserve_async_jobs() {
    compare_promises(
        &[
            "const apply=(fn,x)=>fn(x);return apply(y=>+y,await null);",
            "const apply=(fn,x)=>fn(x);return apply(y=>y,await false);",
            "const make=fn=>x=>fn(x);const f=make(y=>y);await b;return f(null);",
            "let n=0;const apply=(fn,x)=>fn(x);const f=y=>(n++,+y);apply(f,null);try{return apply(f,await Promise.reject(null));}catch(e){return n+(+e);}",
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
fn computed_call_inputs_preserve_async_values_and_jobs() {
    compare_promises(
        &[
            "const make=x=>()=>null;const id=x=>x;await b;return id(make(a)());",
            "const make=x=>()=>false;const id=x=>x;return id(await make(a)());",
            "const make=x=>()=>{};const id=x=>x;return id(await make(a)());",
            "let make=x=>()=>x;let id=x=>x;return id(await make(null)());",
            "const make=x=>()=>null;const apply=(f,x)=>f(x);return apply(y=>y,await make(a)());",
            "const make=x=>()=>x;const id=x=>x;return id(await make(a)());",
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
fn awaited_intrinsic_inputs_do_not_need_a_prior_successful_call() {
    compare_promises(
        &[
            "let n=0;const apply=(fn,x)=>fn(x);try{return apply(y=>(n++,+y),await Promise.reject(null));}catch(e){return n+(+e);}",
            "const apply=(fn,x)=>fn(x);return apply(y=>y,await Promise.resolve(null));",
            "const apply=(fn,x)=>fn(x);return apply(y=>y,await Promise.resolve(false));",
            "const apply=(fn,x)=>fn(x);return apply(y=>y,await Promise.resolve(undefined));",
            "const apply=(fn,x)=>fn(x);return apply(y=>y,await Promise.resolve(a));",
            "const apply=(fn,x)=>fn(x);try{return apply(y=>y,await Promise.reject(null));}catch(e){return e;}",
            "const apply=(fn,x)=>fn(x);try{return apply(y=>y,await Promise.reject(false));}catch(e){return e;}",
            "const apply=(fn,x)=>fn(x);try{return apply(y=>y,await Promise.reject(undefined));}catch(e){return e;}",
            "const apply=(fn,x)=>fn(x);try{return apply(y=>y,await Promise.reject(a));}catch(e){return e;}",
            "let n=0;const apply=(fn,x)=>fn(x);try{return apply((n++,y=>+y),await Promise.reject(null));}catch(e){return n+(+e);}",
            "const apply=(fn,x)=>fn(x);const relay=(f,x)=>apply(f,x);return relay(y=>y,await Promise.resolve(true));",
            "let f=x=>x;return f(await Promise.resolve(null));",
            "let make=x=>()=>x;const f=make(await Promise.resolve(false));await b;return f();",
            "const f=async x=>{await b;return x;};return await f(await Promise.resolve(null));",
            "let f=x=>+x;return 1/f(await Promise.resolve(null));",
            "let f=x=>x;return f(await (Promise.resolve((null))));",
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
fn awaited_input_facts_keep_intrinsic_and_promise_object_guards() {
    for body in [
        "const Promise=a;const f=x=>x;return f(await Promise.resolve(null));",
        "const f=x=>x;return f(await Promise.resolve?.(null));",
        "const f=x=>x;return f(await Promise.resolve(...a));",
        "const f=x=>x;return f(await Promise.other(null));",
    ] {
        let source = source_for(body);
        for typescript in [false, true] {
            assert!(compile_async(&source, typescript).is_err(), "{source}");
        }
    }
}

#[test]
fn joined_private_inputs_preserve_async_results_and_snapshots() {
    compare_promises(
        &[
            "const f=x=>+x;let p=f;p(await null);const g=x=>-x;let h=a?f:g;return h(await b);",
            "const f=x=>x;let p=f;p(null);const g=x=>x;let h=a?f:g;return await h(b);",
            "const f=x=>()=>x;let p=f;const keep=p(null);const g=x=>()=>x;let make=a?f:g;const h=make(await b);await a;return h();",
            "const f=x=>+x;let p=f;try{throw null;}catch(e){p(e);}const g=x=>-x;let h=a?f:g;return h(await b);",
            "const f=x=>+x;let p=f;p(null);const g=x=>-x;let h=a?f:g;return 1/h((h=g,await null));",
            "const f=(fn,x)=>fn(x);let p=f;p(y=>+y,null);const g=(fn,x)=>fn(x)-b;let h=a?f:g;return h(y=>+y,await b);",
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
fn stored_fulfilled_promises_snapshot_primitive_values() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);return await p;",
            "const p=Promise.resolve(null);return await p;",
            "const p=Promise.resolve(false);return await p;",
            "const p=Promise.resolve(undefined);return await p;",
            "const p=Promise.resolve(a);a=b;return await p;",
            "const p=Promise.resolve(b);const q=p;const r=Promise.resolve(q);await a;return await r;",
            "const p=Promise.resolve(a);const first=await p;await b;return first+await p;",
            "let n=0;const p=Promise.resolve((n++,a));await p;await p;return n;",
            "const p=Promise.resolve(a?null:b);return (await p)?a:b;",
            "const p=Promise.resolve(a?undefined:b);try{await Promise.reject(await p);}catch(x){await a;return x;}return b;",
            "const p=Promise.resolve(b);const f=async()=>await p;return await f();",
            "const p=Promise.resolve(await b);return await p;",
            "const p=Promise.resolve(a?false:b);return +(await p);",
            "const p=Promise.resolve(b);{const Promise=a;return await p;}",
            "const p=Promise.resolve(a);return await Promise.resolve(p);",
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
fn known_closures_receive_fulfilled_promise_arguments() {
    compare_promises(
        &[
            "const id=p=>p;return await id(Promise.resolve(a));",
            "const id=p=>{return p;};return await id(Promise.resolve(false));",
            "const read=async p=>await p;return await read(Promise.resolve(null));",
            "const relay=(p,x)=>p;const p=Promise.resolve(a);return await relay(p,await b);",
            "const make=x=>Promise.resolve(x);const p=Promise.resolve(a);return await make(p);",
            "const p=Promise.resolve(a);const id=x=>x;return await id(p);",
            "const id=p=>p;const alias=id;const p=Promise.resolve();return await alias(p);",
            "const get=(p,x)=>{const old=p;p=Promise.resolve(x);return old;};return await get(Promise.resolve(a),b);",
            "const get=p=>{const read=()=>p;p=Promise.resolve(b);return read;};const read=get(Promise.resolve(a));return await read();",
            "let p=Promise.resolve(a);const get=(p,x)=>p;return await get(p,(p=Promise.resolve(b),await b));",
            "const read=async p=>{await b;return await p;};return await read(Promise.resolve(a));",
            "const read=async(p,x)=>{a++;return await p;};try{return await read(Promise.resolve(a),await Promise.reject(b));}catch(e){return +e+a;}",
            "const id=p=>{if(a)return p;return Promise.resolve(p);};return await id(Promise.resolve(null));",
            "const first=p=>p;const second=p=>p;const q=second(Promise.resolve(a));return +(await q)+first(b);",
            "const id=p=>p;const p=Promise.resolve(a);{const p=b;return await id(p);}",
            "const fail=async p=>{throw await p;};try{return await fail(Promise.resolve(b));}catch(e){return e;}",
            "const read=async p=>{const get=()=>p;await b;p=Promise.resolve(a);return await get();};return await read(Promise.resolve(b));",
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
fn runtime_closures_receive_fulfilled_promise_arguments() {
    compare_promises(
        &[
            "let id=p=>p;return await id(Promise.resolve(a));",
            "const id=a?p=>p:p=>Promise.resolve(p);return await id(Promise.resolve(b));",
            "const apply=(fn,p)=>fn(p);return await apply(p=>p,Promise.resolve(a));",
            "const make=a?p=>()=>p:p=>()=>p;return await make(Promise.resolve(b))();",
            "const apply=a?(fn,p)=>fn(p):(fn,p)=>fn(p);return await apply(p=>p,Promise.resolve(b));",
            "const apply=(fn,p)=>{const old=fn(p);p=Promise.resolve(b);return old;};return await apply(x=>x,Promise.resolve(a));",
            "let get=(p,x)=>{const old=p;p=Promise.resolve(x);return old;};return await get(Promise.resolve(a),b);",
            "const make=a?p=>{return ()=>p;}:p=>()=>Promise.resolve(p);return await make(Promise.resolve(null))();",
            "let id=p=>p;return await id((id=p=>Promise.resolve(-b),Promise.resolve(a)));",
            "const apply=(fn,p)=>{const old=fn;fn=x=>Promise.resolve(b);return old(p);};return await apply(x=>x,Promise.resolve(a));",
            "const relay=(fn,p)=>fn(p);const apply=(fn,p)=>relay(fn,p);return await apply(p=>p,Promise.resolve(false));",
            "let f=x=>Promise.resolve(x);return await f(Promise.resolve(a));",
            "const f=a?(p,x,fn,y)=>fn(p):(p,x,fn,y)=>Promise.resolve(fn(p));return await f(Promise.resolve(b),a,p=>p,false);",
            "let p=Promise.resolve(a);const f=a?(x,p,y)=>p:(x,p,y)=>p;return await f(null,p,(p=Promise.resolve(b),await false));",
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
fn source_promise_parameter_frames_keep_wide_arity_and_snapshots() {
    use wasmtime::{Config, Engine, Instance, Module, Store};
    let parameters = (0..1000)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let arguments = (0..1000)
        .map(|i| if i == 999 { "last" } else { "first" })
        .collect::<Vec<_>>()
        .join(",");
    let body = format!(
        "const first=Promise.resolve(a),last=Promise.resolve(b);let f=({parameters})=>a?p0:p999;return await f({arguments});"
    );
    for ts in [false, true] {
        let source = source_for(&body);
        let artifact = compile_async(&source, ts).unwrap();
        let mut config = Config::new();
        config.consume_fuel(true);
        let engine = Engine::new(&config).unwrap();
        let module = Module::new(&engine, &artifact.wasm).unwrap();
        for (a, b) in [(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)] {
            let mut store = Store::new(&engine, ());
            store.set_fuel(10_000_000).unwrap();
            let instance = Instance::new(&mut store, &module, &[]).unwrap();
            let start = instance
                .get_typed_func::<(f64, f64), (i32, i32, f64)>(&mut store, "start")
                .unwrap();
            let resume = instance
                .get_typed_func::<(i32, f64), (i32, i32, f64)>(&mut store, "resume")
                .unwrap();
            let pending = start.call(&mut store, (a, b)).unwrap();
            assert_eq!(pending.0, 1);
            let done = resume.call(&mut store, (pending.1, pending.2)).unwrap();
            assert_eq!((done.0, done.1), (0, 3));
            let expected = if a != 0.0 && !a.is_nan() { a } else { b };
            assert_eq!(done.2.to_bits(), expected.to_bits());
        }
        run_node(&format!(
            "import assert from 'node:assert/strict';import {{pathToFileURL}} from 'node:url';const {{createAsyncFunction}}=await import(pathToFileURL(process.argv[2]));const native=(()=>{{{};return run;}})(),wasm=createAsyncFunction(new Uint8Array({:?}));for(const [a,b] of [[4,2],[0,-0],[NaN,Infinity]])assert.ok(Object.is(await wasm(a,b),await native(a,b)));",
            source.replacen("export async function run", "async function run", 1),
            artifact.wasm
        ));
        let too_wide = source_for(&format!(
            "const first=Promise.resolve(a),last=Promise.resolve(b);let f=({parameters},extra)=>extra;return await f({arguments},last);"
        ));
        assert!(compile_async(&too_wide, ts).is_err());
    }
}

#[test]
fn source_promise_parameters_keep_erasure_domains_and_await_bounds() {
    let writes = format!(
        "const id=p=>{{{}return p;}};return await id(Promise.resolve(b));",
        "p=Promise.resolve(a);".repeat(140)
    );
    let factories = (0..37)
        .map(|i| format!("const f{i}={}a;", "()=>".repeat(7)))
        .collect::<String>();
    let many_arrows = format!("{factories}const id=p=>p;return await id(Promise.resolve(a));");
    compare_promises(&[&writes, &many_arrows], &[(4.0, 2.0)]);
    for (plain, typed) in [
        (
            "export async function run(a,b){let id=p=>p;return await id(Promise.resolve(a));}",
            "export async function run(a:never,b:Missing):Wrong{let id=(p:NotPromise):Wrong=>(p as Bad);return await (id(Promise.resolve(a as any)) as Wrong);}",
        ),
        (
            "export async function run(a,b){const id=p=>p;return await id(a);}",
            "export async function run(a:never,b:Missing):Wrong{const id=(p:Promise<number>):Promise<number>=>p as any;return await id(a as Wrong);}",
        ),
    ] {
        assert_eq!(
            compile_async(plain, false).unwrap().wasm,
            compile_async(typed, true).unwrap().wasm
        );
    }
    for count in [32, 33] {
        let body = format!(
            "let id=p=>p;{}return b;",
            "await id(Promise.resolve(a));".repeat(count)
        );
        for ts in [false, true] {
            assert_eq!(compile_async(&source_for(&body), ts).is_ok(), count == 32);
        }
        if count == 32 {
            compare_promises(&[&body], &[(4.0, 2.0)]);
        }
    }
    for body in [
        "const apply=(fn,p)=>fn(p);return await apply(x=>x,Promise.resolve(a))+apply(x=>x,b);",
        "const id=p=>p;return await id?.(Promise.resolve(a));",
        "const id=p=>p;return await id(...a);",
        "const id=(p,x)=>p;return await id(Promise.resolve(a));",
        "const id=(p=Promise.resolve(a))=>p;return await id(Promise.resolve(a));",
        "const id=(...p)=>p;return await id(Promise.resolve(a));",
        "const id=p=>p;{const Promise=a;return await id(Promise.resolve(b));}",
        "const id=p=>{const old=later;const later=p;return old;};return await id(Promise.resolve(a));",
    ] {
        for ts in [false, true] {
            assert!(compile_async(&source_for(body), ts).is_err(), "{body}");
        }
    }
    let typed = "export async function run(a,b){const id=p=>p;return await id<number>(Promise.resolve(a));}";
    let plain = typed.replace("id<number>(", "id(");
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    compare_async_sources(
        &[plain],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-2.0, -4.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn selected_unused_callback_peers_preserve_promises() {
    compare_promises(
        &[
            "const f=a?(p,x,fn,y)=>fn(p):(p,x,fn,y)=>Promise.resolve(p);return await f(Promise.resolve(b),a,p=>p,false);",
            "const f=a?(fn,p)=>fn(p):(fn,p)=>p;return await f(p=>p,Promise.resolve(null));",
            "const f=a?(p,fn)=>p:(p,fn)=>fn(p);return await f(Promise.resolve(false),p=>p);",
            "let f=(fn,p)=>fn(p);if(a)f=(fn,p)=>p;return await f(p=>p,Promise.resolve());",
            "const make=()=>a?(fn,p)=>fn(p):(fn,p)=>p;const f=make();return await f(p=>p,Promise.resolve(b));",
            "const f=a?(fn,p)=>fn(p):(fn,p)=>p;const relay=(fn,p)=>f(fn,p);return await relay(p=>p,Promise.resolve(b));",
            "let f=(fn,p)=>fn(p);return await f((f=(fn,p)=>p,p=>p),Promise.resolve(await b));",
            "let p=Promise.resolve(a);const f=a?(fn,p,x)=>fn(p):(fn,p,x)=>p;const old=f(p=>p,p,p=Promise.resolve(await b));return (+(await old))+(+(await p));",
            "let n=0;const f=a?(fn,x)=>fn(x):(fn,x)=>x;const out=f((n++,x=>(n+=2,x)),await b);return out+n;",
            "let n=0;const f=a?(fn,x)=>fn(x):(fn,x)=>x;try{return f((n++,x=>(n+=2,x)),await Promise.reject(b));}catch(e){return +e+n;}",
            "const f=a?(fn,x)=>fn(x):(fn,x)=>x;await b;return f(x=>x,null);",
            "const f=a?(fn,x)=>fn(x):(fn,x)=>x;return f(x=>x,await false);",
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
fn selected_closures_return_fulfilled_promise_values() {
    compare_promises(
        &[
            "const make=a?x=>Promise.resolve(x):x=>Promise.resolve(-x);return await make(b);",
            "let make=x=>Promise.resolve(x);return await make(a);",
            "const make=a?()=>Promise.resolve(a):()=>Promise.resolve(b);return await make();",
            "let make=x=>Promise.resolve(x);const old=make;make=x=>Promise.resolve(-x);return +(await old(a))+(+(await make(b)));",
            "let make=x=>Promise.resolve(x);return await make((make=x=>Promise.resolve(-x),await a));",
            "let p=Promise.resolve(a);const get=a?()=>p:()=>p;const old=get();p=Promise.resolve(b);return +(await old)+(+(await get()));",
            "const make=a?()=>Promise.resolve():()=>Promise.resolve(null);return await make();",
            "const make=a?()=>Promise.resolve(false):()=>Promise.resolve(true);return await make();",
            "const make=a?x=>{const p=Promise.resolve(x);return p;}:x=>{if(x)return Promise.resolve(null);return Promise.resolve(false);};return await make(b);",
            "let n=0;const make=a?x=>Promise.resolve((n++,x)):x=>Promise.resolve((n+=2,x));try{return await make(await Promise.reject(b));}catch(e){return +e+n;}",
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
fn higher_order_closures_return_fulfilled_promise_values() {
    compare_promises(
        &[
            "const apply=(fn,x)=>fn(x);return await apply(x=>Promise.resolve(x),a);",
            "const apply=a?(fn,x)=>fn(x):(fn,x)=>fn(-x);return await apply(x=>Promise.resolve(x),b);",
            "const apply=(fn,x)=>{return fn(x);};const relay=(fn,x)=>apply(fn,x);return await relay(x=>{const p=Promise.resolve(x);return p;},false);",
            "const apply=(fn,x)=>fn(x);const p=Promise.resolve(a);return await apply(()=>p,b);",
            "const make=a?()=>x=>Promise.resolve(x):()=>x=>Promise.resolve(-x);return await make()(b);",
            "const make=a?()=>()=>Promise.resolve(false):()=>()=>Promise.resolve(null);const get=make();return await get();",
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
fn known_sync_arrows_return_fulfilled_promise_values() {
    compare_promises(
        &[
            "const make=x=>Promise.resolve(x);const p=make(a);return await p;",
            "const make=x=>Promise.resolve(x);return await make(false);",
            "const make=()=>Promise.resolve();return await make();",
            "const p=Promise.resolve(a);const get=()=>p;return await get();",
            "let p=Promise.resolve(a);const get=()=>p;const old=get();p=Promise.resolve(b);return +(await old)+(+(await get()));",
            "const make=x=>Promise.resolve(x,a++);return await make(a);",
            "const make=x=>Promise.resolve(x);return await make(await b);",
            "let p=Promise.resolve(a);const get=()=>p;return await Promise.resolve(get(),p=Promise.resolve(b));",
            "const make=x=>Promise.resolve(x);const relay=x=>make(x);return await relay(null);",
            "return await (x=>Promise.resolve(x))(a);",
            "const make=x=>Promise.resolve(x);const alias=make;return await alias(a);",
            "let n=0;const make=()=>Promise.resolve(n++);return n;",
            "let n=0;const make=x=>Promise.resolve((n++,x));try{return await make(await Promise.reject(b));}catch(e){return +e+n;}",
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
fn known_block_arrows_return_fulfilled_promise_values() {
    compare_promises(
        &[
            "const make=x=>{return Promise.resolve(x);};const p=make(a);return await p;",
            "const make=x=>{const p=Promise.resolve(x);return p;};return await make(a);",
            "const make=x=>{let p=Promise.resolve(x);p=Promise.resolve(b);return p;};return await make(a);",
            "const make=x=>{if(x)return Promise.resolve(null);return Promise.resolve(false);};return await make(a);",
            "let p=Promise.resolve(a);const get=()=>{return p;};const old=get();p=Promise.resolve(b);return +(await old)+(+(await get()));",
            "const make=x=>{for(let i=0;i<3;i++){if(x)return Promise.resolve(i);}return Promise.resolve(b);};return await make(a);",
            "const make=x=>{const p=Promise.resolve(x);const q=p;return q;};return await make(false);",
            "const make=x=>{let p=Promise.resolve(x);return (p=Promise.resolve(b));};return await make(a);",
            "let p=Promise.resolve(a);const set=x=>{p=Promise.resolve(x);return p;};const old=p;return +(await set(b))+(+(await old));",
            "let n=0;const make=x=>{n++;return Promise.resolve(x);};try{return await make(await Promise.reject(b));}catch(e){return +e+n;}",
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
fn known_promise_results_keep_erasure_interfaces_and_completion_guards() {
    let plain = "export async function run(a,b){const make=x=>{const p=Promise.resolve(x);return p;};const alias=make;return await alias(a);}";
    let typed = "export async function run(a:never,b:Missing):Wrong{const make=(x:NotNumber):NotPromise=>{const p:NotPromise=Promise.resolve(x as Wrong);return p as any;};const alias=make;return await (alias(a as any) as Wrong);}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    for count in [32, 33] {
        let body = format!(
            "const make=()=>{{return Promise.resolve(a);}};{}return b;",
            "await make();".repeat(count)
        );
        for ts in [false, true] {
            assert_eq!(compile_async(&source_for(&body), ts).is_ok(), count == 32);
        }
        if count == 32 {
            compare_promises(&[&body], &[(4.0, 2.0)]);
        }
    }
    for body in [
        "const make=()=>{const p=p;return p;};return a;",
        "const make=x=>Promise.resolve(x);return await make();",
        "const make=x=>Promise.resolve(x);return await make?.(a);",
        "const make=x=>Promise.resolve(x);return await make(...a);",
    ] {
        for ts in [false, true] {
            assert!(compile_async(&source_for(body), ts).is_err(), "{body}");
        }
    }
    for body in [
        "const make=x=>Promise.resolve(x);return await make<number>(a);",
        "const make=x=>{return Promise.resolve(x);};return await make<number>(a);",
    ] {
        let typed = source_for(body);
        let plain = source_for(&body.replace("make<number>(", "make("));
        assert_eq!(
            compile_async(&plain, false).unwrap().wasm,
            compile_async(&typed, true).unwrap().wasm,
            "{typed}"
        );
        compare_async_sources(
            &[plain],
            &[
                (4.0, 2.0),
                (0.0, -0.0),
                (-2.0, -4.0),
                (f64::NAN, f64::INFINITY),
            ],
        );
    }
}

#[test]
fn runtime_promise_results_keep_logical_interfaces_and_bounds() {
    let plain = "export async function run(a,b){let f=x=>{return Promise.resolve(x);};f=x=>{return Promise.resolve(-x);};return await f(a);}";
    let typed = "export async function run(a:never,b:Missing):Wrong{let f=(x:Wrong):NotPromise=>{return Promise.resolve(x as any) as Wrong;};f=(x:Bad):Wrong=>{return Promise.resolve(-(x as any)) as any;};return await (f(a as Bad) as Wrong);}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    for count in [32, 33] {
        let body = format!(
            "let f=()=>Promise.resolve(a);{}return b;",
            "await f();".repeat(count)
        );
        for ts in [false, true] {
            assert_eq!(compile_async(&source_for(&body), ts).is_ok(), count == 32);
        }
        if count == 32 {
            compare_promises(&[&body], &[(4.0, 2.0)]);
        }
    }
    for body in [
        "let f=()=>f();return await f();",
        "const f=a?()=>missing():()=>Promise.resolve(a);return await f();",
    ] {
        for ts in [false, true] {
            assert!(compile_async(&source_for(body), ts).is_err(), "{body}");
        }
    }
    let parameters = (0..1000)
        .map(|i| format!("x{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let arguments = (0..1000).map(|_| "a").collect::<Vec<_>>().join(",");
    compare_promises(
        &[&format!(
            "let f=({parameters})=>Promise.resolve(x999);return await f({arguments});"
        )],
        &[(4.0, 2.0)],
    );
    for ts in [false, true] {
        assert!(compile_async(&source_for(&format!("let f=({parameters},extra)=>Promise.resolve(extra);return await f({arguments},b);")),ts).is_err());
    }
}

#[test]
fn mutable_fulfilled_promises_retain_primitive_values() {
    compare_promises(
        &["let p=Promise.resolve(a);return await p;"],
        &[(4.0, 2.0), (-0.0, -2.0)],
    );
}

#[test]
fn mutable_fulfilled_promise_assignments_keep_snapshots_and_jobs() {
    compare_promises(
        &[
            "let p=Promise.resolve(a);p=Promise.resolve(b);return await p;",
            "let p=Promise.resolve(a);const old=p;p=Promise.resolve(b);return +(await old)+(+(await p));",
            "let p=Promise.resolve(a);const q=Promise.resolve(false);p=q;return await p;",
            "let p=Promise.resolve(a);p=Promise.resolve();return await p;",
            "let p=Promise.resolve(a);return await (p=Promise.resolve(null));",
            "let p=Promise.resolve(a);const old=(p=Promise.resolve(b));p=Promise.resolve(null);return await old;",
            "let p=Promise.resolve(a);p=Promise.resolve(b,a=await b);return +(await p)+a;",
            "let p=Promise.resolve(a);try{p=Promise.resolve(b,await Promise.reject(null));}catch(e){return await p;}return await p;",
            "let p=Promise.resolve(a);const set=()=>{p=Promise.resolve(b);return a;};await a;set();return await p;",
            "let p=Promise.resolve(a);const read=async()=>await p;p=Promise.resolve(b);return await read();",
            "let p=Promise.resolve(a);return await Promise.resolve(p,p=Promise.resolve(b));",
            "let p=Promise.resolve(a);p=Promise.resolve(p,p=Promise.resolve(b));return await p;",
            "let p=Promise.resolve(a);const f=async()=>{p=Promise.resolve(await b);return await p;};const old=p;return +(await f())+(+(await old));",
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
fn fulfilled_promise_selections_preserve_object_truthiness_and_cells() {
    compare_promises(
        &[
            "const p=a?Promise.resolve(a):Promise.resolve(null);return await p;",
            "let p=Promise.resolve(a);const q=Promise.resolve(false);p=a?p:q;return await p;",
            "let n=0;const p=(n++,Promise.resolve(a));return +(await p)+n;",
            "let p=Promise.resolve(a);const q=p&&(p=Promise.resolve(false));return await q;",
            "let p=Promise.resolve(a);const q=p||(p=Promise.resolve(b));return await q;",
            "let p=Promise.resolve(a);const q=p??(p=Promise.resolve(b));return await q;",
            "let p=Promise.resolve(false);p&&=Promise.resolve(b);return await p;",
            "let p=Promise.resolve(undefined);p&&=Promise.resolve(await b);return await p;",
            "let p=Promise.resolve(a);p||=(await Promise.reject(b),Promise.resolve(b));return await p;",
            "let p=Promise.resolve(a);p??=Promise.resolve(b,await b);return await p;",
            "let p=Promise.resolve(a);return await (p&&=Promise.resolve(false));",
            "let p=Promise.resolve(a);return (p=Promise.resolve(false))?await p:b;",
            "const p=Promise.resolve(false);if(p)return await p;return b;",
            "const p=Promise.resolve(0);return !p;",
            "const p=Promise.resolve(Promise.resolve(a,a=b),a++);return +(await p)+a;",
            "let p=Promise.resolve(a);for(let i=0;i<3;i++){p=Promise.resolve(i);await b;}return await p;",
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
fn mutable_promise_assignments_admit_scalar_stores_and_keep_compound_guards() {
    compare_promises(
        &["let p=Promise.resolve(a);return (p=a);"],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
    for body in [
        "let p=Promise.resolve(a);p+=a;return await p;",
        "let p=Promise.resolve(a);p++;return await p;",
    ] {
        let error = compile_async(&source_for(body), false)
            .err()
            .expect("Promise scalar assignment must fail");
        assert!(
            error.starts_with("unsupported at bytes ") && error.contains("Promise"),
            "{body}: {error}"
        );
    }
}

#[test]
fn fulfilled_promise_expressions_keep_erasure_and_source_boundaries() {
    let plain = "export async function run(a,b){let p=Promise.resolve(a);p=a?Promise.resolve(false):p;return await (p||=Promise.resolve(b));}";
    let typed = "export async function run(a:never,b:Missing):Wrong{let p:NotPromise=Promise.resolve(a as never);p=((a as any)?Promise.resolve(false as never):(p as any)) as Wrong;return await ((p||=Promise.resolve(b as Missing)) as any);}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    for (count, admitted) in [(31, true), (32, false)] {
        let body = format!(
            "let p=Promise.resolve(a);{}return await p;",
            "p=Promise.resolve(await b);".repeat(count)
        );
        for ts in [false, true] {
            assert_eq!(compile_async(&source_for(&body), ts).is_ok(), admitted);
        }
        if admitted {
            compare_promises(&[&body], &[(4.0, 2.0)]);
        }
    }
    for body in [
        "const p=Promise.resolve(a);p=Promise.resolve(b);return await p;",
        "const p=Promise.resolve(a);throw p;",
        "const p=Promise.resolve(a);return await Promise.reject(p);",
    ] {
        for ts in [false, true] {
            assert!(compile_async(&source_for(body), ts).is_err(), "{body}");
        }
    }
    compare_promises(
        &["const p=Promise.resolve(a);return await (p&&a);"],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
    compare_promises(
        &[
            "let p=Promise.resolve(a);p=null;return await p;",
            "let p; p=Promise.resolve(b);return await p;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn promise_intrinsics_default_missing_payloads_to_undefined() {
    compare_promises(
        &[
            "return await Promise.resolve();",
            "const f=x=>x;return f(await Promise.resolve());",
            "const p=Promise.resolve();return await p;",
            "try{return await Promise.reject();}catch(e){return e;}",
            "const p=Promise.resolve();await b;return await p;",
            "const undefined=a;const p=Promise.resolve();const q=p;return await q;",
        ],
        &[(4.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn promise_intrinsics_preserve_ignored_argument_effects_and_jobs() {
    compare_promises(
        &[
            "return await Promise.resolve(a,a=b);",
            "const f=x=>x;return f(await Promise.resolve(null,a));",
            "return await Promise.resolve(b,a);",
            "const p=Promise.resolve(a,b);return await p;",
            "return (await Promise.resolve(a,a=b))+a;",
            "let n=0;return +(await Promise.resolve(null,n++,n=n*10+2))+n;",
            "let n=0;return +(await Promise.resolve(false,n++,n++))+n;",
            "try{return await Promise.reject(a,a=b);}catch(e){return e+a;}",
            "let n=0;const p=Promise.resolve(a,n++);a=b;const q=Promise.resolve(p,n++);return +(await q)+n;",
            "const p=Promise.resolve(a,a=await b);return (await p)+a;",
            "return (await Promise.resolve(a,a=await b))+a;",
            "return await Promise.resolve(await a,await b);",
            "try{return await Promise.reject(a,await b);}catch(e){return e;}",
            "let n=0;try{return await Promise.resolve(n++,await Promise.reject(b),n++);}catch(e){return +e+n;}",
            "let n=0;try{const p=Promise.resolve(n++,await Promise.reject(b),n++);return await p;}catch(e){return +e+n;}",
            "const p=Promise.resolve(a);return await Promise.resolve(p,await b);",
            "const f=async()=>{await a;return false;};return await Promise.resolve(await f(),await b);",
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
fn promise_intrinsic_arguments_keep_erasure_and_resource_boundaries() {
    let plain = "export async function run(a,b){const p=Promise.resolve(a,a=await b);return await Promise.resolve(p,a++);}";
    let typed = "export async function run(a:never,b:Missing):Wrong{const p:Promise<any>=(Promise.resolve((a as any),(a=await (b as never)) as Wrong) as any);return await (Promise.resolve((p as any),(a++ as never)) as any);}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    for count in [1000, 1001] {
        let arguments = vec!["a"; count].join(",");
        for body in [
            format!("return await Promise.resolve({arguments});"),
            format!("const p=Promise.resolve({arguments});return await p;"),
        ] {
            for ts in [false, true] {
                assert_eq!(
                    compile_async(&source_for(&body), ts).is_ok(),
                    count == 1000,
                    "{count}: {body}"
                );
            }
        }
    }
    for count in [31, 32] {
        let extras = vec!["await b"; count].join(",");
        let body = format!("return await Promise.resolve(a,{extras});");
        assert_eq!(
            compile_async(&source_for(&body), false).is_ok(),
            count == 31
        );
        if count == 31 {
            compare_promises(&[&body], &[(4.0, 2.0)]);
        }
    }
    for (stored, accepted) in [(false, 126), (true, 127)] {
        for count in [accepted, accepted + 1] {
            let extra = format!("{}b{}", "(".repeat(count), ")".repeat(count));
            let body = if stored {
                format!("const p=Promise.resolve(a,{extra});return await p;")
            } else {
                format!("return await Promise.resolve(a,{extra});")
            };
            assert_eq!(
                compile_async(&source_for(&body), false).is_ok(),
                count == accepted,
                "stored={stored}, wrappers={count}"
            );
        }
    }
    for body in [
        "const Promise=a;return await Promise.resolve(b,a);",
        "return await Promise.resolve?.(a,b);",
        "return await Promise.resolve(a,...b);",
        "const p=Promise.resolve(a,...b);return await p;",
        "return await Promise.reject(a,...b);",
    ] {
        for ts in [false, true] {
            assert!(compile_async(&source_for(body), ts).is_err(), "{body}");
        }
    }
}

#[test]
fn stored_fulfilled_promises_keep_object_and_intrinsic_boundaries() {
    for body in [
        "const p=Promise.resolve(a);return await Promise.reject(p);",
        "const Promise=a;const p=Promise.resolve(b);return await p;",
        "const p=Promise.resolve(...a);return await p;",
        "const p=Promise.resolve?.(a);return await p;",
        "const p=Promise.resolve(()=>a);return await p;",
        "const p=Promise.resolve({});return await p;",
        "const p=Promise.resolve('x');return await p;",
    ] {
        for ts in [false, true] {
            let source = source_for(body);
            assert!(compile_async(&source, ts).is_err(), "{source}");
        }
    }
}

#[test]
fn stored_fulfilled_promises_keep_erasure_and_source_job_bounds() {
    let plain =
        "export async function run(a,b){const p=Promise.resolve(a);const q=p;return await q;}";
    let typed = "export async function run(a:never,b:Missing):Promise<Wrong>{const p:Promise<any>=(Promise.resolve(a) as any);const q=(p as never);return await (q as any) as Wrong;}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    for (count, admitted) in [(127, true), (128, false)] {
        for body in [
            format!(
                "const p={}Promise.resolve(a){};return await p;",
                "(".repeat(count),
                ")".repeat(count)
            ),
            format!(
                "const p=Promise.resolve(a);return await {}p{};",
                "(".repeat(count),
                ")".repeat(count)
            ),
        ] {
            assert_eq!(
                compile_async(&source_for(&body), false).is_ok(),
                admitted,
                "{count} wrappers: {body}"
            );
        }
    }
    let body = format!(
        "const p=Promise.resolve(a);{}return await p;",
        "await p;".repeat(31)
    );
    compare_promises(&[&body], &[(4.0, 2.0)]);
    let over = format!(
        "const p=Promise.resolve(a);{}return await p;",
        "await p;".repeat(32)
    );
    assert!(
        compile_async(&source_for(&over), false)
            .err()
            .unwrap()
            .contains("32 await sites")
    );
}

#[test]
fn lone_factory_return_groups_preserve_async_captures_and_jobs() {
    compare_promises(
        &[
            "const make=()=>a?()=>null:()=>b;return a;",
            "let make=x=>{if(x)return ()=>a;return ()=>null;};return a;",
            "let n=0;const make=()=>{n++;return a?()=>null:()=>b;};await a;return n;",
            "let x=b;const make=()=>a?()=>null:()=>x;const f=make();await a;x=-0;return f();",
            "const make=()=>a?()=>undefined:()=>b;try{await Promise.reject(make()());}catch(x){await a;return x;}return b;",
            "const id=x=>x;const make=()=>a?()=>null:()=>b;return await id(a)+await id(b);",
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
fn nested_factory_result_groups_preserve_async_snapshots_and_jobs() {
    compare_promises(
        &[
            "let make=x=>y=>()=>x+y;make=x=>y=>()=>null;return a;",
            "let make=x=>y=>()=>x+y;make=x=>y=>()=>null;return await a;",
            "let make=x=>y=>()=>x+y;const old=make;make=x=>y=>()=>null;const f=old(await a)(await b);return f();",
            "let make=x=>y=>()=>x+y;const next=x=>y=>()=>null;const f=make((make=next,await a));return f(await b)();",
            "let make=x=>y=>()=>x+y;make=x=>y=>()=>null;try{await Promise.reject(make(a)(b)());}catch(x){await a;return x;}return b;",
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
fn joined_result_interfaces_preserve_async_values_and_snapshots() {
    compare_promises(
        &[
            "const f=()=>null;const g=()=>b;let h=a?f:g;await a;return h();",
            "let f=x=>null;const g=x=>x;return (a?f:g)((f=g,await b));",
            "const f=()=>null;const g=()=>b;let h=a?f:g;try{await Promise.reject(h());}catch(x){await a;return x;}return b;",
            "const f=()=>null;const g=()=>b;let h=a?f:g;return (a?g:g)()+await b;",
            "let f=()=>a;f=()=>true;return a;",
            "let f=()=>a;f&&=()=>true;return a;",
            "let f=()=>a;f||=()=>true;return a;",
            "let f=()=>a;f??=()=>true;return a;",
            "let make=x=>()=>null;make=x=>()=>x;return make(a)();",
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
fn immediate_selections_keep_async_values_and_callee_snapshots() {
    compare_promises(
        &[
            "const f=x=>+x;let p=f;p(await null);const g=x=>-x;return (a?f:g)(await b);",
            "const f=x=>x;let p=f;p(null);const g=x=>x;return await (a?f:g)(b);",
            "const f=x=>()=>x;let p=f;const keep=p(null);const g=x=>()=>x;const h=(a?f:g)(await b);await a;return h();",
            "let f=x=>+x;let p=f;p(null);const g=x=>-x;return 1/(a?f:g)((f=g,await null));",
            "const f=(cb,x)=>cb(x);let p=f;p(y=>+y,null);const g=(cb,x)=>cb(x)-b;return (a?f:g)(y=>+y,await b);",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

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
        Value::TypeError { .. } => "type-error".into(),
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
fn bitwise_compounds_preserve_async_snapshots_stores_and_jobs() {
    let mut bodies = Vec::new();
    for operator in ["&", "|", "^", "<<", ">>", ">>>"] {
        bodies.push(format!("return a{operator}=(await null);"));
        bodies.push(format!(
            "let x=null;const r=(x{operator}=(await(x=a,b)));return x===r?r:NaN;"
        ));
        bodies.push(format!(
            "let x=a;const r=(x{operator}=(await(x=false,b)));return x===r?r:NaN;"
        ));
        bodies.push(format!("let x=undefined;return x{operator}=(await(a<b));"));
        bodies.push(format!("let x=null;try{{x{operator}=(await Promise.reject(b));}}catch(e){{return x===null?e:NaN;}}return a;"));
    }
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    let plain = "export async function run(a,b){let x=null;return x|=(await(a<b));}";
    let typed = "export async function run(a:never,b:Missing):Wrong{let x:Wrong=null;return (x|=(await(a<b))) satisfies Wrong;}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
}

#[test]
fn bitwise_not_primitive_inputs_preserve_async_results_and_jobs() {
    compare_promises(
        &[
            "return ~(await null);",
            "return ~(await undefined);",
            "return ~(await(a<b));",
            "let x=a;const n=~x;await(x=false);return n;",
            "let x=null;return (~x)+(await(x=a));",
            "try{return ~(await Promise.reject(b));}catch(e){return e;}",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    let plain = "export async function run(a,b){return ~(await null);}";
    let typed = "export async function run(a:never,b:Missing):Wrong{return (~(await(null as never))) satisfies Wrong;}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
}

#[test]
fn binary_bitwise_primitive_inputs_preserve_async_snapshots_and_jobs() {
    let mut bodies = Vec::new();
    for operator in ["&", "|", "^", "<<", ">>", ">>>"] {
        bodies.push(format!("return (await null){operator}(await(a<b));"));
        bodies.push(format!("let x=a;return x{operator}(await(x=false,b));"));
        bodies.push(format!("let x=null;return x{operator}(await(x=a,b));"));
        bodies.push(format!("return (await undefined){operator}(await b);"));
        bodies.push(format!(
            "try{{return null{operator}(await Promise.reject(b));}}catch(e){{return e;}}"
        ));
    }
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    let plain = "export async function run(a,b){return (await null)|(await(a<b));}";
    let typed = "export async function run(a:never,b:Missing):Wrong{return ((await(null as never))|(await(a<b)) satisfies Wrong);}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
}

#[test]
fn unary_math_primitive_inputs_preserve_async_snapshots_and_jobs() {
    let mut bodies = Vec::new();
    for method in ["abs", "sqrt", "floor", "ceil", "trunc"] {
        bodies.push(format!("return Math.{method}(await null);"));
        bodies.push(format!("return Math.{method}(await(a<b));"));
        bodies.push(format!("return Math.{method}(await undefined);"));
        bodies.push(format!("let x=null;return Math.{method}(x,await(x=a))+x;"));
        bodies.push(format!(
            "let x=a;return Math.{method}(x,await(x=false))+(+x);"
        ));
        bodies.push(format!(
            "try{{return Math.{method}(await Promise.reject(b));}}catch(e){{return e;}}"
        ));
    }
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    let plain = "export async function run(a,b){return Math.abs(await null,await(a=b));}";
    let typed = "export async function run(a:never,b:Missing):Wrong{return (Math.abs(await(null as never),await(a=b)) satisfies Wrong);}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
}

#[test]
fn unary_math_missing_inputs_preserve_async_results_and_jobs() {
    let mut bodies = Vec::new();
    for method in ["abs", "sqrt", "floor", "ceil", "trunc"] {
        bodies.push(format!("return Math.{method}();"));
        bodies.push(format!("const n=Math.{method}();await(a=b);return n;"));
        bodies.push(format!("return (await Math.{method}())?a:b;"));
    }
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
    let plain = "export async function run(a,b){return Math.abs();}";
    let typed =
        "export async function run(a:never,b:Missing):Wrong{return (Math.abs() satisfies Wrong);}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
}

#[test]
fn unary_math_surplus_arguments_preserve_async_snapshots_and_jobs() {
    let mut bodies = Vec::new();
    for method in ["abs", "sqrt", "floor", "ceil", "trunc"] {
        bodies.push(format!(
            "let x=a;return Math.{method}(x,await(x=b,null))+x;"
        ));
        bodies.push(format!(
            "return Math.{method}(await a,await(a=b,null),await false)+a;"
        ));
        bodies.push(format!(
            "try{{return Math.{method}(a,await Promise.reject(b));}}catch(e){{return e;}}"
        ));
    }
    compare_promises(
        &bodies.iter().map(String::as_str).collect::<Vec<_>>(),
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    let plain = "export async function run(a,b){return Math.abs(a,await(a=b,null));}";
    let typed = "export async function run(a:never,b:Missing):Wrong{return (Math.abs(a as never,await(a=b,null as never)) satisfies Wrong);}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
}

#[test]
fn numeric_helper_surplus_arguments_preserve_async_snapshots_and_jobs() {
    let sources=[
        "function identity(x){return x;} export async function run(a,b){return identity(a,await(a=b,null))+a;}",
        "function value(){return 3;} export async function run(a,b){return value(await(a=b,null),await false)+a;}",
        "function choose(a){return a;} export async function run(a,b){return choose(await a,await(a=b,undefined));}",
        "function fold(n,x){if(n<=0)return x;return fold(n-1,x+1,null);} export async function run(a,b){return fold(3,await a,await null);}",
        "function identity(x){return x;} export async function run(a,b){try{return identity(a,await Promise.reject(b));}catch(e){return e;}}",
        "function identity(x){return x;} export async function run(a,b){const invoke=async x=>identity(x,await(a=b,null));return await invoke(a);}",
        "function identity(x){return x;} export async function run(a,b){const make=x=>(a=x,()=>x);return identity(a,await null,make(b));}",
        "function identity(x){return x;} export async function run(a,b){try{return await Promise.reject(identity(a,await null,b));}catch(e){return e;}}",
    ].map(str::to_string);
    compare_async_sources(
        &sources,
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    let plain = "function identity(x){return x;} export async function run(a,b){return identity(a,await(a=b,null));}";
    let typed = "function identity(x:string):never{return x as never;} export async function run(a:never,b:Missing):Wrong{return identity(a as never,await(a=b,null as never));}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
}

#[test]
fn runtime_surplus_arguments_preserve_async_pairs_and_job_order() {
    compare_promises(
        &[
            "let f=x=>x+a;return f(a,await(f=x=>x-b,a=b,null));",
            "const f=a?(()=>null):(()=>false);return f(await a,await(a=b,null));",
            "const invoke=fn=>fn(a,b);return invoke((await null,x=>x+b));",
            "const invoke=async fn=>fn(a,await(a=b,null));return await invoke(x=>x+a);",
            "let make=x=>()=>x+a;return make(await a,await(make=x=>()=>x-b,a=b,null))();",
            "let make=x=>(fn,y)=>fn(y,b);return make(await a)(()=>null,await b,await null);",
            "let f=x=>x+a;try{return f(a,await Promise.reject(b));}catch(e){return e;}",
            "const invoke=fn=>fn(a,b);try{return await Promise.reject(invoke(()=>null));}catch(e){return e;}",
            "const h=a?fn=>fn():fn=>fn(a);await null;return h(()=>b);",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    let plain =
        "export async function run(a,b){let f=x=>x+a;return f(a,await(f=x=>x-b,a=b,null));}";
    let typed = "export async function run(a:never,b:Missing):Wrong{let f=(x:Wrong)=>(x as any)+a;return f(a as never,await(f=(x:Missing)=>(x as any)-b,a=b,null as never));}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
}

#[test]
fn known_local_surplus_arguments_preserve_async_order_and_boundaries() {
    compare_promises(
        &[
            "const f=x=>x+a;return f(await a,await(a=b,null));",
            "const f=x=>x?null:b;return f(await a,await(a=b,false));",
            "const f=async x=>{await null;return x+a;};return await f(a,await(a=b,b));",
            "const f=async x=>{try{await null;return x+a;}catch(e){return b;}};try{return await f(a,await Promise.reject(a));}catch(e){return e;}",
            "const f=async x=>{await Promise.reject(b);return x;};try{return await f(a,await(a=b,null));}catch(e){return e;}",
            "const make=x=>()=>x+a;const f=make(await a,await(a=b,null));await null;return f();",
            "const make=x=>{const f=()=>x?false:b;return f;};return make(await a,await(a=b,null))();",
            "const id=x=>x;const f=id(()=>a,await(a=b,null));await null;return f();",
            "const f=x=>x+a;try{return f(a,await Promise.reject(b));}catch(e){return e;}",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    let plain = "export async function run(a,b){const f=x=>x+a;return f(a,await(a=b,null));}";
    let typed = "export async function run(a:never,b:Missing):Wrong{const f=(x:Wrong)=>(x as any)+a;return f(a as never,await(a=b,null as never));}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
}

#[test]
fn primitive_callbacks_keep_caller_jobs_and_argument_snapshots() {
    compare_promises(
        &[
            "const apply=(fn,x)=>fn(x);return apply(x=>null,await a);",
            "const apply=(fn,x)=>fn(x);return apply((await null,x=>x?false:b),await a);",
            "const h=a?(fn,x)=>fn(x):(fn,x)=>fn(x);return h(y=>true,await b);",
            "const apply=(fn,x)=>fn(x);const middle=(cb,x)=>cb(y=>y?false:b,x);const outer=(step,x)=>step(apply,x);return outer(middle,await a);",
            "const make=offset=>(fn,x)=>()=>fn(x+offset);const f=make(await a)(x=>x?false:null,await b);await null;return f();",
            "const apply=(fn,x)=>fn(x);let leaf=x=>x?null:b;return apply(leaf,await(leaf=x=>undefined,a));",
            "const apply=(fn,x)=>fn(x);try{return await Promise.reject(apply(x=>null,a));}catch(e){return e;}",
            "const apply=(fn,x)=>fn(x);try{return apply(x=>null,await Promise.reject(a));}catch(e){return e;}",
            "const apply=(fn,x)=>fn(x);let value=apply(x=>x?null:b,await a);await null;return value;",
        ],
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    let plain = "export async function run(a,b){const apply=(fn,x)=>fn(x);return apply(x=>x?null:b,await a);}";
    let typed = "export async function run(a:never,b:Missing):Wrong{const apply=(fn:string,x:never)=>(fn as any)(x);return apply(((x:never)=>x?null:b) as any,await(a as never));}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
}

#[test]
fn higher_callback_shapes_keep_caller_jobs_and_argument_order() {
    compare_promises(
        &[
            "const apply=(fn,x)=>fn(x);const invoke=(cb,x)=>cb(y=>y+b,x);return invoke(apply,await a);",
            "const apply=(fn,x)=>fn(x);const invoke=(cb,fn,x)=>cb(fn,x);return invoke(apply,(await null,y=>y+b),await a);",
            "const apply=(fn,x)=>fn(x);const invoke=(cb,fn,x)=>cb(fn,x);let leaf=y=>y+a;return invoke(apply,leaf,await(leaf=y=>y-b,b))+leaf(a);",
            "const apply=(fn,x)=>fn(x);const invoke=(cb,x)=>{const alias=cb;cb=(fn,y)=>fn(y)+b;return alias(y=>y+a,x)+cb(y=>y-b,x);};await null;return invoke(apply,await a);",
            "const apply=(fn,x)=>fn(x);const middle=(cb,x)=>cb(y=>y+b,x);const outer=(step,x)=>step(apply,x);return outer(middle,await a);",
            "const apply=(fn,x)=>fn(x);const make=offset=>(cb,x)=>()=>cb(y=>y+offset,x);const f=make(await a)(apply,await b);await null;return f();",
            "const apply=(fn,x)=>fn(x);const invoke=(cb,x)=>x?null:cb(y=>y+b,x);await null;return invoke(apply,await a);",
            "const apply=(fn,x)=>fn(x);const invoke=(cb,x)=>{cb(y=>y+b,x);};return invoke(apply,await a);",
            "const apply=(fn,x)=>fn(x);const invoke=(cb,x)=>cb(y=>y+b,x);try{return await Promise.reject(invoke(apply,a));}catch(e){return e;}",
            "const apply=(fn,x)=>fn(x);const invoke=(cb,x)=>cb(y=>y+b,x);try{return invoke(apply,await Promise.reject(a));}catch(e){return e;}",
        ],
        &[
            (4.0, 2.0),
            (0.0, 2.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    let plain = "export async function run(a,b){const apply=(fn,x)=>fn(x);const invoke=(cb,x)=>cb(y=>y+b,x);return invoke(apply,await a);}";
    let typed = "export async function run(a:never,b:Missing):Wrong{const apply=(fn:string,x:never)=>(fn as any)(x);const invoke=(cb:never,x:Missing)=>(cb as any)(((y:number)=>y+b) as any,x);return invoke(apply,await(a as never));}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
}

#[test]
fn returned_function_callbacks_keep_argument_snapshots_and_async_jobs() {
    compare_promises(
        &[
            "let make=x=>(fn,y)=>fn(y)+x;const apply=make(await a);await null;return apply(z=>z+b,b);",
            "let make=x=>(fn,y)=>fn(y)+x;const apply=make(a);return apply((await null,z=>z+b),await b);",
            "let make=x=>(fn,y)=>fn(y);let fn=z=>z+a;const apply=make(a);return apply(fn,await(fn=z=>z-b,b))+fn(a);",
            "let make=x=>(fn,y)=>()=>fn(y)+x;const f=make(a)(z=>z+b,await b);await null;return f();",
            "let make=x=>{return (fn,y)=>{return ()=>fn(y)+x;};};const middle=make(await a);await null;return middle(z=>z+b,await b)();",
            "let make=x=>(fn,y)=>()=>{const old=fn(y);fn=z=>z+b;return old+fn(y)+x;};const f=make(a)(z=>z+a,b);await null;f();await false;return f();",
            "let make=x=>(fn,y)=>x?null:fn(y);const apply=make(await a);await null;return apply(z=>z+b,b);",
            "let make=x=>fn=>x?null:fn();const apply=make(a);await false;return apply(()=>-0);",
            "let make=x=>(fn,y)=>{fn(y);};return make(a)(z=>z+b,await b);",
            "let make=x=>(fn,y)=>x?null:fn(y);const apply=make(a);try{return await Promise.reject(apply(z=>z+b,b));}catch(e){return e;}",
            "let make=x=>(fn,y)=>()=>fn(y);let fn=z=>z+a;const f=make(a)(fn,await(fn=z=>z-b,b));await null;return f();",
            "let make=x=>(fn,y)=>fn(y);const original=make(a);await null;make=x=>(fn,y)=>fn(y)+x;return original(z=>z+b,b);",
            "let make=x=>(fn,y)=>fn(y)+x;make??=(await Promise.reject(a),x=>(fn,y)=>fn(y)-x);return make(a)(z=>z+b,b);",
            "let make=x=>(fn,y)=>fn(y)+x;make&&=(await null,x=>(fn,y)=>fn(y)-x);return make(a)(z=>z+b,b);",
            "let make=x=>{return (fn,y)=>fn(y);};return a;",
            "let make=x=>(fn,y)=>()=>fn(y);return a;",
        ],
        &[
            (4.0, 2.0),
            (0.0, 2.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    let plain = "export async function run(a,b){let make=x=>(fn,y)=>fn(y)+x;const apply=make(await a);await false;return apply(z=>z+b,b);}";
    let typed = "export async function run(a:never,b:Missing):Wrong{let make:Wrong=((x:Missing)=>(fn:string,y:never)=>(fn as any)(y)+x) as any;const apply=make(await(a as never));await(false as number);return (apply as any)(((z:number)=>z+b) as any,b);}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
}

#[test]
fn nested_runtime_factories_retain_pairs_values_and_job_order() {
    compare_promises(
        &[
            "let make=x=>()=>()=>x;return make(a)()();",
            "let make=x=>y=>()=>x+y;const middle=make(await a);await null;const f=middle(await b);await false;return f();",
            "let make=a?(x=>y=>()=>x+y):(x=>y=>()=>x-y);return make(await a)(await b)();",
            "let make=x=>{let y=x;return z=>()=>++y+z;};const middle=make(a);const f=middle(b);await null;const g=middle(a);return f()+g()+f();",
            "let make=x=>y=>()=>x+y;const middle=make(a);await null;make=x=>y=>()=>x-y;return middle(b)()+make(a)(b)();",
            "let make=x=>y=>()=>x+y;let middle=make(a);const f=middle((await null,middle=make(b),a));return f();",
            "let make=x=>{return y=>{return ()=>null;};};const middle=make(await a);await null;return middle(b)();",
            "let make=x=>y=>()=>x?null:false;return await make(await a)(await b)();",
            "let make=x=>()=>()=>undefined;const f=make(a)();await null;return f();",
            "let make=x=>()=>()=>true;const middle=make(a);try{return await Promise.reject(middle()());}catch(e){return e;}",
            "let make=x=>()=>()=>null;const middle=make(a);return await Promise.reject(middle()());",
            "let make=x=>y=>()=>x+y;make??=(await Promise.reject(a),x=>y=>()=>x-y);return make(a)(b)();",
            "let make=x=>y=>()=>x+y;make&&=(await null,x=>y=>()=>x-y);return make(a)(b)();",
            "let make=(fn,x)=>y=>()=>fn(x)+y;const middle=make(z=>z+b,await a);await null;return middle(await b)();",
        ],
        &[
            (4.0, 2.0),
            (0.0, 2.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
    let plain = "export async function run(a,b){let make=x=>y=>()=>null;const middle=make(await a);await false;return middle(b)();}";
    let typed = "export async function run(a:never,b:Missing):Wrong{let make:Wrong=((x:Missing)=>(y:never)=>(()=>null) as any) as any;const middle=make(await(a as never));await(false as number);return (middle as any)(b)();}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
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
function bits(value){if(value===undefined)return 'undefined';if(value===null)return 'null';if(typeof value==='boolean')return 'boolean:'+value;if(value instanceof TypeError)return 'type-error';assert.equal(typeof value,'number');if(Number.isNaN(value))return 'nan';view.setFloat64(0,value);return view.getBigUint64(0).toString(16);}
async function settlements(run,a,b,c,d){const rows=await Promise.allSettled([run(a,b),run(c,d)]);const errors=rows.map(r=>r.status==='fulfilled'?r.value:r.reason).filter(v=>v instanceof TypeError);if(errors.length===2)assert.notStrictEqual(errors[0],errors[1],'separate invocations require separate error objects');return rows.map(r=>(r.status==='fulfilled'?r.value:r.reason) instanceof TypeError?{status:r.status,errorKind:'TypeError'}:r);}
async function observe(run,a,b){
 const trace=[];const promise=run(a,b);assert(promise instanceof Promise);trace.push('returned');
 const done=promise.then(value=>{trace.push('fulfilled:'+bits(value));},reason=>{trace.push('rejected:'+bits(reason));});
 let count=0;const markers=new Promise(resolve=>{function tick(){trace.push('tick'+(++count));if(count<34)queueMicrotask(tick);else resolve();}queueMicrotask(tick);});
 await Promise.all([done,markers]);const outcome=trace.find(entry=>entry.startsWith('fulfilled:')||entry.startsWith('rejected:'));
 return {trace,outcome,ticks:trace.slice(0,trace.indexOf(outcome)).filter(entry=>entry.startsWith('tick')).length};
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
            // One Wasmtime engine + compiled Module per source/mode.
            // Every vector still receives a fresh Store and Instance.
            let prepared = PreparedAsyncModule::new(&artifact.wasm).unwrap();
            let mut rust = Vec::new();
            for &(a, b) in vectors {
                let (state, ticks) = observe_rust_fifo(&prepared, a, b);
                let (kind, value) = match state {
                    AsyncState::Fulfilled(value) => ("fulfilled", value),
                    AsyncState::Rejected(value) => ("rejected", value),
                    _ => unreachable!(),
                };
                let bits = primitive_bits(value);
                rust.push(format!("{{outcome:'{kind}:{bits}',ticks:{ticks}}}"));
            }
            script.push_str(&format!("{{const native=(()=>{{{};return run;}})();const wasm=createAsyncFunction(new Uint8Array({:?}));const rust=[{}];let i=0;for(const [a,b] of vectors){{const expected=await observe(native,a,b);assert.deepEqual(await observe(wasm,a,b),expected,{:?});assert.equal(rust[i].outcome,expected.outcome);assert.equal(rust[i].ticks,expected.ticks);i++;}}const [a,b]=vectors[0], [c,d]=vectors.at(-1);assert.deepEqual(await settlements(wasm,a,b,c,d),await settlements(native,a,b,c,d));}}\n",source.replacen("export async function run", "async function run", 1),artifact.wasm,rust.join(","),source));
        }
    }
    run_node(&script);
}

// Observe the same external marker chain as Node. Several ready guest jobs may
// precede one marker; root settlement can precede the last owned guest job.
fn observe_rust_fifo(prepared: &PreparedAsyncModule, a: f64, b: f64) -> (AsyncState, usize) {
    use std::collections::VecDeque;
    enum Job {
        Guest,
        Tick(u8),
        Observe(AsyncState),
    }
    let mut activation = prepared.instantiate(a, b).unwrap();
    let mut queue = VecDeque::new();
    let mut scheduled = activation.pending_job_count().unwrap();
    for _ in 0..scheduled {
        queue.push_back(Job::Guest);
    }
    let mut settled = !matches!(activation.state().unwrap(), AsyncState::Pending);
    if settled {
        queue.push_back(Job::Observe(activation.state().unwrap()));
    }
    queue.push_back(Job::Tick(1));
    let mut ticks = 0;
    let mut calls = 0;
    let mut result = None;
    while let Some(job) = queue.pop_front() {
        match job {
            Job::Guest => {
                scheduled -= 1;
                assert!(activation.run_one_job().unwrap());
                calls += 1;
                assert!(calls <= 32);
                let owed = activation.pending_job_count().unwrap();
                assert!(owed >= scheduled);
                for _ in scheduled..owed {
                    queue.push_back(Job::Guest);
                }
                scheduled = owed;
                let state = activation.state().unwrap();
                if !settled && !matches!(state, AsyncState::Pending) {
                    settled = true;
                    queue.push_back(Job::Observe(state));
                }
            }
            Job::Tick(n) => {
                ticks += 1;
                if n < 34 {
                    queue.push_back(Job::Tick(n + 1));
                }
            }
            Job::Observe(state) => {
                assert!(result.is_none());
                result = Some((state, ticks));
            }
        }
    }
    assert_eq!(activation.pending_job_count().unwrap(), 0);
    assert!(!activation.run_one_job().unwrap());
    result.expect("root must settle before the observation can finish")
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
    let script = format!("{script}\nconsole.log('ASYNC_TEST_DONE');\n");
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
    assert!(
        String::from_utf8_lossy(&result.stdout)
            .lines()
            .any(|line| line == "ASYNC_TEST_DONE"),
        "Node exited before all async observations finished"
    );
}

#[test]
fn profile_boundaries_and_ordinary_async_rejections_remain_explicit() {
    for source in [
        "export function run(a,b){return a+b;}",
        "export async function run(Promise,b){return await Promise.resolve(b);}",
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
        "const f=async()=>a;return f;",
        "const f=async()=>()=>a;return await f();",
        "const f=async()=>{return ()=>a;};return await f();",
        "const f=async()=>'';return await f();",
        "const f=async()=>({});return await f();",
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
    {
        let body = "const f=async()=>await({});return await f();";
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
    let body = "const f=async()=>Promise.reject(a);return void await f();";
    compare_promises(&[body], &[(4.0, 2.0), (0.0, -0.0)]);
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
