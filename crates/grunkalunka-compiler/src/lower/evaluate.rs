//! Explicit control for the complete Number/Predicate/Discard/operand component.
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
}

#[derive(Clone, Copy)]
enum Value {
    Number(ValueId),
    Predicate(ValueId),
    General(ValueId),
}
impl Value {
    fn id(self) -> ValueId {
        match self {
            Self::Number(id) | Self::Predicate(id) | Self::General(id) => id,
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

impl Lowering<'_, '_> {
    // Logical assignment validates and captures the target before entering its RHS.
    // All RHS work stays in the selected owned child; no old assignment route changes.
    fn schedule_numeric_logical_assignment<'e, 'a>(
        &mut self,
        node: &'e Expression<'a>,
        assignment: &'e oxc_ast::ast::AssignmentExpression<'a>,
        depth: usize,
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
        if self.slots[slot.0].ty == Type::General {
            return Err(unsupported(
                node,
                "General logical assignments are not admitted yet",
            ));
        }
        let old = self.value(Type::Number);
        Self::push(regions.current(), node, Op::Load { result: old, slot });
        let condition = self.value(Type::Predicate);
        Self::push(
            regions.current(),
            node,
            Op::Truthy {
                result: condition,
                input: old,
            },
        );
        regions.children.push(self.region(&assignment.right));
        work.push(Work::LogicalAssign {
            node,
            assignment,
            slot,
            old,
            condition,
        });
        work.push(Work::Eval(&assignment.right, depth + 1, Mode::Number));
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
                    let value = values.pop().expect("logical assignment RHS").number();
                    let mut selected = regions.finish(value);
                    Self::push(&mut selected, node, Op::Store { slot, value });
                    let mut skipped = self.region(&assignment.left);
                    skipped.exit = Exit::Yield(old);
                    let (consequent, alternate) =
                        if assignment.operator == AssignmentOperator::LogicalAnd {
                            (selected, skipped)
                        } else {
                            (skipped, selected)
                        };
                    let result = self.value(Type::Number);
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
                    values.push(Value::Number(result));
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
                        Value::Predicate(_) => unreachable!("typed General assignment RHS"),
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
                    });
                }
                Work::Eval(node, depth, Mode::Discard) => {
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
                        if self.general_predicate_leaf(node, depth) {
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
                    match node {
                        Expression::LogicalExpression(logical) => {
                            if logical.operator == LogicalOperator::Coalesce {
                                return Err(unsupported(
                                    node,
                                    "expression is outside the numeric experiment",
                                ));
                            }
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
                        Expression::SequenceExpression(sequence) => {
                            let (last, preceding) =
                                sequence.expressions.split_last().ok_or_else(|| {
                                    unsupported(
                                        sequence.as_ref(),
                                        "comma expression must have an operand",
                                    )
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
                                AssignmentOperator::LogicalAnd | AssignmentOperator::LogicalOr
                            ) {
                                self.schedule_numeric_logical_assignment(
                                    node,
                                    assignment,
                                    depth,
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
                                        "only =, +=, -=, *=, /=, %=, &=, |=, ^=, <<=, >>=, >>>=, &&= and ||= assignments are admitted",
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
