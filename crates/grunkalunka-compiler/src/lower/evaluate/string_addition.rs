//! String-only local += saves the old value before evaluating its RHS.
use super::managed::ManagedWork;
use super::*;

pub(super) struct Task<'e, 'a> {
    node: &'e Expression<'a>,
    slot: SlotId,
    left: ValueId,
}

impl Lowering<'_, '_> {
    pub(super) fn schedule_string_addition<'e, 'a>(
        &mut self,
        node: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
        regions: &mut Regions,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<bool, String> {
        if !matches!(mode, Mode::Managed | Mode::Predicate | Mode::Discard) {
            return Ok(false);
        }
        let Expression::AssignmentExpression(assignment) = node else {
            return Ok(false);
        };
        if assignment.operator != AssignmentOperator::Addition
            || !self.stored.string(node.node_id())
        {
            return Ok(false);
        }
        let Some(identifier) = assignment
            .left
            .as_simple_assignment_target()
            .map(|target| target_identifier(target, depth))
            .transpose()?
            .flatten()
        else {
            return Ok(false);
        };
        let binding = numeric_binding(identifier, self.semantic, &self.bindings)?;
        let slot = binding.slot;
        if self.slots[slot.0].ty != Type::ManagedValue || !self.stored.string(identifier.node_id())
        {
            return Ok(false);
        }
        if !binding.mutable {
            return Err(unsupported(
                node,
                "assignment to a const binding is not admitted",
            ));
        }
        if matches!(mode, Mode::Discard) {
            work.push(Work::Forget);
        } else if matches!(mode, Mode::Predicate) {
            work.push(Work::Managed(ManagedWork::Truthy(node)));
        }
        let left = self.value(Type::ManagedValue);
        Self::push(regions.current(), node, Op::Load { result: left, slot });
        work.push(Work::StringAddition(Task { node, slot, left }));
        work.push(Work::Eval(&assignment.right, depth + 1, Mode::Managed));
        Ok(true)
    }

    pub(super) fn resume_string_addition(
        &mut self,
        task: Task<'_, '_>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
    ) {
        let right = values.pop().expect("String compound RHS").id();
        let result = self.value(Type::ManagedValue);
        Self::push(
            regions.current(),
            task.node,
            Op::ManagedStringConcat {
                result,
                left: task.left,
                right,
            },
        );
        Self::push(
            regions.current(),
            task.node,
            Op::Store {
                slot: task.slot,
                value: result,
            },
        );
        values.push(Value::Managed(result));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn string_compounds_require_positive_string_origins_and_mutability() {
        for (body, admitted) in [
            ("let text='x';text+='y';return text.length;", true),
            ("let [text]=['x'];return (text+='y').length;", true),
            (
                "for(let text of ['x'])return (text+='y').length;return a;",
                true,
            ),
            ("let text='x';text+=a;return text.length;", false),
            ("let text=a?'x':b;text+='y';return text.length;", false),
            ("const text='x';text+='y';return text.length;", false),
            ("let text='x';text++;text+='y';return text.length;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
        assert!(crate::compile("export function run(a,b){let text='x';text+=(a as unknown as string);return text.length;}", true).is_err());
    }
}
