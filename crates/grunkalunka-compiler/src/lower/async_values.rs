//! Promise/General selection joins; resolve returns or compare their raw variants.
use super::*;

impl Lowering<'_, '_> {
    pub(super) fn compare_async_values(
        &mut self,
        node: &Expression<'_>,
        left: ValueId,
        right: ValueId,
        loose: bool,
        negated: bool,
        region: &mut Region,
    ) -> ValueId {
        let left_promise = self.value(Type::Predicate);
        Self::push(
            region,
            node,
            Op::AsyncValueIsPromise {
                result: left_promise,
                input: left,
            },
        );
        let right_promise = self.value(Type::Predicate);
        Self::push(
            region,
            node,
            Op::AsyncValueIsPromise {
                result: right_promise,
                input: right,
            },
        );

        let mut promises = self.region(node);
        let promise_left = self.value(Type::Promise);
        Self::push(
            &mut promises,
            node,
            Op::AsyncValuePromise {
                result: promise_left,
                input: left,
            },
        );
        let promise_right = self.value(Type::Promise);
        Self::push(
            &mut promises,
            node,
            Op::AsyncValuePromise {
                result: promise_right,
                input: right,
            },
        );
        let equal_promises = self.value(Type::Predicate);
        Self::push(
            &mut promises,
            node,
            Op::PromiseIdentityEqual {
                result: equal_promises,
                left: promise_left,
                right: promise_right,
                negated,
            },
        );
        promises.exit = Exit::Yield(equal_promises);

        let mut primitives = self.region(node);
        let primitive_left = self.value(Type::General);
        Self::push(
            &mut primitives,
            node,
            Op::AsyncValuePrimitive {
                result: primitive_left,
                input: left,
            },
        );
        let primitive_right = self.value(Type::General);
        Self::push(
            &mut primitives,
            node,
            Op::AsyncValuePrimitive {
                result: primitive_right,
                input: right,
            },
        );
        let equal_primitives = self.value(Type::Predicate);
        let compare = if loose {
            Op::PrimitiveLooseEqual {
                result: equal_primitives,
                left: primitive_left,
                right: primitive_right,
                negated,
            }
        } else {
            Op::PrimitiveStrictEqual {
                result: equal_primitives,
                left: primitive_left,
                right: primitive_right,
                negated,
            }
        };
        Self::push(&mut primitives, node, compare);
        primitives.exit = Exit::Yield(equal_primitives);

        // Cross-kind comparisons are false for admitted non-string primitives
        // and intrinsic Errors. General String admission must extend loose ==.
        let mut cross_left = self.region(node);
        let unequal_left = self.value(Type::Predicate);
        Self::push(
            &mut cross_left,
            node,
            Op::Boolean {
                result: unequal_left,
                value: negated,
            },
        );
        cross_left.exit = Exit::Yield(unequal_left);
        let mut cross_right = self.region(node);
        let unequal_right = self.value(Type::Predicate);
        Self::push(
            &mut cross_right,
            node,
            Op::Boolean {
                result: unequal_right,
                value: negated,
            },
        );
        cross_right.exit = Exit::Yield(unequal_right);

        let mut promise_case = self.region(node);
        let selected_promise = self.value(Type::Predicate);
        Self::push(
            &mut promise_case,
            node,
            Op::If {
                result: Some(selected_promise),
                condition: right_promise,
                consequent: promises,
                alternate: cross_left,
            },
        );
        promise_case.exit = Exit::Yield(selected_promise);
        let mut primitive_case = self.region(node);
        let selected_primitive = self.value(Type::Predicate);
        Self::push(
            &mut primitive_case,
            node,
            Op::If {
                result: Some(selected_primitive),
                condition: right_promise,
                consequent: cross_right,
                alternate: primitives,
            },
        );
        primitive_case.exit = Exit::Yield(selected_primitive);
        let result = self.value(Type::Predicate);
        Self::push(
            region,
            node,
            Op::If {
                result: Some(result),
                condition: left_promise,
                consequent: promise_case,
                alternate: primitive_case,
            },
        );
        result
    }

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
