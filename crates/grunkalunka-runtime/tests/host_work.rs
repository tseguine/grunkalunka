//! A bounded host algorithm, not a general host-call duration limit.
use wasmtime::{Caller, Engine, Linker, Module, Store, TypedFunc};

#[derive(Debug, PartialEq)]
enum WorkError {
    Exhausted,
    InvalidCount,
}

impl std::fmt::Display for WorkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "host work: {self:?}")
    }
}

impl std::error::Error for WorkError {}

struct Work {
    remaining: u32,
    processed: usize,
    attempts: usize,
}

fn sum(work: &mut Work, count: i32) -> wasmtime::Result<i64> {
    work.attempts += 1;
    let values = [2, 3, 5, 7];
    // Validate the fixed host slice bound before conversion or any processing.
    if !(0..=4).contains(&count) {
        wasmtime::bail!(WorkError::InvalidCount);
    }
    let mut total = 0;
    for value in values.iter().take(count as usize) {
        if work.remaining == 0 {
            wasmtime::bail!(WorkError::Exhausted);
        }
        work.remaining = work.remaining.saturating_sub(1);
        work.processed += 1;
        total += value;
    }
    Ok(total)
}

fn instance(allowance: u32) -> (Store<Work>, TypedFunc<i32, i64>) {
    let engine = Engine::default();
    let bytes = wat::parse_str(
        r#"(module
            (import "host" "sum" (func $sum (param i32) (result i64)))
            (func (export "run") (param i32) (result i64)
                local.get 0
                call $sum))"#,
    )
    .unwrap();
    let module = Module::from_binary(&engine, &bytes).unwrap();
    let mut linker = Linker::new(&engine);
    linker
        .func_wrap("host", "sum", |mut caller: Caller<'_, Work>, count: i32| {
            sum(caller.data_mut(), count)
        })
        .unwrap();
    let mut store = Store::new(
        &engine,
        Work {
            remaining: allowance,
            processed: 0,
            attempts: 0,
        },
    );
    let instance = linker.instantiate(&mut store, &module).unwrap();
    let run = instance
        .get_typed_func::<i32, i64>(&mut store, "run")
        .unwrap();
    (store, run)
}

#[test]
fn zero_items_need_no_work_allowance() {
    let (mut store, run) = instance(0);
    assert_eq!(run.call(&mut store, 0).unwrap(), 0);
    assert_eq!(store.data().attempts, 1);
    assert_eq!(store.data().processed, 0);
    assert_eq!(store.data().remaining, 0);
}

#[test]
fn successful_work_uses_the_requested_prefix_and_debits_each_item() {
    for (count, expected, processed, remaining) in [(2, 5, 2, 2), (4, 17, 4, 0)] {
        let (mut store, run) = instance(4);
        assert_eq!(run.call(&mut store, count).unwrap(), expected);
        assert_eq!(store.data().attempts, 1);
        assert_eq!(store.data().processed, processed);
        assert_eq!(store.data().remaining, remaining);
    }
}

#[test]
fn quota_exhaustion_stops_before_an_unfunded_item() {
    for allowance in [0, 2] {
        let (mut store, run) = instance(allowance);
        let error = run.call(&mut store, 4).unwrap_err();
        assert_eq!(
            error.downcast_ref::<WorkError>(),
            Some(&WorkError::Exhausted)
        );
        assert_eq!(store.data().attempts, 1);
        assert_eq!(store.data().processed, allowance as usize);
        assert_eq!(store.data().remaining, 0);
    }
}

#[test]
fn consumed_allowance_is_cumulative_and_not_rolled_back_on_failure() {
    let (mut store, run) = instance(3);
    assert_eq!(run.call(&mut store, 2).unwrap(), 5);
    assert_eq!((store.data().processed, store.data().remaining), (2, 1));
    let error = run.call(&mut store, 2).unwrap_err();
    assert_eq!(
        error.downcast_ref::<WorkError>(),
        Some(&WorkError::Exhausted)
    );
    assert_eq!(store.data().attempts, 2);
    assert_eq!((store.data().processed, store.data().remaining), (3, 0));
}

#[test]
fn invalid_counts_are_rejected_before_consuming_allowance_or_processing() {
    for count in [-1, 5, i32::MAX] {
        let (mut store, run) = instance(4);
        let error = run.call(&mut store, count).unwrap_err();
        assert_eq!(
            error.downcast_ref::<WorkError>(),
            Some(&WorkError::InvalidCount)
        );
        assert_eq!(store.data().attempts, 1);
        assert_eq!((store.data().processed, store.data().remaining), (0, 4));
    }
}
