//! Shared spelling recognition for mutation admission and source provenance.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Operation {
    Push,
    Pop,
    Shift,
    Reverse,
    ToReversed,
}
impl Operation {
    pub(super) fn result(self) -> Type {
        match self {
            Self::Push => Type::Number,
            Self::Pop | Self::Shift => Type::ManagedValue,
            Self::Reverse | Self::ToReversed => Type::ManagedArray,
        }
    }
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Push => "push",
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
            "push" => Operation::Push,
            "pop" => Operation::Pop,
            "shift" => Operation::Shift,
            "reverse" => Operation::Reverse,
            "toReversed" => Operation::ToReversed,
            _ => return None,
        },
        receiver,
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn append_is_admitted_only_on_source_proven_arrays() {
        for body in [
            "const xs=[];return xs.push(a);",
            "const xs=[a];return xs.push(b,a);",
            "const xs=[a];xs.push(b);return xs.length;",
            "const xs=[a];return xs['push'](b);",
            "const xs=[a];return xs.push()===1?1:0;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let result = crate::compile(&source, typed);
                assert!(result.is_ok(), "{source}: {:?}", result.err());
            }
        }
        for body in [
            "return a.push(b);",
            "const xs=[a];return xs.push(...[b]);",
            "const xs=[a];return xs.push.call(xs,b);",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            assert!(crate::compile(&source, false).is_err(), "{source}");
        }
    }
}
