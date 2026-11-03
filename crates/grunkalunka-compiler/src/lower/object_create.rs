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
    matches!(
        closures::unwrapped_at(argument, 0),
        Some((Expression::NullLiteral(_), _))
    )
    .then_some(argument)
}
