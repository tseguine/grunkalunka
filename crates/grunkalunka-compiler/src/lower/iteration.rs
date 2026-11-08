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

/// A var head assigns its already hoisted function cell on every visit.
pub(super) fn var_iteration_binding<'n, 'a>(
    left: &'n ForStatementLeft<'a>,
) -> Option<&'n BindingIdentifier<'a>> {
    let ForStatementLeft::VariableDeclaration(declaration) = left else {
        return None;
    };
    if declaration.declare
        || declaration.kind != VariableDeclarationKind::Var
        || declaration.declarations.len() != 1
    {
        return None;
    }
    let local = &declaration.declarations[0];
    if local.definite || local.init.is_some() {
        return None;
    }
    match &local.id {
        BindingPattern::BindingIdentifier(id) => Some(id),
        _ => None,
    }
}

pub(super) fn declaration_iteration_pattern<'n, 'a>(
    left: &'n ForStatementLeft<'a>,
) -> Option<&'n BindingPattern<'a>> {
    if let Some(pattern) = lexical_iteration_pattern(left) {
        return Some(pattern);
    }
    let ForStatementLeft::VariableDeclaration(d) = left else {
        return None;
    };
    if d.declare || d.kind != VariableDeclarationKind::Var || d.declarations.len() != 1 {
        return None;
    }
    let local = &d.declarations[0];
    if local.definite
        || local.init.is_some()
        || matches!(local.id, BindingPattern::BindingIdentifier(_))
    {
        return None;
    }
    Some(&local.id)
}

pub(super) fn for_of_pattern<'n, 'a>(
    source: &'n ForOfStatement<'a>,
) -> Option<&'n BindingPattern<'a>> {
    declaration_iteration_pattern(&source.left)
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
    match lexical_iteration_pattern(&source.left)? {
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

/// A fixed literal projection borrows source expressions. Holes in a spread
/// become explicit Undefined entries; holes in the containing literal stay holes.
#[derive(Clone, Copy)]
pub(super) enum LiteralElement<'n, 'a> {
    Value(&'n Expression<'a>, usize),
    Hole,
    Undefined,
}

pub(super) fn literal_elements<'n, 'a>(
    literal: &'n ArrayExpression<'a>,
) -> Option<Vec<LiteralElement<'n, 'a>>> {
    let mut pending = literal
        .elements
        .iter()
        .rev()
        .map(|element| (element, 1usize, false))
        .collect::<Vec<_>>();
    let mut result = Vec::new();
    while let Some((element, depth, iterated)) = pending.pop() {
        match element {
            ArrayExpressionElement::SpreadElement(spread) => {
                let (Expression::ArrayExpression(array), next) =
                    closures::unwrapped_at(&spread.argument, depth)?
                else {
                    return None;
                };
                if next >= 128 {
                    return None;
                }
                pending.extend(
                    array
                        .elements
                        .iter()
                        .rev()
                        .map(|element| (element, next + 1, true)),
                );
            }
            ArrayExpressionElement::Elision(_) => result.push(if iterated {
                LiteralElement::Undefined
            } else {
                LiteralElement::Hole
            }),
            _ => result.push(LiteralElement::Value(element.as_expression()?, depth)),
        }
    }
    Some(result)
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
