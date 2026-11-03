//! Factory definition checks retain their original admission, captures and
//! lexical checkpoints while disposable children use the shared source cursor.
use super::*;

impl<'s, 'ast> Lowering<'s, 'ast> {
    pub(super) fn start_factory_expression_check<'n>(
        &mut self,
        state: FactoryExpression<'n, 'ast>,
        resume: FactoryCheckResume<'n, 'ast>,
        closure: closures::LocalClosure,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        #[cfg(test)]
        match &resume {
            FactoryCheckResume::Metadata { .. } => {
                SCHEDULED_METADATA_FACTORY_CHECKS.with(|count| count.set(count.get() + 1));
            }
            FactoryCheckResume::Local { .. } => {
                SCHEDULED_LOCAL_FACTORY_CHECKS.with(|count| count.set(count.get() + 1));
            }
        }
        // Publish the consumer before even checking admission can fail. Its
        // local variant owns result8; metadata owns only the caller context.
        cursor.pending.push(Frame::CheckedFactoryExpression {
            state,
            resume,
            depth: self.statement_depth,
        });
        self.start_closure_check(closure, 0, cursor)
    }

    pub(super) fn start_local_factory_preparation<'n>(
        &mut self,
        state: FactoryExpression<'n, 'ast>,
        closure: closures::LocalClosure,
        expression_depth: usize,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        let Expression::CallExpression(call) = state.node else {
            unreachable!("known local factory is a direct call");
        };
        let preparation = match self.begin_local_result_preparation(call) {
            Ok(preparation) => preparation,
            Err(error) => {
                self.restore_factory_expression(&state);
                return Err(error);
            }
        };
        if !closure.checked {
            return self.start_factory_expression_check(
                state,
                FactoryCheckResume::Local {
                    preparation,
                    expression_depth,
                },
                closure,
                cursor,
            );
        }
        self.finish_local_factory_preparation(state, preparation, closure, expression_depth, cursor)
    }

    fn finish_local_factory_preparation<'n>(
        &mut self,
        state: FactoryExpression<'n, 'ast>,
        preparation: function_values::LocalResultPreparation,
        closure: closures::LocalClosure,
        expression_depth: usize,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        let Expression::CallExpression(call) = state.node else {
            unreachable!("known local factory is a direct call");
        };
        let factory = match self.finish_local_result_preparation(preparation, call, closure) {
            Ok(factory) => factory,
            Err(error) => {
                // Validation consumed result8; this popped stage owns the
                // remaining expression/callee context restoration.
                self.restore_factory_expression(&state);
                return Err(error);
            }
        };
        self.start_factory_arguments(
            state,
            function_values::PreparedFactory::LocalResult(factory),
            expression_depth,
            cursor,
        )
    }

    pub(super) fn resume_factory_expression_check<'n>(
        &mut self,
        mut state: FactoryExpression<'n, 'ast>,
        resume: FactoryCheckResume<'n, 'ast>,
        closure: closures::LocalClosure,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        // The child has restored its parent and committed definition costs.
        // Both entry phases publish captures before checking call eligibility.
        self.capture_cells(&closure, state.expression.current_region());
        match resume {
            FactoryCheckResume::Metadata {
                candidate,
                fallback,
                expression_depth,
            } => match self.finish_concise_factory_preparation(candidate, closure) {
                Ok(Some(factory)) => self.start_factory_arguments(
                    state,
                    function_values::PreparedFactory::ClosureMetadata(factory),
                    expression_depth,
                    cursor,
                ),
                Ok(None) => {
                    // Keep the original independent snapshot, including its
                    // unchecked flag. Its later check/capture remains ordered
                    // after this metadata phase's successful None effects.
                    self.start_local_factory_preparation(state, fallback, expression_depth, cursor)
                }
                Err(error) => {
                    self.restore_factory_expression(&state);
                    Err(error)
                }
            },
            FactoryCheckResume::Local {
                preparation,
                expression_depth,
            } => self.finish_local_factory_preparation(
                state,
                preparation,
                closure,
                expression_depth,
                cursor,
            ),
        }
    }

    pub(super) fn abort_factory_expression_check(
        &mut self,
        state: FactoryExpression<'_, '_>,
        resume: FactoryCheckResume<'_, '_>,
    ) {
        if let FactoryCheckResume::Local { preparation, .. } = resume {
            self.abort_local_result_preparation(preparation);
        }
        self.restore_factory_expression(&state);
    }

    pub(super) fn start_returned_factory_check<'n>(
        &mut self,
        mut consumer: FactoryMetadataConsumer<'n, 'ast>,
        factory: closures::PreparedConciseFactory<'n, 'ast>,
        arguments: Vec<ValueId>,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        let (bindings, closure) = match self.begin_concise_factory_result(
            factory,
            arguments,
            consumer.current_region(),
        ) {
            Ok(started) => started,
            Err(error) => {
                // Setup already restored its partially installed bindings.
                self.abort_factory_metadata_consumer(consumer);
                return Err(error);
            }
        };
        if closure.checked {
            return self.resume_returned_factory_check(consumer, bindings, closure, cursor);
        }
        #[cfg(test)]
        SCHEDULED_RETURNED_FACTORY_CHECKS.with(|count| count.set(count.get() + 1));
        cursor.pending.push(Frame::CheckedFactoryResult {
            consumer,
            bindings,
            depth: self.statement_depth,
        });
        self.start_closure_check(closure, 0, cursor)
    }

    pub(super) fn resume_returned_factory_check<'n>(
        &mut self,
        consumer: FactoryMetadataConsumer<'n, 'ast>,
        bindings: closures::FactoryBindings,
        closure: closures::LocalClosure,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        // The returned definition was checked under the fresh factory cells.
        // Publication and real materialization both belong to the caller.
        self.restore_factory_bindings(bindings);
        match consumer {
            FactoryMetadataConsumer::Expression(state) => {
                self.resume_concise_factory_metadata(state, closure, cursor)
            }
            FactoryMetadataConsumer::Declaration {
                sequence,
                local,
                mut output,
            } => {
                let next = self.publish_declaration_closure(local, false, closure, &mut output)?;
                self.schedule_declaration_initializer(sequence, next, output, cursor)
            }
        }
    }

    pub(super) fn resume_concise_factory_metadata<'n>(
        &mut self,
        state: FactoryExpression<'n, 'ast>,
        closure: closures::LocalClosure,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        // MaterializedFunction now owns the still-active result context and
        // any original callee admission, including materialization errors.
        self.start_function_materialization(state, closure, cursor)
    }

    pub(super) fn abort_returned_factory_check(
        &mut self,
        consumer: FactoryMetadataConsumer<'_, '_>,
        bindings: closures::FactoryBindings,
    ) {
        self.restore_factory_bindings(bindings);
        self.abort_factory_metadata_consumer(consumer);
    }

    fn abort_factory_metadata_consumer(&mut self, consumer: FactoryMetadataConsumer<'_, '_>) {
        if let FactoryMetadataConsumer::Expression(state) = consumer {
            self.restore_factory_expression(&state);
        }
    }
}
