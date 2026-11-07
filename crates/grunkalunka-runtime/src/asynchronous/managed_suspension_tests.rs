use super::*;

const SOURCE: &str = "export async function run(a,b){const o={x:a};const p=o;o.self=o;await b;p.x=b;return o===p&&o.self===p&&o.x===b?1:0;}";

#[test]
fn normal_final_switch_arm_keeps_suffix_roots_through_gc() {
    let source = "export async function run(a,b){const o={x:a};switch(a){case 0:await b;o.x=b;}return +o.x;}";
    let artifact = grunkalunka_compiler::compile_async(source, false).unwrap();
    let mut activation = AsyncInstance::new(&artifact.wasm, 0.0, 2.0).unwrap();
    assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
    activation.store.gc(None).unwrap();
    assert!(activation.run_one_job().unwrap());
    assert!(matches!(
        activation.state().unwrap(),
        AsyncState::Fulfilled(super::super::Value::Number(2.0))
    ));
    assert!(!activation.run_one_job().unwrap());
}

#[test]
fn missed_switch_case_keeps_saved_string_live_until_suffix() {
    let source =
        "export async function run(a,b){switch('a'+''){case(await b,'b'):return 99;}return 1;}";
    let artifact = grunkalunka_compiler::compile_async(source, false).unwrap();
    let mut activation = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
    assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
    activation.store.gc(None).unwrap();
    assert!(activation.run_one_job().unwrap());
    assert!(matches!(
        activation.state().unwrap(),
        AsyncState::Fulfilled(super::super::Value::Number(1.0))
    ));
    assert!(!activation.run_one_job().unwrap());
}

#[test]
fn saved_switch_string_discriminant_survives_gc_during_case_test() {
    let source = "export async function run(a,b){switch('a'+''){case(await b,'a'):return 1;default:return 0;}}";
    let artifact = grunkalunka_compiler::compile_async(source, false).unwrap();
    let mut activation = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
    assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
    activation.store.gc(None).unwrap();
    assert!(activation.run_one_job().unwrap());
    assert!(matches!(
        activation.state().unwrap(),
        AsyncState::Fulfilled(super::super::Value::Number(1.0))
    ));
    assert!(!activation.run_one_job().unwrap());
}

#[test]
fn dictionary_receiver_stays_traced_during_primitive_key_selection() {
    let source = "export async function run(a,b){let d=Object.create(null);d.x=a;const other=Object.create(null);other.x=b;const value=d[(d=other,await b,'x')];return +value+ +d['x'];}";
    let artifact = grunkalunka_compiler::compile_async(source, false).unwrap();
    let mut activation = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
    assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
    activation.store.gc(None).unwrap();
    assert!(activation.run_one_job().unwrap());
    assert!(matches!(
        activation.state().unwrap(),
        AsyncState::Fulfilled(super::super::Value::Number(6.0))
    ));
    assert!(!activation.run_one_job().unwrap());
}

#[test]
fn projected_primitive_keeps_its_tag_and_snapshot_through_gc() {
    let source = "export async function run(a,b){const o={v:false};const value=o.v==(o.v=true,await b,0);return value&&o.v===true?1:0;}";
    let artifact = grunkalunka_compiler::compile_async(source, false).unwrap();
    let mut activation = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
    assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
    activation.store.gc(None).unwrap();
    assert!(activation.run_one_job().unwrap());
    assert!(matches!(
        activation.state().unwrap(),
        AsyncState::Fulfilled(super::super::Value::Number(1.0))
    ));
    assert!(!activation.run_one_job().unwrap());
}

#[test]
fn saved_string_comparison_operand_survives_gc_during_rhs_await() {
    let source = "export async function run(a,b){let text='a'+'';const value=text<(text='z',await b,'b');return value&&text==='z'?1:0;}";
    let artifact = grunkalunka_compiler::compile_async(source, false).unwrap();
    let mut activation = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
    assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
    activation.store.gc(None).unwrap();
    assert!(activation.run_one_job().unwrap());
    assert!(matches!(
        activation.state().unwrap(),
        AsyncState::Fulfilled(super::super::Value::Number(1.0))
    ));
    assert!(!activation.run_one_job().unwrap());
}

#[test]
fn boolean_logical_cells_keep_captured_values_through_gc() {
    let source = "export async function run(a,b){let [flag]=[false];const read=()=>flag;const value=flag||=(await b,true);return read()==value&&value==true?1:0;}";
    let artifact = grunkalunka_compiler::compile_async(source, false).unwrap();
    let mut activation = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
    assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
    activation.store.gc(None).unwrap();
    assert!(activation.run_one_job().unwrap());
    assert!(matches!(
        activation.state().unwrap(),
        AsyncState::Fulfilled(super::super::Value::Number(1.0))
    ));
    assert!(!activation.run_one_job().unwrap());
}

#[test]
fn awaited_boolean_storage_survives_gc_between_reactions() {
    let source = "export async function run(a,b){const o={flag:await true};const same=(await o.flag)==(o.flag=false,true);return same&&o.flag==false?1:0;}";
    let artifact = grunkalunka_compiler::compile_async(source, false).unwrap();
    let mut activation = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
    for _ in 0..2 {
        assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
        activation.store.gc(None).unwrap();
        assert!(activation.run_one_job().unwrap());
    }
    assert!(matches!(
        activation.state().unwrap(),
        AsyncState::Fulfilled(super::super::Value::Number(1.0))
    ));
    assert!(!activation.run_one_job().unwrap());
}

#[test]
fn boolean_loose_equality_keeps_saved_values_through_gc() {
    let source = "export async function run(a,b){const o={flag:true};const same=o.flag==(await b,o.flag=false,true);return same&&o.flag==false?1:0;}";
    let artifact = grunkalunka_compiler::compile_async(source, false).unwrap();
    let mut activation = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
    assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
    activation.store.gc(None).unwrap();
    assert!(activation.run_one_job().unwrap());
    assert!(matches!(
        activation.state().unwrap(),
        AsyncState::Fulfilled(super::super::Value::Number(1.0))
    ));
    assert!(!activation.run_one_job().unwrap());
}

#[test]
fn function_loose_equality_keeps_saved_closures_through_gc() {
    let source = "export async function run(a,b){let f=()=>a;const old=f;const same=f==(await b,f=()=>b,old);return same&&f!=old?1:0;}";
    let artifact = grunkalunka_compiler::compile_async(source, false).unwrap();
    let mut activation = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
    assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
    activation.store.gc(None).unwrap();
    assert!(activation.run_one_job().unwrap());
    assert!(matches!(
        activation.state().unwrap(),
        AsyncState::Fulfilled(super::super::Value::Number(1.0))
    ));
    assert!(!activation.run_one_job().unwrap());
}

#[test]
fn boxed_number_loose_equality_keeps_saved_values_through_collection() {
    let source = "export async function run(a,b){const o={n:a};const same=o.n==(await b,o.n=b,a);return same==(a==a)?1:0;}";
    let artifact = grunkalunka_compiler::compile_async(source, false).unwrap();
    let prepared = PreparedAsyncModule::new(&artifact.wasm).unwrap();
    for a in [-0.0, f64::NAN, f64::INFINITY] {
        let mut activation = prepared.instantiate(a, 2.0).unwrap();
        assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
        activation.store.gc(None).unwrap();
        assert!(activation.run_one_job().unwrap());
        assert!(matches!(
            activation.state().unwrap(),
            AsyncState::Fulfilled(super::super::Value::Number(1.0))
        ));
        assert!(!activation.run_one_job().unwrap());
    }
}

#[test]
fn reference_loose_equality_keeps_saved_references_through_gc() {
    let source = "export async function run(a,b){let o={};const old=o;const same=o==(await a,o={},old);return same&&o!=old?1:0;}";
    let artifact = grunkalunka_compiler::compile_async(source, false).unwrap();
    let mut activation = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
    assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
    activation.store.gc(None).unwrap();
    assert!(activation.run_one_job().unwrap());
    assert!(matches!(
        activation.state().unwrap(),
        AsyncState::Fulfilled(super::super::Value::Number(1.0))
    ));
    assert!(!activation.run_one_job().unwrap());
}

#[test]
fn string_loose_equality_keeps_its_saved_value_through_collection() {
    let source = "export async function run(a,b){const o={s:'old'};const value=o.s==(await a,o.s='new','old');return value&&o.s=='new'?1:0;}";
    let artifact = grunkalunka_compiler::compile_async(source, false).unwrap();
    let mut activation = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
    assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
    activation.store.gc(None).unwrap();
    assert!(activation.run_one_job().unwrap());
    assert!(matches!(
        activation.state().unwrap(),
        AsyncState::Fulfilled(super::super::Value::Number(1.0))
    ));
    assert!(!activation.run_one_job().unwrap());
}

#[test]
fn managed_loose_null_checks_observe_language_tags_after_suspension() {
    let source = "export async function run(a,b){let count=0;for(const value of [null,undefined,false,0,'',{},[]]){await a;if(value==null)count++;}return count;}";
    let artifact = grunkalunka_compiler::compile_async(source, false).unwrap();
    let mut activation = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
    for _ in 0..7 {
        assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
        activation.store.gc(None).unwrap();
        assert!(activation.run_one_job().unwrap());
    }
    assert!(matches!(
        activation.state().unwrap(),
        AsyncState::Fulfilled(super::super::Value::Number(2.0))
    ));
    assert!(!activation.run_one_job().unwrap());
}

#[test]
fn awaited_strings_remain_traced_and_pending_until_their_reaction_jobs() {
    for (body, expected_jobs) in [
        (
            r"const text=await '\ud800\udfff\0';return text==='\ud800\udfff\0'?1:0;",
            1,
        ),
        ("return (await (await 'abc'))==='abc'?1:0;", 2),
        (
            "let current={s:'old'};const old=current;const value=current.s+=await (current={s:'other'},'new');return old.s==='oldnew'&&current.s==='other'&&value===old.s?1:0;",
            1,
        ),
    ] {
        let source = format!("export async function run(a,b){{{body}}}");
        let artifact = grunkalunka_compiler::compile_async(&source, false).unwrap();
        let prepared = PreparedAsyncModule::new(&artifact.wasm).unwrap();
        let mut activation = prepared.instantiate(4.0, 2.0).unwrap();
        for _ in 0..expected_jobs {
            assert!(
                matches!(activation.state().unwrap(), AsyncState::Pending),
                "{source}"
            );
            assert_eq!(activation.pending_job_count().unwrap(), 1);
            activation.store.gc(None).unwrap();
            assert!(activation.run_one_job().unwrap());
        }
        assert!(
            matches!(
                activation.state().unwrap(),
                AsyncState::Fulfilled(super::super::Value::Number(1.0))
            ),
            "{source}"
        );
        assert_eq!(activation.pending_job_count().unwrap(), 0);
        assert!(!activation.run_one_job().unwrap());
    }
}

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

#[test]
fn switch_reference_discriminants_survive_gc_during_rebinding_tests() {
    for body in [
        "let x={};switch(x){case(await b,x={},x):return 0;default:return 1;}",
        "let x=a?{}:'x';switch(x){case(await b,x=null,x):return 0;default:return 1;}",
        "let f=a?()=>a:null;switch(f){case(await b,f=()=>a,f):return 0;default:return 1;}",
        "let x=[];switch(x){case(await b,x=[],x):return 0;default:return 1;}",
        "let f=()=>a;switch(f){case(await b,f=()=>a,f):return 0;default:return 1;}",
        "let f=async()=>a;switch(f){case(await b,f=async()=>a,f):return 0;default:return 1;}",
    ] {
        let source = format!("export async function run(a,b){{{body}}}");
        for typed in [false, true] {
            let artifact = grunkalunka_compiler::compile_async(&source, typed).unwrap();
            let mut activation = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
            assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
            activation.store.gc(None).unwrap();
            assert!(activation.run_one_job().unwrap());
            assert!(matches!(
                activation.state().unwrap(),
                AsyncState::Fulfilled(super::super::Value::Number(1.0))
            ));
            assert!(!activation.run_one_job().unwrap());
        }
    }
}

#[test]
fn switch_break_runs_awaited_finally_with_traced_suffix_roots() {
    let source = "export async function run(a,b){const o={x:a};switch(a){case 0:try{break;}finally{await b;o.x=b;}default:o.x=a;}return +o.x;}";
    for typed in [false, true] {
        let artifact = grunkalunka_compiler::compile_async(source, typed).unwrap();
        let mut activation = AsyncInstance::new(&artifact.wasm, 0.0, 2.0).unwrap();
        assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
        activation.store.gc(None).unwrap();
        assert!(activation.run_one_job().unwrap());
        assert!(matches!(
            activation.state().unwrap(),
            AsyncState::Fulfilled(super::super::Value::Number(2.0))
        ));
        assert!(!activation.run_one_job().unwrap());
    }
}

#[test]
fn switch_fallthrough_keeps_selection_and_suffix_roots_through_repeated_gc() {
    let source = "export async function run(a,b){const o={x:a};switch(a){case 0:await b;o.x=b;default:await a;o.x=+o.x+1;}return +o.x;}";
    for typed in [false, true] {
        let artifact = grunkalunka_compiler::compile_async(source, typed).unwrap();
        for (a, jobs, expected) in [(0.0, 2, 3.0), (1.0, 1, 2.0)] {
            let mut activation = AsyncInstance::new(&artifact.wasm, a, 2.0).unwrap();
            for _ in 0..jobs {
                assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
                activation.store.gc(None).unwrap();
                assert!(activation.run_one_job().unwrap());
            }
            assert!(
                matches!(activation.state().unwrap(),AsyncState::Fulfilled(super::super::Value::Number(value)) if value==expected)
            );
            assert!(!activation.run_one_job().unwrap());
        }
    }
}

#[test]
fn omitted_helper_fallback_keeps_returned_managed_root_through_gc() {
    let source = "function choose(x,y){return y??x;}export async function run(a,b){const selected=choose({v:a});await b;return +selected.v;}";
    for typed in [false, true] {
        let artifact = grunkalunka_compiler::compile_async(source, typed).unwrap();
        let mut activation = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
        assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
        activation.store.gc(None).unwrap();
        assert!(activation.run_one_job().unwrap());
        assert!(matches!(
            activation.state().unwrap(),
            AsyncState::Fulfilled(super::super::Value::Number(4.0))
        ));
        assert!(!activation.run_one_job().unwrap());
    }
}

#[test]
fn omitted_arrow_fallback_keeps_returned_managed_root_through_gc() {
    let source = "export async function run(a,b){const choose=(x,y)=>y??x;const selected=choose({v:a});await b;return +selected.v;}";
    for typed in [false, true] {
        let artifact = grunkalunka_compiler::compile_async(source, typed).unwrap();
        let mut activation = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
        assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
        activation.store.gc(None).unwrap();
        assert!(activation.run_one_job().unwrap());
        assert!(matches!(
            activation.state().unwrap(),
            AsyncState::Fulfilled(super::super::Value::Number(4.0))
        ));
        assert!(!activation.run_one_job().unwrap());
    }
}

#[test]
fn omitted_runtime_fallback_keeps_returned_managed_root_through_gc() {
    let source = "export async function run(a,b){let choose=(x,y)=>y??x;const selected=choose({v:a});await b;return +selected.v;}";
    for typed in [false, true] {
        let artifact = grunkalunka_compiler::compile_async(source, typed).unwrap();
        let mut activation = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
        assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
        activation.store.gc(None).unwrap();
        assert!(activation.run_one_job().unwrap());
        assert!(matches!(
            activation.state().unwrap(),
            AsyncState::Fulfilled(super::super::Value::Number(4.0))
        ));
        assert!(!activation.run_one_job().unwrap());
    }
}

#[test]
fn factory_created_local_result_roots_survive_collection() {
    for body in [
        "const f=x=>()=>x;const g=f({v:a});await b;return +g().v;",
        "const f=x=>()=>x;const g=f([a]);const saved=g();await b;return +saved[0];",
    ] {
        let source = format!("export async function run(a,b){{{body}}}");
        for typed in [false, true] {
            let artifact = grunkalunka_compiler::compile_async(&source, typed).unwrap();
            let mut activation = AsyncInstance::new(&artifact.wasm, 4.0, 2.0).unwrap();
            assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
            activation.store.gc(None).unwrap();
            assert!(activation.run_one_job().unwrap());
            assert!(matches!(
                activation.state().unwrap(),
                AsyncState::Fulfilled(super::super::Value::Number(4.0))
            ));
            assert!(!activation.run_one_job().unwrap());
        }
    }
}
