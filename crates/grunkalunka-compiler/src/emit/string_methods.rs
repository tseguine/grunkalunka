//! Private UTF-16 helpers are included only when the corresponding IR is used.
use super::ir;

pub(super) fn runtime(module: &ir::Module) -> &'static str {
    if module.functions.iter().any(|function| {
        ir::any_operation(&function.body, &mut |op| {
            matches!(
                op,
                ir::Op::ManagedStringRange { .. }
                    | ir::Op::ManagedStringCharAt { .. }
                    | ir::Op::ManagedStringCharCodeAt { .. }
                    | ir::Op::ManagedStringSearch { .. }
            )
        })
    }) {
        include_str!("string_methods.wat")
    } else {
        ""
    }
}
