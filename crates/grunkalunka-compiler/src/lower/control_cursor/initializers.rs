//! Ordered declaration initializers on the shared source scheduler.
use super::*;

impl<'s, 'ast> Lowering<'s, 'ast> {
    pub(super) fn start_declaration<'n>(
        &mut self,
        source: &'n VariableDeclaration<'ast>,
        output: Region,
        depth: usize,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        self.admit_declaration(source)?;
        self.advance_declaration(
            Box::new(DeclarationSequence {
                source,
                index: 0,
                depth,
            }),
            output,
            cursor,
        )
    }

    pub(super) fn advance_declaration<'n>(
        &mut self,
        mut sequence: Box<DeclarationSequence<'n, 'ast>>,
        mut output: Region,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        while let Some(local) = sequence.source.declarations.get(sequence.index) {
            let prepared = self.prepare_declaration_initializer(
                local,
                sequence.source.kind == VariableDeclarationKind::Let,
                &mut output,
            )?;
            match prepared {
                declarations::DeclarationInitializer::Complete => sequence.index += 1,
                request => {
                    return self
                        .schedule_declaration_initializer(sequence, request, output, cursor);
                }
            }
        }
        cursor.completed = Some(Outcome::Statement(output, Completion::NORMAL));
        Ok(())
    }

    pub(super) fn schedule_declaration_initializer<'n>(
        &mut self,
        mut sequence: Box<DeclarationSequence<'n, 'ast>>,
        prepared: declarations::DeclarationInitializer<'n, 'ast>,
        output: Region,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        match prepared {
            declarations::DeclarationInitializer::Complete => {
                sequence.index += 1;
                // Even a zero-argument metadata factory returns to the main
                // loop before selecting the next declarator. Consecutive
                // factories must not recursively restart this sequence.
                cursor.pending.push(Frame::AdvanceDeclaration { sequence });
                cursor.completed = Some(Outcome::Statement(output, Completion::NORMAL));
                Ok(())
            }
            declarations::DeclarationInitializer::CheckClosure { local, closure } => {
                cursor.pending.push(Frame::CheckedDeclarationClosure {
                    sequence,
                    local,
                    output,
                });
                self.start_closure_check(closure, 0, cursor)
            }
            declarations::DeclarationInitializer::Materialize {
                local,
                mutable,
                closure,
                signature,
            } => {
                let source = local.init.as_ref().expect("closure initializer");
                // Identity-bearing declarations already selected their
                // signature. They own lifting admission, but do not acquire
                // the ordinary function-value guard used by value requests.
                cursor.pending.push(Frame::MaterializedDeclaration {
                    sequence,
                    local,
                    mutable,
                    signature,
                });
                self.start_closure_materialization(closure, output, source, signature, cursor)
            }
            declarations::DeclarationInitializer::CheckFactory {
                local,
                candidate,
                closure,
            } => {
                #[cfg(test)]
                SCHEDULED_METADATA_FACTORY_CHECKS.with(|count| count.set(count.get() + 1));
                cursor.pending.push(Frame::CheckedDeclarationFactory {
                    sequence,
                    local,
                    candidate,
                    output,
                });
                self.start_closure_check(closure, 0, cursor)
            }
            declarations::DeclarationInitializer::Factory(prepared) => {
                #[cfg(test)]
                SCHEDULED_DECLARATION_FACTORIES.with(|count| count.set(count.get() + 1));
                self.start_declaration_factory_arguments(sequence, prepared, output, cursor)
            }
            declarations::DeclarationInitializer::Value(value) => {
                let active = self.begin_declaration_value(value)?;
                let (source, mode) = active.request();
                #[cfg(test)]
                SCHEDULED_DECLARATION_VALUES.with(|count| count.set(count.get() + 1));
                cursor
                    .pending
                    .push(Frame::DeclarationValue { sequence, active });
                cursor.next = Some(Next::Expression {
                    source,
                    output,
                    depth: 0,
                    mode,
                });
                Ok(())
            }
            declarations::DeclarationInitializer::Object(binding) => {
                let source = binding.source;
                cursor
                    .pending
                    .push(Frame::ObjectBindingSource { sequence, binding });
                cursor.next = Some(Next::Expression {
                    source,
                    output,
                    depth: 0,
                    mode: evaluate::Mode::Object,
                });
                Ok(())
            }
            declarations::DeclarationInitializer::Array(binding) => {
                let source = binding.source;
                cursor
                    .pending
                    .push(Frame::ArrayBindingSource { sequence, binding });
                cursor.next = Some(Next::Expression {
                    source,
                    output,
                    depth: 0,
                    mode: evaluate::Mode::Array,
                });
                Ok(())
            }
        }
    }

    pub(super) fn resume_declaration_value<'n>(
        &mut self,
        mut sequence: Box<DeclarationSequence<'n, 'ast>>,
        active: declarations::ActiveDeclarationValue<'n, 'ast>,
        mut output: Region,
        value: ValueId,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        self.finish_declaration_value(active, &mut output, value)?;
        sequence.index += 1;
        self.advance_declaration(sequence, output, cursor)
    }

    pub(super) fn resume_declaration_closure<'n>(
        &mut self,
        mut sequence: Box<DeclarationSequence<'n, 'ast>>,
        local: &'n VariableDeclarator<'ast>,
        closure: closures::LocalClosure,
        mut output: Region,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        self.finish_declaration_closure_check(local, closure, &mut output)?;
        sequence.index += 1;
        self.advance_declaration(sequence, output, cursor)
    }

    pub(super) fn finish_declaration_factory<'n>(
        &mut self,
        sequence: Box<DeclarationSequence<'n, 'ast>>,
        prepared: declarations::PreparedDeclarationFactory<'n, 'ast>,
        output: Region,
        arguments: Vec<ValueId>,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        let (local, factory) = prepared.into_parts();
        self.start_returned_factory_check(
            FactoryMetadataConsumer::Declaration {
                sequence,
                local,
                output,
            },
            factory,
            arguments,
            cursor,
        )
    }

    pub(super) fn resume_declaration_factory_check<'n>(
        &mut self,
        sequence: Box<DeclarationSequence<'n, 'ast>>,
        local: &'n VariableDeclarator<'ast>,
        candidate: closures::ConciseFactoryCandidate<'n, 'ast>,
        closure: closures::LocalClosure,
        mut output: Region,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        self.capture_cells(&closure, &mut output);
        let next =
            self.finish_declaration_factory_preparation(local, candidate, closure, &mut output)?;
        self.schedule_declaration_initializer(sequence, next, output, cursor)
    }
}
