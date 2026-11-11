//! Compare source-known BigInts without passing through a Number.
use super::*;
use num_bigint::BigInt;

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

#[cfg(test)]
mod tests {
    #[test]
    fn comparisons_require_two_pure_bigints_and_respect_operand_depth() {
        for operand in ["1", "+1n", "(a++,1n)", "BigInt(1)", "(1n+1n)"] {
            let source = format!("export function run(a,b){{return {operand} === 1n ? a : b;}}");
            assert!(crate::compile(&source, false).is_err(), "{source}");
        }
        let operand = format!("{}1n", "- ".repeat(129));
        let source = format!("export function run(a,b){{return {operand} === 1n ? a : b;}}");
        assert!(crate::compile(&source, false).is_err());
    }
}
