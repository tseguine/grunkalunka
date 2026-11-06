//! Intrinsic recognition is independent of representation planning.
use super::*;

fn method_name<'e>(call: &'e CallExpression<'_>, semantic: &Semantic<'_>) -> Option<&'e str> {
    if call.optional {
        return None;
    }
    let (callee, _) = closures::unwrapped_at(invocation::callee(call), 0)?;
    let (object, name) = match callee {
        Expression::StaticMemberExpression(m) if !m.optional => {
            (&m.object, m.property.name.as_str())
        }
        Expression::ComputedMemberExpression(m) if !m.optional => {
            let (Expression::StringLiteral(key), _) = closures::unwrapped_at(&m.expression, 0)?
            else {
                return None;
            };
            (&m.object, key.value.as_str()?)
        }
        _ => return None,
    };
    let (Expression::Identifier(id), _) = closures::unwrapped_at(object, 0)? else {
        return None;
    };
    (id.name == "Object"
        && id
            .reference_id
            .get()
            .and_then(|r| semantic.scoping().get_reference(r).symbol_id())
            .is_none_or(|s| !semantic.scoping().symbol_flags(s).is_value()))
    .then_some(name)
}

pub(super) fn is_method(call: &CallExpression<'_>, semantic: &Semantic<'_>, name: &str) -> bool {
    method_name(call, semantic) == Some(name)
}

pub(super) fn enumeration(
    call: &CallExpression<'_>,
    semantic: &Semantic<'_>,
) -> Option<ir::OwnEnumeration> {
    match method_name(call, semantic)? {
        "keys" => Some(ir::OwnEnumeration::Keys),
        "values" => Some(ir::OwnEnumeration::Values),
        "entries" => Some(ir::OwnEnumeration::Entries),
        _ => None,
    }
}
