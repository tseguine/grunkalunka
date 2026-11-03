//! Native evaluator-host nesting is distinct from evaluator-runner nesting:
//! a completed zero-argument factory can otherwise resume another host inline.
use std::cell::Cell;

thread_local! {
    static ACTIVE: Cell<usize> = const { Cell::new(0) };
    static MAXIMUM: Cell<usize> = const { Cell::new(0) };
    static ENTRIES: Cell<usize> = const { Cell::new(0) };
}

pub(super) struct Entry;

pub(super) fn entry() -> Entry {
    ACTIVE.with(|active| {
        active.set(active.get() + 1);
        MAXIMUM.with(|maximum| maximum.set(maximum.get().max(active.get())));
    });
    ENTRIES.with(|entries| entries.set(entries.get() + 1));
    Entry
}

impl Drop for Entry {
    fn drop(&mut self) {
        ACTIVE.with(|active| active.set(active.get().checked_sub(1).expect("balanced host")));
    }
}

fn observe(
    body: String,
    asynchronous: bool,
    typescript: bool,
) -> (Result<(), String>, usize, usize) {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            ACTIVE.with(|active| assert_eq!(active.get(), 0));
            MAXIMUM.with(|maximum| maximum.set(0));
            ENTRIES.with(|entries| entries.set(0));
            let prefix = if asynchronous { "async " } else { "" };
            let source = format!("export {prefix}function run(a,b){{{body}}}");
            let result = if asynchronous {
                crate::compile_async(&source, typescript).map(|_| ())
            } else {
                crate::compile(&source, typescript).map(|_| ())
            };
            ACTIVE.with(|active| assert_eq!(active.get(), 0, "host restores on every exit"));
            (result, MAXIMUM.with(Cell::get), ENTRIES.with(Cell::get))
        })
        .expect("2 MiB evaluator-host thread")
        .join()
        .expect("evaluator hosts must not exhaust the native stack")
}

#[test]
fn completed_factory_evaluators_resume_from_the_source_loop() {
    for (kind, declaration) in [
        ("concise", "const make=()=>()=>a;"),
        ("block", "const make=()=>{return ()=>a;};"),
        ("runtime", "let make=()=>()=>a;"),
    ] {
        for count in [1, 2, 4, 8, 16] {
            for typescript in [false, true] {
                let expression = std::iter::repeat_n("make()()", count)
                    .collect::<Vec<_>>()
                    .join("+");
                let body = format!("{declaration}return {expression};");
                let (result, maximum, entries) = observe(body, false, typescript);
                assert!(result.is_ok(), "{kind}, count={count}: {result:?}");
                assert_eq!(
                    maximum, 1,
                    "{kind}, count={count}: evaluator host reentered inline"
                );
                assert!(
                    entries > count,
                    "fixture must exercise repeated evaluator resumes"
                );
            }
        }
    }
}

#[test]
fn queued_resumptions_keep_caller_contexts_and_async_results() {
    for (body, asynchronous) in [
        (
            "let n=0;const make=()=>()=>++n;return make()()+make()()+n;",
            false,
        ),
        (
            "const make=()=>()=>a;if(b){return make()()+make()();}return a;",
            false,
        ),
        (
            "let make=()=>async x=>{await a;return x+b;};return await make()(a);",
            true,
        ),
    ] {
        for typescript in [false, true] {
            let (result, maximum, entries) = observe(body.into(), asynchronous, typescript);
            assert!(result.is_ok(), "{body}: {result:?}");
            assert_eq!(maximum, 1, "caller resumed within a live evaluator host");
            assert!(entries > 1);
        }
    }
}

#[test]
fn queued_resumption_errors_release_the_evaluator_host() {
    for (count, expected) in [
        (
            1,
            "unsupported at bytes 62..76: expression is outside the numeric experiment",
        ),
        (
            8,
            "unsupported at bytes 125..139: expression is outside the numeric experiment",
        ),
    ] {
        for typescript in [false, true] {
            let expression = std::iter::repeat_n("make()()", count)
                .collect::<Vec<_>>()
                .join("+");
            let body = format!("const make=()=>()=>a;return {expression}+Math.noSuch(b);");
            let (result, maximum, entries) = observe(body, false, typescript);
            assert_eq!(result.unwrap_err(), expected);
            assert_eq!(maximum, 1, "diagnostic retained a nested evaluator host");
            assert!(entries > count);
        }
    }
}
