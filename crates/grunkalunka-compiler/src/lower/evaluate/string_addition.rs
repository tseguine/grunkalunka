//! String-only += saves its reference and old value before evaluating its RHS.
use super::managed::ManagedWork;
use super::*;

pub(super) enum Destination {
    Local(SlotId),
    Property {
        object: ValueId,
        key: ir::ManagedKey,
    },
}

pub(super) enum Task<'e, 'a> {
    Receiver {
        node: &'e Expression<'a>,
        right: &'e Expression<'a>,
        key: property_update::Key<'e, 'a>,
        depth: usize,
    },
    Key {
        node: &'e Expression<'a>,
        right: &'e Expression<'a>,
        object: ValueId,
        kind: dictionary::RuntimeKey,
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
        if !self.stored.scalar_string_operand(target.node_id()) {
            return Ok(false);
        }
        let property = match target {
            SimpleAssignmentTarget::StaticMemberExpression(member) if !member.optional => Some((
                &member.object,
                property_update::Key::Literal(member.property.name.encode_utf16().collect()),
            )),
            SimpleAssignmentTarget::ComputedMemberExpression(member) if !member.optional => {
                let key = if let Some(key) = closures::unwrapped(&member.expression, depth + 2)
                    .and_then(static_keys::literal)
                {
                    property_update::Key::Literal(key)
                } else if let Some((_, kind)) = self.runtime_member_receiver(member, depth + 2) {
                    property_update::Key::Runtime(&member.expression, kind)
                } else {
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
            let key = match key {
                property_update::Key::Literal(key) => {
                    if array && key.as_slice() == [108, 101, 110, 103, 116, 104] {
                        return Err(unsupported(node, "array length requires plain assignment"));
                    }
                    property_update::Key::Literal(managed_member_key(
                        node,
                        key,
                        array,
                        self.stored.dictionary(object.node_id()),
                    )?)
                }
                key => key,
            };
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
            self.schedule_string_operand(&assignment.right, depth + 1, work)?;
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
    ) -> Result<(), String> {
        let (node, right, depth, object, key) = match task {
            Task::Receiver {
                node,
                right,
                key,
                depth,
            } => {
                let object = values.pop().expect("String update receiver").id();
                match key {
                    property_update::Key::Literal(key) => {
                        (node, right, depth, object, ir::ManagedKey::Literal(key))
                    }
                    property_update::Key::Runtime(key, kind) => {
                        work.push(Work::StringAddition(Task::Key {
                            node,
                            right,
                            object,
                            kind,
                            depth,
                        }));
                        self.schedule_runtime_key(key, depth + 1, kind, work)?;
                        return Ok(());
                    }
                }
            }
            Task::Key {
                node,
                right,
                object,
                kind,
                depth,
            } => {
                let key = values.pop().expect("String update key").id();
                (node, right, depth, object, kind.finish(key))
            }
            Task::Write {
                node,
                destination,
                left,
            } => {
                self.finish_string_addition(node, destination, left, regions, values);
                return Ok(());
            }
        };
        let left = self.value(Type::ManagedValue);
        let read = match &key {
            ir::ManagedKey::Literal(name) if self.values[object.0] == Type::ManagedArray => {
                Op::ManagedArrayGetOwn {
                    result: left,
                    array: object,
                    key: name.clone(),
                }
            }
            ir::ManagedKey::Literal(name) => Op::ManagedGetOwn {
                result: left,
                object,
                key: name.clone(),
            },
            key => Op::ManagedGetOwnDynamic {
                result: left,
                object,
                key: key.clone(),
            },
        };
        Self::push(regions.current(), node, read);
        work.push(Work::StringAddition(Task::Write {
            node,
            destination: Destination::Property { object, key },
            left,
        }));
        self.schedule_string_operand(right, depth, work)
    }

    fn finish_string_addition(
        &mut self,
        node: &Expression<'_>,
        destination: Destination,
        left: ValueId,
        regions: &mut Regions,
        values: &mut Vec<Value>,
    ) {
        let Expression::AssignmentExpression(assignment) = node else {
            unreachable!("validated +=")
        };
        let target = assignment
            .left
            .as_simple_assignment_target()
            .expect("validated reference");
        let left = if self.stored.string(target.node_id()) {
            left
        } else {
            let result = self.value(Type::ManagedValue);
            Self::push(
                regions.current(),
                node,
                Op::ManagedScalarString {
                    result,
                    input: left,
                    numbers: self.stored.may_be_number(target.node_id()),
                },
            );
            result
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
                Destination::Property { object, key } => Op::ManagedSetOwnDynamic {
                    object,
                    key,
                    value: result,
                },
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
            ("const o={s:'x'};o.s+=a;return o.s.length;", true),
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
                true,
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
        assert!(crate::compile("export function run(a,b){const o={s:'x'};o.s+=({} as unknown as string);return o.s.length;}", true).is_err());
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
            ("let text='x';text+=a;return text.length;", true),
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
        assert!(crate::compile("export function run(a,b){let text='x';text+=({} as unknown as string);return text.length;}", true).is_err());
    }
}
