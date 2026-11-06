//! Logical assignments preserve selected values in static own data properties.
use super::managed::ManagedWork;
use super::*;

pub(super) enum Task<'e, 'a> {
    Receiver {
        node: &'e Expression<'a>,
        assignment: &'e oxc_ast::ast::AssignmentExpression<'a>,
        key: Vec<u16>,
        depth: usize,
        numeric: bool,
    },
    Write {
        node: &'e Expression<'a>,
        assignment: &'e oxc_ast::ast::AssignmentExpression<'a>,
        object: ValueId,
        key: Vec<u16>,
        old: ValueId,
        condition: ValueId,
        numeric: bool,
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
        if !matches!(receiver, Some(Type::ManagedObject | Type::ManagedArray)) {
            return Ok(false);
        }
        let numeric = self.stored.number(target.node_id());
        if !numeric && !matches!(mode, Mode::Managed | Mode::Predicate | Mode::Discard) {
            return Ok(false);
        }
        let array = receiver == Some(Type::ManagedArray);
        if array && key.as_slice() == [108, 101, 110, 103, 116, 104] {
            return Ok(false);
        }
        let key = managed_member_key(node, key, array, self.stored.dictionary(object.node_id()))?;
        if matches!(mode, Mode::Discard) {
            work.push(Work::Forget);
        } else if numeric {
            managed::number_consumer(work, node, mode)?;
        } else if matches!(mode, Mode::Predicate) {
            work.push(Work::Managed(ManagedWork::Truthy(node)));
        }
        work.push(Work::PropertyLogical(Task::Receiver {
            node,
            assignment,
            key,
            depth: depth + 1,
            numeric,
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
                numeric,
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
                    if !numeric && assignment.operator == AssignmentOperator::LogicalNullish {
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
                work.push(Work::PropertyLogical(Task::Write {
                    node,
                    assignment,
                    object,
                    key,
                    old,
                    condition,
                    numeric,
                }));
                work.push(Work::Eval(
                    &assignment.right,
                    depth,
                    if numeric { Mode::Number } else { Mode::Managed },
                ));
            }
            Task::Write {
                node,
                assignment,
                object,
                key,
                old,
                condition,
                numeric,
            } => {
                let right = values.pop().expect("logical property RHS").id();
                let value = if numeric {
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
                    value
                } else {
                    right
                };
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
                let result = self.value(if numeric {
                    Type::Number
                } else {
                    Type::ManagedValue
                });
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
                values.push(if numeric {
                    Value::Number(result)
                } else {
                    Value::Managed(result)
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn mixed_property_logical_assignments_keep_receiver_and_key_guards() {
        for body in [
            "const o={x:null};return (o.x??='ok')==='ok'?1:0;",
            "const o={};return (o.x??=a)===a?1:0;",
            "const xs=[false];return (xs[0]&&='skip')===false?1:0;",
            "const o={x:''};if(o.x||='ok')return a;return b;",
            "const o=Object.create(null);return (o.__proto__??='ok')==='ok'?1:0;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert!(result.is_ok(), "{source}: {:?}", result.err());
            }
        }
        for body in [
            "const xs=[];xs.length||=1;return a;",
            "const o={};o.constructor??=a;return a;",
            "const o={};const key='x';o[key]??=a;return a;",
            "const o={x:false};o.x&&=1n;return a;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            let error = crate::compile(&source, false)
                .err()
                .expect("guarded source");
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }

    #[test]
    fn number_properties_accept_logical_assignments_and_guard_other_kinds() {
        for (body, admitted) in [
            ("const o={n:a};return o.n&&=b;", true),
            ("const o={n:a};return o['n']||=b;", true),
            ("const o={n:a};return o.n??=b;", true),
            ("const o={n:'text'};return o.n&&=b;", false),
            ("const o={};return o.n||=b;", true),
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
