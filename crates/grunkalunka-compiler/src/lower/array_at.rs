//! Recognize the supported call spelling; receiver admission stays in lowering.
use super::*;

pub(super) fn receiver<'e, 'a>(call: &'e CallExpression<'a>) -> Option<&'e Expression<'a>> {
    if call.optional {
        return None;
    }
    let (callee, _) = closures::unwrapped_at(&call.callee, 0)?;
    match callee {
        Expression::StaticMemberExpression(m) if !m.optional && m.property.name == "at" => {
            Some(&m.object)
        }
        Expression::ComputedMemberExpression(m)
            if !m.optional
                && matches!(closures::unwrapped_at(&m.expression, 0),
                    Some((Expression::StringLiteral(s), _)) if s.value == "at") =>
        {
            Some(&m.object)
        }
        _ => None,
    }
}
