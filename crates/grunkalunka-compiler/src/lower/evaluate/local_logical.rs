//! Logical assignments preserve the selected value in managed local cells.
use super::managed::ManagedWork;
use super::*;

pub(super) struct Task<'e, 'a> {
    node: &'e Expression<'a>,
    assignment: &'e oxc_ast::ast::AssignmentExpression<'a>,
    slot: SlotId,
    old: ValueId,
    condition: ValueId,
    numeric: bool,
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
            Mode::Number
                | Mode::General
                | Mode::Managed
                | Mode::Predicate
                | Mode::Discard
                | Mode::Object
                | Mode::Array
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
        let slot_type = self.slots[slot.0].ty;
        if !slot_type.is_managed() {
            return Ok(false);
        }
        let reference = matches!(slot_type, Type::ManagedObject | Type::ManagedArray);
        let numeric = !reference
            && self.stored.number_update(identifier.node_id())
            && self.stored.number(node.node_id());
        match mode {
            Mode::Object if slot_type != Type::ManagedObject => return Ok(false),
            Mode::Array if slot_type != Type::ManagedArray => return Ok(false),
            Mode::General | Mode::Number if !numeric => return Ok(false),
            _ => {}
        }
        if !binding.mutable {
            return Err(unsupported(
                node,
                "assignment to a const binding is not admitted",
            ));
        }

        if matches!(mode, Mode::Discard) {
            work.push(Work::Forget);
        } else if numeric {
            managed::number_consumer(work, node, mode)?;
        } else if matches!(mode, Mode::Predicate) {
            work.push(Work::Managed(ManagedWork::Truthy(node)));
        }
        if reference && matches!(mode, Mode::Managed | Mode::Predicate) {
            work.push(Work::Managed(ManagedWork::BoxReference(node)));
        }
        let input = self.value(slot_type);
        Self::push(
            regions.current(),
            node,
            Op::Load {
                result: input,
                slot,
            },
        );
        let old = if numeric {
            let old = self.value(Type::Number);
            Self::push(
                regions.current(),
                node,
                Op::ManagedToNumber { result: old, input },
            );
            old
        } else {
            input
        };
        let condition = self.value(Type::Predicate);
        Self::push(
            regions.current(),
            node,
            if reference {
                Op::Boolean {
                    result: condition,
                    value: assignment.operator != AssignmentOperator::LogicalNullish,
                }
            } else if !numeric && assignment.operator == AssignmentOperator::LogicalNullish {
                Op::ManagedIsNullish {
                    result: condition,
                    input: old,
                }
            } else if !numeric {
                Op::ManagedTruthy {
                    result: condition,
                    input: old,
                }
            } else if assignment.operator == AssignmentOperator::LogicalNullish {
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
            numeric,
        }));
        work.push(Work::Eval(
            &assignment.right,
            depth + 1,
            if numeric {
                Mode::Number
            } else {
                Mode::managed(slot_type)
            },
        ));
        Ok(true)
    }

    pub(super) fn resume_local_logical(
        &mut self,
        task: Task<'_, '_>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
    ) {
        let right = values.pop().expect("boxed logical assignment RHS").id();
        let value = if task.numeric {
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
            value
        } else {
            right
        };
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
        let result_type = self.values[task.old.0];
        let result = self.value(result_type);
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
        values.push(if task.numeric {
            Value::Number(result)
        } else {
            Value::managed(result, result_type)
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn boxed_local_logical_assignments_accept_represented_values() {
        for body in [
            "let [x]=[null];return (x??='ok')==='ok'?1:0;",
            "let {x}={x:''};return (x||=a)===a?1:0;",
            "let [x]=[false];return (x&&='skip')===false?1:0;",
            "const other={n:a};let [x]=[null];return (x??=other)===other?1:0;",
            "let [x]=[undefined];if(x??=a)return b;return a;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert!(result.is_ok(), "{source}: {:?}", result.err());
            }
        }
        for body in [
            "const [x]=[null];return (x??=a)===a?1:0;",
            "let [x]=[null];x??=unknown();return a;",
            "let [x]=[false];x&&=1n;return a;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            let error = crate::compile(&source, false)
                .err()
                .expect("guarded source");
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }

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
