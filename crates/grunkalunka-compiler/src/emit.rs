//! Direct structured Wasm emission. Every computation and source load is
//! materialized immediately: later source-cell stores cannot change a value.
use std::fmt::Write;

mod array_at;
mod array_pop;
mod array_rest;
mod asynchronous;
mod cells;
mod closures;
mod gc_closures;
mod has_own;
#[cfg(test)]
mod iterative_emitter_tests;
mod layout;
mod managed_spread;
mod materialization;
mod object_rest;
mod own_keys;
mod property_membership;
mod relational;
#[cfg(test)]
mod stack_hygiene_tests;
mod string_concat;
mod string_methods;
pub(crate) use asynchronous::emit_async;

use crate::ir::{self, Binary, Compare, Exit, Function, Op, Region, Type, Unary, ValueId};

// Pinned Node/V8 limit for one array.new_fixed instruction.
const MAX_FIXED_TEXT_UNITS: usize = 10_000;

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
enum FinallyStage {
    Cleanup,
    Dispatch,
}

#[derive(Clone, Copy)]
enum FinallyKind {
    Return = 1,
    Throw = 2,
    Break = 3,
    Continue = 4,
}

#[derive(Clone, Copy)]
enum ReturnTarget {
    Inline(usize),
    Finally {
        label: usize,
        returned: ValueId,
        stage: FinallyStage,
    },
}

#[derive(Clone, Copy)]
enum ThrowTarget {
    Settlement(usize),
    Catch { label: usize, reason: ValueId },
    Finally { label: usize, reason: ValueId },
}

#[derive(Clone, Copy)]
enum LoopTarget {
    Loop(usize),
    Finally(usize),
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
    loop_to: Option<LoopTarget>,
    return_to: Option<ReturnTarget>,
    throw_to: Option<ThrowTarget>,
    // Shared coroutine code needs distinct atomic scratch locals per owner.
    finally_owner: usize,
    needs_remainder: bool,
    errors: bool,
    error_kinds: bool,
    gc_closures: bool,
    // Source FunctionIds do not include imported core-Wasm functions.
    function_index_offset: usize,
}

// Ordinary helpers use Wasm locals. A coroutine caller additionally preserves
// its persistent lanes when a helper may start another coroutine synchronously.
struct CallBoundary {
    spill: String,
    restore: String,
}

impl Emitter<'_> {
    fn value_layout(&self, ty: Type) -> layout::Layout {
        layout::closure_value(ty, self.gc_closures)
    }

    fn get(&mut self, value: ValueId) {
        writeln!(self.output, "local.get {}", self.value_offsets[value.0]).unwrap();
        if self.value_layout(self.function.values[value.0]).parts.len() == 2 {
            writeln!(self.output, "local.get {}", self.value_offsets[value.0] + 1).unwrap();
        }
    }
    fn set(&mut self, value: ValueId) {
        if self.value_layout(self.function.values[value.0]).parts.len() == 2 {
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
            Op::ManagedGetOwn {
                result,
                object,
                key,
            } => {
                self.get(*object);
                self.managed_key(key);
                self.output.push_str("call $gc_get\n");
                self.set(*result);
            }
            Op::ManagedGetOwnDynamic {
                result,
                object,
                key,
            } => {
                self.get(*object);
                self.managed_key_value(*key);
                self.output.push_str("call $gc_get\n");
                self.set(*result);
            }
            Op::ManagedSetOwn { object, key, value } => {
                self.get(*object);
                self.managed_key(key);
                self.get(*value);
                self.output.push_str("call $gc_set\n");
            }
            Op::ManagedSetOwnDynamic { object, key, value } => {
                self.get(*object);
                self.managed_key_value(*key);
                self.get(*value);
                self.output.push_str("call $gc_set\n");
            }
            Op::ManagedObjectRest {
                result,
                object,
                excluded,
            } => {
                self.get(*object);
                if self.function.values[object.0] == Type::ManagedArray {
                    // Only the sparse own-property table is enumerable; length is separate.
                    self.output.push_str("struct.get $gc_array 1\n");
                }
                self.output.push_str("ref.null $gc_rest_key\n");
                for key in excluded {
                    self.property_key(key);
                    self.output.push_str("struct.new $gc_rest_key\n");
                }
                self.output.push_str("call $gc_object_rest\n");
                self.set(*result);
            }
            Op::ManagedObjectSpread { object, input } => {
                self.get(*object);
                self.get(*input);
                self.output.push_str("call $gc_object_spread\n");
            }
            Op::ManagedHasProperty { result, input, key } => {
                self.get(*input);
                self.managed_key_value(*key);
                self.output
                    .push_str(if self.function.values[input.0] == Type::ManagedArray {
                        "call $gc_array_has_property\n"
                    } else {
                        "call $gc_object_has_property\n"
                    });
                self.set(*result);
            }
            Op::ManagedDeleteOwn { result, input, key } => {
                self.get(*input);
                if self.function.values[input.0] == Type::ManagedArray {
                    self.output.push_str("struct.get $gc_array 1\n");
                }
                self.property_key(key);
                self.output.push_str("call $gc_delete\n");
                self.set(*result);
            }
            Op::ManagedHasOwn { result, input, key } => {
                let ty = self.function.values[input.0];
                if matches!(ty, Type::Number | Type::Predicate) {
                    // These primitive wrappers have no own properties. The
                    // source cursor already evaluated the receiver and all arguments.
                    if let ir::ManagedKey::String(_) = key {
                        self.property_key(key);
                        self.output.push_str("drop\n");
                    }
                    self.output.push_str("i32.const 0\n");
                } else {
                    let helper = match ty {
                        Type::ManagedValue => {
                            self.managed_key_value(*input);
                            "$gc_string_has_own"
                        }
                        Type::ManagedObject => {
                            self.get(*input);
                            "$gc_has_own"
                        }
                        Type::ManagedArray => {
                            self.get(*input);
                            "$gc_array_has_own"
                        }
                        _ => unreachable!("verified has-own receiver"),
                    };
                    self.property_key(key);
                    writeln!(self.output, "call {helper}").unwrap();
                }
                self.set(*result);
            }
            Op::ManagedEnumerateOwn {
                result,
                input,
                kind,
            } => {
                let ty = self.function.values[input.0];
                if matches!(ty, Type::Number | Type::Closure(_)) {
                    self.get(*input);
                    self.output
                        .push_str("drop\ni32.const 0\ncall $gc_array_new\n");
                } else {
                    if matches!(ty, Type::ManagedObject | Type::ManagedArray) {
                        self.output.push_str("i32.const 4\nf64.const 0\n");
                        self.get(*input);
                        self.output.push_str("struct.new $gc_value\n");
                    } else {
                        self.get(*input);
                    }
                    writeln!(
                        self.output,
                        "i32.const {}\ncall $gc_enumerate_known",
                        *kind as u8
                    )
                    .unwrap();
                }
                self.set(*result);
            }
            Op::TryManagedEnumerateOwn {
                kind,
                result,
                input,
                failure,
            } => {
                self.get(*input);
                writeln!(
                    self.output,
                    "i32.const {}\ncall $gc_try_enumerate",
                    *kind as u8
                )
                .unwrap();
                self.set(failure.reason);
                self.set(failure.threw);
                self.set(*result);
            }
            Op::ManagedString { result, units } => {
                self.output.push_str("i32.const 5\nf64.const 0\n");
                self.managed_key(units);
                self.output.push_str("struct.new $gc_value\n");
                self.set(*result);
            }
            Op::ManagedStringToArray { result, input } => {
                self.get(*input);
                self.output.push_str("call $gc_string_to_array\n");
                self.set(*result);
            }
            Op::ManagedStringLength { result, input } => {
                self.get(*input);
                self.output.push_str(
                    "struct.get $gc_value 0\ni32.const 5\ni32.ne\nif\nunreachable\nend\n",
                );
                self.get(*input);
                self.output.push_str(
                    "struct.get $gc_value 2\nref.cast (ref $gc_key)\narray.len\nf64.convert_i32_u\n",
                );
                self.set(*result);
            }
            Op::ManagedStringConcat {
                result,
                left,
                right,
            } => {
                self.get(*left);
                self.get(*right);
                self.output.push_str("call $gc_string_concat\n");
                self.set(*result);
            }
            Op::ManagedStringRange {
                result,
                input,
                start,
                end,
                substring,
            } => {
                self.get(*input);
                self.get(*start);
                self.get(*end);
                writeln!(
                    self.output,
                    "i32.const {}\ncall $gc_string_range",
                    i32::from(*substring)
                )
                .unwrap();
                self.set(*result);
            }
            Op::ManagedStringIterValue {
                result,
                input,
                index,
            } => {
                self.get(*input);
                self.get(*index);
                self.output.push_str("call $gc_string_iter_value\n");
                self.set(*result);
            }
            Op::ManagedStringCharAt {
                result,
                input,
                index,
            }
            | Op::ManagedStringCodePointAt {
                result,
                input,
                index,
            }
            | Op::ManagedStringGetIndex {
                result,
                input,
                index,
            }
            | Op::ManagedStringAt {
                result,
                input,
                index,
            }
            | Op::ManagedStringCharCodeAt {
                result,
                input,
                index,
            } => {
                self.get(*input);
                self.get(*index);
                self.output.push_str(
                    if matches!(operation.kind, Op::ManagedStringCharAt { .. }) {
                        "call $gc_string_char_at\n"
                    } else if matches!(operation.kind, Op::ManagedStringAt { .. }) {
                        "call $gc_string_at\n"
                    } else if matches!(operation.kind, Op::ManagedStringCodePointAt { .. }) {
                        "call $gc_string_code_point_at\n"
                    } else if matches!(operation.kind, Op::ManagedStringGetIndex { .. }) {
                        "call $gc_string_get_index\n"
                    } else {
                        "call $gc_string_char_code_at\n"
                    },
                );
                self.set(*result);
            }
            Op::ManagedStringSearch {
                result,
                input,
                search,
                position,
                kind,
            } => {
                self.get(*input);
                self.get(*search);
                self.get(*position);
                let mode = match kind {
                    ir::StringSearch::StartsWith => 0,
                    ir::StringSearch::Includes => 1,
                    ir::StringSearch::EndsWith => 2,
                };
                writeln!(self.output, "i32.const {mode}\ncall $gc_string_search").unwrap();
                self.set(*result);
            }
            Op::ManagedStringToWellFormed { result, input } => {
                self.get(*input);
                self.output.push_str("call $gc_string_to_well_formed\n");
                self.set(*result);
            }
            Op::ManagedStringIsWellFormed { result, input } => {
                self.get(*input);
                self.output.push_str("call $gc_string_is_well_formed\n");
                self.set(*result);
            }
            Op::ManagedStringTrim {
                result,
                input,
                ends,
            } => {
                self.get(*input);
                let flags = match ends {
                    ir::StringTrim::Start => 1,
                    ir::StringTrim::End => 2,
                    ir::StringTrim::Both => 3,
                };
                writeln!(self.output, "i32.const {flags}\ncall $gc_string_trim").unwrap();
                self.set(*result);
            }
            Op::ManagedStringIndex {
                result,
                input,
                search,
                position,
                backwards,
            } => {
                self.get(*input);
                self.get(*search);
                self.get(*position);
                writeln!(
                    self.output,
                    "i32.const {}\ncall $gc_string_index",
                    i32::from(*backwards)
                )
                .unwrap();
                self.set(*result);
            }
            Op::ManagedArrayNew { result, length } => {
                writeln!(
                    self.output,
                    "i32.const {}\ncall $gc_array_new",
                    *length as i32
                )
                .unwrap();
                self.set(*result);
            }
            Op::ManagedArrayLength { result, array } => {
                self.get(*array);
                self.output
                    .push_str("struct.get $gc_array 0\nf64.convert_i32_u\n");
                self.set(*result);
            }
            Op::ManagedArrayGetOwn { result, array, key } => {
                self.get(*array);
                self.managed_key(key);
                self.output.push_str("call $gc_array_get\n");
                self.set(*result);
            }
            Op::ManagedArrayShift { result, array } => {
                self.get(*array);
                self.output.push_str("call $gc_array_shift\n");
                self.set(*result);
            }
            Op::ManagedArrayReverse { result, array } => {
                self.get(*array);
                self.output.push_str("call $gc_array_reverse\n");
                self.set(*result);
            }
            Op::ManagedArraySlice {
                result,
                array,
                start,
                end,
            } => {
                self.get(*array);
                self.get(*start);
                self.get(*end);
                self.output.push_str("call $gc_array_slice\n");
                self.set(*result);
            }
            Op::ManagedArraySearch {
                kind,
                result,
                array,
                needle,
                from,
            } => {
                self.get(*array);
                if let Some(needle) = needle {
                    self.get(*needle);
                } else {
                    self.output
                        .push_str("i32.const 0\nf64.const 0\nref.null eq\nstruct.new $gc_value\n");
                }
                if let Some(from) = from {
                    self.get(*from);
                } else {
                    self.output
                        .push_str(if *kind == ir::ArraySearch::LastIndexOf {
                            "f64.const inf\n"
                        } else {
                            "f64.const 0\n"
                        });
                }
                let mode = match kind {
                    ir::ArraySearch::Includes => 0,
                    ir::ArraySearch::IndexOf => 1,
                    ir::ArraySearch::LastIndexOf => 2,
                };
                writeln!(self.output, "i32.const {mode}\ncall $gc_array_search").unwrap();
                self.output.push_str(if *kind == ir::ArraySearch::Includes {
                    "i64.const 0\ni64.ge_s\n"
                } else {
                    "f64.convert_i64_s\n"
                });
                self.set(*result);
            }
            Op::ManagedArrayAt {
                result,
                array,
                index,
            } => {
                self.get(*array);
                self.get(*index);
                self.output.push_str("call $gc_array_at\n");
                self.set(*result);
            }
            Op::ManagedArraySetOwn { array, key, value } => {
                self.get(*array);
                self.managed_key(key);
                self.get(*value);
                self.output.push_str("call $gc_array_set\n");
            }
            Op::ManagedArrayPop { result, array } => {
                self.get(*array);
                self.output.push_str("call $gc_array_pop\n");
                self.set(*result);
            }
            Op::ManagedArrayRest {
                result,
                array,
                start,
            } => {
                self.get(*array);
                writeln!(
                    self.output,
                    "i32.const {}\ncall $gc_array_rest",
                    *start as i32
                )
                .unwrap();
                self.set(*result);
            }
            Op::ManagedObjectNew { result } => {
                self.output.push_str("call $gc_object_new\n");
                self.set(*result);
            }
            Op::ManagedArrayResize { array, length } => {
                self.get(*array);
                writeln!(
                    self.output,
                    "i32.const {}\ncall $gc_array_resize",
                    *length as i32
                )
                .unwrap();
            }
            Op::ManagedFromObject { result, input } | Op::ManagedFromArray { result, input } => {
                self.output.push_str("i32.const 4\nf64.const 0\n");
                self.get(*input);
                self.output.push_str("struct.new $gc_value\n");
                self.set(*result);
            }
            Op::ManagedPrimitive { result, input } => {
                let tag = self.value_offsets[input.0];
                if self.errors {
                    // General's Error tag 4 must not become GC Object tag 4.
                    // Retain the validated invocation-local Error identity in
                    // the scalar payload of a distinct GC language kind.
                    writeln!(self.output,"local.get {tag}\ni32.const 4\ni32.eq\nif (result i32)\nlocal.get {}\ncall $error_record\ndrop\ni32.const 7\nelse\nlocal.get {tag}\ni32.const 3\ni32.gt_u\nif unreachable end\nlocal.get {tag}\nend\nlocal.get {}",tag+1,tag+1).unwrap();
                } else {
                    writeln!(
                        self.output,
                        "local.get {tag}\ni32.const 3\ni32.gt_u\nif unreachable end"
                    )
                    .unwrap();
                    self.get(*input);
                }
                self.output.push_str("ref.null eq\nstruct.new $gc_value\n");
                self.set(*result);
            }
            Op::ManagedToNumber { result, input } => {
                self.get(*input);
                self.output
                    .push_str("struct.get $gc_value 0\ni32.const 3\ni32.ne\nif unreachable end\n");
                self.get(*input);
                self.output.push_str("struct.get $gc_value 1\n");
                self.set(*result);
            }
            Op::ManagedToObject { result, input } | Op::ManagedToArray { result, input } => {
                self.get(*input);
                self.output
                    .push_str("struct.get $gc_value 0\ni32.const 4\ni32.ne\nif unreachable end\n");
                self.get(*input);
                self.output
                    .push_str(if matches!(operation.kind, Op::ManagedToArray { .. }) {
                        "struct.get $gc_value 2\nref.cast (ref $gc_array)\n"
                    } else {
                        "struct.get $gc_value 2\nref.cast (ref $gc_object)\n"
                    });
                self.set(*result);
            }
            Op::ManagedToClosure { result, input } => {
                let Type::Closure(signature) = self.function.values[result.0] else {
                    unreachable!("verified function projection result");
                };
                self.get(*input);
                writeln!(self.output, "call $gc_to_function{signature}").unwrap();
                self.set(*result);
            }
            Op::ManagedFromClosure { result, input } => {
                self.output.push_str("i32.const 6\nf64.const 0\n");
                self.get(*input);
                self.output.push_str("struct.new $gc_value\n");
                self.set(*result);
            }
            Op::ManagedEqual {
                result,
                left,
                right,
            } => {
                self.get(*left);
                self.get(*right);
                self.output.push_str("call $gc_equal\n");
                self.set(*result);
            }
            Op::ManagedTypeof { result, input } => {
                self.output.push_str("i32.const 5\nf64.const 0\n");
                // A constant-bounded dispatch over the shared language-kind tags.
                let names = [
                    "undefined",
                    "object",
                    "boolean",
                    "number",
                    "object",
                    "string",
                    "function",
                    "object",
                ];
                let names = &names[..if self.errors { 8 } else { 7 }];
                for (tag, name) in names.iter().enumerate() {
                    self.get(*input);
                    writeln!(self.output, "struct.get $gc_value 0\ni32.const {tag}\ni32.eq\nif (result (ref null $gc_key))").unwrap();
                    self.managed_key(&name.encode_utf16().collect::<Vec<_>>());
                    self.output.push_str("else\n");
                }
                // Future carrier kinds require their own explicit typeof mapping.
                self.output.push_str("unreachable\n");
                for _ in names {
                    self.output.push_str("end\n");
                }
                self.output.push_str("struct.new $gc_value\n");
                self.set(*result);
            }
            Op::ManagedTruthy { result, input } => {
                self.get(*input);
                self.output.push_str("call $gc_truthy\n");
                self.set(*result);
            }
            Op::ManagedIsArray { result, input } => {
                self.get(*input);
                // Objects and arrays share language tag 4. The actual GC
                // payload carries the array brand, regardless of its own keys.
                self.output
                    .push_str("struct.get $gc_value 0\ni32.const 4\ni32.eq\n");
                self.get(*input);
                self.output
                    .push_str("struct.get $gc_value 2\nref.test (ref $gc_array)\ni32.and\n");
                self.set(*result);
            }
            Op::ManagedIsNullish { result, input } => {
                self.get(*input);
                // Shared managed tags 0/1 denote undefined/null respectively.
                self.output
                    .push_str("struct.get $gc_value 0\ni32.const 1\ni32.le_u\n");
                self.set(*result);
            }
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
            Op::ModuleClosure { result, function } => {
                writeln!(self.output, "global.get $gc_module_function{}", function.0).unwrap();
                self.set(*result);
            }
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
            Op::TryFinally { .. } => unreachable!("structured operation"),
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
                writeln!(
                    self.output,
                    "call {}",
                    function.0 + self.function_index_offset
                )
                .unwrap();
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
                if self
                    .value_layout(self.function.slots[slot.0].ty)
                    .parts
                    .len()
                    == 2
                {
                    writeln!(self.output, "local.get {}", offset + 1).unwrap();
                }
                self.set(*result);
            }
            Op::RebindCell { slot, value } => {
                // The source Load preceding this operation preserves the old
                // value; Initialize-style storage allocates a new cell and
                // redirects future references without changing past closures.
                let pointer = self.cell_offsets[slot.0].expect("rebound cell has storage");
                self.store_cell(*slot, *value, pointer, true);
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
                if self
                    .value_layout(self.function.slots[slot.0].ty)
                    .parts
                    .len()
                    == 2
                {
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

    fn property_key(&mut self, key: &ir::ManagedKey) {
        match key {
            ir::ManagedKey::Literal(units) => self.managed_key(units),
            ir::ManagedKey::String(input) => self.managed_key_value(*input),
        }
    }

    fn managed_key_value(&mut self, key: ValueId) {
        self.get(key);
        self.output
            .push_str("struct.get $gc_value 0\ni32.const 5\ni32.ne\nif unreachable end\n");
        self.get(key);
        self.output
            .push_str("struct.get $gc_value 2\nref.cast (ref $gc_key)\n");
    }

    fn managed_key(&mut self, key: &[u16]) {
        // V8's fixed constructor accepts at most 10,000 elements. Larger
        // literals initialize an internal array before publishing its reference.
        if key.len() > MAX_FIXED_TEXT_UNITS {
            writeln!(
                self.output,
                "i32.const {}\narray.new_default $gc_key\nlocal.set $gc_literal",
                key.len()
            )
            .unwrap();
            for (index, unit) in key.iter().enumerate() {
                writeln!(
                    self.output,
                    "local.get $gc_literal\ni32.const {index}\ni32.const {unit}\narray.set $gc_key"
                )
                .unwrap();
            }
            self.output.push_str("local.get $gc_literal\n");
        } else {
            for unit in key {
                writeln!(self.output, "i32.const {unit}").unwrap();
            }
            writeln!(self.output, "array.new_fixed $gc_key {}", key.len()).unwrap();
        }
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
            RestoreReturn(Option<ReturnTarget>),
            RestoreThrow(Option<ThrowTarget>),
            RestoreLoop(Option<LoopTarget>),
            FinallyDispatch {
                label: usize,
                reason: ValueId,
                continuation: &'a Region,
            },
            FinallyExit {
                label: usize,
                kind: FinallyKind,
                exit: Option<Exit>,
            },
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
                Work::RestoreReturn(previous) => self.return_to = previous,
                Work::RestoreThrow(previous) => self.throw_to = previous,
                Work::RestoreLoop(previous) => self.loop_to = previous,
                Work::FinallyDispatch {
                    label,
                    reason,
                    continuation,
                } => {
                    // The cleanup has restored the enclosing routes. Keep the
                    // original continuation for both protected and cleanup
                    // returns, including nontrivial raw-value continuations.
                    pending.push(Work::FinallyExit {
                        label,
                        kind: FinallyKind::Continue,
                        exit: self.loop_to.map(|_| Exit::Continue),
                    });
                    pending.push(Work::FinallyExit {
                        label,
                        kind: FinallyKind::Break,
                        exit: self.loop_to.map(|_| Exit::Break),
                    });
                    pending.push(Work::FinallyExit {
                        label,
                        kind: FinallyKind::Throw,
                        exit: self.throw_to.map(|_| Exit::Throw(reason)),
                    });
                    pending.push(Work::Text("end\n".into()));
                    pending.push(Work::Region(continuation));
                    self.begin_finally_case(label, FinallyKind::Return);
                }
                Work::FinallyExit { label, kind, exit } => {
                    self.begin_finally_case(label, kind);
                    if let Some(exit) = exit {
                        self.emit_exit(exit);
                    } else {
                        // Verification and routing exclude this completion;
                        // do not manufacture an enclosing throw/loop target.
                        self.output.push_str("unreachable\n");
                    }
                    self.output.push_str("end\n");
                }
                Work::WhileAfterCondition { label, body, step } => {
                    writeln!(self.output, "i32.eqz\nbr_if $exit{label}").unwrap();
                    let previous = self.loop_to.replace(LoopTarget::Loop(label));
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
                        let previous = self.return_to.replace(ReturnTarget::Inline(body.id.0));
                        let previous_loop = self.loop_to.take();
                        let result_types = self.value_layout(self.function.values[result.0]).wat;
                        writeln!(
                            self.output,
                            "block $call{} (result {result_types})",
                            body.id.0
                        )
                        .unwrap();
                        pending.push(Work::Set(*result));
                        pending.push(Work::RestoreLoop(previous_loop));
                        pending.push(Work::RestoreReturn(previous));
                        pending.push(Work::Text("unreachable\nend\n".into()));
                        pending.push(Work::Region(body));
                    }
                    Op::SettlePromise { result, body } => {
                        let previous_return =
                            self.return_to.replace(ReturnTarget::Inline(body.id.0));
                        let previous_throw =
                            self.throw_to.replace(ThrowTarget::Settlement(body.id.0));
                        let previous_loop = self.loop_to.take();
                        writeln!(self.output, "block $call{} (result i32 f64)", body.id.0).unwrap();
                        pending.push(Work::Set(*result));
                        pending.push(Work::RestoreLoop(previous_loop));
                        pending.push(Work::RestoreThrow(previous_throw));
                        pending.push(Work::RestoreReturn(previous_return));
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
                    Op::TryFinally {
                        body,
                        finalizer,
                        returned,
                        reason,
                        continuation,
                    } => {
                        let label = body.id.0;
                        let previous_return = self.return_to.replace(ReturnTarget::Finally {
                            label,
                            returned: *returned,
                            stage: FinallyStage::Cleanup,
                        });
                        let previous_throw = self.throw_to.replace(ThrowTarget::Finally {
                            label,
                            reason: *reason,
                        });
                        let previous_loop = self.loop_to.replace(LoopTarget::Finally(label));
                        // A loop can reenter this node after any prior pending
                        // completion. Reset its private kind on every entry.
                        writeln!(
                            self.output,
                            "i32.const 0\nlocal.set $finally_kind{}_{}\nblock $finally_dispatch{label}\nblock $finally_cleanup{label}",
                            self.finally_owner, label
                        )
                        .unwrap();
                        pending.push(Work::FinallyDispatch {
                            label,
                            reason: *reason,
                            continuation,
                        });
                        pending.push(Work::RestoreReturn(previous_return));
                        pending.push(Work::Text("end\n".into()));
                        pending.push(Work::Region(finalizer));
                        pending.push(Work::RestoreReturn(Some(ReturnTarget::Finally {
                            label,
                            returned: *returned,
                            stage: FinallyStage::Dispatch,
                        })));
                        pending.push(Work::RestoreThrow(previous_throw));
                        pending.push(Work::RestoreLoop(previous_loop));
                        pending.push(Work::Text("end\n".into()));
                        pending.push(Work::Region(body));
                    }
                    Op::DoWhile { body, condition } => {
                        let label = body.id.0;
                        writeln!(self.output, "block $exit{label}\nloop $do{label}").unwrap();
                        let previous = self.loop_to.replace(LoopTarget::Loop(label));
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
                                self.value_layout(self.function.values[result.0]).wat
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

    fn begin_finally_case(&mut self, label: usize, kind: FinallyKind) {
        writeln!(
            self.output,
            "local.get $finally_kind{}_{}\ni32.const {}\ni32.eq\nif",
            self.finally_owner, label, kind as u8
        )
        .unwrap();
    }

    fn jump_finally(&mut self, label: usize, kind: FinallyKind, stage: FinallyStage) {
        let stage = match stage {
            FinallyStage::Cleanup => "cleanup",
            FinallyStage::Dispatch => "dispatch",
        };
        writeln!(
            self.output,
            "i32.const {}\nlocal.set $finally_kind{}_{}\nbr $finally_{stage}{label}",
            kind as u8, self.finally_owner, label
        )
        .unwrap();
    }

    fn emit_loop_exit(&mut self, continuing: bool) {
        match self.loop_to.expect("verified loop completion") {
            LoopTarget::Loop(label) => {
                let target = if continuing { "continue" } else { "exit" };
                writeln!(self.output, "br ${target}{label}").unwrap();
            }
            LoopTarget::Finally(label) => self.jump_finally(
                label,
                if continuing {
                    FinallyKind::Continue
                } else {
                    FinallyKind::Break
                },
                FinallyStage::Cleanup,
            ),
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
                ThrowTarget::Finally { label, reason } => {
                    self.get(value);
                    self.set(reason);
                    self.jump_finally(label, FinallyKind::Throw, FinallyStage::Cleanup);
                }
            },
            Exit::Break => self.emit_loop_exit(false),
            Exit::Continue => self.emit_loop_exit(true),
            Exit::Yield(value) => self.get(value),
            Exit::Return(value) => {
                self.get(value);
                match self.return_to {
                    Some(ReturnTarget::Inline(label)) => {
                        writeln!(self.output, "br $call{label}").unwrap();
                    }
                    Some(ReturnTarget::Finally {
                        label,
                        returned,
                        stage,
                    }) => {
                        self.set(returned);
                        self.jump_finally(label, FinallyKind::Return, stage);
                    }
                    None => self.output.push_str("return\n"),
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
    if own_keys::fallible(module) {
        return Err("fallible own-keys requires the async completion profile".into());
    }
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
    let managed = module_has_managed(module);
    let gc_closures = gc_closures::needed(module);
    // This scan is needed only by the explicit capability profile. Standard
    // numeric/primitive compilation keeps its prior work and stack behavior.
    let mut has_ui_import = false;
    if ui_profile {
        for function in &module.functions {
            ir::visit_operations(&function.body, &mut |op| {
                has_ui_import |= matches!(op, Op::UiSelectVisible { .. });
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
    if has_ui_import {
        // Core imports must precede the captured-cell memory/global definitions.
        output.push_str(
            "(import \"grunkalunka:a1/ui\" \"select-visible\" (func $ui_select_visible (param f64 f64) (result f64)))\n",
        );
    }
    let function_index_offset = usize::from(has_ui_import);
    if has_cells && !gc_closures {
        output.push_str(cells::MEMORY);
    }
    if gc_closures {
        output.push_str(gc_closures::TYPES);
    }
    if has_closures {
        output.push_str(&closures::table(
            module,
            false,
            false,
            gc_closures,
            function_index_offset,
        ));
    }
    if gc_closures {
        output.push_str(&gc_closures::module_globals(module));
    }
    let mut needs_remainder = false;
    for (index, function) in module.functions.iter().enumerate() {
        let environment = ir::closure_environment(function);
        let mut header = if index == 0 && (!has_cells || gc_closures) {
            String::from("(func (export \"run\")")
        } else {
            String::from("(func")
        };
        if environment.is_some() {
            header.push_str(if gc_closures {
                " (param (ref null $gc_frame) (ref null $gc_frame))"
            } else {
                " (param i32 i32)"
            });
        } else if function.parameter_count > 0 {
            header.push_str(" (param");
            for slot in function.slots.iter().take(function.parameter_count) {
                header.push(' ');
                header.push_str(layout::closure_value(slot.ty, gc_closures).wat);
            }
            header.push(')');
        }
        let result_wat = if environment.is_some() && function.return_type.is_managed() {
            "(ref null eq)"
        } else {
            layout::closure_value(function.return_type, gc_closures).wat
        };
        writeln!(header, " (result {})", result_wat).unwrap();
        let (body, remainder) = emit_function(
            function,
            header,
            false,
            gc_closures,
            false,
            function_index_offset,
        );
        needs_remainder |= remainder;
        output.push_str(&body);
    }
    if needs_remainder {
        output.push('\n');
        output.push_str(NUMBER_REMAINDER);
    }
    if has_cells && !gc_closures {
        output.push_str(cells::ALLOCATOR);
        output.push_str("(func (export \"run\") (param f64 f64)");
        writeln!(output, " (result {})", layout::value(entry_type).wat).unwrap();
        writeln!(
            output,
            "i32.const 0 global.set $cell_top\nlocal.get 0 local.get 1 call {function_index_offset})"
        )
        .unwrap();
    }
    if has_closures && !gc_closures {
        output.push_str(closures::FRAME_ALLOCATOR);
    }
    if gc_closures {
        output.push_str(&gc_closures::projection_helpers(module));
    }
    if managed {
        output.push_str(include_str!("emit/managed.wat"));
        emit_managed_helpers(module, &mut output);
    }
    output.push_str(")\n");
    Ok(output)
}

fn emit_managed_helpers(module: &ir::Module, output: &mut String) {
    // Emit the decimal index formatter immediately before its first consumer.
    let mut emitted_index = false;
    let mut indexed = |output: &mut String, runtime: &str| {
        if !runtime.is_empty() {
            if !emitted_index {
                output.push_str(include_str!("emit/managed_index.wat"));
                emitted_index = true;
            }
            output.push_str(runtime);
        }
    };
    indexed(output, managed_spread::runtime(module));
    output.push_str(object_rest::runtime(module));
    output.push_str(has_own::runtime(module));
    if module.functions.iter().any(|function| {
        ir::any_operation(&function.body, &mut |op| {
            matches!(op, Op::ManagedDeleteOwn { .. })
        })
    }) {
        output.push_str(include_str!("emit/property_delete.wat"));
    }
    property_membership::emit_runtime(module, output);
    indexed(output, own_keys::runtime(module));
    indexed(output, array_at::runtime(module));
    if module.functions.iter().any(|f| {
        ir::any_operation(&f.body, &mut |op| {
            matches!(op, Op::ManagedArraySearch { .. })
        })
    }) {
        output.push_str(include_str!("emit/array_search.wat"));
    }
    indexed(output, array_pop::runtime(module));
    if module.functions.iter().any(|f| {
        ir::any_operation(&f.body, &mut |op| {
            matches!(op, Op::ManagedArrayShift { .. })
        })
    }) {
        indexed(output, include_str!("emit/array_shift.wat"));
    }
    if module.functions.iter().any(|f| {
        ir::any_operation(&f.body, &mut |op| {
            matches!(op, Op::ManagedArrayReverse { .. })
        })
    }) {
        indexed(output, include_str!("emit/array_reverse.wat"));
    }
    indexed(output, array_rest::runtime(module));
    if module.functions.iter().any(|f| {
        ir::any_operation(&f.body, &mut |op| {
            matches!(op, Op::ManagedArraySlice { .. })
        })
    }) {
        indexed(output, include_str!("emit/array_slice.wat"));
    }
    output.push_str(string_concat::runtime(module));
    output.push_str(string_methods::runtime(module));
    output.push_str(string_methods::iteration_runtime(module));
    indexed(output, string_methods::spread_runtime(module));
}

pub(crate) fn module_has_managed(module: &ir::Module) -> bool {
    module.functions.iter().any(|function| {
        function
            .slots
            .iter()
            .map(|slot| slot.ty)
            .chain(function.values.iter().copied())
            .any(Type::is_managed)
    })
}

fn module_materializes_promises(module: &ir::Module) -> bool {
    module.functions.iter().any(|function| {
        ir::any_operation(&function.body, &mut |op| {
            matches!(op, Op::MaterializePromise { .. })
        })
    })
}

fn region_has_await(region: &Region) -> bool {
    region_has_control(region, true)
}

fn region_has_control(region: &Region, starts: bool) -> bool {
    // The async routing decision is read-only. Traverse the same children in
    // source order without consuming one native frame per nested region. A
    // primitive throw closed by this function's own catch needs no coroutine.
    let mut pending = vec![(region, false)];
    while let Some((region, caught)) = pending.pop() {
        if matches!(region.exit, Exit::Throw(_)) && !caught {
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
                | Op::AwaitCall { .. } => return true,
                Op::StartAsyncClosure { .. } if starts => return true,
                // Settlement bodies close their own throws, and don't await.
                Op::SettlePromise { .. } | Op::StartAsyncClosure { .. } => {}
                // Preserve the separate source-function completion boundary.
                // A catch inside that body can close its throws independently.
                Op::InlineCall { body, .. } => pending.push((body, false)),
                Op::Block(body) => pending.push((body, caught)),
                Op::TryCatch { body, handler, .. } => {
                    pending.push((handler, caught));
                    pending.push((body, true));
                }
                Op::TryFinally {
                    body,
                    finalizer,
                    continuation,
                    ..
                } => {
                    // Cleanup preserves the enclosing exception ownership. A
                    // finally node does not close a source throw by itself.
                    pending.push((continuation, caught));
                    pending.push((finalizer, caught));
                    pending.push((body, caught));
                }
                Op::If {
                    consequent,
                    alternate,
                    ..
                } => {
                    pending.push((alternate, caught));
                    pending.push((consequent, caught));
                }
                Op::While {
                    condition,
                    body,
                    step,
                } => {
                    if let Some(step) = step {
                        pending.push((step, caught));
                    }
                    pending.push((body, caught));
                    pending.push((condition, caught));
                }
                Op::DoWhile { body, condition } => {
                    pending.push((condition, caught));
                    pending.push((body, caught));
                }
                _ => {}
            }
        }
    }
    false
}

/// Emit one ordinary activation; coroutine frames use a separate storage mapping.
fn emit_function(
    function: &Function,
    header: String,
    errors: bool,
    gc_closures: bool,
    error_kinds: bool,
    function_index_offset: usize,
) -> (String, bool) {
    let environment = ir::closure_environment(function);
    let mut offset = if environment.is_some() { 2 } else { 0 };
    let slot_offsets = function
        .slots
        .iter()
        .map(|slot| {
            let current = offset;
            offset += layout::closure_value(slot.ty, gc_closures).parts.len();
            current
        })
        .collect();
    let mut value_offsets: Vec<usize> = function
        .values
        .iter()
        .map(|ty| {
            let current = offset;
            offset += layout::closure_value(*ty, gc_closures).parts.len();
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
        loop_to: None,
        return_to: None,
        throw_to: None,
        finally_owner: 0,
        needs_remainder: false,
        errors,
        error_kinds,
        gc_closures,
        function_index_offset,
        output: header,
    };
    // Source cells and typed value snapshots occupy disjoint physical locals.
    for slot in function.slots.iter().skip(if environment.is_some() {
        0
    } else {
        function.parameter_count
    }) {
        emitter
            .output
            .push_str(layout::closure_value(slot.ty, gc_closures).local);
    }
    for ty in &function.values {
        emitter
            .output
            .push_str(layout::closure_value(*ty, gc_closures).local);
    }
    for _ in emitter.cell_offsets.iter().flatten() {
        emitter.output.push_str(if gc_closures {
            "(local (ref null $gc_cell))\n"
        } else {
            "(local i32)\n"
        });
    }
    if arena_temp.is_some() {
        emitter.output.push_str(if gc_closures {
            "(local (ref null $gc_frame))\n"
        } else {
            "(local i32)\n"
        });
    }
    if needs_literal_scratch(function) {
        emitter
            .output
            .push_str("(local $gc_literal (ref null $gc_key))\n");
    }
    ir::visit_operations(&function.body, &mut |operation| {
        if let Op::TryFinally { body, .. } = operation {
            declare_finally_local(&mut emitter.output, emitter.finally_owner, body.id.0);
        }
    });
    emitter.init_environment();
    emitter.init_cell_parameters();
    emitter.region(&function.body);
    // The verifier proved return coverage. This only helps Wasm validation after
    // nested void branches whose arms return; it supplies no source semantics.
    emitter.output.push_str("unreachable\n)");
    (emitter.output, emitter.needs_remainder)
}

fn declare_finally_local(output: &mut String, owner: usize, label: usize) {
    writeln!(output, "(local $finally_kind{owner}_{label} i32)").unwrap();
}

fn needs_literal_scratch(function: &ir::Function) -> bool {
    let mut needed = false;
    ir::visit_operations(&function.body, &mut |op| {
        needed |= match op {
            Op::ManagedString { units, .. } => units.len() > MAX_FIXED_TEXT_UNITS,
            Op::ManagedHasOwn { input, key: ir::ManagedKey::Literal(units), .. }
            | Op::ManagedDeleteOwn { input, key: ir::ManagedKey::Literal(units), .. } => {
                function.values[input.0].is_managed() && units.len() > MAX_FIXED_TEXT_UNITS
            }
            Op::ManagedObjectRest { excluded, .. } => {
                excluded.iter().any(|key| matches!(key, ir::ManagedKey::Literal(text) if text.len() > MAX_FIXED_TEXT_UNITS))
            }
            Op::ManagedGetOwn { key, .. }
            | Op::ManagedSetOwn { key, .. }
            | Op::ManagedArrayGetOwn { key, .. }
            | Op::ManagedArraySetOwn { key, .. } => key.len() > MAX_FIXED_TEXT_UNITS,
            _ => false,
        };
    });
    needed
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

    fn caught(body: Region, handler: Region) -> Region {
        region(
            vec![Op::TryCatch {
                body,
                reason: ValueId(0),
                handler,
            }],
            Exit::Fallthrough,
        )
    }

    fn thrown() -> Region {
        region(Vec::new(), Exit::Throw(ValueId(0)))
    }

    #[test]
    fn control_scan_closes_protected_throws_and_preserves_outer_handlers() {
        for starts in [false, true] {
            assert!(!region_has_control(&caught(thrown(), idle()), starts));
            assert!(region_has_control(&caught(thrown(), thrown()), starts));
            assert!(!region_has_control(
                &caught(caught(thrown(), thrown()), idle()),
                starts,
            ));
            let mut later = caught(thrown(), idle());
            later.operations.push(ir::Operation {
                span: Span::new(0, 0),
                kind: Op::Block(thrown()),
            });
            assert!(region_has_control(&later, starts));
        }
    }

    #[test]
    fn control_scan_keeps_awaits_and_inline_function_boundaries() {
        for starts in [false, true] {
            for in_handler in [false, true] {
                let awaiting = region(
                    vec![Op::Await {
                        result: ValueId(0),
                        input: ValueId(0),
                        rejected: false,
                    }],
                    Exit::Fallthrough,
                );
                let source = if in_handler {
                    caught(idle(), awaiting)
                } else {
                    caught(awaiting, idle())
                };
                assert!(region_has_control(&source, starts));
            }
            let inline = |body| {
                region(
                    vec![Op::InlineCall {
                        result: ValueId(0),
                        body,
                    }],
                    Exit::Fallthrough,
                )
            };
            assert!(region_has_control(
                &caught(inline(thrown()), idle()),
                starts
            ));
            assert!(!region_has_control(
                &inline(caught(thrown(), idle())),
                starts,
            ));
        }
    }

    #[test]
    fn control_scan_traverses_finally_without_creating_a_catch() {
        let finalized = |body, finalizer, continuation| {
            region(
                vec![Op::TryFinally {
                    body,
                    finalizer,
                    returned: ValueId(0),
                    reason: ValueId(1),
                    continuation: Box::new(continuation),
                }],
                Exit::Fallthrough,
            )
        };
        for starts in [false, true] {
            assert!(!region_has_control(
                &finalized(idle(), idle(), idle()),
                starts
            ));
            for position in 0..3 {
                for awaiting in [false, true] {
                    let mut parts = [idle(), idle(), idle()];
                    parts[position] = if awaiting {
                        region(
                            vec![Op::Await {
                                result: ValueId(0),
                                input: ValueId(0),
                                rejected: false,
                            }],
                            Exit::Fallthrough,
                        )
                    } else {
                        thrown()
                    };
                    let [body, finalizer, continuation] = parts;
                    let source = finalized(body, finalizer, continuation);
                    assert!(region_has_control(&source, starts));
                    assert_eq!(
                        region_has_control(&caught(source, idle()), starts),
                        awaiting
                    );
                }
            }
        }
    }

    #[test]
    fn control_scan_deep_catches_use_bounded_native_stack() {
        const DEPTH: usize = 4096;
        let mut source = thrown();
        for _ in 0..DEPTH {
            source = caught(source, idle());
        }
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                assert!(!region_has_control(&source, false));
                assert!(!region_has_control(&source, true));
                // Isolate the iterative scan from destruction of synthetic IR
                // whose depth deliberately exceeds source admission limits.
                let mut current = source;
                for _ in 0..DEPTH {
                    let operation = current.operations.pop().expect("nested catch");
                    let Op::TryCatch { body, .. } = operation.kind else {
                        panic!("nested catch");
                    };
                    current = body;
                }
                assert!(matches!(current.exit, Exit::Throw(_)));
            })
            .expect("small-stack catch scan thread")
            .join()
            .expect("small-stack catch scan completed");
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
