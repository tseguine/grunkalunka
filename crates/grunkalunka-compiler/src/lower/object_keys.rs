//! Intrinsic recognition is independent of representation planning.
use super::*;

pub(super) fn is_call(call: &CallExpression<'_>, semantic: &Semantic<'_>) -> bool {
    if call.optional {
        return false;
    }
    let Some((callee, _)) = closures::unwrapped_at(invocation::callee(call), 0) else {
        return false;
    };
    let object = match callee {
        Expression::StaticMemberExpression(m) if !m.optional && m.property.name == "keys" => {
            &m.object
        }
        Expression::ComputedMemberExpression(m)
            if !m.optional
                && matches!(closures::unwrapped_at(&m.expression,0),Some((Expression::StringLiteral(s),_))if s.value=="keys") =>
        {
            &m.object
        }
        _ => return false,
    };
    let Some((Expression::Identifier(id), _)) = closures::unwrapped_at(object, 0) else {
        return false;
    };
    id.name == "Object"
        && id
            .reference_id
            .get()
            .and_then(|r| semantic.scoping().get_reference(r).symbol_id())
            .is_none_or(|s| !semantic.scoping().symbol_flags(s).is_value())
}
