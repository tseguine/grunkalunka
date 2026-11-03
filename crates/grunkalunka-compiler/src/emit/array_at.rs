use super::ir;

pub(super) fn needed(module: &ir::Module) -> bool {
    module.functions.iter().any(|function| {
        ir::any_operation(&function.body, &mut |op| {
            matches!(op, ir::Op::ManagedArrayAt { .. })
        })
    })
}

pub(super) fn runtime(module: &ir::Module) -> &'static str {
    if !needed(module) {
        ""
    } else if super::managed_spread::needed(module) || super::own_keys::needed(module) {
        include_str!("array_at.wat")
    } else {
        concat!(
            include_str!("managed_index.wat"),
            include_str!("array_at.wat")
        )
    }
}
