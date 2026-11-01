use super::ir;

pub(super) fn runtime(module: &ir::Module) -> &'static str {
    let needed = module.functions.iter().any(|function| {
        ir::any_operation(&function.body, &mut |op| match op {
            ir::Op::ManagedOwnKeys { input, .. } => !matches!(
                function.values[input.0],
                ir::Type::Number | ir::Type::Closure(_)
            ),
            ir::Op::TryManagedOwnKeys { .. } => true,
            _ => false,
        })
    });
    if !needed {
        ""
    } else if super::managed_spread::needed(module) {
        include_str!("own_keys.wat")
    } else {
        concat!(
            include_str!("managed_index.wat"),
            include_str!("own_keys.wat")
        )
    }
}

pub(super) fn fallible(module: &ir::Module) -> bool {
    module.functions.iter().any(|f| {
        ir::any_operation(&f.body, &mut |op| {
            matches!(op, ir::Op::TryManagedOwnKeys { .. })
        })
    })
}

pub(super) const FALLIBLE: &str = include_str!("own_keys_fallible.wat");
