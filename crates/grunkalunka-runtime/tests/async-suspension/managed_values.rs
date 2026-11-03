use super::*;

#[test]
fn dictionary_writes_preserve_receiver_and_key_snapshots_across_await() {
    compare_promises(
        &[
            "let d=Object.create(null);const old=d;let key='x';d[key]=(await a,key='y',d=Object.create(null),b);await b;return old.x===b&&old.y===undefined&&d.x===undefined?1:0;",
            "let d=Object.create(null);const old=d;d[(await a,d=Object.create(null),'x')]=(await b,a);return old.x===a&&d.x===undefined?1:0;",
            "const d=Object.create(null);const key='__proto__';const value=d[key]=(await b,a);await a;return d.__proto__===a&&value===a?1:0;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn dictionary_reads_preserve_receiver_and_value_snapshots_across_await() {
    compare_promises(
        &[
            "let d=Object.create(null);const old=d;d.x=a;const value=d[(await a,d=Object.create(null),'x')];await b;return value===a&&d!==old?1:0;",
            "const d=Object.create(null);d.x=a;const key='x';const value=d[(await b,d.x=b,key)];await a;d.x=a;return value===b?1:0;",
            "const d=Object.create(null);d.__proto__=a;const key=(await b,'__proto__');const value=d[key];await a;return value===a?1:0;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn dictionary_static_values_and_aliases_survive_suspension() {
    compare_promises(
        &[
            "const d=Object.create(null);d.__proto__=a;const saved=d.__proto__;await b;d.__proto__=b;await a;return saved===a&&d.__proto__===b?1:0;",
            "let d=Object.create(null);const alias=d;d.constructor=a;await b;d=Object.create(null);d.constructor=b;await a;return alias!==d&&alias.constructor===a&&d.constructor===b?1:0;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_pop_preserves_live_length_and_saved_results_across_suspension() {
    compare_promises(
        &[
            "let x=[a,b];const old=x;const saved=x.pop((await a,x=[b]),(await b,old.length=1));await a;return saved===a&&old.length===0&&x.length===1?1:0;",
            "const o={x:a};const x=[o];const saved=x.pop();await b;x[0]=null;await a;return saved===o?1:0;",
            "const x=[a,b];const take=()=>x.pop();await a;const saved=take();await b;return saved===b&&x.length===1?1:0;",
            "const x=[a,b];try{x.pop(await Promise.reject(a));}catch(e){return x.length===2&&x[1]===b?1:0;}return 0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn array_at_preserves_snapshots_through_suspended_arguments_and_results() {
    compare_promises(
        &[
            "const x=[a,b];const selected=x.at((await a,0),(await b,x[0]=b));await a;return selected===b?1:0;",
            "let x=[a,b];const old=x;const selected=x.at((await a,x=[b],-1),(await b,old.length=1));await a;return selected===a&&old!==x?1:0;",
            "const o={x:a};const x=[o];const saved=x.at(0);await b;x[0]=null;await a;return saved===o?1:0;",
            "const x=Object.keys({...{first:a,second:b}});await a;return x.at(-1)==='second'?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn string_concat_retains_left_snapshots_across_await() {
    compare_promises(
        &[
            "let s='left';const text=s+(await a,s='right');await b;return text==='leftright'&&s==='right'?1:0;",
            r"const left='\ud800';const text=left+(await a,'\udfff');await b;return text==='\ud800\udfff'&&text.length===2?1:0;",
            "let s='';const add=()=>s=s+'x';await a;add();await b;add();return s.length;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn string_length_keeps_utf16_snapshots_across_suspension() {
    compare_promises(
        &[
            r"const s='\ud800\u{1f600}';await b;return s.length;",
            "let s='old';const saved=s;const f=()=>s.length;await a;s='longer';await b;return saved['length']*10+f();",
            "const o={text:a?'one':''};const saved=o.text;await b;o.text='replacement';return saved.length*100+o.text.length;",
            "let n=0;const s='three';return (await a,n++,s).length+n;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn object_keys_nullable_input_uses_a_catchable_typeerror() {
    compare_promises(
        &[
            "const source=a?{x:b}:null;try{const keys=Object.keys(source);return keys.length;}catch(e){return typeof e==='object'?-1:0;}",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn object_keys_fallible_inputs_preserve_finalizers_and_argument_order() {
    compare_promises(
        &[
            "const source=a?'xy':null;try{const k=Object.keys(source);await b;return k.length;}catch(e){await b;return typeof e==='object'&&e===e?-1:0;}",
            "const source=a?[a,,b]:true;const k=Object.keys(source);await b;return a?(k.length===2&&k[1]==='2'?1:0):(k.length===0?1:0);",
            "const f=()=>b;const source={...{f}};const k=Object.keys(source.f);await b;return k.length===0?1:0;",
            "const source=a?a:null;try{return Object.keys(source).length;}catch(e){return -1;}",
            "let n=0;try{try{Object.keys();}catch(e){await a;throw e;}finally{await b;n++;}}catch(e){return n===1&&typeof e==='object'?1:0;}return 0;",
            "try{Object.keys(undefined);}finally{await a;return b;}",
            "let n=0;try{Object.keys.call((n++,null),null,(n++,await Promise.reject(b)));}catch(e){return n===2&&e===b?1:0;}return 0;",
            "let n=0;try{Object.keys(null,(n++,await a));}catch(e){return n===1&&typeof e==='object'?1:0;}return 0;",
            "const o={x:a};const k=Object.keys(o,(await b,o.y=b));await a;return k.length===2&&k[1]==='y'?1:0;",
            "try{Object.keys(null);}catch(e){const k=Object.keys(e);const o={e};await a;return k.length===0&&o.e===e&&typeof e==='object'&&!!e&&+(e)!==+(e)?1:0;}return 0;",
            "let saved=null;try{Object.keys(null);}catch(e){saved=e;}try{Object.keys(undefined);}catch(e){await a;return e!==saved&&!(e<saved)&&e<=saved&&!(e>saved)&&e>=saved?1:0;}return 0;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn object_keys_uses_js_error_string_order_across_intrinsic_causes() {
    compare_promises(
        &[
            "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();try{await p;}catch(cycle){try{Object.keys(null);}catch(toObject){await b;return toObject<cycle&&toObject<=cycle&&!(toObject>cycle)&&!(toObject>=cycle)&&cycle>toObject&&cycle>=toObject&&!(cycle<toObject)&&!(cycle<=toObject)?1:0;}}return 0;",
        ],
        &[(4.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn object_spread_composes_suspended_values_and_preserves_source_order() {
    compare_promises(
        &[
            "const source={x:a};let n=0;const copy={before:(n++,a),...(await b,n++,source),after:(n++,source.x=b)};await a;return n===3&&copy.x===a&&copy.after===b?1:0;",
            "const source={x:a};const f=async()=>{await b;return 1;};const p=f();const copy={...source,...(await p,source)};await b;return copy.x===a?1:0;",
            "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();try{await p;}catch(e){const copy={...e,x:a};await b;return copy.x===a&&copy.message===undefined?1:0;}return 0;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

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
fn public_managed_values_and_unrepresented_promise_payloads_still_reject() {
    for body in [
        "const o={};await a;return o;",
        "const o={};await o;return a;",
        "const o={};await a;throw o;",
        "const p=Promise.resolve(a);const o={p};await b;return o.p===p?1:0;",
        "const p=Promise.resolve(a);try{await Promise.reject(p);}catch(e){const o={x:e};await b;return o.x===e?1:0;}return a;",
    ] {
        let source = format!("export async function run(a,b){{{body}}}");
        for ts in [false, true] {
            let result = std::panic::catch_unwind(|| compile_async(&source, ts))
                .unwrap_or_else(|_| panic!("compiler panic: {source}"));
            let error = result
                .err()
                .unwrap_or_else(|| panic!("unexpected admission: {source}"));
            assert!(!error.contains("internal IR"), "{source}: {error}");
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
