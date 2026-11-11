//! Number exponentiation cases with a direct IEEE 754 operation.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum LiteralPower {
    Zero,
    Identity,
    Square,
    Reciprocal,
}

impl LiteralPower {
    pub(super) fn from_source(source: &Expression<'_>, depth: usize) -> Option<Self> {
        let (source, depth) = closures::unwrapped_at(source, depth)?;
        let value = match source {
            Expression::NumericLiteral(n) => n.value,
            Expression::UnaryExpression(u) if u.operator == UnaryOperator::UnaryNegation => {
                let Expression::NumericLiteral(n) = closures::unwrapped(&u.argument, depth + 1)?
                else {
                    return None;
                };
                -n.value
            }
            _ => return None,
        };
        match value {
            0.0 => Some(Self::Zero),
            1.0 => Some(Self::Identity),
            2.0 => Some(Self::Square),
            -1.0 => Some(Self::Reciprocal),
            _ => None,
        }
    }
}

impl Lowering<'_, '_> {
    pub(super) fn emit_literal_power(
        &mut self,
        source: &impl GetSpan,
        region: &mut Region,
        input: ValueId,
        power: LiteralPower,
    ) -> ValueId {
        let (operator, left) = match power {
            LiteralPower::Identity => return input,
            LiteralPower::Square => (Binary::Multiply, input),
            LiteralPower::Zero | LiteralPower::Reciprocal => {
                let one = self.value(Type::Number);
                Self::push(
                    region,
                    source,
                    Op::Constant {
                        result: one,
                        value: 1.0,
                    },
                );
                if matches!(power, LiteralPower::Zero) {
                    return one;
                }
                (Binary::Divide, one)
            }
        };
        let result = self.value(Type::Number);
        Self::push(
            region,
            source,
            Op::Binary {
                result,
                operator,
                left,
                right: input,
            },
        );
        result
    }
}
