//! Backend-owned invocation frames. Logical IR never names these offsets.
use super::*;

const HEADER: usize = 16;
pub(super) const GLOBALS: &str = "(global $async_frame (mut i32) (i32.const 0))\n(global $async_next (mut i32) (i32.const 0))\n(global $async_tag (mut i32) (i32.const 0))\n(global $async_payload (mut f64) (f64.const 0))\n(global $async_rejected (mut i32) (i32.const 0))\n";

pub(super) struct Frame<'a> {
    pub emitter: Emitter<'a>,
    pub lanes: Vec<&'static str>,
    spill_lanes: Vec<usize>,
    start: usize,
    header: usize,
    spill_template: String,
    restore_template: String,
}

// Spill and restore instructions are invariant for one physical frame.
// Render them once during frame planning, not again at every suspend,
// child invocation or entry state that needs the same complete lane set.
fn spill_restore_templates(
    lanes: &[&str],
    indices: &[usize],
    start: usize,
    header: usize,
) -> (String, String) {
    let mut spill = String::new();
    let mut restore = String::new();
    for &index in indices {
        let lane = lanes[index];
        writeln!(
            spill,
            "global.get $async_frame\nglobal.get {}\n{lane}.store offset={}",
            start + index,
            header + index * 8
        )
        .unwrap();
        writeln!(
            restore,
            "global.get $async_frame\n{lane}.load offset={}\nglobal.set {}",
            header + index * 8,
            start + index
        )
        .unwrap();
    }
    (spill, restore)
}

impl<'a> Frame<'a> {
    pub fn new(
        function: &'a ir::Function,
        start: usize,
        queued: bool,
        errors: bool,
        needs_spills: bool,
    ) -> Self {
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
        let arena_temp = closures::temporary(function, offset, false);
        lanes.resize(offset - start + usize::from(arena_temp.is_some()), "i32");
        let mut value_offsets = offsets[function.slots.len()..].to_vec();
        layout::share_fulfilled_storage(function, &mut value_offsets);
        // Fulfilled Promise snapshots may share their original General
        // value's two physical lanes. The unneeded reserved lanes stay in
        // the stable physical frame layout but need no save/restore traffic.
        let mut used = vec![false; lanes.len()];
        for (index, slot) in function.slots.iter().enumerate() {
            for part in 0..layout::value(slot.ty).parts.len() {
                used[offsets[index] - start + part] = true;
            }
        }
        for (index, ty) in function.values.iter().enumerate() {
            for part in 0..layout::value(*ty).parts.len() {
                used[value_offsets[index] - start + part] = true;
            }
        }
        for pointer in cell_offsets.iter().flatten() {
            used[*pointer - start] = true;
        }
        if let Some(pointer) = arena_temp {
            used[pointer - start] = true;
        }
        let spill_lanes = used
            .iter()
            .enumerate()
            .filter_map(|(index, present)| present.then_some(index))
            .collect::<Vec<_>>();
        let header = if queued { 32 } else { HEADER };
        let (spill_template, restore_template) = if needs_spills {
            spill_restore_templates(&lanes, &spill_lanes, start, header)
        } else {
            (String::new(), String::new())
        };
        Self {
            emitter: Emitter {
                function,
                slot_offsets: offsets[..function.slots.len()].to_vec(),
                value_offsets,
                cell_offsets,
                arena_temp,
                call_boundary: None,
                output: String::new(),
                loop_label: None,
                inline_return: None,
                throw_to: None,
                needs_remainder: false,
                errors,
                gc_closures: false,
            },
            lanes,
            spill_lanes,
            start,
            header,
            spill_template,
            restore_template,
        }
    }

    fn offset(&self, global: usize) -> usize {
        self.header + (global - self.start) * 8
    }

    pub fn parameter_offset(&self, parameter: usize, part: usize) -> usize {
        self.offset(self.emitter.slot_offsets[parameter] + part)
    }

    pub fn bytes(&self) -> usize {
        (self.header + self.lanes.len() * 8).div_ceil(16) * 16
    }

    pub fn spill(&self, output: &mut String) {
        // The unchanged empty template handles frames without coroutine children.
        output.push_str(&self.spill_template);
    }

    pub fn restore(&self, output: &mut String) {
        output.push_str(&self.restore_template);
    }

    pub fn take_output(&mut self, output: &mut String) {
        append_globalized(output, &self.emitter.output);
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

// The atomic emitter renders instructions against numeric-indexed locals.
// Coroutine frames store the same value lanes in globals. Copy the instructions
// directly into the destination, changing only those two instruction mnemonics.
// This keeps the old exact output without two whole-string allocations per state.
fn append_globalized(output: &mut String, instructions: &str) {
    let mut remaining = instructions;
    while let Some((prefix, suffix)) = remaining.split_once("local.") {
        output.push_str(prefix);
        if let Some(rest) = suffix.strip_prefix("get ") {
            output.push_str("global.get ");
            remaining = rest;
        } else if let Some(rest) = suffix.strip_prefix("set ") {
            output.push_str("global.set ");
            remaining = rest;
        } else {
            output.push_str("local.");
            remaining = suffix;
        }
    }
    output.push_str(remaining);
}

#[cfg(test)]
mod globalize_tests {
    use super::append_globalized;

    #[test]
    fn rewrites_only_get_and_set_in_one_pass() {
        for input in [
            "",
            "local.get 0",
            "local.set 14\\nlocal.get 3\\n",
            "global.get 9\\nlocal.get 0\\nlocal.set 1\\n",
            "local.tee 4\\nlocal.get 7\\nlocal.foo local.set 2",
            "a-local.get 1 local.local.set 9 local.get 11",
        ] {
            let mut output = "prefix:".to_string();
            append_globalized(&mut output, input);
            assert_eq!(
                output,
                format!(
                    "prefix:{}",
                    input
                        .replace("local.get ", "global.get ")
                        .replace("local.set ", "global.set ")
                ),
                "{input:?}"
            );
        }
    }

    #[test]
    fn long_instruction_sequence_retains_exact_existing_wat_bytes() {
        let input = (0..4096)
            .map(|i| format!("local.get {i}\\nlocal.set {i}\\n"))
            .collect::<String>();
        let mut result = String::new();
        append_globalized(&mut result, &input);
        assert_eq!(
            result,
            input
                .replace("local.get ", "global.get ")
                .replace("local.set ", "global.set ")
        );
    }
}

#[cfg(test)]
mod spill_lane_tests {
    use super::*;
    use crate::ir::{Operation, RegionId};
    use oxc_span::Span;

    fn function(aliased: bool) -> ir::Function {
        let span = Span::new(0, 0);
        let operations = if aliased {
            vec![Operation {
                span,
                kind: Op::FulfillPromise {
                    result: ValueId(1),
                    input: ValueId(0),
                },
            }]
        } else {
            Vec::new()
        };
        ir::Function {
            return_type: Type::General,
            parameter_count: 0,
            slots: Vec::new(),
            values: vec![Type::General, Type::FulfilledPromise, Type::Number],
            body: Region {
                id: RegionId(0),
                span,
                operations,
                exit: Exit::Fallthrough,
            },
        }
    }

    #[test]
    fn fulfilled_snapshot_alias_skips_only_unreferenced_physical_lanes() {
        let function = function(true);
        let frame = Frame::new(&function, 5, true, false, true);
        assert_eq!(frame.lanes.len(), 5);
        assert_eq!(frame.spill_lanes, vec![0, 1, 4]);
        assert_eq!(frame.emitter.value_offsets, vec![5, 5, 9]);

        let mut spill = String::new();
        frame.spill(&mut spill);
        let mut restore = String::new();
        frame.restore(&mut restore);
        for index in [5, 6, 9] {
            assert!(spill.contains(&format!("global.get {index}\n")));
            assert!(restore.contains(&format!("global.set {index}\n")));
        }
        for index in [7, 8] {
            assert!(!spill.contains(&format!("global.get {index}\n")));
            assert!(!restore.contains(&format!("global.set {index}\n")));
        }
        // Keeping the original frame allocation offsets preserves the
        // private ABI for values after a shared completion snapshot.
        assert_eq!(frame.bytes(), (32 + 5 * 8_usize).div_ceil(16) * 16);
    }

    #[test]
    fn distinct_values_keep_all_physical_lanes() {
        let function = function(false);
        let frame = Frame::new(&function, 0, false, false, true);
        assert_eq!(frame.spill_lanes, vec![0, 1, 2, 3, 4]);
    }
}
