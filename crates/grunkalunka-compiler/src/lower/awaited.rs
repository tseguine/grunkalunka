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
pub(super) struct Intrinsic<'e, 'a> {
    pub argument: Option<&'e Expression<'a>>,
    pub rejected: bool,
    pub extras: &'e [Argument<'a>],
}

impl Lowering<'_, '_> {
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
            },
            |call, depth| {
                if let Some(closure) = self.closure_target(&call.callee, depth + 1) {
                    let arrow = self.arrow(closure.arrow);
                    if arrow.r#async {
                        return None;
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
                self.function_arity_in(&call.callee, depth + 1, bindings, calls + 1)
                    .filter(|signature| {
                        self.closure_signatures.borrow().result(*signature)
                            == Some(Type::FulfilledPromise)
                    })
                    .map(|_| Vec::new())
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
            let initializer = semantic.nodes().iter().find_map(|node| {
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
            });
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
