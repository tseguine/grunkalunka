use super::*;

pub(super) struct Task<'e, 'a> {
    pub node: &'e Expression<'a>,
    pub call: &'e CallExpression<'a>,
    pub arguments: invocation::Arguments<'e, 'a>,
    pub kind: ir::OwnEnumeration,
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
        regions: &mut Regions,
        values: &mut Vec<Value>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        if task.awaiting {
            task.input = Some(values.pop().expect("own-keys argument").id());
            task.awaiting = false;
        }
        if task.index == task.arguments.len() {
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
                    Op::ManagedEnumerateOwn {
                        result,
                        input,
                        kind: task.kind,
                    },
                );
            } else {
                if !self.allow_await {
                    return Err(unsupported(
                        task.node,
                        &format!(
                            "nullable Object.{} requires the async completion profile",
                            task.kind.name()
                        ),
                    ));
                }
                let threw = self.value(Type::Predicate);
                let reason = self.value(Type::General);
                Self::push(
                    regions.current(),
                    task.node,
                    Op::TryManagedEnumerateOwn {
                        kind: task.kind,
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
        if task.arguments.len() > 1000 {
            return Err(unsupported(
                task.node,
                &format!(
                    "at most 1000 Object.{} arguments are admitted",
                    task.kind.name()
                ),
            ));
        }
        let actual = task.arguments.get(task.index).expect("own-keys argument");
        let argument = actual.as_expression().ok_or_else(|| {
            unsupported(
                task.node,
                "only fixed array-literal call spreads are admitted",
            )
        })?;
        let depth = task.depth + 1 + actual.depth;
        let mode = if task.index == invocation::offset(task.call) {
            task.awaiting = true;
            match self.managed_source_type(argument, depth) {
                Some(Type::ManagedObject) => {
                    task.known = true;
                    Mode::Object
                }
                Some(Type::ManagedArray) => {
                    task.known = true;
                    Mode::Array
                }
                Some(Type::ManagedValue)
                    if self.stored.container(argument.node_id()).is_some()
                        || self.stored.string(argument.node_id()) =>
                {
                    // Boxed pattern leaves and Strings can still prove non-nullness;
                    // unknown or mixed alternatives keep the fallible path.
                    task.known = true;
                    Mode::Managed
                }
                _ if self.numeric_result_shape(argument, depth) => {
                    task.known = true;
                    Mode::Number
                }
                _ => {
                    if let Some(arity) = self.function_arity(argument, depth) {
                        task.known = true;
                        Mode::Function(arity)
                    } else {
                        task.known = match closures::unwrapped_at(argument, depth) {
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
        task.index += 1;
        work.push(Work::OwnKeys(task));
        work.push(Work::Eval(argument, depth, mode));
        Ok(())
    }
}

#[cfg(test)]
mod fixed_own_keys_spread_tests {
    #[test]
    fn object_enumerations_admit_bounded_literal_spreads() {
        for (body, admitted) in [
            ("return Object.keys(...[{x:a}]).length;", true),
            ("return Object.values(...[{x:a}]).length;", true),
            ("return Object.entries(...[{x:a}]).length;", true),
            ("return Object.keys(...[[a,,b]]).length;", true),
            ("return Object.keys(...[...'abc']).length;", false),
            ("return Object.keys(...[...[{x:a}]]).length;", true),
            ("return Object.keys(...[{}],b).length;", true),
            ("return Object.keys(...[{x:a},...[]]).length;", true),
            ("return Object.keys(...[a]).length;", true),
            ("return Object.keys(...[true]).length;", true),
            (
                "const o={x:a};return Object.keys(...[o],delete o.x).length;",
                true,
            ),
            ("const xs=[{x:a}];return Object.keys(...xs).length;", false),
            ("return Object.values(...[{},,a]).length;", false),
            ("return Object.entries(...[null]).length;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
            }
        }
    }

    #[test]
    fn object_enumeration_spread_depth_remains_bounded_on_small_stacks() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                for (depth, admitted) in [(16, true), (129, false)] {
                    let source_array = (0..depth).fold("{x:a}".to_owned(), |s, _| format!("[{s}]"));
                    let source = format!(
                        "export function run(a,b){{return Object.keys(...{source_array}).length;}}"
                    );
                    let result = crate::compile(&source, false);
                    assert_eq!(result.is_ok(), admitted, "{depth}: {:?}", result.err());
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
