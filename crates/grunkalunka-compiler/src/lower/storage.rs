//! Readonly storage planning over the original semantic AST. Planning never publishes a binding.
use super::*;
use oxc_ast::AstKind;
use oxc_syntax::node::NodeId;
use std::collections::{BTreeMap, BTreeSet};

type CallResults<'a> = BTreeMap<NodeId, Option<Vec<&'a Expression<'a>>>>;

// Storage planning traverses resolved returns while its symbol facts converge.
// Later shape queries use the resulting positive call-site facts, so argument
// specialization and signature checking do not disagree with storage planning.
#[derive(Clone, Copy)]
pub(super) enum NumberCalls<'s, 'a> {
    Proven(&'s BTreeSet<NodeId>),
    Returns(&'s CallResults<'a>),
}

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
        Statement::TryStatement(statement) if statement.finalizer.is_some() => {
            let sequence = |body: &oxc_ast::ast::BlockStatement<'_>| {
                body.body
                    .iter()
                    .fold(Completion::NORMAL, |completion, child| {
                        completion.then(completion_shape(child, depth + 1))
                    })
            };
            let mut completion = sequence(&statement.block);
            if let Some(handler) = &statement.handler {
                completion = completion.caught_by(sequence(&handler.body));
            }
            statement
                .finalizer
                .as_ref()
                .map_or(completion, |finalizer| {
                    completion.finalized_by(sequence(finalizer))
                })
        }
        _ => Completion::NORMAL,
    }
}

pub(super) fn arrow_returns<'n, 'a: 'n>(
    arrow: &'n oxc_ast::ast::ArrowFunctionExpression<'a>,
    semantic: &Semantic<'a>,
) -> Option<Vec<&'n Expression<'a>>> {
    if let Some(body) = arrow.get_expression() {
        return Some(vec![body]);
    }
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
        return None;
    }
    block_results::return_statements(semantic, arrow.node_id.get())
        .into_iter()
        .map(|statement| statement.argument.as_ref())
        .collect()
}

pub(super) fn arrow_number_result_shape<'a>(
    arrow: &oxc_ast::ast::ArrowFunctionExpression<'a>,
    semantic: &Semantic<'a>,
    functions: &[NumericFunction],
    calls: NumberCalls<'_, 'a>,
    mut is_number_call: impl FnMut(SymbolId) -> bool,
    mut is_number_symbol: impl FnMut(SymbolId) -> bool,
) -> bool {
    arrow_returns(arrow, semantic).is_some_and(|expressions| {
        !expressions.is_empty()
            && expressions.into_iter().all(|expression| {
                number_result_shape(
                    expression,
                    0,
                    semantic,
                    functions,
                    calls,
                    &mut is_number_call,
                    &mut is_number_symbol,
                )
            })
    })
}

// Positive Number-result classification for the represented source domain. Actual lowering
// still validates every operand, target, call, scope and source budget.
pub(super) fn number_result_shape<'a>(
    expression: &Expression<'a>,
    depth: usize,
    semantic: &Semantic<'a>,
    functions: &[NumericFunction],
    calls: NumberCalls<'_, 'a>,
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
    let mut expanded = 0;
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
                let Ok(operand) = awaited::operand(awaited, depth, semantic) else {
                    return false;
                };
                let Some(argument) = operand.argument else {
                    return false;
                };
                pending.push((argument, operand.depth));
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
                if let NumberCalls::Proven(known) = calls
                    && known.contains(&call.node_id.get())
                {
                    continue;
                }
                if let NumberCalls::Returns(known) = calls
                    && let Some(results) = known.get(&call.node_id.get())
                {
                    let Some(results) = results else {
                        return false;
                    };
                    expanded += results.len();
                    if expanded > 256 {
                        return false;
                    }
                    pending.extend(results.iter().map(|result| (*result, depth + 1)));
                    continue;
                }
                if let Some(callee) = closures::unwrapped(&call.callee, depth + 1) {
                    if let Expression::ArrowFunctionExpression(arrow) = callee {
                        if let Some(body) = arrow.get_expression() {
                            pending.push((body, depth + 1));
                        } else if arrow.r#async {
                            // A block may complete with a primitive or throw.
                            // Its checked call retains that General completion;
                            // absence of an expression body proves no Number kind.
                            return false;
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

#[derive(Default)]
pub(crate) struct BindingKinds {
    pub(crate) storage: BTreeSet<SymbolId>,
    pub(crate) inputs: BTreeSet<SymbolId>,
    // Source-local computed calls proven Number under all incoming scalar facts.
    pub(crate) number_calls: BTreeSet<NodeId>,
    // Physical scalar result interfaces; source-local Number proofs stay separate.
    pub(crate) general_results: BTreeSet<NodeId>,
    pub(crate) settled_results: BTreeSet<NodeId>,
    pub(crate) async_results: BTreeSet<NodeId>,
}

pub(crate) fn general_mutable_bindings(
    semantic: &Semantic<'_>,
    functions: &[NumericFunction],
    callback_parameters: &super::callbacks::Parameters,
    signatures: &ir::ClosureSignatures,
    promises: &super::promises::Inputs,
) -> BindingKinds {
    // Also retain non-Number facts for immutable definitions: nested signature
    // queries may inspect their captures before those definitions are lowered.
    // Function constructors still use their separate, checked binding path.
    let mut definitions = BTreeMap::new();
    let mut mutable = BTreeSet::new();
    let mut parameters = BTreeSet::new();
    let numeric_callbacks: BTreeSet<_> = callback_parameters
        .values()
        .flat_map(|parameters| parameters.iter())
        .filter_map(|(symbol, signature)| {
            (signatures.result(*signature) == Some(Type::Number)).then_some(*symbol)
        })
        .collect();
    let callback_formals: BTreeSet<_> = callback_parameters
        .values()
        .flat_map(|parameters| parameters.keys().copied())
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
        return BindingKinds::default();
    }
    let mut writes: BTreeMap<SymbolId, Vec<&Expression<'_>>> = BTreeMap::new();
    for node in semantic.nodes().iter() {
        if let AstKind::AssignmentExpression(assignment) = node.kind()
            && matches!(
                assignment.operator,
                AssignmentOperator::Assign
                    | AssignmentOperator::LogicalAnd
                    | AssignmentOperator::LogicalOr
                    | AssignmentOperator::LogicalNullish
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
        // Follow actual constructors through demanded callback formals. Generic
        // identity inputs have no callback role and keep call-site specialization.
        for node in semantic.nodes().iter() {
            let AstKind::CallExpression(call) = node.kind() else {
                continue;
            };
            let (sources, mut bodies) = returns.expression(&call.callee, &BTreeMap::new(), 0, 0);
            for source in sources {
                if let Some(known) = targets.get(&source) {
                    bodies.extend(known);
                }
            }
            for body in bodies {
                for (parameter, argument) in arrows[&body].params.items.iter().zip(&call.arguments)
                {
                    let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern else {
                        continue;
                    };
                    let (Some(symbol), Some(actual)) =
                        (identifier.symbol_id.get(), argument.as_expression())
                    else {
                        continue;
                    };
                    if !callback_formals.contains(&symbol) {
                        continue;
                    }
                    let (sources, mut constructors) =
                        returns.expression(actual, &BTreeMap::new(), 0, 0);
                    for source in sources {
                        if let Some(known) = targets.get(&source) {
                            constructors.extend(known);
                        }
                    }
                    additions.extend(constructors.into_iter().map(|body| (symbol, body)));
                }
            }
        }
        let mut changed = false;
        for (symbol, body) in additions {
            changed |= targets.entry(symbol).or_default().insert(body);
        }
        if !changed {
            break;
        }
    }
    // Known local calls specialize their actuals without using the private ABI.
    // Match immutable arrows/aliases and concise known factory results, rather
    // than widening a shared private signature because of an inlined call.
    let static_constructor = |expression: &Expression<'_>, known: &BTreeMap<SymbolId, NodeId>| {
        match closures::unwrapped(expression, 0)? {
            Expression::ArrowFunctionExpression(arrow) => Some(arrow.node_id.get()),
            Expression::Identifier(identifier) => identifier
                .reference_id
                .get()
                .and_then(|reference| semantic.scoping().get_reference(reference).symbol_id())
                .and_then(|symbol| known.get(&symbol).copied()),
            _ => None,
        }
    };
    let mut static_targets = BTreeMap::new();
    loop {
        let mut additions = Vec::new();
        for (&symbol, initializer) in &definitions {
            if mutable.contains(&symbol) {
                continue;
            }
            let Some(initializer) = initializer else {
                continue;
            };
            let target = static_constructor(initializer, &static_targets).or_else(|| {
                let Expression::CallExpression(call) = closures::unwrapped(initializer, 0)? else {
                    return None;
                };
                let creator = static_constructor(&call.callee, &static_targets)?;
                static_constructor(arrows[&creator].get_expression()?, &static_targets)
            });
            if let Some(target) = target {
                additions.push((symbol, target));
            }
        }
        let mut changed = false;
        for (symbol, target) in additions {
            changed |= static_targets.insert(symbol, target) != Some(target);
        }
        if !changed {
            break;
        }
    }
    // Known scalar actuals are initial writes to the arrow's formal cells.
    // Function inputs retain their separate call-site specialization instead of
    // turning a generic identity formal into one monomorphic scalar kind.
    let returns = callbacks::ReturnOrigins {
        semantic,
        targets: &targets,
        arrows: &arrows,
    };
    let mut call_results = BTreeMap::new();
    for node in semantic.nodes().iter() {
        let AstKind::CallExpression(call) = node.kind() else {
            continue;
        };
        if matches!(
            closures::unwrapped(&call.callee, 0),
            Some(Expression::Identifier(_) | Expression::ArrowFunctionExpression(_))
        ) || math_unary(call, semantic).is_some()
        {
            continue;
        }
        let (sources, mut bodies) = returns.expression(&call.callee, &BTreeMap::new(), 0, 0);
        for source in sources {
            if let Some(known) = targets.get(&source) {
                bodies.extend(known);
            }
        }
        if bodies.is_empty() {
            continue;
        }
        let results: Option<Vec<_>> =
            bodies
                .into_iter()
                .try_fold(Vec::new(), |mut results, body| {
                    results.extend(arrow_returns(arrows[&body], semantic)?);
                    Some(results)
                });
        call_results.insert(call.node_id.get(), results);
    }
    // Resolve computed callees before admitting their result as an input fact.
    // Unknown callees still supply no scalar-kind evidence.
    let awaited_intrinsics: BTreeSet<_> = semantic
        .nodes()
        .iter()
        .filter_map(|node| {
            let AstKind::AwaitExpression(awaited) = node.kind() else {
                return None;
            };
            let Expression::CallExpression(call) = closures::unwrapped(&awaited.argument, 0)?
            else {
                return None;
            };
            let operand = awaited::operand(awaited, 0, semantic).ok()?;
            // Only a validated Promise.resolve/reject unwraps the call to its
            // payload. An ordinary awaited call retains its complete span.
            operand
                .argument
                .is_none_or(|payload| payload.span() != call.span())
                .then_some(call.node_id.get())
        })
        .collect();
    let opaque_calls: Vec<_> = semantic
        .nodes()
        .iter()
        .filter_map(|node| {
            let AstKind::CallExpression(call) = node.kind() else {
                return None;
            };
            let known = call_results.contains_key(&call.node_id.get());
            let fulfilled = awaited::intrinsic(call, semantic)
                .ok()
                .flatten()
                .is_some_and(|intrinsic| !intrinsic.rejected);
            (!known
                && !fulfilled
                && !awaited_intrinsics.contains(&call.node_id.get())
                && !matches!(
                    closures::unwrapped(&call.callee, 0),
                    Some(Expression::Identifier(_) | Expression::ArrowFunctionExpression(_))
                )
                && math_unary(call, semantic).is_none())
            .then_some(call.span())
        })
        .collect();
    let mut incoming = Vec::new();
    let mut constructor_groups: BTreeSet<_> = targets
        .values()
        .filter(|bodies| bodies.len() > 1)
        .cloned()
        .collect();
    for node in semantic.nodes().iter() {
        let AstKind::CallExpression(call) = node.kind() else {
            continue;
        };
        let runtime_input = static_constructor(&call.callee, &static_targets)
            .is_none_or(|body| arrows[&body].r#async || promises.results.contains(&body));
        let (sources, mut bodies) = returns.expression(&call.callee, &BTreeMap::new(), 0, 0);
        for source in sources {
            if let Some(known) = targets.get(&source) {
                bodies.extend(known);
            }
        }
        if bodies.len() > 1 {
            constructor_groups.insert(bodies.clone());
        }
        for body in bodies {
            for (parameter, argument) in arrows[&body].params.items.iter().zip(&call.arguments) {
                let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern else {
                    continue;
                };
                let (Some(symbol), Some(actual)) =
                    (identifier.symbol_id.get(), argument.as_expression())
                else {
                    continue;
                };
                let (sources, constructors) = returns.expression(actual, &BTreeMap::new(), 0, 0);
                if !constructors.is_empty()
                    || sources.iter().any(|source| {
                        targets.contains_key(source)
                            || callback_parameters
                                .values()
                                .any(|parameters| parameters.contains_key(source))
                    })
                {
                    continue;
                }
                let span = actual.span();
                if opaque_calls
                    .iter()
                    .any(|call| span.start <= call.start && call.end <= span.end)
                {
                    continue;
                }
                writes.entry(symbol).or_default().push(actual);
                if runtime_input {
                    incoming.push((symbol, actual));
                }
            }
        }
    }
    // Each factory selects its returned constructors, even without a source
    // call or peer constructor. Follow complete returns within the eight-frame
    // signature-query bound; retain singleton frontiers because the next layer
    // can contain a conditional selection again. This query executes no body.
    let mut frontier = constructor_groups.clone();
    frontier.extend(arrows.keys().map(|body| BTreeSet::from([*body])));
    for _ in 0..8 {
        let mut next = BTreeSet::new();
        for group in &frontier {
            let mut returned = BTreeSet::new();
            for body in group {
                if let Some(expressions) = arrow_returns(arrows[body], semantic) {
                    for expression in expressions {
                        let (sources, constructors) =
                            returns.expression(expression, &BTreeMap::new(), 0, 0);
                        returned.extend(constructors);
                        for source in sources {
                            if let Some(known) = targets.get(&source) {
                                returned.extend(known);
                            }
                        }
                    }
                }
            }
            if returned.len() > 1 {
                constructor_groups.insert(returned.clone());
            }
            if !returned.is_empty() {
                next.insert(returned);
            }
        }
        if next == frontier {
            break;
        }
        frontier = next;
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
    // A General-result callback cannot supply a Number proof to its callers.
    let mut invalid: BTreeSet<_> = callback_parameters
        .values()
        .flat_map(|parameters| parameters.keys())
        .filter(|symbol| !numeric_callbacks.contains(symbol))
        .copied()
        .collect();
    for symbol in &candidates {
        let required = std::cell::RefCell::new(BTreeSet::new());
        let shape = |expression| {
            number_result_shape(
                expression,
                0,
                semantic,
                functions,
                NumberCalls::Returns(&call_results),
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
                    NumberCalls::Returns(&call_results),
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
    let mut inputs = BTreeSet::new();
    loop {
        while let Some(input) = pending.pop() {
            if numbers.remove(&input)
                && let Some(readers) = dependents.get(&input)
            {
                pending.extend(readers.iter().copied());
            }
        }
        let previous_inputs = inputs.len();
        // Runtime actuals choose the private input ABI; ordinary source writes
        // and known inlined calls still choose only cell/result storage.
        inputs.extend(incoming.iter().filter_map(|(symbol, actual)| {
            (!number_result_shape(
                actual,
                0,
                semantic,
                functions,
                NumberCalls::Returns(&call_results),
                |callee| {
                    numbers.contains(&callee)
                        && (targets.contains_key(&callee) || numeric_callbacks.contains(&callee))
                },
                |input| numbers.contains(&input),
            ))
            .then_some(*symbol)
        }));
        // Constructors sharing a value need one scalar entry interface. Keep
        // callback parameters on their separate signature path.
        for bodies in &constructor_groups {
            let count = arrows[bodies.first().unwrap()].params.items.len();
            if bodies
                .iter()
                .any(|body| arrows[body].params.items.len() != count)
            {
                continue;
            }
            for index in 0..count {
                let group: BTreeSet<_> = bodies
                    .iter()
                    .filter_map(|body| {
                        let BindingPattern::BindingIdentifier(identifier) =
                            &arrows[body].params.items[index].pattern
                        else {
                            return None;
                        };
                        identifier
                            .symbol_id
                            .get()
                            .filter(|symbol| !callback_formals.contains(symbol))
                    })
                    .collect();
                if group.len() == bodies.len() && !group.is_disjoint(&inputs) {
                    inputs.extend(group);
                }
            }
        }
        if inputs.len() == previous_inputs {
            break;
        }
        // A joined General entry is a General formal cell too. Propagate its
        // result dependencies before checking downstream actuals again.
        pending.extend(inputs.iter().copied());
    }
    let storage: BTreeSet<_> = candidates.difference(&numbers).copied().collect();
    let number_calls = call_results
        .iter()
        .filter_map(|(id, results)| {
            results
                .as_ref()
                .is_some_and(|results| {
                    results.iter().all(|result| {
                        number_result_shape(
                            result,
                            0,
                            semantic,
                            functions,
                            NumberCalls::Returns(&call_results),
                            |callee| {
                                !storage.contains(&callee)
                                    && (targets.contains_key(&callee)
                                        || numeric_callbacks.contains(&callee))
                            },
                            |input| numbers.contains(&input),
                        )
                    })
                })
                .then_some(*id)
        })
        .collect();
    let mut general_results = BTreeSet::new();
    loop {
        let previous = general_results.len();
        for bodies in &constructor_groups {
            if bodies.iter().any(|body| {
                general_results.contains(body)
                    || !arrow_number_result_shape(
                        arrows[body],
                        semantic,
                        functions,
                        NumberCalls::Proven(&number_calls),
                        |callee| !storage.contains(&callee),
                        |input| numbers.contains(&input) && !callback_formals.contains(&input),
                    )
            }) {
                general_results.extend(bodies);
            }
        }
        if previous == general_results.len() {
            break;
        }
    }
    let mut settled_results: BTreeSet<_> = arrows
        .iter()
        .filter_map(|(id, arrow)| {
            (awaited::immediate_async_arrow(arrow, semantic)
                && awaited::owns_throw(arrow, semantic))
            .then_some(*id)
        })
        .collect();
    loop {
        let previous = settled_results.len();
        for bodies in &constructor_groups {
            if !bodies.is_disjoint(&settled_results) {
                settled_results.extend(bodies);
            }
        }
        if settled_results.len() == previous {
            break;
        }
    }
    let mut async_results: BTreeSet<_> = arrows
        .iter()
        .filter_map(|(id, arrow)| {
            (arrow.r#async && !awaited::immediate_async_arrow(arrow, semantic)).then_some(*id)
        })
        .collect();
    loop {
        let previous = async_results.len();
        for bodies in &constructor_groups {
            if !bodies.is_disjoint(&async_results) {
                async_results.extend(bodies.iter().filter(|id| arrows[id].r#async));
            }
        }
        if async_results.len() == previous {
            break;
        }
    }
    BindingKinds {
        storage,
        inputs,
        number_calls,
        general_results,
        settled_results,
        async_results,
    }
}
