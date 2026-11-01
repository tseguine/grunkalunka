//! One heap cursor for mixed if/while/do-while source nesting.
//!
//! Previously each control family had its own iterative cursor, but alternating
//! families reentered the large Rust lowering frames through `statement`.
//! Keep source-specific continuations as typed frames and dispatch every mixed
//! child through this one loop. For statements, blocks and try/finally are still
//! separate continuation families and can join this cursor incrementally.
use super::*;
use oxc_ast::ast::{DoWhileStatement, IfStatement, WhileStatement};

#[derive(Clone, Copy)]
pub(super) enum ControlNode<'n, 'a> {
    If(&'n IfStatement<'a>),
    While(&'n WhileStatement<'a>),
    DoWhile(&'n DoWhileStatement<'a>),
}

impl<'n, 'a> ControlNode<'n, 'a> {
    fn of_statement(source: &'n Statement<'a>) -> Option<Self> {
        match source {
            Statement::IfStatement(statement) => Some(Self::If(statement)),
            Statement::WhileStatement(statement) => Some(Self::While(statement)),
            Statement::DoWhileStatement(statement) => Some(Self::DoWhile(statement)),
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
}

impl Frame<'_, '_> {
    fn depth(&self) -> usize {
        match self {
            Self::If { depth, .. }
            | Self::While { depth, .. }
            | Self::DoWhile { depth, .. } => *depth,
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
                                }
                            }

                            match source {
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
                    assert!(
                        result.is_ok(),
                        "{depth} mixed controls: {:?}",
                        result.err()
                    );
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
