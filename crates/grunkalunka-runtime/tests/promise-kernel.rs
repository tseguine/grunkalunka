use wasmtime::{Config, Engine, Instance, Module, Store};

fn wasm() -> Vec<u8> {
    let allocator_source = include_str!("../../grunkalunka-compiler/src/emit/closures.rs");
    let allocator = allocator_source
        .split_once("pub(super) const FRAME_ALLOCATOR: &str = r#\"")
        .unwrap()
        .1
        .split_once("\"#")
        .unwrap()
        .0;
    let kernel = include_str!("../../grunkalunka-compiler/src/emit/asynchronous/promises.wat");
    let exports = [
        ("new", "promise_new"),
        ("state", "promise_state"),
        ("settle", "promise_settle"),
        ("result", "promise_result"),
        ("handled", "promise_handled"),
        ("subscribe", "promise_subscribe"),
        ("schedule", "reaction_schedule"),
        ("ready", "reaction_ready"),
        ("issued", "reaction_issued"),
        ("take", "reaction_take"),
        ("read", "reaction_read"),
    ]
    .map(|(name, function)| format!("(export \"{name}\" (func ${function}))"))
    .join("\n");
    wat::parse_str(format!("(module (memory (export \"memory\") 1 16) (global $cell_top (mut i32) (i32.const 0)) {allocator} {kernel} {exports} (func (export \"top\") (result i32) global.get $cell_top) (func (export \"prime\") (result i32) i32.const 16 call $closure_frame))")).unwrap()
}

fn instance() -> (Store<()>, Instance) {
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config).unwrap();
    let module = Module::new(&engine, wasm()).unwrap();
    let mut store = Store::new(&engine, ());
    store.set_fuel(10_000_000).unwrap();
    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    (store, instance)
}

#[test]
fn distinct_promise_records_start_pending() {
    let (mut store, instance) = instance();
    let new = instance
        .get_typed_func::<(), i32>(&mut store, "new")
        .unwrap();
    let state = instance
        .get_typed_func::<i32, i32>(&mut store, "state")
        .unwrap();
    let first = new.call(&mut store, ()).unwrap();
    let second = new.call(&mut store, ()).unwrap();
    assert_ne!(first, second);
    assert_ne!(first, 0);
    assert_eq!(first & 15, 0);
    assert_eq!(state.call(&mut store, first).unwrap(), 0);
    assert_eq!(state.call(&mut store, second).unwrap(), 0);
}

#[test]
fn primitive_promise_settlement_is_single_assignment() {
    let (mut store, instance) = instance();
    let new = instance
        .get_typed_func::<(), i32>(&mut store, "new")
        .unwrap();
    let settle = instance
        .get_typed_func::<(i32, i32, i32, f64), i32>(&mut store, "settle")
        .unwrap();
    let state = instance
        .get_typed_func::<i32, i32>(&mut store, "state")
        .unwrap();
    let result = instance
        .get_typed_func::<i32, (i32, i32, f64)>(&mut store, "result")
        .unwrap();
    for rejected in [0, 1] {
        for (tag, payload) in [
            (0, 0.0),
            (1, 0.0),
            (2, 0.0),
            (2, 1.0),
            (3, -0.0),
            (3, f64::NAN),
            (3, f64::INFINITY),
            (3, f64::NEG_INFINITY),
            (3, f64::from_bits(1)),
            (3, f64::MAX),
        ] {
            let p = new.call(&mut store, ()).unwrap();
            assert_eq!(
                settle
                    .call(&mut store, (p, rejected, tag, payload))
                    .unwrap(),
                1
            );
            assert_eq!(state.call(&mut store, p).unwrap(), 1 + rejected);
            assert_eq!(
                settle.call(&mut store, (p, 1 - rejected, 3, 17.0)).unwrap(),
                0
            );
            let (actual_rejected, actual_tag, actual) = result.call(&mut store, p).unwrap();
            assert_eq!((actual_rejected, actual_tag), (rejected, tag));
            assert!(actual.is_nan() && payload.is_nan() || actual.to_bits() == payload.to_bits());
        }
    }
}

#[test]
fn pending_and_settled_subscribers_keep_fifo_and_issue_counts() {
    let (mut store, instance) = instance();
    let new = instance
        .get_typed_func::<(), i32>(&mut store, "new")
        .unwrap();
    let settle = instance
        .get_typed_func::<(i32, i32, i32, f64), i32>(&mut store, "settle")
        .unwrap();
    let subscribe = instance
        .get_typed_func::<(i32, i32, i32), ()>(&mut store, "subscribe")
        .unwrap();
    let handled = instance
        .get_typed_func::<i32, i32>(&mut store, "handled")
        .unwrap();
    let ready = instance
        .get_typed_func::<(), i32>(&mut store, "ready")
        .unwrap();
    let issued = instance
        .get_typed_func::<(), i32>(&mut store, "issued")
        .unwrap();
    let take = instance
        .get_typed_func::<(), i32>(&mut store, "take")
        .unwrap();
    let read = instance
        .get_typed_func::<i32, (i32, i32, i32, i32, f64)>(&mut store, "read")
        .unwrap();
    let p = new.call(&mut store, ()).unwrap();
    assert_eq!(handled.call(&mut store, p).unwrap(), 0);
    subscribe.call(&mut store, (p, 10, 3)).unwrap();
    subscribe.call(&mut store, (p, 20, 4)).unwrap();
    assert_eq!(handled.call(&mut store, p).unwrap(), 1);
    assert_eq!(ready.call(&mut store, ()).unwrap(), 0);
    assert_eq!(issued.call(&mut store, ()).unwrap(), 0);
    settle.call(&mut store, (p, 1, 3, -0.0)).unwrap();
    assert_eq!(ready.call(&mut store, ()).unwrap(), 2);
    assert_eq!(issued.call(&mut store, ()).unwrap(), 2);
    assert_eq!(issued.call(&mut store, ()).unwrap(), 0);
    for (frame, pc) in [(10, 3), (20, 4)] {
        let job = take.call(&mut store, ()).unwrap();
        assert_ne!(job, 0);
        let (actual_frame, actual_pc, rejected, tag, payload) = read.call(&mut store, job).unwrap();
        assert_eq!((actual_frame, actual_pc, rejected, tag), (frame, pc, 1, 3));
        assert_eq!(payload.to_bits(), (-0.0f64).to_bits());
    }
    assert_eq!(take.call(&mut store, ()).unwrap(), 0);
    assert_eq!(ready.call(&mut store, ()).unwrap(), 0);
    subscribe.call(&mut store, (p, 30, 5)).unwrap();
    assert_eq!(issued.call(&mut store, ()).unwrap(), 1);
    let job = take.call(&mut store, ()).unwrap();
    assert_eq!(read.call(&mut store, job).unwrap().0, 30);
    assert_eq!(take.call(&mut store, ()).unwrap(), 0);
}

#[test]
fn reactions_from_multiple_promises_share_one_ready_fifo() {
    let (mut store, instance) = instance();
    let new = instance
        .get_typed_func::<(), i32>(&mut store, "new")
        .unwrap();
    let settle = instance
        .get_typed_func::<(i32, i32, i32, f64), i32>(&mut store, "settle")
        .unwrap();
    let subscribe = instance
        .get_typed_func::<(i32, i32, i32), ()>(&mut store, "subscribe")
        .unwrap();
    let schedule = instance
        .get_typed_func::<(i32, i32, i32, i32, f64), ()>(&mut store, "schedule")
        .unwrap();
    let ready = instance
        .get_typed_func::<(), i32>(&mut store, "ready")
        .unwrap();
    let issued = instance
        .get_typed_func::<(), i32>(&mut store, "issued")
        .unwrap();
    let take = instance
        .get_typed_func::<(), i32>(&mut store, "take")
        .unwrap();
    let read = instance
        .get_typed_func::<i32, (i32, i32, i32, i32, f64)>(&mut store, "read")
        .unwrap();
    let p = new.call(&mut store, ()).unwrap();
    let q = new.call(&mut store, ()).unwrap();
    for frame in 0..128 {
        subscribe
            .call(
                &mut store,
                (if frame % 2 == 0 { p } else { q }, frame, frame + 1),
            )
            .unwrap();
    }
    settle.call(&mut store, (q, 1, 1, 0.0)).unwrap();
    settle.call(&mut store, (p, 0, 2, 1.0)).unwrap();
    schedule
        .call(&mut store, (128, 129, 0, 3, f64::INFINITY))
        .unwrap();
    assert_eq!(issued.call(&mut store, ()).unwrap(), 129);
    assert_eq!(ready.call(&mut store, ()).unwrap(), 129);
    for frame in (0..128)
        .filter(|n| n % 2 == 1)
        .chain((0..128).filter(|n| n % 2 == 0))
        .chain([128])
    {
        let job = take.call(&mut store, ()).unwrap();
        let actual = read.call(&mut store, job).unwrap();
        let expected = if frame == 128 {
            (frame, frame + 1, 0, 3, f64::INFINITY)
        } else if frame % 2 == 1 {
            (frame, frame + 1, 1, 1, 0.0)
        } else {
            (frame, frame + 1, 0, 2, 1.0)
        };
        assert_eq!(actual, expected);
    }
    assert_eq!(take.call(&mut store, ()).unwrap(), 0);
    assert_eq!(issued.call(&mut store, ()).unwrap(), 0);
    assert_eq!(ready.call(&mut store, ()).unwrap(), 0);
}

#[test]
fn kernel_private_inputs_reject_invalid_records_and_noncanonical_payloads() {
    let (mut store, instance) = instance();
    let new = instance
        .get_typed_func::<(), i32>(&mut store, "new")
        .unwrap();
    let settle = instance
        .get_typed_func::<(i32, i32, i32, f64), i32>(&mut store, "settle")
        .unwrap();
    let schedule = instance
        .get_typed_func::<(i32, i32, i32, i32, f64), ()>(&mut store, "schedule")
        .unwrap();
    let state = instance
        .get_typed_func::<i32, i32>(&mut store, "state")
        .unwrap();
    let result = instance
        .get_typed_func::<i32, (i32, i32, f64)>(&mut store, "result")
        .unwrap();
    let subscribe = instance
        .get_typed_func::<(i32, i32, i32), ()>(&mut store, "subscribe")
        .unwrap();
    let ready = instance
        .get_typed_func::<(), i32>(&mut store, "ready")
        .unwrap();
    let top = instance
        .get_typed_func::<(), i32>(&mut store, "top")
        .unwrap();
    let take = instance
        .get_typed_func::<(), i32>(&mut store, "take")
        .unwrap();
    let read = instance
        .get_typed_func::<i32, (i32, i32, i32, i32, f64)>(&mut store, "read")
        .unwrap();
    let p = new.call(&mut store, ()).unwrap();
    for invalid in [0, -1, p + 1, p + 16, 1048560, 1048576] {
        assert!(state.call(&mut store, invalid).is_err());
        assert!(subscribe.call(&mut store, (invalid, 1, 1)).is_err());
    }
    assert!(result.call(&mut store, p).is_err());
    let initial_top = top.call(&mut store, ()).unwrap();
    for (rejected, tag, payload) in [
        (2, 3, 0.0),
        (-1, 3, 0.0),
        (0, -1, 0.0),
        (0, 4, 0.0),
        (0, 0, -0.0),
        (0, 1, 1.0),
        (0, 2, -0.0),
        (0, 2, 2.0),
        (0, 2, f64::NAN),
        (0, 0, f64::NAN),
    ] {
        assert!(
            settle
                .call(&mut store, (p, rejected, tag, payload))
                .is_err()
        );
        assert!(
            schedule
                .call(&mut store, (1, 1, rejected, tag, payload))
                .is_err()
        );
        assert_eq!(state.call(&mut store, p).unwrap(), 0);
        assert_eq!(ready.call(&mut store, ()).unwrap(), 0);
        assert_eq!(top.call(&mut store, ()).unwrap(), initial_top);
    }
    subscribe.call(&mut store, (p, 1, 2)).unwrap();
    let waiter = initial_top;
    assert!(state.call(&mut store, waiter).is_err());
    assert!(read.call(&mut store, waiter).is_err());
    settle.call(&mut store, (p, 0, 3, 2.0)).unwrap();
    assert!(read.call(&mut store, waiter).is_err());
    let job = take.call(&mut store, ()).unwrap();
    assert_eq!(job, waiter);
    assert!(state.call(&mut store, job).is_err());
    assert_eq!(read.call(&mut store, job).unwrap(), (1, 2, 0, 3, 2.0));
    assert_eq!(take.call(&mut store, ()).unwrap(), 0);
}

#[test]
fn promise_records_preserve_existing_address_zero_arena_owners() {
    let (mut store, instance) = instance();
    let prime = instance
        .get_typed_func::<(), i32>(&mut store, "prime")
        .unwrap();
    let new = instance
        .get_typed_func::<(), i32>(&mut store, "new")
        .unwrap();
    assert_eq!(prime.call(&mut store, ()).unwrap(), 0);
    let memory = instance.get_memory(&mut store, "memory").unwrap();
    let sentinel = [0x5a; 16];
    memory.write(&mut store, 0, &sentinel).unwrap();
    assert_eq!(new.call(&mut store, ()).unwrap(), 16);
    assert_eq!(&memory.data(&store)[..16], &sentinel);
}

#[test]
fn promise_and_waiter_growth_hit_the_arena_guard_before_fuel_exhaustion() {
    for waiters in [false, true] {
        let (mut store, instance) = instance();
        let new = instance
            .get_typed_func::<(), i32>(&mut store, "new")
            .unwrap();
        let subscribe = instance
            .get_typed_func::<(i32, i32, i32), ()>(&mut store, "subscribe")
            .unwrap();
        let p = new.call(&mut store, ()).unwrap();
        let mut count = 1;
        let error = loop {
            let result = if waiters {
                subscribe.call(&mut store, (p, count, count)).map(|_| 0)
            } else {
                new.call(&mut store, ())
            };
            match result {
                Ok(_) => count += 1,
                Err(error) => break error,
            }
        };
        assert_eq!(count, 32767);
        assert_eq!(
            error.downcast_ref::<wasmtime::Trap>(),
            Some(&wasmtime::Trap::UnreachableCodeReached)
        );
        assert!(store.get_fuel().unwrap() > 0);
        assert_eq!(
            instance
                .get_memory(&mut store, "memory")
                .unwrap()
                .size(&store),
            16
        );
    }
}

#[test]
fn promise_kernel_reactions_match_native_microtask_enqueue_order() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let script = include_str!("promise-kernel.mjs");
    let mut child = Command::new("node")
        .args(["--input-type=module", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let source = format!("const bytes=new Uint8Array({:?});\n{script}", wasm());
    child
        .stdin
        .take()
        .unwrap()
        .write_all(source.as_bytes())
        .unwrap();
    let started = std::time::Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() > std::time::Duration::from_secs(30) {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!(
                "Node kernel probe exceeded 30 seconds: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
