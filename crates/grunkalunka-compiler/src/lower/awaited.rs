//! Shared await-operand inspection for storage planning and actual lowering.
use super::*;
use oxc_ast::ast::AwaitExpression;

impl Lowering<'_, '_> {
    pub(super) fn fulfilled_promise_slot(
        &self,
        expression: &Expression<'_>,
        depth: usize,
    ) -> Option<SlotId> {
        let Expression::Identifier(identifier) = closures::unwrapped(expression, depth)? else {
            return None;
        };
        let binding = numeric_binding(identifier, self.semantic, &self.bindings).ok()?;
        (self.slots[binding.slot.0].ty == Type::FulfilledPromise).then_some(binding.slot)
    }

    pub(super) fn fulfilled_promise_initializer(
        &mut self,
        expression: &Expression<'_>,
        region: &mut Region,
    ) -> Result<Option<ValueId>, String> {
        let payload = match closures::unwrapped_at(expression, 0) {
            Some((Expression::CallExpression(call), depth)) => {
                match intrinsic(call, self.semantic) {
                    Ok(Some((argument, false))) => Some((argument, depth + 1)),
                    _ => None,
                }
            }
            _ => None,
        };
        let (argument, depth) = payload.unwrap_or((expression, 0));
        if let Some(slot) = self.fulfilled_promise_slot(argument, depth) {
            let result = self.value(Type::FulfilledPromise);
            Self::push(region, expression, Op::Load { result, slot });
            return Ok(Some(result));
        }
        if payload.is_none() {
            return Ok(None);
        }
        let input = self
            .evaluate(argument, region, depth, evaluate::Mode::General)?
            .expect("fulfilled primitive payload");
        let result = self.value(Type::FulfilledPromise);
        Self::push(region, expression, Op::FulfillPromise { result, input });
        Ok(Some(result))
    }
}

pub(super) fn operand<'e, 'a>(
    awaited: &'e AwaitExpression<'a>,
    depth: usize,
    semantic: &Semantic<'_>,
) -> Result<(&'e Expression<'a>, usize, bool), String> {
    let mut argument = &awaited.argument;
    let mut argument_depth = depth + 1;
    while let Some(inner) = erased_inner(argument).or_else(|| {
        if let Expression::ParenthesizedExpression(wrapper) = argument {
            Some(&wrapper.expression)
        } else {
            None
        }
    }) {
        argument = inner;
        argument_depth += 1;
        if argument_depth > 128 {
            return Err(unsupported(awaited, "await operand nesting exceeds 128"));
        }
    }
    let mut rejected = false;
    if let Expression::CallExpression(call) = argument
        && let Some((payload, reject)) =
            intrinsic(call, semantic).map_err(|message| unsupported(awaited, message))?
    {
        rejected = reject;
        argument = payload;
        argument_depth += 1;
    }
    Ok((argument, argument_depth, rejected))
}

// Shared validation keeps stored constructors and directly awaited intrinsics
// aligned. Callers decide whether an unsupported shape is an await diagnostic
// or should continue through ordinary source admission.
fn intrinsic<'e, 'a>(
    call: &'e CallExpression<'a>,
    semantic: &Semantic<'_>,
) -> Result<Option<(&'e Expression<'a>, bool)>, &'static str> {
    let Expression::StaticMemberExpression(member) = &call.callee else {
        return Ok(None);
    };
    let Expression::Identifier(object) = &member.object else {
        return Ok(None);
    };
    if object.name != "Promise" {
        return Ok(None);
    }
    let unresolved = object.reference_id.get().is_some_and(|reference| {
        semantic
            .scoping()
            .get_reference(reference)
            .symbol_id()
            .is_none()
    });
    if !unresolved
        || call.optional
        || member.optional
        || call.type_arguments.is_some()
        || call.arguments.len() != 1
        || !matches!(member.property.name.as_str(), "resolve" | "reject")
    {
        return Err(
            "await Promise calls require one argument and the unshadowed resolve/reject intrinsic",
        );
    }
    let argument = call.arguments[0]
        .as_expression()
        .ok_or("spread await arguments are not admitted")?;
    Ok(Some((argument, member.property.name == "reject")))
}
