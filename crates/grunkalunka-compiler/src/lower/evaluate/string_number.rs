//! StringNumericLiteral conversion for side-effect-free source literals.
use super::*;
use num_bigint::BigUint;
use num_traits::ToPrimitive;
use oxc_syntax::{identifier::is_white_space, line_terminator::is_line_terminator};

pub(super) fn literal(source: &Expression<'_>, depth: usize) -> Option<f64> {
    let text = match closures::unwrapped(source, depth)? {
        Expression::StringLiteral(literal) => literal.value.as_str(),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => {
            template.quasis.first()?.value.cooked.as_ref()?.as_str()
        }
        _ => return None,
    };
    // An unpaired UTF-16 surrogate cannot occur in a StringNumericLiteral.
    Some(text.map_or(f64::NAN, parse))
}

// Compare receives the literal as its left operand. The second operator
// restores source order when the literal originally appeared on the right.
pub(super) fn comparisons(operator: BinaryOperator) -> Option<(Compare, Compare)> {
    Some(match operator {
        BinaryOperator::Equality => (Compare::Equal, Compare::Equal),
        BinaryOperator::Inequality => (Compare::NotEqual, Compare::NotEqual),
        BinaryOperator::LessThan => (Compare::Less, Compare::Greater),
        BinaryOperator::LessEqualThan => (Compare::LessEqual, Compare::GreaterEqual),
        BinaryOperator::GreaterThan => (Compare::Greater, Compare::Less),
        BinaryOperator::GreaterEqualThan => (Compare::GreaterEqual, Compare::LessEqual),
        _ => return None,
    })
}

pub(in crate::lower) fn parse(text: &str) -> f64 {
    let text = text.trim_matches(|c| is_white_space(c) || is_line_terminator(c));
    match text {
        "" => return 0.0,
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    let radix = match text.as_bytes() {
        [b'0', b'x' | b'X', ..] => Some(16),
        [b'0', b'o' | b'O', ..] => Some(8),
        [b'0', b'b' | b'B', ..] => Some(2),
        _ => None,
    };
    if let Some(radix) = radix {
        let digits = &text[2..];
        if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
            return f64::NAN;
        }
        let digits = digits.trim_start_matches('0');
        if digits.is_empty() {
            return 0.0;
        }
        if let Ok(exact) = u64::from_str_radix(digits, radix) {
            return exact as f64;
        }
        // Even in binary more than 1024 significant digits must overflow.
        if digits.len() > 1024 {
            return f64::INFINITY;
        }
        return BigUint::parse_bytes(digits.as_bytes(), radix)
            .and_then(|integer| integer.to_f64())
            .unwrap_or(f64::INFINITY);
    }
    // Exclude Rust float spellings such as "inf" before decimal parsing.
    if text
        .bytes()
        .all(|b| matches!(b, b'0'..=b'9' | b'+' | b'-' | b'.' | b'e' | b'E'))
    {
        text.parse().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}
