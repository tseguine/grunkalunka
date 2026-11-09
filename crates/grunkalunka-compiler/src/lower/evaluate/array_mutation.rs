use super::*;

pub(super) struct Task<'e, 'a> {
    pub node: &'e Expression<'a>,
    pub call: &'e CallExpression<'a>,
    pub array: Option<ValueId>,
    pub operation: crate::lower::array_mutations::Operation,
    pub argument: usize,
    pub depth: usize,
    pub push_values: Vec<ValueId>,
    pub awaiting_push: bool,
}

impl Lowering<'_, '_> {
    pub(super) fn resume_array_mutation<'e, 'a>(
        &mut self,
        mut task: Task<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        use crate::lower::array_mutations::Operation;
        if task.array.is_none() {
            task.array = Some(values.pop().expect("array mutation receiver").id());
        } else if task.awaiting_push {
            task.push_values
                .push(values.pop().expect("array push argument").id());
            task.awaiting_push = false;
        }
        if task.call.arguments.len() > 1000 {
            return Err(unsupported(
                task.node,
                &format!(
                    "at most 1000 {} arguments are admitted",
                    task.operation.name()
                ),
            ));
        }
        if let Some(argument) = task.call.arguments.get(task.argument) {
            let argument = argument
                .as_expression()
                .ok_or_else(|| unsupported(task.node, "spread call arguments are not admitted"))?;
            let depth = task.depth + 1;
            task.argument += 1;
            task.awaiting_push = task.operation == Operation::Push;
            let mode = if task.awaiting_push {
                Mode::Managed
            } else {
                Mode::Discard
            };
            work.push(Work::ArrayMutation(task));
            work.push(Work::Eval(argument, depth, mode));
            return Ok(());
        }
        // All argument effects finish before mutating the saved receiver.
        let array = task.array.expect("completed array mutation receiver");
        if task.operation == Operation::Push {
            let mut last = None;
            for value in task.push_values {
                let result = self.value(Type::Number);
                Self::push(
                    regions.current(),
                    task.node,
                    Op::ManagedArrayPush {
                        result,
                        array,
                        value,
                    },
                );
                last = Some(result);
            }
            let result = if let Some(result) = last {
                result
            } else {
                let result = self.value(Type::Number);
                Self::push(
                    regions.current(),
                    task.node,
                    Op::ManagedArrayLength { result, array },
                );
                result
            };
            values.push(Value::Number(result));
            return Ok(());
        }
        let result = self.value(task.operation.result());
        let (op, value) = match task.operation {
            Operation::Pop => (
                Op::ManagedArrayPop { result, array },
                Value::Managed(result),
            ),
            Operation::Shift => (
                Op::ManagedArrayShift { result, array },
                Value::Managed(result),
            ),
            Operation::ToReversed => {
                let copy = self.value(Type::ManagedArray);
                Self::push(
                    regions.current(),
                    task.node,
                    Op::ManagedArrayRest {
                        result: copy,
                        array,
                        start: 0,
                    },
                );
                (
                    Op::ManagedArrayReverse {
                        result,
                        array: copy,
                    },
                    Value::Array(result),
                )
            }
            Operation::Reverse => (
                Op::ManagedArrayReverse { result, array },
                Value::Array(result),
            ),
            Operation::Push => unreachable!("handled above"),
        };
        Self::push(regions.current(), task.node, op);
        values.push(value);
        Ok(())
    }
}
