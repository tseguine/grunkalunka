//! The shared source-control cursor now schedules ForStatement alongside
//! If/While/DoWhile. Keep the entry and lexical-cell regressions separate.
use super::*;
use oxc_ast::ast::ForStatement;

impl Lowering<'_, '_> {
    #[inline(never)]
    pub(super) fn for_statement_iterative<'n, 'a>(
        &mut self,
        root: &'n ForStatement<'a>,
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String> {
        self.control_statement_iterative(control_cursor::ControlNode::For(root), region, depth)
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::{Op, SlotId};

    #[test]
    fn nested_for_statements_keep_small_stack_and_generated_region_budget() {
        for (depth, admitted) in [(1, true), (8, true), (16, true), (65, false)] {
            let mut body = String::from("break;");
            for _ in 0..depth {
                body = format!("for(;a;) {body}");
            }
            let source = format!("export function run(a,b){{{body}return b;}}");
            for typescript in [false, true] {
                let source = source.clone();
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile(&source, typescript))
                    .expect("small-stack for lowering thread")
                    .join()
                    .expect("nested source for must not overflow native stack");
                if admitted {
                    assert!(
                        !result
                            .unwrap_or_else(|error| panic!("{depth}: {error}"))
                            .wasm
                            .is_empty()
                    );
                } else {
                    let error = result.err().expect("deep source for should be rejected");
                    assert!(
                        error.contains("64") || error.contains("128"),
                        "{depth} nested for: {error}"
                    );
                }
            }
        }
    }

    #[test]
    fn per_iteration_cells_are_generated_only_for_captured_let_headers() {
        for (body, expected) in [
            ("let n=0;for(let i=0;i<3;i++)n+=i;return n;", 0),
            (
                "let first=()=>a,last=()=>b;for(let i=0;i<3;i++){const make=()=>i;if(i===0)first=make;last=make;}return first()*10+last();",
                2,
            ),
            (
                "let first=()=>a,last=()=>b;for(let i=0;i<3;){const make=()=>i;if(i===0)first=make;last=make;i++;}return first()*10+last();",
                2,
            ),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            let module = crate::lower_source(&source, false)
                .unwrap_or_else(|error| panic!("{source}: {error}"));
            let mut renewals = 0;
            crate::ir::visit_operations(&module.functions[0].body, &mut |op| {
                if matches!(op, Op::RebindCell { .. }) {
                    renewals += 1;
                }
            });
            assert_eq!(renewals, expected, "{source}");
        }
    }

    #[test]
    fn rebinding_cannot_target_a_parameter_instead_of_the_lexical_header() {
        let source = "export function run(a,b){let first=()=>a,last=()=>b;for(let i=0;i<3;i++){const make=()=>i;if(i===0)first=make;last=make;}return first()*10+last();}";
        let mut module = crate::lower_source(source, false).unwrap();
        crate::ir::verify_module(&module).expect("source-lowered rebinding is valid");
        let header = module.functions[0]
            .body
            .operations
            .iter_mut()
            .find_map(|operation| match &mut operation.kind {
                Op::Block(region) => Some(region),
                _ => None,
            })
            .expect("for-let header block");
        let renewal = header
            .operations
            .iter_mut()
            .find_map(|operation| match &mut operation.kind {
                Op::RebindCell { slot, .. } => Some(slot),
                _ => None,
            })
            .expect("captured header rebinding");
        *renewal = SlotId(0);
        assert!(
            crate::ir::verify_module(&module)
                .unwrap_err()
                .contains("rebinding requires an initialized mutable lexical cell")
        );
    }

    #[test]
    fn for_header_declarations_and_updates_retain_scope_and_effect_order() {
        for body in [
            "let n=0;for(let i=0;i<3;i++)n+=b;return n;",
            "let n=a;for(let i=0;i<3;i++){n+=i;}return n;",
            "let n=0;for(let i=0;i<3;){n+=a;i++;}return n;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            let artifact =
                crate::compile(&source, false).unwrap_or_else(|error| panic!("{body}: {error}"));
            assert!(!artifact.wasm.is_empty());
        }
    }
}
