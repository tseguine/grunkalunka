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
fn represented_errors_keep_identity_when_boxed_in_gc_containers() {
    let bodies = [
        "try{await p;}catch(e){const o={e};return o.e===e?1:0;}return 0;",
        "try{await p;}catch(e){const xs=[e];return xs[0]===e&&xs[0]!==null&&xs[0]!==undefined&&xs[0]!==a?1:0;}return 0;",
        "try{await p;}catch(e){const o={e};return typeof o.e==='object'&&!!o.e&&(o.e??a)===e?1:0;}return 0;",
        "try{await p;}catch(e){const o={e};o.self=o;const saved=o.e;const read=()=>o.e;await b;return read()===e&&saved===e&&o.self===o?1:0;}return 0;",
    ].map(|tail|format!("{CYCLE}{tail}"));
    let sources = bodies.iter().map(String::as_str).collect::<Vec<_>>();
    compare_promises(
        &sources,
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn separately_boxed_errors_keep_distinct_identity_through_suspension() {
    compare_promises(
        &[
            "const f=async()=>{await a;return b;};let p=f(),q=f();const g=async()=>{await a;return p;};const h=async()=>{await b;return q;};p=g();q=h();const saved={};try{await p;}catch(e){saved.first=e;}try{await q;}catch(e){return saved.first!==e?1:0;}return 0;",
        ],
        &[(4.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn boxed_errors_preserve_erased_annotations_and_payload_rejections() {
    let js = format!(
        "export async function run(a,b){{{CYCLE}try{{await p;}}catch(e){{const o={{e}};await b;return o.e===e?1:0;}}return 0;}}"
    );
    let ts = js
        .replace("run(a,b)", "run(a:string,b:never):void")
        .replace("const o={e}", "const o:number={e}")
        .replace("o.e===e", "(o.e as Missing)===(e as unknown)");
    assert_eq!(
        compile_async(&js, false).unwrap().wasm,
        compile_async(&ts, true).unwrap().wasm
    );
    for tail in [
        "try{await p;}catch(e){const o={e};throw o.e;}return 0;",
        "try{await p;}catch(e){const o={e};return o.e;}return 0;",
        "try{await p;}catch(e){const o={e};await o.e;return 1;}return 0;",
    ] {
        let source = source_for(&format!("{CYCLE}{tail}"));
        for ts in [false, true] {
            let error = compile_async(&source, ts)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

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
