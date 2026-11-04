//! Lexical source handlers that close throws without a function-boundary ABI.
use oxc_ast::AstKind;
use oxc_semantic::Semantic;
use oxc_span::GetSpan;
use oxc_syntax::node::NodeId;

pub(super) fn locally_caught(node: NodeId, semantic: &Semantic<'_>) -> bool {
    let span = semantic.nodes().kind(node).span();
    for ancestor in semantic.nodes().ancestor_kinds(node) {
        match ancestor {
            AstKind::TryStatement(statement)
                if statement.handler.is_some()
                    && statement.block.span.start <= span.start
                    && span.end <= statement.block.span.end =>
            {
                return true;
            }
            // A handler does not catch its own exceptions. Continue to an
            // enclosing protected block, but never into a caller's context.
            AstKind::Function(_) | AstKind::ArrowFunctionExpression(_) => return false,
            _ => (),
        }
    }
    false
}
