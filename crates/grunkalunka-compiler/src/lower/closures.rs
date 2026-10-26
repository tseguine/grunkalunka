//! Statically known local closures specialize into their caller. Captures name
//! existing slots, so copying this metadata never snapshots captured values.
use super::*;
use oxc_ast::{
    AstKind,
    ast::{ArrowFunctionBody, ArrowFunctionExpression},
};
use oxc_syntax::node::NodeId;

#[derive(Clone)]
pub(crate) struct LocalClosure {
    pub symbol: Option<SymbolId>,
    pub arrow: NodeId,
    pub captures: Vec<NumericBinding>,
    pub checked: bool,
    pub factory: bool,
}

pub(super) fn unwrapped<'e, 'a>(
    expression: &'e Expression<'a>,
    depth: usize,
) -> Option<&'e Expression<'a>> {
    unwrapped_at(expression, depth).map(|(expression, _)| expression)
}

fn unwrapped_at<'e, 'a>(
    mut expression: &'e Expression<'a>,
    mut depth: usize,
) -> Option<(&'e Expression<'a>, usize)> {
    while depth <= 128 {
        if let Some(inner) = erased_inner(expression) {
            expression = inner;
        } else if let Expression::ParenthesizedExpression(wrapper) = expression {
            expression = &wrapper.expression;
        } else {
            return Some((expression, depth));
        }
        depth += 1;
    }
    None
}

impl<'s, 'a> Lowering<'s, 'a> {
    pub(super) fn arrow(&self, id: NodeId) -> &'a ArrowFunctionExpression<'a> {
        let AstKind::ArrowFunctionExpression(arrow) = self.semantic.nodes().kind(id) else {
            unreachable!("closure metadata comes from the original arrow AST");
        };
        arrow
    }

    pub(super) fn closure_target(
        &self,
        expression: &Expression<'_>,
        depth: usize,
    ) -> Option<LocalClosure> {
        match unwrapped(expression, depth)? {
            Expression::ArrowFunctionExpression(arrow) => Some(LocalClosure {
                symbol: None,
                arrow: arrow.node_id.get(),
                captures: self.bindings.clone(),
                checked: false,
                factory: false,
            }),
            Expression::Identifier(identifier) => {
                let symbol = self
                    .semantic
                    .scoping()
                    .get_reference(identifier.reference_id.get()?)
                    .symbol_id()?;
                self.closures
                    .iter()
                    .find(|closure| closure.symbol == Some(symbol))
                    .cloned()
            }
            _ => None,
        }
    }

    pub(super) fn check_arrow(&self, arrow: &ArrowFunctionExpression<'_>) -> Result<(), String> {
        if arrow.r#async {
            return Err(unsupported(
                arrow,
                "only synchronous local arrows are admitted",
            ));
        }
        if arrow.params.rest.is_some() || arrow.params.items.len() > 1000 {
            return Err(unsupported(
                arrow,
                "local arrows require at most 1000 ordinary parameters",
            ));
        }
        for parameter in &arrow.params.items {
            if !matches!(parameter.pattern, BindingPattern::BindingIdentifier(_))
                || parameter.initializer.is_some()
                || parameter.optional
                || !parameter.decorators.is_empty()
                || parameter.accessibility.is_some()
                || parameter.readonly
                || parameter.r#override
            {
                return Err(unsupported(
                    parameter,
                    "local arrow parameters cannot have defaults, destructuring or modifiers",
                ));
            }
        }
        Ok(())
    }

    pub(super) fn bind_arrow_parameters(
        &mut self,
        arrow: &ArrowFunctionExpression<'_>,
        arguments: Vec<ValueId>,
        region: &mut Region,
    ) -> Result<(), String> {
        for (parameter, mut value) in arrow.params.items.iter().zip(arguments) {
            let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern else {
                unreachable!("checked arrow parameter");
            };
            let symbol = identifier
                .symbol_id
                .get()
                .ok_or("missing arrow parameter binding")?;
            let input_type = self.values[value.0];
            if let Some(expected) = self
                .callback_parameters
                .get(&arrow.node_id.get())
                .and_then(|parameters| parameters.get(&symbol))
                && input_type != Type::Closure(*expected)
            {
                return Err(unsupported(
                    parameter,
                    "callback argument requires a function with the inferred arity",
                ));
            }
            let ty = if matches!(input_type, Type::Closure(_)) {
                input_type
            } else if self.general_bindings.contains(&symbol) {
                Type::General
            } else {
                Type::Number
            };
            if ty == Type::General {
                let packed = self.value(ty);
                Self::push(
                    region,
                    parameter,
                    Op::Pack {
                        result: packed,
                        input: value,
                    },
                );
                value = packed;
            }
            let slot = SlotId(self.slots.len());
            self.slots.push(Slot {
                ty,
                owner: region.id,
                mutable: true,
                parameter: false,
                span: parameter.span(),
            });
            Self::push(region, parameter, Op::Initialize { slot, value });
            self.bindings.push(NumericBinding {
                symbol,
                slot,
                mutable: true,
            });
        }
        Ok(())
    }

    pub(super) fn check_local_closure(&mut self, closure: &mut LocalClosure) -> Result<(), String> {
        self.check_closure_at(closure, 0)
    }

    fn check_closure_at(
        &mut self,
        closure: &mut LocalClosure,
        factory_depth: usize,
    ) -> Result<(), String> {
        let arrow = self.arrow(closure.arrow);
        if factory_depth > 8 {
            return Err(unsupported(
                arrow,
                "at most 8 nested factory results are admitted",
            ));
        }
        self.check_arrow(arrow)?;
        // Validate unused bodies too, without running effects or publishing their
        // scratch values/slots in the real function's verified IR.
        let mut check = self.clone();
        check.function_result_depth = 0;
        check.bindings = closure.captures.clone();
        let mut region = check.region(arrow);
        let mut arguments = Vec::new();
        for parameter in &arrow.params.items {
            let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern else {
                unreachable!("checked parameter");
            };
            if let Some(arity) =
                self.callback_parameters
                    .get(&arrow.node_id.get())
                    .and_then(|parameters| {
                        parameters.get(&identifier.symbol_id.get().expect("parameter symbol"))
                    })
            {
                // A type-only input to disposable definition checking. This SSA
                // value has no runtime target and never reaches published IR;
                // actual calls construct/load and verify complete closure pairs.
                arguments.push(check.value(Type::Closure(*arity)));
                continue;
            }
            let value = check.value(Type::Number);
            Self::push(
                &mut region,
                parameter,
                Op::Constant {
                    result: value,
                    value: 0.0,
                },
            );
            arguments.push(value);
        }
        check.bind_arrow_parameters(arrow, arguments, &mut region)?;
        if let Some(expression) = arrow.get_expression() {
            if let Some(mut returned) = check.closure_target(expression, 0) {
                if !returned.checked {
                    check.check_closure_at(&mut returned, factory_depth + 1)?;
                }
                check.capture_cells(&returned, &mut region);
                closure.factory = true;
            } else if let Some(arity) = check.function_arity(expression, 0) {
                check.function_value(expression, &mut region, arity, 0)?;
            } else {
                check.evaluate(expression, &mut region, 0, evaluate::Mode::Number)?;
            }
        } else {
            region = check.arrow_block(arrow, 0)?;
        }
        // Include transitive references recorded by nested (even unused) arrows,
        // but exclude unrelated bindings and erased type-only references.
        let referenced = crate::ir::referenced_cells(&region);
        closure
            .captures
            .retain(|binding| referenced.contains(&binding.slot));
        // Source body costs are paid at its definition (including unused bodies),
        // never once per generated specialization.
        self.local_count = check.local_count;
        self.statement_count = check.statement_count;
        closure.checked = true;
        Ok(())
    }

    pub(super) fn check_local_call(
        &self,
        call: &CallExpression<'_>,
        closure: &LocalClosure,
    ) -> Result<(), String> {
        if call.optional || call.type_arguments.is_some() {
            return Err(unsupported(
                call,
                "optional and generic local calls are not admitted",
            ));
        }
        if call.arguments.len() != self.arrow(closure.arrow).params.items.len() {
            return Err(unsupported(
                call,
                "local call requires exactly the declared number of arguments",
            ));
        }
        Ok(())
    }

    pub(super) fn charge_closure_expansion(&mut self, node: &impl GetSpan) -> Result<(), String> {
        if self.closure_expansions >= 256 {
            return Err(unsupported(
                node,
                "at most 256 local arrow call expansions are admitted",
            ));
        }
        self.closure_expansions += 1;
        Ok(())
    }

    pub(super) fn factory_result(
        &mut self,
        expression: &Expression<'_>,
        region: &mut Region,
        depth: usize,
    ) -> Result<Option<LocalClosure>, String> {
        let Some((Expression::CallExpression(call), call_depth)) = unwrapped_at(expression, depth)
        else {
            return Ok(None);
        };
        let Some(mut factory) = self.closure_target(&call.callee, call_depth + 1) else {
            return Ok(None);
        };
        if !factory.checked {
            // Read-only shape recognition avoids validating/charging an ordinary
            // numeric IIFE twice. Actual validation uses its fresh parameter cells.
            if self
                .arrow(factory.arrow)
                .get_expression()
                .and_then(|body| self.closure_target(body, 0))
                .is_none()
            {
                return Ok(None);
            }
            self.check_local_closure(&mut factory)?;
            self.capture_cells(&factory, region);
        }
        if !factory.factory {
            return Ok(None);
        }
        self.check_local_call(call, &factory)?;
        self.charge_closure_expansion(call.as_ref())?;
        // Source arguments finish left-to-right before the creator's bindings
        // replace the caller's. Every specialization gets fresh parameter slots.
        let mut arguments = Vec::new();
        for argument in &call.arguments {
            let argument = argument.as_expression().ok_or_else(|| {
                unsupported(call.as_ref(), "spread call arguments are not admitted")
            })?;
            let value = if let Some(arity) = self.function_arity(argument, call_depth + 1) {
                self.callback_argument(argument, region, arity, call_depth + 1)?
            } else {
                self.expression(argument, region, call_depth + 1)?
            };
            arguments.push(value);
        }
        let arrow = self.arrow(factory.arrow);
        let previous = std::mem::replace(&mut self.bindings, factory.captures);
        let charging = std::mem::replace(&mut self.source_charging, false);
        let result = (|| {
            self.bind_arrow_parameters(arrow, arguments, region)?;
            let mut returned = self
                .closure_target(arrow.get_expression().expect("checked concise factory"), 0)
                .expect("checked function result");
            if !returned.checked {
                self.check_local_closure(&mut returned)?;
            }
            Ok(Some(returned))
        })();
        self.bindings = previous;
        self.source_charging = charging;
        result
    }

    pub(super) fn capture_cells(&self, closure: &LocalClosure, region: &mut Region) {
        if !closure.captures.is_empty() {
            Self::push(
                region,
                self.arrow(closure.arrow),
                Op::CaptureCells {
                    cells: closure
                        .captures
                        .iter()
                        .map(|binding| binding.slot)
                        .collect(),
                },
            );
        }
    }

    pub(super) fn arrow_block(
        &mut self,
        arrow: &ArrowFunctionExpression<'_>,
        parent_depth: usize,
    ) -> Result<Region, String> {
        // Statement lowering can reenter the expression cursor. Bound this native
        // recursion separately from concise-arrow's explicit continuation stack.
        if self.active_block_calls >= 8 {
            return Err(unsupported(
                arrow,
                "at most 8 nested block-arrow calls are admitted",
            ));
        }
        self.active_block_calls += 1;
        let outer = (
            self.return_type,
            self.in_loop_body,
            self.allow_await,
            self.inline_depth,
            self.statement_depth,
        );
        let bindings = self.bindings.len();
        let closures = self.closures.len();
        self.return_type = Type::Number;
        self.in_loop_body = false;
        self.allow_await = false;
        self.inline_depth = parent_depth;
        self.statement_depth = 0;
        let lowered = (|| {
            let ArrowFunctionBody::FunctionBody(body) = &arrow.body else {
                unreachable!("block arrow body")
            };
            let mut region = self.region(body.as_ref());
            for directive in &body.directives {
                self.charge_statement(directive)?;
            }
            if self.sequence(&body.statements, &mut region, 0)? != Completion::RETURN {
                return Err(unsupported(
                    arrow,
                    "block arrows must return a Number on every structural path",
                ));
            }
            Ok(region)
        })();
        (
            self.return_type,
            self.in_loop_body,
            self.allow_await,
            self.inline_depth,
            self.statement_depth,
        ) = outer;
        self.bindings.truncate(bindings);
        self.closures.truncate(closures);
        self.active_block_calls -= 1;
        lowered
    }
}
