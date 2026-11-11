//! Compare source-known BigInts without passing through a Number.
use super::*;
use num_bigint::BigInt;
use num_traits::{FromPrimitive, ToPrimitive};
use std::cmp::Ordering;

pub(super) fn literal(mut node: &Expression<'_>, mut depth: usize) -> Option<BigInt> {
    let mut negative = false;
    loop {
        let (inner, next) = closures::unwrapped_at(node, depth)?;
        match inner {
            Expression::UnaryExpression(unary)
                if unary.operator == UnaryOperator::UnaryNegation =>
            {
                negative = !negative;
                node = &unary.argument;
                depth = next + 1;
            }
            Expression::BigIntLiteral(literal) => {
                // Oxc normalizes separators and nondecimal radices to decimal digits.
                let value = BigInt::parse_bytes(literal.value.as_bytes(), 10)?;
                return Some(if negative { -value } else { value });
            }
            _ => return None,
        }
    }
}

pub(super) fn comparison(node: &Expression<'_>, depth: usize) -> Option<bool> {
    let Expression::BinaryExpression(binary) = node else {
        return None;
    };
    let left = literal(&binary.left, depth + 1)?;
    let right = literal(&binary.right, depth + 1)?;
    Some(match binary.operator {
        BinaryOperator::Equality | BinaryOperator::StrictEquality => left == right,
        BinaryOperator::Inequality | BinaryOperator::StrictInequality => left != right,
        BinaryOperator::LessThan => left < right,
        BinaryOperator::LessEqualThan => left <= right,
        BinaryOperator::GreaterThan => left > right,
        BinaryOperator::GreaterEqualThan => left >= right,
        _ => return None,
    })
}

// The literal is the left operand of the emitted comparison. For an inexact
// conversion there is no binary64 number between the integer and its rounded
// neighbor. Include that neighbor exactly when it satisfies the source test.
pub(super) fn number_threshold(integer: BigInt, operator: Compare) -> Option<(f64, Compare)> {
    let rounded = integer.to_f64()?;
    let rounding = if rounded.is_infinite() {
        if rounded.is_sign_positive() {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    } else {
        BigInt::from_f64(rounded)?.cmp(&integer)
    };
    if rounding == Ordering::Equal {
        return Some((rounded, operator));
    }
    let rounded_up = rounding == Ordering::Greater;
    Some(match operator {
        Compare::Equal | Compare::NotEqual => (f64::NAN, operator),
        Compare::Less | Compare::LessEqual => (
            rounded,
            if rounded_up {
                Compare::LessEqual
            } else {
                Compare::Less
            },
        ),
        Compare::Greater | Compare::GreaterEqual => (
            rounded,
            if rounded_up {
                Compare::Greater
            } else {
                Compare::GreaterEqual
            },
        ),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn strict_comparisons_guard_unsupported_operands_and_operand_depth() {
        for operand in ["+1n", "(a++,1n)", "BigInt(1)", "(1n+1n)"] {
            let source = format!("export function run(a,b){{return {operand} === 1n ? a : b;}}");
            assert!(crate::compile(&source, false).is_err(), "{source}");
        }
        for expression in ["a == (1n + 1n)", "a < +1n"] {
            let source = format!("export function run(a,b){{return {expression} ? a : b;}}");
            assert!(crate::compile(&source, false).is_err(), "{source}");
        }
        let operand = format!("{}1n", "- ".repeat(129));
        let source = format!("export function run(a,b){{return {operand} === 1n ? a : b;}}");
        assert!(crate::compile(&source, false).is_err());
    }
}
