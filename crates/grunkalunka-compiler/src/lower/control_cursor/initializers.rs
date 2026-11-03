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

    fn advance_declaration<'n>(
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
                declarations::DeclarationInitializer::CheckClosure { local, closure } => {
                    cursor.pending.push(Frame::CheckedDeclarationClosure {
                        sequence,
                        local,
                        output,
                    });
                    return self.start_closure_check(closure, 0, cursor);
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
                    return Ok(());
                }
            }
        }
        cursor.completed = Some(Outcome::Statement(output, Completion::NORMAL));
        Ok(())
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
}
