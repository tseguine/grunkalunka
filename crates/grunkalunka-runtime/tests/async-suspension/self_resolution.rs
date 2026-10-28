use super::*;

#[test]
fn self_resolution_typeerror_enters_outer_await_catch() {
    compare_promises(
        &[
            "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();try{return await p;}catch(e){return a;}",
        ],
        &[(4.0, 2.0), (0.0, -0.0)],
    );
}

const CYCLE: &str =
    "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();";

#[test]
fn self_resolution_typeerror_preserves_catch_rethrow_and_object_operations() {
    let bodies = [
        "return await p;",
        "return p;",
        "try{return await p;}catch(e){return e;}",
        "try{await p;}catch(e){throw e;}",
        "try{await p;}catch(e){await a;throw e;}",
        "let first=null;try{await p;}catch(e){first=e;}try{await p;}catch(e){return e===first;}",
        "try{await p;}catch(e){return !!e;}",
        "try{await p;}catch(e){return e??a;}",
        "try{await p;}catch(e){return +e;}",
        "try{await p;}catch(e){const coerce=x=>+x;return coerce(e);}",
        "try{await p;}catch(e){return e==e;}",
        "try{await p;}catch(e){return e==null;}",
        "try{await p;}catch(e){return e===a;}",
        "try{await p;}catch(e){const use=async x=>x;return await use(e);}",
    ]
    .map(|tail| format!("{CYCLE}{tail}"));
    let refs = bodies.iter().map(String::as_str).collect::<Vec<_>>();
    compare_promises(&refs, &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)]);
}

#[test]
fn self_resolution_typeerror_has_distinct_object_identity_and_no_numeric_alias() {
    compare_promises(
        &[
            "const f=async()=>{await a;return b;};let p=f();let q=f();const g=async()=>{await a;return p;};const h=async()=>{await b;return q;};p=g();q=h();let first=null;try{await p;}catch(e){first=e;}try{await q;}catch(e){return e!==first;}",
        ],
        &[(4.0, 2.0), (0.0, -0.0)],
    );
    let source = source_for(&format!(
        "{CYCLE}try{{await p;}}catch(e){{if(b)return e;return e==a;}}"
    ));
    let bytes = compile_async(&source, false).unwrap().wasm;
    let mut run = AsyncInstance::new(&bytes, 4.0, 1.0).unwrap();
    while run.run_one_job().unwrap() {}
    let AsyncState::Fulfilled(Value::TypeError { id }) = run.state().unwrap() else {
        panic!("error identity must escape");
    };
    compare_async_sources(&[source], &[(f64::from(id), 0.0)]);
}
