use super::*;

const SOURCE: &str = "export async function run(a,b){const o={x:a};const p=o;o.self=o;await b;p.x=b;return o===p&&o.self===p&&o.x===b?1:0;}";

#[test]
fn generated_managed_references_survive_copying_collection_while_pending() {
    for source in [
        SOURCE,
        "export async function run(a,b){const o={x:a};try{await b;return 1;}finally{await a;o.x=b;if(o.x!==b)throw 0;}}",
        "export async function run(a,b){const xs=[a,,b];const saved=xs;await b;xs[1]=b;return saved===xs&&xs.length===3&&xs[1]===b?1:0;}",
        "export async function run(a,b){const s='😀\\uD800';const o={s};await b;return o.s===s?1:0;}",
    ] {
        for ts in [false, true] {
            let artifact = grunkalunka_compiler::compile_async(source, ts).unwrap();
            let prepared = PreparedAsyncModule::new(&artifact.wasm).unwrap();
            let mut activation = prepared.instantiate(4.0, 2.0).unwrap();
            assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
            let jobs = activation.pending_job_count().unwrap();
            assert!(jobs > 0);
            for _ in 0..4 {
                activation.store.gc(None).unwrap();
                assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
                assert_eq!(activation.pending_job_count().unwrap(), jobs);
            }
            loop {
                if matches!(activation.state().unwrap(), AsyncState::Pending) {
                    activation.store.gc(None).unwrap();
                }
                if !activation.run_one_job().unwrap() {
                    break;
                }
            }
            assert!(matches!(
                activation.state().unwrap(),
                AsyncState::Fulfilled(super::super::Value::Number(1.0))
            ));
            assert!(!activation.run_one_job().unwrap());
            assert!(
                activation
                    .resume
                    .call(&mut activation.store, (3, 0.0))
                    .is_err()
            );
        }
    }
}

// Export only the generated reference globals for test observation. The
// production instructions, control flow and reference storage stay unchanged.
pub(super) fn root_exports(wat: &str) -> (Vec<u8>, Vec<String>) {
    let mut names = Vec::new();
    let mut exports = String::new();
    for (index, declaration) in wat
        .lines()
        .filter(|line| line.starts_with("(global "))
        .enumerate()
    {
        if declaration.starts_with("(global (mut (ref null ") {
            let name = format!("root{index}");
            exports.push_str(&format!("(export \"{name}\" (global {index}))\n"));
            names.push(name);
        }
    }
    assert!(!names.is_empty());
    let end = wat.rfind(')').unwrap();
    let observed = format!("{}{}{}", &wat[..end], exports, &wat[end..]);
    (wat::parse_str(observed).unwrap(), names)
}

#[test]
fn suspended_closures_trace_environments_after_unrelated_roots_are_removed() {
    for body in [
        "const make=o=>()=>o.x;const f=make({x:a});const keep={f};await b;return f()===a?1:0;",
        "const make=x=>{const o={x};const f=()=>o.x;o.f=f;o.self=o;return f;};const f=make(a);const keep={f};await b;return f()===a?1:0;",
        "const make=s=>()=>s;const f=make('text');const keep={f};await b;return f()==='text'?1:0;",
        "const make=x=>{let o={x};return()=>{const saved=o;o={x:b};return saved!==o?1:0;};};const f=make(a);const keep={f};await b;return f();",
    ] {
        let source = format!("export async function run(a,b){{{body}}}");
        for ts in [false, true] {
            let artifact = grunkalunka_compiler::compile_async(&source, ts).unwrap();
            let (wasm, names) = root_exports(&artifact.wat);
            let prepared = PreparedAsyncModule::new(&wasm)
                .unwrap_or_else(|error| panic!("{source}: {error:?}"));
            let mut store = Store::new(&prepared.engine, ());
            store.set_fuel(super::super::FUEL_PER_CALL).unwrap();
            let instance = Instance::new(&mut store, &prepared.module, &[]).unwrap();
            let declarations: Vec<_> = artifact
                .wat
                .lines()
                .filter(|line| line.starts_with("(global "))
                .collect();
            let roots: Vec<_> = names
                .iter()
                .map(|name| {
                    let index = name.strip_prefix("root").unwrap().parse::<usize>().unwrap();
                    (
                        instance.get_global(&mut store, name).unwrap(),
                        declarations[index].starts_with("(global (mut (ref null $gc_function)"),
                    )
                })
                .collect();
            let mut activation =
                AsyncInstance::start_with_instance(store, instance, 4.0, 2.0).unwrap();
            assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
            assert!(roots.iter().any(|(_, function)| *function), "{source}");
            // Test-only root removal leaves actual emitted closure/environment/
            // cell edges as the sole path to captured objects and Strings.
            for (root, function) in &roots {
                if !function {
                    root.set(&mut activation.store, wasmtime::Val::AnyRef(None))
                        .unwrap();
                }
            }
            for _ in 0..4 {
                activation.store.gc(None).unwrap();
                assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
            }
            while activation.run_one_job().unwrap() {}
            assert!(matches!(
                activation.state().unwrap(),
                AsyncState::Fulfilled(super::super::Value::Number(1.0))
            ));
            assert!(roots.iter().all(|(root, _)| matches!(
                root.get(&mut activation.store),
                wasmtime::Val::AnyRef(None)
            )));
        }
    }
}

#[test]
fn terminal_success_and_rejection_clear_generated_reference_globals() {
    for body in [
        "const o={};o.self=o;await b;return Promise.resolve(a);",
        "const o={};o.self=o;await b;return Promise.reject(a);",
        "const o={};o.self=o;await b;return;",
        "const o={};o.self=o;await b;",
        "const o={};o.self=o;await b;return o.self===o?1:0;",
        "const o={};o.self=o;await b;throw a;",
        "const o={};o.self=o;await Promise.resolve(b);return o.self===o?1:0;",
        "const o={};o.self=o;await Promise.resolve(b);throw a;",
        "const o={};const f=()=>o;o.f=f;await b;return f()===o?a:b;",
        "const o={};const f=()=>o;o.f=f;await b;throw a;",
        "const o={};const f=()=>o;await b;return Promise.resolve(f()===o?a:b);",
        "const o={};const f=()=>o;await b;return Promise.reject(f()===o?a:b);",
    ] {
        let source = format!("export async function run(a,b){{{body}}}");
        let artifact = grunkalunka_compiler::compile_async(&source, false).unwrap();
        let (wasm, names) = root_exports(&artifact.wat);
        let prepared = PreparedAsyncModule::new(&wasm).unwrap();
        let mut store = Store::new(&prepared.engine, ());
        store.set_fuel(super::super::FUEL_PER_CALL).unwrap();
        let instance = Instance::new(&mut store, &prepared.module, &[]).unwrap();
        let roots: Vec<_> = names
            .iter()
            .map(|name| instance.get_global(&mut store, name).unwrap())
            .collect();
        let mut activation = AsyncInstance::start_with_instance(store, instance, 4.0, 2.0).unwrap();
        assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
        assert!(
            roots.iter().any(|root| !matches!(
                root.get(&mut activation.store),
                wasmtime::Val::AnyRef(None)
            ))
        );
        while activation.run_one_job().unwrap() {}
        assert!(!matches!(activation.state().unwrap(), AsyncState::Pending));
        assert!(
            roots
                .iter()
                .all(|root| matches!(root.get(&mut activation.store), wasmtime::Val::AnyRef(None)))
        );
        activation.store.gc(None).unwrap();
        assert!(!activation.run_one_job().unwrap());
    }
}

#[test]
fn uncaught_await_rejection_releases_terminal_guest_reference_globals() {
    let source =
        "export async function run(a,b){const o={};o.self=o;await Promise.reject(b);return a;}";
    let artifact = grunkalunka_compiler::compile_async(source, false).unwrap();
    let (wasm, names) = root_exports(&artifact.wat);
    let prepared = PreparedAsyncModule::new(&wasm).unwrap();
    let mut store = Store::new(&prepared.engine, ());
    store.set_fuel(super::super::FUEL_PER_CALL).unwrap();
    let instance = Instance::new(&mut store, &prepared.module, &[]).unwrap();
    let roots: Vec<_> = names
        .iter()
        .map(|name| instance.get_global(&mut store, name).unwrap())
        .collect();
    let mut activation = AsyncInstance::start_with_instance(store, instance, 4.0, 2.0).unwrap();
    while activation.run_one_job().unwrap() {}
    assert!(matches!(
        activation.state().unwrap(),
        AsyncState::Rejected(super::super::Value::Number(2.0))
    ));
    assert!(
        roots
            .iter()
            .all(|root| matches!(root.get(&mut activation.store), wasmtime::Val::AnyRef(None)))
    );
}

#[test]
fn trapped_managed_invocations_are_terminal_until_store_teardown() {
    let source = "export async function run(a,b){const o={};o.self=o;await b;while(o){}}";
    let artifact = grunkalunka_compiler::compile_async(source, false).unwrap();
    let mut activation = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
    activation.store.gc(None).unwrap();
    assert!(activation.run_one_job().is_err());
    assert!(activation.run_one_job().is_err());
    assert!(activation.pending_job_count().is_err());
    drop(activation);
    // A new isolated invocation still has its own valid state and roots.
    let artifact = grunkalunka_compiler::compile_async(SOURCE, false).unwrap();
    let mut fresh = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
    while fresh.run_one_job().unwrap() {}
    assert!(matches!(
        fresh.state().unwrap(),
        AsyncState::Fulfilled(super::super::Value::Number(1.0))
    ));
}
