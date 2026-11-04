use super::*;
use crate::lower::string_methods::Method;

pub(super) struct Task<'e, 'a> {
    pub node: &'e Expression<'a>,
    pub call: &'e CallExpression<'a>,
    pub method: Method,
    pub receiver: Option<ValueId>,
    pub inputs: [Option<ValueId>; 2],
    pub argument: usize,
    pub pending: bool,
    pub depth: usize,
}

impl Lowering<'_, '_> {
    pub(super) fn resume_string_method<'e, 'a>(
        &mut self,
        mut task: Task<'e, 'a>,
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        if task.receiver.is_none() {
            task.receiver = Some(values.pop().expect("String receiver").id());
        } else if task.pending {
            task.inputs[task.argument - 1] = Some(values.pop().expect("String argument").id());
            task.pending = false;
        }
        if task.call.arguments.len() > 1000 {
            return Err(unsupported(
                task.node,
                "at most 1000 String method arguments are admitted",
            ));
        }
        if let Some(argument) = task.call.arguments.get(task.argument) {
            let argument = argument
                .as_expression()
                .ok_or_else(|| unsupported(task.node, "spread call arguments are not admitted"))?;
            let needle =
                matches!(task.method, Method::StartsWith | Method::Includes) && task.argument == 0;
            if needle && !self.stored.string(argument.node_id()) {
                return Err(unsupported(
                    argument,
                    "String search requires a known String needle",
                ));
            }
            task.pending = task.argument < task.method.arguments()
                && (needle || !crate::lower::string_methods::undefined(argument, self.semantic));
            let mode = if needle {
                Mode::Managed
            } else if task.pending {
                Mode::Number
            } else {
                Mode::Discard
            };
            let depth = task.depth + 1;
            task.argument += 1;
            work.push(Work::StringMethod(task));
            work.push(Work::Eval(argument, depth, mode));
            return Ok(());
        }
        let search = matches!(task.method, Method::StartsWith | Method::Includes);
        if search && task.inputs[0].is_none() {
            return Err(unsupported(
                task.node,
                "String search requires a known String needle",
            ));
        }
        let defaults = if search {
            [0.0, 0.0]
        } else {
            [0.0, f64::INFINITY]
        };
        for (input, default) in task
            .inputs
            .iter_mut()
            .zip(defaults)
            .take(task.method.arguments())
        {
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
        let result = self.value(task.method.result());
        let input = task.receiver.expect("completed String receiver");
        let first = || task.inputs[0].expect("completed String first argument");
        let operation = match task.method {
            Method::Slice | Method::Substring => Op::ManagedStringRange {
                result,
                input,
                start: first(),
                end: task.inputs[1].expect("completed String end"),
                substring: matches!(task.method, Method::Substring),
            },
            Method::CharAt => Op::ManagedStringCharAt {
                result,
                input,
                index: first(),
            },
            Method::CharCodeAt => Op::ManagedStringCharCodeAt {
                result,
                input,
                index: first(),
            },
            Method::StartsWith | Method::Includes => Op::ManagedStringSearch {
                result,
                input,
                search: first(),
                position: task.inputs[1].expect("completed String search position"),
                includes: matches!(task.method, Method::Includes),
            },
            Method::Trim | Method::TrimStart | Method::TrimEnd => Op::ManagedStringTrim {
                result,
                input,
                ends: match task.method {
                    Method::TrimStart => ir::StringTrim::Start,
                    Method::TrimEnd => ir::StringTrim::End,
                    _ => ir::StringTrim::Both,
                },
            },
        };
        Self::push(regions.current(), task.node, operation);
        values.push(match task.method.result() {
            Type::ManagedValue => Value::Managed(result),
            Type::Number => Value::Number(result),
            Type::Predicate => Value::Predicate(result),
            _ => unreachable!("known String method result"),
        });
        Ok(())
    }
}
