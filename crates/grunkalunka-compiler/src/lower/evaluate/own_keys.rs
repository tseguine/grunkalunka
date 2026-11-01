use super::*;

pub(super) struct Task<'e, 'a> {
    pub node: &'e Expression<'a>,
    pub call: &'e CallExpression<'a>,
    pub index: usize,
    pub input: Option<ValueId>,
    pub awaiting: bool,
    pub known: bool,
    pub depth: usize,
}

impl Lowering<'_, '_> {
    pub(super) fn resume_own_keys<'e, 'a>(
        &mut self,
        mut task: Task<'e, 'a>,
        regions: &mut Regions<'_>,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        if task.awaiting {
            task.input = Some(values.pop().expect("own-keys argument").id());
            task.awaiting = false;
        }
        if task.index == task.call.arguments.len() {
            // Taking keys happens after every argument, including ignored extras.
            // Retain the first value snapshot while their effects execute.
            let input = if let Some(input) = task.input {
                input
            } else {
                let primitive = self.value(Type::General);
                Self::push(
                    regions.current(),
                    task.node,
                    Op::Nullish {
                        result: primitive,
                        value: ir::Nullish::Undefined,
                    },
                );
                let input = self.value(Type::ManagedValue);
                Self::push(
                    regions.current(),
                    task.node,
                    Op::ManagedPrimitive {
                        result: input,
                        input: primitive,
                    },
                );
                input
            };
            let result = self.value(Type::ManagedArray);
            if task.known {
                Self::push(
                    regions.current(),
                    task.node,
                    Op::ManagedOwnKeys { result, input },
                );
            } else {
                if !self.allow_await {
                    return Err(unsupported(
                        task.node,
                        "nullable Object.keys requires the async completion profile",
                    ));
                }
                let threw = self.value(Type::Predicate);
                let reason = self.value(Type::General);
                Self::push(
                    regions.current(),
                    task.node,
                    Op::TryManagedOwnKeys {
                        result,
                        input,
                        failure: ir::Failure { threw, reason },
                    },
                );
                let mut consequent = self.region(task.node);
                consequent.exit = Exit::Throw(reason);
                let alternate = self.region(task.node);
                Self::push(
                    regions.current(),
                    task.node,
                    Op::If {
                        result: None,
                        condition: threw,
                        consequent,
                        alternate,
                    },
                );
            }
            values.push(Value::Array(result));
            return Ok(());
        }
        if task.call.arguments.len() > 1000 {
            return Err(unsupported(
                task.node,
                "at most 1000 Object.keys arguments are admitted",
            ));
        }
        let argument = task.call.arguments[task.index]
            .as_expression()
            .ok_or_else(|| unsupported(task.node, "spread call arguments are not admitted"))?;
        let mode = if task.index == invocation::offset(task.call) {
            task.awaiting = true;
            match self.managed_source_type(argument, task.depth + 1) {
                Some(Type::ManagedObject) => {
                    task.known = true;
                    Mode::Object
                }
                Some(Type::ManagedArray) => {
                    task.known = true;
                    Mode::Array
                }
                _ if self.numeric_result_shape(argument, task.depth + 1) => {
                    task.known = true;
                    Mode::Number
                }
                _ => {
                    if let Some(arity) = self.function_arity(argument, task.depth + 1) {
                        task.known = true;
                        Mode::Function(arity)
                    } else {
                        task.known = match closures::unwrapped_at(argument, task.depth + 1) {
                            Some((
                                Expression::StringLiteral(_) | Expression::BooleanLiteral(_),
                                _,
                            )) => true,
                            Some((Expression::UnaryExpression(unary), _)) => {
                                unary.operator == UnaryOperator::Typeof
                            }
                            _ => false,
                        };
                        Mode::Managed
                    }
                }
            }
        } else {
            Mode::Discard
        };
        let depth = task.depth + 1;
        task.index += 1;
        work.push(Work::OwnKeys(task));
        work.push(Work::Eval(argument, depth, mode));
        Ok(())
    }
}
