//! Copy the remaining data fields of a proven object into a fresh object.
use super::ir;

pub(super) fn runtime(module: &ir::Module) -> &'static str {
    if module.functions.iter().any(|function| {
        ir::any_operation(&function.body, &mut |op| {
            matches!(op, ir::Op::ManagedObjectRest { .. })
        })
    }) {
        include_str!("object_rest.wat")
    } else {
        ""
    }
}
