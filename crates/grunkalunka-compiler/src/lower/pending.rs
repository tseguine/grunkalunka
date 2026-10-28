//! Positive identity facts. Queries do not evaluate source expressions.
use super::*;
use std::collections::BTreeMap;

impl Lowering<'_, '_> {
    pub(super) fn async_result(
        &mut self,
        expression: &Expression<'_>,
        region: &mut Region,
        depth: usize,
    ) -> Result<ValueId, String> {
        if self.pending_promise_shape(expression, depth) {
            let input = self
                .evaluate(expression, region, depth, evaluate::Mode::Pending)?
                .expect("pending async return");
            let result = self.value(Type::General);
            Self::push(region, expression, Op::AdoptPromise { result, input });
            Ok(result)
        } else {
            Ok(self
                .evaluate(expression, region, depth, evaluate::Mode::General)?
                .expect("async primitive return"))
        }
    }
    pub(super) fn pending_promise_shape(&self, expression: &Expression<'_>, depth: usize) -> bool {
        self.pending_promise_shape_in(expression, depth, &BTreeMap::new(), 0)
    }

    pub(super) fn pending_promise_shape_in(
        &self,
        expression: &Expression<'_>,
        depth: usize,
        bindings: &BTreeMap<SymbolId, Option<usize>>,
        calls: usize,
    ) -> bool {
        if calls >= 8 {
            return false;
        }
        let mut work = vec![(expression, depth)];
        while let Some((node, depth)) = work.pop() {
            if depth > 128 {
                return false;
            }
            if let Some(inner) = erased_inner(node) {
                work.push((inner, depth + 1));
                continue;
            }
            match node {
                Expression::ParenthesizedExpression(p) => work.push((&p.expression, depth + 1)),
                Expression::Identifier(id) => {
                    if !numeric_binding(id, self.semantic, &self.bindings)
                        .ok()
                        .map(|b| self.slots[b.slot.0].ty == Type::Promise)
                        .unwrap_or_else(|| {
                            id.reference_id
                                .get()
                                .and_then(|r| self.semantic.scoping().get_reference(r).symbol_id())
                                .is_some_and(|s| self.promise_parameters.values.contains(&s))
                        })
                    {
                        return false;
                    }
                }
                Expression::CallExpression(call) => {
                    if let Some(intrinsic) = awaited::intrinsic(call, self.semantic).ok().flatten()
                        && (self.promise_parameters.identities || !intrinsic.rejected)
                    {
                        if self.promise_parameters.identities {
                            continue;
                        }
                        let Some(argument) = intrinsic.argument else {
                            return false;
                        };
                        work.push((argument, depth + 1));
                        continue;
                    }
                    if !self
                        .function_arity_in(&call.callee, depth + 1, bindings, calls + 1)
                        .is_some_and(|s| {
                            let signatures = self.closure_signatures.borrow();
                            signatures.is_async(s) || signatures.result(s) == Some(Type::Promise)
                        })
                    {
                        return false;
                    }
                }
                Expression::ConditionalExpression(c) => {
                    work.push((&c.consequent, depth + 1));
                    work.push((&c.alternate, depth + 1));
                }
                Expression::LogicalExpression(l) => {
                    work.push((&l.left, depth + 1));
                    work.push((&l.right, depth + 1));
                }
                Expression::SequenceExpression(s) => {
                    let Some(last) = s.expressions.last() else {
                        return false;
                    };
                    work.push((last, depth + 1));
                }
                Expression::AssignmentExpression(a)
                    if matches!(
                        a.operator,
                        AssignmentOperator::Assign
                            | AssignmentOperator::LogicalAnd
                            | AssignmentOperator::LogicalOr
                            | AssignmentOperator::LogicalNullish
                    ) =>
                {
                    let Some(id) = a
                        .left
                        .as_simple_assignment_target()
                        .and_then(|t| target_identifier(t, depth).ok().flatten())
                    else {
                        return false;
                    };
                    if !numeric_binding(id, self.semantic, &self.bindings)
                        .ok()
                        .is_some_and(|b| self.slots[b.slot.0].ty == Type::Promise)
                    {
                        return false;
                    }
                    work.push((&a.right, depth + 1));
                }
                _ => return false,
            }
        }
        true
    }
}
