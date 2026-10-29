use super::*;

fn compare_catch_cells(sources: &[&str]) {
    compare_async_sources(
        &sources.iter().map(|s| (*s).to_string()).collect::<Vec<_>>(),
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-2.0, -4.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn catch_cells_accept_later_promise_assignments_without_raw_producers() {
    compare_catch_cells(&[
        "export async function run(a,b){const p=Promise.resolve(a);try{throw b;}catch(e){e=p;return e===p;}}",
    ]);
}

#[test]
fn catch_cells_preserve_primitive_snapshots_and_captured_later_writes() {
    compare_catch_cells(&[
        "export async function run(a,b){const p=Promise.resolve(a);try{throw b;}catch(e){const old=e;e=p;return old===b&&e===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw b;}catch(e){const read=()=>e,old=read();e=p;await b;return old===b&&read()===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw b;}catch(e){e=p;const old=e;e=b;return old===p&&e===b;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw b;}catch(e){const set=x=>{e=x;return x;};set(p);return e===p;}}",
        "async function f(p,x){try{throw x;}catch(e){const read=()=>e;e=p;await x;return read()===p;}}export async function run(a,b){return await f(Promise.resolve(a),b);}",
    ]);
    for primitive in ["undefined", "null", "false", "true", "b", "-0", "0/0"] {
        let source = format!(
            "export async function run(a,b){{const p=Promise.resolve(a);try{{throw {primitive};}}catch(e){{const old=e;e=p;if(e!==p||!(old==={primitive}||(old!==old&&{primitive}!=={primitive})))return a;return +old;}}}}"
        );
        compare_catch_cells(&[&source]);
    }
}

#[test]
fn catch_cells_keep_logical_assignment_selection_and_effects() {
    compare_catch_cells(&[
        "export async function run(a,b){const p=Promise.resolve(a);let n=0;try{throw b;}catch(e){e||=(n=1,p);return n===(b?0:1)&&(b?e===b:e===p);}}",
        "export async function run(a,b){const p=Promise.resolve(a);let n=0;try{throw b;}catch(e){e&&=(n=1,p);return n===(b?1:0)&&(b?e===p:e===b);}}",
        "export async function run(a,b){const p=Promise.resolve(a);let n=0;try{throw(a?undefined:b);}catch(e){e??=(n=1,p);return n===(a?1:0)&&(a?e===p:e===b);}}",
        "export async function run(a,b){let n=0;try{throw b;}catch(e){e||=Promise.resolve(a,n=1);return n===(b?0:1)&&(b?e===b:(await e)===a);}}",
    ]);
}

#[test]
fn catch_cells_save_values_across_assignment_failure_and_finally() {
    compare_catch_cells(&[
        "export async function run(a,b){const p=Promise.resolve(a);try{throw b;}catch(e){const read=()=>e;try{e=(await Promise.reject(a),p);}catch(x){return read()===b;}}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw b;}catch(e){const read=()=>e;e=(await a,p);return read()===p;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw b;}catch(e){e=p;return e;}}",
        "export async function run(a,b){const p=Promise.reject(a);try{throw b;}catch(e){e=p;try{await e;}catch(x){return x===a&&e===p;}}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw b;}catch(e){e=p;try{return e;}finally{await b;e=b;}}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw b;}catch(e){try{return e;}finally{e=p;await a;}}}",
    ]);
}

#[test]
fn catch_cells_erase_types_and_keep_unsupported_storage_domains_closed() {
    let plain = "export async function run(a,b){const p=Promise.resolve(a);try{throw b;}catch(e){const old=e;e=p;await b;return old===b&&e===p;}}";
    let typed = "export async function run(a:never,b:Wrong):Missing{const p:never=Promise.resolve<Wrong>(a as Missing);try{throw(b as never);}catch(e){const old:Wrong=e;e=p as Missing;await(b as never);return(old as Missing)===(b as never)&&(e as Wrong)===(p as Missing);}}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    compare_catch_cells(&[plain]);
    for source in [
        "export async function run(a,b){try{throw b;}catch(e){e={then:a};return e;}}",
        "export async function run(a,b){try{throw b;}catch(e){e=()=>a;return e;}}",
        "export async function run(a,b){try{throw b;}catch(e){e='x';return e;}}",
        "export async function run(a,b){try{throw b;}catch(e){e=missing as Promise<number>;return e;}}",
        "export async function run(a,b){const p=Promise.resolve(a);try{throw b;}catch(e){e=p;return e+b;}}",
    ] {
        assert!(compile_async(source, true).is_err(), "{source}");
    }
}
