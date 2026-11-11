//! Shared numeric read-modify-write operator planning for locals and properties.
use super::*;
use oxc_ast::ast::AssignmentExpression;

#[derive(Clone, Copy)]
pub(super) enum NumericCompound {
    Binary(Binary),
    Power(literal_power::LiteralPower),
}

impl NumericCompound {
    pub(super) fn from_source(source: &AssignmentExpression<'_>, depth: usize) -> Option<Self> {
        if source.operator == AssignmentOperator::Exponential {
            return literal_power::LiteralPower::from_source(&source.right, depth + 1)
                .map(Self::Power);
        }
        Some(Self::Binary(match source.operator {
            AssignmentOperator::Addition => Binary::Add,
            AssignmentOperator::Subtraction => Binary::Subtract,
            AssignmentOperator::Multiplication => Binary::Multiply,
            AssignmentOperator::Division => Binary::Divide,
            AssignmentOperator::Remainder => Binary::Remainder,
            AssignmentOperator::BitwiseAnd => Binary::BitAnd,
            AssignmentOperator::BitwiseOR => Binary::BitOr,
            AssignmentOperator::BitwiseXOR => Binary::BitXor,
            AssignmentOperator::ShiftLeft => Binary::ShiftLeft,
            AssignmentOperator::ShiftRight => Binary::ShiftRight,
            AssignmentOperator::ShiftRightZeroFill => Binary::ShiftRightUnsigned,
            _ => return None,
        }))
    }

    pub(super) fn is_addition(self) -> bool {
        matches!(self, Self::Binary(Binary::Add))
    }
}

impl Lowering<'_, '_> {
    pub(super) fn emit_compound(
        &mut self,
        node: &impl GetSpan,
        region: &mut Region,
        op: NumericCompound,
        left: ValueId,
        right: ValueId,
    ) -> ValueId {
        match op {
            NumericCompound::Power(power) => self.emit_literal_power(node, region, left, power),
            NumericCompound::Binary(operator) => {
                let result = self.value(Type::Number);
                Self::push(
                    region,
                    node,
                    Op::Binary {
                        result,
                        operator,
                        left,
                        right,
                    },
                );
                result
            }
        }
    }
}
