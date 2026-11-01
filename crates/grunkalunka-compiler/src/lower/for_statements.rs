//! Schedule direct nested ForStatement bodies on the heap without changing the
//! source loop's generated lexical Block and While region or completion algebra.
use super::*;
use oxc_ast::ast::ForStatement;

impl Lowering<'_, '_> {
    #[inline(never)]
    pub(super) fn for_statement_iterative<'n, 'a>(
        &mut self,
        root: &'n ForStatement<'a>,
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String> {
        type Next<'n, 'a> = source_control::StatementTask<'n, 'a, &'n ForStatement<'a>>;

        struct Frame<'n, 'a> {
            source: &'n ForStatement<'a>,
            output: Region,
            header: Region,
            condition: Region,
            depth: usize,
            outer: usize,
            header_bindings: usize,
            let_slots: Vec<SlotId>,
            old_loop: bool,
        }

        let original_depth = self.statement_depth;
        let original_loop = self.in_loop_body;
        // Preserve previously emitted operations at the original call site.
        let root_output = Self::take_cursor_region(region);
        let result = (|| {
            let mut pending: Vec<Frame<'n, 'a>> = Vec::new();
            let mut next = Some(Next::Enter {
                source: root,
                output: root_output,
                depth,
                charge: source_control::SourceCharge::Charged,
            });
            let mut completed: Option<(Region, Completion)> = None;

            loop {
                if let Some(task) = next.take() {
                    match task {
                        Next::Enter {
                            source,
                            output,
                            depth,
                            charge,
                        } => {
                            self.statement_depth = depth;
                            if charge == source_control::SourceCharge::Uncharged {
                                self.admit_source_statement(source, depth)?;
                            }
                            // Generated header Block and loop Regions are not
                            // charged as source statements or lexical depth.
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
                                            let_slots.extend(
                                                self.bindings[start..]
                                                    .iter()
                                                    .map(|binding| binding.slot),
                                            );
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
                            pending.push(Frame {
                                source,
                                output,
                                header,
                                condition,
                                depth,
                                outer,
                                header_bindings,
                                let_slots,
                                old_loop,
                            });
                            self.in_loop_body = true;
                            next = Some(Next::Statement {
                                source: &source.body,
                                output: body,
                                depth: depth + 1,
                            });
                        }
                        Next::Statement {
                            source,
                            mut output,
                            depth,
                        } => {
                            if let Statement::ForStatement(inner) = source {
                                next = Some(Next::Enter {
                                    source: inner,
                                    output,
                                    depth,
                                    charge: source_control::SourceCharge::Uncharged,
                                });
                            } else {
                                self.statement_depth =
                                    pending.last().map_or(original_depth, |frame| frame.depth);
                                self.in_loop_body = true;
                                let completion = self.statement(source, &mut output, depth)?;
                                completed = Some((output, completion));
                            }
                        }
                    }
                    continue;
                }

                let (body, completion) = completed.take().expect("completed source for body");
                let Some(mut frame) = pending.pop() else {
                    *region = body;
                    return Ok(completion);
                };
                self.statement_depth = frame.depth;
                self.in_loop_body = frame.old_loop;
                // The body cannot leak declarations into the update or the
                // enclosing lexical scope, but the header bindings stay live
                // while the update expression is admitted and lowered.
                self.bindings.truncate(frame.header_bindings);
                let mut step = if let Some(update) = &frame.source.update {
                    let mut step = self.region(update);
                    self.discarded(update, &mut step, 0)?;
                    Some(step)
                } else {
                    None
                };
                // Each for-let iteration has a fresh lexical environment.
                // Only captured header bindings need a new physical cell:
                // unobserved Number/primitive slots keep their existing fast
                // path and emitted bytes. Collect captures across all parts,
                // including initializer, condition, body and update.
                let mut captured = ir::captured_cells(&frame.header);
                captured.extend(ir::captured_cells(&frame.condition));
                captured.extend(ir::captured_cells(&body));
                if let Some(step) = &step {
                    captured.extend(ir::captured_cells(step));
                }
                let renewal = frame
                    .let_slots
                    .into_iter()
                    .filter(|slot| captured.contains(slot))
                    .collect::<Vec<_>>();
                if !renewal.is_empty() {
                    // The initializer is in the original lexical binding.
                    // Copy before the first test so an initializer closure
                    // cannot alias closures from the first loop body.
                    for &slot in &renewal {
                        let result = self.value(self.slots[slot.0].ty);
                        Self::push(&mut frame.header, frame.source, Op::Load { result, slot });
                        Self::push(
                            &mut frame.header,
                            frame.source,
                            Op::RebindCell {
                                slot,
                                value: result,
                            },
                        );
                    }
                    // Continue and ordinary body completion both enter step;
                    // a new cell is cloned *before* update side effects. The
                    // source update (when present) then mutates that new cell.
                    let mut next = step.take().unwrap_or_else(|| self.region(frame.source));
                    let updates = std::mem::take(&mut next.operations);
                    for &slot in &renewal {
                        let result = self.value(self.slots[slot.0].ty);
                        Self::push(&mut next, frame.source, Op::Load { result, slot });
                        Self::push(
                            &mut next,
                            frame.source,
                            Op::RebindCell {
                                slot,
                                value: result,
                            },
                        );
                    }
                    next.operations.extend(updates);
                    step = Some(next);
                }
                Self::push(
                    &mut frame.header,
                    frame.source,
                    Op::While {
                        condition: frame.condition,
                        body,
                        step,
                    },
                );
                self.bindings.truncate(frame.outer);
                Self::push(&mut frame.output, frame.source, Op::Block(frame.header));
                completed = Some((frame.output, completion.pretest_loop()));
            }
        })();
        self.statement_depth = original_depth;
        self.in_loop_body = original_loop;
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::{Op, SlotId};

    #[test]
    fn nested_for_statements_keep_small_stack_and_generated_region_budget() {
        for (depth, admitted) in [(1, true), (8, true), (16, true), (65, false)] {
            let mut body = String::from("break;");
            for _ in 0..depth {
                body = format!("for(;a;) {body}");
            }
            let source = format!("export function run(a,b){{{body}return b;}}");
            for typescript in [false, true] {
                let source = source.clone();
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile(&source, typescript))
                    .expect("small-stack for lowering thread")
                    .join()
                    .expect("nested source for must not overflow native stack");
                if admitted {
                    assert!(
                        !result
                            .unwrap_or_else(|error| panic!("{depth}: {error}"))
                            .wasm
                            .is_empty()
                    );
                } else {
                    let error = result.err().expect("deep source for should be rejected");
                    assert!(
                        error.contains("64") || error.contains("128"),
                        "{depth} nested for: {error}"
                    );
                }
            }
        }
    }

    #[test]
    fn per_iteration_cells_are_generated_only_for_captured_let_headers() {
        for (body, expected) in [
            ("let n=0;for(let i=0;i<3;i++)n+=i;return n;", 0),
            (
                "let first=()=>a,last=()=>b;for(let i=0;i<3;i++){const make=()=>i;if(i===0)first=make;last=make;}return first()*10+last();",
                2,
            ),
            (
                "let first=()=>a,last=()=>b;for(let i=0;i<3;){const make=()=>i;if(i===0)first=make;last=make;i++;}return first()*10+last();",
                2,
            ),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            let module = crate::lower_source(&source, false)
                .unwrap_or_else(|error| panic!("{source}: {error}"));
            let mut renewals = 0;
            crate::ir::visit_operations(&module.functions[0].body, &mut |op| {
                if matches!(op, Op::RebindCell { .. }) {
                    renewals += 1;
                }
            });
            assert_eq!(renewals, expected, "{source}");
        }
    }

    #[test]
    fn rebinding_cannot_target_a_parameter_instead_of_the_lexical_header() {
        let source = "export function run(a,b){let first=()=>a,last=()=>b;for(let i=0;i<3;i++){const make=()=>i;if(i===0)first=make;last=make;}return first()*10+last();}";
        let mut module = crate::lower_source(source, false).unwrap();
        crate::ir::verify_module(&module).expect("source-lowered rebinding is valid");
        let header = module.functions[0]
            .body
            .operations
            .iter_mut()
            .find_map(|operation| match &mut operation.kind {
                Op::Block(region) => Some(region),
                _ => None,
            })
            .expect("for-let header block");
        let renewal = header
            .operations
            .iter_mut()
            .find_map(|operation| match &mut operation.kind {
                Op::RebindCell { slot, .. } => Some(slot),
                _ => None,
            })
            .expect("captured header rebinding");
        *renewal = SlotId(0);
        assert!(
            crate::ir::verify_module(&module)
                .unwrap_err()
                .contains("rebinding requires an initialized mutable lexical cell")
        );
    }

    #[test]
    fn for_header_declarations_and_updates_retain_scope_and_effect_order() {
        for body in [
            "let n=0;for(let i=0;i<3;i++)n+=b;return n;",
            "let n=a;for(let i=0;i<3;i++){n+=i;}return n;",
            "let n=0;for(let i=0;i<3;){n+=a;i++;}return n;",
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            let artifact =
                crate::compile(&source, false).unwrap_or_else(|error| panic!("{body}: {error}"));
            assert!(!artifact.wasm.is_empty());
        }
    }
}
