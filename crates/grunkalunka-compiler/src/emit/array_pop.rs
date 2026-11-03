use super::ir;

pub(super) fn runtime(module: &ir::Module) -> &'static str {
    let needed = module.functions.iter().any(|function| {
        ir::any_operation(&function.body, &mut |op| {
            matches!(op, ir::Op::ManagedArrayPop { .. })
        })
    });
    if !needed {
        ""
    } else if super::managed_spread::needed(module)
        || super::own_keys::needed(module)
        || super::array_at::needed(module)
    {
        include_str!("array_pop.wat")
    } else {
        concat!(
            include_str!("managed_index.wat"),
            include_str!("array_pop.wat")
        )
    }
}
