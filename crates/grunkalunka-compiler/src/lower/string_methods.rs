//! Recognize intrinsic spellings; positive receiver facts still own admission.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum Method {
    Slice,
    Substring,
    CharAt,
    CharCodeAt,
    StartsWith,
    Includes,
    Trim,
    TrimStart,
    TrimEnd,
}

impl Method {
    pub(super) fn result(self) -> Type {
        match self {
            Self::Slice
            | Self::Substring
            | Self::CharAt
            | Self::Trim
            | Self::TrimStart
            | Self::TrimEnd => Type::ManagedValue,
            Self::CharCodeAt => Type::Number,
            Self::StartsWith | Self::Includes => Type::Predicate,
        }
    }

    pub(super) fn arguments(self) -> usize {
        match self {
            Self::Slice | Self::Substring | Self::StartsWith | Self::Includes => 2,
            Self::CharAt | Self::CharCodeAt => 1,
            Self::Trim | Self::TrimStart | Self::TrimEnd => 0,
        }
    }
}

pub(super) fn call<'e, 'a>(call: &'e CallExpression<'a>) -> Option<(&'e Expression<'a>, Method)> {
    if call.optional {
        return None;
    }
    let (callee, _) = closures::unwrapped_at(&call.callee, 0)?;
    let (receiver, name) = match callee {
        Expression::StaticMemberExpression(member) if !member.optional => {
            (&member.object, member.property.name.as_str())
        }
        Expression::ComputedMemberExpression(member) if !member.optional => {
            let (Expression::StringLiteral(name), _) =
                closures::unwrapped_at(&member.expression, 0)?
            else {
                return None;
            };
            (&member.object, name.value.as_str()?)
        }
        _ => return None,
    };
    let method = match name {
        "slice" => Method::Slice,
        "substring" => Method::Substring,
        "charAt" => Method::CharAt,
        "charCodeAt" => Method::CharCodeAt,
        "startsWith" => Method::StartsWith,
        "includes" => Method::Includes,
        "trim" => Method::Trim,
        "trimStart" => Method::TrimStart,
        "trimEnd" => Method::TrimEnd,
        _ => return None,
    };
    Some((receiver, method))
}

// A source-proven undefined argument supplies the intrinsic default, while its
// original expression still executes for effects. Annotations never prove this.
pub(super) fn undefined(node: &Expression<'_>, semantic: &Semantic<'_>) -> bool {
    match closures::unwrapped_at(node, 0).map(|(node, _)| node) {
        Some(Expression::UnaryExpression(unary)) => unary.operator == UnaryOperator::Void,
        Some(Expression::Identifier(identifier)) if identifier.name == "undefined" => {
            identifier.reference_id.get().is_some_and(|reference| {
                semantic
                    .scoping()
                    .get_reference(reference)
                    .symbol_id()
                    .is_none()
            })
        }
        _ => false,
    }
}
