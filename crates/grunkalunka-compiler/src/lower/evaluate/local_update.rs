//! Numeric updates to source-proven Number values in boxed local cells.
use super::*;

pub(super) struct Task<'e, 'a> {
    node: &'e Expression<'a>,
    slot: SlotId,
    old: ValueId,
    operator: Binary,
    postfix: bool,
}

impl Lowering<'_, '_> {
    pub(super) fn schedule_local_update<'e, 'a>(
        &mut self,
        node: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<bool, String> {
        if !matches!(
            mode,
            Mode::Number | Mode::General | Mode::Managed | Mode::Predicate | Mode::Discard
        ) {
            return Ok(false);
        }
        let (identifier, operator, right, postfix) = match node {
            Expression::UpdateExpression(update) => {
                let Some(identifier) = target_identifier(&update.argument, depth)? else {
                    return Ok(false);
                };
                (
                    identifier,
                    if update.operator == UpdateOperator::Increment {
                        Binary::Add
                    } else {
                        Binary::Subtract
                    },
                    None,
                    !update.prefix,
                )
            }
            Expression::AssignmentExpression(assignment) => {
                let Some(operator) = property_update::compound(assignment.operator) else {
                    return Ok(false);
                };
                let Some(identifier) = assignment
                    .left
                    .as_simple_assignment_target()
                    .map(|target| target_identifier(target, depth))
                    .transpose()?
                    .flatten()
                else {
                    return Ok(false);
                };
                (identifier, operator, Some(&assignment.right), false)
            }
            _ => return Ok(false),
        };
        let binding = numeric_binding(identifier, self.semantic, &self.bindings)?;
        let slot = binding.slot;
        if self.slots[slot.0].ty != Type::ManagedValue
            || !self.stored.number_update(identifier.node_id())
            || (matches!(operator, Binary::Add) && !self.stored.number(node.node_id()))
        {
            return Ok(false);
        }
        if !binding.mutable {
            return Err(unsupported(
                node,
                if right.is_some() {
                    "assignment to a const binding is not admitted"
                } else {
                    "update to a const binding is not admitted"
                },
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
        work.push(Work::LocalUpdate(Task {
            node,
            slot,
            old,
            operator,
            postfix,
        }));
        if let Some(right) = right {
            work.push(Work::Eval(
                right,
                depth + 1,
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
        Ok(true)
    }

    pub(super) fn resume_local_update(
        &mut self,
        task: Task<'_, '_>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
    ) {
        let right = values.pop().expect("boxed local update right").number();
        let result = self.value(Type::Number);
        Self::push(
            regions.current(),
            task.node,
            Op::Binary {
                result,
                operator: task.operator,
                left: task.old,
                right,
            },
        );
        let primitive = self.value(Type::General);
        Self::push(
            regions.current(),
            task.node,
            Op::Pack {
                result: primitive,
                input: result,
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
        values.push(Value::Number(if task.postfix { task.old } else { result }));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn boxed_number_locals_accept_updates_and_keep_uncertain_values_guarded() {
        for (body, admitted) in [
            ("let [x]=[a];return x++;", true),
            ("let {x}={x:a};return ++x;", true),
            ("let [x]=[a];return x+=b;", true),
            ("let {x}={x:a};x>>>=b;return x;", true),
            ("let [x]=[a];const old=x--;return old+x;", true),
            ("for(let x of [a])return x++;return b;", true),
            ("for(let [x] of [[a]])return ++x;return b;", true),
            ("let [x]=['text'];return x++;", false),
            ("let {x}={};return ++x;", false),
            ("let [x]=[a?'text':b];return x+=b;", false),
            ("let [x]=[a];return x+='text';", false),
            ("const [x]=[a];return x++;", false),
            ("const {x}={x:a};return x+=b;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
    }
}
