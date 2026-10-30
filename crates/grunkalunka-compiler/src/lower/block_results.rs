//! Known block arrows specialize into the caller while retaining typed early returns.
use super::*;
use oxc_ast::{AstKind, ast::ArrowFunctionExpression};
use oxc_syntax::node::NodeId;
use std::collections::BTreeMap;

// Shared by result-kind and callback-origin queries. The nearest function owns
// each return; nested bodies never provide evidence about their creator.
pub(super) fn return_statements<'a>(
    semantic: &Semantic<'a>,
    body: NodeId,
) -> Vec<&'a oxc_ast::ast::ReturnStatement<'a>> {
    semantic
        .nodes()
        .iter()
        .filter_map(|node| {
            let AstKind::ReturnStatement(statement) = node.kind() else {
                return None;
            };
            let owner = semantic
                .nodes()
                .ancestor_kinds(node.id())
                .find_map(|kind| match kind {
                    AstKind::ArrowFunctionExpression(owner) => Some(owner.node_id.get()),
                    AstKind::Function(owner) => Some(owner.node_id.get()),
                    _ => None,
                });
            (owner == Some(body)).then_some(statement)
        })
        .collect()
}

impl Lowering<'_, '_> {
    // Only returns owned by this arrow participate; nested arrow bodies have their
    // own return contract. Shape recognition emits nothing and never runs effects.
    // Preserve full signature IDs; Number-leaf IDs alone coincide with arities.
    pub(super) fn block_function_arity(
        &self,
        arrow: &ArrowFunctionExpression<'_>,
        depth: usize,
    ) -> Option<usize> {
        self.block_function_arity_in(arrow, depth, &BTreeMap::new(), 0)
    }

    pub(super) fn block_function_arity_in(
        &self,
        arrow: &ArrowFunctionExpression<'_>,
        depth: usize,
        bindings: &BTreeMap<SymbolId, Option<usize>>,
        calls: usize,
    ) -> Option<usize> {
        if depth > 128 || arrow.get_expression().is_some() {
            return None;
        }
        let mut arity = None;
        for statement in return_statements(self.semantic, arrow.node_id.get()) {
            let next = self.block_expression_arity(
                statement.argument.as_ref()?,
                depth + 1,
                bindings,
                calls,
            )?;
            if arity.is_some_and(|expected| expected != next) {
                return None;
            }
            arity = Some(next);
        }
        arity
    }

    fn block_expression_arity(
        &self,
        expression: &Expression<'_>,
        depth: usize,
        bindings: &BTreeMap<SymbolId, Option<usize>>,
        calls: usize,
    ) -> Option<usize> {
        if depth > 128 {
            return None;
        }
        if let Some(arity) = self.function_arity_in(expression, depth, bindings, calls) {
            return Some(arity);
        }
        if let Some(inner) = erased_inner(expression) {
            return self.block_expression_arity(inner, depth + 1, bindings, calls);
        }
        match expression {
            Expression::ParenthesizedExpression(wrapper) => {
                self.block_expression_arity(&wrapper.expression, depth + 1, bindings, calls)
            }
            Expression::ConditionalExpression(conditional) => {
                let yes = self.block_expression_arity(
                    &conditional.consequent,
                    depth + 1,
                    bindings,
                    calls,
                )?;
                let no = self.block_expression_arity(
                    &conditional.alternate,
                    depth + 1,
                    bindings,
                    calls,
                )?;
                (yes == no).then_some(yes)
            }
            Expression::SequenceExpression(sequence) => self.block_expression_arity(
                sequence.expressions.last()?,
                depth + 1,
                bindings,
                calls,
            ),
            Expression::AssignmentExpression(assignment)
                if assignment.operator == AssignmentOperator::Assign =>
            {
                self.block_expression_arity(&assignment.right, depth + 1, bindings, calls)
            }
            Expression::Identifier(identifier) => {
                let symbol = self
                    .semantic
                    .scoping()
                    .get_reference(identifier.reference_id.get()?)
                    .symbol_id()?;
                if let Some(kind) = bindings.get(&symbol) {
                    return *kind;
                }
                // Symbol identity preserves shadowing. Recognition may inspect a
                // later initializer, but real lowering still enforces initialization.
                self.semantic
                    .nodes()
                    .iter()
                    .find_map(|node| {
                        let AstKind::VariableDeclarator(declaration) = node.kind() else {
                            return None;
                        };
                        let BindingPattern::BindingIdentifier(binding) = &declaration.id else {
                            return None;
                        };
                        (binding.symbol_id.get() == Some(symbol))
                            .then_some(declaration.init.as_ref())
                            .flatten()
                    })
                    .and_then(|initializer| {
                        self.block_expression_arity(initializer, depth + 1, bindings, calls)
                    })
            }
            _ => None,
        }
    }
}
