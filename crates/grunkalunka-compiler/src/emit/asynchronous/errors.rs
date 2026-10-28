pub(super) fn runtime() -> String {
    let original = "local.get $tag i32.const 3 i32.gt_u if unreachable end";
    let extended = "local.get $tag i32.const 4 i32.eq\nif local.get $payload call $error_record drop return end\n  local.get $tag i32.const 3 i32.gt_u if unreachable end";
    debug_assert_eq!(super::promises::RUNTIME.matches(original).count(), 1);
    let mut runtime = super::promises::RUNTIME.replacen(original, extended, 1);
    runtime.push_str(include_str!("errors.wat"));
    runtime
}
