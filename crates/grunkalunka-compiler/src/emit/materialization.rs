//! Allocate at construction, preserving completion metadata and opaque identity.
use super::*;

impl Emitter<'_> {
    pub(super) fn materialize_promise(&mut self, result: ValueId, input: ValueId) {
        if self.function.values[input.0] == Type::SettledPromise {
            let offset = self.value_offsets[input.0];
            writeln!(self.output,"local.get {offset}\ni32.const {}\ni32.and\ni32.eqz\ni32.eqz\nlocal.get {offset}\ni32.const {}\ni32.and\nlocal.get {}",layout::SETTLED_REJECTED_BIT,layout::SETTLED_TAG_MASK,offset+1).unwrap();
        } else {
            self.output.push_str("i32.const 0\n");
            self.get(input);
        }
        self.output.push_str("call $promise_completed\n");
        self.set(result);
    }
}
