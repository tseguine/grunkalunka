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

#[test]
fn nested_multistatement_blocks_keep_depth_and_binding_boundaries_on_small_stacks() {
    // Every source block has a declaration *and* a nested statement. This
    // prevents the one-child-block shortcut from masking recursive sequence
    // reentry. The innermost return still consumes the existing depth limit.
    for (depth, admitted) in [(1, true), (16, true), (64, true), (65, false)] {
        let mut body = String::from("return a;");
        for index in 0..depth {
            body = format!("{{let value_{index}=a;{body}}}");
        }
        let source = format!("export function run(a,b){{{body}}}");
        for typescript in [false, true] {
            let source = source.clone();
            let result = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || crate::compile(&source, typescript))
                .expect("small-stack nested-sequence thread")
                .join()
                .expect("native stack must remain bounded");
            if admitted {
                let artifact = result.unwrap_or_else(|error| panic!("{depth}: {error}"));
                assert!(!artifact.wasm.is_empty());
            } else {
                let error = result.err().expect("over-depth source must be rejected");
                assert!(
                    error.contains("statement nesting exceeds 64"),
                    "{depth} blocks: {error}"
                );
            }
        }
    }
}
