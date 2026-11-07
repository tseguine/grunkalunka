//! Statically known local closures specialize into their caller. Captures name
//! existing slots, so copying this metadata never snapshots captured values.
use super::*;
use oxc_ast::{
    AstKind,
    ast::{ArrowFunctionBody, ArrowFunctionExpression},
};
use oxc_syntax::node::NodeId;

/// The caller's lexical bindings and source-charge mode are restored only
/// after an invoked function's parameters/body have finished. A heap
/// continuation retains this token while a factory body is suspended.
pub(super) struct FactoryBindings {
    previous: Vec<NumericBinding>,
    charging: bool,
}

/// The source-ordered classification shared by concise and block-bodied local
/// factories. Classification is read-only; a source continuation schedules
/// the corresponding expression or callback without rerunning it.
#[derive(Clone, Copy)]
pub(super) enum FactoryArgumentKind {
    Discard,
    Callback(usize),
    Scalar(evaluate::Mode),
}

/// A concise factory that returns a statically known local closure. Selection
/// owns no function-result admission: unlike general function-valued results,
/// this path evaluates arguments and returns closure metadata for its caller.
pub(super) struct PreparedConciseFactory<'n, 'a> {
    call: &'n CallExpression<'a>,
    call_depth: usize,
    closure: LocalClosure,
}

/// Shape recognition retains the original call and depth while its independently
/// selected closure is checked. This stage owns no function/result admission.
pub(super) struct ConciseFactoryCandidate<'n, 'a> {
    call: &'n CallExpression<'a>,
    call_depth: usize,
}

impl<'n, 'a> PreparedConciseFactory<'n, 'a> {
    pub(super) fn call(&self) -> (&'n CallExpression<'a>, usize) {
        (self.call, self.call_depth)
    }

    pub(super) fn arrow(&self) -> NodeId {
        self.closure.arrow
    }
}

/// Disposable checking owns its cloned state and original scratch Region.
/// Only definition costs and checked closure metadata return to the caller;
/// the borrowed signature table retains its existing shared identity.
pub(super) struct PreparedClosureCheck<'s, 'a> {
    pub(super) lowering: Box<Lowering<'s, 'a>>,
    pub(super) region: Region,
}

/// Body selection retains the original checking order while the source cursor
/// owns the actual lowering in its disposable child.
pub(super) enum ClosureCheckBody<'a> {
    Returned(LocalClosure),
    Expression {
        source: &'a Expression<'a>,
        mode: evaluate::Mode,
    },
    Block {
        result_type: Type,
        factory: bool,
    },
}

#[derive(Clone)]
pub(crate) struct LocalClosure {
    pub symbol: Option<SymbolId>,
    pub arrow: NodeId,
    pub captures: Vec<NumericBinding>,
    pub checked: bool,
    pub factory: bool,
    pub result: Type,
}

/// Context retained with a suspended block-arrow source-body request and
/// restored as one typed value on success and error.
struct BlockArrowCheckpoint {
    source: source_control::SourceContext,
    allow_await: bool,
    inline_depth: usize,
    visible_bindings: usize,
    visible_closures: usize,
}

impl BlockArrowCheckpoint {
    fn capture(lowering: &Lowering<'_, '_>) -> Self {
        Self {
            source: lowering.source_context(),
            allow_await: lowering.allow_await,
            inline_depth: lowering.inline_depth,
            visible_bindings: lowering.bindings.len(),
            visible_closures: lowering.closures.len(),
        }
    }

    fn restore(self, lowering: &mut Lowering<'_, '_>) {
        lowering.restore_source_context(self.source);
        lowering.allow_await = self.allow_await;
        lowering.inline_depth = self.inline_depth;
        lowering.bindings.truncate(self.visible_bindings);
        lowering.closures.truncate(self.visible_closures);
    }
}

/// The suspended block-arrow activation owns the function mode and lexical
/// restoration checkpoint. The body itself is a separate owned request, so
/// source lowering yields without borrowing the evaluator stack.
pub(super) struct BlockArrowActivation {
    arrow: NodeId,
    result_type: Type,
    checkpoint: BlockArrowCheckpoint,
}

/// Owned source region plus the original body AST for a LowerBody task.
/// No evaluator work/Region is borrowed.
pub(super) struct ArrowBodyRequest<'n, 'a> {
    pub(super) statements: &'n [Statement<'a>],
    pub(super) region: Region,
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
            let ty = if input_type.is_managed()
                || matches!(
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

    /// Preserve definition admission, scratch allocation and parameter order
    /// before the checking child enters the body. No caller-owned IR is
    /// published by this preparation or by dropping its result on an error.
    pub(super) fn prepare_closure_check(
        &self,
        closure: &LocalClosure,
        factory_depth: usize,
    ) -> Result<PreparedClosureCheck<'s, 'a>, String> {
        let arrow = self.arrow(closure.arrow);
        if factory_depth > 8 {
            return Err(unsupported(
                arrow,
                "at most 8 nested factory results are admitted",
            ));
        }
        self.check_arrow_kind(
            arrow,
            (self.allow_await && factory_depth == 0)
                || self.allow_async_closures
                || awaited::immediate_async_arrow(arrow, self.semantic),
        )?;
        // Validate unused bodies too, without running effects or publishing their
        // scratch values/slots in the real function's verified IR.
        let mut check = Box::new(self.clone());
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
            if let Some(ty) = self
                .promise_parameters
                .parameter(&identifier.symbol_id.get().expect("parameter symbol"))
            {
                // Disposable type-only input, like callback pairs above; actual
                // calls must provide a fully defined, verified Promise value.
                arguments.push(check.value(ty));
                continue;
            }
            if let Some(ty) = self
                .managed_kinds
                .inputs
                .get(&identifier.symbol_id.get().expect("parameter symbol"))
            {
                arguments.push(check.value(*ty));
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
        Ok(PreparedClosureCheck {
            lowering: check,
            region,
        })
    }

    /// Classify a prepared checking body without lowering it. Signature facts
    /// retain their existing shared interning and left-to-right query order.
    pub(super) fn prepare_closure_check_body(
        &self,
        closure: &mut LocalClosure,
    ) -> Result<ClosureCheckBody<'a>, String> {
        let arrow = self.arrow(closure.arrow);
        if let Some(expression) = arrow.get_expression() {
            if arrow.r#async
                && (self.closure_target(expression, 0).is_some()
                    || self.function_arity(expression, 0).is_some())
            {
                return Err(unsupported(
                    expression,
                    "awaited async arrows require primitive results",
                ));
            }
            if let Some(returned) = self.closure_target(expression, 0) {
                return Ok(ClosureCheckBody::Returned(returned));
            }
            if let Some(arity) = self.function_arity(expression, 0) {
                return Ok(ClosureCheckBody::Expression {
                    source: expression,
                    mode: evaluate::Mode::Function(arity),
                });
            }
            closure.result = if !arrow.r#async
                && (self
                    .promise_parameters
                    .mixed_results
                    .contains(&arrow.node_id.get())
                    || self.has_promise_choice(expression, 0)
                        && !self.pending_promise_shape(expression, 0))
            {
                Type::AsyncValue
            } else if self.pending_promise_shape(expression, 0) {
                if arrow.r#async {
                    Type::General
                } else {
                    Type::Promise
                }
            } else if self.settled_promise_shape_in(
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
            } else if self.fulfilled_promise_shape(expression, 0) {
                if arrow.r#async {
                    return Err(unsupported(
                        expression,
                        "async Promise-valued returns require Promise adoption",
                    ));
                }
                Type::FulfilledPromise
            } else if !arrow.r#async
                && let Some(ty) = self.managed_kinds.results.get(&arrow.node_id.get())
            {
                *ty
            } else if self.numeric_result_shape(expression, 0) {
                Type::Number
            } else {
                Type::General
            };
            let mode = if arrow.r#async && closure.result == Type::General {
                evaluate::Mode::AsyncReturn
            } else if closure.result.is_managed() {
                evaluate::Mode::managed(closure.result)
            } else if closure.result == Type::AsyncValue {
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
            };
            Ok(ClosureCheckBody::Expression {
                source: expression,
                mode,
            })
        } else if let Some(arity) = self.block_function_arity(arrow, 0) {
            if arrow.r#async {
                return Err(unsupported(
                    arrow,
                    "awaited async arrows require primitive results",
                ));
            }
            Ok(ClosureCheckBody::Block {
                result_type: Type::Closure(arity),
                factory: true,
            })
        } else {
            let inferred = self.arrow_scalar_result(arrow, &std::collections::BTreeMap::new());
            closure.result = if arrow.r#async && awaited::owns_finally(arrow, self.semantic) {
                Type::General
            } else if inferred == Type::Promise {
                inferred
            } else if self.arrow_fulfilled_promise_result(arrow) {
                Type::FulfilledPromise
            } else {
                inferred
            };
            Ok(ClosureCheckBody::Block {
                result_type: closure.result,
                factory: false,
            })
        }
    }

    /// Successful checking publishes only source costs and closure metadata.
    /// Scratch identities, generated operations and lifted bodies remain owned
    /// by the disposable child, while signature interning stays shared.
    pub(super) fn finish_closure_check(
        &mut self,
        check: &Lowering<'s, 'a>,
        closure: &mut LocalClosure,
        region: &Region,
    ) {
        // Include transitive references recorded by nested (even unused) arrows,
        // but exclude unrelated bindings and erased type-only references.
        let referenced = crate::ir::referenced_cells(region);
        closure
            .captures
            .retain(|binding| referenced.contains(&binding.slot));
        // Source body costs are paid at its definition (including unused bodies),
        // never once per generated specialization.
        self.local_count = check.local_count;
        self.statement_count = check.statement_count;
        closure.checked = true;
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

    /// Fresh factory formals are published only after caller arguments have
    /// finished in the caller's lexical environment.
    pub(super) fn enter_factory_bindings(
        &mut self,
        captures: Vec<NumericBinding>,
    ) -> FactoryBindings {
        FactoryBindings {
            previous: std::mem::replace(&mut self.bindings, captures),
            charging: std::mem::replace(&mut self.source_charging, false),
        }
    }

    pub(super) fn restore_factory_bindings(&mut self, checkpoint: FactoryBindings) {
        self.bindings = checkpoint.previous;
        self.source_charging = checkpoint.charging;
    }

    pub(super) fn classify_factory_argument(
        &self,
        call: &CallExpression<'_>,
        arrow: NodeId,
        index: usize,
        expression: &Expression<'_>,
        depth: usize,
    ) -> FactoryArgumentKind {
        let offset = invocation::offset(call);
        if index < offset || index - offset >= self.arrow(arrow).params.items.len() {
            return FactoryArgumentKind::Discard;
        }
        if let Some(signature) = self.function_arity(expression, depth) {
            FactoryArgumentKind::Callback(signature)
        } else {
            FactoryArgumentKind::Scalar(self.local_arrow_argument_mode(
                arrow,
                index - offset,
                expression,
                depth,
            ))
        }
    }

    /// Select without checking or charging. A known-local consumer must retain
    /// its own original closure snapshot if it may fall through after this
    /// metadata stage: checking this candidate does not check that snapshot.
    pub(super) fn select_concise_factory<'n, 'ast>(
        &self,
        expression: &'n Expression<'ast>,
        depth: usize,
    ) -> Option<(ConciseFactoryCandidate<'n, 'ast>, LocalClosure)> {
        let Some((Expression::CallExpression(call), call_depth)) = unwrapped_at(expression, depth)
        else {
            return None;
        };
        let factory = self.closure_target(
            invocation::callee(call),
            invocation::depth(call, call_depth + 1),
        )?;
        // Read-only shape recognition avoids validating/charging an ordinary
        // numeric IIFE twice. Actual validation uses its fresh parameter cells.
        if !factory.checked
            && self
                .arrow(factory.arrow)
                .get_expression()
                .and_then(|body| self.closure_target(body, 0))
                .is_none()
        {
            return None;
        }
        Some((ConciseFactoryCandidate { call, call_depth }, factory))
    }

    /// A newly checked candidate has already published its capture operation.
    /// Preserve the remaining factory flag, call validation and expansion order;
    /// a successful None may therefore follow checking costs/capture effects.
    pub(super) fn finish_concise_factory_preparation<'n, 'ast>(
        &mut self,
        candidate: ConciseFactoryCandidate<'n, 'ast>,
        factory: LocalClosure,
    ) -> Result<Option<PreparedConciseFactory<'n, 'ast>>, String> {
        if !factory.factory || self.arrow(factory.arrow).get_expression().is_none() {
            return Ok(None);
        }
        let ConciseFactoryCandidate { call, call_depth } = candidate;
        self.check_local_call(call, &factory)?;
        self.charge_closure_expansion(call)?;
        Ok(Some(PreparedConciseFactory {
            call,
            call_depth,
            closure: factory,
        }))
    }

    /// Install fresh factory parameters and select the returned metadata. On
    /// success the caller owns restoration across any returned-definition check;
    /// setup failure restores immediately, before a continuation can be stored.
    pub(super) fn begin_concise_factory_result(
        &mut self,
        factory: PreparedConciseFactory<'_, '_>,
        arguments: Vec<ValueId>,
        region: &mut Region,
    ) -> Result<(FactoryBindings, LocalClosure), String> {
        let arrow = self.arrow(factory.closure.arrow);
        let checkpoint = self.enter_factory_bindings(factory.closure.captures);
        let result = (|| {
            self.bind_arrow_parameters(arrow, arguments, region)?;
            let returned = self
                .closure_target(arrow.get_expression().expect("checked concise factory"), 0)
                .expect("checked function result");
            Ok(returned)
        })();
        match result {
            Ok(returned) => Ok((checkpoint, returned)),
            Err(error) => {
                self.restore_factory_bindings(checkpoint);
                Err(error)
            }
        }
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

    /// Prepare the source block with its ordered directive admission. The
    /// source cursor retains this request while its body is suspended.
    #[inline(never)]
    pub(super) fn prepare_arrow_block_body<'n>(
        &mut self,
        arrow: &'n ArrowFunctionExpression<'a>,
    ) -> Result<ArrowBodyRequest<'n, 'a>, String>
    where
        'a: 'n,
    {
        let ArrowFunctionBody::FunctionBody(body) = &arrow.body else {
            unreachable!("block arrow body")
        };
        let region = self.region(body.as_ref());
        for directive in &body.directives {
            self.charge_statement(directive)?;
        }
        Ok(ArrowBodyRequest {
            statements: &body.statements,
            region,
        })
    }

    /// Enter a block-arrow's own source context and return the owned body
    /// request. The source dispatcher has *not* been entered on this path.
    /// This preserves the existing eight-active-block admission boundary.
    #[inline(never)]
    pub(super) fn begin_arrow_block_activation<'n>(
        &mut self,
        arrow: &'n ArrowFunctionExpression<'a>,
        parent_depth: usize,
        result_type: Type,
    ) -> Result<(BlockArrowActivation, ArrowBodyRequest<'n, 'a>), String>
    where
        'a: 'n,
    {
        if self.active_block_calls >= 8 {
            return Err(unsupported(
                arrow,
                "at most 8 nested block-arrow calls are admitted",
            ));
        }
        self.active_block_calls += 1;
        let checkpoint = BlockArrowCheckpoint::capture(self);
        self.return_type = result_type;
        self.in_loop_body = false;
        self.allow_await = arrow.r#async && checkpoint.allow_await;
        self.inline_depth = parent_depth;
        self.statement_depth = 0;
        match self.prepare_arrow_block_body(arrow) {
            Ok(request) => Ok((
                BlockArrowActivation {
                    arrow: arrow.node_id.get(),
                    result_type,
                    checkpoint,
                },
                request,
            )),
            Err(error) => {
                checkpoint.restore(self);
                self.active_block_calls -= 1;
                Err(error)
            }
        }
    }

    /// Finish a suspended block-arrow body on normal or diagnostic completion.
    /// The caller owns the Region and Completion; no native evaluator frame is
    /// needed here. Every exit restores the exact prior function/lexical modes.
    #[inline(never)]
    pub(super) fn finish_arrow_block_activation(
        &mut self,
        activation: BlockArrowActivation,
        region: Region,
        completion: Result<Completion, String>,
    ) -> Result<Region, String> {
        let result = completion.and_then(|completion| {
            let arrow = self.arrow(activation.arrow);
            self.complete_arrow_block_body(arrow, activation.result_type, region, completion)
        });
        self.abort_arrow_block_activation(activation);
        result
    }

    /// Unwind an activation when the common source cursor returns a diagnostic
    /// before its suspended source-body frame completes. Nested activations
    /// must be restored in the reverse order in which they were begun.
    pub(super) fn abort_arrow_block_activation(&mut self, activation: BlockArrowActivation) {
        activation.checkpoint.restore(self);
        self.active_block_calls -= 1;
    }

    /// Complete the source body's owned Region and Completion after its
    /// continuation restores the block-arrow activation.
    #[inline(never)]
    pub(super) fn complete_arrow_block_body(
        &mut self,
        arrow: &ArrowFunctionExpression<'_>,
        result_type: Type,
        mut region: Region,
        completion: Completion,
    ) -> Result<Region, String> {
        if matches!(
            result_type,
            Type::General | Type::AsyncValue | Type::ManagedValue
        ) && completion.normal
            && !completion.breaks
            && !completion.continues
            && (arrow.r#async || !completion.throws)
        {
            self.return_undefined(arrow, &mut region);
        } else if completion != Completion::RETURN
            && !(arrow.r#async && result_type == Type::General && completion.returns_or_throws())
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
    }
}

#[cfg(test)]
mod checkpoint_tests {
    #[test]
    fn shared_factory_argument_modes_preserve_callbacks_and_order_on_small_stacks() {
        for body in [
            "const make=x=>()=>x+a;return make(b)();",
            "const make=(x,y)=>()=>x+y+a;return make(a,b)();",
            "const make=(fn,x)=>()=>fn(x);return make(y=>y+b,a)();",
            "let n=a;const make=x=>()=>n+x;return make((n+=b))()+n;",
        ] {
            for typescript in [false, true] {
                let source = format!("export function run(a,b){{{body}}}");
                let compiled = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile(&source, typescript))
                    .expect("2 MiB factory argument thread")
                    .join()
                    .expect("factory arguments must not overflow the native stack");
                assert!(compiled.is_ok(), "{body}: {:?}", compiled.err());
            }
        }
    }

    #[test]
    fn factory_binding_checkpoint_restores_caller_context_on_small_stack() {
        for (body, admitted) in [
            ("const id=fn=>fn;const f=id(x=>x+a);return f(b);", true),
            (
                "const make=x=>{return ()=>x+a;};const f=make(b);return f();",
                true,
            ),
            (
                "let n=a;const make=x=>{return ()=>n+x;};const f=make(b);return f()+n;",
                true,
            ),
            (
                "const make=x=>{if(x)return ()=>x+a;};const f=make(b);return f();",
                false,
            ),
        ] {
            for typescript in [false, true] {
                let source = format!("export function run(a,b){{{body}}}");
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile(&source, typescript))
                    .expect("2 MiB factory checkpoint thread")
                    .join()
                    .expect("factory binding restoration must not overflow");
                assert_eq!(result.is_ok(), admitted, "{body}: {:?}", result.err());
            }
        }
    }

    #[test]
    fn block_arrow_checkpoint_preserves_source_modes_on_small_native_stack() {
        for (body, admitted) in [
            (
                "const get=()=>{return ()=>a;};const f=get();return f();",
                true,
            ),
            (
                "let make=x=>{if(x)return ()=>x+a;return ()=>x-b;};return make(a)();",
                true,
            ),
            ("let make=x=>{if(x)return ()=>a;};return a;", false),
        ] {
            for typescript in [false, true] {
                let source = format!("export function run(a,b){{{body}}}");
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile(&source, typescript))
                    .expect("small-stack block-arrow checkpoint thread")
                    .join()
                    .expect("block-arrow source reentry must not overflow native stack");
                assert_eq!(result.is_ok(), admitted, "{body}: {:?}", result.err());
            }
        }
    }
}
