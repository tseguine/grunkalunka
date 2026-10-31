//! One-child BlockStatement chains are source constructs, not synthetic IR.
#[test]
fn nested_single_statement_blocks_do_not_consume_native_stack() {
    for (depth, admitted) in [(1, true), (32, true), (64, true), (65, false)] {
        let body = format!("{}return a;{}", "{".repeat(depth), "}".repeat(depth));
        let source = format!("export function run(a,b){{{body}}}");
        for typescript in [false, true] {
            let source = source.clone();
            let result = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || crate::compile(&source, typescript))
                .expect("small-stack lowering thread")
                .join()
                .expect("source block lowering must not overflow");
            if admitted {
                assert!(!result.unwrap().wasm.is_empty(), "{depth} blocks");
            } else {
                let error = result.err().expect("source nesting must be rejected");
                assert!(
                    error.contains("statement nesting exceeds 64"),
                    "{depth} blocks: {error}"
                );
            }
        }
    }
}
