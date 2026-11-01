//! Object spread needs no new carrier or host operation.
use super::ir;

pub(super) fn needed(module: &ir::Module) -> bool {
    module.functions.iter().any(|function| {
        ir::any_operation(&function.body, &mut |op| {
            matches!(op, ir::Op::ManagedObjectSpread { .. })
        })
    })
}

pub(super) fn runtime(module: &ir::Module) -> &'static str {
    if needed(module) {
        concat!(
            include_str!("managed_index.wat"),
            include_str!("managed_spread.wat")
        )
    } else {
        ""
    }
}
