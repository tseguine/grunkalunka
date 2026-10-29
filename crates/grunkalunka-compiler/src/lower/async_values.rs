//! Promise/General selection joins; resolve returns or compare their raw variants.
use super::*;

impl Lowering<'_, '_> {
    pub(super) fn number_from_general_or_async_value(
        &mut self,
        node: &Expression<'_>,
        input: ValueId,
        region: &mut Region,
    ) -> ValueId {
        if self.values[input.0] == Type::General {
            let result = self.value(Type::Number);
            Self::push(region, node, Op::NumberFromGeneral { result, input });
            return result;
        }
        assert_eq!(self.values[input.0], Type::AsyncValue);
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
        // Built-in Promise conversion is nonnumeric in this closed source domain.
        // This is a value conversion, never settlement/adoption or handling.
        let nan = self.value(Type::Number);
        Self::push(
            &mut consequent,
            node,
            Op::Constant {
                result: nan,
                value: f64::NAN,
            },
        );
        consequent.exit = Exit::Yield(nan);
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
        let number = self.value(Type::Number);
        Self::push(
            &mut alternate,
            node,
            Op::NumberFromGeneral {
                result: number,
                input: primitive,
            },
        );
        alternate.exit = Exit::Yield(number);
        let result = self.value(Type::Number);
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

    pub(super) fn lift_async_input(
        &mut self,
        node: &impl GetSpan,
        mut input: ValueId,
        region: &mut Region,
    ) -> Result<ValueId, String> {
        match self.values[input.0] {
            Type::AsyncValue => return Ok(input),
            Type::Number | Type::Predicate => {
                let result = self.value(Type::General);
                Self::push(region, node, Op::Pack { result, input });
                input = result;
            }
            Type::General | Type::Promise => (),
            _ => {
                return Err(unsupported(
                    node,
                    "mixed inputs require a represented primitive or Promise",
                ));
            }
        }
        let result = self.value(Type::AsyncValue);
        Self::push(region, node, Op::ToAsyncValue { result, input });
        Ok(result)
    }

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

    pub(super) fn async_value_slot(
        &self,
        expression: &Expression<'_>,
        depth: usize,
    ) -> Option<SlotId> {
        let (Expression::Identifier(id), _) = closures::unwrapped_at(expression, depth)? else {
            return None;
        };
        let binding = numeric_binding(id, self.semantic, &self.bindings).ok()?;
        (self.slots[binding.slot.0].ty == Type::AsyncValue).then_some(binding.slot)
    }

    pub(super) fn async_value_assignment_slot(
        &self,
        expression: &Expression<'_>,
        depth: usize,
    ) -> Option<SlotId> {
        let (Expression::AssignmentExpression(assignment), depth) =
            closures::unwrapped_at(expression, depth)?
        else {
            return None;
        };
        let id = assignment
            .left
            .as_simple_assignment_target()
            .and_then(|target| target_identifier(target, depth).ok().flatten())?;
        let binding = numeric_binding(id, self.semantic, &self.bindings).ok()?;
        (self.slots[binding.slot.0].ty == Type::AsyncValue).then_some(binding.slot)
    }

    pub(super) fn has_promise_choice(&self, expression: &Expression<'_>, depth: usize) -> bool {
        let mut work = vec![(expression, depth)];
        while let Some((node, depth)) = work.pop() {
            let Some((node, depth)) = closures::unwrapped_at(node, depth) else {
                return false;
            };
            if self.pending_promise_shape(node, depth)
                || self.async_value_slot(node, depth).is_some()
                || self.async_value_assignment_slot(node, depth).is_some()
                || self.mixed_call(node)
            {
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

    pub(super) fn mixed_call(&self, node: &Expression<'_>) -> bool {
        matches!(node, Expression::CallExpression(call)
            if self.promise_parameters.mixed_calls.contains(&call.node_id.get()))
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

    // Promise.resolve preserves an existing identity. Unlike async return
    // adoption, this conversion neither awaits nor handles the selected Promise.
    pub(super) fn resolve_promise_value(
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
        consequent.exit = Exit::Yield(promise);
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
        let snapshot = self.value(Type::FulfilledPromise);
        Self::push(
            &mut alternate,
            node,
            Op::FulfillPromise {
                result: snapshot,
                input: primitive,
            },
        );
        let wrapped = self.value(Type::Promise);
        Self::push(
            &mut alternate,
            node,
            Op::MaterializePromise {
                result: wrapped,
                input: snapshot,
            },
        );
        alternate.exit = Exit::Yield(wrapped);
        let result = self.value(Type::Promise);
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
        node: &impl GetSpan,
        input: ValueId,
        region: &mut Region,
    ) -> ValueId {
        self.consume_async_value(node, input, None, region)
    }

    pub(super) fn await_async_value(
        &mut self,
        node: &Expression<'_>,
        input: ValueId,
        site: usize,
        region: &mut Region,
    ) -> ValueId {
        self.consume_async_value(node, input, Some(site), region)
    }

    fn consume_async_value(
        &mut self,
        node: &impl GetSpan,
        input: ValueId,
        await_site: Option<usize>,
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
        if let Some(site) = await_site {
            Self::push_await(
                &mut consequent,
                node,
                Some(site),
                Op::AwaitPromise {
                    result: adopted,
                    input: promise,
                },
            );
        } else {
            Self::push(
                &mut consequent,
                node,
                Op::AdoptPromise {
                    result: adopted,
                    input: promise,
                },
            );
        }
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
        let completion = if let Some(site) = await_site {
            let result = self.value(Type::General);
            Self::push_await(
                &mut alternate,
                node,
                Some(site),
                Op::Await {
                    result,
                    input: primitive,
                    rejected: false,
                },
            );
            result
        } else {
            primitive
        };
        alternate.exit = Exit::Yield(completion);
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
