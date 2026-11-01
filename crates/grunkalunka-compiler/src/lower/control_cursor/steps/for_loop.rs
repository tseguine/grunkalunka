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
        // The synthetic header Block and loop Region
        // are not source statements and are not charged.
        let mut header = self.region(source);
        let outer = self.bindings.len();
        let mut let_slots = Vec::new();
        if let Some(init) = &source.init {
            match init {
                ForStatementInit::VariableDeclaration(declaration) => {
                    self.charge_statement(declaration.as_ref())?;
                    let start = self.bindings.len();
                    self.declaration(declaration, &mut header)?;
                    if declaration.kind == VariableDeclarationKind::Let {
                        let_slots.extend(self.bindings[start..].iter().map(|binding| binding.slot));
                    }
                }
                _ => {
                    self.discarded(init.to_expression(), &mut header, 0)?;
                }
            }
        }
        let header_bindings = self.bindings.len();
        let mut condition = self.region(source);
        let predicate = if let Some(test) = &source.test {
            condition.span = test.span();
            self.predicate(test, &mut condition, 0)?
        } else {
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
            predicate
        };
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
        let ForFrame {
            source,
            mut output,
            mut header,
            condition,
            outer,
            header_bindings,
            let_slots,
            old_loop,
            ..
        } = frame;
        self.in_loop_body = old_loop;
        // Body declarations are hidden from the update expression;
        // the lexical header remains live until update is lowered.
        self.bindings.truncate(header_bindings);
        let mut step = if let Some(update) = &source.update {
            let mut step = self.region(update);
            self.discarded(update, &mut step, 0)?;
            Some(step)
        } else {
            None
        };

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
        cursor.completed = Some((output, completion.pretest_loop()));
        Ok(())
    }
}
