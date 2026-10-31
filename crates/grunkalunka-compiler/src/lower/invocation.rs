//! Original-source view of a represented Function.prototype.call invocation.
//! Intrinsic recognition still inspects the original call, not this projection.
use super::*;
use oxc_ast::ast::Argument;

fn method<'e, 'a>(call: &'e CallExpression<'a>) -> Option<(&'e Expression<'a>, usize)> {
    let (callee, depth) = closures::unwrapped_at(&call.callee, 0)?;
    let Expression::StaticMemberExpression(member) = callee else {
        return None;
    };
    (member.property.name == "call" && !member.optional).then_some((&member.object, depth + 1))
}

pub(super) fn callee<'e, 'a>(call: &'e CallExpression<'a>) -> &'e Expression<'a> {
    method(call).map_or(&call.callee, |(callee, _)| callee)
}

pub(super) fn depth(call: &CallExpression<'_>, depth: usize) -> usize {
    depth + method(call).map_or(0, |(_, extra)| extra)
}

pub(super) fn offset(call: &CallExpression<'_>) -> usize {
    usize::from(is_method(call) && !call.arguments.is_empty())
}

pub(super) fn is_method(call: &CallExpression<'_>) -> bool {
    method(call).is_some()
}

pub(super) fn arguments<'e, 'a>(call: &'e CallExpression<'a>) -> &'e [Argument<'a>] {
    &call.arguments[offset(call)..]
}
