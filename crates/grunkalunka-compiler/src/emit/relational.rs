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
        if self.errors {
            writeln!(
                self.output,
                "local.get {}\ni32.const 4\ni32.eq\nlocal.get {}\ni32.const 4\ni32.eq\ni32.and\nif (result i32)",
                self.value_offsets[left.0], self.value_offsets[right.0]
            )
            .unwrap();
            // All currently represented Error objects are the same intrinsic
            // TypeError with the same message. Their primitive strings compare
            // equal even when the object identities differ. Future Error kinds
            // and string values must extend this dispatch with string ordering.
            let equal_strings =
                matches!(operator, Relational::LessEqual | Relational::GreaterEqual);
            writeln!(self.output, "i32.const {}\nelse", u8::from(equal_strings)).unwrap();
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
        self.set(result);
    }
}
