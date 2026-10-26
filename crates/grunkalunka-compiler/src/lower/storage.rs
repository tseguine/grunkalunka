//! Readonly storage planning over the original semantic AST. Planning never publishes a binding.
use super::*;
use oxc_ast::AstKind;
use std::collections::{BTreeMap, BTreeSet};

// The same completion algebra as statement lowering, without evaluating source
// expressions or looking inside nested functions. Unsupported statements are
// still rejected by lowering; this query only proves complete Number returns.
fn completion_shape(statement: &Statement<'_>, depth: usize) -> Completion {
    if depth > 128 {
        return Completion::NORMAL;
    }
    match statement {
        Statement::ReturnStatement(_) => Completion::RETURN,
        Statement::ThrowStatement(_) => Completion::THROW,
        Statement::BreakStatement(_) => Completion::BREAK,
        Statement::ContinueStatement(_) => Completion::CONTINUE,
        Statement::BlockStatement(block) => block
            .body
            .iter()
            .fold(Completion::NORMAL, |completion, statement| {
                completion.then(completion_shape(statement, depth + 1))
            }),
        Statement::IfStatement(branch) => completion_shape(&branch.consequent, depth + 1).union(
            branch
                .alternate
                .as_ref()
                .map_or(Completion::NORMAL, |statement| {
                    completion_shape(statement, depth + 1)
                }),
        ),
        Statement::WhileStatement(loop_) => completion_shape(&loop_.body, depth + 1).pretest_loop(),
        Statement::ForStatement(loop_) => completion_shape(&loop_.body, depth + 1).pretest_loop(),
        Statement::DoWhileStatement(loop_) => completion_shape(&loop_.body, depth + 1).do_loop(),
        _ => Completion::NORMAL,
    }
}

pub(super) fn arrow_number_result_shape(
    arrow: &oxc_ast::ast::ArrowFunctionExpression<'_>,
    semantic: &Semantic<'_>,
    functions: &[NumericFunction],
    mut is_number_call: impl FnMut(SymbolId) -> bool,
    mut is_number_symbol: impl FnMut(SymbolId) -> bool,
) -> bool {
    let expressions = if let Some(body) = arrow.get_expression() {
        vec![Some(body)]
    } else {
        let oxc_ast::ast::ArrowFunctionBody::FunctionBody(body) = &arrow.body else {
            unreachable!("block arrow body")
        };
        if body
            .statements
            .iter()
            .fold(Completion::NORMAL, |completion, statement| {
                completion.then(completion_shape(statement, 0))
            })
            != Completion::RETURN
        {
            return false;
        }
        block_results::return_statements(semantic, arrow.node_id.get())
            .into_iter()
            .map(|statement| statement.argument.as_ref())
            .collect()
    };
    !expressions.is_empty()
        && expressions.into_iter().all(|expression| {
            expression.is_some_and(|expression| {
                number_result_shape(
                    expression,
                    0,
                    semantic,
                    functions,
                    &mut is_number_call,
                    &mut is_number_symbol,
                )
            })
        })
}

// Positive Number-result classification for the represented source domain. Actual lowering
// still validates every operand, target, call, scope and source budget.
pub(super) fn number_result_shape(
    expression: &Expression<'_>,
    depth: usize,
    semantic: &Semantic<'_>,
    functions: &[NumericFunction],
    mut is_number_call: impl FnMut(SymbolId) -> bool,
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
            Expression::LogicalExpression(logical) => {
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
                    if let Expression::ArrowFunctionExpression(arrow) = callee {
                        if let Some(body) = arrow.get_expression() {
                            pending.push((body, depth + 1));
                        }
                        continue;
                    }
                    if let Expression::Identifier(identifier) = callee
                        && identifier
                            .reference_id
                            .get()
                            .and_then(|reference| {
                                semantic.scoping().get_reference(reference).symbol_id()
                            })
                            .is_some_and(&mut is_number_call)
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
    // Also retain non-Number facts for immutable definitions: nested signature
    // queries may inspect their captures before those definitions are lowered.
    // Function constructors still use their separate, checked binding path.
    let mut definitions = BTreeMap::new();
    let mut mutable = BTreeSet::new();
    let mut parameters = BTreeSet::new();
    let numeric_callbacks: BTreeSet<_> = callback_parameters
        .values()
        .flat_map(|parameters| parameters.keys())
        .copied()
        .collect();
    let mut arrows = BTreeMap::new();
    for node in semantic.nodes().iter() {
        match node.kind() {
            AstKind::ArrowFunctionExpression(arrow) => {
                arrows.insert(arrow.node_id.get(), arrow);
            }

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
    // Resolve the constructors of a function value separately from the result
    // of calling its creator. Reuse callback-origin analysis, including its
    // bounded factory-call and parameter scopes.
    let mut targets: BTreeMap<SymbolId, BTreeSet<oxc_syntax::node::NodeId>> = BTreeMap::new();
    loop {
        let returns = callbacks::ReturnOrigins {
            semantic,
            targets: &targets,
            arrows: &arrows,
        };
        let mut additions = Vec::new();
        for (symbol, expression) in definitions
            .iter()
            .filter_map(|(symbol, init)| init.map(|init| (*symbol, init)))
            .chain(
                writes
                    .iter()
                    .flat_map(|(symbol, values)| values.iter().map(move |value| (*symbol, *value))),
            )
        {
            let (sources, mut bodies) = returns.expression(expression, &BTreeMap::new(), 0, 0);
            for source in sources {
                if let Some(known) = targets.get(&source) {
                    bodies.extend(known);
                }
            }
            additions.extend(bodies.into_iter().map(|body| (symbol, body)));
        }
        let mut changed = false;
        for (symbol, body) in additions {
            changed |= targets.entry(symbol).or_default().insert(body);
        }
        if !changed {
            break;
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
        let required = std::cell::RefCell::new(BTreeSet::new());
        let shape = |expression| {
            number_result_shape(
                expression,
                0,
                semantic,
                functions,
                |callee| {
                    if !targets.contains_key(&callee) && !numeric_callbacks.contains(&callee) {
                        return false;
                    }
                    required.borrow_mut().insert(callee);
                    true
                },
                |input| {
                    required.borrow_mut().insert(input);
                    true
                },
            )
        };
        let initialized_number = if let Some(bodies) = targets.get(symbol) {
            bodies.iter().all(|body| {
                arrow_number_result_shape(
                    arrows[body],
                    semantic,
                    functions,
                    |callee| {
                        if !targets.contains_key(&callee) && !numeric_callbacks.contains(&callee) {
                            return false;
                        }
                        required.borrow_mut().insert(callee);
                        true
                    },
                    |input| {
                        required.borrow_mut().insert(input);
                        true
                    },
                )
            })
        } else {
            parameters.contains(symbol)
                || definitions
                    .get(symbol)
                    .and_then(|value| *value)
                    .is_some_and(shape)
        };
        let writes_number = targets.contains_key(symbol)
            || writes
                .get(symbol)
                .is_none_or(|values| values.iter().copied().all(shape));
        if !initialized_number || !writes_number {
            invalid.insert(*symbol);
        }
        for input in required.into_inner() {
            if candidates.contains(&input) {
                dependents.entry(input).or_default().insert(*symbol);
            } else {
                invalid.insert(*symbol);
            }
        }
    }
    let mut numbers = candidates.clone();
    let mut pending = invalid.into_iter().collect::<Vec<_>>();
    while let Some(input) = pending.pop() {
        if numbers.remove(&input)
            && let Some(readers) = dependents.get(&input)
        {
            pending.extend(readers.iter().copied());
        }
    }
    candidates.difference(&numbers).copied().collect()
}
