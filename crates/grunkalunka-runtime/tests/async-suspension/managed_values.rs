use super::*;

#[test]
fn large_managed_literals_keep_construction_scratch_separate_from_async_roots() {
    use wasmtime::{Collector, Config, Engine, Instance, Module, Store};

    let text = "x".repeat(10_001);
    for source in [
        format!("export async function run(a,b){{const s='{text}';await b;return s?a:b;}}"),
        format!("export async function run(a,b){{await b;const s='{text}';await a;return s?a:b;}}"),
        format!("export async function run(a,b){{const o={{'{text}':a}};await b;return o?a:b;}}"),
        format!(
            "export async function run(a,b){{const o={{}};await b;return o['{text}']===undefined?a:b;}}"
        ),
        format!(
            "function make(){{return '{text}';}}export async function run(a,b){{const s=make();await b;return s?a:b;}}"
        ),
    ] {
        for ts in [false, true] {
            let artifact = compile_async(&source, ts).unwrap();
            // Large construction measures semantics with a bounded core-engine
            // budget; the ordinary production budget must still exhaust cleanly.
            let mut config = Config::new();
            config.consume_fuel(true).collector(Collector::Copying);
            let engine = Engine::new(&config).unwrap();
            let module = Module::new(&engine, &artifact.wasm).unwrap();
            for (a, b) in [(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)] {
                let mut store = Store::new(&engine, ());
                store.set_fuel(1_000_000).unwrap();
                let instance = Instance::new(&mut store, &module, &[]).unwrap();
                let start = instance
                    .get_typed_func::<(f64, f64), (i32, i32, f64)>(&mut store, "start")
                    .unwrap();
                let resume = instance
                    .get_typed_func::<(i32, f64), (i32, i32, f64)>(&mut store, "resume")
                    .unwrap();
                let mut state = start.call(&mut store, (a, b)).unwrap();
                let mut suspensions = 0;
                while state.0 == 1 {
                    suspensions += 1;
                    assert!(suspensions <= 2);
                    store.gc(None).unwrap();
                    state = resume.call(&mut store, (state.1, state.2)).unwrap();
                }
                assert!(suspensions > 0);
                assert_eq!((state.0, state.1), (0, 3));
                assert!(state.2.is_nan() && a.is_nan() || state.2.to_bits() == a.to_bits());
            }
            let error = match AsyncInstance::new(&artifact.wasm, 4.0, 2.0) {
                Err(error) => error,
                Ok(mut activation) => loop {
                    match activation.run_one_job() {
                        Err(error) => break error,
                        Ok(true) => (),
                        Ok(false) => {
                            panic!("large construction escaped the production fuel budget")
                        }
                    }
                },
            };
            assert!(format!("{error:?}").contains("fuel"), "{error:?}");
            run_node(&format!(
                r#"
import assert from 'node:assert/strict';import {{pathToFileURL}} from 'node:url';
const {{createAsyncFunction}}=await import(pathToFileURL(process.argv[2]));
const native=(()=>{{{};return run;}})(),wasm=createAsyncFunction(new Uint8Array({:?}));
async function observe(run,a,b){{const trace=['returned'];const done=run(a,b).then(v=>trace.push(['fulfilled',v]));let n=0;const markers=new Promise(resolve=>{{function tick(){{trace.push(++n);if(n<12)queueMicrotask(tick);else resolve();}}queueMicrotask(tick);}});await Promise.all([done,markers]);return trace;}}
for(const [a,b] of [[4,2],[-0,0],[NaN,Infinity]])assert.deepEqual(await observe(wasm,a,b),await observe(native,a,b));
assert.deepEqual(await Promise.all([wasm(4,2),wasm(-0,0)]),await Promise.all([native(4,2),native(-0,0)]));
"#,
                source.replacen("export async function run", "async function run", 1),
                artifact.wasm
            ));
        }
    }
}

#[test]
fn managed_objects_keep_identity_and_alias_mutation_across_await() {
    compare_promises(
        &["const o={x:a};const p=o;await b;p.x=b;return o===p&&o.x===b?1:0;"],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn managed_arrays_strings_and_saved_values_survive_suspension() {
    compare_promises(
        &[
            "const x=[a,,b];await b;x[1]=b;x.length=5;return x.length+(x[1]===b?1:0);",
            "const xs=[a,b];await a;xs[0]=b;return xs[0]===b?1:0;",
            "const o={child:{x:a}};await b;o.child.x=b;return o.child.x===b?1:0;",
            "const s=a?'x':'';await b;return s==='x'?a:b;",
            "const o={x:a?undefined:null};const saved=o.x;await b;o.x=b;return saved===null?b:a;",
            "let o={x:a};const saved=o;await b;o={x:b};return saved!==o&&saved.x===a&&o.x===b?1:0;",
            "const o={};o.self=o;await a;return o.self===o?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn managed_values_follow_existing_jobs_handlers_and_loop_completions() {
    compare_promises(
        &[
            "const o={x:a};for(let i=0;i<3;i++){await i;o.x=b;}return o.x===b?1:0;",
            "const o={x:a};try{await Promise.reject(b);}catch(e){o.x=e;}finally{await a;}return o.x===b?1:0;",
            "const o={x:a};const n=o?a:await Promise.reject(b);return n;",
            "const o={x:a};await b;return o?null:undefined;",
            "const o={x:a};await b;return Promise.resolve(o.x?a:b);",
            "const o={x:a};await b;return Promise.reject(o.x?a:b);",
            "const o={};const f=async x=>x;await f(a);return o?1:0;",
            "const f=async x=>{const o={x};await b;return o.x===x?1:0;};return await f(a);",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn synchronous_managed_helpers_keep_reference_signatures_in_async_modules() {
    compare_async_sources(
        &[
            "export async function run(a,b){const kind=typeof a;await b;return kind==='number'?a:b;}".into(),
            "function kind(x){return typeof x==='number'?1:0;}export async function run(a,b){await a;return kind(b)?a+b:a;}".into(),
            "function identity(o){return o;} export async function run(a,b){const o={};return identity(o)===o?a:b;}".into(),
            "function identity(o){return o;}export async function run(a,b){const o={x:a};const p=identity(o);await b;p.x=b;return p===o&&o.x===b?1:0;}".into(),
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn reference_bearing_multiple_frames_and_public_managed_values_still_reject() {
    for (index, body) in [
        "const o={};const f=()=>o;await a;return f()===o?1:0;",
        "const o={};const f=async x=>{await b;return x;};const p=f(a);await p;return o?1:0;",
        "const f=async x=>{const o={x};await b;return o.x===x?1:0;};const p=f(a);return await p;",
        "const o={};await a;return o;",
        "const o={};await o;return a;",
        "const o={};await a;throw o;",
        "const p=Promise.resolve(a);const o={p};await b;return o.p===p?1:0;",
        "const p=Promise.resolve(a);try{await Promise.reject(p);}catch(e){const o={x:e};await b;return o.x===e?1:0;}return a;",
    ]
    .into_iter()
    .enumerate()
    {
        let source = format!("export async function run(a,b){{{body}}}");
        for ts in [false, true] {
            let result = std::panic::catch_unwind(|| compile_async(&source, ts))
                .unwrap_or_else(|_| panic!("compiler panic: {source}"));
            let error = result
                .err()
                .unwrap_or_else(|| panic!("unexpected admission: {source}"));
            if index < 3 {
                assert_eq!(
                    error, "managed values require reference-aware async frames",
                    "{source}"
                );
            }
        }
    }
}

#[test]
fn former_managed_async_guards_execute_the_original_sources() {
    compare_async_sources(
        &[
            "function identity(o){return o;} export async function run(a,b){const o={};return identity(o)===o?a:b;}".into(),
            "export async function run(a,b){const x=[a];return x.length;}".into(),
            "export async function run(a,b){const s='x';return s==='x'?a:b;}".into(),
            "export async function run(a,b){const o={x:a};return o.x?a:b;}".into(),
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn managed_async_values_keep_erased_types_and_small_stack_limits() {
    let js = "export async function run(a,b){const xs=[a,b];await a;xs[0]=b;return xs[0]===b?1:0;}";
    let ts = "export async function run(a:string,b:boolean):void{const xs=[a,b];await(a as unknown);(xs as never)[0]=b;return(xs[0]as Missing)===b?1:0;}";
    assert_eq!(
        compile_async(js, false).unwrap().wasm,
        compile_async(ts, true).unwrap().wasm
    );
    for (count, admitted) in [(24, true), (129, false)] {
        let aliases: String = (1..=count)
            .map(|i| format!("const o{i}=o{};", i - 1))
            .collect();
        for ts in [false, true] {
            let source = format!(
                "export async function run(a,b){{const o0={{x:a}};{aliases}await b;return o{count}===o0?1:0;}}"
            );
            let result = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || compile_async(&source, ts))
                .unwrap()
                .join()
                .unwrap();
            assert_eq!(result.is_ok(), admitted, "{:?}", result.as_ref().err());
            if let Err(error) = result {
                assert!(error.contains("unsupported"), "{error}");
            }
        }
    }
}
