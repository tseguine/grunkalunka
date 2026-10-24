//! Readonly storage planning over the original semantic AST. Planning never publishes a binding.
use super::*;
use oxc_ast::AstKind;
use std::collections::{BTreeMap, BTreeSet};

// Positive Number-result classification for the represented source domain. Actual lowering
// still validates every operand, target, call, scope and source budget.
pub(super) fn number_result_shape(
    expression: &Expression<'_>,
    depth: usize,
    semantic: &Semantic<'_>,
    functions: &[NumericFunction],
    is_number_symbol: impl Fn(SymbolId) -> bool,
) -> bool {
    let identifier_number = |identifier: &IdentifierReference<'_>| {
        let Some(reference) = identifier.reference_id.get() else {
            return false;
        };
        match semantic.scoping().get_reference(reference).symbol_id() {
            Some(symbol) => is_number_symbol(symbol),
            None => matches!(identifier.name.as_str(), "Infinity" | "NaN"),
        }
    };

    let mut pending = vec![(expression, depth)];
    while let Some((node, depth)) = pending.pop() {
        if depth > 128 {
            return false;
        }
        if let Some(inner) = erased_inner(node) {
            pending.push((inner, depth + 1));
            continue;
        }
        match node {
            Expression::ParenthesizedExpression(wrapper) => {
                pending.push((&wrapper.expression, depth + 1))
            }
            Expression::ConditionalExpression(conditional) => {
                pending.push((&conditional.alternate, depth + 1));
                pending.push((&conditional.consequent, depth + 1));
            }
            Expression::SequenceExpression(sequence) => {
                let Some(last) = sequence.expressions.last() else {
                    return false;
                };
                pending.push((last, depth + 1));
            }
            Expression::LogicalExpression(logical)
                if matches!(logical.operator, LogicalOperator::And | LogicalOperator::Or) =>
            {
                pending.push((&logical.right, depth + 1));
                pending.push((&logical.left, depth + 1));
            }
            Expression::NumericLiteral(_) | Expression::UpdateExpression(_) => (),
            Expression::AssignmentExpression(assignment) => match assignment.operator {
                AssignmentOperator::Assign => pending.push((&assignment.right, depth + 1)),
                AssignmentOperator::LogicalAnd
                | AssignmentOperator::LogicalOr
                | AssignmentOperator::LogicalNullish => {
                    let Some(identifier) = assignment
                        .left
                        .as_simple_assignment_target()
                        .and_then(|target| target_identifier(target, depth).ok().flatten())
                    else {
                        return false;
                    };
                    if !identifier_number(identifier) {
                        return false;
                    }
                    pending.push((&assignment.right, depth + 1));
                }
                _ => (),
            },
            Expression::UnaryExpression(unary)
                if matches!(
                    unary.operator,
                    UnaryOperator::UnaryPlus
                        | UnaryOperator::UnaryNegation
                        | UnaryOperator::BitwiseNot
                ) => {}
            Expression::BinaryExpression(binary)
                if matches!(
                    binary.operator,
                    BinaryOperator::Addition
                        | BinaryOperator::Subtraction
                        | BinaryOperator::Multiplication
                        | BinaryOperator::Division
                        | BinaryOperator::Remainder
                        | BinaryOperator::BitwiseAnd
                        | BinaryOperator::BitwiseOR
                        | BinaryOperator::BitwiseXOR
                        | BinaryOperator::ShiftLeft
                        | BinaryOperator::ShiftRight
                        | BinaryOperator::ShiftRightZeroFill
                ) => {}
            Expression::Identifier(identifier) => {
                if !identifier_number(identifier) {
                    return false;
                }
            }
            Expression::CallExpression(call) => {
                if math_unary(call, semantic).is_none() {
                    let Expression::Identifier(identifier) = &call.callee else {
                        return false;
                    };
                    let symbol = identifier.reference_id.get().and_then(|reference| {
                        semantic.scoping().get_reference(reference).symbol_id()
                    });
                    if !functions.iter().any(|function| {
                        Some(function.symbol) == symbol && function.return_type == Type::Number
                    }) {
                        return false;
                    }
                }
            }
            _ => return false,
        }
    }
    true
}

pub(crate) fn general_mutable_locals(
    semantic: &Semantic<'_>,
    functions: &[NumericFunction],
) -> BTreeSet<SymbolId> {
    let mut definitions = BTreeMap::new();
    let mut mutable = BTreeSet::new();
    let mut parameters = BTreeSet::new();
    for node in semantic.nodes().iter() {
        match node.kind() {
            AstKind::VariableDeclaration(declaration)
                if matches!(
                    declaration.kind,
                    VariableDeclarationKind::Let | VariableDeclarationKind::Const
                ) =>
            {
                for local in &declaration.declarations {
                    if let BindingPattern::BindingIdentifier(identifier) = &local.id
                        && let Some(symbol) = identifier.symbol_id.get()
                    {
                        definitions.insert(symbol, local.init.as_ref());
                        if declaration.kind == VariableDeclarationKind::Let {
                            mutable.insert(symbol);
                        }
                    }
                }
            }
            AstKind::FormalParameter(parameter) => {
                if let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern
                    && let Some(symbol) = identifier.symbol_id.get()
                {
                    parameters.insert(symbol);
                }
            }
            _ => (),
        }
    }
    if mutable.is_empty() || definitions.len() > 128 {
        // Invalid oversized modules retain ordinary lowering's source-local diagnostics.
        return BTreeSet::new();
    }
    let mut writes: BTreeMap<SymbolId, Vec<&Expression<'_>>> = BTreeMap::new();
    for node in semantic.nodes().iter() {
        if let AstKind::AssignmentExpression(assignment) = node.kind()
            && assignment.operator == AssignmentOperator::Assign
            && let Some(identifier) = assignment
                .left
                .as_simple_assignment_target()
                .and_then(|target| target_identifier(target, 0).ok().flatten())
            && let Some(symbol) = identifier
                .reference_id
                .get()
                .and_then(|reference| semantic.scoping().get_reference(reference).symbol_id())
            && mutable.contains(&symbol)
        {
            writes.entry(symbol).or_default().push(&assignment.right);
        }
    }
    let mut numbers = definitions.keys().copied().collect::<BTreeSet<_>>();
    loop {
        let is_number = |symbol| parameters.contains(&symbol) || numbers.contains(&symbol);
        let removed = definitions
            .iter()
            .filter_map(|(symbol, initializer)| {
                if !numbers.contains(symbol) {
                    return None;
                }
                let initialized_number = initializer.is_some_and(|expression| {
                    number_result_shape(expression, 0, semantic, functions, is_number)
                });
                let writes_number = writes.get(symbol).is_none_or(|values| {
                    values.iter().all(|expression| {
                        number_result_shape(expression, 0, semantic, functions, is_number)
                    })
                });
                (!initialized_number || !writes_number).then_some(*symbol)
            })
            .collect::<Vec<_>>();
        if removed.is_empty() {
            break;
        }
        for symbol in removed {
            numbers.remove(&symbol);
        }
    }
    mutable.difference(&numbers).copied().collect()
}
