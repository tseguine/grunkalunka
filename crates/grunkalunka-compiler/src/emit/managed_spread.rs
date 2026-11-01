//! Object spread needs no new carrier or host operation.
use super::ir;

pub(super) fn runtime(module: &ir::Module) -> &'static str {
    if module.functions.iter().any(|function| {
        ir::any_operation(&function.body, &mut |op| {
            matches!(op, ir::Op::ManagedObjectSpread { .. })
        })
    }) {
        include_str!("managed_spread.wat")
    } else {
        ""
    }
}
