use super::*;

pub(super) struct Task<'e, 'a> {
    pub node: &'e Expression<'a>,
    pub call: &'e CallExpression<'a>,
    pub array: Option<ValueId>,
    pub argument: usize,
    pub depth: usize,
}

impl Lowering<'_, '_> {
    pub(super) fn resume_array_pop<'e, 'a>(
        &mut self,
        mut task: Task<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        if task.array.is_none() {
            task.array = Some(values.pop().expect("pop receiver").id());
        }
        if task.call.arguments.len() > 1000 {
            return Err(unsupported(
                task.node,
                "at most 1000 pop arguments are admitted",
            ));
        }
        if let Some(argument) = task.call.arguments.get(task.argument) {
            let argument = argument
                .as_expression()
                .ok_or_else(|| unsupported(task.node, "spread call arguments are not admitted"))?;
            let depth = task.depth + 1;
            task.argument += 1;
            work.push(Work::ArrayPop(task));
            work.push(Work::Eval(argument, depth, Mode::Discard));
            return Ok(());
        }
        // Pop ignores argument values, but every effect runs before observing
        // the saved array's current length and removing its final own element.
        let result = self.value(Type::ManagedValue);
        Self::push(
            regions.current(),
            task.node,
            Op::ManagedArrayPop {
                result,
                array: task.array.expect("completed pop receiver"),
            },
        );
        values.push(Value::Managed(result));
        Ok(())
    }
}
