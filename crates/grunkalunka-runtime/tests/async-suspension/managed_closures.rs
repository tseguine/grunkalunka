use super::*;

#[test]
fn managed_closures_keep_capture_identity_across_await() {
    compare_promises(
        &["const o={};const f=()=>o;await a;return f()===o?1:0;"],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn previous_capture_and_module_identity_rejections_execute_unchanged() {
    compare_async_sources(
        &[
            "export async function run(a,b){const o={};let f=()=>o===o?a:b;await 0;return f();}".into(),
            "export async function run(a,b){let make=n=>{const x=[n];return ()=>x.length;};const f=make(a);await 0;return f();}".into(),
            "function id(x){return x;}export async function run(a,b){const apply=(fn,x)=>fn(x);const o={};await 0;return apply(id,o)===o?1:0;}".into(),
            "function f(x){return x;}export async function run(a,b){const g=f;await 0;return g(a);}".into(),
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn managed_capture_cells_keep_rebinding_mutation_cycles_and_saved_values() {
    compare_promises(
        &[
            "let o={x:a};const saved=o;const f=()=>o;await b;o={x:b};return f()===o&&f()!==saved?1:0;",
            "const o={x:a};const f=()=>o.x;await b;o.x=b;return f()===b?1:0;",
            "let s='old';const f=()=>s;const saved=f();await b;s=a?'new':'';return f()===s&&saved==='old'?1:0;",
            "const xs=[a,b];const f=()=>xs;await b;f()[0]=b;return f()===xs&&xs[0]===b?1:0;",
            "const o={};const f=()=>o;o.f=f;await b;return o.f===f&&f()===o?1:0;",
            "const o={};const f=()=>a;const box={o,f};await b;return box.f();",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn suspended_managed_function_values_keep_typed_calls_and_factory_environments() {
    compare_async_sources(
        &[
            "export async function run(a,b){const make=o=>()=>o;const o={x:a};const f=make(o);await b;return f()===o?1:0;}".into(),
            "function identity(o){return o;}export async function run(a,b){const apply=(fn,x)=>fn(x);const o={x:a};await b;return apply(identity,o)===o?1:0;}".into(),
            "function identity(o){return o;}export async function run(a,b){const o={x:a};const f=identity;const box={f};await b;return box.f(o)===o?1:0;}".into(),
            "export async function run(a,b){const o={};let f=()=>o;const saved=f;const box={f};await b;f=()=>o;return saved!==f&&box.f===saved&&box.f()===o?1:0;}".into(),
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn managed_closures_preserve_handlers_finalizers_and_primitive_settlements() {
    compare_promises(
        &[
            "const o={x:a};const f=()=>o.x;try{await Promise.reject(b);}catch(e){o.x=e;}finally{await a;}return f()===b?1:0;",
            "const o={x:a};const f=()=>o;try{await b;return f()===o?a:b;}finally{await a;o.x=b;}",
            "const o={x:a};const f=()=>o;await b;return Promise.resolve(f()===o?a:b);",
            "const o={x:a};const f=()=>o;await b;return Promise.reject(f()===o?a:b);",
            "const o={x:a};const f=()=>o;await b;try{throw a;}catch(e){await b;return f()===o?e:b;}",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn returned_closure_environments_keep_captured_values_after_factory_return() {
    compare_promises(
        &[
            "const make=o=>()=>o.x;const f=make({x:a});const keep={f};await b;return f()===a?1:0;",
            "const make=x=>{const o={x};const f=()=>o.x;o.f=f;o.self=o;return f;};const f=make(a);const keep={f};await b;return f()===a?1:0;",
            "const make=s=>()=>s;const f=make('text');const keep={f};await b;return f()==='text'?1:0;",
            "const make=x=>{let o={x};return()=>{const saved=o;o={x:b};return saved!==o?1:0;};};const f=make(a);const keep={f};await b;return f();",
        ],
        &[(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn managed_closure_suspension_preserves_erased_types_and_small_stack_limits() {
    let js =
        "export async function run(a,b){const o={x:a};const f=()=>o;await b;return f()===o?1:0;}";
    let ts = "export async function run(a:string,b:boolean):void{const o:any={x:a};const f:never=()=>o;await(b as unknown);return(f()as Missing)===(o as never)?1:0;}";
    assert_eq!(
        compile_async(js, false).unwrap().wasm,
        compile_async(ts, true).unwrap().wasm
    );
    for (depth, admitted) in [(16, true), (140, false)] {
        for ts in [false, true] {
            let source = format!(
                "export async function run(a,b){{const o={{x:a}};const f=()=>{}o{};await b;return f()===o?1:0;}}",
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
            if let Err(error) = result {
                assert!(error.contains("limit"), "{error}");
            }
        }
    }
}

#[test]
fn managed_closures_keep_managed_settlement_and_public_reference_boundaries() {
    for body in [
        "const o={};const f=async()=>{await b;return o;};const p=f();await p;return a;",
        "const o={};const f=()=>o;await a;return f;",
        "const o={};const f=()=>o;await a;return f();",
        "const o={};const f=()=>o;await f();return a;",
        "const o={};const f=()=>o;await a;throw f();",
    ] {
        for ts in [false, true] {
            let source = source_for(body);
            assert!(compile_async(&source, ts).is_err(), "admitted: {source}");
        }
    }
}
