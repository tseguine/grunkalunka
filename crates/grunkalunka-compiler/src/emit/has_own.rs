use super::ir;

pub(super) fn runtime(module: &ir::Module) -> &'static str {
    if module.functions.iter().any(|function| {
        ir::any_operation(&function.body, &mut |op| match op {
            ir::Op::ManagedHasOwn { input, .. } => function.values[input.0].is_managed(),
            ir::Op::ManagedHasProperty { .. } => true,
            _ => false,
        })
    }) {
        include_str!("has_own.wat")
    } else {
        ""
    }
}
