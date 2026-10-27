//! Explicit control for Number/Predicate/General/Function/Discard evaluation.
//! Requests borrow original AST nodes. Only this cursor owns unfinished regions;
//! continuations own completed children until their parent operation is emitted.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum Mode {
    Number,
    Predicate,
    Discard,
    Arithmetic,
    General,
    Function(usize),
}

#[derive(Clone, Copy)]
enum Value {
    Number(ValueId),
    Predicate(ValueId),
    General(ValueId),
    Function(ValueId),
}
impl Value {
    fn id(self) -> ValueId {
        match self {
            Self::Number(id) | Self::Predicate(id) | Self::General(id) | Self::Function(id) => id,
        }
    }
    fn general(self) -> ValueId {
        let Self::General(id) = self else {
            unreachable!("General continuation")
        };
        id
    }
    fn number(self) -> ValueId {
        let Self::Number(id) = self else {
            unreachable!("Number continuation")
        };
        id
    }
    fn predicate(self) -> ValueId {
        let Self::Predicate(id) = self else {
            unreachable!("Predicate continuation")
        };
        id
    }
}

struct Regions<'r> {
    root: &'r mut Region,
    children: Vec<Region>,
}

fn call_result<'e, 'a>(
    work: &mut Vec<Work<'e, 'a>>,
    node: &'e Expression<'a>,
    mode: Mode,
    ty: Type,
) -> Result<(), String> {
    match (ty, mode) {
        (Type::Number, Mode::General) => work.push(Work::Pack(node)),
        (Type::Number, Mode::Predicate) => work.push(Work::Truthy(node)),
        (Type::General, Mode::Predicate) => work.push(Work::GeneralTruthy(node)),
        (Type::General, Mode::Number) => {
            return Err(unsupported(
                node,
                "primitive closure result requires a value or coercion position",
            ));
        }
        (Type::Number, Mode::Number) | (Type::General, Mode::General) => (),
        _ => unreachable!("scalar call result after mode routing"),
    }
    Ok(())
}
impl Regions<'_> {
    fn current(&mut self) -> &mut Region {
        self.children.last_mut().unwrap_or(self.root)
    }
    fn finish(&mut self, value: ValueId) -> Region {
        let mut child = self.children.pop().expect("owned child region");
        child.exit = Exit::Yield(value);
        child
    }
}

enum Work<'e, 'a> {
    Eval(&'e Expression<'a>, usize, Mode),
    Forget,
    Await(&'e Expression<'a>, bool, Type),
    FinishAwaitCall(&'e Expression<'a>, Type),
    VoidResult(&'e Expression<'a>),
    Pack(&'e Expression<'a>),
    Convert(&'e Expression<'a>),
    Truthy(&'e Expression<'a>),
    GeneralTruthy(&'e Expression<'a>),
    NumberFromGeneral(&'e Expression<'a>),
    Not(&'e Expression<'a>),
    Unary(&'e Expression<'a>, Unary),
    BinaryRight {
        node: &'e Expression<'a>,
        right: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
        operator: Binary,
    },
    Binary {
        node: &'e Expression<'a>,
        left: ValueId,
        operator: Binary,
    },
    PrimitiveCompareRight {
        node: &'e Expression<'a>,
        right: &'e Expression<'a>,
        depth: usize,
        loose: bool,
        negated: bool,
    },
    PrimitiveCompare {
        node: &'e Expression<'a>,
        left: ValueId,
        loose: bool,
        negated: bool,
    },
    CompareRight {
        node: &'e Expression<'a>,
        right: &'e Expression<'a>,
        depth: usize,
        booleans: bool,
        operator: Compare,
    },
    Compare {
        node: &'e Expression<'a>,
        left: ValueId,
        operator: Compare,
    },
    LogicalAssign {
        node: &'e Expression<'a>,
        assignment: &'e oxc_ast::ast::AssignmentExpression<'a>,
        slot: SlotId,
        old: ValueId,
        condition: ValueId,
    },
    StoreGeneral {
        node: &'e Expression<'a>,
        slot: SlotId,
    },
    StoreFunction {
        node: &'e Expression<'a>,
        slot: SlotId,
    },
    Assign {
        node: &'e Expression<'a>,
        slot: SlotId,
        previous: Option<ValueId>,
        operator: Option<Binary>,
    },
    Call {
        node: &'e Expression<'a>,
        call: &'e CallExpression<'a>,
        function: ir::FunctionId,
        index: usize,
        arguments: Vec<ValueId>,
        depth: usize,
    },
    LocalCall {
        node: &'e Expression<'a>,
        call: &'e CallExpression<'a>,
        closure: closures::LocalClosure,
        index: usize,
        arguments: Vec<ValueId>,
        depth: usize,
    },
    ClosureCall {
        node: &'e Expression<'a>,
        call: &'e CallExpression<'a>,
        callee: ValueId,
        index: usize,
        arguments: Vec<ValueId>,
        depth: usize,
    },
    RestoreBindings(Vec<NumericBinding>, bool),
    Sequence {
        remaining: &'e [Expression<'a>],
        last: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
    },
    Condition {
        node: &'e Expression<'a>,
        consequent: &'e Expression<'a>,
        alternate: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
    },
    Consequent {
        node: &'e Expression<'a>,
        condition: ValueId,
        alternate: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
    },
    Alternate {
        node: &'e Expression<'a>,
        condition: ValueId,
        consequent: Region,
        mode: Mode,
    },
    LogicalLeft {
        node: &'e Expression<'a>,
        left: &'e Expression<'a>,
        right: &'e Expression<'a>,
        operator: LogicalOperator,
        depth: usize,
        mode: Mode,
    },
    LogicalRight {
        node: &'e Expression<'a>,
        left_node: &'e Expression<'a>,
        left: Value,
        condition: ValueId,
        operator: LogicalOperator,
        mode: Mode,
    },
}

fn predicate_number<'e, 'a>(work: &mut Vec<Work<'e, 'a>>, node: &'e Expression<'a>, depth: usize) {
    work.push(Work::Convert(node));
    work.push(Work::Eval(node, depth, Mode::Predicate));
}

// Selection has one traversal and one set of region-owning continuations for
// every admitted result kind. Constructor/call semantics remain typed leaves.
fn schedule_selection<'e, 'a>(
    work: &mut Vec<Work<'e, 'a>>,
    node: &'e Expression<'a>,
    depth: usize,
    mode: Mode,
) -> Result<bool, String> {
    match node {
        Expression::SequenceExpression(sequence) => {
            let (last, preceding) = sequence.expressions.split_last().ok_or_else(|| {
                unsupported(sequence.as_ref(), "comma expression must have an operand")
            })?;
            work.push(Work::Sequence {
                remaining: preceding,
                last,
                depth: depth + 1,
                mode,
            });
        }
        Expression::ConditionalExpression(conditional) => {
            work.push(Work::Condition {
                node,
                consequent: &conditional.consequent,
                alternate: &conditional.alternate,
                depth: depth + 1,
                mode,
            });
            work.push(Work::Eval(&conditional.test, depth + 1, Mode::Predicate));
        }
        Expression::LogicalExpression(logical) => {
            work.push(Work::LogicalLeft {
                node,
                left: &logical.left,
                right: &logical.right,
                operator: logical.operator,
                depth: depth + 1,
                mode,
            });
            work.push(Work::Eval(&logical.left, depth + 1, mode));
        }
        _ => return Ok(false),
    }
    Ok(true)
}

impl Lowering<'_, '_> {
    // Logical assignment validates and captures the target before entering its RHS.
    // All RHS work stays in the selected owned child; no old assignment route changes.
    fn schedule_logical_assignment<'e, 'a>(
        &mut self,
        node: &'e Expression<'a>,
        assignment: &'e oxc_ast::ast::AssignmentExpression<'a>,
        depth: usize,
        mode: Mode,
        regions: &mut Regions<'_>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        let Some(identifier) = assignment
            .left
            .as_simple_assignment_target()
            .map(|target| target_identifier(target, depth))
            .transpose()?
            .flatten()
        else {
            return Err(unsupported(
                node,
                "assignment target must be an initialized identifier",
            ));
        };
        let binding = numeric_binding(identifier, self.semantic, &self.bindings)?;
        if !binding.mutable {
            return Err(unsupported(
                node,
                "assignment to a const binding is not admitted",
            ));
        }
        let slot = binding.slot;
        let ty = self.slots[slot.0].ty;
        if matches!(ty, Type::Closure(_)) {
            return Err(unsupported(
                node,
                "function bindings support only plain = assignment",
            ));
        }
        let general = ty == Type::General;
        if general && !matches!(mode, Mode::General) {
            return Err(unsupported(
                node,
                "General logical assignments require a primitive value position",
            ));
        }
        let old = self.value(ty);
        Self::push(regions.current(), node, Op::Load { result: old, slot });
        let condition = self.value(Type::Predicate);
        let test = if assignment.operator == AssignmentOperator::LogicalNullish {
            if general {
                Op::GeneralIsNullish {
                    result: condition,
                    input: old,
                }
            } else {
                Op::Boolean {
                    result: condition,
                    value: false,
                }
            }
        } else if general {
            Op::GeneralTruthy {
                result: condition,
                input: old,
            }
        } else {
            Op::Truthy {
                result: condition,
                input: old,
            }
        };
        Self::push(regions.current(), node, test);
        regions.children.push(self.region(&assignment.right));
        work.push(Work::LogicalAssign {
            node,
            assignment,
            slot,
            old,
            condition,
        });
        work.push(Work::Eval(
            &assignment.right,
            depth + 1,
            if general { Mode::General } else { Mode::Number },
        ));
        Ok(())
    }

    pub(super) fn evaluate<'e, 'a>(
        &mut self,
        expression: &'e Expression<'a>,
        region: &mut Region,
        depth: usize,
        mode: Mode,
    ) -> Result<Option<ValueId>, String> {
        #[cfg(test)]
        let _entry = crate::evaluator_tests::entry();
        let mut regions = Regions {
            root: region,
            children: Vec::new(),
        };
        let mut work = vec![Work::Eval(expression, depth, mode)];
        let mut values: Vec<Value> = Vec::new();
        while !work.is_empty() {
            #[cfg(test)]
            crate::evaluator_tests::pending(work.len());
            match work.pop().expect("pending work") {
                Work::VoidResult(node) => {
                    let result = self.value(Type::General);
                    Self::push(
                        regions.current(),
                        node,
                        Op::Nullish {
                            result,
                            value: ir::Nullish::Undefined,
                        },
                    );
                    values.push(Value::General(result));
                }
                Work::FinishAwaitCall(node, ty) => {
                    let mut value = values.pop().expect("async body result").id();
                    let mut body = regions.children.pop().expect("owned async body");
                    if ty == Type::Number {
                        let result = self.value(Type::General);
                        Self::push(
                            &mut body,
                            node,
                            Op::Pack {
                                result,
                                input: value,
                            },
                        );
                        value = result;
                    }
                    body.exit = Exit::Return(value);
                    let result = self.value(Type::General);
                    Self::push(regions.current(), node, Op::AwaitCall { result, body });
                    values.push(Value::General(result));
                }
                Work::Await(node, rejected, ty) => {
                    let input = values.pop().expect("await operand").id();
                    let result = self.value(ty);
                    Self::push(
                        regions.current(),
                        node,
                        Op::Await {
                            result,
                            input,
                            rejected,
                        },
                    );
                    values.push(if ty == Type::General {
                        Value::General(result)
                    } else {
                        Value::Number(result)
                    });
                }
                Work::RestoreBindings(bindings, source_charging) => {
                    self.bindings = bindings;
                    self.source_charging = source_charging;
                    self.active_local_calls -= 1;
                }
                Work::LocalCall {
                    node,
                    call,
                    closure,
                    index,
                    mut arguments,
                    depth,
                } => {
                    // Function arguments are completed directly into this vector;
                    // only a pending Number argument needs the scalar value stack.
                    if arguments.len() < index {
                        arguments.push(values.pop().expect("local call argument").number());
                    }
                    if index < call.arguments.len() {
                        let argument = call.arguments[index].as_expression().ok_or_else(|| {
                            unsupported(node, "spread call arguments are not admitted")
                        })?;
                        let function = self.function_arity(argument, depth);
                        if let Some(arity) = function {
                            let outer = self.function_result_depth;
                            self.function_result_depth = (self.inline_depth + self.statement_depth)
                                .max(outer)
                                + regions.children.len();
                            let value =
                                self.callback_argument(argument, regions.current(), arity, depth);
                            self.function_result_depth = outer;
                            arguments.push(value?);
                        }
                        work.push(Work::LocalCall {
                            node,
                            call,
                            closure,
                            index: index + 1,
                            arguments,
                            depth,
                        });
                        if function.is_none() {
                            work.push(Work::Eval(argument, depth, Mode::Number));
                        }
                    } else {
                        self.charge_closure_expansion(node)?;
                        self.active_local_calls += 1;
                        let arrow = self.arrow(closure.arrow);
                        let previous = std::mem::replace(&mut self.bindings, closure.captures);
                        self.bind_arrow_parameters(arrow, arguments, regions.current())?;
                        let charging = std::mem::replace(&mut self.source_charging, false);
                        work.push(Work::RestoreBindings(previous, charging));
                        if arrow.r#async {
                            regions.children.push(self.region(arrow));
                            work.push(Work::FinishAwaitCall(node, closure.result));
                        }
                        if let Some(expression) = arrow.get_expression() {
                            work.push(Work::Eval(
                                expression,
                                0,
                                if closure.result == Type::General {
                                    Mode::General
                                } else {
                                    Mode::Number
                                },
                            ));
                        } else {
                            let depth = self.inline_depth
                                + self.statement_depth
                                + regions.children.len()
                                + 1;
                            let body = self.arrow_block_result(arrow, depth, closure.result)?;
                            let result = self.value(closure.result);
                            Self::push(regions.current(), node, Op::InlineCall { result, body });
                            values.push(if closure.result == Type::General {
                                Value::General(result)
                            } else {
                                Value::Number(result)
                            });
                        }
                    }
                }
                Work::ClosureCall {
                    node,
                    call,
                    callee,
                    index,
                    mut arguments,
                    depth,
                } => {
                    if arguments.len() < index {
                        arguments.push(values.pop().expect("runtime closure argument").number());
                    }
                    if index < call.arguments.len() {
                        let argument = call.arguments[index].as_expression().ok_or_else(|| {
                            unsupported(node, "spread call arguments are not admitted")
                        })?;
                        let Type::Closure(signature) = self.values[callee.0] else {
                            unreachable!()
                        };
                        let parameter = self
                            .closure_signatures
                            .borrow()
                            .parameter(signature, index)
                            .expect("checked signature");
                        let callback = if let Type::Closure(signature) = parameter {
                            let outer = self.function_result_depth;
                            self.function_result_depth = (self.inline_depth + self.statement_depth)
                                .max(outer)
                                + regions.children.len();
                            let value = self.callback_argument(
                                argument,
                                regions.current(),
                                signature,
                                depth,
                            );
                            self.function_result_depth = outer;
                            arguments.push(value?);
                            true
                        } else {
                            false
                        };
                        work.push(Work::ClosureCall {
                            node,
                            call,
                            callee,
                            index: index + 1,
                            arguments,
                            depth,
                        });
                        if !callback {
                            work.push(Work::Eval(argument, depth, Mode::Number));
                        }
                    } else {
                        self.charge_closure_expansion(node)?;
                        let Type::Closure(signature) = self.values[callee.0] else {
                            unreachable!()
                        };
                        let ty = self
                            .closure_signatures
                            .borrow()
                            .result(signature)
                            .expect("checked signature");
                        let result = self.value(ty);
                        Self::push(
                            regions.current(),
                            node,
                            Op::ClosureCall {
                                result,
                                callee,
                                arguments,
                            },
                        );
                        values.push(if ty == Type::General {
                            Value::General(result)
                        } else {
                            Value::Number(result)
                        });
                    }
                }
                Work::Forget => {
                    values.pop().expect("discarded result");
                }
                Work::Convert(node) => {
                    let input = values.pop().expect("conversion input").predicate();
                    let result = self.value(Type::Number);
                    Self::push(
                        regions.current(),
                        node,
                        Op::NumberFromPredicate { result, input },
                    );
                    values.push(Value::Number(result));
                }
                Work::Pack(node) => {
                    let input = values.pop().expect("pack input").id();
                    let result = self.value(Type::General);
                    Self::push(regions.current(), node, Op::Pack { result, input });
                    values.push(Value::General(result));
                }
                Work::NumberFromGeneral(node) => {
                    let input = values.pop().expect("General numeric input").general();
                    let result = self.value(Type::Number);
                    Self::push(
                        regions.current(),
                        node,
                        Op::NumberFromGeneral { result, input },
                    );
                    values.push(Value::Number(result));
                }
                Work::GeneralTruthy(node) => {
                    let input = values.pop().expect("General truthiness input").general();
                    let result = self.value(Type::Predicate);
                    Self::push(regions.current(), node, Op::GeneralTruthy { result, input });
                    values.push(Value::Predicate(result));
                }
                Work::Truthy(node) => {
                    let input = values.pop().expect("truthiness input").number();
                    let result = self.value(Type::Predicate);
                    Self::push(regions.current(), node, Op::Truthy { result, input });
                    values.push(Value::Predicate(result));
                }
                Work::Not(node) => {
                    let input = values.pop().expect("not input").predicate();
                    let result = self.value(Type::Predicate);
                    Self::push(regions.current(), node, Op::Not { result, input });
                    values.push(Value::Predicate(result));
                }
                Work::Unary(node, operator) => {
                    let input = values.pop().expect("unary input").number();
                    let result = self.value(Type::Number);
                    Self::push(
                        regions.current(),
                        node,
                        Op::Unary {
                            result,
                            operator,
                            input,
                        },
                    );
                    values.push(Value::Number(result));
                }
                Work::BinaryRight {
                    node,
                    right,
                    depth,
                    mode,
                    operator,
                } => {
                    let left = values.pop().expect("binary left").number();
                    work.push(Work::Binary {
                        node,
                        left,
                        operator,
                    });
                    work.push(Work::Eval(right, depth, mode));
                }
                Work::Binary {
                    node,
                    left,
                    operator,
                } => {
                    let right = values.pop().expect("binary right").number();
                    let result = self.value(Type::Number);
                    Self::push(
                        regions.current(),
                        node,
                        Op::Binary {
                            result,
                            operator,
                            left,
                            right,
                        },
                    );
                    values.push(Value::Number(result));
                }
                Work::PrimitiveCompareRight {
                    node,
                    right,
                    depth,
                    loose,
                    negated,
                } => {
                    let left = values.pop().expect("primitive comparison left").general();
                    work.push(Work::PrimitiveCompare {
                        node,
                        left,
                        loose,
                        negated,
                    });
                    work.push(Work::Eval(right, depth, Mode::General));
                }
                Work::PrimitiveCompare {
                    node,
                    left,
                    loose,
                    negated,
                } => {
                    let right = values.pop().expect("primitive comparison right").general();
                    let result = self.value(Type::Predicate);
                    let operation = if loose {
                        Op::PrimitiveLooseEqual {
                            result,
                            left,
                            right,
                            negated,
                        }
                    } else {
                        Op::PrimitiveStrictEqual {
                            result,
                            left,
                            right,
                            negated,
                        }
                    };
                    Self::push(regions.current(), node, operation);
                    values.push(Value::Predicate(result));
                }
                Work::CompareRight {
                    node,
                    right,
                    depth,
                    booleans,
                    operator,
                } => {
                    let left = values.pop().expect("comparison left").number();
                    work.push(Work::Compare {
                        node,
                        left,
                        operator,
                    });
                    if booleans {
                        predicate_number(&mut work, right, depth);
                    } else {
                        work.push(Work::Eval(
                            right,
                            depth,
                            if matches!(
                                operator,
                                Compare::Less
                                    | Compare::LessEqual
                                    | Compare::Greater
                                    | Compare::GreaterEqual
                            ) {
                                Mode::Arithmetic
                            } else {
                                Mode::Number
                            },
                        ));
                    }
                }
                Work::Compare {
                    node,
                    left,
                    operator,
                } => {
                    let right = values.pop().expect("comparison right").number();
                    let result = self.value(Type::Predicate);
                    Self::push(
                        regions.current(),
                        node,
                        Op::Compare {
                            result,
                            operator,
                            left,
                            right,
                        },
                    );
                    values.push(Value::Predicate(result));
                }
                Work::LogicalAssign {
                    node,
                    assignment,
                    slot,
                    old,
                    condition,
                } => {
                    let ty = self.slots[slot.0].ty;
                    let general = ty == Type::General;
                    let value = values.pop().expect("logical assignment RHS");
                    let value = if general {
                        value.general()
                    } else {
                        value.number()
                    };
                    let mut selected = regions.finish(value);
                    Self::push(&mut selected, node, Op::Store { slot, value });
                    let mut skipped = self.region(&assignment.left);
                    skipped.exit = Exit::Yield(old);
                    let (consequent, alternate) =
                        if assignment.operator != AssignmentOperator::LogicalOr {
                            (selected, skipped)
                        } else {
                            (skipped, selected)
                        };
                    let result = self.value(ty);
                    Self::push(
                        regions.current(),
                        node,
                        Op::If {
                            result: Some(result),
                            condition,
                            consequent,
                            alternate,
                        },
                    );
                    values.push(if general {
                        Value::General(result)
                    } else {
                        Value::Number(result)
                    });
                }
                Work::StoreFunction { node, slot } => {
                    Self::push(
                        regions.current(),
                        node,
                        Op::Store {
                            slot,
                            value: values.last().expect("function assignment RHS").id(),
                        },
                    );
                }
                Work::StoreGeneral { node, slot } => {
                    let value = values.pop().expect("General assignment RHS");
                    let stored = match value {
                        Value::General(id) => id,
                        Value::Number(input) => {
                            let result = self.value(Type::General);
                            Self::push(regions.current(), node, Op::Pack { result, input });
                            result
                        }
                        Value::Predicate(_) | Value::Function(_) => {
                            unreachable!("typed General assignment RHS")
                        }
                    };
                    Self::push(
                        regions.current(),
                        node,
                        Op::Store {
                            slot,
                            value: stored,
                        },
                    );
                    values.push(value);
                }
                Work::Assign {
                    node,
                    slot,
                    previous,
                    operator,
                } => {
                    let mut value = values.pop().expect("assignment right").number();
                    if let (Some(left), Some(operator)) = (previous, operator) {
                        let left = if self.slots[slot.0].ty == Type::General {
                            let result = self.value(Type::Number);
                            Self::push(
                                regions.current(),
                                node,
                                Op::NumberFromGeneral {
                                    result,
                                    input: left,
                                },
                            );
                            result
                        } else {
                            left
                        };
                        let result = self.value(Type::Number);
                        Self::push(
                            regions.current(),
                            node,
                            Op::Binary {
                                result,
                                operator,
                                left,
                                right: value,
                            },
                        );
                        value = result;
                    }
                    let stored = if self.slots[slot.0].ty == Type::General {
                        let result = self.value(Type::General);
                        Self::push(
                            regions.current(),
                            node,
                            Op::Pack {
                                result,
                                input: value,
                            },
                        );
                        result
                    } else {
                        value
                    };
                    Self::push(
                        regions.current(),
                        node,
                        Op::Store {
                            slot,
                            value: stored,
                        },
                    );
                    values.push(Value::Number(value));
                }
                Work::Call {
                    node,
                    call,
                    function,
                    index,
                    mut arguments,
                    depth,
                } => {
                    if index != 0 {
                        arguments.push(values.pop().expect("call argument").number());
                    }
                    if index == call.arguments.len() {
                        let ty = self.functions[function.0].return_type;
                        let result = self.value(ty);
                        Self::push(
                            regions.current(),
                            node,
                            Op::Call {
                                result,
                                function,
                                arguments,
                            },
                        );
                        values.push(if ty == Type::General {
                            Value::General(result)
                        } else {
                            Value::Number(result)
                        });
                    } else {
                        // Validate a later spread only after earlier arguments finish.
                        let Some(argument) = call.arguments[index].as_expression() else {
                            return Err(unsupported(
                                node,
                                "spread call arguments are not admitted",
                            ));
                        };
                        work.push(Work::Call {
                            node,
                            call,
                            function,
                            index: index + 1,
                            arguments,
                            depth,
                        });
                        work.push(Work::Eval(argument, depth, Mode::Number));
                    }
                }
                Work::Sequence {
                    remaining,
                    last,
                    depth,
                    mode,
                } => {
                    if let Some((first, rest)) = remaining.split_first() {
                        work.push(Work::Sequence {
                            remaining: rest,
                            last,
                            depth,
                            mode,
                        });
                        work.push(Work::Eval(first, depth, Mode::Discard));
                    } else {
                        work.push(Work::Eval(last, depth, mode));
                    }
                }
                Work::Condition {
                    node,
                    consequent,
                    alternate,
                    depth,
                    mode,
                } => {
                    let condition = values.pop().expect("conditional test").predicate();
                    regions.children.push(self.region(consequent));
                    work.push(Work::Consequent {
                        node,
                        condition,
                        alternate,
                        depth,
                        mode,
                    });
                    work.push(Work::Eval(consequent, depth, mode));
                }
                Work::Consequent {
                    node,
                    condition,
                    alternate,
                    depth,
                    mode,
                } => {
                    let value = values.pop().expect("consequent result").id();
                    let consequent = regions.finish(value);
                    // Preserve allocation order: complete consequent before alternate.
                    regions.children.push(self.region(alternate));
                    work.push(Work::Alternate {
                        node,
                        condition,
                        consequent,
                        mode,
                    });
                    work.push(Work::Eval(alternate, depth, mode));
                }
                Work::Alternate {
                    node,
                    condition,
                    consequent,
                    mode,
                } => {
                    let value = values.pop().expect("alternate result").id();
                    let alternate = regions.finish(value);
                    let ty = match mode {
                        Mode::General => Type::General,
                        Mode::Predicate => Type::Predicate,
                        Mode::Function(signature) => Type::Closure(signature),
                        _ => Type::Number,
                    };
                    let result = self.value(ty);
                    Self::push(
                        regions.current(),
                        node,
                        Op::If {
                            result: Some(result),
                            condition,
                            consequent,
                            alternate,
                        },
                    );
                    values.push(match ty {
                        Type::General => Value::General(result),
                        Type::Predicate => Value::Predicate(result),
                        Type::Number => Value::Number(result),
                        Type::Closure(_) => Value::Function(result),
                    });
                }
                Work::LogicalLeft {
                    node,
                    left: left_node,
                    right,
                    operator,
                    depth,
                    mode,
                } => {
                    let left = values.pop().expect("logical left");
                    let condition = match left {
                        Value::General(input) if operator == LogicalOperator::Coalesce => {
                            let nullish = self.value(Type::Predicate);
                            Self::push(
                                regions.current(),
                                node,
                                Op::GeneralIsNullish {
                                    result: nullish,
                                    input,
                                },
                            );
                            let result = self.value(Type::Predicate);
                            Self::push(
                                regions.current(),
                                node,
                                Op::Not {
                                    result,
                                    input: nullish,
                                },
                            );
                            result
                        }
                        Value::Number(_) | Value::Predicate(_)
                            if operator == LogicalOperator::Coalesce =>
                        {
                            let result = self.value(Type::Predicate);
                            Self::push(
                                regions.current(),
                                node,
                                Op::Boolean {
                                    result,
                                    value: true,
                                },
                            );
                            result
                        }
                        Value::General(input) => {
                            let result = self.value(Type::Predicate);
                            Self::push(
                                regions.current(),
                                node,
                                Op::GeneralTruthy { result, input },
                            );
                            result
                        }
                        Value::Predicate(id) => id,
                        Value::Number(input) => {
                            let result = self.value(Type::Predicate);
                            Self::push(regions.current(), node, Op::Truthy { result, input });
                            result
                        }
                        Value::Function(_) => {
                            // Functions are truthy and non-nullish, so || and ??
                            // both keep this already evaluated left snapshot.
                            let result = self.value(Type::Predicate);
                            Self::push(
                                regions.current(),
                                node,
                                Op::Boolean {
                                    result,
                                    value: true,
                                },
                            );
                            result
                        }
                    };
                    regions.children.push(self.region(right));
                    work.push(Work::LogicalRight {
                        node,
                        left_node,
                        left,
                        condition,
                        operator,
                        mode,
                    });
                    work.push(Work::Eval(right, depth, mode));
                }
                Work::LogicalRight {
                    node,
                    left_node,
                    left,
                    condition,
                    operator,
                    mode,
                } => {
                    let right = values.pop().expect("logical right").id();
                    let selected = regions.finish(right);
                    let mut skipped = self.region(left_node);
                    skipped.exit = Exit::Yield(left.id());
                    let (consequent, alternate) = if operator == LogicalOperator::And {
                        (selected, skipped)
                    } else {
                        (skipped, selected)
                    };
                    let ty = match mode {
                        Mode::General => Type::General,
                        Mode::Predicate => Type::Predicate,
                        Mode::Function(signature) => Type::Closure(signature),
                        _ => Type::Number,
                    };
                    let result = self.value(ty);
                    Self::push(
                        regions.current(),
                        node,
                        Op::If {
                            result: Some(result),
                            condition,
                            consequent,
                            alternate,
                        },
                    );
                    values.push(match ty {
                        Type::General => Value::General(result),
                        Type::Predicate => Value::Predicate(result),
                        Type::Number => Value::Number(result),
                        Type::Closure(_) => Value::Function(result),
                    });
                }
                Work::Eval(node, depth, Mode::Discard) => {
                    if let Some(arity) = self.function_arity(node, depth) {
                        let outer = self.function_result_depth;
                        self.function_result_depth = (self.inline_depth + self.statement_depth)
                            .max(outer)
                            + regions.children.len();
                        let result = self.function_value(node, regions.current(), arity, depth);
                        self.function_result_depth = outer;
                        result?;
                        continue;
                    }
                    // Routing owns classification before the selected mode's guard.
                    let mode = if self.general_binding_shape(node, depth)
                        || self.general_assignment_target(node, depth).is_some()
                    {
                        Mode::General
                    } else if discarded_boolean_shape(node, depth)
                        || self.operand_boolean_shape(node, depth)
                    {
                        Mode::Predicate
                    } else if self.general_logical_shape(node, depth) {
                        Mode::General
                    } else if self.numeric_result_shape(node, depth) {
                        Mode::Number
                    } else {
                        Mode::General
                    };
                    work.push(Work::Forget);
                    work.push(Work::Eval(node, depth, mode));
                }
                Work::Eval(node, depth, Mode::Arithmetic) => {
                    if self.operand_boolean_shape(node, depth) {
                        predicate_number(&mut work, node, depth);
                    } else if self.numeric_result_shape(node, depth) {
                        work.push(Work::Eval(node, depth, Mode::Number));
                    } else {
                        work.push(Work::NumberFromGeneral(node));
                        work.push(Work::Eval(node, depth, Mode::General));
                    }
                }
                Work::Eval(node, depth, mode) => {
                    if (self.active_block_calls != 0
                        || self.function_result_depth != 0
                        || self.active_function_values != 0)
                        && (self.inline_depth + self.statement_depth)
                            .max(self.function_result_depth)
                            + regions.children.len()
                            > 128
                    {
                        return Err(unsupported(
                            node,
                            if matches!(mode, Mode::Function(_)) {
                                "function-value expansion exceeds 128 aggregate nested regions"
                            } else if self.function_result_depth != 0
                                || self.active_function_values != 0
                            {
                                "local function-value expansion exceeds 128 nested regions"
                            } else {
                                "block-arrow expansion exceeds 128 nested regions"
                            },
                        ));
                    }
                    if self.active_local_calls != 0 && regions.children.len() > 128 {
                        return Err(unsupported(
                            node,
                            "local arrow expansion exceeds 128 nested expression regions",
                        ));
                    }
                    if depth > 128 {
                        return Err(unsupported(
                            node,
                            if matches!(mode, Mode::Predicate) {
                                "predicate nesting exceeds this experiment's limit"
                            } else {
                                "expression nesting exceeds this experiment's limit"
                            },
                        ));
                    }
                    if let Some(inner) = erased_inner(node) {
                        work.push(Work::Eval(inner, depth + 1, mode));
                        continue;
                    }
                    if let Expression::ParenthesizedExpression(wrapper) = node {
                        work.push(Work::Eval(&wrapper.expression, depth + 1, mode));
                        continue;
                    }
                    if let Mode::Function(signature) = mode {
                        if schedule_selection(&mut work, node, depth, mode)? {
                            continue;
                        }
                        if let Expression::AssignmentExpression(assignment) = node {
                            let (binding, actual) = self.function_assignment_target(assignment, depth)
                                .ok_or_else(|| unsupported(node, "function assignment requires an initialized function binding"))?;
                            if !binding.mutable {
                                return Err(unsupported(
                                    node,
                                    "assignment to a const binding is not admitted",
                                ));
                            }
                            if assignment.operator != AssignmentOperator::Assign
                                || actual != signature
                            {
                                return Err(unsupported(
                                    node,
                                    "function assignment requires plain = with unchanged arity",
                                ));
                            }
                            work.push(Work::StoreFunction {
                                node,
                                slot: binding.slot,
                            });
                            work.push(Work::Eval(&assignment.right, depth + 1, mode));
                            continue;
                        }
                        let outer = self.function_result_depth;
                        if matches!(node, Expression::CallExpression(_)) {
                            self.function_result_depth = (self.inline_depth + self.statement_depth)
                                .max(outer)
                                + regions.children.len();
                        }
                        let result =
                            self.function_value_leaf(node, regions.current(), signature, depth);
                        self.function_result_depth = outer;
                        values.push(Value::Function(result?));
                        continue;
                    }
                    if matches!(mode, Mode::Predicate)
                        && let Some(signature) = self.function_arity(node, depth)
                    {
                        // Evaluating a function can have constructor/argument/
                        // assignment effects. Its ToBoolean result is always true;
                        // evaluating its body is neither needed nor permitted here.
                        let outer = self.function_result_depth;
                        self.function_result_depth = (self.inline_depth + self.statement_depth)
                            .max(outer)
                            + regions.children.len();
                        let value = self.function_value(node, regions.current(), signature, depth);
                        self.function_result_depth = outer;
                        value?;
                        let result = self.value(Type::Predicate);
                        Self::push(
                            regions.current(),
                            node,
                            Op::Boolean {
                                result,
                                value: true,
                            },
                        );
                        values.push(Value::Predicate(result));
                        continue;
                    }
                    if let Expression::UnaryExpression(unary) = node
                        && unary.operator == UnaryOperator::Void
                    {
                        if matches!(mode, Mode::Predicate) {
                            work.push(Work::GeneralTruthy(node));
                            work.push(Work::Eval(node, depth, Mode::General));
                            continue;
                        }
                        if !matches!(mode, Mode::General) {
                            return Err(unsupported(
                                node,
                                "void result requires a primitive value or coercion position",
                            ));
                        }
                        work.push(Work::VoidResult(node));
                        work.push(Work::Eval(&unary.argument, depth + 1, Mode::Discard));
                        continue;
                    }
                    if let Expression::AwaitExpression(awaited) = node {
                        if !self.allow_await {
                            return Err(unsupported(node, "await requires the async profile"));
                        }
                        if matches!(mode, Mode::Predicate) {
                            work.push(Work::GeneralTruthy(node));
                            work.push(Work::Eval(node, depth, Mode::General));
                            continue;
                        }
                        let mode = if matches!(mode, Mode::General)
                            && self.numeric_result_shape(node, depth)
                        {
                            work.push(Work::Pack(node));
                            Mode::Number
                        } else {
                            mode
                        };
                        let (argument, argument_depth, rejected) =
                            awaited::operand(awaited, depth, self.semantic)?;
                        let ty = if matches!(mode, Mode::General) {
                            Type::General
                        } else {
                            Type::Number
                        };
                        work.push(Work::Await(node, rejected, ty));
                        if let Some((Expression::CallExpression(call), call_depth)) =
                            closures::unwrapped_at(argument, argument_depth)
                            && let Some(mut closure) =
                                self.closure_target(&call.callee, call_depth + 1)
                            && self.arrow(closure.arrow).r#async
                        {
                            if rejected {
                                return Err(unsupported(
                                    node,
                                    "a rejected async-call result would require a Promise-valued reason",
                                ));
                            }
                            if !closure.checked {
                                self.check_local_closure(&mut closure)?;
                                self.capture_cells(&closure, regions.current());
                            }
                            self.check_local_call(call, &closure)?;
                            // AwaitCall produces a General completion pair even
                            // when the normal body result is known Number.
                            work.pop().expect("scheduled caller await");
                            if closure.result == Type::Number && matches!(mode, Mode::Number) {
                                work.push(Work::NumberFromGeneral(node));
                            } else {
                                call_result(&mut work, node, mode, Type::General)?;
                            }
                            work.push(Work::LocalCall {
                                node: argument,
                                call,
                                closure,
                                index: 0,
                                arguments: Vec::new(),
                                depth: call_depth + 1,
                            });
                            continue;
                        }
                        work.push(Work::Eval(argument, argument_depth, mode));
                        continue;
                    }
                    if let Expression::CallExpression(call) = node
                        && self.closure_target(&call.callee, depth + 1).is_none()
                        && let Some(arity) = self.function_arity(&call.callee, depth + 1)
                    {
                        if call.optional || call.type_arguments.is_some() {
                            return Err(unsupported(
                                node,
                                "optional and generic runtime closure calls are not admitted",
                            ));
                        }
                        if call.arguments.len()
                            != self
                                .closure_signatures
                                .borrow()
                                .parameter_count(arity)
                                .expect("checked closure signature")
                        {
                            return Err(unsupported(
                                node,
                                "runtime closure call requires exactly the declared number of arguments",
                            ));
                        }
                        let result_type = self
                            .closure_signatures
                            .borrow()
                            .result(arity)
                            .expect("checked signature");
                        if !matches!(result_type, Type::Number | Type::General) {
                            return Err(unsupported(
                                node,
                                "function-valued runtime calls require a function position",
                            ));
                        }
                        let callee = if let Some((slot, _)) =
                            self.runtime_closure_callee(&call.callee, depth + 1)
                        {
                            let value = self.value(Type::Closure(arity));
                            Self::push(
                                regions.current(),
                                node,
                                Op::Load {
                                    result: value,
                                    slot,
                                },
                            );
                            value
                        } else {
                            let outer = self.function_result_depth;
                            self.function_result_depth = (self.inline_depth + self.statement_depth)
                                .max(outer)
                                + regions.children.len();
                            let value = self.function_value(
                                &call.callee,
                                regions.current(),
                                arity,
                                depth + 1,
                            );
                            self.function_result_depth = outer;
                            value?
                        };
                        call_result(&mut work, node, mode, result_type)?;
                        work.push(Work::ClosureCall {
                            node,
                            call,
                            callee,
                            index: 0,
                            arguments: Vec::new(),
                            depth: depth + 1,
                        });
                        continue;
                    }
                    if let Expression::CallExpression(call) = node
                        && let Some(mut closure) = self.closure_target(&call.callee, depth + 1)
                    {
                        let arrow = self.arrow(closure.arrow);
                        self.check_arrow(arrow)?;
                        if !closure.checked {
                            self.check_local_closure(&mut closure)?;
                            self.capture_cells(&closure, regions.current());
                        }
                        self.check_local_call(call, &closure)?;
                        if closure.factory {
                            return Err(unsupported(
                                node,
                                "function-valued local calls require an immutable closure binding",
                            ));
                        }
                        call_result(&mut work, node, mode, closure.result)?;
                        work.push(Work::LocalCall {
                            node,
                            call,
                            closure,
                            index: 0,
                            arguments: Vec::new(),
                            depth: depth + 1,
                        });
                        continue;
                    }
                    if matches!(mode, Mode::General) {
                        if let Expression::Identifier(identifier) = node
                            && let Ok(binding) =
                                numeric_binding(identifier, self.semantic, &self.bindings)
                            && self.slots[binding.slot.0].ty == Type::General
                        {
                            let slot = binding.slot;
                            let result = self.value(Type::General);
                            Self::push(regions.current(), node, Op::Load { result, slot });
                            values.push(Value::General(result));
                            continue;
                        }
                        let nullish = match node {
                            Expression::NullLiteral(_) => Some(ir::Nullish::Null),
                            Expression::Identifier(identifier)
                                if identifier.name == "undefined"
                                    && identifier.reference_id.get().is_some_and(|reference| {
                                        self.semantic
                                            .scoping()
                                            .get_reference(reference)
                                            .symbol_id()
                                            .is_none()
                                    }) =>
                            {
                                Some(ir::Nullish::Undefined)
                            }
                            _ => None,
                        };
                        // Resolve the original callee symbol, never its spelling or a TS annotation.
                        let general_call = match node {
                            Expression::CallExpression(call) => match &call.callee {
                                Expression::Identifier(identifier) => {
                                    let symbol =
                                        identifier.reference_id.get().and_then(|reference| {
                                            self.semantic
                                                .scoping()
                                                .get_reference(reference)
                                                .symbol_id()
                                        });
                                    self.functions.iter().any(|function| {
                                        Some(function.symbol) == symbol
                                            && function.return_type == Type::General
                                    })
                                }
                                _ => false,
                            },
                            _ => false,
                        };
                        if let Some(value) = nullish {
                            let result = self.value(Type::General);
                            Self::push(regions.current(), node, Op::Nullish { result, value });
                            values.push(Value::General(result));
                            continue;
                        } else if !general_call
                            && self.general_assignment_target(node, depth).is_none()
                            && !self.general_logical_shape(node, depth)
                            && !matches!(
                                node,
                                Expression::ConditionalExpression(_)
                                    | Expression::SequenceExpression(_)
                            )
                        {
                            let scalar = if self.operand_boolean_shape(node, depth) {
                                Mode::Predicate
                            } else {
                                Mode::Number
                            };
                            work.push(Work::Pack(node));
                            work.push(Work::Eval(node, depth, scalar));
                            continue;
                        }
                    }
                    if matches!(mode, Mode::Predicate) {
                        if self.general_predicate_leaf(node, depth)
                            || matches!(node, Expression::LogicalExpression(logical)
                                if logical.operator == LogicalOperator::Coalesce)
                        {
                            work.push(Work::GeneralTruthy(node));
                            work.push(Work::Eval(node, depth, Mode::General));
                            continue;
                        }
                        if let Expression::BooleanLiteral(literal) = node {
                            let result = self.value(Type::Predicate);
                            Self::push(
                                regions.current(),
                                node,
                                Op::Boolean {
                                    result,
                                    value: literal.value,
                                },
                            );
                            values.push(Value::Predicate(result));
                            continue;
                        }
                        if let Expression::Identifier(identifier) = node
                            && let Ok(binding) =
                                numeric_binding(identifier, self.semantic, &self.bindings)
                            && self.slots[binding.slot.0].ty == Type::Predicate
                        {
                            let slot = binding.slot;
                            let result = self.value(Type::Predicate);
                            Self::push(regions.current(), node, Op::Load { result, slot });
                            values.push(Value::Predicate(result));
                            continue;
                        }
                        if let Expression::UnaryExpression(unary) = node
                            && unary.operator == UnaryOperator::LogicalNot
                        {
                            work.push(Work::Not(node));
                            work.push(Work::Eval(&unary.argument, depth + 1, Mode::Predicate));
                            continue;
                        }
                        if let Expression::BinaryExpression(binary) = node {
                            let operator = match binary.operator {
                                BinaryOperator::LessThan => Some(Compare::Less),
                                BinaryOperator::LessEqualThan => Some(Compare::LessEqual),
                                BinaryOperator::GreaterThan => Some(Compare::Greater),
                                BinaryOperator::GreaterEqualThan => Some(Compare::GreaterEqual),
                                BinaryOperator::Equality | BinaryOperator::StrictEquality => {
                                    Some(Compare::Equal)
                                }
                                BinaryOperator::Inequality | BinaryOperator::StrictInequality => {
                                    Some(Compare::NotEqual)
                                }
                                _ => None,
                            };
                            if let Some(operator) = operator {
                                let booleans = matches!(
                                    binary.operator,
                                    BinaryOperator::StrictEquality
                                        | BinaryOperator::StrictInequality
                                ) && self
                                    .operand_boolean_shape(&binary.left, depth + 1)
                                    && self.operand_boolean_shape(&binary.right, depth + 1);
                                let strict = matches!(
                                    binary.operator,
                                    BinaryOperator::StrictEquality
                                        | BinaryOperator::StrictInequality
                                );
                                let equality = strict
                                    || matches!(
                                        binary.operator,
                                        BinaryOperator::Equality | BinaryOperator::Inequality
                                    );
                                if equality
                                    && !(strict && booleans)
                                    && !(self.numeric_result_shape(&binary.left, depth + 1)
                                        && self.numeric_result_shape(&binary.right, depth + 1))
                                {
                                    work.push(Work::PrimitiveCompareRight {
                                        node,
                                        right: &binary.right,
                                        depth: depth + 1,
                                        loose: !strict,
                                        negated: matches!(
                                            binary.operator,
                                            BinaryOperator::Inequality
                                                | BinaryOperator::StrictInequality
                                        ),
                                    });
                                    work.push(Work::Eval(&binary.left, depth + 1, Mode::General));
                                    continue;
                                }
                                work.push(Work::CompareRight {
                                    node,
                                    right: &binary.right,
                                    depth: depth + 1,
                                    booleans,
                                    operator,
                                });
                                if booleans {
                                    predicate_number(&mut work, &binary.left, depth + 1);
                                } else {
                                    work.push(Work::Eval(
                                        &binary.left,
                                        depth + 1,
                                        if equality {
                                            Mode::Number
                                        } else {
                                            Mode::Arithmetic
                                        },
                                    ));
                                }
                                continue;
                            }
                        }
                        if !matches!(
                            node,
                            Expression::ConditionalExpression(_)
                                | Expression::LogicalExpression(_)
                                | Expression::SequenceExpression(_)
                        ) {
                            work.push(Work::Truthy(node));
                            // Predicate fallback does not consume another source node.
                            work.push(Work::Eval(node, depth, Mode::Number));
                            continue;
                        }
                    }
                    if schedule_selection(&mut work, node, depth, mode)? {
                        continue;
                    }
                    match node {
                        Expression::NumericLiteral(literal) => {
                            let result = self.value(Type::Number);
                            Self::push(
                                regions.current(),
                                node,
                                Op::Constant {
                                    result,
                                    value: literal.value,
                                },
                            );
                            values.push(Value::Number(result));
                        }
                        Expression::Identifier(identifier) => {
                            let reference = identifier
                                .reference_id
                                .get()
                                .ok_or("missing semantic reference")?;
                            let constant = match (
                                self.semantic.scoping().get_reference(reference).symbol_id(),
                                identifier.name.as_str(),
                            ) {
                                (None, "Infinity") => Some(f64::INFINITY),
                                (None, "NaN") => Some(f64::NAN),
                                _ => None,
                            };
                            let result = self.value(Type::Number);
                            let kind = if let Some(value) = constant {
                                Op::Constant { result, value }
                            } else {
                                let slot =
                                    numeric_binding(identifier, self.semantic, &self.bindings)?
                                        .slot;
                                if self.slots[slot.0].ty != Type::Number {
                                    return Err(unsupported(
                                        node,
                                        if self.slots[slot.0].ty == Type::General {
                                            "General bindings require a primitive value position"
                                        } else {
                                            "Boolean bindings are not admitted in numeric value positions"
                                        },
                                    ));
                                }
                                Op::Load { result, slot }
                            };
                            Self::push(regions.current(), node, kind);
                            values.push(Value::Number(result));
                        }
                        Expression::CallExpression(call) => {
                            if let Some((operator, method)) = math_unary(call, self.semantic) {
                                if call.arguments.len() != 1 {
                                    return Err(unsupported(
                                        node,
                                        &format!(
                                            "Math.{method} requires exactly one numeric argument"
                                        ),
                                    ));
                                }
                                let Some(argument) = call.arguments[0].as_expression() else {
                                    return Err(unsupported(
                                        node,
                                        "spread call arguments are not admitted",
                                    ));
                                };
                                work.push(Work::Unary(node, operator));
                                work.push(Work::Eval(argument, depth + 1, Mode::Number));
                            } else {
                                let Expression::Identifier(identifier) = &call.callee else {
                                    return Err(unsupported(
                                        node,
                                        "expression is outside the numeric experiment",
                                    ));
                                };
                                let symbol = identifier.reference_id.get().and_then(|reference| {
                                    self.semantic.scoping().get_reference(reference).symbol_id()
                                });
                                let function = self
                                    .functions
                                    .iter()
                                    .find(|function| Some(function.symbol) == symbol)
                                    .ok_or_else(|| {
                                        unsupported(
                                            node,
                                            "expression is outside the numeric experiment",
                                        )
                                    })?;
                                if call.optional || call.type_arguments.is_some() {
                                    return Err(unsupported(
                                        node,
                                        "optional and generic calls are not admitted",
                                    ));
                                }
                                if call.arguments.len() != function.parameter_count {
                                    return Err(unsupported(
                                        node,
                                        "call requires exactly the declared number of arguments",
                                    ));
                                }
                                if function.return_type == Type::General
                                    && !matches!(mode, Mode::General)
                                {
                                    return Err(unsupported(
                                        node,
                                        "value-returning calls require a General result position",
                                    ));
                                }
                                work.push(Work::Call {
                                    node,
                                    call,
                                    function: function.id,
                                    index: 0,
                                    arguments: Vec::new(),
                                    depth: depth + 1,
                                });
                            }
                        }
                        Expression::AssignmentExpression(assignment) => {
                            if matches!(
                                assignment.operator,
                                AssignmentOperator::LogicalAnd
                                    | AssignmentOperator::LogicalOr
                                    | AssignmentOperator::LogicalNullish
                            ) {
                                self.schedule_logical_assignment(
                                    node,
                                    assignment,
                                    depth,
                                    mode,
                                    &mut regions,
                                    &mut work,
                                )?;
                                continue;
                            }
                            let operator = match assignment.operator {
                                AssignmentOperator::Assign => None,
                                AssignmentOperator::Addition => Some(Binary::Add),
                                AssignmentOperator::Subtraction => Some(Binary::Subtract),
                                AssignmentOperator::Multiplication => Some(Binary::Multiply),
                                AssignmentOperator::Division => Some(Binary::Divide),
                                AssignmentOperator::Remainder => Some(Binary::Remainder),
                                AssignmentOperator::BitwiseAnd => Some(Binary::BitAnd),
                                AssignmentOperator::BitwiseOR => Some(Binary::BitOr),
                                AssignmentOperator::BitwiseXOR => Some(Binary::BitXor),
                                AssignmentOperator::ShiftLeft => Some(Binary::ShiftLeft),
                                AssignmentOperator::ShiftRight => Some(Binary::ShiftRight),
                                AssignmentOperator::ShiftRightZeroFill => {
                                    Some(Binary::ShiftRightUnsigned)
                                }
                                _ => {
                                    return Err(unsupported(
                                        node,
                                        "only =, +=, -=, *=, /=, %=, &=, |=, ^=, <<=, >>=, >>>=, &&=, ||= and ??= assignments are admitted",
                                    ));
                                }
                            };
                            let Some(identifier) = assignment
                                .left
                                .as_simple_assignment_target()
                                .map(|target| target_identifier(target, depth))
                                .transpose()?
                                .flatten()
                            else {
                                return Err(unsupported(
                                    node,
                                    "assignment target must be an initialized identifier",
                                ));
                            };
                            let binding =
                                numeric_binding(identifier, self.semantic, &self.bindings)?;
                            if !binding.mutable {
                                return Err(unsupported(
                                    node,
                                    "assignment to a const binding is not admitted",
                                ));
                            }
                            let slot = binding.slot;
                            if matches!(self.slots[slot.0].ty, Type::Closure(_)) {
                                return Err(unsupported(
                                    node,
                                    "function assignments require a function-valued or discarded position",
                                ));
                            }
                            let general = self.slots[slot.0].ty == Type::General;
                            let arithmetic = matches!(
                                operator,
                                Some(
                                    Binary::Add
                                        | Binary::Subtract
                                        | Binary::Multiply
                                        | Binary::Divide
                                        | Binary::Remainder
                                )
                            );
                            if general {
                                if operator.is_some() && !arithmetic {
                                    return Err(unsupported(
                                        node,
                                        "General compound assignments require numeric coercion support",
                                    ));
                                }
                                if operator.is_none() {
                                    work.push(Work::StoreGeneral { node, slot });
                                    work.push(Work::Eval(
                                        &assignment.right,
                                        depth + 1,
                                        if matches!(mode, Mode::Number) {
                                            Mode::Number
                                        } else {
                                            Mode::General
                                        },
                                    ));
                                    continue;
                                }
                            }
                            let previous = if operator.is_some() {
                                let result =
                                    self.value(if general { Type::General } else { Type::Number });
                                Self::push(regions.current(), node, Op::Load { result, slot });
                                Some(result)
                            } else {
                                None
                            };
                            work.push(Work::Assign {
                                node,
                                slot,
                                previous,
                                operator,
                            });
                            work.push(Work::Eval(
                                &assignment.right,
                                depth + 1,
                                if arithmetic {
                                    Mode::Arithmetic
                                } else {
                                    Mode::Number
                                },
                            ));
                        }
                        Expression::UpdateExpression(update) => {
                            let Some(identifier) = target_identifier(&update.argument, depth)?
                            else {
                                return Err(unsupported(
                                    node,
                                    "update target must be an initialized identifier",
                                ));
                            };
                            let binding =
                                numeric_binding(identifier, self.semantic, &self.bindings)?;
                            if !binding.mutable {
                                return Err(unsupported(
                                    node,
                                    "update to a const binding is not admitted",
                                ));
                            }
                            let slot = binding.slot;
                            if matches!(self.slots[slot.0].ty, Type::Closure(_)) {
                                return Err(unsupported(
                                    node,
                                    "function bindings do not support numeric updates",
                                ));
                            }
                            let general = self.slots[slot.0].ty == Type::General;
                            let loaded =
                                self.value(if general { Type::General } else { Type::Number });
                            Self::push(
                                regions.current(),
                                node,
                                Op::Load {
                                    result: loaded,
                                    slot,
                                },
                            );
                            // ToNumeric is ToNumber for the four represented primitive kinds.
                            let previous = if general {
                                let result = self.value(Type::Number);
                                Self::push(
                                    regions.current(),
                                    node,
                                    Op::NumberFromGeneral {
                                        result,
                                        input: loaded,
                                    },
                                );
                                result
                            } else {
                                loaded
                            };
                            let one = self.value(Type::Number);
                            Self::push(
                                regions.current(),
                                node,
                                Op::Constant {
                                    result: one,
                                    value: 1.0,
                                },
                            );
                            let next = self.value(Type::Number);
                            let operator = match update.operator {
                                UpdateOperator::Increment => Binary::Add,
                                UpdateOperator::Decrement => Binary::Subtract,
                            };
                            Self::push(
                                regions.current(),
                                node,
                                Op::Binary {
                                    result: next,
                                    operator,
                                    left: previous,
                                    right: one,
                                },
                            );
                            let stored = if general {
                                let result = self.value(Type::General);
                                Self::push(
                                    regions.current(),
                                    node,
                                    Op::Pack {
                                        result,
                                        input: next,
                                    },
                                );
                                result
                            } else {
                                next
                            };
                            Self::push(
                                regions.current(),
                                node,
                                Op::Store {
                                    slot,
                                    value: stored,
                                },
                            );
                            values.push(Value::Number(if update.prefix { next } else { previous }));
                        }
                        Expression::UnaryExpression(unary) => {
                            let operator = match unary.operator {
                                UnaryOperator::UnaryPlus => Unary::Plus,
                                UnaryOperator::UnaryNegation => Unary::Negate,
                                UnaryOperator::BitwiseNot => Unary::BitNot,
                                _ => {
                                    return Err(unsupported(
                                        node,
                                        "only unary +, - and ~ are admitted",
                                    ));
                                }
                            };
                            work.push(Work::Unary(node, operator));
                            if !matches!(operator, Unary::BitNot) {
                                work.push(Work::Eval(&unary.argument, depth + 1, Mode::Arithmetic));
                            } else if self.operand_boolean_shape(&unary.argument, depth + 1) {
                                predicate_number(&mut work, &unary.argument, depth + 1);
                            } else {
                                work.push(Work::Eval(&unary.argument, depth + 1, Mode::Number));
                            }
                        }
                        Expression::BinaryExpression(binary) => {
                            let operator = match binary.operator {
                                BinaryOperator::Addition => Binary::Add,
                                BinaryOperator::Subtraction => Binary::Subtract,
                                BinaryOperator::Multiplication => Binary::Multiply,
                                BinaryOperator::Division => Binary::Divide,
                                BinaryOperator::Remainder => Binary::Remainder,
                                BinaryOperator::BitwiseAnd => Binary::BitAnd,
                                BinaryOperator::BitwiseOR => Binary::BitOr,
                                BinaryOperator::BitwiseXOR => Binary::BitXor,
                                BinaryOperator::ShiftLeft => Binary::ShiftLeft,
                                BinaryOperator::ShiftRight => Binary::ShiftRight,
                                BinaryOperator::ShiftRightZeroFill => Binary::ShiftRightUnsigned,
                                _ => {
                                    return Err(unsupported(
                                        node,
                                        "only binary +, -, *, /, %, &, |, ^, <<, >> and >>> are admitted",
                                    ));
                                }
                            };
                            let mode = if matches!(
                                operator,
                                Binary::Add
                                    | Binary::Subtract
                                    | Binary::Multiply
                                    | Binary::Divide
                                    | Binary::Remainder
                            ) {
                                Mode::Arithmetic
                            } else {
                                Mode::Number
                            };
                            work.push(Work::BinaryRight {
                                node,
                                right: &binary.right,
                                depth: depth + 1,
                                mode,
                                operator,
                            });
                            work.push(Work::Eval(&binary.left, depth + 1, mode));
                        }
                        _ => {
                            return Err(unsupported(
                                node,
                                "expression is outside the numeric experiment",
                            ));
                        }
                    }
                }
            }
        }
        assert!(regions.children.is_empty(), "all child regions completed");
        let result = if matches!(mode, Mode::Discard) {
            None
        } else {
            Some(values.pop().expect("evaluation result").id())
        };
        assert!(values.is_empty(), "all intermediate values consumed");
        Ok(result)
    }
}
