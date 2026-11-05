//! Dense copying for an admitted intrinsic Array iterator's remaining values.
use super::ir;

pub(super) fn runtime(module: &ir::Module) -> &'static str {
    let needed = module.functions.iter().any(|function| {
        ir::any_operation(&function.body, &mut |op| {
            matches!(op, ir::Op::ManagedArrayRest { .. })
        })
    });
    if needed {
        include_str!("array_rest.wat")
    } else {
        ""
    }
}
