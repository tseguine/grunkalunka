//! Runtime-key logical writes save the receiver, key and old value in order.
use super::managed::ManagedWork;
use super::*;

pub(super) enum Task<'e, 'a> {
    Receiver {
        node: &'e Expression<'a>,
        assignment: &'e oxc_ast::ast::AssignmentExpression<'a>,
        key: &'e Expression<'a>,
        depth: usize,
    },
    Key {
        node: &'e Expression<'a>,
        assignment: &'e oxc_ast::ast::AssignmentExpression<'a>,
        object: ValueId,
        depth: usize,
    },
    Write {
        node: &'e Expression<'a>,
        assignment: &'e oxc_ast::ast::AssignmentExpression<'a>,
        object: ValueId,
        key: ValueId,
        old: ValueId,
        condition: ValueId,
    },
}

impl Lowering<'_, '_> {
    pub(super) fn schedule_dictionary_logical<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<bool, String> {
        if !matches!(mode, Mode::Managed | Mode::Predicate | Mode::Discard) {
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
        let Some(SimpleAssignmentTarget::ComputedMemberExpression(member)) =
            assignment.left.as_simple_assignment_target()
        else {
            return Ok(false);
        };
        if member.optional || !self.stored.dictionary(member.object.node_id()) {
            return Ok(false);
        }
        if closures::unwrapped_at(&member.expression, depth + 1)
            .is_some_and(|(key, _)| static_keys::literal(key).is_some())
        {
            return Ok(false);
        }
        if !self.stored.string(member.expression.node_id()) {
            return Err(unsupported(
                node,
                "runtime dictionary keys require a known String",
            ));
        }
        if matches!(mode, Mode::Discard) {
            work.push(Work::Forget);
        } else if matches!(mode, Mode::Predicate) {
            work.push(Work::Managed(ManagedWork::Truthy(node)));
        }
        work.push(Work::DictionaryLogical(Task::Receiver {
            node,
            assignment,
            key: &member.expression,
            depth: depth + 1,
        }));
        work.push(Work::Eval(&member.object, depth + 1, Mode::Object));
        Ok(true)
    }

    pub(super) fn resume_dictionary_logical<'e, 'a>(
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
                let object = values.pop().expect("logical dictionary receiver").id();
                work.push(Work::DictionaryLogical(Task::Key {
                    node,
                    assignment,
                    object,
                    depth,
                }));
                work.push(Work::Eval(key, depth, Mode::Managed));
            }
            Task::Key {
                node,
                assignment,
                object,
                depth,
            } => {
                let key = values.pop().expect("logical dictionary key").id();
                let old = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedGetOwnDynamic {
                        result: old,
                        object,
                        key: ir::ManagedKey::String(key),
                    },
                );
                let condition = self.value(Type::Predicate);
                Self::push(
                    regions.current(),
                    node,
                    if assignment.operator == AssignmentOperator::LogicalNullish {
                        Op::ManagedIsNullish {
                            result: condition,
                            input: old,
                        }
                    } else {
                        Op::ManagedTruthy {
                            result: condition,
                            input: old,
                        }
                    },
                );
                regions.children.push(self.region(&assignment.right));
                work.push(Work::DictionaryLogical(Task::Write {
                    node,
                    assignment,
                    object,
                    key,
                    old,
                    condition,
                }));
                work.push(Work::Eval(&assignment.right, depth, Mode::Managed));
            }
            Task::Write {
                node,
                assignment,
                object,
                key,
                old,
                condition,
            } => {
                let value = values.pop().expect("logical dictionary RHS").id();
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedSetOwnDynamic {
                        object,
                        key: ir::ManagedKey::String(key),
                        value,
                    },
                );
                let selected = regions.finish(value);
                let mut skipped = self.region(&assignment.left);
                skipped.exit = Exit::Yield(old);
                let (consequent, alternate) =
                    if assignment.operator == AssignmentOperator::LogicalOr {
                        (skipped, selected)
                    } else {
                        (selected, skipped)
                    };
                let result = self.value(Type::ManagedValue);
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
                values.push(Value::Managed(result));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn dynamic_logical_writes_require_dictionary_and_string_key_proofs() {
        for (body, admitted) in [
            (
                "const d=Object.create(null);const k='x';return (d[k]??=a)===a?1:0;",
                true,
            ),
            (
                "const d=Object.create(null);const k='x';d[k]||='ok';return d[k]==='ok'?1:0;",
                true,
            ),
            ("const d={};const k='x';d[k]??=a;return a;", false),
            ("const d=Object.create(null);d[a]??=b;return a;", false),
            ("const d=[];const k='x';d[k]??=a;return a;", false),
            (
                "const d=Object.create(null);const k='x';d.x='old';d[k]??=a;return d.x.length;",
                false,
            ),
            (
                "const d=Object.create(null);const k='x';d[k]&&=1n;return a;",
                false,
            ),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
    }
}
