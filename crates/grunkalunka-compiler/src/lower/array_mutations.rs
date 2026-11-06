//! Shared spelling recognition for mutation admission and source provenance.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Operation {
    Pop,
    Shift,
    Reverse,
    ToReversed,
}
impl Operation {
    pub(super) fn result(self) -> Type {
        match self {
            Self::Pop | Self::Shift => Type::ManagedValue,
            Self::Reverse | Self::ToReversed => Type::ManagedArray,
        }
    }
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Pop => "pop",
            Self::Shift => "shift",
            Self::Reverse => "reverse",
            Self::ToReversed => "toReversed",
        }
    }
}
pub(super) fn operation<'e, 'a>(
    call: &'e CallExpression<'a>,
) -> Option<(Operation, &'e Expression<'a>)> {
    if call.optional {
        return None;
    }
    let (callee, _) = closures::unwrapped_at(&call.callee, 0)?;
    let (receiver, name) = match callee {
        Expression::StaticMemberExpression(m) if !m.optional => {
            (&m.object, m.property.name.as_str())
        }
        Expression::ComputedMemberExpression(m) if !m.optional => {
            let (Expression::StringLiteral(s), _) = closures::unwrapped_at(&m.expression, 0)?
            else {
                return None;
            };
            (&m.object, s.value.as_str()?)
        }
        _ => return None,
    };
    Some((
        match name {
            "pop" => Operation::Pop,
            "shift" => Operation::Shift,
            "reverse" => Operation::Reverse,
            "toReversed" => Operation::ToReversed,
            _ => return None,
        },
        receiver,
    ))
}
