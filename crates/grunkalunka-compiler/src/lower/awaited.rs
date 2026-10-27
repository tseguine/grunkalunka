//! Shared await-operand inspection for storage planning and actual lowering.
use super::*;
use oxc_ast::ast::{Argument, AwaitExpression};

pub(super) struct Operand<'e, 'a> {
    pub argument: Option<&'e Expression<'a>>,
    pub depth: usize,
    pub rejected: bool,
    pub extras: &'e [Argument<'a>],
}

#[derive(Clone, Copy)]
struct Intrinsic<'e, 'a> {
    argument: Option<&'e Expression<'a>>,
    rejected: bool,
    extras: &'e [Argument<'a>],
}

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
                    Ok(Some(intrinsic)) if !intrinsic.rejected => Some((intrinsic, depth + 1)),
                    _ => None,
                }
            }
            _ => None,
        };
        let (intrinsic, depth) = payload.unwrap_or((
            Intrinsic {
                argument: Some(expression),
                rejected: false,
                extras: &[],
            },
            0,
        ));
        let argument = intrinsic.argument;
        let result = if let Some(slot) =
            argument.and_then(|argument| self.fulfilled_promise_slot(argument, depth))
        {
            let result = self.value(Type::FulfilledPromise);
            Self::push(region, expression, Op::Load { result, slot });
            result
        } else {
            if payload.is_none() {
                return Ok(None);
            }
            let input = if let Some(argument) = argument {
                self.evaluate(argument, region, depth, evaluate::Mode::General)?
                    .expect("fulfilled primitive payload")
            } else {
                self.undefined_value(expression, region)
            };
            let result = self.value(Type::FulfilledPromise);
            Self::push(region, expression, Op::FulfillPromise { result, input });
            result
        };
        // The first value is already a snapshot before later arguments run.
        for extra in intrinsic.extras {
            self.evaluate(
                extra.as_expression().expect("validated ordinary argument"),
                region,
                depth,
                evaluate::Mode::Discard,
            )?;
        }
        Ok(Some(result))
    }

    pub(super) fn undefined_value(
        &mut self,
        node: &Expression<'_>,
        region: &mut Region,
    ) -> ValueId {
        let result = self.value(Type::General);
        Self::push(
            region,
            node,
            Op::Nullish {
                result,
                value: ir::Nullish::Undefined,
            },
        );
        result
    }
}

pub(super) fn operand<'e, 'a>(
    awaited: &'e AwaitExpression<'a>,
    depth: usize,
    semantic: &Semantic<'_>,
) -> Result<Operand<'e, 'a>, String> {
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
    if let Expression::CallExpression(call) = argument
        && let Some(intrinsic) =
            intrinsic(call, semantic).map_err(|message| unsupported(awaited, message))?
    {
        return Ok(Operand {
            argument: intrinsic.argument,
            depth: argument_depth + 1,
            rejected: intrinsic.rejected,
            extras: intrinsic.extras,
        });
    }
    Ok(Operand {
        argument: Some(argument),
        depth: argument_depth,
        rejected: false,
        extras: &[],
    })
}

// Shared validation keeps stored constructors and directly awaited intrinsics
// aligned. Callers decide whether an unsupported shape is an await diagnostic
// or should continue through ordinary source admission.
fn intrinsic<'e, 'a>(
    call: &'e CallExpression<'a>,
    semantic: &Semantic<'_>,
) -> Result<Option<Intrinsic<'e, 'a>>, &'static str> {
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
        || !matches!(member.property.name.as_str(), "resolve" | "reject")
    {
        return Err(
            "await Promise calls require the unshadowed nonoptional nongeneric resolve/reject intrinsic",
        );
    }
    if call.arguments.len() > 1000 {
        return Err("at most 1000 ordinary Promise intrinsic arguments are admitted");
    }
    if call
        .arguments
        .iter()
        .any(|argument| argument.as_expression().is_none())
    {
        return Err("spread await arguments are not admitted");
    }
    let argument = call.arguments.first().map(|argument| {
        argument
            .as_expression()
            .expect("validated ordinary argument")
    });
    Ok(Some(Intrinsic {
        argument,
        rejected: member.property.name == "reject",
        extras: call.arguments.get(1..).unwrap_or(&[]),
    }))
}
