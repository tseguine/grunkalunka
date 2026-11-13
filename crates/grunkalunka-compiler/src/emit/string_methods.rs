//! Private UTF-16 helpers are included only when the corresponding IR is used.
use super::ir;

pub(super) fn runtime(module: &ir::Module) -> &'static str {
    if module.functions.iter().any(|function| {
        ir::any_operation(&function.body, &mut |op| {
            matches!(
                op,
                ir::Op::ManagedStringRange { .. }
                    | ir::Op::ManagedStringCodePointAt { .. }
                    | ir::Op::ManagedStringGetIndex { .. }
                    | ir::Op::ManagedStringGetKey { .. }
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

// The property-key helper (and its Number formatter) are linked only for
// String own-key lookup, not for every String method.
pub(super) fn key_runtime(module: &ir::Module) -> &'static str {
    if module.functions.iter().any(|function| {
        ir::any_operation(&function.body, &mut |op| {
            matches!(op, ir::Op::ManagedStringGetKey { .. })
        })
    }) {
        include_str!("string_get_key.wat")
    } else {
        ""
    }
}

pub(super) fn repeat_runtime(module: &ir::Module) -> &'static str {
    if module.functions.iter().any(|function| {
        ir::any_operation(&function.body, &mut |op| {
            matches!(op, ir::Op::ManagedStringRepeat { .. })
        })
    }) {
        include_str!("string_repeat.wat")
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

#[cfg(test)]
mod boxed_key_linkage_tests {
    #[test]
    fn ordinary_string_methods_do_not_link_unused_property_key_converter() {
        let source = "export function run(a,b){const s='abc';return s.charCodeAt(0);}";
        let artifact = crate::compile(source, false).expect("ordinary String method compiles");
        assert!(!artifact.wat.contains("call $gc_property_key"));
    }

    #[test]
    fn computed_primitive_string_keys_link_the_converter_and_getter() {
        let source = "export function run(a,b){const s='ab';const k=a?0:false;return s[k]===(a?'a':undefined)?1:0;}";
        let artifact = crate::compile(source, false).expect("boxed String key compiles");
        assert!(artifact.wat.contains("call $gc_property_key"));
        assert!(artifact.wat.contains("(func $gc_string_get_key"));
    }
}
