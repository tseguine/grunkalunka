//! Guest-owned settlement records and FIFO reactions. Modules without Promise
//! identity IR retain the legacy coroutine ABI.
pub(super) const RUNTIME: &str = include_str!("promises.wat");

pub(super) const GLOBALS: &str = "(global $async_promise (mut i32) (i32.const 0))\n(global $root_state (mut i32) (i32.const 1))\n(global $root_tag (mut i32) (i32.const 0))\n(global $root_payload (mut f64) (f64.const 0))\n(global $started (mut i32) (i32.const 0))\n";

// Return the started child's identity to its synchronous caller exactly once.
// The parent pointer may be zero: the separate flag distinguishes a live link.
pub(super) fn park(output: &mut String) {
    output.push_str("global.get $async_frame\ni32.load offset=20\nif\nglobal.get $async_frame\ni32.const 0\ni32.store offset=20\nglobal.get $async_frame\ni32.load offset=16\nglobal.set $async_promise\nglobal.get $async_frame\ni32.load offset=4\nglobal.set $pc\nglobal.get $async_frame\ni32.load\nglobal.set $async_frame\nbr $dispatch\nend\ncall $root_result\nreturn\n");
}

pub(super) fn complete(
    output: &mut String,
    emitter: &super::Emitter<'_>,
    value: super::ValueId,
    root: bool,
    rejected: bool,
) {
    use std::fmt::Write;
    if root {
        super::primitive(output, emitter, value);
        output.push_str("global.set $root_payload\nglobal.set $root_tag\n");
        writeln!(
            output,
            "i32.const {}\nglobal.set $root_state\ncall $root_result\nreturn",
            if rejected { 3 } else { 0 }
        )
        .unwrap();
    } else {
        writeln!(
            output,
            "global.get $async_frame\ni32.load offset=16\ni32.const {}",
            u8::from(rejected)
        )
        .unwrap();
        super::primitive(output, emitter, value);
        output.push_str("call $promise_settle\ndrop\n");
        park(output);
    }
}

pub(super) const WRAPPERS: &str = r#"
(func $root_result (result i32 i32 f64)
  global.get $root_state global.get $root_tag global.get $root_payload)
(func (export "start") (param f64 f64) (result i32 i32 f64)
  global.get $started if unreachable end
  i32.const 1 global.set $started
  local.get 0 global.set 0 local.get 1 global.set 1
  i32.const 0 f64.const 0 call $advance)
(func (export "resume") (param i32 f64) (result i32 i32 f64) (local $job i32)
  global.get $started i32.eqz if unreachable end
  call $reaction_take local.tee $job i32.eqz if unreachable end
  local.get $job call $reaction_read
  global.set $async_payload global.set $async_tag global.set $async_rejected
  global.set $pc global.set $async_frame
  global.get $async_tag global.get $async_payload call $advance)
(func (export "take_jobs") (result i32) call $reaction_issued)
(func (export "take_promise_event") (result i32 i32 i32 f64) call $promise_event_take)
"#;
