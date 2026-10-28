//! Private function values carry a table index and an environment address.
//! Calls use (environment, typed argument frame) and a verified logical result.
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

pub(super) fn table(module: &ir::Module, outlined: bool) -> String {
    let mut output = format!(
        "(type $closure (func (param i32 i32) (result {})))\n(table {} funcref)\n",
        layout::value(Type::Number).wat,
        module.functions.len()
    );
    if module
        .functions
        .iter()
        .any(|function| matches!(function.return_type, Type::Closure(_)))
    {
        writeln!(
            output,
            "(type $closure_pair (func (param i32 i32) (result {})))",
            layout::value(Type::Closure(0)).wat
        )
        .unwrap();
    }
    if module.functions.iter().any(|function| {
        matches!(
            function.return_type,
            Type::General | Type::FulfilledPromise | Type::SettledPromise
        ) && ir::closure_environment(function).is_some()
    }) {
        writeln!(
            output,
            "(type $closure_general (func (param i32 i32) (result {})))",
            layout::value(Type::General).wat
        )
        .unwrap();
    }
    if module
        .functions
        .iter()
        .any(|f| f.return_type == Type::Promise)
    {
        output.push_str("(type $closure_promise (func (param i32 i32) (result i32)))\n");
    }
    for (i, function) in module.functions.iter().enumerate() {
        if ir::closure_environment(function).is_some() {
            let target = if outlined {
                format!("$closure_body{i}")
            } else {
                i.to_string()
            };
            writeln!(output, "(elem (i32.const {i}) func {target})").unwrap();
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
            let mut offset = 0;
            for parameter in 0..self.function.parameter_count {
                let ty = self.function.slots[parameter].ty;
                let local = self.slot_offsets[parameter];
                for (index, part) in layout::value(ty).parts.iter().enumerate() {
                    writeln!(
                        self.output,
                        "local.get 1\n{}.load offset={}\nlocal.set {}",
                        part.lane.wat(),
                        offset + part.byte_offset,
                        local + index
                    )
                    .unwrap();
                }
                offset += layout::argument_bytes(ty);
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
        let bytes = arguments
            .iter()
            .map(|value| layout::argument_bytes(self.function.values[value.0]))
            .sum();
        let temp = self.frame(bytes);
        let mut offset = 0;
        for value in arguments {
            let local = self.value_offsets[value.0];
            let ty = self.function.values[value.0];
            self.store_memory(temp, local, ty, offset, true);
            offset += layout::argument_bytes(ty);
        }
        let signature = match self.function.values[result.0] {
            Type::Closure(_) => "$closure_pair",
            Type::Promise => "$closure_promise",
            Type::General | Type::FulfilledPromise | Type::SettledPromise => "$closure_general",
            _ => "$closure",
        };
        let callee = self.value_offsets[callee.0];
        writeln!(
            self.output,
            "local.get {}\nlocal.get {temp}\nlocal.get {callee}\ncall_indirect (type {signature})",
            callee + 1
        )
        .unwrap();
        self.set(result);
    }
}
