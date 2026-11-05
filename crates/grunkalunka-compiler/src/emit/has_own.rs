use super::ir;

pub(super) fn runtime(module: &ir::Module) -> &'static str {
    if module.functions.iter().any(|function| {
        ir::any_operation(&function.body, &mut |op| {
            matches!(op, ir::Op::ManagedHasOwn { .. })
        })
    }) {
        include_str!("has_own.wat")
    } else {
        ""
    }
}
