//! General relational operands retain their kinds until ToPrimitive dispatch.
use super::*;
use crate::ir::Relational;

impl Emitter<'_> {
    pub(super) fn relational_compare(
        &mut self,
        result: ValueId,
        operator: Relational,
        left: ValueId,
        right: ValueId,
    ) {
        let selected = [left, right]
            .iter()
            .any(|v| self.function.values[v.0] == Type::AsyncValue);
        if selected {
            self.relational_tag(left, 5);
            self.relational_tag(right, 5);
            self.output.push_str("i32.or\nif (result i32)\n");
            // The builtin Promise string starts with '['; each represented
            // intrinsic TypeError string starts with 'T'. Other General kinds
            // compare numerically against Promise's NaN and always yield false.
            let (error, promise) = match operator {
                Relational::Less | Relational::LessEqual => (left, right),
                Relational::Greater | Relational::GreaterEqual => (right, left),
            };
            self.relational_tag(error, 4);
            self.relational_tag(promise, 5);
            self.output.push_str("i32.and\n");
            if matches!(operator, Relational::LessEqual | Relational::GreaterEqual) {
                // Different Promise identities still have equal primitive strings.
                self.relational_tag(left, 5);
                self.relational_tag(right, 5);
                self.output.push_str("i32.and\ni32.or\n");
            }
            self.output.push_str("else\n");
        }
        if self.errors {
            writeln!(
                self.output,
                "local.get {}\ni32.const 4\ni32.eq\nlocal.get {}\ni32.const 4\ni32.eq\ni32.and\nif (result i32)",
                self.value_offsets[left.0], self.value_offsets[right.0]
            )
            .unwrap();
            // Identity is separate from the fixed intrinsic Error strings.
            // Modules without the new cause keep the original equal-string path.
            if self.error_kinds {
                // Cause 2's "Cannot convert..." precedes cause 1's Promise
                // self-resolution message, in both native Node and our adapter.
                writeln!(self.output,"local.get {}\ncall $error_record\ni32.load offset=4\nlocal.get {}\ncall $error_record\ni32.load offset=4",self.value_offsets[left.0]+1,self.value_offsets[right.0]+1).unwrap();
                self.output.push_str(match operator {
                    Relational::Less => "i32.gt_u\n",
                    Relational::LessEqual => "i32.ge_u\n",
                    Relational::Greater => "i32.lt_u\n",
                    Relational::GreaterEqual => "i32.le_u\n",
                });
                self.output.push_str("else\n");
            } else {
                let equal_strings =
                    matches!(operator, Relational::LessEqual | Relational::GreaterEqual);
                writeln!(self.output, "i32.const {}\nelse", u8::from(equal_strings)).unwrap();
            }
        }
        self.general_number(left);
        self.general_number(right);
        self.output.push_str(match operator {
            Relational::Less => "f64.lt\n",
            Relational::LessEqual => "f64.le\n",
            Relational::Greater => "f64.gt\n",
            Relational::GreaterEqual => "f64.ge\n",
        });
        if self.errors {
            self.output.push_str("end\n");
        }
        if selected {
            self.output.push_str("end\n");
        }
        self.set(result);
    }

    fn relational_tag(&mut self, value: ValueId, tag: u8) {
        writeln!(
            self.output,
            "local.get {}\ni32.const {tag}\ni32.eq",
            self.value_offsets[value.0]
        )
        .unwrap();
    }
}
