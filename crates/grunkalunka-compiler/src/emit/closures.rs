//! Private function values carry a table index and an environment address.
//! Populated closure slots use (environment, numeric argument frame) -> Number.
use super::*;

pub(super) const FRAME_ALLOCATOR: &str = r#"(func $closure_frame (param $bytes i32) (result i32) (local $old i32) (local $end i32)
  local.get $bytes i32.const 1048576 i32.gt_u if unreachable end
  local.get $bytes i32.const 15 i32.and if unreachable end
  global.get $cell_top local.tee $old local.get $bytes i32.add local.tee $end
  i32.const 1048576 i32.gt_u if unreachable end
  local.get $end memory.size i32.const 16 i32.shl i32.gt_u
  if
    local.get $end i32.const 65535 i32.add i32.const 16 i32.shr_u
    memory.size i32.sub memory.grow i32.const -1 i32.eq if unreachable end
  end
  local.get $end global.set $cell_top
  local.get $old)
"#;

pub(super) fn table(module: &ir::Module) -> String {
    let mut output = format!(
        "(type $closure (func (param i32 i32) (result f64)))\n(table {} funcref)\n",
        module.functions.len()
    );
    for (i, function) in module.functions.iter().enumerate() {
        if ir::closure_environment(function).is_some() {
            writeln!(output, "(elem (i32.const {i}) func {i})").unwrap();
        }
    }
    output
}

pub(super) fn temporary(function: &Function, offset: usize) -> Option<usize> {
    let mut used = false;
    ir::visit_operations(&function.body, &mut |op| {
        used |= matches!(op, Op::MakeClosure { .. } | Op::ClosureCall { .. })
    });
    used.then_some(offset)
}

impl Emitter<'_> {
    pub(super) fn init_environment(&mut self) {
        if let Some(captures) = ir::closure_environment(self.function) {
            for parameter in 0..self.function.parameter_count {
                writeln!(
                    self.output,
                    "local.get 1\nf64.load offset={}\nlocal.set {}",
                    parameter * 8,
                    self.slot_offsets[parameter]
                )
                .unwrap();
            }
            for (index, slot) in captures.iter().enumerate() {
                writeln!(
                    self.output,
                    "local.get 0\ni32.load offset={}\nlocal.set {}",
                    index * 4,
                    self.cell_offsets[slot.0].expect("imported capture pointer")
                )
                .unwrap();
            }
        }
    }

    fn frame(&mut self, bytes: usize) -> usize {
        let temp = self.arena_temp.expect("closure frame local");
        let aligned = bytes.max(16).div_ceil(16) * 16;
        writeln!(
            self.output,
            "i32.const {aligned}\ncall $closure_frame\nlocal.set {temp}"
        )
        .unwrap();
        temp
    }

    pub(super) fn make_closure(
        &mut self,
        result: ValueId,
        function: ir::FunctionId,
        captures: &[ir::SlotId],
    ) {
        let temp = self.frame(captures.len() * 4);
        for (index, slot) in captures.iter().enumerate() {
            writeln!(
                self.output,
                "local.get {temp}\nlocal.get {}\ni32.store offset={}",
                self.cell_offsets[slot.0].expect("captured source cell"),
                index * 4
            )
            .unwrap();
        }
        writeln!(self.output, "i32.const {}\nlocal.get {temp}", function.0).unwrap();
        self.set(result);
    }

    pub(super) fn call_closure(&mut self, result: ValueId, callee: ValueId, arguments: &[ValueId]) {
        let temp = self.frame(arguments.len() * 8);
        for (index, value) in arguments.iter().enumerate() {
            writeln!(
                self.output,
                "local.get {temp}\nlocal.get {}\nf64.store offset={}",
                self.value_offsets[value.0],
                index * 8
            )
            .unwrap();
        }
        let callee = self.value_offsets[callee.0];
        writeln!(
            self.output,
            "local.get {}\nlocal.get {temp}\nlocal.get {callee}\ncall_indirect (type $closure)",
            callee + 1
        )
        .unwrap();
        self.set(result);
    }
}
