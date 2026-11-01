pub(super) fn runtime() -> String {
    let original = "local.get $tag i32.const 3 i32.gt_u if unreachable end";
    let extended = "local.get $tag i32.const 4 i32.eq\nif local.get $payload call $error_record drop return end\n  local.get $tag i32.const 3 i32.gt_u if unreachable end";
    debug_assert_eq!(super::promises::RUNTIME.matches(original).count(), 1);
    let mut runtime = super::promises::RUNTIME.replacen(original, extended, 1);
    runtime.push_str(include_str!("errors.wat"));
    runtime
}

pub(super) fn managed_runtime() -> String {
    let original = "local.get $value struct.get $gc_value 0 i32.const 6 i32.eq i32.or";
    let extended = "local.get $value struct.get $gc_value 0 i32.const 6 i32.eq i32.or\n    local.get $value struct.get $gc_value 0 i32.const 7 i32.eq i32.or";
    let runtime = include_str!("../managed.wat");
    debug_assert_eq!(runtime.matches(original).count(), 1);
    // Equality already compares scalar payloads for non-reference kinds. GC
    // kind 7 carries the existing Error record identity, never an eqref or Number.
    runtime.replacen(original, extended, 1)
}
