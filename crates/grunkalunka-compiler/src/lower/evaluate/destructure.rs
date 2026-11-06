//! Assignment patterns consume one saved RHS and update existing lexical cells.
use super::*;
use oxc_ast::ast::{ArrayAssignmentTarget, AssignmentTarget};

pub(super) struct Task<'e, 'a> {
    node: &'e Expression<'a>,
    pattern: &'e ArrayAssignmentTarget<'a>,
    slots: Vec<Option<SlotId>>,
    rest: Option<SlotId>,
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
        let binding = numeric_binding(identifier, self.semantic, &self.bindings)?;
        if !binding.mutable || self.slots[binding.slot.0].ty != Type::ManagedValue {
            return Err(unsupported(
                identifier,
                "assignment patterns require initialized mutable managed-value bindings",
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
        let AssignmentTarget::ArrayAssignmentTarget(pattern) = &assignment.left else {
            return Ok(false);
        };
        if assignment.operator != AssignmentOperator::Assign {
            return Err(unsupported(
                node,
                "assignment patterns require plain assignment",
            ));
        }
        if self.receiver_type(&assignment.right, depth + 1) != Some(Type::ManagedArray) {
            return Err(unsupported(
                node,
                "array assignments require a source-proven Array",
            ));
        }
        if !matches!(
            mode,
            Mode::Array | Mode::Managed | Mode::Predicate | Mode::Discard
        ) {
            return Err(unsupported(
                node,
                "array assignment results require a managed position",
            ));
        }
        let slots = pattern
            .elements
            .iter()
            .map(|element| {
                element
                    .as_ref()
                    .map(|element| {
                        let target = element.as_assignment_target().ok_or_else(|| {
                            unsupported(element, "assignment defaults are not yet admitted")
                        })?;
                        self.assignment_pattern_slot(target)
                    })
                    .transpose()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let rest = pattern
            .rest
            .as_ref()
            .map(|rest| self.assignment_pattern_slot(&rest.target))
            .transpose()?;
        work.push(Work::Destructure(Task {
            node,
            pattern,
            slots,
            rest,
            mode,
        }));
        work.push(Work::Eval(&assignment.right, depth + 1, Mode::Array));
        Ok(true)
    }

    pub(super) fn resume_destructure(
        &mut self,
        task: Task<'_, '_>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
    ) -> Result<(), String> {
        let array = values.pop().expect("assignment RHS").id();
        for (index, slot) in task.slots.into_iter().enumerate() {
            if let Some(slot) = slot {
                let index = u32::try_from(index)
                    .map_err(|_| unsupported(task.node, "array assignment exceeds index limit"))?;
                let value =
                    self.read_array_binding_element(task.node, regions.current(), array, index);
                Self::push(regions.current(), task.node, Op::Store { slot, value });
            }
        }
        if let Some(slot) = task.rest {
            let start = u32::try_from(task.pattern.elements.len())
                .map_err(|_| unsupported(task.node, "array assignment exceeds index limit"))?;
            let rest =
                self.collect_array_binding_rest(task.node, regions.current(), array, None, start);
            let value = self.value(Type::ManagedValue);
            Self::push(
                regions.current(),
                task.node,
                Op::ManagedFromArray {
                    result: value,
                    input: rest,
                },
            );
            Self::push(regions.current(), task.node, Op::Store { slot, value });
        }
        match task.mode {
            Mode::Array => values.push(Value::Array(array)),
            Mode::Managed => {
                let result = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    task.node,
                    Op::ManagedFromArray {
                        result,
                        input: array,
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
