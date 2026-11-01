//! One traced cell domain for synchronous managed captures, arguments and results.
//! Environments retain cells, not snapshots; argument entries hold fresh cells.
use super::*;

pub(super) const TYPES: &str = include_str!("gc_closures.wat");

pub(super) fn needed(module: &ir::Module) -> bool {
    module.functions.iter().any(|f| {
        (ir::closure_environment(f).is_some()
            && (f.return_type.is_managed()
                || f.slots
                    .iter()
                    .take(f.parameter_count)
                    .any(|slot| slot.ty.is_managed())))
            || ir::captured_cells(&f.body)
                .into_iter()
                .any(|slot| f.slots[slot.0].ty.is_managed())
    })
}

fn field(lane: layout::Lane) -> usize {
    match lane {
        layout::Lane::I32 => 0,
        layout::Lane::F64 => 1,
        layout::Lane::ManagedRef
        | layout::Lane::ObjectRef
        | layout::Lane::ArrayRef
        | layout::Lane::EnvironmentRef => 2,
    }
}
fn cast(lane: layout::Lane) -> &'static str {
    match lane {
        layout::Lane::ManagedRef => "ref.cast (ref null $gc_value)\n",
        layout::Lane::ObjectRef => "ref.cast (ref null $gc_object)\n",
        layout::Lane::ArrayRef => "ref.cast (ref null $gc_array)\n",
        layout::Lane::EnvironmentRef => "ref.cast (ref null $gc_frame)\n",
        layout::Lane::I32 | layout::Lane::F64 => "",
    }
}

impl Emitter<'_> {
    pub(super) fn load_gc_cell(&mut self, pointer: usize, ty: Type) {
        for part in self.value_layout(ty).parts {
            writeln!(
                self.output,
                "local.get {pointer}\nstruct.get $gc_cell {}",
                field(part.lane)
            )
            .unwrap();
            self.output.push_str(cast(part.lane));
        }
    }
    pub(super) fn store_gc_cell(&mut self, pointer: usize, local: usize, ty: Type) {
        for (index, part) in self.value_layout(ty).parts.iter().enumerate() {
            writeln!(
                self.output,
                "local.get {pointer}\nlocal.get {}\nstruct.set $gc_cell {}",
                local + index,
                field(part.lane)
            )
            .unwrap();
        }
    }
    fn gc_frame(&mut self, count: usize) -> usize {
        let temp = self.arena_temp.expect("GC closure frame local");
        writeln!(
            self.output,
            "i32.const {count}\narray.new_default $gc_frame\nlocal.set {temp}"
        )
        .unwrap();
        temp
    }
    pub(super) fn init_gc_environment(&mut self) {
        let Some(captures) = ir::closure_environment(self.function) else {
            return;
        };
        for parameter in 0..self.function.parameter_count {
            let ty = self.function.slots[parameter].ty;
            for (index, part) in self.value_layout(ty).parts.iter().enumerate() {
                writeln!(self.output,"local.get 1\ni32.const {parameter}\narray.get $gc_frame\nstruct.get $gc_cell {}",field(part.lane)).unwrap();
                self.output.push_str(cast(part.lane));
                writeln!(
                    self.output,
                    "local.set {}",
                    self.slot_offsets[parameter] + index
                )
                .unwrap();
            }
        }
        for (index, slot) in captures.iter().enumerate() {
            writeln!(
                self.output,
                "local.get 0\ni32.const {index}\narray.get $gc_frame\nlocal.set {}",
                self.cell_offsets[slot.0].expect("imported GC cell")
            )
            .unwrap();
        }
    }
    pub(super) fn make_gc_closure(
        &mut self,
        result: ValueId,
        function: ir::FunctionId,
        captures: &[ir::SlotId],
    ) {
        let temp = self.gc_frame(captures.len());
        for (index, slot) in captures.iter().enumerate() {
            writeln!(
                self.output,
                "local.get {temp}\ni32.const {index}\nlocal.get {}\narray.set $gc_frame",
                self.cell_offsets[slot.0].expect("captured GC cell")
            )
            .unwrap();
        }
        writeln!(self.output, "i32.const {}\nlocal.get {temp}", function.0).unwrap();
        self.set(result);
    }
    // Cells use separate unboxed fields for numeric lanes and one traced edge.
    fn gc_argument_cell(&mut self, local: usize, ty: Type) {
        for field_index in 0..3 {
            if let Some((index, _)) = self
                .value_layout(ty)
                .parts
                .iter()
                .enumerate()
                .find(|(_, part)| field(part.lane) == field_index)
            {
                writeln!(self.output, "local.get {}", local + index).unwrap();
            } else {
                self.output.push_str(match field_index {
                    0 => "i32.const 0\n",
                    1 => "f64.const 0\n",
                    _ => "ref.null eq\n",
                });
            }
        }
        self.output.push_str("struct.new $gc_cell\n");
    }
    pub(super) fn call_gc_closure(
        &mut self,
        result: ValueId,
        callee: ValueId,
        arguments: &[ValueId],
    ) {
        let temp = self.gc_frame(arguments.len());
        for (index, value) in arguments.iter().enumerate() {
            writeln!(self.output, "local.get {temp}\ni32.const {index}").unwrap();
            self.gc_argument_cell(self.value_offsets[value.0], self.function.values[value.0]);
            self.output.push_str("array.set $gc_frame\n");
        }
        let signature = match self.function.values[result.0] {
            Type::ManagedValue | Type::ManagedObject | Type::ManagedArray => "$closure_managed",
            Type::Closure(_) => "$closure_pair",
            Type::Promise => "$closure_promise",
            Type::General | Type::AsyncValue | Type::FulfilledPromise | Type::SettledPromise => {
                "$closure_general"
            }
            _ => "$closure",
        };
        let callee = self.value_offsets[callee.0];
        // Managed modules cannot use the async spill protocol; no numeric spill.
        assert!(self.call_boundary.is_none());
        writeln!(
            self.output,
            "local.get {}\nlocal.get {temp}\nlocal.get {callee}\ncall_indirect (type {signature})",
            callee + 1
        )
        .unwrap();
        let ty = self.function.values[result.0];
        if ty.is_managed() {
            // The private ABI shares eqref; the verified signature retains the
            // precise result kind, restored before assigning its typed local.
            let lane = self.value_layout(ty).parts[0].lane;
            self.output.push_str(cast(lane));
        }
        self.set(result);
    }
}
