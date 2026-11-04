//! Original-source lexical iteration bindings shared with representation plans.
use super::*;
use oxc_ast::ast::{BindingIdentifier, ForOfStatement, ForStatementLeft};

/// Planning does not admit the iterable, await mode or body. The source cursor
/// checks those before scheduling any execution or publishing this binding.
pub(super) fn for_of_binding<'n, 'a>(
    source: &'n ForOfStatement<'a>,
) -> Option<&'n BindingIdentifier<'a>> {
    let ForStatementLeft::VariableDeclaration(declaration) = &source.left else {
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
    match &local.id {
        BindingPattern::BindingIdentifier(identifier) => Some(identifier),
        _ => None,
    }
}
