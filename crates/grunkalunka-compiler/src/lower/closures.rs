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
    pub result: Type,
}

pub(super) fn unwrapped<'e, 'a>(
    expression: &'e Expression<'a>,
    depth: usize,
) -> Option<&'e Expression<'a>> {
    unwrapped_at(expression, depth).map(|(expression, _)| expression)
}

pub(super) fn unwrapped_at<'e, 'a>(
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
                result: Type::Number,
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
        self.check_arrow_kind(arrow, false)
    }

    fn check_arrow_kind(
        &self,
        arrow: &ArrowFunctionExpression<'_>,
        allow_async: bool,
    ) -> Result<(), String> {
        if arrow.r#async && !allow_async {
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
                    "callback argument requires a function with the inferred signature",
                ));
            }
            if self.promise_parameters.parameter(&symbol) == Some(Type::AsyncValue) {
                value = self.lift_async_input(parameter, value, region)?;
            }
            let input_type = self.values[value.0];
            if self.promise_parameters.is_fulfilled(&symbol) && input_type != Type::FulfilledPromise
            {
                return Err(unsupported(
                    parameter,
                    "fulfilled Promise parameters require Promise arguments",
                ));
            }
            if self.promise_parameters.parameter(&symbol) == Some(Type::Promise)
                && input_type != Type::Promise
            {
                return Err(unsupported(
                    parameter,
                    "pending Promise parameters require Promise identity arguments",
                ));
            }
            let ty = if matches!(
                input_type,
                Type::Closure(_) | Type::FulfilledPromise | Type::Promise | Type::AsyncValue
            ) {
                input_type
            } else if input_type == Type::General || self.general_bindings.contains(&symbol) {
                Type::General
            } else {
                Type::Number
            };
            if ty == Type::General && input_type != Type::General {
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
        // Each nested factory has its own disposable Lowering clone. Keep
        // the clones, their regions and the post-child capture continuation on
        // the heap; the eight-factory budget is a source limit, not a promise
        // about Rust frame size.
        enum Work<'s, 'a> {
            Enter {
                parent: Lowering<'s, 'a>,
                closure: LocalClosure,
                factory_depth: usize,
            },
            AfterChild {
                parent: Lowering<'s, 'a>,
                closure: LocalClosure,
                region: Region,
            },
        }

        let finish = |parent: &mut Lowering<'s, 'a>,
                      check: &Lowering<'s, 'a>,
                      closure: &mut LocalClosure,
                      region: &Region| {
            // Include transitive references recorded by nested (even unused)
            // arrows, but exclude erased type-only references.
            let referenced = crate::ir::referenced_cells(region);
            closure
                .captures
                .retain(|binding| referenced.contains(&binding.slot));
            // Definition costs are charged just once, including unused bodies.
            parent.local_count = check.local_count;
            parent.statement_count = check.statement_count;
            closure.checked = true;
        };

        let mut work = vec![Work::Enter {
            parent: self.clone(),
            closure: closure.clone(),
            factory_depth,
        }];
        let mut results = Vec::new();
        while let Some(task) = work.pop() {
            match task {
                Work::Enter {
                    mut parent,
                    mut closure,
                    factory_depth,
                } => {
                    let arrow = parent.arrow(closure.arrow);
                    if factory_depth > 8 {
                        return Err(unsupported(
                            arrow,
                            "at most 8 nested factory results are admitted",
                        ));
                    }
                    parent.check_arrow_kind(
                        arrow,
                        (parent.allow_await && factory_depth == 0)
                            || parent.allow_async_closures
                            || awaited::immediate_async_arrow(arrow, parent.semantic),
                    )?;
                    // Validate unused bodies too, without running effects or publishing their
                    // scratch values/slots in the real function's verified IR.
                    let mut check = parent.clone();
                    if arrow.r#async {
                        // AwaitCall owns the callee's return/rejection boundary and retains
                        // the caller's separate reaction job.
                        check.allow_await = true;
                    }
                    check.function_result_depth = 0;
                    check.bindings = closure.captures.clone();
                    let mut region = check.region(arrow);
                    let mut arguments = Vec::new();
                    for parameter in &arrow.params.items {
                        let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern
                        else {
                            unreachable!("checked parameter");
                        };
                        if let Some(arity) = parent
                            .callback_parameters
                            .get(&arrow.node_id.get())
                            .and_then(|parameters| {
                                parameters
                                    .get(&identifier.symbol_id.get().expect("parameter symbol"))
                            })
                        {
                            // A type-only input to disposable definition checking. This SSA
                            // value has no runtime target and never reaches published IR;
                            // actual calls construct/load and verify complete closure pairs.
                            arguments.push(check.value(Type::Closure(*arity)));
                            continue;
                        }
                        if let Some(ty) = parent
                            .promise_parameters
                            .parameter(&identifier.symbol_id.get().expect("parameter symbol"))
                        {
                            // Disposable type-only input, like callback pairs above; actual
                            // calls must provide a fully defined, verified Promise value.
                            arguments.push(check.value(ty));
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
                        if arrow.r#async
                            && (check.closure_target(expression, 0).is_some()
                                || check.function_arity(expression, 0).is_some())
                        {
                            return Err(unsupported(
                                expression,
                                "awaited async arrows require primitive results",
                            ));
                        }
                        if let Some(returned) = check.closure_target(expression, 0) {
                            if !returned.checked {
                                // Child factory checking resumes here after the descendant
                                // has updated the disposable lowering's source budgets.
                                work.push(Work::AfterChild {
                                    parent,
                                    closure,
                                    region,
                                });
                                work.push(Work::Enter {
                                    parent: check,
                                    closure: returned,
                                    factory_depth: factory_depth + 1,
                                });
                                continue;
                            }
                            check.capture_cells(&returned, &mut region);
                            closure.factory = true;
                        } else if let Some(arity) = check.function_arity(expression, 0) {
                            check.function_value(expression, &mut region, arity, 0)?;
                        } else {
                            closure.result = if !arrow.r#async
                                && (check
                                    .promise_parameters
                                    .mixed_results
                                    .contains(&arrow.node_id.get())
                                    || check.has_promise_choice(expression, 0)
                                        && !check.pending_promise_shape(expression, 0))
                            {
                                Type::AsyncValue
                            } else if check.pending_promise_shape(expression, 0) {
                                if arrow.r#async {
                                    Type::General
                                } else {
                                    Type::Promise
                                }
                            } else if check.settled_promise_shape_in(
                                expression,
                                0,
                                &std::collections::BTreeMap::new(),
                                0,
                            ) {
                                if arrow.r#async {
                                    return Err(unsupported(
                                        expression,
                                        "async Promise-valued returns require Promise adoption",
                                    ));
                                }
                                Type::SettledPromise
                            } else if check.fulfilled_promise_shape(expression, 0) {
                                if arrow.r#async {
                                    return Err(unsupported(
                                        expression,
                                        "async Promise-valued returns require Promise adoption",
                                    ));
                                }
                                Type::FulfilledPromise
                            } else if check.numeric_result_shape(expression, 0) {
                                Type::Number
                            } else {
                                Type::General
                            };
                            if arrow.r#async && closure.result == Type::General {
                                check.async_result(expression, &mut region, 0)?;
                            } else {
                                check.evaluate(
                                    expression,
                                    &mut region,
                                    0,
                                    if closure.result == Type::AsyncValue {
                                        evaluate::Mode::AsyncValue
                                    } else if closure.result == Type::Promise {
                                        evaluate::Mode::Pending
                                    } else if closure.result == Type::SettledPromise {
                                        evaluate::Mode::Settled
                                    } else if closure.result == Type::FulfilledPromise {
                                        evaluate::Mode::Promise
                                    } else if closure.result == Type::General {
                                        evaluate::Mode::General
                                    } else {
                                        evaluate::Mode::Number
                                    },
                                )?;
                            }
                        }
                    } else if let Some(arity) = check.block_function_arity(arrow, 0) {
                        if arrow.r#async {
                            return Err(unsupported(
                                arrow,
                                "awaited async arrows require primitive results",
                            ));
                        }
                        region = check.arrow_block_result(arrow, 0, Type::Closure(arity))?;
                        closure.factory = true;
                    } else {
                        let inferred =
                            check.arrow_scalar_result(arrow, &std::collections::BTreeMap::new());
                        closure.result =
                            if arrow.r#async && awaited::owns_finally(arrow, parent.semantic) {
                                Type::General
                            } else if inferred == Type::Promise {
                                inferred
                            } else if check.arrow_fulfilled_promise_result(arrow) {
                                Type::FulfilledPromise
                            } else {
                                inferred
                            };
                        region = check.arrow_block_result(arrow, 0, closure.result)?;
                    }
                    finish(&mut parent, &check, &mut closure, &region);
                    results.push((parent, closure));
                }
                Work::AfterChild {
                    mut parent,
                    mut closure,
                    mut region,
                } => {
                    let (mut check, returned) = results.pop().expect("checked nested factory");
                    check.capture_cells(&returned, &mut region);
                    closure.factory = true;
                    finish(&mut parent, &check, &mut closure, &region);
                    results.push((parent, closure));
                }
            }
        }
        let (parent, checked) = results.pop().expect("checked root closure");
        self.local_count = parent.local_count;
        self.statement_count = parent.statement_count;
        *closure = checked;
        Ok(())
    }

    pub(super) fn check_local_call(
        &self,
        call: &CallExpression<'_>,
        closure: &LocalClosure,
    ) -> Result<(), String> {
        if call.optional {
            return Err(unsupported(call, "optional local calls are not admitted"));
        }
        if call.arguments.len() > 1000 {
            return Err(unsupported(
                call,
                "at most 1000 ordinary local call arguments are admitted",
            ));
        }
        if invocation::arguments(call).len() < self.arrow(closure.arrow).params.items.len() {
            return Err(unsupported(
                call,
                "local call is missing declared arguments",
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
        let Some(mut factory) = self.closure_target(
            invocation::callee(call),
            invocation::depth(call, call_depth + 1),
        ) else {
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
        if !factory.factory || self.arrow(factory.arrow).get_expression().is_none() {
            return Ok(None);
        }
        self.check_local_call(call, &factory)?;
        self.charge_closure_expansion(call.as_ref())?;
        // Source arguments finish left-to-right before the creator's bindings
        // replace the caller's. Every specialization gets fresh parameter slots.
        let mut arguments = Vec::new();
        for (index, argument) in call.arguments.iter().enumerate() {
            let offset = invocation::offset(call);
            let argument = argument.as_expression().ok_or_else(|| {
                unsupported(call.as_ref(), "spread call arguments are not admitted")
            })?;
            if index < offset {
                self.evaluate(argument, region, call_depth + 1, evaluate::Mode::Discard)?;
                continue;
            }
            let index = index - offset;
            if index >= self.arrow(factory.arrow).params.items.len() {
                self.evaluate(argument, region, call_depth + 1, evaluate::Mode::Discard)?;
                continue;
            }
            let value = if let Some(arity) = self.function_arity(argument, call_depth + 1) {
                self.callback_argument(argument, region, arity, call_depth + 1)?
            } else {
                let mode = self.local_scalar_argument_mode(argument, call_depth + 1);
                self.evaluate(argument, region, call_depth + 1, mode)?
                    .expect("factory scalar argument")
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

    pub(super) fn arrow_block_result(
        &mut self,
        arrow: &ArrowFunctionExpression<'_>,
        parent_depth: usize,
        result_type: Type,
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
        self.return_type = result_type;
        self.in_loop_body = false;
        self.allow_await = arrow.r#async && outer.2;
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
            let completion = self.sequence(&body.statements, &mut region, 0)?;
            if matches!(result_type, Type::General | Type::AsyncValue)
                && completion.normal
                && !completion.breaks
                && !completion.continues
                && (arrow.r#async || !completion.throws)
            {
                self.return_undefined(arrow, &mut region);
            } else if completion != Completion::RETURN
                && !(arrow.r#async
                    && result_type == Type::General
                    && completion.returns_or_throws())
            {
                return Err(unsupported(
                    arrow,
                    if matches!(result_type, Type::Closure(_)) {
                        "block arrows must return a function on every structural path"
                    } else if result_type == Type::FulfilledPromise {
                        "block arrows must return a fulfilled Promise on every structural path"
                    } else if result_type == Type::General {
                        "block arrows must return a primitive value on every structural path"
                    } else {
                        "block arrows must return a Number on every structural path"
                    },
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
