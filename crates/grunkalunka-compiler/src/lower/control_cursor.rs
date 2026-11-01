//! One heap cursor for mixed if/while/do-while source nesting.
//!
//! Previously each control family had its own iterative cursor, but alternating
//! families reentered the large Rust lowering frames through `statement`.
//! Keep source-specific continuations as typed frames and dispatch every mixed
//! child through this one loop. For statements, blocks and try/finally are still
//! separate continuation families and can join this cursor incrementally.
use super::*;
use oxc_ast::ast::{BlockStatement, DoWhileStatement, ForStatement, IfStatement, WhileStatement};

#[derive(Clone, Copy)]
pub(super) enum ControlNode<'n, 'a> {
    If(&'n IfStatement<'a>),
    While(&'n WhileStatement<'a>),
    DoWhile(&'n DoWhileStatement<'a>),
    For(&'n ForStatement<'a>),
    Block(&'n BlockStatement<'a>),
}

impl<'n, 'a> ControlNode<'n, 'a> {
    fn of_statement(source: &'n Statement<'a>) -> Option<Self> {
        match source {
            Statement::IfStatement(statement) => Some(Self::If(statement)),
            Statement::WhileStatement(statement) => Some(Self::While(statement)),
            Statement::DoWhileStatement(statement) => Some(Self::DoWhile(statement)),
            Statement::ForStatement(statement) => Some(Self::For(statement)),
            Statement::BlockStatement(statement) => Some(Self::Block(statement)),
            _ => None,
        }
    }
}

enum Frame<'n, 'a> {
    If {
        source: &'n IfStatement<'a>,
        output: Region,
        depth: usize,
        visible: usize,
        condition: ValueId,
        consequent: Option<Region>,
        consequent_returns: Option<Completion>,
    },
    While {
        source: &'n WhileStatement<'a>,
        output: Region,
        depth: usize,
        visible: usize,
        old_loop: bool,
        condition: Region,
    },
    DoWhile {
        source: &'n DoWhileStatement<'a>,
        output: Region,
        depth: usize,
        visible: usize,
        old_loop: bool,
    },
    For {
        source: &'n ForStatement<'a>,
        output: Region,
        header: Region,
        condition: Region,
        depth: usize,
        outer: usize,
        header_bindings: usize,
        let_slots: Vec<SlotId>,
        old_loop: bool,
    },
    Block {
        source: &'n BlockStatement<'a>,
        output: Region,
        depth: usize,
        visible: usize,
        index: usize,
        completion: Completion,
    },
}

impl Frame<'_, '_> {
    fn depth(&self) -> usize {
        match self {
            Self::If { depth, .. }
            | Self::While { depth, .. }
            | Self::DoWhile { depth, .. }
            | Self::For { depth, .. }
            | Self::Block { depth, .. } => *depth,
        }
    }
}

impl Lowering<'_, '_> {
    #[inline(never)]
    pub(super) fn control_statement_iterative<'n, 'a>(
        &mut self,
        root: ControlNode<'n, 'a>,
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String> {
        type Next<'n, 'a> = source_control::StatementTask<'n, 'a, ControlNode<'n, 'a>>;

        // The root has already been charged by statement_inner. Every direct
        // nested control node is charged exactly once when its Enter runs.
        let original_depth = self.statement_depth;
        let original_loop = self.in_loop_body;
        let root_output = Self::take_cursor_region(region);
        let result = (|| {
            let mut pending = Vec::<Frame<'n, 'a>>::new();
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
                            mut output,
                            depth,
                            charge,
                        } => {
                            self.statement_depth = depth;
                            if charge == source_control::SourceCharge::Uncharged {
                                match source {
                                    ControlNode::If(node) => {
                                        self.admit_source_statement(node, depth)?;
                                    }
                                    ControlNode::While(node) => {
                                        self.admit_source_statement(node, depth)?;
                                    }
                                    ControlNode::DoWhile(node) => {
                                        self.admit_source_statement(node, depth)?;
                                    }
                                    ControlNode::For(node) => {
                                        self.admit_source_statement(node, depth)?;
                                    }
                                    ControlNode::Block(node) => {
                                        self.admit_source_statement(node, depth)?;
                                    }
                                }
                            }

                            match source {
                                ControlNode::Block(source) => {
                                    // The block's new Region is lexical, not
                                    // an additional source statement charge.
                                    let child = self.region(source);
                                    let visible = self.bindings.len();
                                    if let Some(statement) = source.body.first() {
                                        pending.push(Frame::Block {
                                            source,
                                            output,
                                            depth,
                                            visible,
                                            index: 0,
                                            completion: Completion::NORMAL,
                                        });
                                        next = Some(Next::Statement {
                                            source: statement,
                                            output: child,
                                            depth: depth + 1,
                                        });
                                    } else {
                                        Self::push(&mut output, source, Op::Block(child));
                                        completed = Some((output, Completion::NORMAL));
                                    }
                                }
                                ControlNode::If(source) => {
                                    // If conditions execute before either branch.
                                    let condition = self.predicate(&source.test, &mut output, 0)?;
                                    let consequent = self.region(&source.consequent);
                                    let visible = self.bindings.len();
                                    pending.push(Frame::If {
                                        source,
                                        output,
                                        depth,
                                        visible,
                                        condition,
                                        consequent: None,
                                        consequent_returns: None,
                                    });
                                    next = Some(Next::Statement {
                                        source: &source.consequent,
                                        output: consequent,
                                        depth: depth + 1,
                                    });
                                }
                                ControlNode::While(source) => {
                                    // While tests execute before their bodies.
                                    let mut condition = self.region(&source.test);
                                    let predicate =
                                        self.predicate(&source.test, &mut condition, 0)?;
                                    condition.exit = Exit::Yield(predicate);
                                    let body = self.region(&source.body);
                                    let visible = self.bindings.len();
                                    let old_loop = self.in_loop_body;
                                    pending.push(Frame::While {
                                        source,
                                        output,
                                        depth,
                                        visible,
                                        old_loop,
                                        condition,
                                    });
                                    self.in_loop_body = true;
                                    next = Some(Next::Statement {
                                        source: &source.body,
                                        output: body,
                                        depth: depth + 1,
                                    });
                                }
                                ControlNode::For(source) => {
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
                                                if declaration.kind == VariableDeclarationKind::Let
                                                {
                                                    let_slots.extend(
                                                        self.bindings[start..]
                                                            .iter()
                                                            .map(|binding| binding.slot),
                                                    );
                                                }
                                            }
                                            _ => {
                                                self.discarded(
                                                    init.to_expression(),
                                                    &mut header,
                                                    0,
                                                )?;
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
                                    pending.push(Frame::For {
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
                                ControlNode::DoWhile(source) => {
                                    // Do-while tests are admitted only after the body.
                                    let body = self.region(&source.body);
                                    let visible = self.bindings.len();
                                    let old_loop = self.in_loop_body;
                                    pending.push(Frame::DoWhile {
                                        source,
                                        output,
                                        depth,
                                        visible,
                                        old_loop,
                                    });
                                    self.in_loop_body = true;
                                    next = Some(Next::Statement {
                                        source: &source.body,
                                        output: body,
                                        depth: depth + 1,
                                    });
                                }
                            }
                        }
                        Next::Statement {
                            source,
                            mut output,
                            depth,
                        } => {
                            if let Some(node) = ControlNode::of_statement(source) {
                                next = Some(Next::Enter {
                                    source: node,
                                    output,
                                    depth,
                                    charge: source_control::SourceCharge::Uncharged,
                                });
                            } else {
                                // The child may itself be a block, for, try or
                                // function body. Preserve its enclosing loop mode
                                // and source depth until the child has completed.
                                self.statement_depth =
                                    pending.last().map_or(original_depth, Frame::depth);
                                let completion = self.statement(source, &mut output, depth)?;
                                completed = Some((output, completion));
                            }
                        }
                    }
                    continue;
                }

                let (finished_region, completion) =
                    completed.take().expect("completed mixed control child");
                let Some(frame) = pending.pop() else {
                    *region = finished_region;
                    return Ok(completion);
                };

                self.statement_depth = frame.depth();
                match frame {
                    Frame::Block {
                        source,
                        mut output,
                        depth,
                        visible,
                        index,
                        completion: prefix,
                    } => {
                        // A sequence keeps its local declarations visible to
                        // each sibling. Its completion channels combine in
                        // order and only normal paths reach the next sibling.
                        let completion = prefix.then(completion);
                        let next_index = index + 1;
                        if let Some(statement) = source.body.get(next_index) {
                            if !completion.normal && !is_type_declaration(statement) {
                                return Err(unsupported(
                                    statement,
                                    if completion == Completion::RETURN {
                                        "statements after a guaranteed return are not admitted"
                                    } else {
                                        "statements after a guaranteed abrupt completion are not admitted"
                                    },
                                ));
                            }
                            pending.push(Frame::Block {
                                source,
                                output,
                                depth,
                                visible,
                                index: next_index,
                                completion,
                            });
                            next = Some(Next::Statement {
                                source: statement,
                                output: finished_region,
                                depth: depth + 1,
                            });
                        } else {
                            self.bindings.truncate(visible);
                            Self::push(&mut output, source, Op::Block(finished_region));
                            completed = Some((output, completion));
                        }
                    }
                    Frame::If {
                        source,
                        mut output,
                        depth,
                        visible,
                        condition,
                        consequent,
                        consequent_returns,
                    } => {
                        self.bindings.truncate(visible);
                        if let Some(consequent) = consequent {
                            // Both branches completed and own distinct Regions.
                            let first = consequent_returns.expect("consequent completion");
                            Self::push(
                                &mut output,
                                source,
                                Op::If {
                                    result: None,
                                    condition,
                                    consequent,
                                    alternate: finished_region,
                                },
                            );
                            completed = Some((output, first.union(completion)));
                            continue;
                        }

                        // The consequent has finished. Its lexical declarations
                        // must not be visible while lowering the alternate.
                        let mut alternate = self.region(source);
                        if let Some(alternate_source) = &source.alternate {
                            alternate.span = alternate_source.span();
                            pending.push(Frame::If {
                                source,
                                output,
                                depth,
                                visible,
                                condition,
                                consequent: Some(finished_region),
                                consequent_returns: Some(completion),
                            });
                            next = Some(Next::Statement {
                                source: alternate_source,
                                output: alternate,
                                depth: depth + 1,
                            });
                        } else {
                            Self::push(
                                &mut output,
                                source,
                                Op::If {
                                    result: None,
                                    condition,
                                    consequent: finished_region,
                                    alternate,
                                },
                            );
                            completed = Some((output, completion.union(Completion::NORMAL)));
                        }
                    }
                    Frame::While {
                        source,
                        mut output,
                        visible,
                        old_loop,
                        condition,
                        ..
                    } => {
                        self.in_loop_body = old_loop;
                        self.bindings.truncate(visible);
                        Self::push(
                            &mut output,
                            source,
                            Op::While {
                                condition,
                                body: finished_region,
                                step: None,
                            },
                        );
                        completed = Some((output, completion.pretest_loop()));
                    }
                    Frame::For {
                        source,
                        mut output,
                        mut header,
                        condition,
                        outer,
                        header_bindings,
                        let_slots,
                        old_loop,
                        ..
                    } => {
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
                        completed = Some((output, completion.pretest_loop()));
                    }
                    Frame::DoWhile {
                        source,
                        mut output,
                        visible,
                        old_loop,
                        ..
                    } => {
                        self.in_loop_body = old_loop;
                        self.bindings.truncate(visible);
                        // The source depth limit was checked before descending,
                        // but this predicate is evaluated after the body even
                        // when its completion cannot fall through.
                        let mut condition = self.region(&source.test);
                        let value = self.predicate(&source.test, &mut condition, 0)?;
                        condition.exit = Exit::Yield(value);
                        Self::push(
                            &mut output,
                            source,
                            Op::DoWhile {
                                body: finished_region,
                                condition,
                            },
                        );
                        completed = Some((output, completion.do_loop()));
                    }
                }
            }
        })();
        self.statement_depth = original_depth;
        self.in_loop_body = original_loop;
        result
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn alternating_lexical_blocks_and_control_nodes_preserve_source_guards() {
        // Every nested block has a source declaration and a following control
        // node: a singleton-block peel alone cannot avoid this reentry pattern.
        for depth in [4, 12, 24, 40] {
            let mut body = String::from("a+=b;");
            for index in 0..depth {
                body = match index % 3 {
                    0 => format!("if(a){{let v_{index}=b;{body}}}else{{b+=a;}}"),
                    1 => format!("while(a){{let v_{index}=b;{body}}}"),
                    _ => format!("do{{let v_{index}=b;{body}}}while(false);"),
                };
            }
            let source = format!("export function run(a,b){{{body}return a;}}");
            for ts in [false, true] {
                let source = source.clone();
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile(&source, ts))
                    .expect("small stack control/block thread")
                    .join()
                    .expect("mixed lexical controls must not overflow Rust stack");
                if depth <= 12 {
                    assert!(result.is_ok(), "{depth} levels: {:?}", result.err());
                } else if let Err(error) = result {
                    assert!(
                        error.contains("64") || error.contains("128") || error.contains("256"),
                        "{depth} mixed lexical controls: {error}"
                    );
                }
            }
        }
    }

    #[test]
    fn alternating_if_and_loop_families_keep_native_stack_bounded() {
        // Homogeneous nesting was already iterative. Alternating kinds used to
        // reenter different native-stack cursors for each source layer.
        for depth in [4, 12, 24, 40, 66] {
            let mut body = String::from("a+=b;");
            for index in 0..depth {
                body = match index % 3 {
                    0 => format!("if(a){{{body}}}else{{b+=a;}}"),
                    1 => format!("while(a){{{body}}}"),
                    _ => format!("do{{{body}}}while(false);"),
                };
            }
            let source = format!("export function run(a,b){{{body}return a;}}");
            for ts in [false, true] {
                let source = source.clone();
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile(&source, ts))
                    .expect("2 MiB mixed-control compiler thread")
                    .join()
                    .expect("mixed statement cursor must not exhaust native stack");
                if depth <= 12 {
                    assert!(result.is_ok(), "{depth} mixed controls: {:?}", result.err());
                }
                if depth == 66 {
                    let error = result.err().expect("over-depth mixed source must reject");
                    assert!(
                        error.contains("64") || error.contains("128"),
                        "{depth} mixed controls: {error}"
                    );
                }
            }
        }
    }
}
