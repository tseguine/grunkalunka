//! Explicit continuations for nested source While and DoWhile statements.
//! While tests lower before their bodies; DoWhile tests lower afterwards.
use super::*;
use oxc_ast::ast::{DoWhileStatement, WhileStatement};

#[derive(Clone, Copy)]
pub(super) enum LoopNode<'n, 'a> {
    While(&'n WhileStatement<'a>),
    DoWhile(&'n DoWhileStatement<'a>),
}

impl Lowering<'_, '_> {
    #[inline(never)]
    pub(super) fn loop_statement_iterative<'n, 'a>(
        &mut self,
        root: LoopNode<'n, 'a>,
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String> {
        enum Next<'n, 'a> {
            Loop {
                source: LoopNode<'n, 'a>,
                output: Region,
                depth: usize,
                charged: bool,
            },
            Statement {
                source: &'n Statement<'a>,
                output: Region,
                depth: usize,
            },
        }

        enum Kind<'n, 'a> {
            While {
                source: &'n WhileStatement<'a>,
                condition: Region,
            },
            DoWhile {
                source: &'n DoWhileStatement<'a>,
            },
        }

        struct Frame<'n, 'a> {
            output: Region,
            kind: Kind<'n, 'a>,
            depth: usize,
            visible: usize,
            old_loop: bool,
        }

        // The enclosing statement has already been charged by statement_inner.
        // Preserve its partially constructed Region and both dynamic bindings
        // while scheduling nested loop bodies without native recursion.
        let original_depth = self.statement_depth;
        let original_loop = self.in_loop_body;
        let root_output = Region {
            id: region.id,
            span: region.span,
            operations: std::mem::take(&mut region.operations),
            exit: std::mem::replace(&mut region.exit, Exit::Fallthrough),
        };
        let result = (|| {
            let mut pending: Vec<Frame<'n, 'a>> = Vec::new();
            let mut next = Some(Next::Loop {
                source: root,
                output: root_output,
                depth,
                charged: true,
            });
            let mut completed: Option<(Region, Completion)> = None;

            loop {
                if let Some(task) = next.take() {
                    match task {
                        Next::Loop {
                            source,
                            mut output,
                            depth,
                            charged,
                        } => {
                            self.statement_depth = depth;
                            if !charged {
                                let span = match source {
                                    LoopNode::While(node) => node.span(),
                                    LoopNode::DoWhile(node) => node.span(),
                                };
                                if self.active_block_calls != 0 && self.inline_depth + depth > 128 {
                                    return Err(unsupported(
                                        &span,
                                        "block-arrow expansion exceeds 128 nested regions",
                                    ));
                                }
                                if depth > 64 {
                                    return Err(unsupported(
                                        &span,
                                        "statement nesting exceeds 64",
                                    ));
                                }
                                self.charge_statement(&span)?;
                            }

                            let old_loop = self.in_loop_body;
                            match source {
                                LoopNode::While(statement) => {
                                    let mut condition = self.region(&statement.test);
                                    let value = self.predicate(
                                        &statement.test,
                                        &mut condition,
                                        0,
                                    )?;
                                    condition.exit = Exit::Yield(value);
                                    let body = self.region(&statement.body);
                                    let visible = self.bindings.len();
                                    pending.push(Frame {
                                        output,
                                        kind: Kind::While {
                                            source: statement,
                                            condition,
                                        },
                                        depth,
                                        visible,
                                        old_loop,
                                    });
                                    self.in_loop_body = true;
                                    next = Some(Next::Statement {
                                        source: &statement.body,
                                        output: body,
                                        depth: depth + 1,
                                    });
                                }
                                LoopNode::DoWhile(statement) => {
                                    let body = self.region(&statement.body);
                                    let visible = self.bindings.len();
                                    pending.push(Frame {
                                        output,
                                        kind: Kind::DoWhile { source: statement },
                                        depth,
                                        visible,
                                        old_loop,
                                    });
                                    self.in_loop_body = true;
                                    next = Some(Next::Statement {
                                        source: &statement.body,
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
                        } => match source {
                            Statement::WhileStatement(statement) => {
                                next = Some(Next::Loop {
                                    source: LoopNode::While(statement),
                                    output,
                                    depth,
                                    charged: false,
                                });
                            }
                            Statement::DoWhileStatement(statement) => {
                                next = Some(Next::Loop {
                                    source: LoopNode::DoWhile(statement),
                                    output,
                                    depth,
                                    charged: false,
                                });
                            }
                            _ => {
                                self.in_loop_body = true;
                                self.statement_depth = pending
                                    .last()
                                    .map_or(original_depth, |frame| frame.depth);
                                let completion = self.statement(source, &mut output, depth)?;
                                completed = Some((output, completion));
                            }
                        },
                    }
                    continue;
                }

                let (body, completion) = completed.take().expect("scheduled loop body");
                let Some(mut frame) = pending.pop() else {
                    *region = body;
                    return Ok(completion);
                };
                self.statement_depth = frame.depth;
                self.in_loop_body = frame.old_loop;
                self.bindings.truncate(frame.visible);
                match frame.kind {
                    Kind::While { source, condition } => {
                        Self::push(
                            &mut frame.output,
                            source,
                            Op::While {
                                condition,
                                body,
                                step: None,
                            },
                        );
                        completed = Some((frame.output, completion.pretest_loop()));
                    }
                    Kind::DoWhile { source } => {
                        // As in the original recursive lowering, the condition
                        // is admitted after the body even when the body returns.
                        let mut condition = self.region(&source.test);
                        let value = self.predicate(&source.test, &mut condition, 0)?;
                        condition.exit = Exit::Yield(value);
                        Self::push(
                            &mut frame.output,
                            source,
                            Op::DoWhile { body, condition },
                        );
                        completed = Some((frame.output, completion.do_loop()));
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
    fn mixed_nested_while_and_do_while_use_bounded_native_stack() {
        for (depth, admitted) in [(1, true), (16, true), (32, true), (65, false)] {
            let mut body = String::from("break;");
            for level in 0..depth {
                body = if level % 2 == 0 {
                    format!("while(a){body}")
                } else {
                    format!("do {body} while(a);")
                };
            }
            let source = format!("export function run(a,b){{{body}return b;}}");
            for typescript in [false, true] {
                let source = source.clone();
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile(&source, typescript))
                    .expect("small-stack source loop thread")
                    .join()
                    .expect("source loop lowering must not overflow");
                if admitted {
                    let artifact = result.unwrap_or_else(|error| panic!("{depth}: {error}"));
                    assert!(!artifact.wasm.is_empty());
                } else {
                    let error = result.err().expect("over-depth loop must be rejected");
                    assert!(
                        error.contains("64") || error.contains("128"),
                        "{depth} loops: {error}"
                    );
                }
            }
        }
    }
}
