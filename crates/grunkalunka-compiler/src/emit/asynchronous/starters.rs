//! Callable async entries for synchronous helpers; starting does not await.
use super::*;

pub(super) fn emit(output: &mut String, frame: &frames::Frame<'_>, id: usize, entry: usize) {
    // The Wasm call stack retains these values even when the child's prefix
    // calls another synchronous helper which starts a further child.
    let ty = if frame.managed() {
        "(ref null $gc_frame)"
    } else {
        "i32"
    };
    let environment = if frame.managed() {
        ""
    } else {
        "local.get $child\nlocal.get $environment\ni32.store offset=8\n"
    };
    writeln!(output, "(func $closure_body{id} (param $environment {ty}) (param $arguments {ty}) (result i32)\n(local $caller i32) (local $continuation i32) (local $child i32)\nglobal.get $async_frame\nlocal.set $caller\nglobal.get $pc\nlocal.set $continuation\ni32.const {}\ncall $closure_frame\nlocal.set $child\n{environment}local.get $child\ncall $promise_new\ni32.store offset=16", frame.bytes()).unwrap();
    frame.register(output, "local.get $child", "local.get $environment");
    // Parent/startup fields stay zero: the child parks back to this Wasm call,
    // rather than following the dispatcher's parent continuation mechanism.
    let mut argument_offset = 0;
    for (index, slot) in frame
        .emitter
        .function
        .slots
        .iter()
        .take(frame.emitter.function.parameter_count)
        .enumerate()
    {
        for (part, lane) in frame.emitter.value_layout(slot.ty).parts.iter().enumerate() {
            if frame.managed() {
                let global = frame.emitter.slot_offsets[index] + part;
                if let Some(reference) = frame.reference_index(global) {
                    frame.reference_array(output, "local.get $child");
                    writeln!(output,"i32.const {reference}\nlocal.get $arguments\ni32.const {index}\narray.get $gc_frame\nstruct.get $gc_cell 2\narray.set $async_saved_refs").unwrap();
                } else {
                    writeln!(output,"local.get $child\nlocal.get $arguments\ni32.const {index}\narray.get $gc_frame\nstruct.get $gc_cell {}\n{}.store offset={}",super::super::gc_closures::field(lane.lane),lane.lane.wat(),frame.parameter_offset(index,part)).unwrap();
                }
                continue;
            }
            writeln!(
                output,
                "local.get $child\nlocal.get $arguments\n{}.load offset={}\n{}.store offset={}",
                lane.lane.wat(),
                argument_offset + lane.byte_offset,
                lane.lane.wat(),
                frame.parameter_offset(index, part)
            )
            .unwrap();
        }
        argument_offset += layout::argument_bytes(slot.ty);
    }
    writeln!(output, "local.get $child\nglobal.set $async_frame\ni32.const {entry}\nglobal.set $pc\ni32.const 0\nf64.const 0\ncall $advance\ndrop\ndrop\ndrop\nlocal.get $caller\nglobal.set $async_frame\nlocal.get $continuation\nglobal.set $pc\nlocal.get $child\ni32.load offset=16\n)").unwrap();
}
