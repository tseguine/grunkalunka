//! Constant and earlier-parameter defaults share lazy Undefined selection.
use super::*;
use oxc_ast::{
    AstKind,
    ast::{ArrowFunctionExpression, FormalParameters, Function},
};
use std::collections::BTreeSet;

enum Constant {
    Number(f64),
    Boolean(bool),
    Null,
    Undefined,
    String(Vec<u16>),
}

// Compare resolved symbols in the same parameter list. Source names, body var
// bindings and outer captures cannot establish this initialization-order proof.
fn earlier_parameter<'n, 'a>(
    expression: &'n Expression<'a>,
    semantic: &Semantic<'a>,
) -> Option<(&'n IdentifierReference<'a>, Vec<UnaryOperator>)> {
    let mut node = expression;
    let mut depth = 0;
    let mut operators = Vec::new();
    let identifier = loop {
        let (inner, next) = closures::unwrapped_at(node, depth)?;
        node = inner;
        depth = next;
        match node {
            Expression::UnaryExpression(unary)
                if matches!(
                    unary.operator,
                    UnaryOperator::Void
                        | UnaryOperator::LogicalNot
                        | UnaryOperator::UnaryPlus
                        | UnaryOperator::UnaryNegation
                ) =>
            {
                operators.push(unary.operator);
                node = &unary.argument;
                depth += 1;
            }
            Expression::Identifier(id) => break id.as_ref(),
            _ => return None,
        }
    };
    let symbol = semantic
        .scoping()
        .get_reference(identifier.reference_id.get()?)
        .symbol_id()?;
    let parameters = semantic
        .nodes()
        .ancestor_kinds(expression.node_id())
        .find_map(|kind| match kind {
            AstKind::FormalParameters(parameters) => Some(parameters),
            _ => None,
        })?;
    let index = parameters.items.iter().position(|parameter| {
        parameter
            .initializer
            .as_ref()
            .is_some_and(|value| value.node_id() == expression.node_id())
    })?;
    parameters.items[..index].iter().any(|parameter| {
        matches!(&parameter.pattern, BindingPattern::BindingIdentifier(binding) if binding.symbol_id.get() == Some(symbol))
    }).then_some((identifier,operators))
}

// A direct reference to an already initialized parameter may select a pure
// fallback. Resolve the symbol in this parameter list, not by its spelling:
// a body var, outer capture, self read or later parameter is not initialized.
fn earlier_logical<'n, 'a>(
    expression: &'n Expression<'a>,
    semantic: &Semantic<'a>,
) -> Option<(&'n IdentifierReference<'a>, LogicalOperator, Constant)> {
    let (node, depth) = closures::unwrapped_at(expression, 0)?;
    let Expression::LogicalExpression(logical) = node else {
        return None;
    };
    let (left, _) = closures::unwrapped_at(&logical.left, depth + 1)?;
    let Expression::Identifier(identifier) = left else {
        return None;
    };
    let identifier = identifier.as_ref();
    let symbol = semantic
        .scoping()
        .get_reference(identifier.reference_id.get()?)
        .symbol_id()?;
    let parameters = semantic
        .nodes()
        .ancestor_kinds(expression.node_id())
        .find_map(|kind| match kind {
            AstKind::FormalParameters(parameters) => Some(parameters),
            _ => None,
        })?;
    let index = parameters.items.iter().position(|parameter| {
        parameter
            .initializer
            .as_ref()
            .is_some_and(|value| value.node_id() == expression.node_id())
    })?;
    if !parameters.items[..index].iter().any(|parameter| {
        matches!(&parameter.pattern, BindingPattern::BindingIdentifier(binding) if binding.symbol_id.get() == Some(symbol))
    }) {
        return None;
    }
    Some((identifier, logical.operator, constant(&logical.right, semantic)?))
}

impl Constant {
    fn truthy(&self) -> bool {
        match self {
            Self::Number(value) => *value != 0.0 && !value.is_nan(),
            Self::Boolean(value) => *value,
            Self::String(units) => !units.is_empty(),
            Self::Null | Self::Undefined => false,
        }
    }

    fn nullish(&self) -> bool {
        matches!(self, Self::Null | Self::Undefined)
    }
}

// Defaults must be independent of the caller and free of effects. Evaluate
// every operand, including unselected branches, to reject hidden calls, name
// reads or assignments rather than silently treating them as pure constants.
fn constant(expression: &Expression<'_>, semantic: &Semantic<'_>) -> Option<Constant> {
    enum Task<'n, 'a> {
        Visit(&'n Expression<'a>, usize),
        Unary(UnaryOperator),
        Binary(BinaryOperator),
        Logical(LogicalOperator),
        Conditional,
    }
    let mut work = vec![Task::Visit(expression, 0)];
    let mut values = Vec::new();
    while let Some(task) = work.pop() {
        match task {
            Task::Visit(expression, depth) => {
                let (node, depth) = closures::unwrapped_at(expression, depth)?;
                if depth > 128 {
                    return None;
                }
                let value = match node {
                    Expression::UnaryExpression(unary)
                        if matches!(
                            unary.operator,
                            UnaryOperator::UnaryPlus
                                | UnaryOperator::UnaryNegation
                                | UnaryOperator::LogicalNot
                                | UnaryOperator::Void
                        ) =>
                    {
                        work.push(Task::Unary(unary.operator));
                        work.push(Task::Visit(&unary.argument, depth + 1));
                        continue;
                    }
                    Expression::BinaryExpression(binary)
                        if matches!(
                            binary.operator,
                            BinaryOperator::Addition
                                | BinaryOperator::Subtraction
                                | BinaryOperator::Multiplication
                                | BinaryOperator::Division
                                | BinaryOperator::Remainder
                        ) =>
                    {
                        work.push(Task::Binary(binary.operator));
                        work.push(Task::Visit(&binary.right, depth + 1));
                        work.push(Task::Visit(&binary.left, depth + 1));
                        continue;
                    }
                    Expression::LogicalExpression(logical) => {
                        work.push(Task::Logical(logical.operator));
                        work.push(Task::Visit(&logical.right, depth + 1));
                        work.push(Task::Visit(&logical.left, depth + 1));
                        continue;
                    }
                    Expression::ConditionalExpression(conditional) => {
                        work.push(Task::Conditional);
                        work.push(Task::Visit(&conditional.alternate, depth + 1));
                        work.push(Task::Visit(&conditional.consequent, depth + 1));
                        work.push(Task::Visit(&conditional.test, depth + 1));
                        continue;
                    }
                    Expression::NumericLiteral(n) => Constant::Number(n.value),
                    Expression::BooleanLiteral(b) => Constant::Boolean(b.value),
                    Expression::NullLiteral(_) => Constant::Null,
                    Expression::StringLiteral(s) => {
                        Constant::String(s.value.encode_utf16().collect())
                    }
                    Expression::Identifier(id)
                        if id.reference_id.get().is_some_and(|r| {
                            semantic.scoping().get_reference(r).symbol_id().is_none()
                        }) =>
                    {
                        match id.name.as_str() {
                            "undefined" => Constant::Undefined,
                            "Infinity" => Constant::Number(f64::INFINITY),
                            "NaN" => Constant::Number(f64::NAN),
                            _ => return None,
                        }
                    }
                    _ => return None,
                };
                values.push(value);
            }
            Task::Unary(op) => {
                let operand = values.pop()?;
                let value = match (op, operand) {
                    (UnaryOperator::Void, _) => Constant::Undefined,
                    (UnaryOperator::LogicalNot, operand) => Constant::Boolean(!operand.truthy()),
                    (UnaryOperator::UnaryPlus, Constant::Number(n)) => Constant::Number(n),
                    (UnaryOperator::UnaryNegation, Constant::Number(n)) => Constant::Number(-n),
                    _ => return None,
                };
                values.push(value);
            }
            Task::Binary(op) => {
                let right = values.pop()?;
                let left = values.pop()?;
                let value = match (op, left, right) {
                    (
                        BinaryOperator::Addition,
                        Constant::String(mut left),
                        Constant::String(right),
                    ) => {
                        left.extend(right);
                        Constant::String(left)
                    }
                    (op, Constant::Number(left), Constant::Number(right)) => {
                        Constant::Number(match op {
                            BinaryOperator::Addition => left + right,
                            BinaryOperator::Subtraction => left - right,
                            BinaryOperator::Multiplication => left * right,
                            BinaryOperator::Division => left / right,
                            BinaryOperator::Remainder => left % right,
                            _ => unreachable!("planned constant arithmetic"),
                        })
                    }
                    _ => return None,
                };
                values.push(value);
            }
            Task::Logical(op) => {
                let right = values.pop()?;
                let left = values.pop()?;
                let take_right = match op {
                    LogicalOperator::And => left.truthy(),
                    LogicalOperator::Or => !left.truthy(),
                    LogicalOperator::Coalesce => left.nullish(),
                };
                values.push(if take_right { right } else { left });
            }
            Task::Conditional => {
                let alternate = values.pop()?;
                let consequent = values.pop()?;
                let test = values.pop()?;
                values.push(if test.truthy() { consequent } else { alternate });
            }
        }
    }
    values.pop()
}

pub(crate) fn validate(
    function: &Function<'_>,
    semantic: &Semantic<'_>,
    allowed: bool,
) -> Result<(), String> {
    for parameter in &function.params.items {
        if let Some(initializer) = &parameter.initializer {
            let initializer = initializer.as_ref();
            if !allowed {
                return Err(unsupported(
                    parameter,
                    "parameter modifiers, defaults and decorators are not admitted",
                ));
            }
            if constant(initializer, semantic).is_none()
                && earlier_parameter(initializer, semantic).is_none()
                && earlier_logical(initializer, semantic).is_none()
            {
                return Err(unsupported(
                    initializer,
                    "helper defaults require primitive constants",
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_arrow(
    arrow: &ArrowFunctionExpression<'_>,
    semantic: &Semantic<'_>,
    allowed: bool,
) -> Result<(), String> {
    for parameter in &arrow.params.items {
        if let Some(initializer) = &parameter.initializer
            && (!allowed
                || (constant(initializer.as_ref(), semantic).is_none()
                    && earlier_parameter(initializer.as_ref(), semantic).is_none()
                    && earlier_logical(initializer.as_ref(), semantic).is_none()))
        {
            return Err(unsupported(
                parameter,
                "local arrow parameters cannot have defaults, destructuring or modifiers",
            ));
        }
    }
    Ok(())
}

pub(super) fn present(semantic: &Semantic<'_>) -> bool {
    semantic.nodes().iter().any(|node| match node.kind() {
        AstKind::Function(f) => f.params.items.iter().any(|p| p.initializer.is_some()),
        AstKind::ArrowFunctionExpression(a) => {
            a.params.items.iter().any(|p| p.initializer.is_some())
        }
        _ => false,
    })
}

impl Lowering<'_, '_> {
    pub(crate) fn initialize_helper_defaults(
        &mut self,
        function: &Function<'_>,
        output: &mut Region,
    ) -> Result<(), String> {
        self.initialize_parameter_defaults(&function.params, output)
    }

    pub(super) fn initialize_parameter_defaults(
        &mut self,
        parameters: &FormalParameters<'_>,
        output: &mut Region,
    ) -> Result<(), String> {
        for (index, parameter) in parameters.items.iter().enumerate() {
            let Some(initializer) = &parameter.initializer else {
                continue;
            };
            let initializer = initializer.as_ref();
            let slot = SlotId(index);
            if self.slots[slot.0].ty != Type::ManagedValue {
                return Err(unsupported(
                    parameter,
                    "default parameter requires managed storage",
                ));
            }
            let selected = self.value(Type::ManagedValue);
            Self::push(
                output,
                parameter,
                Op::Load {
                    result: selected,
                    slot,
                },
            );
            let value = self.select_parameter_default(parameter, initializer, selected, output)?;
            Self::push(output, parameter, Op::Store { slot, value });
        }
        Ok(())
    }

    pub(super) fn select_parameter_default(
        &mut self,
        parameter: &impl GetSpan,
        initializer: &Expression<'_>,
        mut selected: ValueId,
        output: &mut Region,
    ) -> Result<ValueId, String> {
        if self.values[selected.0] != Type::ManagedValue {
            let primitive = match self.values[selected.0] {
                Type::General => selected,
                Type::Number | Type::Predicate => {
                    let packed = self.value(Type::General);
                    Self::push(
                        output,
                        parameter,
                        Op::Pack {
                            result: packed,
                            input: selected,
                        },
                    );
                    packed
                }
                _ => {
                    return Err(unsupported(
                        parameter,
                        "default parameter requires represented primitive storage",
                    ));
                }
            };
            selected = self.value(Type::ManagedValue);
            Self::push(
                output,
                parameter,
                Op::ManagedPrimitive {
                    result: selected,
                    input: primitive,
                },
            );
        }
        let undefined = self.binding_undefined(parameter, output);
        let pending = self.prepare_binding_default(parameter, output, selected, undefined);
        let mut consequent = self.region(initializer);
        if let Some((identifier, operator, fallback)) =
            earlier_logical(initializer, self.semantic)
        {
            let slot = numeric_binding(identifier, self.semantic, &self.bindings)?.slot;
            let ty = self.slots[slot.0].ty;
            let input = self.value(ty);
            Self::push(
                &mut consequent,
                initializer,
                Op::Load {
                    result: input,
                    slot,
                },
            );
            let original = if ty == Type::ManagedValue {
                input
            } else {
                let result = self.value(Type::ManagedValue);
                let op = match ty {
                    Type::ManagedArray => Op::ManagedFromArray { result, input },
                    Type::ManagedObject => Op::ManagedFromObject { result, input },
                    Type::Number | Type::Predicate | Type::General => {
                        let primitive = if ty == Type::General {
                            input
                        } else {
                            let packed = self.value(Type::General);
                            Self::push(
                                &mut consequent,
                                initializer,
                                Op::Pack {
                                    result: packed,
                                    input,
                                },
                            );
                            packed
                        };
                        Op::ManagedPrimitive {
                            result,
                            input: primitive,
                        }
                    }
                    _ => {
                        return Err(unsupported(
                            initializer,
                            "earlier logical default requires a represented parameter",
                        ));
                    }
                };
                Self::push(&mut consequent, initializer, op);
                result
            };
            let condition = self.value(Type::Predicate);
            Self::push(
                &mut consequent,
                initializer,
                if operator == LogicalOperator::Coalesce {
                    Op::ManagedIsNullish {
                        result: condition,
                        input: original,
                    }
                } else {
                    Op::ManagedTruthy {
                        result: condition,
                        input: original,
                    }
                },
            );
            let mut selected = self.region(initializer);
            let fallback = self.emit_constant_default(initializer, &mut selected, fallback);
            selected.exit = Exit::Yield(fallback);
            let mut kept = self.region(initializer);
            kept.exit = Exit::Yield(original);
            let (if_true, if_false) = if operator == LogicalOperator::Or {
                (kept, selected)
            } else {
                (selected, kept)
            };
            let result = self.value(Type::ManagedValue);
            Self::push(
                &mut consequent,
                initializer,
                Op::If {
                    result: Some(result),
                    condition,
                    consequent: if_true,
                    alternate: if_false,
                },
            );
            return Ok(self.finish_binding_default(parameter, output, pending, consequent, result));
        }
        if let Some((identifier, operators)) = earlier_parameter(initializer, self.semantic) {
            let slot = numeric_binding(identifier, self.semantic, &self.bindings)?.slot;
            let ty = self.slots[slot.0].ty;
            let input = self.value(ty);
            Self::push(
                &mut consequent,
                initializer,
                Op::Load {
                    result: input,
                    slot,
                },
            );
            let mut value = if ty == Type::ManagedValue {
                input
            } else {
                let result = self.value(Type::ManagedValue);
                let op = match ty {
                    Type::ManagedArray => Op::ManagedFromArray { result, input },
                    Type::ManagedObject => Op::ManagedFromObject { result, input },
                    Type::Number | Type::Predicate | Type::General => {
                        let primitive = if ty == Type::General {
                            input
                        } else {
                            let packed = self.value(Type::General);
                            Self::push(
                                &mut consequent,
                                initializer,
                                Op::Pack {
                                    result: packed,
                                    input,
                                },
                            );
                            packed
                        };
                        Op::ManagedPrimitive {
                            result,
                            input: primitive,
                        }
                    }
                    _ => {
                        return Err(unsupported(
                            initializer,
                            "earlier parameter default requires a represented value",
                        ));
                    }
                };
                Self::push(&mut consequent, initializer, op);
                result
            };
            let mut primitive = self.stored.primitive(identifier.node_id());
            for operator in operators.into_iter().rev() {
                if operator == UnaryOperator::Void {
                    value = self.binding_undefined(initializer, &mut consequent);
                    primitive = true;
                    continue;
                }
                let scalar = if operator == UnaryOperator::LogicalNot {
                    let truth = self.value(Type::Predicate);
                    Self::push(
                        &mut consequent,
                        initializer,
                        Op::ManagedTruthy {
                            result: truth,
                            input: value,
                        },
                    );
                    let result = self.value(Type::Predicate);
                    Self::push(
                        &mut consequent,
                        initializer,
                        Op::Not {
                            result,
                            input: truth,
                        },
                    );
                    result
                } else {
                    if !primitive {
                        return Err(unsupported(
                            initializer,
                            "numeric parameter defaults require a proven four-kind primitive operand",
                        ));
                    }
                    let general = self.value(Type::General);
                    Self::push(
                        &mut consequent,
                        initializer,
                        Op::ManagedToPrimitive {
                            result: general,
                            input: value,
                        },
                    );
                    let number = self.value(Type::Number);
                    Self::push(
                        &mut consequent,
                        initializer,
                        Op::NumberFromGeneral {
                            result: number,
                            input: general,
                        },
                    );
                    if operator == UnaryOperator::UnaryNegation {
                        let result = self.value(Type::Number);
                        Self::push(
                            &mut consequent,
                            initializer,
                            Op::Unary {
                                result,
                                operator: Unary::Negate,
                                input: number,
                            },
                        );
                        result
                    } else {
                        number
                    }
                };
                let general = self.value(Type::General);
                Self::push(
                    &mut consequent,
                    initializer,
                    Op::Pack {
                        result: general,
                        input: scalar,
                    },
                );
                value = self.value(Type::ManagedValue);
                Self::push(
                    &mut consequent,
                    initializer,
                    Op::ManagedPrimitive {
                        result: value,
                        input: general,
                    },
                );
                primitive = true;
            }
            return Ok(self.finish_binding_default(parameter, output, pending, consequent, value));
        }
        let folded = constant(initializer, self.semantic).ok_or_else(|| {
            unsupported(initializer, "helper defaults require primitive constants")
        })?;
        let value = self.emit_constant_default(initializer, &mut consequent, folded);
        Ok(self.finish_binding_default(parameter, output, pending, consequent, value))
    }

    fn emit_constant_default(
        &mut self,
        node: &Expression<'_>,
        region: &mut Region,
        constant: Constant,
    ) -> ValueId {
        match constant {
            Constant::String(units) => {
                let value = self.value(Type::ManagedValue);
                Self::push(
                    region,
                    node,
                    Op::ManagedString {
                        result: value,
                        units,
                    },
                );
                value
            }
            constant => {
                let primitive = self.value(Type::General);
                match constant {
                    Constant::Number(value) => {
                        let scalar = self.value(Type::Number);
                        Self::push(
                            region,
                            node,
                            Op::Constant {
                                result: scalar,
                                value,
                            },
                        );
                        Self::push(
                            region,
                            node,
                            Op::Pack {
                                result: primitive,
                                input: scalar,
                            },
                        );
                    }
                    Constant::Boolean(value) => {
                        let scalar = self.value(Type::Predicate);
                        Self::push(
                            region,
                            node,
                            Op::Boolean {
                                result: scalar,
                                value,
                            },
                        );
                        Self::push(
                            region,
                            node,
                            Op::Pack {
                                result: primitive,
                                input: scalar,
                            },
                        );
                    }
                    Constant::Null | Constant::Undefined => Self::push(
                        region,
                        node,
                        Op::Nullish {
                            result: primitive,
                            value: if matches!(constant, Constant::Null) {
                                ir::Nullish::Null
                            } else {
                                ir::Nullish::Undefined
                            },
                        },
                    ),
                    Constant::String(_) => unreachable!(),
                }
                let value = self.value(Type::ManagedValue);
                Self::push(
                    region,
                    node,
                    Op::ManagedPrimitive {
                        result: value,
                        input: primitive,
                    },
                );
                value
            }

        }
    }

    // Reflection counts parameters before the first default, independently of
    // the full callable ABI. Mixed lengths need a future dynamic reflection path.
    pub(super) fn default_function_length(
        &self,
        object: &Expression<'_>,
        fallback: usize,
    ) -> Result<usize, String> {
        if !present(self.semantic) {
            return Ok(fallback);
        }
        let (names, arrows) = self.stored.origins(object.node_id());
        let mut lengths = BTreeSet::new();
        for node in self.semantic.nodes().iter() {
            match node.kind() {
                AstKind::Function(f)
                    if f.id
                        .as_ref()
                        .and_then(|id| id.symbol_id.get())
                        .is_some_and(|id| names.contains(&id)) =>
                {
                    lengths.insert(
                        f.params
                            .items
                            .iter()
                            .take_while(|p| p.initializer.is_none())
                            .count(),
                    );
                }
                AstKind::ArrowFunctionExpression(a) if arrows.contains(&node.id()) => {
                    lengths.insert(
                        a.params
                            .items
                            .iter()
                            .take_while(|p| p.initializer.is_none())
                            .count(),
                    );
                }
                _ => (),
            }
        }
        if lengths.len() == 1 {
            Ok(*lengths.first().unwrap())
        } else {
            Err(unsupported(
                object,
                "function length requires one proven default-parameter length",
            ))
        }
    }
}
