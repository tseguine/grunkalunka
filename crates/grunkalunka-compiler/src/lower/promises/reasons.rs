//! Lexical destinations of represented raw rejection reasons.
use super::*;

#[derive(Default)]
pub(super) struct Routes {
    pub handlers: BTreeSet<u32>,
    pub finalizers: BTreeSet<u32>,
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
    let mut work = vec![(expression, 0)];
    let mut aliases = BTreeSet::new();
    while let Some((expression, depth)) = work.pop() {
        let Some((expression, depth)) = closures::unwrapped_at(expression, depth) else {
            return true;
        };
        match expression {
            Expression::Identifier(identifier) => {
                let initializer = identifier
                    .reference_id
                    .get()
                    .and_then(|reference| semantic.scoping().get_reference(reference).symbol_id())
                    .filter(|symbol| {
                        !semantic
                            .scoping()
                            .get_resolved_references(*symbol)
                            .any(|reference| reference.is_write())
                    })
                    .and_then(|symbol| {
                        semantic
                            .scoping()
                            .symbol_declarations(symbol)
                            .find_map(|node| match semantic.nodes().kind(node) {
                                AstKind::VariableDeclarator(declaration) => declaration
                                    .init
                                    .as_ref()
                                    .map(|initializer| (symbol, initializer)),
                                _ => None,
                            })
                    });
                if let Some((symbol, initializer)) = initializer {
                    // Same source/depth is the same query state; shared aliases
                    // must not expand exponentially. Depth still bounds cycles.
                    if aliases.insert((symbol, depth)) {
                        work.push((initializer, depth + 1));
                    }
                } else if return_adopts(expression, semantic, returns, inputs) {
                    return true;
                }
            }
            Expression::ConditionalExpression(conditional) => {
                work.push((&conditional.consequent, depth + 1));
                work.push((&conditional.alternate, depth + 1));
            }
            Expression::LogicalExpression(logical) => {
                work.push((&logical.left, depth + 1));
                work.push((&logical.right, depth + 1));
            }
            Expression::SequenceExpression(sequence) => {
                let Some(last) = sequence.expressions.last() else {
                    return true;
                };
                work.push((last, depth + 1));
            }
            Expression::AssignmentExpression(assignment)
                if assignment.operator == AssignmentOperator::Assign =>
            {
                work.push((&assignment.right, depth + 1));
            }
            Expression::CallExpression(call) => match awaited::intrinsic(call, semantic) {
                Ok(Some(intrinsic)) => {
                    if let Some(argument) = intrinsic.argument {
                        if intrinsic.rejected {
                            if return_adopts(argument, semantic, returns, inputs) {
                                return true;
                            }
                        } else {
                            work.push((argument, depth + 1));
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
                            if !calls.insert((body, depth)) {
                                continue;
                            }
                            let arrow = returns.arrows[&body];
                            // Async bodies can also reject from their own await/throw sites.
                            if arrow.r#async {
                                return true;
                            }
                            if let Some(value) = arrow.get_expression() {
                                work.push((value, depth + 1));
                            } else {
                                // Bare returns and fallthrough yield non-rejecting undefined.
                                work.extend(
                                    block_results::return_statements(semantic, body)
                                        .into_iter()
                                        .filter_map(|statement| statement.argument.as_ref())
                                        .map(|value| (value, depth + 1)),
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
