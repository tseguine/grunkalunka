//! Returned-function factory transitions on the shared source scheduler.
use super::*;

impl<'s, 'ast> Lowering<'s, 'ast> {
    pub(super) fn start_factory_arguments<'n>(
        &mut self,
        state: FactoryExpression<'n, 'ast>,
        factory: function_values::PreparedLocalFactory<'n, 'ast>,
        expression_depth: usize,
        cursor: &mut Cursor<'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        let (call, expression_depth) = match &factory {
            function_values::PreparedLocalFactory::Block(_) => {
                let Expression::CallExpression(call) = state.node else {
                    unreachable!("prepared block factory is a direct call");
                };
                (call.as_ref(), expression_depth)
            }
            function_values::PreparedLocalFactory::Concise(factory) => factory.call(),
        };
        self.advance_factory_arguments(
            Box::new(FactoryArguments {
                state,
                factory,
                call,
                expression_depth,
                index: 0,
                values: Vec::new(),
            }),
            cursor,
        )
    }

    fn advance_factory_arguments<'n>(
        &mut self,
        mut arguments: Box<FactoryArguments<'n, 'ast>>,
        cursor: &mut Cursor<'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        if arguments.index < arguments.call.arguments.len() {
            let call = arguments.call;
            let depth = arguments.expression_depth + 1;
            // Classify only the next argument, after all preceding effects.
            // In particular, a later spread must not preempt an earlier error.
            let prepared = (|| {
                let source = call.arguments[arguments.index]
                    .as_expression()
                    .ok_or_else(|| unsupported(call, "spread call arguments are not admitted"))?;
                let kind = self.classify_factory_argument(
                    call,
                    arguments.factory.arrow(),
                    arguments.index,
                    source,
                    depth,
                );
                let mode = match kind {
                    closures::FactoryArgumentKind::Discard => evaluate::Mode::Discard,
                    closures::FactoryArgumentKind::Scalar(mode) => mode,
                    closures::FactoryArgumentKind::Callback(signature) => {
                        self.enter_callback_argument(source)?;
                        evaluate::Mode::Function(signature)
                    }
                };
                Ok::<_, String>((source, kind, mode))
            })();
            let (source, kind, mode) = match prepared {
                Ok(prepared) => prepared,
                Err(error) => {
                    self.abort_local_factory_arguments(arguments.factory);
                    self.restore_factory_expression(&arguments.state);
                    return Err(error);
                }
            };
            // Reuse the current Region identity; the child evaluator owns its
            // operations only until this exact continuation receives it back.
            let output = Self::take_cursor_region(arguments.state.expression.current_region());
            let caller_depth = self.statement_depth;
            cursor.pending.push(Frame::EvaluateFactoryArgument {
                arguments,
                kind,
                depth: caller_depth,
            });
            cursor.next = Some(Next::Expression {
                source,
                output,
                depth,
                mode,
            });
            return Ok(());
        }

        let FactoryArguments {
            mut state,
            factory,
            values,
            ..
        } = *arguments;
        let factory = match factory {
            function_values::PreparedLocalFactory::Block(factory) => factory,
            function_values::PreparedLocalFactory::Concise(factory) => {
                #[cfg(test)]
                BRIDGED_CONCISE_FACTORIES.with(|count| count.set(count.get() + 1));
                return self.finish_concise_factory_expression(state, factory, values, cursor);
            }
        };
        let started = self.begin_block_factory_body(
            factory,
            values,
            state.expression.current_region(),
            state.signature,
        );
        match started {
            Ok((factory, activation, request)) => {
                #[cfg(test)]
                BRIDGED_BLOCK_FACTORIES.with(|count| count.set(count.get() + 1));
                cursor.pending.push(Frame::EvaluateFactoryBody {
                    state,
                    activation,
                    factory,
                    depth: 0,
                });
                cursor.next = Some(Next::Enter {
                    source: ControlNode::Sequence(request.statements),
                    output: request.region,
                    depth: 0,
                    charge: source_control::SourceCharge::Charged,
                });
                Ok(())
            }
            Err(error) => {
                // Body preparation has already restored its bindings and
                // result admission. This popped transition owns the remainder.
                self.restore_factory_expression(&state);
                Err(error)
            }
        }
    }

    pub(super) fn resume_factory_argument<'n>(
        &mut self,
        mut arguments: Box<FactoryArguments<'n, 'ast>>,
        kind: closures::FactoryArgumentKind,
        output: Region,
        value: Option<ValueId>,
        cursor: &mut Cursor<'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        *arguments.state.expression.current_region() = output;
        if matches!(kind, closures::FactoryArgumentKind::Callback(_)) {
            self.leave_function_value();
        }
        if !matches!(kind, closures::FactoryArgumentKind::Discard) {
            arguments
                .values
                .push(value.expect("factory argument value"));
        }
        arguments.index += 1;
        self.advance_factory_arguments(arguments, cursor)
    }

    pub(super) fn abort_factory_argument(
        &mut self,
        arguments: FactoryArguments<'_, '_>,
        kind: closures::FactoryArgumentKind,
    ) {
        if matches!(kind, closures::FactoryArgumentKind::Callback(_)) {
            self.leave_function_value();
        }
        self.abort_local_factory_arguments(arguments.factory);
        self.restore_factory_expression(&arguments.state);
    }

    fn abort_local_factory_arguments(
        &mut self,
        factory: function_values::PreparedLocalFactory<'_, '_>,
    ) {
        if let function_values::PreparedLocalFactory::Block(factory) = factory {
            self.abort_block_factory_arguments(factory);
        }
        // Concise closure metadata owns no active function-result charge.
    }

    /// The callee consumer alone owns an extra function-value admission.
    /// Both consumers restore the same aggregate generated-region depth.
    pub(super) fn restore_factory_expression(&mut self, state: &FactoryExpression<'_, '_>) {
        if matches!(state.resume, FactoryResume::Callee(_)) {
            self.leave_function_value();
        }
        self.restore_function_result_depth(state.previous_result_depth);
    }

    pub(super) fn finish_factory_body<'n>(
        &mut self,
        mut state: FactoryExpression<'n, 'ast>,
        activation: closures::BlockArrowActivation,
        factory: function_values::BlockFactory,
        finished_region: Region,
        completion: Completion,
        cursor: &mut Cursor<'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        // Complete the body in the callee environment, emit its inline call,
        // then restore the caller before any outer-call arguments run.
        let body_result =
            self.finish_arrow_block_activation(activation, finished_region, Ok(completion));
        let emitted = body_result.map(|body| {
            self.resume_block_factory(state.expression.as_mut(), state.node, state.signature, body);
        });
        self.finish_block_factory(factory);
        self.restore_factory_expression(&state);
        emitted?;

        self.resume_factory_expression(state, cursor)
    }

    fn finish_concise_factory_expression<'n>(
        &mut self,
        mut state: FactoryExpression<'n, 'ast>,
        factory: closures::PreparedConciseFactory<'n, 'ast>,
        arguments: Vec<ValueId>,
        cursor: &mut Cursor<'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        // Completion restores caller bindings before materialization, exactly
        // as factory_result did before function_value_leaf consumed metadata.
        // No function-result admission, InlineCall or child body Region belongs
        // to this path.
        let result = self
            .finish_concise_factory(factory, arguments, state.expression.current_region())
            .and_then(|closure| {
                self.materialize_closure(
                    closure,
                    state.expression.current_region(),
                    state.node,
                    state.signature,
                )
            });
        self.restore_factory_expression(&state);
        state.expression.resume_function_value(result?);
        self.resume_factory_expression(state, cursor)
    }

    /// Both factory kinds have restored their own state before delivering the
    /// result to this shared expression/callee consumer.
    fn resume_factory_expression<'n>(
        &mut self,
        mut state: FactoryExpression<'n, 'ast>,
        cursor: &mut Cursor<'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        if let FactoryResume::Callee(continuation) = state.resume {
            let callee = state
                .expression
                .pop_function_value()
                .expect("returned factory callee function");
            self.resume_sync_closure_callee(state.expression.as_mut(), continuation, callee)?;
        }
        self.advance_owned_expression(*state.expression, state.mode, cursor)
    }
}
