//! Readonly storage planning over the original semantic AST. Planning never publishes a binding.
use super::*;
use oxc_ast::AstKind;
use std::collections::{BTreeMap, BTreeSet};

// Only a call-result storage hint: all admitted local function calls currently
// return Number. Actual lowering validates constructors, factories and callees.
// A scalar result tentatively included here still cannot become a callable value.
fn closure_hint(
    expression: &Expression<'_>,
    semantic: &Semantic<'_>,
    known: &BTreeSet<SymbolId>,
) -> bool {
    let mut pending = vec![(expression, 0)];
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
                pending.push((&conditional.consequent, depth + 1));
                pending.push((&conditional.alternate, depth + 1));
            }
            Expression::SequenceExpression(sequence) => {
                let Some(last) = sequence.expressions.last() else {
                    return false;
                };
                pending.push((last, depth + 1));
            }
            Expression::AssignmentExpression(assignment)
                if assignment.operator == AssignmentOperator::Assign =>
            {
                pending.push((&assignment.right, depth + 1));
            }
            Expression::ArrowFunctionExpression(_) => (),
            Expression::CallExpression(call) => pending.push((&call.callee, depth + 1)),
            Expression::Identifier(identifier) => {
                if !identifier
                    .reference_id
                    .get()
                    .and_then(|reference| semantic.scoping().get_reference(reference).symbol_id())
                    .is_some_and(|symbol| known.contains(&symbol))
                {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

// Positive Number-result classification for the represented source domain. Actual lowering
// still validates every operand, target, call, scope and source budget.
pub(super) fn number_result_shape(
    expression: &Expression<'_>,
    depth: usize,
    semantic: &Semantic<'_>,
    functions: &[NumericFunction],
    numeric_closures: &BTreeSet<SymbolId>,
    mut is_number_symbol: impl FnMut(SymbolId) -> bool,
) -> bool {
    let mut identifier_number = |identifier: &IdentifierReference<'_>| {
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
            Expression::AwaitExpression(awaited) => {
                let Ok((argument, depth, _)) = awaited::operand(awaited, depth, semantic) else {
                    return false;
                };
                pending.push((argument, depth));
            }
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
                if let Some(callee) = closures::unwrapped(&call.callee, depth + 1) {
                    if matches!(callee, Expression::ArrowFunctionExpression(_)) {
                        continue;
                    }
                    if let Expression::Identifier(identifier) = callee
                        && identifier
                            .reference_id
                            .get()
                            .and_then(|reference| {
                                semantic.scoping().get_reference(reference).symbol_id()
                            })
                            .is_some_and(|symbol| numeric_closures.contains(&symbol))
                    {
                        continue;
                    }
                }
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

pub(crate) fn general_mutable_bindings(
    semantic: &Semantic<'_>,
    functions: &[NumericFunction],
    callback_parameters: &super::callbacks::Parameters,
) -> BTreeSet<SymbolId> {
    let mut definitions = BTreeMap::new();
    let mut mutable = BTreeSet::new();
    let mut parameters = BTreeSet::new();
    // Local arrow calls tentatively seed Number storage. This is only a planning
    // hint: actual lowering fixes the function arity and validates every write;
    // factories/scalar values still cannot become callable via this hint.
    let mut numeric_closures = BTreeSet::new();
    numeric_closures.extend(
        callback_parameters
            .values()
            .flat_map(|parameters| parameters.keys())
            .copied(),
    );
    let mut closure_aliases = Vec::new();
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
                        if let Some(initializer) = local
                            .init
                            .as_ref()
                            .and_then(|init| closures::unwrapped(init, 0))
                        {
                            match initializer {
                                Expression::ArrowFunctionExpression(_) => {
                                    numeric_closures.insert(symbol);
                                }
                                Expression::Identifier(identifier) => {
                                    if let Some(target) =
                                        identifier.reference_id.get().and_then(|reference| {
                                            semantic.scoping().get_reference(reference).symbol_id()
                                        })
                                    {
                                        closure_aliases.push((symbol, target));
                                    }
                                }
                                _ => (),
                            }
                        }
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
                    mutable.insert(symbol);
                }
            }
            _ => (),
        }
    }
    if mutable.is_empty() || definitions.len() > 128 {
        // Invalid oversized modules retain ordinary lowering's source-local diagnostics.
        return BTreeSet::new();
    }
    loop {
        let before = numeric_closures.len();
        for (symbol, initializer) in &definitions {
            if initializer
                .is_some_and(|expression| closure_hint(expression, semantic, &numeric_closures))
            {
                numeric_closures.insert(*symbol);
            }
        }
        for &(alias, target) in &closure_aliases {
            if numeric_closures.contains(&target) {
                numeric_closures.insert(alias);
            }
        }
        if numeric_closures.len() == before {
            break;
        }
    }
    let mut writes: BTreeMap<SymbolId, Vec<&Expression<'_>>> = BTreeMap::new();
    for node in semantic.nodes().iter() {
        if let AstKind::AssignmentExpression(assignment) = node.kind()
            && matches!(
                assignment.operator,
                AssignmentOperator::Assign
                    | AssignmentOperator::LogicalAnd
                    | AssignmentOperator::LogicalOr
            )
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
    // A candidate stays Number exactly when its initial value and every plain/logical RHS write
    // have Number shape and all symbol dependencies stay Number. Start with the
    // greatest candidate set, then propagate removals through reverse edges once.
    let candidates = definitions
        .keys()
        .chain(parameters.iter())
        .copied()
        .collect::<BTreeSet<_>>();
    let mut dependents: BTreeMap<SymbolId, BTreeSet<SymbolId>> = BTreeMap::new();
    let mut invalid = BTreeSet::new();
    for symbol in &candidates {
        let mut required = BTreeSet::new();
        let mut shape = |expression| {
            number_result_shape(
                expression,
                0,
                semantic,
                functions,
                &numeric_closures,
                |input| {
                    required.insert(input);
                    true
                },
            )
        };
        let initialized_number = parameters.contains(symbol)
            || definitions
                .get(symbol)
                .and_then(|value| *value)
                .is_some_and(&mut shape);
        let writes_number = writes
            .get(symbol)
            .is_none_or(|values| values.iter().all(|expression| shape(expression)));
        if !initialized_number || !writes_number {
            invalid.insert(*symbol);
        }
        for input in required {
            if candidates.contains(&input) {
                dependents.entry(input).or_default().insert(*symbol);
            } else {
                invalid.insert(*symbol);
            }
        }
    }
    let mut numbers = candidates;
    let mut pending = invalid.into_iter().collect::<Vec<_>>();
    while let Some(input) = pending.pop() {
        if numbers.remove(&input)
            && let Some(readers) = dependents.get(&input)
        {
            pending.extend(readers.iter().copied());
        }
    }
    mutable.difference(&numbers).copied().collect()
}
