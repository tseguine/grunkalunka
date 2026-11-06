//! Assignment patterns consume one saved RHS and update existing lexical cells.
use super::*;
use oxc_ast::ast::{AssignmentTarget, AssignmentTargetProperty};

enum Selection {
    Index(u32),
    Field(Vec<u16>),
    ArrayRest(u32),
    ObjectRest(Vec<ir::ManagedKey>),
}

struct Write {
    selection: Selection,
    slot: SlotId,
}

pub(super) struct Task<'e, 'a> {
    node: &'e Expression<'a>,
    writes: Vec<Write>,
    mode: Mode,
}

impl Lowering<'_, '_> {
    fn assignment_pattern_slot(&self, target: &AssignmentTarget<'_>) -> Result<SlotId, String> {
        let identifier = target
            .as_simple_assignment_target()
            .map(|target| target_identifier(target, 0))
            .transpose()?
            .flatten()
            .ok_or_else(|| unsupported(target, "assignment patterns require identifier targets"))?;
        self.assignment_identifier_slot(identifier)
    }

    fn assignment_identifier_slot(
        &self,
        identifier: &IdentifierReference<'_>,
    ) -> Result<SlotId, String> {
        let binding = numeric_binding(identifier, self.semantic, &self.bindings)?;
        if !binding.mutable || self.slots[binding.slot.0].ty != Type::ManagedValue {
            return Err(unsupported(
                identifier,
                "assignment target must be an initialized identifier with mutable managed-value storage",
            ));
        }
        Ok(binding.slot)
    }

    pub(super) fn schedule_destructure<'e, 'a>(
        &self,
        node: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<bool, String> {
        let Expression::AssignmentExpression(assignment) = node else {
            return Ok(false);
        };
        if assignment.left.as_assignment_target_pattern().is_none() {
            return Ok(false);
        }
        if assignment.operator != AssignmentOperator::Assign {
            return Err(unsupported(
                node,
                "assignment patterns require plain assignment",
            ));
        }
        let receiver = self.receiver_type(&assignment.right, depth + 1);
        let input_mode = match receiver {
            Some(Type::ManagedArray) => Mode::Array,
            Some(Type::ManagedObject) => Mode::Object,
            _ => {
                return Err(unsupported(
                    node,
                    "assignment patterns require a source-proven Array or object",
                ));
            }
        };
        if !matches!(mode, Mode::Managed | Mode::Predicate | Mode::Discard)
            && !matches!(
                (mode, receiver),
                (Mode::Array, Some(Type::ManagedArray)) | (Mode::Object, Some(Type::ManagedObject))
            )
        {
            return Err(unsupported(
                node,
                "assignment pattern results require a compatible managed position",
            ));
        }
        let mut writes = Vec::new();
        match &assignment.left {
            AssignmentTarget::ArrayAssignmentTarget(pattern) => {
                if receiver != Some(Type::ManagedArray) {
                    return Err(unsupported(
                        node,
                        "array assignments require a source-proven Array",
                    ));
                }
                for (index, element) in pattern.elements.iter().enumerate() {
                    if let Some(element) = element {
                        let target = element.as_assignment_target().ok_or_else(|| {
                            unsupported(element, "assignment defaults are not yet admitted")
                        })?;
                        let index = u32::try_from(index).map_err(|_| {
                            unsupported(node, "array assignment exceeds index limit")
                        })?;
                        writes.push(Write {
                            selection: Selection::Index(index),
                            slot: self.assignment_pattern_slot(target)?,
                        });
                    }
                }
                if let Some(rest) = &pattern.rest {
                    let start = u32::try_from(pattern.elements.len())
                        .map_err(|_| unsupported(node, "array assignment exceeds index limit"))?;
                    writes.push(Write {
                        selection: Selection::ArrayRest(start),
                        slot: self.assignment_pattern_slot(&rest.target)?,
                    });
                }
            }
            AssignmentTarget::ObjectAssignmentTarget(pattern) => {
                let dictionary = self.stored.dictionary(assignment.right.node_id());
                let mut excluded = Vec::new();
                for property in &pattern.properties {
                    let (key, slot) = match property {
                        AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(p) => {
                            if p.init.is_some() {
                                return Err(unsupported(
                                    p.as_ref(),
                                    "assignment defaults are not yet admitted",
                                ));
                            }
                            (
                                p.binding.name.encode_utf16().collect(),
                                self.assignment_identifier_slot(&p.binding)?,
                            )
                        }
                        AssignmentTargetProperty::AssignmentTargetPropertyProperty(p) => {
                            let key = static_keys::property(&p.name, p.computed, depth + 1)
                                .ok_or_else(|| {
                                    unsupported(
                                        p.as_ref(),
                                        "object assignment keys require static property names",
                                    )
                                })?;
                            let target = p.binding.as_assignment_target().ok_or_else(|| {
                                unsupported(p.as_ref(), "assignment defaults are not yet admitted")
                            })?;
                            (key, self.assignment_pattern_slot(target)?)
                        }
                    };
                    let key = managed::member_key(
                        property,
                        key,
                        receiver == Some(Type::ManagedArray),
                        dictionary,
                    )?;
                    excluded.push(ir::ManagedKey::Literal(key.clone()));
                    writes.push(Write {
                        selection: Selection::Field(key),
                        slot,
                    });
                }
                if let Some(rest) = &pattern.rest {
                    writes.push(Write {
                        selection: Selection::ObjectRest(excluded),
                        slot: self.assignment_pattern_slot(&rest.target)?,
                    });
                }
            }
            _ => unreachable!("checked assignment pattern"),
        }
        work.push(Work::Destructure(Task { node, writes, mode }));
        work.push(Work::Eval(&assignment.right, depth + 1, input_mode));
        Ok(true)
    }

    pub(super) fn resume_destructure(
        &mut self,
        task: Task<'_, '_>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
    ) -> Result<(), String> {
        let input = values.pop().expect("assignment RHS");
        let receiver = input.id();
        for Write { selection, slot } in task.writes {
            let value = match selection {
                Selection::Index(index) => {
                    self.read_array_binding_element(task.node, regions.current(), receiver, index)
                }
                Selection::Field(key) => {
                    let result = self.value(Type::ManagedValue);
                    let op = if self.values[receiver.0] == Type::ManagedArray {
                        Op::ManagedArrayGetOwn {
                            result,
                            array: receiver,
                            key,
                        }
                    } else {
                        Op::ManagedGetOwn {
                            result,
                            object: receiver,
                            key,
                        }
                    };
                    Self::push(regions.current(), task.node, op);
                    result
                }
                Selection::ArrayRest(start) => {
                    let rest = self.collect_array_binding_rest(
                        task.node,
                        regions.current(),
                        receiver,
                        None,
                        start,
                    );
                    let result = self.value(Type::ManagedValue);
                    Self::push(
                        regions.current(),
                        task.node,
                        Op::ManagedFromArray {
                            result,
                            input: rest,
                        },
                    );
                    result
                }
                Selection::ObjectRest(excluded) => {
                    let rest = self.value(Type::ManagedObject);
                    Self::push(
                        regions.current(),
                        task.node,
                        Op::ManagedObjectRest {
                            result: rest,
                            object: receiver,
                            excluded,
                        },
                    );
                    let result = self.value(Type::ManagedValue);
                    Self::push(
                        regions.current(),
                        task.node,
                        Op::ManagedFromObject {
                            result,
                            input: rest,
                        },
                    );
                    result
                }
            };
            Self::push(regions.current(), task.node, Op::Store { slot, value });
        }
        match task.mode {
            Mode::Array | Mode::Object => values.push(input),
            Mode::Managed => {
                let result = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    task.node,
                    if self.values[receiver.0] == Type::ManagedArray {
                        Op::ManagedFromArray {
                            result,
                            input: receiver,
                        }
                    } else {
                        Op::ManagedFromObject {
                            result,
                            input: receiver,
                        }
                    },
                );
                values.push(Value::Managed(result));
            }
            Mode::Predicate => {
                let result = self.value(Type::Predicate);
                Self::push(
                    regions.current(),
                    task.node,
                    Op::Boolean {
                        result,
                        value: true,
                    },
                );
                values.push(Value::Predicate(result));
            }
            Mode::Discard => (),
            _ => unreachable!("validated array assignment result"),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn assignment_patterns_plan_boxed_local_cells_without_annotation_proofs() {
        for (body, admitted) in [
            ("let x=0,y=0;[x,y]=[a,b];return x===a&&y===b?1:0;", true),
            ("let x,y;({x,y}={x:a,y:b});return x===a&&y===b?1:0;", true),
            ("let tail=[];[,...tail]=[a,b];return tail.length;", true),
            (
                "let rest={};({...rest}={name:'text'});return rest.name==='text'?1:0;",
                true,
            ),
            (
                "let x=0;const read=()=>x;[x]=['text'];return read()==='text'?1:0;",
                true,
            ),
            (
                "let x;const before=x===undefined;[x]=[a];return before&&x===a?1:0;",
                true,
            ),
            ("let x=0;[x]=['text'];return x.length;", false),
            ("let x=0;[x,y]=[a,b];let y=0;return a;", false),
            ("const x=0;[x]=[a];return a;", false),
            ("[a]=[b];return a;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
        assert!(crate::compile("export function run(a:number,b:number):number{let x:number=0;[x]=['text'];return x==='text'?1:0;}", true).is_ok());
    }

    #[test]
    fn object_assignment_admission_and_provenance() {
        for (body, admitted) in [
            ("let x='';({x}={x:'text'});return x.length;", true),
            ("let x='';({name:x}={name:'text'});return x.length;", true),
            (
                "let x='';const d=Object.create(null);d.constructor='text';({constructor:x}=d);return x==='text'?1:0;",
                true,
            ),
            (
                "let x='';const d=Object.create(null);d.constructor='text';({constructor:x}=d);return x.length;",
                false,
            ),
            ("let [x]=[];({0:x}=[a]);return x===a?1:0;", true),
            (
                "let {...rest}={};({...rest}={name:'text'});return rest.name==='text'?1:0;",
                true,
            ),
            ("let x='';({x}={x:a});return x.length;", false),
            ("const x='';({x}={x:'text'});return a;", false),
            ("let x='';({x}=null);return a;", false),
            ("let x='';({constructor:x}={});return a;", false),
            ("let x='';({x='text'}={});return a;", false),
            (
                "let x='';const key='x';({[key]:x}={x:'text'});return a;",
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

    #[test]
    fn array_assignment_admission_and_provenance() {
        for (body, admitted) in [
            ("let x='';[x]=['text'];return x.length;", true),
            ("let [x,y]=[];[x,y]=[a,b];return x===a&&y===b?1:0;", true),
            ("let [tail]=[[]];[,...tail]=[a,b];return tail.length;", true),
            (
                "let x='';const source=['text'];return ([x]=source)===source?1:0;",
                true,
            ),
            ("let x='';if([x]=['x'])return x.length;return 0;", true),
            ("let x='';[x]=[a];return x.length;", false),
            ("const x='';[x]=['text'];return a;", false),
            ("let x='';[x]=a;return a;", false),
            ("[missing]=['text'];return a;", false),
            ("let x='';[x='default']=[];return a;", false),
            ("let x='';[[x]]=[['text']];return a;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
    }
}
