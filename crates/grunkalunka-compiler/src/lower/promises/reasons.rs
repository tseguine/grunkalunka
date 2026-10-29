//! Lexical destinations of represented raw rejection reasons.
use super::*;

#[derive(Default)]
pub(super) struct Routes {
    pub handlers: BTreeSet<u32>,
    pub finalizers: BTreeSet<u32>,
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
            // A pristine intrinsic rejects with (or adopts) its first argument;
            // nested argument failures have their own await/throw source nodes.
            // Other pending inputs remain conservatively raw across body calls.
            AstKind::AwaitExpression(expression) => awaited::operand(expression, 0, semantic)
                .map(|operand| {
                    operand
                        .argument
                        .is_some_and(|argument| return_adopts(argument, semantic, returns, inputs))
                })
                .unwrap_or(true),
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
