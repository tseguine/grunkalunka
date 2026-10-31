//! Stable typed cells for captured bindings. Numeric-only modules use an
//! invocation arena; managed-capture modules use traced GC cells throughout.
use super::*;

pub(super) const MEMORY: &str = "(memory 1 16)\n(global $cell_top (mut i32) (i32.const 0))\n";

pub(super) const ALLOCATOR: &str = r#"(func $cell_new (result i32) (local $old i32)
  global.get $cell_top local.tee $old
  i32.const 16 i32.add
  i32.const 1048576 i32.gt_u if unreachable end
  local.get $old i32.const 16 i32.add
  memory.size i32.const 16 i32.shl i32.gt_u
  if
    i32.const 1 memory.grow i32.const -1 i32.eq if unreachable end
  end
  local.get $old i32.const 16 i32.add global.set $cell_top
  local.get $old)
"#;

pub(super) fn offsets(function: &Function, offset: &mut usize) -> Vec<Option<usize>> {
    let mut offsets = vec![None; function.slots.len()];
    for slot in ir::captured_cells(&function.body) {
        offsets[slot.0] = Some(*offset);
        *offset += 1;
    }
    offsets
}

impl Emitter<'_> {
    pub(super) fn init_cell_parameters(&mut self) {
        for slot in 0..self.function.parameter_count {
            if let Some(pointer) = self.cell_offsets[slot] {
                self.new_cell(pointer);
                let local = self.slot_offsets[slot];
                if self.gc_closures {
                    self.store_gc_cell(pointer, local, self.function.slots[slot].ty);
                } else {
                    self.store_memory(pointer, local, self.function.slots[slot].ty, 0, false);
                }
            }
        }
    }

    fn new_cell(&mut self, pointer: usize) {
        if self.gc_closures {
            writeln!(
                self.output,
                "struct.new_default $gc_cell\nlocal.set {pointer}"
            )
            .unwrap();
        } else {
            writeln!(self.output, "call $cell_new\nlocal.set {pointer}").unwrap();
        }
    }

    pub(super) fn load_cell(&mut self, slot: ir::SlotId, pointer: usize) {
        if self.gc_closures {
            self.load_gc_cell(pointer, self.function.slots[slot.0].ty);
            return;
        }
        for part in layout::value(self.function.slots[slot.0].ty).parts {
            write!(self.output, "local.get {pointer}\n{}.load", part.lane.wat()).unwrap();
            if part.byte_offset != 0 {
                write!(self.output, " offset={}", part.byte_offset).unwrap();
            }
            self.output.push('\n');
        }
    }

    pub(super) fn store_cell(
        &mut self,
        slot: ir::SlotId,
        value: ValueId,
        pointer: usize,
        initialize: bool,
    ) {
        if initialize {
            self.new_cell(pointer);
        }
        let offset = self.value_offsets[value.0];
        if self.gc_closures {
            self.store_gc_cell(pointer, offset, self.function.slots[slot.0].ty);
        } else {
            self.store_memory(pointer, offset, self.function.slots[slot.0].ty, 0, false);
        }
    }

    // Explicit zero offsets retain the existing argument-frame WAT spelling;
    // both frame and cell access use the same lane and byte-offset policy.
    pub(super) fn store_memory(
        &mut self,
        pointer: usize,
        local: usize,
        ty: Type,
        base: usize,
        explicit_zero: bool,
    ) {
        for (index, part) in layout::value(ty).parts.iter().enumerate() {
            let offset = base + part.byte_offset;
            write!(
                self.output,
                "local.get {pointer}\nlocal.get {}\n{}.store",
                local + index,
                part.lane.wat()
            )
            .unwrap();
            if offset != 0 || explicit_zero {
                write!(self.output, " offset={offset}").unwrap();
            }
            self.output.push('\n');
        }
    }
}
