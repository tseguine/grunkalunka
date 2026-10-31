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

// A single-pass read can precede writes in a later loop in the same activation.
fn source_owner(node: NodeId, semantic: &Semantic<'_>, allow_loops: bool) -> Option<NodeId> {
    for ancestor in semantic.nodes().ancestor_kinds(node) {
        match ancestor {
            AstKind::ForStatement(_)
            | AstKind::ForInStatement(_)
            | AstKind::ForOfStatement(_)
            | AstKind::WhileStatement(_)
            | AstKind::DoWhileStatement(_)
                if !allow_loops =>
            {
                return None;
            }
            AstKind::ArrowFunctionExpression(arrow) => return Some(arrow.node_id.get()),
            AstKind::Function(function) => return Some(function.node_id.get()),
            _ => (),
        }
    }
    None
}

// A saved read precedes later writes even when the saved Promise is awaited later.
// Captured writes and loop backedges cannot use textual order as a reaching proof.
fn snapshot_initializer<'a>(
    identifier: &oxc_ast::ast::IdentifierReference<'a>,
    semantic: &Semantic<'a>,
) -> Option<&'a Expression<'a>> {
    let reference = semantic
        .scoping()
        .get_reference(identifier.reference_id.get()?);
    let symbol = reference.symbol_id()?;
    let owner = source_owner(reference.node_id(), semantic, false)?;
    let initializer =
        semantic
            .scoping()
            .symbol_declarations(symbol)
            .find_map(|node| match semantic.nodes().kind(node) {
                AstKind::VariableDeclarator(declaration)
                    if source_owner(node, semantic, false) == Some(owner) =>
                {
                    declaration.init.as_ref()
                }
                _ => None,
            })?;
    if initializer.span().end > identifier.span.start
        || semantic
            .scoping()
            .get_resolved_references(symbol)
            .filter(|reference| reference.is_write())
            .any(|reference| {
                semantic.nodes().kind(reference.node_id()).span().start <= identifier.span.start
                    || source_owner(reference.node_id(), semantic, true) != Some(owner)
            })
    {
        return None;
    }
    Some(initializer)
}

// A completed standalone assignment dominates a later read in its own block
// or a contained block. Conditional expressions and loop backedges do not.
fn assigned_snapshot<'a>(
    identifier: &oxc_ast::ast::IdentifierReference<'a>,
    semantic: &Semantic<'a>,
) -> Option<&'a Expression<'a>> {
    let reference = semantic
        .scoping()
        .get_reference(identifier.reference_id.get()?);
    let symbol = reference.symbol_id()?;
    let owner = source_owner(reference.node_id(), semantic, false)?;
    let declaration =
        semantic
            .scoping()
            .symbol_declarations(symbol)
            .find_map(|node| match semantic.nodes().kind(node) {
                AstKind::VariableDeclarator(declaration)
                    if source_owner(node, semantic, false) == Some(owner) =>
                {
                    Some(declaration)
                }
                _ => None,
            })?;
    let containers: BTreeSet<_> = semantic
        .nodes()
        .ancestor_kinds(reference.node_id())
        .filter_map(|kind| match kind {
            AstKind::BlockStatement(block) => Some(block.node_id.get()),
            AstKind::FunctionBody(body) => Some(body.node_id.get()),
            _ => None,
        })
        .collect();
    let writes: Vec<_> = semantic
        .scoping()
        .get_resolved_references(symbol)
        .filter(|reference| reference.is_write())
        .collect();
    if writes
        .iter()
        .any(|write| source_owner(write.node_id(), semantic, true) != Some(owner))
    {
        return None;
    }
    let mut latest = None;
    for write in &writes {
        let mut ancestors = semantic.nodes().ancestor_kinds(write.node_id());
        let Some(AstKind::AssignmentExpression(assignment)) = ancestors.next() else {
            continue;
        };
        if assignment.operator != AssignmentOperator::Assign
            || assignment.span.end > identifier.span.start
            || declaration.span.end > assignment.span.start
        {
            continue;
        }
        let Some(AstKind::ExpressionStatement(statement)) = ancestors.next() else {
            continue;
        };
        if !matches!(&statement.expression, Expression::AssignmentExpression(value)
            if value.node_id.get() == assignment.node_id.get())
        {
            continue;
        }
        let container = match ancestors.next() {
            Some(AstKind::BlockStatement(block)) => block.node_id.get(),
            Some(AstKind::FunctionBody(body)) => body.node_id.get(),
            _ => continue,
        };
        if containers.contains(&container)
            && latest.is_none_or(
                |(_, previous): (NodeId, &oxc_ast::ast::AssignmentExpression<'a>)| {
                    previous.span.end < assignment.span.end
                },
            )
        {
            latest = Some((write.node_id(), assignment));
        }
    }
    let (write, assignment) = latest?;
    if writes.iter().any(|other| {
        other.node_id() != write && {
            let span = semantic.nodes().kind(other.node_id()).span();
            assignment.span.start <= span.start && span.start <= identifier.span.start
        }
    }) {
        return None;
    }
    Some(&assignment.right)
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

fn destination(node: NodeId, semantic: &Semantic<'_>) -> Option<u32> {
    let span = semantic.nodes().kind(node).span();
    for ancestor in semantic.nodes().ancestor_kinds(node) {
        match ancestor {
            AstKind::Function(_) | AstKind::ArrowFunctionExpression(_) => return None,
            AstKind::TryStatement(statement)
                if statement.block.span.start <= span.start
                    && span.end <= statement.block.span.end =>
            {
                if let Some(handler) = &statement.handler {
                    return Some(handler.span.start);
                }
            }
            _ => (),
        }
    }
    None
}

// A catch receives its protected sources, not every throw in its enclosing body.
// Keep source values and awaited rejections distinct: throw never adopts a Promise.
fn catch_sources<'a>(
    symbol: SymbolId,
    semantic: &Semantic<'a>,
) -> Option<Vec<(&'a Expression<'a>, bool)>> {
    if semantic
        .scoping()
        .get_resolved_references(symbol)
        .any(|reference| reference.is_write())
    {
        return None;
    }
    let handler = semantic.nodes().iter().find_map(|node| {
        let AstKind::CatchClause(handler) = node.kind() else {
            return None;
        };
        let BindingPattern::BindingIdentifier(binding) = &handler.param.as_ref()?.pattern else {
            return None;
        };
        (binding.symbol_id.get() == Some(symbol)).then_some(handler.span.start)
    })?;
    let sources: Vec<_> = semantic
        .nodes()
        .iter()
        .filter_map(|node| {
            let source = match node.kind() {
                AstKind::ThrowStatement(statement) => (&statement.argument, false),
                AstKind::AwaitExpression(expression) => (&expression.argument, true),
                _ => return None,
            };
            (destination(node.id(), semantic) == Some(handler)).then_some(source)
        })
        .collect();
    (!sources.is_empty()).then_some(sources)
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
    let mut work = vec![(expression, 0, false)];
    let mut aliases = BTreeSet::new();
    while let Some((expression, depth, rejected)) = work.pop() {
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
                        if aliases.insert((symbol, depth, rejected)) {
                            work.push((initializer, depth + 1, rejected));
                        }
                        continue;
                    }
                    if !rejected && let Some(sources) = catch_sources(symbol, semantic) {
                        if aliases.insert((symbol, depth, rejected)) {
                            work.extend(
                                sources
                                    .into_iter()
                                    .map(|(value, rejected)| (value, depth + 1, rejected)),
                            );
                        }
                        continue;
                    }
                }
                if return_adopts(expression, semantic, returns, inputs) {
                    return true;
                }
            }
            Expression::ConditionalExpression(choice) => {
                work.push((&choice.consequent, depth + 1, rejected));
                work.push((&choice.alternate, depth + 1, rejected));
            }
            Expression::LogicalExpression(choice) => {
                work.push((&choice.left, depth + 1, rejected));
                work.push((&choice.right, depth + 1, rejected));
            }
            Expression::SequenceExpression(sequence) => {
                let Some(last) = sequence.expressions.last() else {
                    return true;
                };
                work.push((last, depth + 1, rejected));
            }
            Expression::CallExpression(call) if rejected => {
                match awaited::intrinsic(call, semantic) {
                    Ok(Some(intrinsic)) => {
                        if let Some(argument) = intrinsic.argument {
                            work.push((argument, depth + 1, !intrinsic.rejected));
                        }
                    }
                    _ if return_adopts(expression, semantic, returns, inputs) => return true,
                    _ => (),
                }
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
                            .or_else(|| snapshot_initializer(identifier, semantic))
                            .or_else(|| assigned_snapshot(identifier, semantic))
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
                    if let Some(bodies) = callees(invocation::callee(call), returns) {
                        for body in bodies {
                            let arrow = returns.arrows[&body];
                            let mut primitive =
                                if current_capture_callee(invocation::callee(call), semantic) {
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
                                arrow.params.items.iter().zip(invocation::arguments(call))
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
