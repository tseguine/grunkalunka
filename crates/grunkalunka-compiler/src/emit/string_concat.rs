//! UTF-16 concatenation is emitted only for admitted String additions.
use super::ir;

pub(super) fn runtime(module: &ir::Module) -> &'static str {
    if module.functions.iter().any(|function| {
        ir::any_operation(&function.body, &mut |op| {
            matches!(
                op,
                ir::Op::ManagedStringConcat { .. } | ir::Op::ManagedScalarAdd { .. }
            )
        })
    }) {
        include_str!("string_concat.wat")
    } else {
        ""
    }
}
