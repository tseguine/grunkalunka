//! Backend-owned invocation frames. Logical IR never names these offsets.
use super::*;

const HEADER: usize = 16;
pub(super) const GLOBALS: &str = "(global $async_frame (mut i32) (i32.const 0))\n(global $async_next (mut i32) (i32.const 0))\n(global $async_tag (mut i32) (i32.const 0))\n(global $async_payload (mut f64) (f64.const 0))\n(global $async_rejected (mut i32) (i32.const 0))\n";

pub(super) struct Frame<'a> {
    pub emitter: Emitter<'a>,
    pub lanes: Vec<&'static str>,
    start: usize,
    header: usize,
}

impl<'a> Frame<'a> {
    pub fn new(function: &'a ir::Function, start: usize, queued: bool) -> Self {
        let mut lanes = Vec::new();
        let mut offsets = Vec::new();
        for ty in function
            .slots
            .iter()
            .map(|slot| slot.ty)
            .chain(function.values.iter().copied())
        {
            offsets.push(start + lanes.len());
            lanes.extend(layout::value(ty).parts.iter().map(|part| part.lane.wat()));
        }
        let mut offset = start + lanes.len();
        let cell_offsets = cells::offsets(function, &mut offset);
        let arena_temp = closures::temporary(function, offset);
        lanes.resize(offset - start + usize::from(arena_temp.is_some()), "i32");
        Self {
            emitter: Emitter {
                function,
                slot_offsets: offsets[..function.slots.len()].to_vec(),
                value_offsets: offsets[function.slots.len()..].to_vec(),
                cell_offsets,
                arena_temp,
                output: String::new(),
                loop_label: None,
                inline_return: None,
                throw_to: None,
                needs_remainder: false,
            },
            lanes,
            start,
            header: if queued { 32 } else { HEADER },
        }
    }

    fn offset(&self, global: usize) -> usize {
        self.header + (global - self.start) * 8
    }

    pub fn bytes(&self) -> usize {
        (self.header + self.lanes.len() * 8).div_ceil(16) * 16
    }

    pub fn spill(&self, output: &mut String) {
        for (index, lane) in self.lanes.iter().enumerate() {
            writeln!(
                output,
                "global.get $async_frame\nglobal.get {}\n{lane}.store offset={}",
                self.start + index,
                self.header + index * 8
            )
            .unwrap();
        }
    }

    pub fn restore(&self, output: &mut String) {
        for (index, lane) in self.lanes.iter().enumerate() {
            writeln!(
                output,
                "global.get $async_frame\n{lane}.load offset={}\nglobal.set {}",
                self.header + index * 8,
                self.start + index
            )
            .unwrap();
        }
    }

    pub fn take_output(&mut self, output: &mut String) {
        output.push_str(
            &self
                .emitter
                .output
                .replace("local.get ", "global.get ")
                .replace("local.set ", "global.set "),
        );
        self.emitter.output.clear();
    }

    pub fn enter(&mut self, output: &mut String) {
        let function = self.emitter.function;
        for slot in 0..function.parameter_count {
            let global = self.emitter.slot_offsets[slot];
            for (part, lane) in layout::value(function.slots[slot].ty)
                .parts
                .iter()
                .enumerate()
            {
                writeln!(
                    output,
                    "global.get $async_frame\n{}.load offset={}\nglobal.set {}",
                    lane.lane.wat(),
                    self.offset(global + part),
                    global + part
                )
                .unwrap();
            }
        }
        for (index, slot) in ir::closure_environment(function)
            .expect("async environment")
            .iter()
            .enumerate()
        {
            writeln!(
                output,
                "global.get $async_frame\ni32.load offset=8\ni32.load offset={}\nglobal.set {}",
                index * 4,
                self.emitter.cell_offsets[slot.0].expect("imported capture")
            )
            .unwrap();
        }
        self.emitter.init_cell_parameters();
        self.take_output(output);
    }

    pub fn invoke(
        &self,
        output: &mut String,
        target: &Self,
        callee: ValueId,
        arguments: &[ValueId],
        returned: usize,
        entry: usize,
    ) {
        writeln!(output,"i32.const {}\ncall $closure_frame\nglobal.set $async_next\nglobal.get $async_next\nglobal.get $async_frame\ni32.store\nglobal.get $async_next\ni32.const {returned}\ni32.store offset=4\nglobal.get $async_next\nglobal.get {}\ni32.store offset=8",target.bytes(),self.emitter.value_offsets[callee.0]+1).unwrap();
        if self.header != HEADER {
            output.push_str("global.get $async_next\ncall $promise_new\ni32.store offset=16\nglobal.get $async_next\ni32.const 1\ni32.store offset=20\n");
        }
        for (index, argument) in arguments.iter().enumerate() {
            let source = self.emitter.value_offsets[argument.0];
            let destination = target.emitter.slot_offsets[index];
            for (part, lane) in layout::value(self.emitter.function.values[argument.0])
                .parts
                .iter()
                .enumerate()
            {
                writeln!(
                    output,
                    "global.get $async_next\nglobal.get {}\n{}.store offset={}",
                    source + part,
                    lane.lane.wat(),
                    target.offset(destination + part)
                )
                .unwrap();
            }
        }
        output.push_str("global.get $async_next\nglobal.set $async_frame\n");
        jump(output, entry);
    }
}

pub(super) fn complete(output: &mut String, rejected: bool) {
    writeln!(output,"global.set $async_payload\nglobal.set $async_tag\ni32.const {}\nglobal.set $async_rejected\nglobal.get $async_frame\ni32.load offset=4\nglobal.set $pc\nglobal.get $async_frame\ni32.load\nglobal.set $async_frame\nbr $dispatch",u8::from(rejected)).unwrap();
}
