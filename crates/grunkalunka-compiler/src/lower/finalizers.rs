//! Lexical try/catch/finally lowering is owned by the shared source
//! continuation cursor. This file retains the entry point and source probes.
use super::*;
use oxc_ast::ast::TryStatement;

impl Lowering<'_, '_> {
    pub(super) fn try_statement(
        &mut self,
        statement: &TryStatement<'_>,
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String> {
        let node = if statement.finalizer.is_some() {
            control_cursor::ControlNode::TryFinally(statement)
        } else {
            control_cursor::ControlNode::TryCatch(statement)
        };
        self.control_statement_iterative(node, region, depth)
    }
}

#[cfg(test)]
mod stack_hygiene_tests {
    #[test]
    fn alternating_try_catch_and_if_blocks_stay_on_heap_continuations() {
        for depth in [2, 8, 16, 32] {
            let mut body = String::from("return a;");
            for index in 0..depth {
                body = if index % 2 == 0 {
                    format!("try{{{body}}}catch(e){{return b;}}")
                } else {
                    format!("if(a){{{body}}}else return b;")
                };
            }
            let source = format!("export async function run(a,b){{{body}}}");
            for ts in [false, true] {
                let source = source.clone();
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile_async(&source, ts))
                    .expect("small-stack mixed try/if compiler")
                    .join()
                    .expect("alternating try/catch source cannot overflow native stack");
                if depth <= 8 {
                    assert!(result.is_ok(), "{depth} controls: {:?}", result.err());
                } else if let Err(error) = result {
                    assert!(
                        error.contains("64")
                            || error.contains("128")
                            || error.contains("256")
                            || error.contains("nested"),
                        "{depth} controls: {error}"
                    );
                }
            }
        }
    }

    #[test]
    fn direct_nested_try_catch_chain_obeys_small_stack_and_source_limits() {
        for (depth, admitted) in [(1, true), (8, true), (16, true), (34, false)] {
            let mut body = String::from("return a;");
            for _ in 0..depth {
                body = format!("try{{{body}}}catch(e){{return b;}}");
            }
            let source = format!("export async function run(a,b){{{body}}}");
            for typescript in [false, true] {
                let source = source.clone();
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile_async(&source, typescript))
                    .expect("small-stack source try/catch thread")
                    .join()
                    .expect("nested try/catch must not overflow native stack");
                if admitted {
                    let artifact = result.unwrap_or_else(|error| panic!("{depth}: {error}"));
                    assert!(!artifact.wasm.is_empty());
                } else {
                    let error = result.err().expect("excessive source nesting must reject");
                    assert!(
                        error.contains("64") || error.contains("128"),
                        "{depth} nested try/catch: {error}"
                    );
                }
            }
        }
    }

    #[test]
    fn interleaved_catch_finally_and_branch_controls_use_one_stack() {
        for depth in [2, 4, 8, 24] {
            let mut body = String::from("return a;");
            for index in 0..depth {
                body = match index % 3 {
                    0 => format!("try{{{body}}}finally{{await b;}}"),
                    1 => format!("if(a){{{body}}}else return b;"),
                    _ => format!("try{{{body}}}catch(e){{return b;}}"),
                };
            }
            let source = format!("export async function run(a,b){{{body}}}");
            for ts in [false, true] {
                let source = source.clone();
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile_async(&source, ts))
                    .expect("small-stack mixed catch/finally lowering")
                    .join()
                    .expect("source cleanup and branches must not overflow native stack");
                if depth <= 4 {
                    assert!(result.is_ok(), "{depth} controls: {:?}", result.err());
                } else if let Err(error) = result {
                    assert!(
                        error.contains("64")
                            || error.contains("128")
                            || error.contains("4096")
                            || error.contains("nest"),
                        "{depth} controls: {error}"
                    );
                }
            }
        }
    }

    #[test]
    fn direct_nested_finalizers_keep_small_stack_and_original_depth_guards() {
        for depth in [1, 2, 4, 8, 33] {
            let mut body = String::from("return a;");
            for _ in 0..depth {
                body = format!("try{{{body}}}finally{{await b;}}");
            }
            let source = format!("export async function run(a,b){{{body}}}");
            for typescript in [false, true] {
                let source = source.clone();
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile_async(&source, typescript))
                    .expect("2 MiB compiler thread")
                    .join()
                    .expect("source cleanup nesting must not overflow");
                if depth <= 4 {
                    let artifact = result.unwrap_or_else(|error| panic!("{depth}: {error}"));
                    assert!(!artifact.wasm.is_empty());
                } else if let Err(error) = result {
                    assert!(
                        error.contains("64")
                            || error.contains("128")
                            || error.contains("4096")
                            || error.contains("nest"),
                        "{depth} nested finalizers: {error}",
                    );
                }
            }
        }
    }

}
