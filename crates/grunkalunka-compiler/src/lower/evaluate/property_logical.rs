//! Logical assignments to source-proven own Number data properties.
use super::*;

pub(super) enum Task<'e, 'a> {
    Receiver {
        node: &'e Expression<'a>,
        assignment: &'e oxc_ast::ast::AssignmentExpression<'a>,
        key: Vec<u16>,
        depth: usize,
    },
    Write {
        node: &'e Expression<'a>,
        assignment: &'e oxc_ast::ast::AssignmentExpression<'a>,
        object: ValueId,
        key: Vec<u16>,
        old: ValueId,
        condition: ValueId,
    },
}

impl Lowering<'_, '_> {
    pub(super) fn schedule_property_logical<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
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
        let Some(target) = assignment.left.as_simple_assignment_target() else {
            return Ok(false);
        };
        let (object, key) = match target {
            SimpleAssignmentTarget::StaticMemberExpression(member) if !member.optional => (
                &member.object,
                member.property.name.encode_utf16().collect(),
            ),
            SimpleAssignmentTarget::ComputedMemberExpression(member) if !member.optional => {
                let Some(key) = closures::unwrapped_at(&member.expression, depth + 2)
                    .and_then(|(key, _)| static_keys::literal(key))
                else {
                    return Ok(false);
                };
                (&member.object, key)
            }
            _ => return Ok(false),
        };
        let receiver = self.receiver_type(object, depth + 2);
        if !matches!(receiver, Some(Type::ManagedObject | Type::ManagedArray))
            || !self.stored.number(target.node_id())
        {
            return Ok(false);
        }
        let array = receiver == Some(Type::ManagedArray);
        let key = managed_member_key(node, key, array, self.stored.dictionary(object.node_id()))?;
        if matches!(mode, Mode::Discard) {
            work.push(Work::Forget);
        } else {
            managed::number_consumer(work, node, mode)?;
        }
        work.push(Work::PropertyLogical(Task::Receiver {
            node,
            assignment,
            key,
            depth: depth + 1,
        }));
        work.push(Work::Eval(
            object,
            depth + 2,
            if array { Mode::Array } else { Mode::Object },
        ));
        Ok(true)
    }

    pub(super) fn resume_property_logical<'e, 'a>(
        &mut self,
        task: Task<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) {
        match task {
            Task::Receiver {
                node,
                assignment,
                key,
                depth,
            } => {
                let object = values.pop().expect("logical write receiver").id();
                let input = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    node,
                    if self.values[object.0] == Type::ManagedArray {
                        Op::ManagedArrayGetOwn {
                            result: input,
                            array: object,
                            key: key.clone(),
                        }
                    } else {
                        Op::ManagedGetOwn {
                            result: input,
                            object,
                            key: key.clone(),
                        }
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
                work.push(Work::PropertyLogical(Task::Write {
                    node,
                    assignment,
                    object,
                    key,
                    old,
                    condition,
                }));
                work.push(Work::Eval(&assignment.right, depth, Mode::Number));
            }
            Task::Write {
                node,
                assignment,
                object,
                key,
                old,
                condition,
            } => {
                let right = values.pop().expect("logical property RHS").number();
                let primitive = self.value(Type::General);
                Self::push(
                    regions.current(),
                    node,
                    Op::Pack {
                        result: primitive,
                        input: right,
                    },
                );
                let value = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedPrimitive {
                        result: value,
                        input: primitive,
                    },
                );
                Self::push(
                    regions.current(),
                    node,
                    if self.values[object.0] == Type::ManagedArray {
                        Op::ManagedArraySetOwn {
                            array: object,
                            key,
                            value,
                        }
                    } else {
                        Op::ManagedSetOwn { object, key, value }
                    },
                );
                let selected = regions.finish(right);
                let mut skipped = self.region(&assignment.left);
                skipped.exit = Exit::Yield(old);
                let (consequent, alternate) =
                    if assignment.operator == AssignmentOperator::LogicalOr {
                        (skipped, selected)
                    } else {
                        (selected, skipped)
                    };
                let result = self.value(Type::Number);
                Self::push(
                    regions.current(),
                    node,
                    Op::If {
                        result: Some(result),
                        condition,
                        consequent,
                        alternate,
                    },
                );
                values.push(Value::Number(result));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn number_properties_accept_logical_assignments_and_guard_other_kinds() {
        for (body, admitted) in [
            ("const o={n:a};return o.n&&=b;", true),
            ("const o={n:a};return o['n']||=b;", true),
            ("const o={n:a};return o.n??=b;", true),
            ("const o={n:'text'};return o.n&&=b;", false),
            ("const o={};return o.n||=b;", false),
            ("const o={n:a};return o.n||='text';", false),
            ("const o={n:a};return o[a]&&=b;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
    }
}
