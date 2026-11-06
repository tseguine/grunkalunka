//! Private UTF-16 helpers are included only when the corresponding IR is used.
use super::ir;

pub(super) fn runtime(module: &ir::Module) -> &'static str {
    if module.functions.iter().any(|function| {
        ir::any_operation(&function.body, &mut |op| {
            matches!(
                op,
                ir::Op::ManagedStringRange { .. }
                    | ir::Op::ManagedStringCodePointAt { .. }
                    | ir::Op::ManagedStringAt { .. }
                    | ir::Op::ManagedStringCharAt { .. }
                    | ir::Op::ManagedStringCharCodeAt { .. }
                    | ir::Op::ManagedStringSearch { .. }
                    | ir::Op::ManagedStringToWellFormed { .. }
                    | ir::Op::ManagedStringIsWellFormed { .. }
                    | ir::Op::ManagedStringTrim { .. }
                    | ir::Op::ManagedStringIndex { .. }
            )
        })
    }) {
        include_str!("string_methods.wat")
    } else {
        ""
    }
}

pub(super) fn iteration_runtime(module: &ir::Module) -> &'static str {
    if module.functions.iter().any(|f| {
        ir::any_operation(&f.body, &mut |op| {
            matches!(
                op,
                ir::Op::ManagedStringIterValue { .. } | ir::Op::ManagedStringToArray { .. }
            )
        })
    }) {
        include_str!("string_iterator.wat")
    } else {
        ""
    }
}

pub(super) fn spread_runtime(module: &ir::Module) -> &'static str {
    if module.functions.iter().any(|f| {
        ir::any_operation(&f.body, &mut |op| {
            matches!(op, ir::Op::ManagedStringToArray { .. })
        })
    }) {
        include_str!("string_spread.wat")
    } else {
        ""
    }
}
