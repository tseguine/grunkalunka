//! Shared await-operand inspection for storage planning and actual lowering.
use super::*;
use oxc_ast::ast::{Argument, AwaitExpression};

// This query only selects the existing fulfilled representation. Actual body
// lowering must still prove a primitive result, without suspension or rejection.
// A nested function owns its own awaits/throws; it cannot suspend this creator.
pub(super) fn eager_async_arrow(
    arrow: &oxc_ast::ast::ArrowFunctionExpression<'_>,
    semantic: &Semantic<'_>,
) -> bool {
    immediate_async_arrow(arrow, semantic) && !owns_throw(arrow, semantic)
}

pub(super) fn immediate_async_arrow(
    arrow: &oxc_ast::ast::ArrowFunctionExpression<'_>,
    semantic: &Semantic<'_>,
) -> bool {
    arrow.r#async
        && !semantic.nodes().iter().any(|node| {
            matches!(node.kind(), oxc_ast::AstKind::AwaitExpression(_))
                && semantic
                    .nodes()
                    .ancestor_kinds(node.id())
                    .find_map(|kind| match kind {
                        oxc_ast::AstKind::ArrowFunctionExpression(owner) => {
                            Some(Some(owner.node_id.get()))
                        }
                        oxc_ast::AstKind::Function(_) => Some(None),
                        _ => None,
                    })
                    == Some(Some(arrow.node_id.get()))
        })
}

pub(super) fn owns_throw(
    arrow: &oxc_ast::ast::ArrowFunctionExpression<'_>,
    semantic: &Semantic<'_>,
) -> bool {
    semantic.nodes().iter().any(|node| {
        matches!(node.kind(), oxc_ast::AstKind::ThrowStatement(_))
            && semantic
                .nodes()
                .ancestor_kinds(node.id())
                .find_map(|kind| match kind {
                    oxc_ast::AstKind::ArrowFunctionExpression(owner) => {
                        Some(Some(owner.node_id.get()))
                    }
                    oxc_ast::AstKind::Function(_) => Some(None),
                    _ => None,
                })
                == Some(Some(arrow.node_id.get()))
    })
}

pub(super) struct Operand<'e, 'a> {
    pub argument: Option<&'e Expression<'a>>,
    pub depth: usize,
    pub rejected: bool,
    pub extras: &'e [Argument<'a>],
}

pub(super) struct PrivateCall<'e, 'a> {
    pub call: &'e CallExpression<'a>,
    pub depth: usize,
    pub prefixes: Vec<(&'e Expression<'a>, usize)>,
}

// Only inspect the awaited result path; prefixes remain original expressions
// and execute before callee/arguments in the evaluator's explicit cursor.
pub(super) fn private_call<'e, 'a>(
    mut expression: &'e Expression<'a>,
    mut depth: usize,
) -> Option<PrivateCall<'e, 'a>> {
    let mut prefixes = Vec::new();
    loop {
        let (node, at) = closures::unwrapped_at(expression, depth)?;
        match node {
            Expression::CallExpression(call) => {
                return Some(PrivateCall {
                    call,
                    depth: at,
                    prefixes,
                });
            }
            Expression::SequenceExpression(sequence) => {
                let (last, before) = sequence.expressions.split_last()?;
                prefixes.extend(before.iter().map(|expression| (expression, at + 1)));
                expression = last;
                depth = at + 1;
            }
            _ => return None,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Intrinsic<'e, 'a> {
    pub argument: Option<&'e Expression<'a>>,
    pub rejected: bool,
    pub extras: &'e [Argument<'a>],
}

impl Lowering<'_, '_> {
    // Opt pending or mixed snapshot/scalar selections into the await context.
    // Already accepted uniform snapshots keep their original emission path.
    pub(super) fn composed_await_selection(
        &self,
        expression: &Expression<'_>,
        depth: usize,
    ) -> bool {
        let mut pending = vec![(expression, depth)];
        let mut selected = false;
        let mut asynchronous = false;
        let mut snapshot = false;
        while let Some((expression, depth)) = pending.pop() {
            let Some((expression, depth)) = closures::unwrapped_at(expression, depth) else {
                return false;
            };
            match expression {
                Expression::ConditionalExpression(conditional) => {
                    selected = true;
                    pending.push((&conditional.consequent, depth + 1));
                    pending.push((&conditional.alternate, depth + 1));
                }
                Expression::SequenceExpression(sequence) => {
                    if let Some(last) = sequence.expressions.last() {
                        pending.push((last, depth + 1));
                    }
                }
                Expression::CallExpression(call) => {
                    asynchronous |= self
                        .function_arity(&call.callee, depth + 1)
                        .is_some_and(|id| self.closure_signatures.borrow().is_async(id));
                }
                _ => (),
            }
            snapshot |= self.fulfilled_promise_shape(expression, depth)
                || self.settled_promise_shape_in(
                    expression,
                    depth,
                    &std::collections::BTreeMap::new(),
                    0,
                )
                || matches!(expression, Expression::CallExpression(call) if intrinsic(call, self.semantic).ok().flatten().is_some());
        }
        selected
            && (asynchronous
                || (snapshot
                    && !self.fulfilled_promise_shape(expression, depth)
                    && !self.settled_promise_shape_in(
                        expression,
                        depth,
                        &std::collections::BTreeMap::new(),
                        0,
                    )))
    }

    pub(super) fn settled_promise_shape_in(
        &self,
        expression: &Expression<'_>,
        depth: usize,
        bindings: &std::collections::BTreeMap<SymbolId, Option<usize>>,
        calls: usize,
    ) -> bool {
        if calls >= 8 {
            return false;
        }
        settled_shape(
            expression,
            depth,
            |call, depth| {
                self.function_arity_in(&call.callee, depth + 1, bindings, calls + 1)
                    .and_then(|signature| self.closure_signatures.borrow().result(signature))
            },
            |expression, depth| self.fulfilled_promise_shape_in(expression, depth, bindings, calls),
        )
    }

    pub(super) fn arrow_fulfilled_promise_result(
        &self,
        arrow: &oxc_ast::ast::ArrowFunctionExpression<'_>,
    ) -> bool {
        !arrow.r#async
            && storage::arrow_returns(arrow, self.semantic).is_some_and(|results| {
                !results.is_empty()
                    && results
                        .into_iter()
                        .all(|result| self.fulfilled_promise_shape(result, 0))
            })
    }

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
        if !self.fulfilled_promise_shape(expression, 0) {
            return Ok(None);
        }
        self.evaluate(expression, region, 0, evaluate::Mode::Promise)
    }

    // Readonly positive type query. Actual evaluation still checks every source
    // expression, target, payload, intrinsic and depth in the owned cursor.
    pub(super) fn fulfilled_promise_shape(
        &self,
        expression: &Expression<'_>,
        depth: usize,
    ) -> bool {
        self.fulfilled_promise_shape_in(expression, depth, &std::collections::BTreeMap::new(), 0)
    }

    pub(super) fn fulfilled_promise_shape_in(
        &self,
        expression: &Expression<'_>,
        depth: usize,
        bindings: &std::collections::BTreeMap<SymbolId, Option<usize>>,
        calls: usize,
    ) -> bool {
        fulfilled_shape(
            expression,
            self.semantic,
            depth,
            |identifier| {
                numeric_binding(identifier, self.semantic, &self.bindings)
                    .ok()
                    .map(|binding| self.slots[binding.slot.0].ty == Type::FulfilledPromise)
                    .or_else(|| {
                        identifier
                            .reference_id
                            .get()
                            .and_then(|reference| {
                                self.semantic.scoping().get_reference(reference).symbol_id()
                            })
                            .filter(|symbol| self.promise_parameters.contains(symbol))
                            .map(|_| true)
                    })
            },
            |call, depth| {
                if let Some(closure) = self.closure_target(&call.callee, depth + 1) {
                    let arrow = self.arrow(closure.arrow);
                    if arrow.r#async {
                        return eager_async_arrow(arrow, self.semantic).then(Vec::new);
                    }
                    if closure.checked {
                        return (closure.result == Type::FulfilledPromise).then(Vec::new);
                    }
                    let results = storage::arrow_returns(arrow, self.semantic)?;
                    return (!results.is_empty()).then_some(results);
                }
                if calls >= 8 {
                    return None;
                }
                let mut callee = &call.callee;
                let mut depth = depth + 1;
                while let Some((node, unwrapped_depth)) = closures::unwrapped_at(callee, depth) {
                    if let Some(signature) =
                        self.function_arity_in(node, unwrapped_depth, bindings, calls + 1)
                    {
                        return (self.closure_signatures.borrow().result(signature)
                            == Some(Type::FulfilledPromise))
                        .then(Vec::new);
                    }
                    let Expression::Identifier(identifier) = node else {
                        return None;
                    };
                    let symbol = identifier.reference_id.get().and_then(|reference| {
                        self.semantic.scoping().get_reference(reference).symbol_id()
                    })?;
                    if bindings.contains_key(&symbol)
                        || numeric_binding(identifier, self.semantic, &self.bindings).is_ok()
                    {
                        return None;
                    }
                    callee = source_initializer(symbol, self.semantic)?;
                    depth = unwrapped_depth + 1;
                }
                None
            },
        )
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

// A positive kind query shared with callback definition checking. At least one
// leaf must be settled; other leaves can be explicitly fulfilled. It never runs
// conditions, prefixes or calls; the evaluator validates every such expression.
pub(super) fn settled_shape(
    expression: &Expression<'_>,
    depth: usize,
    call_kind: impl Fn(&CallExpression<'_>, usize) -> Option<Type>,
    fulfilled: impl Fn(&Expression<'_>, usize) -> bool,
) -> bool {
    let mut pending = vec![(expression, depth)];
    let mut settled = false;
    while let Some((expression, depth)) = pending.pop() {
        let Some((expression, depth)) = closures::unwrapped_at(expression, depth) else {
            return false;
        };
        match expression {
            Expression::CallExpression(call) => match call_kind(call, depth) {
                Some(Type::SettledPromise) => settled = true,
                Some(Type::FulfilledPromise) => (),
                _ if fulfilled(expression, depth) => (),
                _ => return false,
            },
            Expression::SequenceExpression(sequence) => {
                let Some(last) = sequence.expressions.last() else {
                    return false;
                };
                pending.push((last, depth + 1));
            }
            Expression::ConditionalExpression(conditional) => {
                pending.push((&conditional.consequent, depth + 1));
                pending.push((&conditional.alternate, depth + 1));
            }
            _ if fulfilled(expression, depth) => (),
            _ => return false,
        }
    }
    settled
}

// Shared positive result-kind proof for callback inference and actual lowering.
// Resolver results are original source expressions, never rewritten/evaluated AST.
pub(super) fn fulfilled_shape<'e, 'a>(
    expression: &'e Expression<'a>,
    semantic: &Semantic<'a>,
    depth: usize,
    binding: impl Fn(&IdentifierReference<'a>) -> Option<bool>,
    call_results: impl Fn(&CallExpression<'a>, usize) -> Option<Vec<&'e Expression<'a>>>,
) -> bool {
    let mut pending = vec![(expression, depth)];
    while let Some((expression, depth)) = pending.pop() {
        let Some((expression, depth)) = closures::unwrapped_at(expression, depth) else {
            return false;
        };
        let identifier = match expression {
            Expression::Identifier(identifier) => Some(&**identifier),
            Expression::AssignmentExpression(assignment) => assignment
                .left
                .as_simple_assignment_target()
                .and_then(|target| target_identifier(target, depth).ok().flatten()),
            _ => None,
        };
        if let Some(identifier) = identifier {
            if let Some(promise) = binding(identifier) {
                if !promise {
                    return false;
                }
                continue;
            }
            // Definition checking has not executed the arrow's declarations.
            // An original initializer may prove its logical result kind;
            // evaluation must still publish/validate the binding in order.
            let Some(symbol) = identifier
                .reference_id
                .get()
                .and_then(|reference| semantic.scoping().get_reference(reference).symbol_id())
            else {
                return false;
            };
            let initializer = source_initializer(symbol, semantic);
            let Some(initializer) = initializer else {
                return false;
            };
            pending.push((initializer, depth + 1));
            continue;
        }
        match expression {
            Expression::CallExpression(call)
                if intrinsic(call, semantic)
                    .ok()
                    .flatten()
                    .is_some_and(|intrinsic| !intrinsic.rejected) => {}
            Expression::CallExpression(call) => {
                let Some(results) = call_results(call, depth) else {
                    return false;
                };
                pending.extend(results.into_iter().map(|result| (result, depth + 1)));
            }
            Expression::SequenceExpression(sequence) => {
                let Some(last) = sequence.expressions.last() else {
                    return false;
                };
                pending.push((last, depth + 1));
            }
            Expression::ConditionalExpression(conditional) => {
                pending.push((&conditional.alternate, depth + 1));
                pending.push((&conditional.consequent, depth + 1));
            }
            Expression::LogicalExpression(logical) => {
                pending.push((&logical.right, depth + 1));
                pending.push((&logical.left, depth + 1));
            }
            _ => return false,
        }
    }
    true
}

fn source_initializer<'s, 'a>(
    symbol: SymbolId,
    semantic: &'s Semantic<'a>,
) -> Option<&'s Expression<'a>> {
    semantic.nodes().iter().find_map(|node| {
        let oxc_ast::AstKind::VariableDeclaration(declaration) = node.kind() else {
            return None;
        };
        declaration.declarations.iter().find_map(|local| {
            let BindingPattern::BindingIdentifier(binding) = &local.id else {
                return None;
            };
            (binding.symbol_id.get() == Some(symbol))
                .then_some(local.init.as_ref())
                .flatten()
        })
    })
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
pub(super) fn intrinsic<'e, 'a>(
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
