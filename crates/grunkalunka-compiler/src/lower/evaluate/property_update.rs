//! Numeric own-data-property writes use saved references and Number snapshots.
use super::*;

pub(in crate::lower) fn compound(operator: AssignmentOperator) -> Option<Binary> {
    Some(match operator {
        AssignmentOperator::Addition => Binary::Add,
        AssignmentOperator::Subtraction => Binary::Subtract,
        AssignmentOperator::Multiplication => Binary::Multiply,
        AssignmentOperator::Division => Binary::Divide,
        AssignmentOperator::Remainder => Binary::Remainder,
        AssignmentOperator::BitwiseAnd => Binary::BitAnd,
        AssignmentOperator::BitwiseOR => Binary::BitOr,
        AssignmentOperator::BitwiseXOR => Binary::BitXor,
        AssignmentOperator::ShiftLeft => Binary::ShiftLeft,
        AssignmentOperator::ShiftRight => Binary::ShiftRight,
        AssignmentOperator::ShiftRightZeroFill => Binary::ShiftRightUnsigned,
        _ => return None,
    })
}

pub(super) enum Task<'e, 'a> {
    Receiver {
        node: &'e Expression<'a>,
        key: Vec<u16>,
        operator: Binary,
        right: Option<&'e Expression<'a>>,
        postfix: bool,
        depth: usize,
    },
    Write {
        node: &'e Expression<'a>,
        object: ValueId,
        key: Vec<u16>,
        old: ValueId,
        operator: Binary,
        postfix: bool,
    },
}

impl Lowering<'_, '_> {
    pub(super) fn schedule_property_update<'e, 'a>(
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
        let (target, operator, right, postfix) = match node {
            Expression::UpdateExpression(update) => (
                &update.argument,
                if update.operator == UpdateOperator::Increment {
                    Binary::Add
                } else {
                    Binary::Subtract
                },
                None,
                !update.prefix,
            ),
            Expression::AssignmentExpression(assignment) => {
                let Some(operator) = compound(assignment.operator) else {
                    return Ok(false);
                };
                let Some(target) = assignment.left.as_simple_assignment_target() else {
                    return Ok(false);
                };
                (target, operator, Some(&assignment.right), false)
            }
            _ => return Ok(false),
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
            // Keep existing validation/first diagnostics for unsupported writes.
            return Ok(false);
        }
        let array = receiver == Some(Type::ManagedArray);
        let key = managed_member_key(node, key, array, self.stored.dictionary(object.node_id()))?;
        if matches!(mode, Mode::Discard) {
            work.push(Work::Forget);
        } else {
            managed::number_consumer(work, node, mode)?;
        }
        work.push(Work::PropertyUpdate(Task::Receiver {
            node,
            key,
            operator,
            right,
            postfix,
            depth: depth + 1,
        }));
        work.push(Work::Eval(
            object,
            depth + 2,
            if array { Mode::Array } else { Mode::Object },
        ));
        Ok(true)
    }

    pub(super) fn resume_property_update<'e, 'a>(
        &mut self,
        task: Task<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) {
        match task {
            Task::Receiver {
                node,
                key,
                operator,
                right,
                postfix,
                depth,
            } => {
                let object = values.pop().expect("numeric write receiver").id();
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
                work.push(Work::PropertyUpdate(Task::Write {
                    node,
                    object,
                    key,
                    old,
                    operator,
                    postfix,
                }));
                if let Some(right) = right {
                    work.push(Work::Eval(
                        right,
                        depth,
                        if matches!(operator, Binary::Add) {
                            Mode::Arithmetic
                        } else {
                            Mode::Numeric
                        },
                    ));
                } else {
                    let one = self.value(Type::Number);
                    Self::push(
                        regions.current(),
                        node,
                        Op::Constant {
                            result: one,
                            value: 1.0,
                        },
                    );
                    values.push(Value::Number(one));
                }
            }
            Task::Write {
                node,
                object,
                key,
                old,
                operator,
                postfix,
            } => {
                let right = values.pop().expect("numeric write right").number();
                let result = self.value(Type::Number);
                Self::push(
                    regions.current(),
                    node,
                    Op::Binary {
                        result,
                        operator,
                        left: old,
                        right,
                    },
                );
                let primitive = self.value(Type::General);
                Self::push(
                    regions.current(),
                    node,
                    Op::Pack {
                        result: primitive,
                        input: result,
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
                values.push(Value::Number(if postfix { old } else { result }));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn member_update_receivers_keep_source_depth_charges() {
        for (wraps, admitted) in [(126, true), (127, false)] {
            let source = format!(
                "export function run(a,b){{const o={{n:a}};return {}o{}.n++;}}",
                "(".repeat(wraps),
                ")".repeat(wraps)
            );
            let result = crate::compile(&source, false);
            assert_eq!(
                result.is_ok(),
                admitted,
                "wraps {wraps}: {:?}",
                result.err()
            );
        }
    }

    #[test]
    fn number_property_writes_require_known_own_fields_and_keep_result_positions() {
        for (body, admitted) in [
            ("const o={n:a};return o.n++;", true),
            ("const o={n:a};return ++o.n;", true),
            ("const o={n:a};return o.n+=b;", true),
            ("const xs=[a];return xs[0]>>>=b;", true),
            ("const o={n:a};const box={x:o.n++};return box.x+b;", true),
            ("const o={n:a};o.n+=undefined;return o.n;", true),
            ("const o={n:'text'};o.n++;return a;", false),
            ("const o={};o.n++;return a;", false),
            ("const o={n:a};delete o.n;o.n++;return a;", false),
            ("const o={n:a};o.n='text';o.n++;return a;", false),
            ("const o={n:a};return o.n+='text';", false),
            ("const o={n:a};const k='n';return o[k]++;", false),
            ("const xs=[a];return xs.length++;", false),
            ("return a.n++;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
    }
}
