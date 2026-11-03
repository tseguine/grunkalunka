use super::*;

pub(super) struct Task<'e, 'a> {
    pub node: &'e Expression<'a>,
    pub call: &'e CallExpression<'a>,
    pub array: Option<ValueId>,
    pub index: Option<ValueId>,
    pub argument: usize,
    pub depth: usize,
}

impl Lowering<'_, '_> {
    pub(super) fn resume_array_at<'e, 'a>(
        &mut self,
        mut task: Task<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        if task.array.is_none() {
            task.array = Some(values.pop().expect("at receiver").id());
        } else if task.argument == 1 && task.index.is_none() {
            task.index = Some(values.pop().expect("at index").id());
        }
        if task.call.arguments.len() > 1000 {
            return Err(unsupported(
                task.node,
                "at most 1000 at arguments are admitted",
            ));
        }
        if let Some(argument) = task.call.arguments.get(task.argument) {
            let argument = argument
                .as_expression()
                .ok_or_else(|| unsupported(task.node, "spread call arguments are not admitted"))?;
            let mode = if task.argument == 0 {
                Mode::Number
            } else {
                Mode::Discard
            };
            let depth = task.depth + 1;
            task.argument += 1;
            work.push(Work::ArrayAt(task));
            work.push(Work::Eval(argument, depth, mode));
            return Ok(());
        }
        let index = task.index.unwrap_or_else(|| {
            let result = self.value(Type::Number);
            Self::push(
                regions.current(),
                task.node,
                Op::Constant { result, value: 0.0 },
            );
            result
        });
        // Read only after every argument effect. The receiver and Number index
        // are snapshots, while the array's current length and element are live.
        let result = self.value(Type::ManagedValue);
        Self::push(
            regions.current(),
            task.node,
            Op::ManagedArrayAt {
                result,
                array: task.array.expect("completed at receiver"),
                index,
            },
        );
        values.push(Value::Managed(result));
        Ok(())
    }
}
