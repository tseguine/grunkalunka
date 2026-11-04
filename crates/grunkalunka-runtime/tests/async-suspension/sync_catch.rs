use super::*;

fn compare_sync_catches(sources: &[&str]) {
    compare_async_sources(
        &sources
            .iter()
            .map(|source| (*source).to_string())
            .collect::<Vec<_>>(),
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-2.0, -4.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn async_callers_compose_with_synchronous_helpers_that_catch_locally() {
    compare_sync_catches(&[
        "function recover(x){try{throw x;}catch(e){return +e;}}export async function run(a,b){const first=recover(a);await b;return first+recover(b);}",
        "export async function run(a,b){const recover=x=>{try{throw x;}catch(e){return e===null?7:+e;}};const first=recover(a?null:b);await a;return first+recover(a);}",
        "export async function run(a,b){let n=0;const recover=()=>{try{n++;throw a;}catch(e){n=n*10+2;return +e;}};const first=recover();await b;return first+n+recover();}",
        "function recover(x){try{throw x;}catch(e){return +e;}}export async function run(a,b){try{const value=recover(a);await b;return value;}catch(e){return 99;}}",
        "export async function run(a,b){try{throw b;}catch(outer){const recover=()=>{try{throw a;}catch(inner){return +inner;}};await a;return recover()+(+outer);}}",
        "export async function run(a,b){const recover=()=>{try{return a;}finally{b;}};return recover();}",
    ]);
}

#[test]
fn async_callers_retain_synchronous_catch_cells_without_authorizing_escaping_throws() {
    compare_sync_catches(&[
        "export async function run(a,b){const make=x=>{try{throw x;}catch(e){return()=>+e;}};const first=make(a);await b;const last=make(b);await a;return first()+last();}",
        "export async function run(a,b){let first=()=>0,last=()=>0,change=()=>0;const populate=()=>{for(let i=0;i<2;i++){try{throw i;}catch(e){const read=()=>+e;if(i===0){first=read;change=()=>{e=9;return 0;};}last=read;}}return 0;};populate();await a;change();await b;return first()*10+last();}",
    ]);
    for source in [
        "export async function run(a,b){const fail=()=>{if(a)throw b;return 0;};try{return fail();}catch(e){return e;}}",
        "export async function run(a,b){try{const fail=()=>{if(a)throw b;return 0;};return fail();}catch(e){return e;}}",
        "export async function run(a,b){try{const fail=()=>{try{throw a;}catch(e){if(a)throw e;return 0;}};return fail();}catch(e){return e;}}",
        "function fail(x){if(x)throw x;return 0;}export async function run(a,b){try{return fail(a);}catch(e){return e;}}",
    ] {
        for typed in [false, true] {
            let error = compile_async(source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(error.contains("unsupported"), "{source}: {error}");
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

#[test]
fn synchronous_helpers_keep_primitive_reasons_inside_raw_async_modules() {
    compare_sync_catches(&[
        "export async function run(a,b){const p=Promise.resolve(a);const recover=()=>{try{throw b;}catch(e){return +e;}};try{throw p;}catch(e){return(e===p?1:0)+recover();}}",
        "export async function run(a,b){const p=Promise.resolve(a);const recover=a?(()=>{try{throw b;}catch(e){return +e;}}):(()=>{try{throw a;}catch(e){return +e;}});try{throw p;}catch(e){await b;return(e===p?1:0)+recover();}}",
    ]);
    for source in [
        "export async function run(a,b){const p=Promise.resolve(a);const recover=()=>{try{throw p;}catch(e){return e===p?1:0;}};return recover();}",
        "export async function run(a,b){const p=Promise.resolve(a);const recover=a?(()=>{try{throw p;}catch(e){return e===p?1:0;}}):(()=>{try{throw p;}catch(e){return e===p?2:0;}});return recover();}",
    ] {
        for typed in [false, true] {
            let error = compile_async(source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(
                error.contains("synchronous throw requires a represented primitive reason"),
                "{source}: {error}"
            );
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}
