//! Each intrinsic array binding owns its live length and sticky exhaustion.
//! The shared pattern cursor retains these independent states across defaults.
use super::*;

pub(super) struct IteratorState {
    active: ValueId,
    pub(super) undefined: ValueId,
}

impl Lowering<'_, '_> {
    pub(super) fn begin_array_binding_iterator(
        &mut self,
        node: &impl GetSpan,
        output: &mut Region,
    ) -> IteratorState {
        let active = self.value(Type::Predicate);
        Self::push(
            output,
            node,
            Op::Boolean {
                result: active,
                value: true,
            },
        );
        let undefined = self.binding_undefined(node, output);
        IteratorState { active, undefined }
    }

    pub(super) fn read_array_binding_element(
        &mut self,
        node: &impl GetSpan,
        output: &mut Region,
        array: ValueId,
        index: u32,
    ) -> ValueId {
        let selected = self.value(Type::ManagedValue);
        Self::push(
            output,
            node,
            Op::ManagedArrayGetOwn {
                result: selected,
                array,
                key: index.to_string().encode_utf16().collect(),
            },
        );
        selected
    }

    pub(super) fn select_array_binding_step(
        &mut self,
        node: &impl GetSpan,
        output: &mut Region,
        array: ValueId,
        iterator: &mut IteratorState,
        index: u32,
    ) -> ValueId {
        // No admitted hooks run in elisions. Checking the next bound index
        // also observes exhaustion anywhere in an intervening run of holes.
        // Length remains live until the first exhausted step, then stays done.
        let mut live = self.region(node);
        let length = self.value(Type::Number);
        Self::push(
            &mut live,
            node,
            Op::ManagedArrayLength {
                result: length,
                array,
            },
        );
        let position = self.value(Type::Number);
        Self::push(
            &mut live,
            node,
            Op::Constant {
                result: position,
                value: f64::from(index),
            },
        );
        let in_bounds = self.value(Type::Predicate);
        Self::push(
            &mut live,
            node,
            Op::Compare {
                result: in_bounds,
                operator: ir::Compare::Less,
                left: position,
                right: length,
            },
        );
        live.exit = Exit::Yield(in_bounds);
        let mut exhausted = self.region(node);
        exhausted.exit = Exit::Yield(iterator.active);
        let active = self.value(Type::Predicate);
        Self::push(
            output,
            node,
            Op::If {
                result: Some(active),
                condition: iterator.active,
                consequent: live,
                alternate: exhausted,
            },
        );
        iterator.active = active;

        let mut present = self.region(node);
        let selected = self.read_array_binding_element(node, &mut present, array, index);
        present.exit = Exit::Yield(selected);
        let mut absent = self.region(node);
        absent.exit = Exit::Yield(iterator.undefined);
        let result = self.value(Type::ManagedValue);
        Self::push(
            output,
            node,
            Op::If {
                result: Some(result),
                condition: active,
                consequent: present,
                alternate: absent,
            },
        );
        result
    }
}
