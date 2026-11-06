//! Logical assignments to source-proven Number values in boxed local cells.
use super::*;

pub(super) struct Task<'e, 'a> {
    node: &'e Expression<'a>,
    assignment: &'e oxc_ast::ast::AssignmentExpression<'a>,
    slot: SlotId,
    old: ValueId,
    condition: ValueId,
}

impl Lowering<'_, '_> {
    pub(super) fn schedule_local_logical<'e, 'a>(
        &mut self,
        node: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
        regions: &mut Regions,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<bool, String> {
        if !matches!(
            mode,
            Mode::Number | Mode::General | Mode::Managed | Mode::Predicate | Mode::Discard
        ) {
            return Ok(false);
        }
        let Expression::AssignmentExpression(assignment) = node else {
            return Ok(false);
        };
        if !matches!(
            assignment.operator,
            AssignmentOperator::LogicalAnd
                | AssignmentOperator::LogicalOr
                | AssignmentOperator::LogicalNullish
        ) {
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
        if self.slots[slot.0].ty != Type::ManagedValue
            || !self.stored.number_update(identifier.node_id())
            || !self.stored.number(node.node_id())
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
        } else {
            managed::number_consumer(work, node, mode)?;
        }
        let input = self.value(Type::ManagedValue);
        Self::push(
            regions.current(),
            node,
            Op::Load {
                result: input,
                slot,
            },
        );
        let old = self.value(Type::Number);
        Self::push(
            regions.current(),
            node,
            Op::ManagedToNumber { result: old, input },
        );
        let condition = self.value(Type::Predicate);
        Self::push(
            regions.current(),
            node,
            if assignment.operator == AssignmentOperator::LogicalNullish {
                Op::Boolean {
                    result: condition,
                    value: false,
                }
            } else {
                Op::Truthy {
                    result: condition,
                    input: old,
                }
            },
        );
        regions.children.push(self.region(&assignment.right));
        work.push(Work::LocalLogical(Task {
            node,
            assignment,
            slot,
            old,
            condition,
        }));
        work.push(Work::Eval(&assignment.right, depth + 1, Mode::Number));
        Ok(true)
    }

    pub(super) fn resume_local_logical(
        &mut self,
        task: Task<'_, '_>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
    ) {
        let right = values.pop().expect("boxed logical assignment RHS").number();
        let primitive = self.value(Type::General);
        Self::push(
            regions.current(),
            task.node,
            Op::Pack {
                result: primitive,
                input: right,
            },
        );
        let value = self.value(Type::ManagedValue);
        Self::push(
            regions.current(),
            task.node,
            Op::ManagedPrimitive {
                result: value,
                input: primitive,
            },
        );
        Self::push(
            regions.current(),
            task.node,
            Op::Store {
                slot: task.slot,
                value,
            },
        );
        let selected = regions.finish(right);
        let mut skipped = self.region(&task.assignment.left);
        skipped.exit = Exit::Yield(task.old);
        let (consequent, alternate) = if task.assignment.operator == AssignmentOperator::LogicalOr {
            (skipped, selected)
        } else {
            (selected, skipped)
        };
        let result = self.value(Type::Number);
        Self::push(
            regions.current(),
            task.node,
            Op::If {
                result: Some(result),
                condition: task.condition,
                consequent,
                alternate,
            },
        );
        values.push(Value::Number(result));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn boxed_number_locals_accept_logical_assignments_and_guard_kind_changes() {
        for (body, admitted) in [
            ("let [x]=[a];return x&&=b;", true),
            ("let {x}={x:a};return x||=b;", true),
            ("for(let x of [a])return x??=b;return b;", true),
            ("let [x]=['text'];return x&&=b;", false),
            ("let [x]=[a];return x||='text';", false),
            ("let [x]=[a?'text':b];return x??=b;", false),
            ("const {x}={x:a};return x&&=b;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
    }
}
