//! Literal property names shared by source admission and field-origin analysis.
use super::*;
use oxc_ast::ast::PropertyKey;
use oxc_syntax::number::ToJsString;

pub(super) fn property(key: &PropertyKey<'_>, computed: bool, depth: usize) -> Option<Vec<u16>> {
    if let PropertyKey::StaticIdentifier(identifier) = key
        && !computed
    {
        return Some(identifier.name.encode_utf16().collect());
    }
    literal(closures::unwrapped(key.as_expression()?, depth)?)
}

pub(super) fn literal(expr: &Expression<'_>) -> Option<Vec<u16>> {
    if let Expression::StringLiteral(text) = expr {
        return Some(text.value.encode_utf16().collect());
    }
    let number = number(expr)?;
    // Both zeros name the same property. Other values use ECMAScript Number::toString.
    let name = if number == 0.0 {
        "0".to_owned()
    } else {
        number.to_js_string()
    };
    Some(name.encode_utf16().collect())
}

pub(super) fn number(expr: &Expression<'_>) -> Option<f64> {
    match expr {
        Expression::NumericLiteral(value) => Some(value.value),
        Expression::UnaryExpression(unary) => {
            let Expression::NumericLiteral(value) = &unary.argument else {
                return None;
            };
            match unary.operator {
                UnaryOperator::UnaryNegation => Some(-value.value),
                UnaryOperator::UnaryPlus => Some(value.value),
                _ => None,
            }
        }
        _ => None,
    }
}
