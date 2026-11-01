use super::*;

#[test]
fn related_async_activations_share_objects_and_keep_separate_snapshots() {
    compare_promises(
        &[
            "const o={x:a};const f=async n=>{await n;o.x=n;return o.x===n?1:0;};const p=f(a);const q=f(b);const x=await p;const y=await q;return o.x===b&&x===1&&y===1?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn overlapping_frames_keep_local_objects_arrays_strings_and_cycles() {
    compare_promises(
        &[
            "const f=async n=>{const o={x:n};o.self=o;await b;return o.self===o&&o.x===n?1:0;};const p=f(a);const q=f(b);return(await p)===1&&(await q)===1?1:0;",
            "const f=async n=>{const xs=[n];await b;xs[1]=b;return xs.length===2&&xs[0]===n?1:0;};const p=f(a);const q=f(b);return(await p)===1&&(await q)===1?1:0;",
            "let s='old';const f=async n=>{const saved=s;await n;return saved==='old'?1:0;};const p=f(a);s='new';const q=f(b);return(await p)===1&&(await q)===0?1:0;",
            "const o={x:a};let current=o;const f=async n=>{const saved=current;await n;return saved===o?1:0;};const p=f(a);current={x:b};const q=f(b);return(await p)===1&&(await q)===0?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn async_frame_parameters_transport_managed_references_and_functions() {
    compare_promises(
        &[
            "const o={x:a};const f=async(target,n)=>{const saved=target;await n;target.x=n;return saved===target?1:0;};const p=f(o,a);const q=f(o,b);return(await p)===1&&(await q)===1&&o.x===b?1:0;",
            "const xs=[a];const f=async(target,n)=>{await n;target[0]=n;return target===xs?1:0;};const p=f(xs,a);const q=f(xs,b);return(await p)===1&&(await q)===1&&xs[0]===b?1:0;",
            "const target=()=>a;const keep={target};const f=async fn=>{await b;return fn();};const p=f(target);const q=f(()=>b);return(await p)===a&&(await q)===b?1:0;",
            "const f=async(s,n)=>{await n;return s==='text'?1:0;};const p=f('text',a);const q=f('other',b);return(await p)===1&&(await q)===0?1:0;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn directly_awaited_gc_closures_preserve_dispatch_and_parent_links() {
    compare_promises(
        &[
            "const o={x:a};const f=a?async n=>{await b;return o.x===n?1:0;}:async n=>{await a;return o.x===n?1:0;};return await f(a);",
            "const f=a?async o=>{await a;return o.x===b?1:0;}:async o=>{await b;return o.x===b?1:0;};return await f({x:b});",
            "const o={x:a};const g=async n=>{await b;return o.x===n?1:0;};const f=async n=>{const saved=o;const x=await g(n);return saved===o?x:0;};return await f(a);",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn related_gc_frames_preserve_finalizers_rejections_and_jobs_after_root_settlement() {
    compare_promises(
        &[
            "const o={x:a};const f=async n=>{try{await Promise.reject(n);}catch(e){o.x=e;}finally{await b;}return o.x===b?1:0;};const p=f(a);const q=f(b);await p;return await q;",
            "const o={x:a};const f=async n=>{await n;o.x=n;return 1;};f(b);return o.x===a?1:0;",
            "const o={x:a};const f=async n=>{await n;o.x=n;throw n;};const p=f(a);try{return await p;}catch(e){return o.x===e?1:0;}",
            "const o={x:a};const f=async n=>{await n;return Promise.resolve(o.x===n?1:0);};const p=f(a);const q=f(b);await q;return await p;",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn synchronous_helper_starts_preserve_gc_results_and_reentrant_caller_snapshots() {
    compare_async_sources(
        &[
            "function helper(o,n){const f=async x=>{await x;o.x=x;return 1;};f(n);return o;}export async function run(a,b){const o={x:a};const saved=helper(o,b);await a;return saved===o&&o.x===b?1:0;}".into(),
            "export async function run(a,b){const launch=(fn,x)=>fn(x);const o={x:a};let g=async x=>{await x;return o.x===a?x:0;};let f=async x=>{const saved={x};const p=launch(g,x+1);return saved.x===x?await p:0;};const p=launch(f,a),q=launch(f,b);return(await p)+(await q);}".into(),
            "export async function run(a,b){let next=async x=>{await x;return 0;};const launch=(fn,x)=>fn(x);let f=async x=>{const before={x};const saved=next;next=async y=>{await y;return 0;};const p=launch(saved,x+1);const same=before.x===a?1:0;return(await p)+same;};next=f;return await launch(f,a);}".into(),
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn managed_frame_parameters_keep_erased_annotations_and_two_megabyte_stacks() {
    let js = "export async function run(a,b){const o={x:a};const f=async(t,n)=>{await n;t.x=n;return t===o?1:0;};const p=f(o,b);return await p;}";
    let ts = "export async function run(a:string,b:boolean):void{const o:number={x:a};const f=async(t:never,n:string):boolean=>{await(n as Missing);t.x=n;return(t as unknown)===(o as never)?1:0;};const p=f(o,b);return await p;}";
    assert_eq!(
        compile_async(js, false).unwrap().wasm,
        compile_async(ts, true).unwrap().wasm
    );
    for (depth, admitted) in [(16, true), (140, false)] {
        for ts in [false, true] {
            let source = format!(
                "export async function run(a,b){{const o={{x:a}};const f=async(t,n)=>{{await n;return {}t===o{}?1:0;}};const p=f(o,b);return await p;}}",
                "(".repeat(depth),
                ")".repeat(depth)
            );
            let result = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || compile_async(&source, ts))
                .unwrap()
                .join()
                .unwrap();
            assert_eq!(result.is_ok(), admitted, "{:?}", result.as_ref().err());
            if let Err(e) = result {
                assert!(e.contains("limit"), "{e}");
            }
        }
    }
}

#[test]
fn managed_frames_keep_public_payload_and_unknown_input_boundaries_explicit() {
    for body in [
        "const f=async o=>{await a;return o.x;};const p=f(a);return await p;",
        "const f=async o=>{await a;return o.x===b?1:0;};f({x:b});const p=f(a);return await p;",
        "const f=async o=>{await a;return o;};const p=f({});return await p;",
        "const o={};const f=async()=>{await a;throw o;};return await f();",
        "const o={};const f=async()=>{await o;return a;};const p=f();return await p;",
        "const o={};const f=async()=>{await a;return b;};const p=f();await p;return o;",
    ] {
        let source = source_for(body);
        for ts in [false, true] {
            let error = std::panic::catch_unwind(|| compile_async(&source, ts))
                .unwrap()
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

#[test]
fn former_reference_frame_rejections_execute_the_original_sources() {
    compare_promises(
        &[
            "const f=async()=>{await a;return b;};const use=async p=>await p;return await use(f(),{});",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
    compare_async_sources(&[
        "export async function run(a,b){const o={};const f=async x=>{await b;return x;};const p=f(a);await p;return o?1:0;}".into(),
        "export async function run(a,b){const f=async x=>{const o={x};await b;return o.x===x?1:0;};const p=f(a);return await p;}".into(),
        "export async function run(a,b){const o={};const f=()=>o;const g=async x=>{await b;return x;};const p=g(a);await p;return f()===o?1:0;}".into(),
        "export async function run(a,b){const kind=typeof a;const f=async x=>{await b;return x;};const p=f(a);await p;return kind==='number'?a:b;}".into(),
        "function id(x){return x;}export async function run(a,b){const apply=(fn,x)=>fn(x);const o={};const g=async x=>{await b;return x;};const p=g(a);await p;return apply(id,o)===o?1:0;}".into(),
        "function f(x){return x;}export async function run(a,b){const g=f;const h=async x=>{await b;return x;};const p=h(a);await p;return g(a);}".into(),
    ],&[(4.0,2.0),(-0.0,0.0),(f64::NAN,f64::INFINITY)]);
}
