//! Test-only observations of the native source-control cursor call stack.
//!
//! Deep mixed syntax must remain on one heap continuation scheduler. This
//! probes that invariant independently of source-depth admission and the
//! expression evaluator's separately tracked reentry.
use std::cell::Cell;

thread_local! {
    static ACTIVE: Cell<usize> = const { Cell::new(0) };
    static MAXIMUM: Cell<usize> = const { Cell::new(0) };
    static ENTRIES: Cell<usize> = const { Cell::new(0) };
    static PENDING: Cell<usize> = const { Cell::new(0) };
}

pub(crate) struct Entry;

pub(crate) fn entry() -> Entry {
    ACTIVE.with(|active| {
        active.set(active.get() + 1);
        MAXIMUM.with(|maximum| maximum.set(maximum.get().max(active.get())));
    });
    ENTRIES.with(|entries| entries.set(entries.get() + 1));
    Entry
}

impl Drop for Entry {
    fn drop(&mut self) {
        ACTIVE.with(|active| active.set(active.get().checked_sub(1).expect("balanced cursor")));
    }
}

pub(crate) fn pending(count: usize) {
    PENDING.with(|pending| pending.set(pending.get().max(count)));
}

fn observe<T>(run: impl FnOnce() -> T) -> (T, usize, usize, usize) {
    ACTIVE.with(|active| assert_eq!(active.get(), 0));
    MAXIMUM.with(|maximum| maximum.set(0));
    ENTRIES.with(|entries| entries.set(0));
    PENDING.with(|pending| pending.set(0));
    let result = run();
    ACTIVE.with(|active| assert_eq!(active.get(), 0, "cursor must restore on all exits"));
    (
        result,
        MAXIMUM.with(Cell::get),
        ENTRIES.with(Cell::get),
        PENDING.with(Cell::get),
    )
}

#[test]
fn mixed_source_control_lowers_with_a_single_native_cursor_entry() {
    // All source-control families should share the *same* scheduling loop;
    // a second native entry would reveal an uncaught control/control SCC.
    for (body, asynchronous) in [
        (
            "if(a){while(b){break;}}else{for(let i=0;i<2;i++){a+=i;}}return a;",
            false,
        ),
        (
            "let n=a;do{if(n){n+=b;}else{n+=1;}}while(false);return n;",
            false,
        ),
        (
            "try{try{return a;}catch(e){return b;}}finally{await b;}",
            true,
        ),
    ] {
        let source = format!(
            "export {}function run(a,b){{{body}}}",
            if asynchronous { "async " } else { "" }
        );
        for ts in [false, true] {
            let (result, maximum, entries, _) = observe(|| {
                if asynchronous {
                    crate::compile_async(&source, ts)
                } else {
                    crate::compile(&source, ts)
                }
            });
            assert!(result.is_ok(), "{source}: {:?}", result.err());
            assert_eq!(maximum, 1, "{source}: source-control reentered");
            assert!(entries > 0, "{source}: no source cursor entry");
        }
    }
}

#[test]
fn rejected_mixed_source_nesting_exits_without_reentering_native_cursor() {
    let mut body = String::from("a+=b;");
    for index in 0..66 {
        body = if index % 2 == 0 {
            format!("if(a){{{body}}}else{{b+=a;}}")
        } else {
            format!("while(a){{{body}}}")
        };
    }
    let source = format!("export function run(a,b){{{body}return a;}}");
    let (result, maximum, entries, _) = observe(|| crate::compile(&source, false));
    let error = match result {
        Ok(_) => panic!("over-depth mixed source must reject"),
        Err(error) => error,
    };
    assert!(
        error.contains("64") || error.contains("128"),
        "mixed nesting: {error}"
    );
    assert_eq!(maximum, 1, "rejected mixed source reentered the cursor");
    assert!(entries > 0);
}

#[test]
fn suspended_if_predicate_preserves_expression_diagnostics_on_two_mib_stack() {
    // The condition now crosses the source -> expression request/result seam
    // instead of being executed inside the If entry transition. Preserve
    // ordering of comma/conditional expressions and failure before either arm.
    for (body, admitted) in [
        ("if((a=1,!b,a<b)) return a; return b;", true),
        ("if(a?(b?true:false):!a) return a; return b;", true),
        ("if(a ** b) return a; return b;", false),
    ] {
        for typescript in [false, true] {
            let source = format!("export function run(a,b) {{ {body} }}");
            let (result, maximum, entries, _) = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || {
                    observe(|| crate::compile(&source, typescript))
                })
                .expect("2 MiB source expression request thread")
                .join()
                .expect("condition request must not exhaust native stack");
            assert_eq!(result.is_ok(), admitted, "{body}: {:?}", result.err());
            assert_eq!(maximum, 1, "source cursor reentered for {body}");
            assert!(entries > 0, "no source cursor entered for {body}");
        }
    }
}
