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
    let mut work = vec![(expression, 0)];
    while let Some((expression, depth)) = work.pop() {
        let Some((expression, depth)) = closures::unwrapped_at(expression, depth) else {
            return true;
        };
        match expression {
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
                    if return_adopts(expression, semantic, returns, inputs) {
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
