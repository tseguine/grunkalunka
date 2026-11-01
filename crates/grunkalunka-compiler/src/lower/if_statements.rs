//! The shared heap source-control cursor owns mixed If/While/DoWhile
//! continuations; this module keeps the If entry and focused regressions.
use super::*;
use oxc_ast::ast::IfStatement;

impl Lowering<'_, '_> {
    #[inline(never)]
    pub(super) fn if_statement_iterative<'n, 'a>(
        &mut self,
        root: &'n IfStatement<'a>,
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String> {
        self.control_statement_iterative(control_cursor::ControlNode::If(root), region, depth)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn nested_consequent_and_alternate_ifs_respect_two_mib_stack_and_depth_budget() {
        for (depth, admitted) in [(1, true), (16, true), (64, true), (65, false)] {
            let mut body = String::from("return a;");
            for _ in 0..depth {
                body = format!("if(a){body}else return b;");
            }
            let source = format!("export function run(a,b){{{body}}}");
            for typescript in [false, true] {
                let source = source.clone();
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile(&source, typescript))
                    .expect("native small-stack compiler thread")
                    .join()
                    .expect("nested source If must not overflow native stack");
                if admitted {
                    let artifact = result.unwrap_or_else(|error| panic!("{depth}: {error}"));
                    assert!(!artifact.wasm.is_empty());
                } else {
                    let error = result.err().expect("over-depth source must be rejected");
                    assert!(
                        error.contains("statement nesting exceeds 64"),
                        "{depth} if statements: {error}"
                    );
                }
            }
        }
    }

    #[test]
    fn nested_else_if_regions_keep_both_completion_paths() {
        let mut body = String::from("return b;");
        for _ in 0..24 {
            body = format!("if(a)return a;else {body}");
        }
        let source = format!("export function run(a,b){{{body}}}");
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                crate::compile(&source, false).expect("nested else-if branches");
            })
            .expect("small stack else-if thread")
            .join()
            .expect("else-if compiler must not overflow");
    }
}
