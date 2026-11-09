use super::*;
use crate::lower::string_methods::Method;

pub(super) struct Task<'e, 'a> {
    pub node: &'e Expression<'a>,
    pub call: &'e CallExpression<'a>,
    pub method: Method,
    pub receiver: Option<ValueId>,
    pub inputs: [Option<ValueId>; 2],
    pub fragments: Vec<ValueId>,
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
            let value = values.pop().expect("String argument").id();
            if matches!(task.method, Method::Concat) {
                task.fragments.push(value);
            } else {
                task.inputs[task.argument - 1] = Some(value);
            }
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
            let concat = matches!(task.method, Method::Concat);
            let needle = concat || (task.method.search() && task.argument == 0);
            if needle && !self.stored.string(argument.node_id()) {
                return Err(unsupported(
                    argument,
                    "String method requires a known String argument",
                ));
            }
            task.pending = concat
                || (task.argument < task.method.arguments()
                    && (needle
                        || !crate::lower::string_methods::undefined(argument, self.semantic)));
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
        if task.method.search() && task.inputs[0].is_none() {
            return Err(unsupported(
                task.node,
                "String method requires a known String argument",
            ));
        }
        for (input, default) in task
            .inputs
            .iter_mut()
            .zip(task.method.defaults())
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
        if matches!(task.method, Method::Concat) {
            // Save every argument before concatenating: argument effects and
            // rejection happen before the intrinsic operation. String-only
            // admission means no user coercion runs during this fold.
            let mut left = task.receiver.expect("concat receiver");
            for right in task.fragments {
                let result = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    task.node,
                    Op::ManagedStringConcat {
                        result,
                        left,
                        right,
                    },
                );
                left = result;
            }
            values.push(Value::Managed(left));
            return Ok(());
        }
        let result = self.value(task.method.result());
        let input = task.receiver.expect("completed String receiver");
        let first = || task.inputs[0].expect("completed String first argument");
        let operation = match task.method {
            Method::Concat => unreachable!("concat completed above"),
            Method::Repeat => Op::ManagedStringRepeat {
                result,
                input,
                count: first(),
            },
            Method::ToWellFormed => Op::ManagedStringToWellFormed { result, input },
            Method::IsWellFormed => Op::ManagedStringIsWellFormed { result, input },
            Method::Slice | Method::Substring => Op::ManagedStringRange {
                result,
                input,
                start: first(),
                end: task.inputs[1].expect("completed String end"),
                substring: matches!(task.method, Method::Substring),
            },
            Method::At => Op::ManagedStringAt {
                result,
                input,
                index: first(),
            },
            Method::CharAt => Op::ManagedStringCharAt {
                result,
                input,
                index: first(),
            },
            Method::CodePointAt => Op::ManagedStringCodePointAt {
                result,
                input,
                index: first(),
            },
            Method::CharCodeAt => Op::ManagedStringCharCodeAt {
                result,
                input,
                index: first(),
            },
            Method::StartsWith | Method::Includes | Method::EndsWith => Op::ManagedStringSearch {
                result,
                input,
                search: first(),
                position: task.inputs[1].expect("completed String search position"),
                kind: match task.method {
                    Method::StartsWith => ir::StringSearch::StartsWith,
                    Method::Includes => ir::StringSearch::Includes,
                    _ => ir::StringSearch::EndsWith,
                },
            },
            Method::IndexOf | Method::LastIndexOf => Op::ManagedStringIndex {
                result,
                input,
                search: first(),
                position: task.inputs[1].expect("completed String search position"),
                backwards: matches!(task.method, Method::LastIndexOf),
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
