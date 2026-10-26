//! Direct structured Wasm emission. Every computation and source load is
//! materialized immediately: later source-cell stores cannot change a value.
use std::fmt::Write;

mod asynchronous;
mod cells;
mod closures;
pub(crate) use asynchronous::emit_async;

use crate::ir::{self, Binary, Compare, Exit, Function, Op, Region, Type, Unary, ValueId};

// Original integer-significand implementation of ECMAScript Number::remainder.
// After classification, r < my < 2^53. Each nonzero reduction advances at least
// min(d, 11) exponent bits, bounding the loop by ceil(2045 / 11) = 186 reductions.
// Zero is guarded before clz/reconstruction; ADD preserves the implicit-bit carry.
// This private helper is appended after source functions, only when emitted IR uses it.
const NUMBER_REMAINDER: &str = r#"(func $number_remainder (param $a f64) (param $b f64) (result f64)
  (local $x i64) (local $y i64) (local $sign i64)
  (local $ex i64) (local $ey i64) (local $mx i64) (local $my i64)
  (local $r i64) (local $d i64) (local $k i64) (local $s i64)
  (local.set $sign (i64.and (i64.reinterpret_f64 (local.get $a)) (i64.const 0x8000000000000000)))
  (local.set $x (i64.and (i64.reinterpret_f64 (local.get $a)) (i64.const 0x7fffffffffffffff)))
  (local.set $y (i64.and (i64.reinterpret_f64 (local.get $b)) (i64.const 0x7fffffffffffffff)))
  (if (i32.or (i64.ge_u (local.get $x) (i64.const 0x7ff0000000000000))
        (i32.or (i64.gt_u (local.get $y) (i64.const 0x7ff0000000000000)) (i64.eqz (local.get $y))))
    (then (return (f64.const nan))))
  (if (i64.lt_u (local.get $x) (local.get $y)) (then (return (local.get $a))))
  (if (i64.eq (local.get $x) (local.get $y)) (then (return (f64.reinterpret_i64 (local.get $sign)))))
  (local.set $ex (i64.shr_u (local.get $x) (i64.const 52)))
  (local.set $ey (i64.shr_u (local.get $y) (i64.const 52)))
  (local.set $mx (i64.and (local.get $x) (i64.const 0xfffffffffffff)))
  (local.set $my (i64.and (local.get $y) (i64.const 0xfffffffffffff)))
  (if (i64.eqz (local.get $ex)) (then) (else
   (local.set $mx (i64.or (local.get $mx) (i64.const 0x10000000000000)))
   (local.set $ex (i64.sub (local.get $ex) (i64.const 1)))))
  (if (i64.eqz (local.get $ey)) (then) (else
   (local.set $my (i64.or (local.get $my) (i64.const 0x10000000000000)))
   (local.set $ey (i64.sub (local.get $ey) (i64.const 1)))))
  (local.set $r (i64.rem_u (local.get $mx) (local.get $my)))
  (local.set $d (i64.sub (local.get $ex) (local.get $ey)))
  (block $done (loop $reduce
   (br_if $done (i32.or (i64.eqz (local.get $r)) (i64.eqz (local.get $d))))
   (local.set $k (i64.clz (local.get $r)))
   (local.set $k (select (local.get $d) (local.get $k) (i64.lt_u (local.get $d) (local.get $k))))
   (local.set $r (i64.rem_u (i64.shl (local.get $r) (local.get $k)) (local.get $my)))
   (local.set $d (i64.sub (local.get $d) (local.get $k)))
   (br $reduce)))
  (if (i64.eqz (local.get $r)) (then (return (f64.reinterpret_i64 (local.get $sign)))))
  (local.set $s (i64.sub (i64.clz (local.get $r)) (i64.const 11)))
  (local.set $s (select (local.get $ey) (local.get $s) (i64.lt_u (local.get $ey) (local.get $s))))
  (f64.reinterpret_i64 (i64.or (local.get $sign)
   (i64.add (i64.shl (local.get $r) (local.get $s))
    (i64.shl (i64.sub (local.get $ey) (local.get $s)) (i64.const 52))))))
"#;

struct Emitter<'a> {
    function: &'a Function,
    slot_offsets: Vec<usize>,
    value_offsets: Vec<usize>,
    cell_offsets: Vec<Option<usize>>,
    arena_temp: Option<usize>,
    output: String,
    // One ID names paired exit/body-end targets, active only inside its loop body.
    loop_label: Option<usize>,
    inline_return: Option<usize>,
    needs_remainder: bool,
}

impl Emitter<'_> {
    fn get(&mut self, value: ValueId) {
        writeln!(self.output, "local.get {}", self.value_offsets[value.0]).unwrap();
        if self.function.values[value.0].width() == 2 {
            writeln!(self.output, "local.get {}", self.value_offsets[value.0] + 1).unwrap();
        }
    }
    fn set(&mut self, value: ValueId) {
        if self.function.values[value.0].width() == 2 {
            writeln!(self.output, "local.set {}", self.value_offsets[value.0] + 1).unwrap();
        }
        writeln!(self.output, "local.set {}", self.value_offsets[value.0]).unwrap();
    }

    // Reduce a captured Number modulo 2^32 before conversion; saturation alone
    // would clamp large numbers. Nonfinite inputs produce NaN, which saturates to 0.
    fn int32_bits(&mut self, input: ValueId) {
        self.get(input);
        self.output.push_str("f64.trunc\n");
        self.get(input);
        self.output.push_str(
            "f64.trunc\nf64.const 4294967296\nf64.div\nf64.trunc\nf64.const 4294967296\nf64.mul\nf64.sub\ni64.trunc_sat_f64_s\ni32.wrap_i64\n",
        );
    }

    fn operations(&mut self, operations: &[ir::Operation]) {
        for operation in operations {
            match &operation.kind {
                Op::CaptureCells { .. } => (),
                Op::ClosureEntry { .. } => (),
                Op::MakeClosure {
                    result,
                    function,
                    captures,
                } => self.make_closure(*result, *function, captures),
                Op::ClosureCall {
                    result,
                    callee,
                    arguments,
                } => self.call_closure(*result, *callee, arguments),
                Op::InlineCall { result, body } => {
                    let outer = self.inline_return.replace(body.id.0);
                    writeln!(self.output, "block $call{} (result f64)", body.id.0).unwrap();
                    self.region(body);
                    self.output.push_str("unreachable\nend\n");
                    self.inline_return = outer;
                    self.set(*result);
                }
                Op::Await { .. } => unreachable!("await uses the async emitter"),
                Op::Nullish { result, value } => {
                    let tag = match value {
                        ir::Nullish::Undefined => 0,
                        ir::Nullish::Null => 1,
                    };
                    writeln!(self.output, "i32.const {tag}\nf64.const 0").unwrap();
                    self.set(*result);
                }
                Op::Pack { result, input } => {
                    let boolean = self.function.values[input.0] == Type::Predicate;
                    writeln!(self.output, "i32.const {}", if boolean { 2 } else { 3 }).unwrap();
                    self.get(*input);
                    if boolean {
                        self.output.push_str("f64.convert_i32_u\n");
                    }
                    self.set(*result);
                }
                Op::Call {
                    result,
                    function,
                    arguments,
                } => {
                    for argument in arguments {
                        self.get(*argument);
                    }
                    writeln!(self.output, "call {}", function.0).unwrap();
                    self.set(*result);
                }
                Op::NumberFromPredicate { result, input } => {
                    self.get(*input);
                    self.output.push_str("f64.convert_i32_u\n");
                    self.set(*result);
                }
                Op::Not { result, input } => {
                    self.get(*input);
                    self.output.push_str("i32.eqz\n");
                    self.set(*result);
                }
                Op::Boolean { result, value } => {
                    writeln!(self.output, "i32.const {}", i32::from(*value)).unwrap();
                    self.set(*result);
                }
                Op::DoWhile { body, condition } => {
                    let label = body.id.0;
                    writeln!(self.output, "block $exit{label}\nloop $do{label}").unwrap();
                    let outer = self.loop_label.replace(label);
                    writeln!(self.output, "block $continue{label}").unwrap();
                    self.region(body);
                    self.output.push_str("end\n");
                    self.loop_label = outer;
                    self.region(condition);
                    writeln!(self.output, "br_if $do{label}\nend\nend").unwrap();
                }
                Op::Constant { result, value } => {
                    let literal = if value.is_nan() {
                        "nan".to_string()
                    } else if *value == f64::INFINITY {
                        "inf".to_string()
                    } else if *value == f64::NEG_INFINITY {
                        "-inf".to_string()
                    } else {
                        value.to_string()
                    };
                    writeln!(self.output, "f64.const {literal}").unwrap();
                    self.set(*result);
                }
                Op::Load { result, slot } => {
                    if let Some(pointer) = self.cell_offsets[slot.0] {
                        self.load_cell(*slot, pointer);
                        self.set(*result);
                        continue;
                    }
                    let offset = self.slot_offsets[slot.0];
                    writeln!(self.output, "local.get {offset}").unwrap();
                    if self.function.slots[slot.0].ty.width() == 2 {
                        writeln!(self.output, "local.get {}", offset + 1).unwrap();
                    }
                    self.set(*result);
                }
                Op::Initialize { slot, value } | Op::Store { slot, value } => {
                    if let Some(pointer) = self.cell_offsets[slot.0] {
                        self.store_cell(
                            *slot,
                            *value,
                            pointer,
                            matches!(operation.kind, Op::Initialize { .. }),
                        );
                        continue;
                    }
                    self.get(*value);
                    let offset = self.slot_offsets[slot.0];
                    if self.function.slots[slot.0].ty.width() == 2 {
                        writeln!(self.output, "local.set {}", offset + 1).unwrap();
                    }
                    writeln!(self.output, "local.set {offset}").unwrap();
                }
                Op::Unary {
                    result,
                    operator,
                    input,
                } => {
                    match operator {
                        Unary::Abs => {
                            self.get(*input);
                            self.output.push_str("f64.abs\n");
                        }
                        Unary::Sqrt => {
                            self.get(*input);
                            self.output.push_str("f64.sqrt\n");
                        }
                        Unary::Floor => {
                            self.get(*input);
                            self.output.push_str("f64.floor\n");
                        }
                        Unary::Ceil => {
                            self.get(*input);
                            self.output.push_str("f64.ceil\n");
                        }
                        Unary::Trunc => {
                            self.get(*input);
                            self.output.push_str("f64.trunc\n");
                        }
                        Unary::Plus => self.get(*input),
                        Unary::Negate => {
                            self.get(*input);
                            self.output.push_str("f64.neg\n");
                        }
                        Unary::BitNot => {
                            self.int32_bits(*input);
                            self.output
                                .push_str("i32.const -1\ni32.xor\nf64.convert_i32_s\n");
                        }
                    }
                    self.set(*result);
                }
                Op::Binary {
                    result,
                    operator,
                    left,
                    right,
                } => {
                    let (instruction, bitwise) = match operator {
                        Binary::Add => ("f64.add\n", false),
                        Binary::Subtract => ("f64.sub\n", false),
                        Binary::Multiply => ("f64.mul\n", false),
                        Binary::Divide => ("f64.div\n", false),
                        Binary::Remainder => {
                            self.needs_remainder = true;
                            ("call $number_remainder\n", false)
                        }
                        Binary::BitAnd => ("i32.and\n", true),
                        Binary::BitOr => ("i32.or\n", true),
                        Binary::BitXor => ("i32.xor\n", true),
                        Binary::ShiftLeft => ("i32.shl\n", true),
                        Binary::ShiftRight => ("i32.shr_s\n", true),
                        Binary::ShiftRightUnsigned => ("i32.shr_u\n", true),
                    };
                    if bitwise {
                        self.int32_bits(*left);
                        self.int32_bits(*right);
                    } else {
                        self.get(*left);
                        self.get(*right);
                    }
                    self.output.push_str(instruction);
                    if bitwise {
                        self.output
                            .push_str(if matches!(operator, Binary::ShiftRightUnsigned) {
                                "f64.convert_i32_u\n"
                            } else {
                                "f64.convert_i32_s\n"
                            });
                    }
                    self.set(*result);
                }
                Op::PrimitiveLooseEqual {
                    result,
                    left,
                    right,
                    negated,
                } => {
                    let left = self.value_offsets[left.0];
                    let right = self.value_offsets[right.0];
                    // Only four canonical kinds reach this operation. Nullish payloads
                    // are +0; Boolean payloads already equal their Number conversion.
                    writeln!(self.output,"local.get {left}\ni32.const 2\ni32.lt_u\nlocal.get {right}\ni32.const 2\ni32.lt_u\ni32.eq").unwrap();
                    writeln!(
                        self.output,
                        "local.get {}\nlocal.get {}\nf64.eq\ni32.and",
                        left + 1,
                        right + 1
                    )
                    .unwrap();
                    if *negated {
                        self.output.push_str("i32.eqz\n");
                    }
                    self.set(*result);
                }
                Op::PrimitiveStrictEqual {
                    result,
                    left,
                    right,
                    negated,
                } => {
                    let left = self.value_offsets[left.0];
                    let right = self.value_offsets[right.0];
                    writeln!(self.output, "local.get {left}\nlocal.get {right}\ni32.eq").unwrap();
                    writeln!(
                        self.output,
                        "local.get {}\nlocal.get {}\nf64.eq\ni32.and",
                        left + 1,
                        right + 1
                    )
                    .unwrap();
                    if *negated {
                        self.output.push_str("i32.eqz\n");
                    }
                    self.set(*result);
                }
                Op::Compare {
                    result,
                    operator,
                    left,
                    right,
                } => {
                    self.get(*left);
                    self.get(*right);
                    self.output.push_str(match operator {
                        Compare::Less => "f64.lt\n",
                        Compare::LessEqual => "f64.le\n",
                        Compare::Greater => "f64.gt\n",
                        Compare::GreaterEqual => "f64.ge\n",
                        Compare::Equal => "f64.eq\n",
                        Compare::NotEqual => "f64.ne\n",
                    });
                    self.set(*result);
                }
                Op::NumberFromGeneral { result, input } => {
                    // Canonical null/Boolean/Number payloads already equal ToNumber.
                    // Undefined alone needs NaN instead of its canonical zero payload.
                    writeln!(self.output, "local.get {}", self.value_offsets[input.0]).unwrap();
                    self.output
                        .push_str("i32.eqz\nif (result f64)\nf64.const nan\nelse\n");
                    writeln!(self.output, "local.get {}", self.value_offsets[input.0] + 1).unwrap();
                    self.output.push_str("end\n");
                    self.set(*result);
                }
                Op::GeneralTruthy { result, input } => {
                    // Canonical nullish payloads are zero, Boolean payloads are 0/1;
                    // Number payloads keep their value. This implements all four kinds.
                    writeln!(self.output, "local.get {}", self.value_offsets[input.0] + 1).unwrap();
                    self.output.push_str("f64.abs\nf64.const 0\nf64.gt\n");
                    self.set(*result);
                }
                Op::Truthy { result, input } => {
                    self.get(*input);
                    // abs(x) > 0 is false exactly for +/-0 and NaN.
                    self.output.push_str("f64.abs\nf64.const 0\nf64.gt\n");
                    self.set(*result);
                }
                Op::Block(child) => self.region(child),
                Op::If {
                    result,
                    condition,
                    consequent,
                    alternate,
                } => {
                    self.get(*condition);
                    self.output
                        .push_str(match result.map(|id| self.function.values[id.0]) {
                            Some(Type::Number) => "if (result f64)\n",
                            Some(Type::Predicate) => "if (result i32)\n",
                            Some(Type::General) => "if (result i32 f64)\n",
                            Some(Type::Closure(_)) => "if (result i32 i32)\n",
                            None => "if\n",
                        });
                    self.region(consequent);
                    self.output.push_str("else\n");
                    self.region(alternate);
                    self.output.push_str("end\n");
                    if let Some(result) = result {
                        self.set(*result);
                    }
                }
                Op::While {
                    condition,
                    body,
                    step,
                } => {
                    let label = condition.id.0;
                    writeln!(self.output, "block $exit{label}\nloop $loop{label}").unwrap();
                    // The entire condition region executes on every visit, including
                    // the final failed test. Its values never escape to the body or step.
                    self.region(condition);
                    writeln!(self.output, "i32.eqz\nbr_if $exit{label}").unwrap();
                    let outer = self.loop_label.replace(label);
                    writeln!(self.output, "block $continue{label}").unwrap();
                    self.region(body);
                    self.output.push_str("end\n");
                    self.loop_label = outer;
                    if let Some(step) = step {
                        self.region(step);
                    }
                    writeln!(self.output, "br $loop{label}\nend\nend").unwrap();
                }
            }
        }
    }

    fn region(&mut self, region: &Region) {
        self.operations(&region.operations);
        match region.exit {
            Exit::Fallthrough => {}
            Exit::Break => {
                writeln!(
                    self.output,
                    "br $exit{}",
                    self.loop_label.expect("verified loop break")
                )
                .unwrap();
            }
            Exit::Continue => {
                writeln!(
                    self.output,
                    "br $continue{}",
                    self.loop_label.expect("verified loop continue")
                )
                .unwrap();
            }
            Exit::Yield(value) => self.get(value),
            Exit::Return(value) => {
                self.get(value);
                if let Some(label) = self.inline_return {
                    writeln!(self.output, "br $call{label}").unwrap();
                } else {
                    self.output.push_str("return\n");
                }
            }
        }
    }
}

pub(crate) fn emit(module: &ir::Module) -> Result<String, String> {
    ir::verify_module(module)?;
    emit_verified(module, Type::Number)
}

pub(crate) fn emit_with_entry_type(
    module: &ir::Module,
    entry_type: Type,
) -> Result<String, String> {
    ir::verify_module_with_entry_type(module, entry_type)?;
    emit_verified(module, entry_type)
}

fn emit_verified(module: &ir::Module, entry_type: Type) -> Result<String, String> {
    if module
        .functions
        .iter()
        .any(|function| region_has_await(&function.body))
    {
        return Err("await IR requires the async emitter".into());
    }
    let mut output = String::from("(module ");
    let has_closures = module
        .functions
        .iter()
        .any(|function| ir::closure_environment(function).is_some());
    let has_cells = has_closures
        || module
            .functions
            .iter()
            .any(|f| !ir::captured_cells(&f.body).is_empty());
    if has_cells {
        output.push_str(cells::MEMORY);
    }
    if has_closures {
        output.push_str(&closures::table(module, false));
    }
    let mut needs_remainder = false;
    for (index, function) in module.functions.iter().enumerate() {
        let environment = ir::closure_environment(function);
        let mut header = if index == 0 && !has_cells {
            String::from("(func (export \"run\")")
        } else {
            String::from("(func")
        };
        if environment.is_some() {
            header.push_str(" (param i32 i32)");
        } else if function.parameter_count > 0 {
            header.push_str(" (param");
            for _ in 0..function.parameter_count {
                header.push_str(" f64");
            }
            header.push(')');
        }
        header.push_str(if index == 0 && entry_type == Type::General {
            " (result i32 f64)\n"
        } else {
            " (result f64)\n"
        });
        let (body, remainder) = emit_function(function, header);
        needs_remainder |= remainder;
        output.push_str(&body);
    }
    if needs_remainder {
        output.push('\n');
        output.push_str(NUMBER_REMAINDER);
    }
    if has_cells {
        output.push_str(cells::ALLOCATOR);
        output.push_str("(func (export \"run\") (param f64 f64)");
        output.push_str(if entry_type == Type::General {
            " (result i32 f64)\n"
        } else {
            " (result f64)\n"
        });
        output.push_str("i32.const 0 global.set $cell_top\nlocal.get 0 local.get 1 call 0)\n");
    }
    if has_closures {
        output.push_str(closures::FRAME_ALLOCATOR);
    }
    output.push_str(")\n");
    Ok(output)
}

fn region_has_await(region: &Region) -> bool {
    region
        .operations
        .iter()
        .any(|operation| match &operation.kind {
            Op::Await { .. } => true,
            Op::InlineCall { body, .. } => region_has_await(body),
            Op::Block(child) => region_has_await(child),
            Op::If {
                consequent,
                alternate,
                ..
            } => region_has_await(consequent) || region_has_await(alternate),
            Op::While {
                condition,
                body,
                step,
            } => {
                region_has_await(condition)
                    || region_has_await(body)
                    || step.as_ref().is_some_and(region_has_await)
            }
            Op::DoWhile { body, condition } => {
                region_has_await(body) || region_has_await(condition)
            }
            _ => false,
        })
}

/// Emit one ordinary activation; coroutine frames use a separate storage mapping.
fn emit_function(function: &Function, header: String) -> (String, bool) {
    let environment = ir::closure_environment(function);
    let mut offset = if environment.is_some() { 2 } else { 0 };
    let slot_offsets = function
        .slots
        .iter()
        .map(|slot| {
            let current = offset;
            offset += slot.ty.width();
            current
        })
        .collect();
    let value_offsets = function
        .values
        .iter()
        .map(|ty| {
            let current = offset;
            offset += ty.width();
            current
        })
        .collect();
    let cell_offsets = cells::offsets(function, &mut offset);
    let arena_temp = closures::temporary(function, offset);
    let mut emitter = Emitter {
        function,
        slot_offsets,
        value_offsets,
        cell_offsets,
        arena_temp,
        loop_label: None,
        inline_return: None,
        needs_remainder: false,
        output: header,
    };
    // Source cells and typed value snapshots occupy disjoint physical locals.
    for slot in function.slots.iter().skip(if environment.is_some() {
        0
    } else {
        function.parameter_count
    }) {
        emitter.output.push_str(match slot.ty {
            Type::Number => "(local f64)\n",
            Type::Predicate => "(local i32)\n",
            Type::General => "(local i32 f64)\n",
            Type::Closure(_) => "(local i32 i32)\n",
        });
    }
    for ty in &function.values {
        emitter.output.push_str(match ty {
            Type::Number => "(local f64)\n",
            Type::Predicate => "(local i32)\n",
            Type::General => "(local i32 f64)\n",
            Type::Closure(_) => "(local i32 i32)\n",
        });
    }
    for _ in emitter.cell_offsets.iter().flatten() {
        emitter.output.push_str("(local i32)\n");
    }
    if arena_temp.is_some() {
        emitter.output.push_str("(local i32)\n");
    }
    emitter.init_environment();
    emitter.init_cell_parameters();
    emitter.region(&function.body);
    // The verifier proved return coverage. This only helps Wasm validation after
    // nested void branches whose arms return; it supplies no source semantics.
    emitter.output.push_str("unreachable\n)");
    (emitter.output, emitter.needs_remainder)
}
