//! Object spread needs no new carrier or host operation.
use super::ir;

fn needed(module: &ir::Module) -> bool {
    module.functions.iter().any(|function| {
        ir::any_operation(&function.body, &mut |op| {
            matches!(op, ir::Op::ManagedObjectSpread { .. })
        })
    })
}

pub(super) fn runtime(module: &ir::Module) -> &'static str {
    if needed(module) {
        include_str!("managed_spread.wat")
    } else {
        ""
    }
}
