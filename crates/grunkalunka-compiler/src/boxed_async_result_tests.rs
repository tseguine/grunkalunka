//! Async returns project only source-proven non-String primitive managed values.

#[test]
fn boxed_primitive_async_returns_admit_existing_scalar_sources() {
    for source in [
        "export async function run(a,b){const box={value:a};await b;return box.value;}",
        "export async function run(a,b){const values=[a,b];await a;return values[1];}",
        "export async function run(a,b){const box={value:a?true:false};await b;return box.value;}",
        "export async function run(a,b){const box={value:a?null:undefined};await b;return box.value;}",
        "export async function run(a,b){const box={yes:null,no:b};await b;return a?box.yes:box.no;}",
        "export async function run(a,b){const box={value:a};const saved=box.value;box.value=b;await b;return saved;}",
        "function get(box){return box.value;}export async function run(a,b){const box={value:a};await b;return get(box);}",
        "function get(box){return box.value;}export async function run(a,b){const box={value:a?null:undefined};await b;return get(box);}",
        "export async function run(a,b){const make=x=>()=>x;const get=make();await b;return get();}",
        "export async function run(a,b){const make=x=>()=>x;const get=make(a?false:null);await b;return get();}",
        "export async function run(a,b){const get=async x=>{const box={value:x};await b;return box.value;};return await get(a);}",
    ] {
        for typescript in [false, true] {
            let result = crate::compile_async(source, typescript);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
}

#[test]
fn boxed_primitive_async_returns_keep_finalizer_and_promise_resolution_order() {
    for source in [
        "export async function run(a,b){const box={value:a};try{return box.value;}finally{box.value=b;}}",
        "export async function run(a,b){const box={value:a?null:undefined};try{return box.value;}finally{await b;box.value=b;}}",
        "export async function run(a,b){const box={value:a};try{return box.value;}finally{throw b;}}",
        "export async function run(a,b){const box={value:a};try{if(b)return Promise.resolve(box.value);return box.value;}finally{box.value=b;}}",
    ] {
        for typescript in [false, true] {
            let result = crate::compile_async(source, typescript);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
}

#[test]
fn boxed_primitive_async_returns_keep_managed_payload_guards() {
    for body in [
        "const box={value:'text'};await b;return box.value;",
        "const box={value:{n:a}};await b;return box.value;",
        "const values=[[a]];await b;return values[0];",
        "const box={value:a?'text':b};await b;return box.value;",
        "const box={value:a};box.value={n:b};await b;return box.value;",
        "const box={value:a};const change=()=>{box.value='text';};change();await b;return box.value;",
        "const make=x=>()=>x;const get=make({n:a});await b;return get();",
        "const box={value:{n:a}};try{return box.value;}finally{await b;}",
        "const box={value:'text'};try{return b?Promise.resolve(a):box.value;}finally{await b;}",
        "const box={value:{n:a}};try{return b?Promise.resolve(a):box.value;}finally{await b;}",
        "const box={value:'text'};return Promise.resolve(box.value);",
    ] {
        let source = format!("export async function run(a,b){{{body}}}");
        for typescript in [false, true] {
            let result = std::panic::catch_unwind(|| crate::compile_async(&source, typescript))
                .unwrap_or_else(|_| panic!("compiler panic: {source}"));
            let error = result
                .err()
                .unwrap_or_else(|| panic!("unexpected admission: {source}"));
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

#[test]
fn boxed_primitive_async_returns_erase_annotations_without_trusting_them() {
    let js = "function get(box){return box.value;}export async function run(a,b){const box={value:a?null:undefined};await b;return get(box);}";
    let ts = "function get(box:never):Missing{return box.value as Missing;}export async function run(a:string,b:boolean):Wrong{const box:never={value:a?null:undefined};await (b as unknown);return get(box) as number;}";
    assert_eq!(
        crate::compile_async(js, false).unwrap().wasm,
        crate::compile_async(ts, true).unwrap().wasm
    );
    assert!(
        crate::compile_async(
            "export async function run(a,b){const box={value:'text'};return box.value as number;}",
            true
        )
        .is_err()
    );
}
