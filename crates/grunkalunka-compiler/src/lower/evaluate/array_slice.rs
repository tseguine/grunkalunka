//! Save the receiver and endpoints before invoking the intrinsic sparse copy.
use super::*;

pub(super) struct Task<'e, 'a> {
    pub node: &'e Expression<'a>,
    pub call: &'e CallExpression<'a>,
    pub array: Option<ValueId>,
    pub inputs: [Option<ValueId>; 2],
    pub argument: usize,
    pub pending: bool,
    pub depth: usize,
}

impl Lowering<'_, '_> {
    pub(super) fn resume_array_slice<'e, 'a>(
        &mut self,
        mut task: Task<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        if task.array.is_none() {
            task.array = Some(values.pop().expect("slice receiver").id());
        } else if task.pending {
            task.inputs[task.argument - 1] = Some(values.pop().expect("slice endpoint").id());
            task.pending = false;
        }
        if task.call.arguments.len() > 1000 {
            return Err(unsupported(
                task.node,
                "at most 1000 array slice arguments are admitted",
            ));
        }
        if let Some(argument) = task.call.arguments.get(task.argument) {
            let argument = argument
                .as_expression()
                .ok_or_else(|| unsupported(task.node, "spread call arguments are not admitted"))?;
            task.pending = task.argument < 2
                && !crate::lower::string_methods::undefined(argument, self.semantic);
            let mode = if task.pending {
                Mode::Number
            } else {
                Mode::Discard
            };
            let depth = task.depth + 1;
            task.argument += 1;
            work.push(Work::ArraySlice(task));
            work.push(Work::Eval(argument, depth, mode));
            return Ok(());
        }
        for (input, default) in task.inputs.iter_mut().zip([0.0, f64::INFINITY]) {
            if input.is_none() {
                let result = self.value(Type::Number);
                Self::push(
                    regions.current(),
                    task.node,
                    Op::Constant {
                        result,
                        value: default,
                    },
                );
                *input = Some(result);
            }
        }
        let result = self.value(Type::ManagedArray);
        Self::push(
            regions.current(),
            task.node,
            Op::ManagedArraySlice {
                result,
                array: task.array.expect("slice receiver"),
                start: task.inputs[0].expect("slice start"),
                end: task.inputs[1].expect("slice end"),
            },
        );
        values.push(Value::Array(result));
        Ok(())
    }
}
