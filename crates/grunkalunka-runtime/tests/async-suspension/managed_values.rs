use super::*;

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
