//! Runtime-key logical writes save the receiver, key and old value in order.
use super::managed::ManagedWork;
use super::*;

pub(super) enum Task<'e, 'a> {
    Receiver {
        node: &'e Expression<'a>,
        assignment: &'e oxc_ast::ast::AssignmentExpression<'a>,
        key: &'e Expression<'a>,
        depth: usize,
        uint32: bool,
    },
    Key {
        node: &'e Expression<'a>,
        assignment: &'e oxc_ast::ast::AssignmentExpression<'a>,
        object: ValueId,
        depth: usize,
        uint32: bool,
    },
    Write {
        node: &'e Expression<'a>,
        assignment: &'e oxc_ast::ast::AssignmentExpression<'a>,
        object: ValueId,
        key: ValueId,
        old: ValueId,
        condition: ValueId,
        uint32: bool,
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
        let Some((receiver_mode, kind)) = self.runtime_member_receiver(member, depth + 1) else {
            return Ok(false);
        };
        let uint32 = matches!(kind, dictionary::RuntimeKey::Uint32);
        if !uint32 && !self.stored.string(member.expression.node_id()) {
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
            uint32,
        }));
        work.push(Work::Eval(&member.object, depth + 1, receiver_mode));
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
                uint32,
            } => {
                let object = values.pop().expect("logical dictionary receiver").id();
                work.push(Work::DictionaryLogical(Task::Key {
                    node,
                    assignment,
                    object,
                    depth,
                    uint32,
                }));
                work.push(Work::Eval(
                    key,
                    depth,
                    if uint32 { Mode::Number } else { Mode::Managed },
                ));
            }
            Task::Key {
                node,
                assignment,
                object,
                depth,
                uint32,
            } => {
                let key = values.pop().expect("logical dictionary key").id();
                let old = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedGetOwnDynamic {
                        result: old,
                        object,
                        key: if uint32 {
                            ir::ManagedKey::Uint32(key)
                        } else {
                            ir::ManagedKey::String(key)
                        },
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
                    uint32,
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
                uint32,
            } => {
                let value = values.pop().expect("logical dictionary RHS").id();
                Self::push(
                    regions.current(),
                    node,
                    Op::ManagedSetOwnDynamic {
                        object,
                        key: if uint32 {
                            ir::ManagedKey::Uint32(key)
                        } else {
                            ir::ManagedKey::String(key)
                        },
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
    fn dynamic_logical_writes_require_receiver_and_name_proofs() {
        for (body, admitted) in [
            (
                "const d=Object.create(null);const k='x';return (d[k]??=a)===a?1:0;",
                true,
            ),
            (
                "const d=Object.create(null);const k='x';d[k]||='ok';return d[k]==='ok'?1:0;",
                true,
            ),
            ("const d={};const k='x';d[k]??=a;return a;", true),
            (
                "const o={x:'old'};const k=a?'x':'y';o[k]&&=b;return o.x.length;",
                false,
            ),
            (
                "const o={};const k=a?'x':'__proto__';o[k]||=b;return a;",
                false,
            ),
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

#[cfg(test)]
mod unsigned_tests {
    #[test]
    fn unsigned_logical_writes_retain_proofs_and_operand_checks() {
        for (body, ok) in [
            ("const xs=[];xs[a>>>0]??=b;return xs.length;", true),
            ("const o={0:a};return +(o[0>>>0]||=b);", true),
            ("const xs=[];xs[a]??=b;return a;", false),
            ("const xs=[];xs[a>>0]||=b;return a;", false),
            ("const xs=[];const k='length';xs[k]&&=b;return a;", false),
            (
                "const xs=['text'];xs[a>>>0]&&=b;return xs[0].trim().length;",
                false,
            ),
            ("const xs=[];xs[0>>>0]&&=1n;return a;", false),
            ("a[0>>>0]??=b;return a;", false),
        ] {
            let s = format!("export function run(a,b){{{body}}}");
            for ts in [false, true] {
                let r = crate::compile(&s, ts);
                assert_eq!(r.is_ok(), ok, "{s}: {:?}", r.err());
            }
        }
        let js = "export function run(a,b){const xs=[];xs[a>>>0]??=b;return xs.length;}";
        let ts = "export function run(a:string,b:boolean):never{const xs=[];xs[(a>>>0) as never]??=b;return xs.length;}";
        assert_eq!(
            crate::compile(js, false).unwrap().wasm,
            crate::compile(ts, true).unwrap().wasm
        );
    }
}
