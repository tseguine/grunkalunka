//! Selection-local Promise/General joins; resolve only after testing the raw value.
use super::*;

impl Lowering<'_, '_> {
    pub(super) fn has_promise_choice(&self, expression: &Expression<'_>, depth: usize) -> bool {
        let mut work = vec![(expression, depth)];
        while let Some((node, depth)) = work.pop() {
            let Some((node, depth)) = closures::unwrapped_at(node, depth) else {
                return false;
            };
            if self.pending_promise_shape(node, depth) {
                return true;
            }
            match node {
                Expression::ConditionalExpression(c) => {
                    work.push((&c.consequent, depth + 1));
                    work.push((&c.alternate, depth + 1));
                }
                Expression::LogicalExpression(l) => {
                    work.push((&l.left, depth + 1));
                    work.push((&l.right, depth + 1));
                }
                Expression::SequenceExpression(s) => {
                    if let Some(last) = s.expressions.last() {
                        work.push((last, depth + 1));
                    }
                }
                _ => (),
            }
        }
        false
    }

    pub(super) fn async_value_condition(
        &mut self,
        node: &Expression<'_>,
        input: ValueId,
        operator: LogicalOperator,
        region: &mut Region,
    ) -> ValueId {
        let condition = self.value(Type::Predicate);
        Self::push(
            region,
            node,
            Op::AsyncValueIsPromise {
                result: condition,
                input,
            },
        );
        let mut consequent = self.region(node);
        let truthy = self.value(Type::Predicate);
        Self::push(
            &mut consequent,
            node,
            Op::Boolean {
                result: truthy,
                value: true,
            },
        );
        consequent.exit = Exit::Yield(truthy);
        let mut alternate = self.region(node);
        let primitive = self.value(Type::General);
        Self::push(
            &mut alternate,
            node,
            Op::AsyncValuePrimitive {
                result: primitive,
                input,
            },
        );
        let test = self.value(Type::Predicate);
        if operator == LogicalOperator::Coalesce {
            let nullish = self.value(Type::Predicate);
            Self::push(
                &mut alternate,
                node,
                Op::GeneralIsNullish {
                    result: nullish,
                    input: primitive,
                },
            );
            Self::push(
                &mut alternate,
                node,
                Op::Not {
                    result: test,
                    input: nullish,
                },
            );
        } else {
            Self::push(
                &mut alternate,
                node,
                Op::GeneralTruthy {
                    result: test,
                    input: primitive,
                },
            );
        }
        alternate.exit = Exit::Yield(test);
        let result = self.value(Type::Predicate);
        Self::push(
            region,
            node,
            Op::If {
                result: Some(result),
                condition,
                consequent,
                alternate,
            },
        );
        result
    }

    pub(super) fn resolve_async_value(
        &mut self,
        node: &Expression<'_>,
        input: ValueId,
        region: &mut Region,
    ) -> ValueId {
        let condition = self.value(Type::Predicate);
        Self::push(
            region,
            node,
            Op::AsyncValueIsPromise {
                result: condition,
                input,
            },
        );
        let mut consequent = self.region(node);
        let promise = self.value(Type::Promise);
        Self::push(
            &mut consequent,
            node,
            Op::AsyncValuePromise {
                result: promise,
                input,
            },
        );
        let adopted = self.value(Type::General);
        Self::push(
            &mut consequent,
            node,
            Op::AdoptPromise {
                result: adopted,
                input: promise,
            },
        );
        consequent.exit = Exit::Yield(adopted);
        let mut alternate = self.region(node);
        let primitive = self.value(Type::General);
        Self::push(
            &mut alternate,
            node,
            Op::AsyncValuePrimitive {
                result: primitive,
                input,
            },
        );
        alternate.exit = Exit::Yield(primitive);
        let result = self.value(Type::General);
        Self::push(
            region,
            node,
            Op::If {
                result: Some(result),
                condition,
                consequent,
                alternate,
            },
        );
        result
    }
}
