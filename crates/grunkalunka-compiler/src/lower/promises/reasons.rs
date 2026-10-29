//! Lexical destinations of represented raw rejection reasons.
use super::*;

#[derive(Default)]
pub(super) struct Routes {
    pub handlers: BTreeSet<u32>,
    pub finalizers: BTreeSet<u32>,
}

// A source exception reaches this arrow only if no inner protected catch owns it.
fn escapes_arrow(node: NodeId, body: NodeId, semantic: &Semantic<'_>) -> bool {
    let span = semantic.nodes().kind(node).span();
    for ancestor in semantic.nodes().ancestor_kinds(node) {
        match ancestor {
            AstKind::TryStatement(statement) => {
                let protected = statement.block.span.start <= span.start
                    && span.end <= statement.block.span.end;
                if statement.handler.is_some() && protected {
                    return false;
                }
            }
            AstKind::ArrowFunctionExpression(arrow) => return arrow.node_id.get() == body,
            AstKind::Function(_) => return false,
            _ => (),
        }
    }
    false
}

fn unwritten_initializer<'a>(
    symbol: SymbolId,
    semantic: &Semantic<'a>,
) -> Option<&'a Expression<'a>> {
    if semantic
        .scoping()
        .get_resolved_references(symbol)
        .any(|reference| reference.is_write())
    {
        return None;
    }
    semantic
        .scoping()
        .symbol_declarations(symbol)
        .find_map(|node| match semantic.nodes().kind(node) {
            AstKind::VariableDeclarator(declaration) => declaration.init.as_ref(),
            _ => None,
        })
}

// A stored/factory closure can capture the same source symbol in another activation.
// Only a stable binding chain to a local constructor preserves incoming capture proofs.
fn current_capture_callee(expression: &Expression<'_>, semantic: &Semantic<'_>) -> bool {
    let mut work = vec![(expression, 0)];
    let mut aliases = BTreeSet::new();
    while let Some((expression, depth)) = work.pop() {
        let Some((expression, depth)) = closures::unwrapped_at(expression, depth) else {
            return false;
        };
        match expression {
            Expression::ArrowFunctionExpression(_) => (),
            Expression::Identifier(identifier) => {
                let Some(symbol) = identifier
                    .reference_id
                    .get()
                    .and_then(|id| semantic.scoping().get_reference(id).symbol_id())
                else {
                    return false;
                };
                let Some(initializer) = unwritten_initializer(symbol, semantic) else {
                    return false;
                };
                if aliases.insert((symbol, depth)) {
                    work.push((initializer, depth + 1));
                }
            }
            Expression::ConditionalExpression(choice) => {
                work.push((&choice.consequent, depth + 1));
                work.push((&choice.alternate, depth + 1));
            }
            Expression::LogicalExpression(choice) => {
                work.push((&choice.left, depth + 1));
                work.push((&choice.right, depth + 1));
            }
            Expression::SequenceExpression(sequence) => {
                let Some(last) = sequence.expressions.last() else {
                    return false;
                };
                work.push((last, depth + 1));
            }
            Expression::AssignmentExpression(assignment)
                if assignment.operator == AssignmentOperator::Assign =>
            {
                work.push((&assignment.right, depth + 1))
            }
            _ => return false,
        }
    }
    true
}

// Call-site proofs refine provenance, never the callee's physical signature.
fn value_may_be_promise(
    expression: &Expression<'_>,
    primitive: &BTreeSet<SymbolId>,
    semantic: &Semantic<'_>,
    returns: &callbacks::ReturnOrigins<'_, '_, '_>,
    inputs: &Inputs,
) -> bool {
    if !return_adopts(expression, semantic, returns, inputs) {
        return false;
    }
    let mut work = vec![(expression, 0)];
    let mut aliases = BTreeSet::new();
    while let Some((expression, depth)) = work.pop() {
        let Some((expression, depth)) = closures::unwrapped_at(expression, depth) else {
            return true;
        };
        match expression {
            Expression::Identifier(identifier) => {
                if let Some(symbol) = identifier
                    .reference_id
                    .get()
                    .and_then(|id| semantic.scoping().get_reference(id).symbol_id())
                {
                    if primitive.contains(&symbol) {
                        continue;
                    }
                    if let Some(initializer) = unwritten_initializer(symbol, semantic) {
                        if aliases.insert((symbol, depth)) {
                            work.push((initializer, depth + 1));
                        }
                        continue;
                    }
                }
                if return_adopts(expression, semantic, returns, inputs) {
                    return true;
                }
            }
            Expression::ConditionalExpression(choice) => {
                work.push((&choice.consequent, depth + 1));
                work.push((&choice.alternate, depth + 1));
            }
            Expression::LogicalExpression(choice) => {
                work.push((&choice.left, depth + 1));
                work.push((&choice.right, depth + 1));
            }
            Expression::SequenceExpression(sequence) => {
                let Some(last) = sequence.expressions.last() else {
                    return true;
                };
                work.push((last, depth + 1));
            }
            _ if return_adopts(expression, semantic, returns, inputs) => return true,
            _ => (),
        }
    }
    false
}

// Describe the rejection of an awaited value, rather than whether that value
// itself can be a Promise. Reject uses its argument; resolve follows its input.
fn may_reject_raw(
    expression: &Expression<'_>,
    semantic: &Semantic<'_>,
    returns: &callbacks::ReturnOrigins<'_, '_, '_>,
    inputs: &Inputs,
) -> bool {
    let mut calls = BTreeSet::new();
    let mut contexts = vec![BTreeSet::new()];
    let mut work = vec![(expression, 0, 0)];
    let mut aliases = BTreeSet::new();
    while let Some((expression, depth, context)) = work.pop() {
        let Some((expression, depth)) = closures::unwrapped_at(expression, depth) else {
            return true;
        };
        match expression {
            Expression::Identifier(identifier) => {
                if identifier
                    .reference_id
                    .get()
                    .and_then(|id| semantic.scoping().get_reference(id).symbol_id())
                    .is_some_and(|symbol| contexts[context].contains(&symbol))
                {
                    continue;
                }
                let initializer = identifier
                    .reference_id
                    .get()
                    .and_then(|reference| semantic.scoping().get_reference(reference).symbol_id())
                    .and_then(|symbol| {
                        unwritten_initializer(symbol, semantic)
                            .map(|initializer| (symbol, initializer))
                    });
                if let Some((symbol, initializer)) = initializer {
                    // Same source/depth is the same query state; shared aliases
                    // must not expand exponentially. Depth still bounds cycles.
                    if aliases.insert((symbol, depth, context)) {
                        work.push((initializer, depth + 1, context));
                    }
                } else if return_adopts(expression, semantic, returns, inputs) {
                    return true;
                }
            }
            Expression::ConditionalExpression(conditional) => {
                work.push((&conditional.consequent, depth + 1, context));
                work.push((&conditional.alternate, depth + 1, context));
            }
            Expression::LogicalExpression(logical) => {
                work.push((&logical.left, depth + 1, context));
                work.push((&logical.right, depth + 1, context));
            }
            Expression::SequenceExpression(sequence) => {
                let Some(last) = sequence.expressions.last() else {
                    return true;
                };
                work.push((last, depth + 1, context));
            }
            Expression::AssignmentExpression(assignment)
                if assignment.operator == AssignmentOperator::Assign =>
            {
                work.push((&assignment.right, depth + 1, context));
            }
            Expression::CallExpression(call) => match awaited::intrinsic(call, semantic) {
                Ok(Some(intrinsic)) => {
                    if let Some(argument) = intrinsic.argument {
                        if intrinsic.rejected {
                            if value_may_be_promise(
                                argument,
                                &contexts[context],
                                semantic,
                                returns,
                                inputs,
                            ) {
                                return true;
                            }
                        } else {
                            work.push((argument, depth + 1, context));
                        }
                    }
                }
                Err(_) => return true,
                Ok(None) => {
                    if !return_adopts(expression, semantic, returns, inputs) {
                        continue;
                    }
                    if let Some(bodies) = callees(&call.callee, returns) {
                        for body in bodies {
                            let arrow = returns.arrows[&body];
                            let mut primitive = if current_capture_callee(&call.callee, semantic) {
                                contexts[context].clone()
                            } else {
                                BTreeSet::new()
                            };
                            for parameter in &arrow.params.items {
                                if let BindingPattern::BindingIdentifier(binding) =
                                    &parameter.pattern
                                    && let Some(symbol) = binding.symbol_id.get()
                                {
                                    primitive.remove(&symbol);
                                }
                            }
                            for (parameter, argument) in
                                arrow.params.items.iter().zip(&call.arguments)
                            {
                                if let BindingPattern::BindingIdentifier(binding) =
                                    &parameter.pattern
                                    && let (Some(symbol), Some(argument)) =
                                        (binding.symbol_id.get(), argument.as_expression())
                                    && !semantic
                                        .scoping()
                                        .get_resolved_references(symbol)
                                        .any(|reference| reference.is_write())
                                    && !value_may_be_promise(
                                        argument,
                                        &contexts[context],
                                        semantic,
                                        returns,
                                        inputs,
                                    )
                                {
                                    primitive.insert(symbol);
                                }
                            }
                            let child = if let Some(index) =
                                contexts.iter().position(|known| known == &primitive)
                            {
                                index
                            } else {
                                // Exhausted precision uses the original module-wide facts.
                                if contexts.len() >= 128 {
                                    0
                                } else {
                                    contexts.push(primitive);
                                    contexts.len() - 1
                                }
                            };
                            if !calls.insert((body, depth, child)) {
                                continue;
                            }
                            if arrow.r#async {
                                for node in semantic.nodes().iter() {
                                    let (argument, thrown) = match node.kind() {
                                        AstKind::ThrowStatement(statement) => {
                                            (&statement.argument, true)
                                        }
                                        AstKind::AwaitExpression(expression) => {
                                            (&expression.argument, false)
                                        }
                                        _ => continue,
                                    };
                                    if !escapes_arrow(node.id(), body, semantic) {
                                        continue;
                                    }
                                    if thrown {
                                        // Throw preserves a value; await follows its rejection.
                                        if value_may_be_promise(
                                            argument,
                                            &contexts[child],
                                            semantic,
                                            returns,
                                            inputs,
                                        ) {
                                            return true;
                                        }
                                    } else {
                                        work.push((argument, depth + 1, child));
                                    }
                                }
                            }
                            if let Some(value) = arrow.get_expression() {
                                work.push((value, depth + 1, child));
                            } else {
                                // Bare returns and fallthrough yield non-rejecting undefined.
                                work.extend(
                                    block_results::return_statements(semantic, body)
                                        .into_iter()
                                        .filter_map(|statement| statement.argument.as_ref())
                                        .map(|value| (value, depth + 1, child)),
                                );
                            }
                        }
                    } else {
                        return true;
                    }
                }
            },
            _ => {
                if return_adopts(expression, semantic, returns, inputs) {
                    return true;
                }
            }
        }
    }
    false
}

pub(super) fn routes(
    semantic: &Semantic<'_>,
    returns: &callbacks::ReturnOrigins<'_, '_, '_>,
    inputs: &Inputs,
) -> Routes {
    let mut routes = Routes::default();
    for node in semantic.nodes().iter() {
        let raw = match node.kind() {
            AstKind::ThrowStatement(statement) => {
                return_adopts(&statement.argument, semantic, returns, inputs)
            }
            // Nested argument failures have their own await/throw source nodes.
            AstKind::AwaitExpression(expression) => {
                may_reject_raw(&expression.argument, semantic, returns, inputs)
            }
            _ => false,
        };
        if !raw {
            continue;
        }
        let span = node.kind().span();
        for ancestor in semantic.nodes().ancestor_kinds(node.id()) {
            match ancestor {
                // A nested body's exceptions reach callers through its Promise,
                // not through the catch surrounding its declaration.
                AstKind::Function(_) | AstKind::ArrowFunctionExpression(_) => break,
                AstKind::TryStatement(statement) => {
                    let protected = statement.block.span.start <= span.start
                        && span.end <= statement.block.span.end;
                    if protected && let Some(handler) = &statement.handler {
                        routes.handlers.insert(handler.span.start);
                        break;
                    }
                    let handling = statement.handler.as_ref().is_some_and(|handler| {
                        handler.span.start <= span.start && span.end <= handler.span.end
                    });
                    // Cleanup exceptions bypass this scope's catch and pending
                    // reason buffer. Body/handler exceptions cross its buffer.
                    if statement.finalizer.is_some() && (protected || handling) {
                        routes.finalizers.insert(statement.span.start);
                    }
                }
                _ => (),
            }
        }
    }
    routes
}
