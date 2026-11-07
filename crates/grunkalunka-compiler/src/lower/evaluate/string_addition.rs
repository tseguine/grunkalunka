//! String-only += saves its reference and old value before evaluating its RHS.
use super::managed::ManagedWork;
use super::*;

pub(super) enum Destination {
    Local(SlotId),
    Property { object: ValueId, key: Vec<u16> },
}

pub(super) enum Task<'e, 'a> {
    Receiver {
        node: &'e Expression<'a>,
        right: &'e Expression<'a>,
        key: Vec<u16>,
        depth: usize,
    },
    Write {
        node: &'e Expression<'a>,
        destination: Destination,
        left: ValueId,
    },
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
        let Some(target) = assignment.left.as_simple_assignment_target() else {
            return Ok(false);
        };
        if !self.stored.string(target.node_id()) {
            return Ok(false);
        }
        let property = match target {
            SimpleAssignmentTarget::StaticMemberExpression(member) if !member.optional => Some((
                &member.object,
                member.property.name.encode_utf16().collect(),
            )),
            SimpleAssignmentTarget::ComputedMemberExpression(member) if !member.optional => {
                let Some(key) = closures::unwrapped_at(&member.expression, depth + 2)
                    .and_then(|(key, _)| static_keys::literal(key))
                else {
                    return Ok(false);
                };
                Some((&member.object, key))
            }
            _ => None,
        };
        let local = if property.is_none() {
            let Some(identifier) = target_identifier(target, depth)? else {
                return Ok(false);
            };
            let binding = numeric_binding(identifier, self.semantic, &self.bindings)?;
            if self.slots[binding.slot.0].ty != Type::ManagedValue {
                return Ok(false);
            }
            if !binding.mutable {
                return Err(unsupported(
                    node,
                    "assignment to a const binding is not admitted",
                ));
            }
            Some(binding.slot)
        } else {
            None
        };
        let property = if let Some((object, key)) = property {
            let receiver = self.receiver_type(object, depth + 2);
            if !matches!(receiver, Some(Type::ManagedObject | Type::ManagedArray)) {
                return Ok(false);
            }
            let array = receiver == Some(Type::ManagedArray);
            let key =
                managed_member_key(node, key, array, self.stored.dictionary(object.node_id()))?;
            Some((object, key, array))
        } else {
            None
        };
        if matches!(mode, Mode::Discard) {
            work.push(Work::Forget);
        } else if matches!(mode, Mode::Predicate) {
            work.push(Work::Managed(ManagedWork::Truthy(node)));
        }
        if let Some(slot) = local {
            let left = self.value(Type::ManagedValue);
            Self::push(regions.current(), node, Op::Load { result: left, slot });
            work.push(Work::StringAddition(Task::Write {
                node,
                destination: Destination::Local(slot),
                left,
            }));
            work.push(Work::Eval(&assignment.right, depth + 1, Mode::Managed));
        } else if let Some((object, key, array)) = property {
            work.push(Work::StringAddition(Task::Receiver {
                node,
                right: &assignment.right,
                key,
                depth: depth + 1,
            }));
            work.push(Work::Eval(
                object,
                depth + 2,
                if array { Mode::Array } else { Mode::Object },
            ));
        }
        Ok(true)
    }

    pub(super) fn resume_string_addition<'e, 'a>(
        &mut self,
        task: Task<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) {
        let (node, destination, left) = match task {
            Task::Receiver {
                node,
                right,
                key,
                depth,
            } => {
                let object = values.pop().expect("String write receiver").id();
                let left = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    node,
                    if self.values[object.0] == Type::ManagedArray {
                        Op::ManagedArrayGetOwn {
                            result: left,
                            array: object,
                            key: key.clone(),
                        }
                    } else {
                        Op::ManagedGetOwn {
                            result: left,
                            object,
                            key: key.clone(),
                        }
                    },
                );
                work.push(Work::StringAddition(Task::Write {
                    node,
                    destination: Destination::Property { object, key },
                    left,
                }));
                work.push(Work::Eval(right, depth, Mode::Managed));
                return;
            }
            Task::Write {
                node,
                destination,
                left,
            } => (node, destination, left),
        };
        let right = values.pop().expect("String compound RHS").id();
        let result = self.value(Type::ManagedValue);
        Self::push(
            regions.current(),
            node,
            Op::ManagedStringConcat {
                result,
                left,
                right,
            },
        );
        Self::push(
            regions.current(),
            node,
            match destination {
                Destination::Local(slot) => Op::Store {
                    slot,
                    value: result,
                },
                Destination::Property { object, key } => {
                    if self.values[object.0] == Type::ManagedArray {
                        Op::ManagedArraySetOwn {
                            array: object,
                            key,
                            value: result,
                        }
                    } else {
                        Op::ManagedSetOwn {
                            object,
                            key,
                            value: result,
                        }
                    }
                }
            },
        );
        values.push(Value::Managed(result));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn string_property_compounds_require_static_string_data_references() {
        for (body, admitted) in [
            ("const o={s:'x'};return (o.s+='y').length;", true),
            ("const xs=['x'];return (xs[0]+='y').length;", true),
            // Dictionary fields retain an absent alternative; a prior plain
            // write is not a flow-sensitive proof of presence.
            (
                "const o=Object.create(null);o.s='x';o.s+='y';return o.s.length;",
                false,
            ),
            ("const o={s:'x'};o.s+=a;return o.s.length;", false),
            ("const o={n:a};o.n+='y';return o.n;", false),
            ("const o={s:a?'x':b};o.s+='y';return o.s.length;", false),
            ("const o={};o.s+='y';return o.s.length;", false),
            (
                "const o={s:'x'};delete o.s;o.s+='y';return o.s.length;",
                false,
            ),
            (
                "const o={s:'x'};const alias=o;alias.s=a;o.s+='y';return o.s.length;",
                false,
            ),
            (
                "const o={s:'x'};const key='s';o[key]+='y';return o.s.length;",
                false,
            ),
            ("const o={toString:'x'};o.toString+='y';return a;", false),
            ("const xs=['x'];xs.length+='y';return a;", false),
            // The existing Number path still converts represented primitives.
            ("const o={n:a};o.n+=undefined;return o.n;", true),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
                if let Err(error) = result {
                    assert!(!error.contains("internal IR"), "{source}: {error}");
                }
            }
        }
        assert!(crate::compile("export function run(a,b){const o={s:'x'};o.s+=(a as unknown as string);return o.s.length;}", true).is_err());
    }

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
