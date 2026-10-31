//! Direct structured Wasm emission. Every computation and source load is
//! materialized immediately: later source-cell stores cannot change a value.
use std::fmt::Write;

mod asynchronous;
mod cells;
mod closures;
#[cfg(test)]
mod iterative_emitter_tests;
mod layout;
mod materialization;
mod relational;
#[cfg(test)]
mod stack_hygiene_tests;
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

#[derive(Clone, Copy)]
enum ThrowTarget {
    Settlement(usize),
    Catch { label: usize, reason: ValueId },
}

struct Emitter<'a> {
    function: &'a Function,
    slot_offsets: Vec<usize>,
    value_offsets: Vec<usize>,
    cell_offsets: Vec<Option<usize>>,
    arena_temp: Option<usize>,
    call_boundary: Option<Box<CallBoundary>>,
    output: String,
    // One ID names paired exit/body-end targets, active only inside its loop body.
    loop_label: Option<usize>,
    inline_return: Option<usize>,
    throw_to: Option<ThrowTarget>,
    needs_remainder: bool,
    errors: bool,
}

// Ordinary helpers use Wasm locals. A coroutine caller additionally preserves
// its persistent lanes when a helper may start another coroutine synchronously.
struct CallBoundary {
    spill: String,
    restore: String,
}

impl Emitter<'_> {
    fn get(&mut self, value: ValueId) {
        writeln!(self.output, "local.get {}", self.value_offsets[value.0]).unwrap();
        if layout::value(self.function.values[value.0]).parts.len() == 2 {
            writeln!(self.output, "local.get {}", self.value_offsets[value.0] + 1).unwrap();
        }
    }
    fn set(&mut self, value: ValueId) {
        if layout::value(self.function.values[value.0]).parts.len() == 2 {
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

    // Keep conversion/equality formatting out of the recursively entered region
    // walker so admitted nested source still fits the CLI's default stack.
    fn primitive_loose_equal(
        &mut self,
        result: ValueId,
        left: ValueId,
        right: ValueId,
        negated: bool,
    ) {
        let left = self.value_offsets[left.0];
        let right = self.value_offsets[right.0];
        if self.errors {
            writeln!(self.output,"local.get {left}\ni32.const 4\ni32.eq\nlocal.get {right}\ni32.const 4\ni32.eq\ni32.or\nif (result i32)\nlocal.get {left}\nlocal.get {right}\ni32.eq\nlocal.get {}\nlocal.get {}\nf64.eq\ni32.and\nelse",left+1,right+1).unwrap();
        }
        // Nullish payloads are +0; Boolean payloads already equal their Number
        // conversion. The optional Error branch above handles object identity.
        writeln!(self.output,"local.get {left}\ni32.const 2\ni32.lt_u\nlocal.get {right}\ni32.const 2\ni32.lt_u\ni32.eq").unwrap();
        writeln!(
            self.output,
            "local.get {}\nlocal.get {}\nf64.eq\ni32.and",
            left + 1,
            right + 1
        )
        .unwrap();
        if self.errors {
            self.output.push_str("end\n");
        }
        if negated {
            self.output.push_str("i32.eqz\n");
        }
        self.set(result);
    }

    fn general_number(&mut self, input: ValueId) {
        // Primitive null/Boolean/Number payloads already equal ToNumber.
        // Undefined and an intrinsic Error's string conversion yield NaN.
        writeln!(self.output, "local.get {}", self.value_offsets[input.0]).unwrap();
        self.output.push_str("i32.eqz\n");
        if self.errors {
            writeln!(
                self.output,
                "local.get {}\ni32.const 4\ni32.eq\ni32.or",
                self.value_offsets[input.0]
            )
            .unwrap();
        }
        self.output
            .push_str("if (result f64)\nf64.const nan\nelse\n");
        writeln!(self.output, "local.get {}", self.value_offsets[input.0] + 1).unwrap();
        self.output.push_str("end\n");
    }

    fn number_from_general(&mut self, result: ValueId, input: ValueId) {
        self.general_number(input);
        self.set(result);
    }

    fn lift_async_value(&mut self, result: ValueId, input: ValueId) {
        if self.function.values[input.0] == Type::Promise {
            writeln!(self.output, "i32.const {}", layout::ASYNC_PROMISE_TAG).unwrap();
            self.get(input);
            self.output.push_str("f64.convert_i32_u\n");
        } else {
            self.get(input);
        }
        self.set(result);
    }

    fn async_value_is_promise(&mut self, result: ValueId, input: ValueId) {
        writeln!(
            self.output,
            "local.get {}\ni32.const {}\ni32.eq",
            self.value_offsets[input.0],
            layout::ASYNC_PROMISE_TAG
        )
        .unwrap();
        self.set(result);
    }

    fn project_async_primitive(&mut self, result: ValueId, input: ValueId) {
        writeln!(
            self.output,
            "local.get {}\ni32.const {}\ni32.ge_u\nif unreachable end",
            self.value_offsets[input.0],
            layout::ASYNC_PROMISE_TAG
        )
        .unwrap();
        self.get(input);
        self.set(result);
    }

    fn project_async_promise(&mut self, result: ValueId, input: ValueId) {
        writeln!(
            self.output,
            "local.get {}\ni32.const {}\ni32.ne\nif unreachable end\nlocal.get {}\ni32.trunc_f64_u",
            self.value_offsets[input.0],
            layout::ASYNC_PROMISE_TAG,
            self.value_offsets[input.0] + 1
        )
        .unwrap();
        self.set(result);
    }

    // Async states are already flattened into sequences of atomic operations.
    // Retain the shared interface without reintroducing region recursion.
    // Async states may contain structured operations too. Schedule them with
    // the same iterative walker, but without adding a region exit.
    fn operations(&mut self, operations: &[ir::Operation]) {
        self.emit_walk(None, operations);
    }

    fn atomic_operation(&mut self, operation: &ir::Operation) {
        match &operation.kind {
            Op::CaptureCells { .. } | Op::AwaitSite { .. } => (),
            Op::ClosureEntry { .. } | Op::AsyncClosureEntry { .. } => (),
            Op::FulfillPromise { result, input }
            | Op::PromiseValue { result, input }
            | Op::PromotePromise { result, input } => {
                if self.value_offsets[result.0] != self.value_offsets[input.0] {
                    self.get(*input);
                    self.set(*result);
                }
            }
            Op::MaterializePromise { result, input } => self.materialize_promise(*result, *input),
            Op::ToAsyncValue { result, input } => self.lift_async_value(*result, *input),
            Op::AsyncValueIsPromise { result, input } => {
                self.async_value_is_promise(*result, *input)
            }
            Op::AsyncValuePrimitive { result, input } => {
                self.project_async_primitive(*result, *input)
            }
            Op::AsyncValuePromise { result, input } => self.project_async_promise(*result, *input),
            Op::MakeClosure {
                result,
                function,
                captures,
            } => self.make_closure(*result, *function, captures),
            Op::UiSelectVisible {
                result,
                expanded,
                count,
            } => {
                self.get(*expanded);
                self.get(*count);
                self.output.push_str("call $ui_select_visible\n");
                self.set(*result);
            }
            Op::ClosureCall {
                result,
                callee,
                arguments,
            }
            | Op::StartAsyncClosure {
                result,
                callee,
                arguments,
            } => self.call_closure(*result, *callee, arguments),
            Op::InlineCall { .. } => unreachable!("structured operation"),
            Op::SettlePromise { .. } => unreachable!("structured operation"),
            Op::SettledValue { result, input } => {
                writeln!(
                    self.output,
                    "local.get {}\ni32.const {}\ni32.and\nlocal.get {}",
                    self.value_offsets[input.0],
                    layout::SETTLED_TAG_MASK,
                    self.value_offsets[input.0] + 1
                )
                .unwrap();
                self.set(*result);
            }
            Op::PromiseRejected { result, input } => {
                writeln!(
                    self.output,
                    "local.get {}\ni32.const {}\ni32.and\ni32.eqz\ni32.eqz",
                    self.value_offsets[input.0],
                    layout::SETTLED_REJECTED_BIT
                )
                .unwrap();
                self.set(*result);
            }
            Op::TryCatch { .. } => unreachable!("structured operation"),
            Op::Await { .. }
            | Op::AwaitSettled { .. }
            | Op::AwaitCall { .. }
            | Op::AwaitClosureCall { .. }
            | Op::AwaitPromise { .. }
            | Op::AdoptPromise { .. } => {
                unreachable!("async control uses the async emitter")
            }
            Op::TryFinally { .. } => unreachable!("finally uses the async emitter"),
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
                if let Some(boundary) = &self.call_boundary {
                    self.output.push_str(&boundary.spill);
                }
                writeln!(self.output, "call {}", function.0).unwrap();
                if let Some(boundary) = &self.call_boundary {
                    self.output.push_str(&boundary.restore);
                }
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
            Op::DoWhile { .. } => unreachable!("structured operation"),
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
                    return;
                }
                let offset = self.slot_offsets[slot.0];
                writeln!(self.output, "local.get {offset}").unwrap();
                if layout::value(self.function.slots[slot.0].ty).parts.len() == 2 {
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
                    return;
                }
                self.get(*value);
                let offset = self.slot_offsets[slot.0];
                if layout::value(self.function.slots[slot.0].ty).parts.len() == 2 {
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
            } => self.primitive_loose_equal(*result, *left, *right, *negated),
            Op::PromiseIdentityEqual {
                result,
                left,
                right,
                negated,
            } => {
                self.get(*left);
                self.get(*right);
                self.output
                    .push_str(if *negated { "i32.ne\n" } else { "i32.eq\n" });
                self.set(*result);
            }
            Op::RelationalCompare {
                result,
                operator,
                left,
                right,
            } => {
                self.relational_compare(*result, *operator, *left, *right);
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
            Op::NumberFromGeneral { result, input } => self.number_from_general(*result, *input),
            Op::GeneralIsNullish { result, input } => {
                // Only the canonical Undefined (0) and Null (1) tags are nullish.
                writeln!(self.output, "local.get {}", self.value_offsets[input.0]).unwrap();
                self.output.push_str("i32.const 1\ni32.le_u\n");
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
            Op::Block(_) | Op::If { .. } | Op::While { .. } => {
                unreachable!("structured operation")
            }
        }
    }

    fn region(&mut self, root: &Region) {
        self.emit_walk(Some(root), &[]);
    }

    fn emit_walk<'a>(&mut self, root: Option<&'a Region>, operations: &'a [ir::Operation]) {
        // Explicit continuations preserve WAT byte order and the lexical
        // context of nested blocks without allocating native stack frames.
        enum Work<'a> {
            Region(&'a Region),
            Operation(&'a ir::Operation),
            Exit(Exit),
            Text(String),
            Set(ValueId),
            RestoreInline(Option<usize>),
            RestoreThrow(Option<ThrowTarget>),
            RestoreLoop(Option<usize>),
            WhileAfterCondition {
                label: usize,
                body: &'a Region,
                step: Option<&'a Region>,
            },
        }

        let mut pending = Vec::new();
        if let Some(root) = root {
            pending.push(Work::Region(root));
        } else {
            for operation in operations.iter().rev() {
                pending.push(Work::Operation(operation));
            }
        }
        while let Some(task) = pending.pop() {
            match task {
                Work::Region(region) => {
                    pending.push(Work::Exit(region.exit));
                    for operation in region.operations.iter().rev() {
                        pending.push(Work::Operation(operation));
                    }
                }
                Work::Exit(exit) => self.emit_exit(exit),
                Work::Text(text) => self.output.push_str(&text),
                Work::Set(value) => self.set(value),
                Work::RestoreInline(previous) => self.inline_return = previous,
                Work::RestoreThrow(previous) => self.throw_to = previous,
                Work::RestoreLoop(previous) => self.loop_label = previous,
                Work::WhileAfterCondition { label, body, step } => {
                    writeln!(self.output, "i32.eqz\nbr_if $exit{label}").unwrap();
                    let previous = self.loop_label.replace(label);
                    writeln!(self.output, "block $continue{label}").unwrap();
                    pending.push(Work::Text(format!("br $loop{label}\nend\nend\n")));
                    if let Some(step) = step {
                        pending.push(Work::Region(step));
                    }
                    pending.push(Work::RestoreLoop(previous));
                    pending.push(Work::Text("end\n".into()));
                    pending.push(Work::Region(body));
                }
                Work::Operation(operation) => match &operation.kind {
                    Op::InlineCall { result, body } => {
                        let previous = self.inline_return.replace(body.id.0);
                        let result_types = layout::value(self.function.values[result.0]).wat;
                        writeln!(
                            self.output,
                            "block $call{} (result {result_types})",
                            body.id.0
                        )
                        .unwrap();
                        pending.push(Work::Set(*result));
                        pending.push(Work::RestoreInline(previous));
                        pending.push(Work::Text("unreachable\nend\n".into()));
                        pending.push(Work::Region(body));
                    }
                    Op::SettlePromise { result, body } => {
                        let previous_return = self.inline_return.replace(body.id.0);
                        let previous_throw =
                            self.throw_to.replace(ThrowTarget::Settlement(body.id.0));
                        let previous_loop = self.loop_label.take();
                        writeln!(self.output, "block $call{} (result i32 f64)", body.id.0).unwrap();
                        pending.push(Work::Set(*result));
                        pending.push(Work::RestoreLoop(previous_loop));
                        pending.push(Work::RestoreThrow(previous_throw));
                        pending.push(Work::RestoreInline(previous_return));
                        pending.push(Work::Text("unreachable\nend\n".into()));
                        pending.push(Work::Region(body));
                    }
                    Op::TryCatch {
                        body,
                        reason,
                        handler,
                    } => {
                        let label = body.id.0;
                        writeln!(
                            self.output,
                            "block $try_done{label}\nblock $try_catch{label}"
                        )
                        .unwrap();
                        let previous = self.throw_to.replace(ThrowTarget::Catch {
                            label,
                            reason: *reason,
                        });
                        pending.push(Work::Text("end\n".into()));
                        pending.push(Work::Region(handler));
                        pending.push(Work::RestoreThrow(previous));
                        pending.push(Work::Text(format!("br $try_done{label}\nend\n")));
                        pending.push(Work::Region(body));
                    }
                    Op::DoWhile { body, condition } => {
                        let label = body.id.0;
                        writeln!(self.output, "block $exit{label}\nloop $do{label}").unwrap();
                        let previous = self.loop_label.replace(label);
                        writeln!(self.output, "block $continue{label}").unwrap();
                        pending.push(Work::Text(format!("br_if $do{label}\nend\nend\n")));
                        pending.push(Work::Region(condition));
                        pending.push(Work::RestoreLoop(previous));
                        pending.push(Work::Text("end\n".into()));
                        pending.push(Work::Region(body));
                    }
                    Op::Block(child) => pending.push(Work::Region(child)),
                    Op::If {
                        result,
                        condition,
                        consequent,
                        alternate,
                    } => {
                        self.get(*condition);
                        if let Some(result) = result {
                            writeln!(
                                self.output,
                                "if (result {})",
                                layout::value(self.function.values[result.0]).wat
                            )
                            .unwrap();
                        } else {
                            self.output.push_str("if\n");
                        }
                        if let Some(result) = result {
                            pending.push(Work::Set(*result));
                        }
                        pending.push(Work::Text("end\n".into()));
                        pending.push(Work::Region(alternate));
                        pending.push(Work::Text("else\n".into()));
                        pending.push(Work::Region(consequent));
                    }
                    Op::While {
                        condition,
                        body,
                        step,
                    } => {
                        let label = condition.id.0;
                        writeln!(self.output, "block $exit{label}\nloop $loop{label}").unwrap();
                        pending.push(Work::WhileAfterCondition {
                            label,
                            body,
                            step: step.as_ref(),
                        });
                        pending.push(Work::Region(condition));
                    }
                    _ => self.atomic_operation(operation),
                },
            }
        }
    }

    fn emit_exit(&mut self, exit: Exit) {
        match exit {
            Exit::Fallthrough => {}
            Exit::Throw(value) => match self.throw_to.expect("verified closed primitive throw") {
                ThrowTarget::Settlement(label) => {
                    writeln!(
                        self.output,
                        "local.get {}\ni32.const {}\ni32.or\nlocal.get {}\nbr $call{label}",
                        self.value_offsets[value.0],
                        layout::SETTLED_REJECTED_BIT,
                        self.value_offsets[value.0] + 1
                    )
                    .unwrap();
                }
                ThrowTarget::Catch { label, reason } => {
                    self.get(value);
                    self.set(reason);
                    writeln!(self.output, "br $try_catch{label}").unwrap();
                }
            },
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
    emit_verified(module, Type::Number, false)
}

/// Opt-in A1 service code generation. Ordinary source compilation must not
/// incur the import scan or change function indices.
pub(crate) fn emit_ui_import(module: &ir::Module) -> Result<String, String> {
    ir::verify_module(module)?;
    emit_verified(module, Type::Number, true)
}

pub(crate) fn emit_with_entry_type(
    module: &ir::Module,
    entry_type: Type,
) -> Result<String, String> {
    ir::verify_module_with_entry_type(module, entry_type)?;
    emit_verified(module, entry_type, false)
}

fn emit_verified(
    module: &ir::Module,
    entry_type: Type,
    ui_profile: bool,
) -> Result<String, String> {
    if module_materializes_promises(module) {
        return Err("Promise materialization requires the async runtime profile".into());
    }
    if module
        .functions
        .iter()
        .any(|function| region_has_await(&function.body))
    {
        return Err("await IR requires the async emitter".into());
    }
    let mut output = String::from("(module ");
    // This scan is needed only by the explicit capability profile. Standard
    // numeric/primitive compilation keeps its prior work and stack behavior.
    let mut has_ui_import = false;
    let mut has_direct_call = false;
    if ui_profile {
        for function in &module.functions {
            ir::visit_operations(&function.body, &mut |op| {
                has_ui_import |= matches!(op, Op::UiSelectVisible { .. });
                has_direct_call |= matches!(op, Op::Call { .. });
            });
        }
        if !has_ui_import {
            return Err("A1 UI profile requires a source call to the imported service".into());
        }
    }
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
    if has_ui_import {
        // Importing a function prepends its core-Wasm function index. This
        // first profile disallows all internal numeric calls and lifted functions;
        // expanding either requires named/index-independent internal references.
        if module.functions.len() != 1 || has_cells || has_direct_call {
            return Err(
                "A1 scalar import cannot yet compose with local functions or captured cells".into(),
            );
        }
        output.push_str(
            "(import \"grunkalunka:a1/ui\" \"select-visible\" (func $ui_select_visible (param f64 f64) (result f64)))\n",
        );
    }
    if has_closures {
        output.push_str(&closures::table(module, false, false));
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
        writeln!(
            header,
            " (result {})",
            layout::value(function.return_type).wat
        )
        .unwrap();
        let (body, remainder) = emit_function(function, header, false);
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
        writeln!(output, " (result {})", layout::value(entry_type).wat).unwrap();
        output.push_str("i32.const 0 global.set $cell_top\nlocal.get 0 local.get 1 call 0)\n");
    }
    if has_closures {
        output.push_str(closures::FRAME_ALLOCATOR);
    }
    output.push_str(")\n");
    Ok(output)
}

fn module_materializes_promises(module: &ir::Module) -> bool {
    module.functions.iter().any(|function| {
        let mut found = false;
        ir::visit_operations(&function.body, &mut |op| {
            found |= matches!(op, Op::MaterializePromise { .. })
        });
        found
    })
}

fn region_has_await(region: &Region) -> bool {
    region_has_control(region, true)
}

fn region_has_control(region: &Region, starts: bool) -> bool {
    // The async routing decision is read-only. Traverse the same children in
    // source order without consuming one native frame per nested region.
    let mut pending = vec![region];
    while let Some(region) = pending.pop() {
        if matches!(region.exit, Exit::Throw(_)) {
            return true;
        }
        for operation in region.operations.iter().rev() {
            match &operation.kind {
                Op::Await { .. }
                | Op::AsyncClosureEntry { .. }
                | Op::AwaitPromise { .. }
                | Op::AdoptPromise { .. }
                | Op::AwaitClosureCall { .. }
                | Op::AwaitSettled { .. }
                | Op::AwaitCall { .. }
                | Op::TryCatch { .. }
                | Op::TryFinally { .. } => return true,
                Op::StartAsyncClosure { .. } if starts => return true,
                // Settlement bodies close their own throws, and don't await.
                Op::SettlePromise { .. } | Op::StartAsyncClosure { .. } => {}
                Op::InlineCall { body, .. } | Op::Block(body) => pending.push(body),
                Op::If {
                    consequent,
                    alternate,
                    ..
                } => {
                    pending.push(alternate);
                    pending.push(consequent);
                }
                Op::While {
                    condition,
                    body,
                    step,
                } => {
                    if let Some(step) = step {
                        pending.push(step);
                    }
                    pending.push(body);
                    pending.push(condition);
                }
                Op::DoWhile { body, condition } => {
                    pending.push(condition);
                    pending.push(body);
                }
                _ => {}
            }
        }
    }
    false
}

/// Emit one ordinary activation; coroutine frames use a separate storage mapping.
fn emit_function(function: &Function, header: String, errors: bool) -> (String, bool) {
    let environment = ir::closure_environment(function);
    let mut offset = if environment.is_some() { 2 } else { 0 };
    let slot_offsets = function
        .slots
        .iter()
        .map(|slot| {
            let current = offset;
            offset += layout::value(slot.ty).parts.len();
            current
        })
        .collect();
    let mut value_offsets: Vec<usize> = function
        .values
        .iter()
        .map(|ty| {
            let current = offset;
            offset += layout::value(*ty).parts.len();
            current
        })
        .collect();
    let cell_offsets = cells::offsets(function, &mut offset);
    let arena_temp = closures::temporary(function, offset, true);
    layout::share_fulfilled_storage(function, &mut value_offsets);
    let mut emitter = Emitter {
        function,
        slot_offsets,
        value_offsets,
        cell_offsets,
        arena_temp,
        call_boundary: None,
        loop_label: None,
        inline_return: None,
        throw_to: None,
        needs_remainder: false,
        errors,
        output: header,
    };
    // Source cells and typed value snapshots occupy disjoint physical locals.
    for slot in function.slots.iter().skip(if environment.is_some() {
        0
    } else {
        function.parameter_count
    }) {
        emitter.output.push_str(layout::value(slot.ty).local);
    }
    for ty in &function.values {
        emitter.output.push_str(layout::value(*ty).local);
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

#[cfg(test)]
mod control_scan_tests {
    use super::*;
    use oxc_span::Span;

    fn region(operations: Vec<Op>, exit: Exit) -> Region {
        let span = Span::new(0, 0);
        Region {
            id: ir::RegionId(0),
            span,
            operations: operations
                .into_iter()
                .map(|kind| ir::Operation { span, kind })
                .collect(),
            exit,
        }
    }

    fn idle() -> Region {
        region(Vec::new(), Exit::Fallthrough)
    }

    #[test]
    fn control_scan_preserves_nested_exits_and_settlement_exclusion() {
        let selected = region(
            vec![Op::If {
                result: None,
                condition: ValueId(0),
                consequent: region(Vec::new(), Exit::Throw(ValueId(0))),
                alternate: idle(),
            }],
            Exit::Fallthrough,
        );
        assert!(region_has_control(&selected, false));

        let wrapped = region(
            vec![Op::While {
                condition: idle(),
                body: region(
                    vec![Op::DoWhile {
                        body: idle(),
                        condition: region(
                            vec![Op::Await {
                                result: ValueId(0),
                                input: ValueId(0),
                                rejected: false,
                            }],
                            Exit::Fallthrough,
                        ),
                    }],
                    Exit::Fallthrough,
                ),
                step: Some(idle()),
            }],
            Exit::Fallthrough,
        );
        assert!(region_has_control(&wrapped, false));

        let started = region(
            vec![Op::Block(region(
                vec![Op::StartAsyncClosure {
                    result: ValueId(0),
                    callee: ValueId(0),
                    arguments: Vec::new(),
                }],
                Exit::Fallthrough,
            ))],
            Exit::Fallthrough,
        );
        assert!(!region_has_control(&started, false));
        assert!(region_has_control(&started, true));

        let settled = region(
            vec![Op::SettlePromise {
                result: ValueId(0),
                body: region(
                    vec![Op::Await {
                        result: ValueId(0),
                        input: ValueId(0),
                        rejected: true,
                    }],
                    Exit::Throw(ValueId(0)),
                ),
            }],
            Exit::Fallthrough,
        );
        // Scanner-only synthetic IR: settlement bodies are excluded regardless
        // of whether the separately enforced verifier contract would admit them.
        assert!(!region_has_control(&settled, true));
    }

    #[test]
    fn control_scan_deep_region_chain_uses_bounded_native_stack() {
        const DEPTH: usize = 4096;
        let mut root = idle();
        for _ in 0..DEPTH {
            root = region(vec![Op::Block(root)], Exit::Fallthrough);
        }
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                assert!(!region_has_control(&root, false));
                assert!(!region_has_control(&root, true));

                // Drop this intentionally out-of-contract IR tree iteratively
                // so the test isolates the inspection path, not destruction.
                let mut current = root;
                for _ in 0..DEPTH {
                    let operation = current.operations.pop().expect("nested block");
                    let Op::Block(child) = operation.kind else {
                        panic!("nested block");
                    };
                    current = child;
                }
                assert!(current.operations.is_empty());
            })
            .expect("small-stack scan thread")
            .join()
            .expect("small-stack scan completed");
    }
}
