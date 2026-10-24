//! Source admission and AST-to-region lowering. Source limits count source nodes,
//! never the generated regions, values or Wasm temporary locals.
use oxc_ast::ast::{
    BindingPattern, CallExpression, Expression, ForStatementInit, IdentifierReference,
    SimpleAssignmentTarget, Statement, VariableDeclaration, VariableDeclarationKind,
};
use oxc_semantic::{Semantic, SymbolId};
use oxc_span::GetSpan;
use oxc_syntax::operator::{
    AssignmentOperator, BinaryOperator, LogicalOperator, UnaryOperator, UpdateOperator,
};

use crate::ir::{
    self, Binary, Compare, Completion, Exit, Op, Operation, Region, RegionId, Slot, SlotId, Type,
    Unary, ValueId,
};
use crate::{is_type_declaration, unsupported};

pub(super) struct NumericBinding {
    pub symbol: SymbolId,
    pub slot: SlotId,
    pub mutable: bool,
}

pub(super) struct NumericFunction {
    pub symbol: SymbolId,
    pub id: ir::FunctionId,
    pub parameter_count: usize,
}

fn numeric_binding<'b>(
    identifier: &IdentifierReference<'_>,
    semantic: &Semantic<'_>,
    bindings: &'b [NumericBinding],
) -> Result<&'b NumericBinding, String> {
    let reference = identifier
        .reference_id
        .get()
        .ok_or("missing semantic reference")?;
    let symbol = semantic.scoping().get_reference(reference).symbol_id();
    bindings
        .iter()
        .find(|binding| Some(binding.symbol) == symbol)
        .ok_or_else(|| {
            unsupported(
                identifier,
                "reference is not an initialized numeric parameter or local",
            )
        })
}

// The numeric module supplies a pristine intrinsic, never an external realm object.
fn math_unary(call: &CallExpression<'_>, semantic: &Semantic<'_>) -> Option<(Unary, &'static str)> {
    let Expression::StaticMemberExpression(member) = &call.callee else {
        return None;
    };
    let Expression::Identifier(object) = &member.object else {
        return None;
    };
    let reference = object.reference_id.get()?;
    if call.optional
        || call.type_arguments.is_some()
        || member.optional
        || object.name != "Math"
        || semantic
            .scoping()
            .get_reference(reference)
            .symbol_id()
            .is_some()
    {
        return None;
    }
    match member.property.name.as_str() {
        "abs" => Some((Unary::Abs, "abs")),
        "sqrt" => Some((Unary::Sqrt, "sqrt")),
        "floor" => Some((Unary::Floor, "floor")),
        "ceil" => Some((Unary::Ceil, "ceil")),
        "trunc" => Some((Unary::Trunc, "trunc")),
        _ => None,
    }
}

// One wrapper at a time keeps each erased source node in the nesting budget.
// These type-only forms do not convert or validate their runtime operand.
fn erased_inner<'e, 'a>(expression: &'e Expression<'a>) -> Option<&'e Expression<'a>> {
    match expression {
        Expression::TSAsExpression(wrapper) => Some(&wrapper.expression),
        Expression::TSTypeAssertion(wrapper) => Some(&wrapper.expression),
        Expression::TSNonNullExpression(wrapper) => Some(&wrapper.expression),
        Expression::TSSatisfiesExpression(wrapper) => Some(&wrapper.expression),
        _ => None,
    }
}

// Only known Boolean-producing shapes select the bridge. This scan emits nothing
// and is bounded by the source nesting limit. Type assertions never prove a type.
fn boolean_shape(mut expression: &Expression<'_>, mut depth: usize) -> bool {
    while depth <= 128 {
        if let Some(inner) = erased_inner(expression) {
            expression = inner;
        } else if let Expression::ParenthesizedExpression(wrapper) = expression {
            expression = &wrapper.expression;
        } else {
            return match expression {
                Expression::BooleanLiteral(_) => true,
                Expression::UnaryExpression(unary) => unary.operator == UnaryOperator::LogicalNot,
                Expression::BinaryExpression(binary) => matches!(
                    binary.operator,
                    BinaryOperator::LessThan
                        | BinaryOperator::LessEqualThan
                        | BinaryOperator::GreaterThan
                        | BinaryOperator::GreaterEqualThan
                        | BinaryOperator::Equality
                        | BinaryOperator::StrictEquality
                        | BinaryOperator::Inequality
                        | BinaryOperator::StrictInequality
                ),
                _ => false,
            };
        }
        depth += 1;
    }
    // Original lowering still diagnoses over-depth input; no flattened retry.
    false
}

// Inspect only result-producing syntax, before emitting anything. Types asserted by
// TypeScript are not runtime kinds. Ordinary lowering owns the depth diagnostic.
fn discarded_boolean_shape(expression: &Expression<'_>, depth: usize) -> bool {
    if depth > 128 {
        return false;
    }
    if let Some(inner) = erased_inner(expression) {
        return discarded_boolean_shape(inner, depth + 1);
    }
    match expression {
        Expression::BooleanLiteral(_) => true,
        Expression::UnaryExpression(unary) => unary.operator == UnaryOperator::LogicalNot,
        Expression::BinaryExpression(binary) => matches!(
            binary.operator,
            BinaryOperator::LessThan
                | BinaryOperator::LessEqualThan
                | BinaryOperator::GreaterThan
                | BinaryOperator::GreaterEqualThan
                | BinaryOperator::Equality
                | BinaryOperator::StrictEquality
                | BinaryOperator::Inequality
                | BinaryOperator::StrictInequality
        ),
        Expression::ParenthesizedExpression(wrapper) => {
            discarded_boolean_shape(&wrapper.expression, depth + 1)
        }
        Expression::SequenceExpression(sequence) => sequence
            .expressions
            .last()
            .is_some_and(|last| discarded_boolean_shape(last, depth + 1)),
        Expression::ConditionalExpression(conditional) => {
            discarded_boolean_shape(&conditional.consequent, depth + 1)
                || discarded_boolean_shape(&conditional.alternate, depth + 1)
        }
        Expression::LogicalExpression(logical) if logical.operator != LogicalOperator::Coalesce => {
            discarded_boolean_shape(&logical.left, depth + 1)
                || discarded_boolean_shape(&logical.right, depth + 1)
        }
        _ => false,
    }
}

// Resolve only the original reference; never evaluate a write target as a value.
// Oxc discards outer target parentheses, but retained inner nodes each cost depth.
fn target_identifier<'e, 'a>(
    target: &'e SimpleAssignmentTarget<'a>,
    depth: usize,
) -> Result<Option<&'e IdentifierReference<'a>>, String> {
    if let SimpleAssignmentTarget::AssignmentTargetIdentifier(identifier) = target {
        return Ok(Some(identifier));
    }
    let Some(mut expression) = target.get_expression() else {
        return Ok(None);
    };
    let mut depth = depth + 1;
    loop {
        if depth > 128 {
            return Err(unsupported(
                expression,
                "expression nesting exceeds this experiment's limit",
            ));
        }
        match expression {
            Expression::Identifier(identifier) => return Ok(Some(identifier)),
            Expression::ParenthesizedExpression(wrapper) => expression = &wrapper.expression,
            _ => match erased_inner(expression) {
                Some(inner) => expression = inner,
                None => return Ok(None),
            },
        }
        depth += 1;
    }
}

pub(super) struct Lowering<'s, 'a> {
    pub semantic: &'s Semantic<'a>,
    pub functions: &'s [NumericFunction],
    pub bindings: Vec<NumericBinding>,
    pub slots: Vec<Slot>,
    pub values: Vec<Type>,
    pub next_region: usize,
    pub local_count: usize,
    pub statement_count: usize,
    pub in_loop_body: bool,
}

impl Lowering<'_, '_> {
    // Number and Boolean results can be discarded without exposing a value kind.
    // Keep numeric lowering byte-identical; never retry a partially emitted tree.
    fn discarded(
        &mut self,
        expression: &Expression<'_>,
        region: &mut Region,
        depth: usize,
    ) -> Result<(), String> {
        if discarded_boolean_shape(expression, depth) {
            self.predicate(expression, region, depth)?;
        } else {
            self.expression(expression, region, depth)?;
        }
        Ok(())
    }

    pub fn region(&mut self, node: &impl GetSpan) -> Region {
        let id = RegionId(self.next_region);
        self.next_region += 1;
        Region {
            id,
            span: node.span(),
            operations: Vec::new(),
            exit: Exit::Fallthrough,
        }
    }

    fn value(&mut self, ty: Type) -> ValueId {
        let id = ValueId(self.values.len());
        self.values.push(ty);
        id
    }

    fn push(region: &mut Region, node: &impl GetSpan, kind: Op) {
        region.operations.push(Operation {
            span: node.span(),
            kind,
        });
    }

    // Unary bridges may read only published immutable Predicate bindings.
    // Keep original wrapper depth and never infer a runtime kind from type syntax.
    fn unary_boolean_shape(&self, mut expression: &Expression<'_>, mut depth: usize) -> bool {
        while depth <= 128 {
            if let Some(inner) = erased_inner(expression) {
                expression = inner;
            } else if let Expression::ParenthesizedExpression(wrapper) = expression {
                expression = &wrapper.expression;
            } else if let Expression::Identifier(identifier) = expression {
                return numeric_binding(identifier, self.semantic, &self.bindings).is_ok_and(
                    |binding| !binding.mutable && self.slots[binding.slot.0].ty == Type::Predicate,
                );
            } else {
                return boolean_shape(expression, depth);
            }
            depth += 1;
        }
        false
    }

    fn expression(
        &mut self,
        expression: &Expression<'_>,
        region: &mut Region,
        depth: usize,
    ) -> Result<ValueId, String> {
        if depth > 128 {
            return Err(unsupported(
                expression,
                "expression nesting exceeds this experiment's limit",
            ));
        }
        if let Some(inner) = erased_inner(expression) {
            return self.expression(inner, region, depth + 1);
        }
        let kind = match expression {
            Expression::CallExpression(call) => {
                if let Some((operator, method)) = math_unary(call, self.semantic) {
                    if call.arguments.len() != 1 {
                        return Err(unsupported(
                            expression,
                            &format!("Math.{method} requires exactly one numeric argument"),
                        ));
                    }
                    let Some(argument) = call.arguments[0].as_expression() else {
                        return Err(unsupported(
                            expression,
                            "spread call arguments are not admitted",
                        ));
                    };
                    let input = self.expression(argument, region, depth + 1)?;
                    let result = self.value(Type::Number);
                    Op::Unary {
                        result,
                        operator,
                        input,
                    }
                } else {
                    let Expression::Identifier(identifier) = &call.callee else {
                        return Err(unsupported(
                            expression,
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
                            unsupported(expression, "expression is outside the numeric experiment")
                        })?;
                    if call.optional || call.type_arguments.is_some() {
                        return Err(unsupported(
                            expression,
                            "optional and generic calls are not admitted",
                        ));
                    }
                    if call.arguments.len() != function.parameter_count {
                        return Err(unsupported(
                            expression,
                            "call requires exactly the declared number of arguments",
                        ));
                    }
                    let function = function.id;
                    let mut arguments = Vec::new();
                    for argument in &call.arguments {
                        let Some(argument) = argument.as_expression() else {
                            return Err(unsupported(
                                expression,
                                "spread call arguments are not admitted",
                            ));
                        };
                        arguments.push(self.expression(argument, region, depth + 1)?);
                    }
                    let result = self.value(Type::Number);
                    Op::Call {
                        result,
                        function,
                        arguments,
                    }
                }
            }
            Expression::NumericLiteral(literal) => {
                let result = self.value(Type::Number);
                Op::Constant {
                    result,
                    value: literal.value,
                }
            }
            Expression::Identifier(identifier) => {
                let reference = identifier
                    .reference_id
                    .get()
                    .ok_or("missing semantic reference")?;
                // A resolved symbol, including a TDZ local, always uses source binding rules.
                let constant = match (
                    self.semantic.scoping().get_reference(reference).symbol_id(),
                    identifier.name.as_str(),
                ) {
                    (None, "Infinity") => Some(f64::INFINITY),
                    (None, "NaN") => Some(f64::NAN),
                    _ => None,
                };
                let result = self.value(Type::Number);
                if let Some(value) = constant {
                    Op::Constant { result, value }
                } else {
                    let slot = numeric_binding(identifier, self.semantic, &self.bindings)?.slot;
                    if self.slots[slot.0].ty == Type::Predicate {
                        return Err(unsupported(
                            expression,
                            "Boolean bindings are not admitted in numeric value positions",
                        ));
                    }
                    Op::Load { result, slot }
                }
            }
            Expression::AssignmentExpression(assignment) => {
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
                    AssignmentOperator::ShiftRightZeroFill => Some(Binary::ShiftRightUnsigned),
                    _ => {
                        return Err(unsupported(
                            expression,
                            "only =, +=, -=, *=, /=, %=, &=, |=, ^=, <<=, >>= and >>>= assignments are admitted",
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
                        expression,
                        "assignment target must be an initialized identifier",
                    ));
                };
                let binding = numeric_binding(identifier, self.semantic, &self.bindings)?;
                if !binding.mutable {
                    return Err(unsupported(
                        expression,
                        "assignment to a const binding is not admitted",
                    ));
                }
                let slot = binding.slot;
                // Compound assignment reads the target before RHS side effects.
                let previous = if operator.is_some() {
                    let result = self.value(Type::Number);
                    Self::push(region, expression, Op::Load { result, slot });
                    Some(result)
                } else {
                    None
                };
                let value = self.expression(&assignment.right, region, depth + 1)?;
                let value = if let (Some(left), Some(operator)) = (previous, operator) {
                    let result = self.value(Type::Number);
                    Self::push(
                        region,
                        expression,
                        Op::Binary {
                            result,
                            operator,
                            left,
                            right: value,
                        },
                    );
                    result
                } else {
                    value
                };
                Self::push(region, expression, Op::Store { slot, value });
                // An assignment yields the stored value snapshot, never a delayed cell read.
                return Ok(value);
            }
            Expression::UpdateExpression(update) => {
                let Some(identifier) = target_identifier(&update.argument, depth)? else {
                    return Err(unsupported(
                        expression,
                        "update target must be an initialized identifier",
                    ));
                };
                let binding = numeric_binding(identifier, self.semantic, &self.bindings)?;
                if !binding.mutable {
                    return Err(unsupported(
                        expression,
                        "update to a const binding is not admitted",
                    ));
                }
                let slot = binding.slot;
                let previous = self.value(Type::Number);
                Self::push(
                    region,
                    expression,
                    Op::Load {
                        result: previous,
                        slot,
                    },
                );
                let one = self.value(Type::Number);
                Self::push(
                    region,
                    expression,
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
                    region,
                    expression,
                    Op::Binary {
                        result: next,
                        operator,
                        left: previous,
                        right: one,
                    },
                );
                Self::push(region, expression, Op::Store { slot, value: next });
                return Ok(if update.prefix { next } else { previous });
            }
            Expression::LogicalExpression(logical) => {
                if logical.operator == LogicalOperator::Coalesce {
                    return Err(unsupported(
                        expression,
                        "expression is outside the numeric experiment",
                    ));
                }
                let left = self.expression(&logical.left, region, depth + 1)?;
                let condition = self.value(Type::Predicate);
                Self::push(
                    region,
                    expression,
                    Op::Truthy {
                        result: condition,
                        input: left,
                    },
                );
                let mut selected = self.region(&logical.right);
                let right = self.expression(&logical.right, &mut selected, depth + 1)?;
                selected.exit = Exit::Yield(right);
                let mut skipped = self.region(&logical.left);
                skipped.exit = Exit::Yield(left);
                let (consequent, alternate) = if logical.operator == LogicalOperator::And {
                    (selected, skipped)
                } else {
                    (skipped, selected)
                };
                let result = self.value(Type::Number);
                Self::push(
                    region,
                    expression,
                    Op::If {
                        result: Some(result),
                        condition,
                        consequent,
                        alternate,
                    },
                );
                return Ok(result);
            }
            Expression::SequenceExpression(sequence) => {
                let (last, preceding) = sequence.expressions.split_last().ok_or_else(|| {
                    unsupported(sequence.as_ref(), "comma expression must have an operand")
                })?;
                for operand in preceding {
                    self.discarded(operand, region, depth + 1)?;
                }
                return self.expression(last, region, depth + 1);
            }
            Expression::ParenthesizedExpression(parenthesized) => {
                return self.expression(&parenthesized.expression, region, depth + 1);
            }
            Expression::ConditionalExpression(conditional) => {
                let condition = self.predicate(&conditional.test, region, depth + 1)?;
                let mut consequent = self.region(&conditional.consequent);
                let value = self.expression(&conditional.consequent, &mut consequent, depth + 1)?;
                consequent.exit = Exit::Yield(value);
                let mut alternate = self.region(&conditional.alternate);
                let value = self.expression(&conditional.alternate, &mut alternate, depth + 1)?;
                alternate.exit = Exit::Yield(value);
                let result = self.value(Type::Number);
                Self::push(
                    region,
                    expression,
                    Op::If {
                        result: Some(result),
                        condition,
                        consequent,
                        alternate,
                    },
                );
                return Ok(result);
            }
            Expression::UnaryExpression(unary) => {
                let operator = match unary.operator {
                    UnaryOperator::UnaryPlus => Unary::Plus,
                    UnaryOperator::UnaryNegation => Unary::Negate,
                    UnaryOperator::BitwiseNot => Unary::BitNot,
                    _ => {
                        return Err(unsupported(
                            expression,
                            "only unary +, - and ~ are admitted",
                        ));
                    }
                };
                let input = if self.unary_boolean_shape(&unary.argument, depth + 1) {
                    let predicate = self.predicate(&unary.argument, region, depth + 1)?;
                    let number = self.value(Type::Number);
                    Self::push(
                        region,
                        &unary.argument,
                        Op::NumberFromPredicate {
                            result: number,
                            input: predicate,
                        },
                    );
                    number
                } else {
                    self.expression(&unary.argument, region, depth + 1)?
                };
                let result = self.value(Type::Number);
                Op::Unary {
                    result,
                    operator,
                    input,
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
                            expression,
                            "only binary +, -, *, /, %, &, |, ^, <<, >> and >>> are admitted",
                        ));
                    }
                };
                let (left, right) = if matches!(
                    operator,
                    Binary::Add
                        | Binary::Subtract
                        | Binary::Multiply
                        | Binary::Divide
                        | Binary::Remainder
                ) {
                    let left = self.arithmetic_operand(&binary.left, region, depth + 1)?;
                    let right = self.arithmetic_operand(&binary.right, region, depth + 1)?;
                    (left, right)
                } else {
                    let left = self.expression(&binary.left, region, depth + 1)?;
                    let right = self.expression(&binary.right, region, depth + 1)?;
                    (left, right)
                };
                let result = self.value(Type::Number);
                Op::Binary {
                    result,
                    operator,
                    left,
                    right,
                }
            }
            _ => {
                return Err(unsupported(
                    expression,
                    "expression is outside the numeric experiment",
                ));
            }
        };
        let result = match &kind {
            Op::Call { result, .. }
            | Op::Constant { result, .. }
            | Op::Load { result, .. }
            | Op::Unary { result, .. }
            | Op::Binary { result, .. } => *result,
            _ => unreachable!("numeric expression operation"),
        };
        Self::push(region, expression, kind);
        Ok(result)
    }

    // Primitive Boolean conversion belongs only to binary arithmetic operands.
    // Preserve the retained node's depth and evaluate it once into a snapshot.
    fn arithmetic_operand(
        &mut self,
        expression: &Expression<'_>,
        region: &mut Region,
        depth: usize,
    ) -> Result<ValueId, String> {
        if boolean_shape(expression, depth) {
            let input = self.predicate(expression, region, depth)?;
            let result = self.value(Type::Number);
            Self::push(
                region,
                expression,
                Op::NumberFromPredicate { result, input },
            );
            Ok(result)
        } else {
            self.expression(expression, region, depth)
        }
    }

    fn predicate(
        &mut self,
        expression: &Expression<'_>,
        region: &mut Region,
        depth: usize,
    ) -> Result<ValueId, String> {
        if depth > 128 {
            return Err(unsupported(
                expression,
                "predicate nesting exceeds this experiment's limit",
            ));
        }
        if let Some(inner) = erased_inner(expression) {
            return self.predicate(inner, region, depth + 1);
        }
        if let Expression::ParenthesizedExpression(parenthesized) = expression {
            return self.predicate(&parenthesized.expression, region, depth + 1);
        }
        if let Expression::BooleanLiteral(literal) = expression {
            let result = self.value(Type::Predicate);
            Self::push(
                region,
                expression,
                Op::Boolean {
                    result,
                    value: literal.value,
                },
            );
            return Ok(result);
        }
        if let Expression::Identifier(identifier) = expression {
            let slot = numeric_binding(identifier, self.semantic, &self.bindings);
            if let Ok(binding) = slot
                && self.slots[binding.slot.0].ty == Type::Predicate
            {
                let slot = binding.slot;
                let result = self.value(Type::Predicate);
                Self::push(region, expression, Op::Load { result, slot });
                return Ok(result);
            }
        }
        if let Expression::UnaryExpression(unary) = expression
            && unary.operator == UnaryOperator::LogicalNot
        {
            let input = self.predicate(&unary.argument, region, depth + 1)?;
            let result = self.value(Type::Predicate);
            Self::push(region, expression, Op::Not { result, input });
            return Ok(result);
        }
        if let Expression::ConditionalExpression(conditional) = expression {
            let condition = self.predicate(&conditional.test, region, depth + 1)?;
            let mut consequent = self.region(&conditional.consequent);
            let left = self.predicate(&conditional.consequent, &mut consequent, depth + 1)?;
            consequent.exit = Exit::Yield(left);
            let mut alternate = self.region(&conditional.alternate);
            let right = self.predicate(&conditional.alternate, &mut alternate, depth + 1)?;
            alternate.exit = Exit::Yield(right);
            let result = self.value(Type::Predicate);
            Self::push(
                region,
                expression,
                Op::If {
                    result: Some(result),
                    condition,
                    consequent,
                    alternate,
                },
            );
            return Ok(result);
        }
        if let Expression::LogicalExpression(logical) = expression {
            if logical.operator == LogicalOperator::Coalesce {
                return Err(unsupported(
                    expression,
                    "expression is outside the numeric experiment",
                ));
            }
            let left = self.predicate(&logical.left, region, depth + 1)?;
            let mut selected = self.region(&logical.right);
            let right = self.predicate(&logical.right, &mut selected, depth + 1)?;
            selected.exit = Exit::Yield(right);
            let mut skipped = self.region(&logical.left);
            skipped.exit = Exit::Yield(left);
            let (consequent, alternate) = if logical.operator == LogicalOperator::And {
                (selected, skipped)
            } else {
                (skipped, selected)
            };
            let result = self.value(Type::Predicate);
            Self::push(
                region,
                expression,
                Op::If {
                    result: Some(result),
                    condition: left,
                    consequent,
                    alternate,
                },
            );
            return Ok(result);
        }
        if let Expression::SequenceExpression(sequence) = expression {
            let (last, preceding) = sequence.expressions.split_last().ok_or_else(|| {
                unsupported(sequence.as_ref(), "comma expression must have an operand")
            })?;
            for operand in preceding {
                self.discarded(operand, region, depth + 1)?;
            }
            return self.predicate(last, region, depth + 1);
        }
        if let Expression::BinaryExpression(binary) = expression {
            let operator = match binary.operator {
                BinaryOperator::LessThan => Some(Compare::Less),
                BinaryOperator::LessEqualThan => Some(Compare::LessEqual),
                BinaryOperator::GreaterThan => Some(Compare::Greater),
                BinaryOperator::GreaterEqualThan => Some(Compare::GreaterEqual),
                BinaryOperator::Equality | BinaryOperator::StrictEquality => Some(Compare::Equal),
                BinaryOperator::Inequality | BinaryOperator::StrictInequality => {
                    Some(Compare::NotEqual)
                }
                _ => None,
            };
            if let Some(operator) = operator {
                let left = self.expression(&binary.left, region, depth + 1)?;
                let right = self.expression(&binary.right, region, depth + 1)?;
                let result = self.value(Type::Predicate);
                Self::push(
                    region,
                    expression,
                    Op::Compare {
                        result,
                        operator,
                        left,
                        right,
                    },
                );
                return Ok(result);
            }
        }
        let input = self.expression(expression, region, depth)?;
        let result = self.value(Type::Predicate);
        Self::push(region, expression, Op::Truthy { result, input });
        Ok(result)
    }

    // Preserve every possible completion; only normal paths reach the suffix.
    pub fn sequence(
        &mut self,
        statements: &[Statement<'_>],
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String> {
        let mut completion = Completion::NORMAL;
        for statement in statements {
            if !completion.normal && !is_type_declaration(statement) {
                return Err(unsupported(
                    statement,
                    if completion == Completion::RETURN {
                        "statements after a guaranteed return are not admitted"
                    } else {
                        "statements after a guaranteed abrupt completion are not admitted"
                    },
                ));
            }
            completion = completion.then(self.statement(statement, region, depth)?);
        }
        Ok(completion)
    }

    pub(super) fn charge_statement(&mut self, statement: &impl GetSpan) -> Result<(), String> {
        if self.statement_count >= 256 {
            return Err(unsupported(
                statement,
                "at most 256 statements including blocks and returns are admitted",
            ));
        }
        self.statement_count += 1;
        Ok(())
    }

    // Inspect retained nodes without emitting or trusting type annotations. Only
    // already-published immutable Predicate bindings can supply an alias.
    fn const_boolean_initializer(&self, mut expression: &Expression<'_>, mut depth: usize) -> bool {
        while depth <= 128 {
            if let Some(inner) = erased_inner(expression) {
                expression = inner;
            } else if let Expression::ParenthesizedExpression(wrapper) = expression {
                expression = &wrapper.expression;
            } else if let Expression::Identifier(identifier) = expression {
                return numeric_binding(identifier, self.semantic, &self.bindings).is_ok_and(
                    |binding| !binding.mutable && self.slots[binding.slot.0].ty == Type::Predicate,
                );
            } else {
                return boolean_shape(expression, depth);
            }
            depth += 1;
        }
        false
    }

    fn declaration(
        &mut self,
        declaration: &VariableDeclaration<'_>,
        region: &mut Region,
    ) -> Result<(), String> {
        if !matches!(
            declaration.kind,
            VariableDeclarationKind::Const | VariableDeclarationKind::Let
        ) || declaration.declare
            || declaration.declarations.is_empty()
        {
            return Err(unsupported(
                declaration,
                "initialized const or let identifiers are required",
            ));
        }
        for local in &declaration.declarations {
            if self.local_count >= 128 {
                return Err(unsupported(local, "at most 128 source locals are admitted"));
            }
            if local.definite {
                return Err(unsupported(
                    local,
                    "definite-assignment assertions are not admitted",
                ));
            }
            let BindingPattern::BindingIdentifier(identifier) = &local.id else {
                return Err(unsupported(local, "local destructuring is not admitted"));
            };
            let initializer = local
                .init
                .as_ref()
                .ok_or("numeric local must have an initializer")?;
            let mutable = declaration.kind == VariableDeclarationKind::Let;
            let ty = if !mutable && self.const_boolean_initializer(initializer, 0) {
                Type::Predicate
            } else {
                Type::Number
            };
            let value = match ty {
                Type::Predicate => self.predicate(initializer, region, 0)?,
                Type::Number => self.expression(initializer, region, 0)?,
            };
            let slot = SlotId(self.slots.len());
            self.slots.push(Slot {
                ty,
                owner: region.id,
                mutable,
                parameter: false,
                span: local.span(),
            });
            self.local_count += 1;
            Self::push(region, local, Op::Initialize { slot, value });
            self.bindings.push(NumericBinding {
                symbol: identifier.symbol_id.get().ok_or("missing local binding")?,
                slot,
                mutable,
            });
        }
        Ok(())
    }

    fn loop_body(
        &mut self,
        statement: &Statement<'_>,
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String> {
        let outer = self.in_loop_body;
        self.in_loop_body = true;
        let result = self.statement(statement, region, depth);
        self.in_loop_body = outer;
        result
    }

    fn statement(
        &mut self,
        statement: &Statement<'_>,
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String> {
        if depth > 64 {
            return Err(unsupported(statement, "statement nesting exceeds 64"));
        }
        self.charge_statement(statement)?;
        if is_type_declaration(statement) {
            return Ok(Completion::NORMAL);
        }
        match statement {
            Statement::ContinueStatement(statement) => {
                if statement.label.is_some() || !self.in_loop_body {
                    return Err(unsupported(
                        statement.as_ref(),
                        "only unlabelled continue inside a loop is admitted",
                    ));
                }
                region.exit = Exit::Continue;
                Ok(Completion::CONTINUE)
            }
            Statement::BreakStatement(statement) => {
                if statement.label.is_some() || !self.in_loop_body {
                    return Err(unsupported(
                        statement.as_ref(),
                        "only unlabelled break inside a loop is admitted",
                    ));
                }
                region.exit = Exit::Break;
                Ok(Completion::BREAK)
            }
            Statement::EmptyStatement(_) => Ok(Completion::NORMAL),
            Statement::DoWhileStatement(statement) => {
                let mut body = self.region(&statement.body);
                let visible = self.bindings.len();
                let returns = self.loop_body(&statement.body, &mut body, depth + 1)?;
                self.bindings.truncate(visible);
                // Admit the whole condition even when a body return skips it.
                let mut condition = self.region(&statement.test);
                let predicate = self.predicate(&statement.test, &mut condition, 0)?;
                condition.exit = Exit::Yield(predicate);
                Self::push(region, statement.as_ref(), Op::DoWhile { body, condition });
                // The body executes at least once; this loop consumes its loop control.
                Ok(returns.do_loop())
            }
            Statement::ForStatement(statement) => {
                // This lexical block and the loop's regions are generated, so
                // they consume neither source statements nor nesting depth.
                let mut header = self.region(statement.as_ref());
                let outer = self.bindings.len();
                if let Some(init) = &statement.init {
                    match init {
                        ForStatementInit::VariableDeclaration(declaration) => {
                            self.charge_statement(declaration.as_ref())?;
                            self.declaration(declaration, &mut header)?;
                        }
                        _ => {
                            self.discarded(init.to_expression(), &mut header, 0)?;
                        }
                    }
                }
                let header_bindings = self.bindings.len();
                let mut condition = self.region(statement.as_ref());
                let predicate = if let Some(test) = &statement.test {
                    condition.span = test.span();
                    self.predicate(test, &mut condition, 0)?
                } else {
                    let one = self.value(Type::Number);
                    Self::push(
                        &mut condition,
                        statement.as_ref(),
                        Op::Constant {
                            result: one,
                            value: 1.0,
                        },
                    );
                    let predicate = self.value(Type::Predicate);
                    Self::push(
                        &mut condition,
                        statement.as_ref(),
                        Op::Truthy {
                            result: predicate,
                            input: one,
                        },
                    );
                    predicate
                };
                condition.exit = Exit::Yield(predicate);
                let mut body = self.region(&statement.body);
                let completion = self.loop_body(&statement.body, &mut body, depth + 1)?;
                self.bindings.truncate(header_bindings);
                // Always admit the update, including after a returning body.
                let step = if let Some(update) = &statement.update {
                    let mut step = self.region(update);
                    self.discarded(update, &mut step, 0)?;
                    Some(step)
                } else {
                    None
                };
                Self::push(
                    &mut header,
                    statement.as_ref(),
                    Op::While {
                        condition,
                        body,
                        step,
                    },
                );
                self.bindings.truncate(outer);
                Self::push(region, statement.as_ref(), Op::Block(header));
                // Preserve the existing conservative structural return policy,
                // even when the source omits its test.
                Ok(completion.pretest_loop())
            }
            Statement::WhileStatement(statement) => {
                let mut condition = self.region(&statement.test);
                let value = self.predicate(&statement.test, &mut condition, 0)?;
                condition.exit = Exit::Yield(value);
                let mut body = self.region(&statement.body);
                let visible = self.bindings.len();
                let completion = self.loop_body(&statement.body, &mut body, depth + 1)?;
                self.bindings.truncate(visible);
                Self::push(
                    region,
                    statement.as_ref(),
                    Op::While {
                        condition,
                        body,
                        step: None,
                    },
                );
                Ok(completion.pretest_loop())
            }
            Statement::IfStatement(statement) => {
                let condition = self.predicate(&statement.test, region, 0)?;
                let mut consequent = self.region(&statement.consequent);
                let visible = self.bindings.len();
                let consequent_returns =
                    self.statement(&statement.consequent, &mut consequent, depth + 1)?;
                self.bindings.truncate(visible);
                let mut alternate = self.region(statement.as_ref());
                let alternate_returns = if let Some(source) = &statement.alternate {
                    alternate.span = source.span();
                    let returns = self.statement(source, &mut alternate, depth + 1)?;
                    self.bindings.truncate(visible);
                    returns
                } else {
                    Completion::NORMAL
                };
                Self::push(
                    region,
                    statement.as_ref(),
                    Op::If {
                        result: None,
                        condition,
                        consequent,
                        alternate,
                    },
                );
                Ok(consequent_returns.union(alternate_returns))
            }
            Statement::BlockStatement(block) => {
                let mut child = self.region(block.as_ref());
                let visible = self.bindings.len();
                let returns = self.sequence(&block.body, &mut child, depth + 1)?;
                self.bindings.truncate(visible);
                Self::push(region, block.as_ref(), Op::Block(child));
                Ok(returns)
            }
            Statement::ReturnStatement(statement) => {
                let expression = statement
                    .argument
                    .as_ref()
                    .ok_or("return must have a numeric expression")?;
                region.exit = Exit::Return(self.expression(expression, region, 0)?);
                Ok(Completion::RETURN)
            }
            Statement::ExpressionStatement(statement) => {
                self.discarded(&statement.expression, region, 0)?;
                Ok(Completion::NORMAL)
            }
            Statement::VariableDeclaration(declaration) => {
                self.declaration(declaration, region)?;
                Ok(Completion::NORMAL)
            }
            _ => Err(unsupported(
                statement,
                "statement is outside the numeric experiment",
            )),
        }
    }

    pub fn finish(self, body: Region, parameter_count: usize) -> ir::Function {
        ir::Function {
            parameter_count,
            slots: self.slots,
            values: self.values,
            body,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn shift_unsigned_is_admitted() {
        crate::compile("export function run(a,b) { return -1 >>> 0; }", false).unwrap();
    }

    #[test]
    fn shift_signed_is_admitted() {
        crate::compile("export function run(a,b) { return -1 >> 0; }", false).unwrap();
    }

    #[test]
    fn shift_left_is_admitted() {
        crate::compile("export function run(a,b) { return 1 << 31; }", false).unwrap();
    }

    #[test]
    fn bitwise_binary_and_is_admitted() {
        crate::compile("export function run(a,b) { return 5 & 3; }", false).unwrap();
    }

    #[test]
    fn bitwise_binary_or_is_admitted() {
        crate::compile("export function run(a,b) { return 5 | 3; }", false).unwrap();
    }

    #[test]
    fn bitwise_binary_xor_is_admitted() {
        crate::compile("export function run(a,b) { return 5 ^ 3; }", false).unwrap();
    }

    #[test]
    fn bit_not_zero_is_admitted() {
        crate::compile("export function run(a,b) { return ~0; }", false).unwrap();
    }

    #[test]
    fn bit_not_negative_one_is_admitted() {
        crate::compile("export function run(a,b) { return ~(-1); }", false).unwrap();
    }

    #[test]
    fn continue_reaches_each_loops_next_iteration_machinery() {
        for body in [
            "let i=0; while(i++<3) { continue; } return i;",
            "let g=0,s=0; for(;g++<3;s++) { continue; } return 10*g+s;",
            "let i=0,t=0; do { i++; if(i<3) continue; } while((t++,0)); return i*10+t;",
        ] {
            crate::compile(&format!("export function run(a,b) {{ {body} }}"), false)
                .unwrap_or_else(|error| panic!("{body}: {error}"));
        }
    }

    #[test]
    fn continue_alternative_survives_a_return_suffix() {
        let error = crate::compile(
            "export function run(a,b) { do { if(a) continue; return b; } while(0); }",
            false,
        )
        .err()
        .expect("a continue path still needs a numeric return after the loop");
        assert!(
            error.contains("every structural path must return"),
            "{error}"
        );
    }

    #[test]
    fn break_exits_each_numeric_loop() {
        for body in [
            "while(a) { break; } return b;",
            "for(;;a=b) { break; } return a;",
            "do { break; } while(a=b); return a;",
        ] {
            crate::compile(&format!("export function run(a,b) {{ {body} }}"), false)
                .unwrap_or_else(|error| panic!("{body}: {error}"));
        }
    }

    #[test]
    fn break_alternatives_survive_a_return_suffix() {
        let error = crate::compile(
            "export function run(a,b) { do { if(a) break; return b; } while(0); }",
            false,
        )
        .err()
        .expect("break path still needs a numeric return");
        assert!(
            error.contains("every structural path must return"),
            "{error}"
        );
    }

    #[test]
    fn reads_unresolved_infinity() {
        let intrinsic =
            crate::compile("export function run(a,b) { return Infinity; }", false).unwrap();
        let literal = crate::compile("export function run(a,b) { return 1e999; }", false).unwrap();
        assert_eq!(intrinsic.wasm, literal.wasm);
    }

    #[test]
    fn reads_unresolved_nan() {
        crate::compile("export function run(a,b) { return NaN; }", false).unwrap();
    }

    #[test]
    fn intrinsic_names_remain_mutable_parameters() {
        for typescript in [false, true] {
            let shadowed = crate::compile(
                "export function run(Infinity,NaN) { Infinity+=NaN; NaN++; return Infinity+NaN; }",
                typescript,
            )
            .unwrap();
            let renamed = crate::compile(
                "export function run(a,b) { a+=b; b++; return a+b; }",
                typescript,
            )
            .unwrap();
            assert_eq!(shadowed.wasm, renamed.wasm);
        }
    }

    #[test]
    fn intrinsic_names_keep_self_and_forward_tdz() {
        for name in ["Infinity", "NaN"] {
            for body in [
                format!("let {name}={name}; return {name};"),
                format!("const x={name}; let {name}=a; return x;"),
                format!("const x=0?{name}:a; let {name}=b; return x;"),
                format!("let x={name},{name}=a; return x;"),
                format!("let x=0?{name}:a,{name}=b; return x;"),
                format!("for(let x={name},{name}=0;0;) {{}} return a;"),
            ] {
                let error =
                    crate::compile(&format!("export function run(a,b) {{ {body} }}"), false)
                        .err()
                        .expect("a resolved TDZ reference must not become a global constant");
                assert!(
                    error.contains("reference is not an initialized numeric parameter or local")
                );
            }
        }
    }

    #[test]
    fn empty_statement_falls_through() {
        let empty = crate::compile("export function run(a,b) { ; return a; }", false).unwrap();
        let plain = crate::compile("export function run(a,b) { return a; }", false).unwrap();
        assert_eq!(empty.wasm, plain.wasm);
    }
    #[test]
    fn empty_loop_body_is_admitted() {
        crate::compile(
            "export function run(a,b) { let i=0; while(i++<3); return a; }",
            false,
        )
        .unwrap();
    }

    #[test]
    fn discarded_numeric_statement() {
        crate::compile("export function run(a,b) { a+(a=b); return a; }", false).unwrap();
    }
    #[test]
    fn discarded_comma_statement() {
        crate::compile(
            "export function run(a,b) { (a=b,b=a+1); return a+b; }",
            false,
        )
        .unwrap();
    }
    #[test]
    fn discarded_wrapped_statement() {
        crate::compile(
            "export function run(a,b) { (a=b) as string; return a; }",
            true,
        )
        .unwrap();
    }

    fn same_numeric_equality(loose: &str, strict: &str) {
        let compile = |expression| {
            crate::compile(
                &format!("export function run(a,b) {{ return {expression}?-0:0; }}"),
                false,
            )
            .unwrap()
            .wasm
        };
        assert_eq!(compile(loose), compile(strict));
    }

    #[test]
    fn numeric_loose_equality_uses_number_equality() {
        same_numeric_equality("a==b", "a===b");
    }
    #[test]
    fn numeric_loose_inequality_uses_number_inequality() {
        same_numeric_equality("a!=b", "a!==b");
    }
    #[test]
    fn numeric_loose_equality_preserves_operand_snapshots() {
        same_numeric_equality("a==(a=b)", "a===(a=b)");
    }

    fn admits_do(body: &str) {
        crate::compile(&format!("export function run(a,b) {{ {body} }}"), false).unwrap();
    }

    #[test]
    fn do_runs_before_zero_test() {
        admits_do("do { a+=b; } while(0); return a;");
    }
    #[test]
    fn do_three_iterations() {
        admits_do("let i=0; do { a+=b; i++; } while(i<3); return a;");
    }
    #[test]
    fn do_return_skips_test_without_fallback() {
        admits_do("do { return a; } while(a=b);");
    }

    fn admits_for(body: &str) {
        crate::compile(&format!("export function run(a,b) {{ {body} }}"), false).unwrap();
    }

    #[test]
    fn for_zero_iterations() {
        admits_for("for (;0;) { a=b; } return a;");
    }
    #[test]
    fn for_three_iterations() {
        admits_for("let x=a; for(let i=0;i<3;i++) { x+=b; } return x;");
    }
    #[test]
    fn for_step_follows_body() {
        admits_for("let x=0; for(let i=0;i<3;i++) { x=x*10+i; } return x;");
    }
    #[test]
    fn for_return_skips_step() {
        admits_for("for(;;a=b) { return a; } return b;");
    }

    fn same_runtime_expression(expression: &str) {
        let js = crate::compile("export function run(a,b) { return a; }", false).unwrap();
        let ts = crate::compile(
            &format!("export function run(a:number,b:number):number {{ return {expression}; }}"),
            true,
        )
        .unwrap();
        assert_eq!(js.wasm, ts.wasm);
    }

    #[test]
    fn erases_as_expression() {
        same_runtime_expression("a as number");
    }
    #[test]
    fn erases_angle_assertion() {
        same_runtime_expression("<number>a");
    }
    #[test]
    fn erases_non_null_expression() {
        same_runtime_expression("a!");
    }
    #[test]
    fn erases_satisfies_expression() {
        same_runtime_expression("a satisfies number");
    }

    fn same_runtime_predicate(predicate: &str) {
        let js =
            crate::compile("export function run(a,b) { return a < b ? -0 : b; }", false).unwrap();
        let ts = crate::compile(
            &format!(
                "export function run(a:number,b:number):number {{ return {predicate} ? -0 : b; }}"
            ),
            true,
        )
        .unwrap();
        assert_eq!(js.wasm, ts.wasm);
    }

    #[test]
    fn erases_as_predicate() {
        same_runtime_predicate("(a < b) as boolean");
    }
    #[test]
    fn erases_angle_predicate() {
        same_runtime_predicate("<boolean>(a < b)");
    }
    #[test]
    fn erases_non_null_predicate() {
        same_runtime_predicate("(a < b)!");
    }
    #[test]
    fn erases_satisfies_predicate() {
        same_runtime_predicate("(a < b) satisfies boolean");
    }
}
