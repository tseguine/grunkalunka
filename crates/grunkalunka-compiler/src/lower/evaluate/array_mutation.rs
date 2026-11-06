use super::*;

pub(super) struct Task<'e, 'a> {
    pub node: &'e Expression<'a>,
    pub call: &'e CallExpression<'a>,
    pub array: Option<ValueId>,
    pub operation: crate::lower::array_mutations::Operation,
    pub argument: usize,
    pub depth: usize,
}

impl Lowering<'_, '_> {
    pub(super) fn resume_array_mutation<'e, 'a>(
        &mut self,
        mut task: Task<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        if task.array.is_none() {
            task.array = Some(values.pop().expect("array mutation receiver").id());
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
            work.push(Work::ArrayMutation(task));
            work.push(Work::Eval(argument, depth, Mode::Discard));
            return Ok(());
        }
        // All argument effects finish before observing the saved array.
        let result = self.value(task.operation.result());
        let array = task.array.expect("completed array mutation receiver");
        let (op, value) = match task.operation {
            crate::lower::array_mutations::Operation::Pop => (
                Op::ManagedArrayPop { result, array },
                Value::Managed(result),
            ),
            crate::lower::array_mutations::Operation::Shift => (
                Op::ManagedArrayShift { result, array },
                Value::Managed(result),
            ),
            crate::lower::array_mutations::Operation::Reverse => (
                Op::ManagedArrayReverse { result, array },
                Value::Array(result),
            ),
        };
        Self::push(regions.current(), task.node, op);
        values.push(value);
        Ok(())
    }
}
