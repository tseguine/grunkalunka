//! Specialized source ForStatement transitions for the shared cursor.
//!
//! This owns generated header/condition/step regions and captured let-cell
//! renewal while the parent scheduler owns source traversal and resume order.
use super::*;

impl Lowering<'_, '_> {
    pub(super) fn begin_for_loop<'n, 'a>(
        &mut self,
        source: &'n ForStatement<'a>,
        output: Region,
        depth: usize,
        cursor: &mut Cursor<'n, 'a>,
    ) -> Result<(), String> {
        // The synthetic header Block is not charged as a source statement.
        // Expression and declaration initializers both emit into that Region.
        let header = self.region(source);
        let outer = self.bindings.len();
        if let Some(init) = &source.init {
            match init {
                ForStatementInit::VariableDeclaration(declaration) => {
                    self.charge_statement(declaration.as_ref())?;
                    let start = self.bindings.len();
                    cursor.pending.push(Frame::ForDeclarationInitializer {
                        source,
                        output,
                        depth,
                        outer,
                        start,
                        mutable: declaration.kind == VariableDeclarationKind::Let,
                    });
                    return self.start_declaration(declaration, header, depth, cursor);
                }
                _ => {
                    cursor.pending.push(Frame::ForInitializer {
                        source,
                        output,
                        depth,
                        outer,
                    });
                    cursor.next = Some(Next::Expression {
                        source: init.to_expression(),
                        output: header,
                        depth: 0,
                        mode: evaluate::Mode::Discard,
                    });
                    return Ok(());
                }
            }
        }
        self.finish_for_initializer(
            ForPredicateFrame {
                source,
                output,
                header,
                depth,
                outer,
                header_bindings: self.bindings.len(),
                let_slots: Vec::new(),
            },
            cursor,
        )
    }

    /// Resume only after the initializer has emitted its side effects into
    /// the lexical header. Declaration continuations collect their let bindings
    /// after all values are published; expression initializers add none.
    pub(super) fn finish_for_initializer<'n, 'a>(
        &mut self,
        frame: ForPredicateFrame<'n, 'a>,
        cursor: &mut Cursor<'n, 'a>,
    ) -> Result<(), String> {
        let source = frame.source;
        let mut condition = self.region(source);
        if let Some(test) = &source.test {
            condition.span = test.span();
            cursor.pending.push(Frame::ForPredicate(frame));
            cursor.next = Some(Next::Expression {
                source: test,
                output: condition,
                depth: 0,
                mode: evaluate::Mode::Predicate,
            });
            return Ok(());
        }

        // Omitted tests still create the original constant-true predicate,
        // without scheduling an expression that does not exist in the AST.
        let one = self.value(Type::Number);
        Self::push(
            &mut condition,
            source,
            Op::Constant {
                result: one,
                value: 1.0,
            },
        );
        let predicate = self.value(Type::Predicate);
        Self::push(
            &mut condition,
            source,
            Op::Truthy {
                result: predicate,
                input: one,
            },
        );
        self.resume_for_predicate(frame, condition, predicate, cursor)
    }

    pub(super) fn resume_for_predicate<'n, 'a>(
        &mut self,
        frame: ForPredicateFrame<'n, 'a>,
        mut condition: Region,
        predicate: ValueId,
        cursor: &mut Cursor<'n, 'a>,
    ) -> Result<(), String> {
        let ForPredicateFrame {
            source,
            output,
            header,
            depth,
            outer,
            header_bindings,
            let_slots,
        } = frame;
        condition.exit = Exit::Yield(predicate);
        let body = self.region(&source.body);
        let old_loop = self.in_loop_body;
        cursor.pending.push(Frame::For(ForFrame {
            source,
            output,
            header,
            condition,
            depth,
            outer,
            header_bindings,
            let_slots,
            old_loop,
        }));
        self.in_loop_body = true;
        cursor.next = Some(Next::Statement {
            source: &source.body,
            output: body,
            depth: depth + 1,
        });
        Ok(())
    }

    pub(super) fn resume_for_loop<'n, 'a>(
        &mut self,
        frame: ForFrame<'n, 'a>,
        finished_region: Region,
        completion: Completion,
        cursor: &mut Cursor<'n, 'a>,
    ) -> Result<(), String> {
        // Body locals are not visible to the update, but the header bindings
        // remain live. Restore the outer loop flag before evaluating the step.
        self.in_loop_body = frame.old_loop;
        self.bindings.truncate(frame.header_bindings);
        let source = frame.source;
        if let Some(update) = &source.update {
            let step = self.region(update);
            cursor.pending.push(Frame::ForUpdate {
                frame,
                body: finished_region,
                completion,
            });
            cursor.next = Some(Next::Expression {
                source: update,
                output: step,
                depth: 0,
                mode: evaluate::Mode::Discard,
            });
            return Ok(());
        }
        self.finish_for_loop(frame, finished_region, completion, None, cursor)
    }

    /// Assemble the original header/while IR only after the optional step
    /// has finished. Capture discovery must observe its emitted operations
    /// before deciding which lexical let-header cells need renewal.
    pub(super) fn finish_for_loop<'n, 'a>(
        &mut self,
        frame: ForFrame<'n, 'a>,
        finished_region: Region,
        completion: Completion,
        mut step: Option<Region>,
        cursor: &mut Cursor<'n, 'a>,
    ) -> Result<(), String> {
        let ForFrame {
            source,
            mut output,
            mut header,
            condition,
            outer,
            let_slots,
            ..
        } = frame;
        // Captured 'let' headers need distinct per-iteration
        // cells. Keep the exact original observable renewal
        // order, including before update side effects.
        let mut captured = ir::captured_cells(&header);
        captured.extend(ir::captured_cells(&condition));
        captured.extend(ir::captured_cells(&finished_region));
        if let Some(step) = &step {
            captured.extend(ir::captured_cells(step));
        }
        let renewal = let_slots
            .into_iter()
            .filter(|slot| captured.contains(slot))
            .collect::<Vec<_>>();
        if !renewal.is_empty() {
            for &slot in &renewal {
                let result = self.value(self.slots[slot.0].ty);
                Self::push(&mut header, source, Op::Load { result, slot });
                Self::push(
                    &mut header,
                    source,
                    Op::RebindCell {
                        slot,
                        value: result,
                    },
                );
            }
            let mut next_step = step.take().unwrap_or_else(|| self.region(source));
            let updates = std::mem::take(&mut next_step.operations);
            for &slot in &renewal {
                let result = self.value(self.slots[slot.0].ty);
                Self::push(&mut next_step, source, Op::Load { result, slot });
                Self::push(
                    &mut next_step,
                    source,
                    Op::RebindCell {
                        slot,
                        value: result,
                    },
                );
            }
            next_step.operations.extend(updates);
            step = Some(next_step);
        }
        Self::push(
            &mut header,
            source,
            Op::While {
                condition,
                body: finished_region,
                step,
            },
        );
        self.bindings.truncate(outer);
        Self::push(&mut output, source, Op::Block(header));
        cursor.completed = Some(Outcome::Statement(output, completion.pretest_loop()));
        Ok(())
    }
}
