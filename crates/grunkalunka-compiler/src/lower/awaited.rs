//! Shared await-operand inspection for storage planning and actual lowering.
use super::*;
use oxc_ast::{
    AstKind,
    ast::{ArrowFunctionExpression, AwaitExpression},
};

// Concise async bodies inline into their immediately awaiting caller. Until
// callee rejection has its own boundary, only fulfillment paths are admitted.
// Nested functions own their awaits and undergo their own body validation.
pub(super) fn check_fulfilled_body(
    arrow: &ArrowFunctionExpression<'_>,
    semantic: &Semantic<'_>,
) -> Result<(), String> {
    for node in semantic.nodes().iter() {
        let AstKind::AwaitExpression(awaited) = node.kind() else {
            continue;
        };
        let owner = semantic
            .nodes()
            .ancestor_kinds(node.id())
            .find_map(|kind| match kind {
                AstKind::ArrowFunctionExpression(owner) => Some(Some(owner.node_id.get())),
                AstKind::Function(_) => Some(None),
                _ => None,
            });
        if owner == Some(Some(arrow.node_id.get())) && operand(awaited, 0, semantic)?.2 {
            return Err(unsupported(
                awaited,
                "concise async bodies currently require fulfilled awaits",
            ));
        }
    }
    Ok(())
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
        && let Expression::StaticMemberExpression(member) = &call.callee
        && let Expression::Identifier(object) = &member.object
        && object.name == "Promise"
    {
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
            return Err(unsupported(
                awaited,
                "await Promise calls require one argument and the unshadowed resolve/reject intrinsic",
            ));
        }
        rejected = member.property.name == "reject";
        argument = call.arguments[0]
            .as_expression()
            .ok_or_else(|| unsupported(awaited, "spread await arguments are not admitted"))?;
        argument_depth += 1;
    }
    Ok((argument, argument_depth, rejected))
}
