//! Shared spelling recognition for mutation admission and source provenance.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum Operation {
    Pop,
}

pub(super) fn operation<'e, 'a>(
    call: &'e CallExpression<'a>,
) -> Option<(Operation, &'e Expression<'a>)> {
    if call.optional {
        return None;
    }
    let (callee, _) = closures::unwrapped_at(&call.callee, 0)?;
    let receiver = match callee {
        Expression::StaticMemberExpression(m) if !m.optional && m.property.name == "pop" => {
            &m.object
        }
        Expression::ComputedMemberExpression(m)
            if !m.optional
                && matches!(closures::unwrapped_at(&m.expression, 0),
                    Some((Expression::StringLiteral(s), _)) if s.value == "pop") =>
        {
            &m.object
        }
        _ => return None,
    };
    Some((Operation::Pop, receiver))
}
