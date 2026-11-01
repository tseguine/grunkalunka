use super::*;

const SHARED: &str = "export async function run(a,b){const o={x:a};const f=async n=>{await n;o.x=n;return 1;};const p=f(a);const q=f(b);await p;await q;return o.x===b?1:0;}";

fn prepare(source: &str, ts: bool) -> (PreparedAsyncModule, Vec<String>) {
    let artifact = grunkalunka_compiler::compile_async(source, ts).unwrap();
    assert!(artifact.wat.contains("(type $async_root "));
    let end = artifact.wat.rfind(')').unwrap();
    let mut wat = artifact.wat[..end].to_owned();
    wat.push_str(
        r#"
      (func (export "roots_live") (result i32) (local $i i32) (local $count i32)
        global.get $async_root_map ref.is_null if i32.const 0 return end
        block $done loop $next
          local.get $i global.get $async_root_map array.len i32.ge_u br_if $done
          global.get $async_root_map local.get $i array.get $async_roots ref.is_null i32.eqz
          if local.get $count i32.const 1 i32.add local.set $count end
          local.get $i i32.const 1 i32.add local.set $i br $next
        end end local.get $count)
      (func (export "first_root") (result i32) (local $i i32)
        block $done loop $next
          local.get $i global.get $async_root_map array.len i32.ge_u br_if $done
          global.get $async_root_map local.get $i array.get $async_roots ref.is_null i32.eqz
          if local.get $i i32.const 4 i32.shl return end
          local.get $i i32.const 1 i32.add local.set $i br $next
        end end i32.const -1)
      (func (export "root_owner") (param i32) (result i32)
        global.get $async_root_map local.get 0 call $async_root_index array.get $async_roots
        struct.get $async_root 0)
      (func (export "root_capacity") (result i32) global.get $async_root_map array.len)
      (func (export "check_root") (param i32 i32)
        local.get 0 local.get 1 call $async_roots_get drop)
      (func (export "new_root") (param i32 i32 i32)
        local.get 0 local.get 1 local.get 2 ref.null $gc_frame call $async_roots_new)
    )"#,
    );
    let (wasm, names) = super::managed_suspension_tests::root_exports(&wat);
    (PreparedAsyncModule::new(&wasm).unwrap(), names)
}

#[test]
fn related_frames_survive_copying_gc_with_only_saved_activation_roots() {
    for source in [
        SHARED,
        "export async function run(a,b){const f=async n=>{const o={x:n};o.self=o;await b;return o.self===o&&o.x===n?1:0;};const p=f(a);const q=f(b);return(await p)===1&&(await q)===1?1:0;}",
        "export async function run(a,b){const o={x:a};const f=a?async n=>{await b;return o.x===n?1:0;}:async n=>{await a;return o.x===n?1:0;};return await f(a);}",
        "export async function run(a,b){const f=a?async o=>{await a;return o.x===b?1:0;}:async o=>{await b;return o.x===b?1:0;};return await f({x:b});}",
        "export async function run(a,b){const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();try{await p;}catch(e){const o={e};o.self=o;const read=()=>o.e;await b;return read()===e&&o.self===o?1:0;}return 0;}",
        "export async function run(a,b){const source={x:a};source.self=source;const f=async()=>{await b;return 1;};const p=f();const copy={...source,...(await p,source)};await b;return copy.self===source&&copy.x===a?1:0;}",
        "export async function run(a,b){const source={x:a};const f=async()=>{await b;return 1;};const p=f();const k=Object.keys(a?source:null);await p;source.y=b;return k.length===1&&k[0]==='x'&&Object.keys(source).length===2?1:0;}",
        "export async function run(a,b){const f=async()=>{await b;return 1;};const p=f();try{Object.keys(null);}catch(e){const o={e};const k=Object.keys(e);await p;return o.e===e&&k.length===0?1:0;}return 0;}",
    ] {
        for ts in [false, true] {
            let (prepared, names) = prepare(source, ts);
            let mut store = Store::new(&prepared.engine, ());
            store.set_fuel(super::super::FUEL_PER_CALL).unwrap();
            let instance = Instance::new(&mut store, &prepared.module, &[]).unwrap();
            let roots: Vec<_> = names
                .iter()
                .map(|n| instance.get_global(&mut store, n).unwrap())
                .collect();
            let live = instance
                .get_typed_func::<(), i32>(&mut store, "roots_live")
                .unwrap();
            let capacity = instance
                .get_typed_func::<(), i32>(&mut store, "root_capacity")
                .unwrap();
            let mut activation =
                AsyncInstance::start_with_instance(store, instance, 4.0, 2.0).unwrap();
            assert!(matches!(activation.state().unwrap(), AsyncState::Pending));
            assert!(live.call(&mut activation.store, ()).unwrap() >= 2);
            assert!(capacity.call(&mut activation.store, ()).unwrap() <= 65536);
            let mut jobs = 0;
            while activation.pending_job_count().unwrap() > 0 {
                for root in &roots {
                    root.set(&mut activation.store, wasmtime::Val::AnyRef(None))
                        .unwrap();
                }
                for _ in 0..3 {
                    activation.store.gc(None).unwrap();
                }
                assert!(
                    activation
                        .run_one_job()
                        .unwrap_or_else(|error| panic!("{source}: {error:?}")),
                    "{source}"
                );
                jobs += 1;
                assert!(jobs < 32);
            }
            assert!(matches!(
                activation.state().unwrap(),
                AsyncState::Fulfilled(super::super::Value::Number(1.0))
            ));
            assert_eq!(live.call(&mut activation.store, ()).unwrap(), 0);
            assert!(roots.iter().all(|root| matches!(
                root.get(&mut activation.store),
                wasmtime::Val::AnyRef(None)
            )));
            assert!(
                activation
                    .resume
                    .call(&mut activation.store, (3, 0.0))
                    .is_err()
            );
        }
    }
}

#[test]
fn root_completion_releases_only_its_frame_while_owned_child_jobs_keep_roots() {
    let source = "export async function run(a,b){const o={x:a};const f=async n=>{await n;o.x=n;if(o.x!==n)throw 0;return 1;};f(b);return 1;}";
    let (prepared, names) = prepare(source, false);
    let mut store = Store::new(&prepared.engine, ());
    store.set_fuel(super::super::FUEL_PER_CALL).unwrap();
    let instance = Instance::new(&mut store, &prepared.module, &[]).unwrap();
    let roots: Vec<_> = names
        .iter()
        .map(|n| instance.get_global(&mut store, n).unwrap())
        .collect();
    let live = instance
        .get_typed_func::<(), i32>(&mut store, "roots_live")
        .unwrap();
    let mut activation = AsyncInstance::start_with_instance(store, instance, 4.0, 2.0).unwrap();
    assert!(matches!(
        activation.state().unwrap(),
        AsyncState::Fulfilled(super::super::Value::Number(1.0))
    ));
    assert_eq!(live.call(&mut activation.store, ()).unwrap(), 1);
    assert!(activation.pending_job_count().unwrap() > 0);
    for root in &roots {
        root.set(&mut activation.store, wasmtime::Val::AnyRef(None))
            .unwrap();
    }
    activation.store.gc(None).unwrap();
    while activation.run_one_job().unwrap() {}
    assert_eq!(live.call(&mut activation.store, ()).unwrap(), 0);
}

#[test]
fn uncaught_root_rejection_releases_its_saved_activation_record() {
    let source = "export async function run(a,b){const o={x:a};const f=a?async n=>{await n;return o.x===n?1:0;}:async n=>{await n;return o.x===n?1:0;};await f(a);await Promise.reject(b);return a;}";
    let (prepared, names) = prepare(source, false);
    let mut store = Store::new(&prepared.engine, ());
    store.set_fuel(super::super::FUEL_PER_CALL).unwrap();
    let instance = Instance::new(&mut store, &prepared.module, &[]).unwrap();
    let live = instance
        .get_typed_func::<(), i32>(&mut store, "roots_live")
        .unwrap();
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
    assert_eq!(live.call(&mut activation.store, ()).unwrap(), 0);
    assert!(
        roots
            .iter()
            .all(|root| matches!(root.get(&mut activation.store), wasmtime::Val::AnyRef(None)))
    );
}

#[test]
fn checked_root_identities_reject_bad_owners_addresses_duplicates_and_counts() {
    for fault in 0..6 {
        let (prepared, _) = prepare(SHARED, false);
        let mut store = Store::new(&prepared.engine, ());
        store.set_fuel(super::super::FUEL_PER_CALL).unwrap();
        let instance = Instance::new(&mut store, &prepared.module, &[]).unwrap();
        let first = instance
            .get_typed_func::<(), i32>(&mut store, "first_root")
            .unwrap();
        let owner = instance
            .get_typed_func::<i32, i32>(&mut store, "root_owner")
            .unwrap();
        let check = instance
            .get_typed_func::<(i32, i32), ()>(&mut store, "check_root")
            .unwrap();
        let new = instance
            .get_typed_func::<(i32, i32, i32), ()>(&mut store, "new_root")
            .unwrap();
        let mut activation = AsyncInstance::start_with_instance(store, instance, 4.0, 2.0).unwrap();
        let first = first.call(&mut activation.store, ()).unwrap();
        let owner = owner.call(&mut activation.store, first).unwrap();
        if fault < 4 {
            let args = match fault {
                0 => (first, owner + 1),
                1 => (first + 1, owner),
                2 => (1048576, owner),
                _ => (-16, owner),
            };
            assert!(check.call(&mut activation.store, args).is_err());
        } else {
            let error = new
                .call(
                    &mut activation.store,
                    (
                        if fault == 4 { first } else { first + 16 },
                        owner,
                        if fault == 4 { 0 } else { -1 },
                    ),
                )
                .unwrap_err();
            assert!(format!("{error:?}").contains("unreachable"), "{error:?}");
        }
    }
}

#[test]
fn trapped_gc_frames_stay_unusable_until_store_teardown() {
    let source = "export async function run(a,b){const f=async n=>{const o={};await n;while(o){}};const p=f(a);return await p;}";
    let (prepared, _) = prepare(source, false);
    let mut activation = prepared.instantiate(4.0, 2.0).unwrap();
    activation.store.gc(None).unwrap();
    assert!(activation.run_one_job().is_err());
    assert!(activation.run_one_job().is_err());
    assert!(activation.pending_job_count().is_err());
    drop(activation);
    let (prepared, _) = prepare(SHARED, false);
    let mut fresh = prepared.instantiate(4.0, 2.0).unwrap();
    while fresh.run_one_job().unwrap() {}
    assert!(matches!(
        fresh.state().unwrap(),
        AsyncState::Fulfilled(super::super::Value::Number(1.0))
    ));
}
