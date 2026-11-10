//! The first Object.create profile constructs a dictionary with no prototype.
use super::*;

pub(super) fn argument<'e, 'a>(
    call: &'e CallExpression<'a>,
    semantic: &Semantic<'_>,
) -> Option<&'e Expression<'a>> {
    if call.optional || call.arguments.len() != 1 {
        return None;
    }
    let (callee, _) = closures::unwrapped_at(&call.callee, 0)?;
    let object = match callee {
        Expression::StaticMemberExpression(m) if !m.optional && m.property.name == "create" => {
            &m.object
        }
        Expression::ComputedMemberExpression(m)
            if !m.optional
                && matches!(closures::unwrapped_at(&m.expression, 0),
                    Some((Expression::StringLiteral(s), _)) if s.value == "create") =>
        {
            &m.object
        }
        _ => return None,
    };
    let (Expression::Identifier(id), _) = closures::unwrapped_at(object, 0)? else {
        return None;
    };
    if id.name != "Object"
        || id
            .reference_id
            .get()
            .and_then(|r| semantic.scoping().get_reference(r).symbol_id())
            .is_some_and(|s| semantic.scoping().symbol_flags(s).is_value())
    {
        return None;
    }
    let argument = call.arguments[0].as_expression()?;
    null_prototype_value(argument, semantic, 0).then_some(argument)
}

// Only the colon form denotes a prototype setter. A shorthand or computed
// "__proto__" is an ordinary data property, including in a dictionary.
pub(super) fn prototype_setter(property: &oxc_ast::ast::ObjectProperty<'_>) -> bool {
    use oxc_ast::ast::{PropertyKey, PropertyKind};
    !property.computed
        && !property.shorthand
        && !property.method
        && property.kind == PropertyKind::Init
        && match &property.key {
            PropertyKey::StaticIdentifier(id) => id.name == "__proto__",
            PropertyKey::StringLiteral(name) => name.value == "__proto__",
            _ => false,
        }
}

pub(super) fn null_prototype_value<'a>(
    value: &Expression<'a>,
    semantic: &Semantic<'a>,
    depth: usize,
) -> bool {
    use oxc_ast::AstKind;
    let mut pending = vec![(value, depth)];
    let mut visited = std::collections::BTreeSet::new();
    while let Some((value, depth)) = pending.pop() {
        let Some((expr, at)) = closures::unwrapped_at(value, depth) else {
            return false;
        };
        match expr {
            Expression::NullLiteral(_) => {}
            Expression::SequenceExpression(seq) => {
                let Some(last) = seq.expressions.last() else {
                    return false;
                };
                pending.push((last, at + 1));
            }
            Expression::ConditionalExpression(choice) => {
                pending.push((&choice.consequent, at + 1));
                pending.push((&choice.alternate, at + 1));
            }
            Expression::Identifier(id) => {
                let Some(symbol) = id
                    .reference_id
                    .get()
                    .and_then(|r| semantic.scoping().get_reference(r).symbol_id())
                else {
                    return false;
                };
                let declaration = semantic.symbol_declaration(symbol);
                let AstKind::VariableDeclarator(decl) = declaration.kind() else {
                    return false;
                };
                if decl.span.end > id.span.start
                    || !matches!(&decl.id, BindingPattern::BindingIdentifier(_))
                    || !matches!(semantic.nodes().parent_kind(declaration.id()), AstKind::VariableDeclaration(d) if d.kind == VariableDeclarationKind::Const)
                {
                    return false;
                }
                if visited.insert(symbol) {
                    let Some(init) = &decl.init else { return false };
                    pending.push((init, at + 1));
                }
            }
            _ => return false,
        }
    }
    true
}
