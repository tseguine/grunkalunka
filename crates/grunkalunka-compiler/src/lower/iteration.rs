//! Original-source lexical iteration bindings shared with representation plans.
use super::*;
use oxc_ast::ast::{
    ArrayExpression, ArrayExpressionElement, BindingIdentifier, ForOfStatement, ForStatementLeft,
};

/// Planning does not admit the iterable, await mode or body. The source cursor
/// checks those before scheduling any execution or publishing this binding.
pub(super) fn lexical_iteration_pattern<'n, 'a>(
    left: &'n ForStatementLeft<'a>,
) -> Option<&'n BindingPattern<'a>> {
    let ForStatementLeft::VariableDeclaration(declaration) = left else {
        return None;
    };
    if declaration.declare
        || !matches!(
            declaration.kind,
            VariableDeclarationKind::Const | VariableDeclarationKind::Let
        )
        || declaration.declarations.len() != 1
    {
        return None;
    }
    let local = &declaration.declarations[0];
    if local.definite || local.init.is_some() {
        return None;
    }
    Some(&local.id)
}

pub(super) fn for_of_pattern<'n, 'a>(
    source: &'n ForOfStatement<'a>,
) -> Option<&'n BindingPattern<'a>> {
    lexical_iteration_pattern(&source.left)
}

/// An existing lexical binding is an assignable iteration head. Patterns,
/// members and unsupported TS targets retain their existing diagnostics.
pub(super) fn assignment_iteration_identifier<'n, 'a>(
    left: &'n ForStatementLeft<'a>,
) -> Result<Option<&'n IdentifierReference<'a>>, String> {
    left.as_simple_assignment_target()
        .map(|target| target_identifier(target, 0))
        .transpose()
        .map(Option::flatten)
}

pub(super) fn for_of_binding<'n, 'a>(
    source: &'n ForOfStatement<'a>,
) -> Option<&'n BindingIdentifier<'a>> {
    match for_of_pattern(source)? {
        BindingPattern::BindingIdentifier(identifier) => Some(identifier),
        _ => None,
    }
}

pub(super) fn single_spread<'n, 'a>(
    literal: &'n ArrayExpression<'a>,
) -> Option<&'n Expression<'a>> {
    let [ArrayExpressionElement::SpreadElement(spread)] = literal.elements.as_slice() else {
        return None;
    };
    Some(&spread.argument)
}

impl Lowering<'_, '_> {
    pub(super) fn intrinsic_iterable_mode(
        &self,
        source: &Expression<'_>,
        depth: usize,
    ) -> Option<evaluate::Mode> {
        if self.stored.string(source.node_id()) {
            Some(evaluate::Mode::Managed)
        } else if self.receiver_type(source, depth) == Some(Type::ManagedArray) {
            Some(evaluate::Mode::Array)
        } else {
            None
        }
    }
}
