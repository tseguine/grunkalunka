//! Heap-owned continuations for source IfStatement control flow.
//! This complements iterative expression evaluation and block sequences.
use super::*;
use oxc_ast::ast::IfStatement;

impl Lowering<'_, '_> {
    #[inline(never)]
    pub(super) fn if_statement_iterative<'n, 'a>(
        &mut self,
        root: &'n IfStatement<'a>,
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String> {
        // The root statement was charged by statement_inner. Nested If nodes
        // are charged here with exactly the same source and expansion limits.
        // Move its current operation buffer into an owned Region so nested
        // branches and their completion continuations live on the heap.
        type Next<'n, 'a> = source_control::StatementTask<'n, 'a, &'n IfStatement<'a>>;

        struct Frame<'n, 'a> {
            source: &'n IfStatement<'a>,
            output: Region,
            depth: usize,
            visible: usize,
            condition: ValueId,
            consequent: Option<Region>,
            consequent_returns: Option<Completion>,
        }

        let original_depth = self.statement_depth;
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
                            mut output,
                            depth,
                            charge,
                        } => {
                            self.statement_depth = depth;
                            if charge == source_control::SourceCharge::Uncharged {
                                self.admit_source_statement(source, depth)?;
                            }

                            let condition = self.predicate(&source.test, &mut output, 0)?;
                            let consequent = self.region(&source.consequent);
                            let visible = self.bindings.len();
                            pending.push(Frame {
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
                        Next::Statement {
                            source,
                            mut output,
                            depth,
                        } => {
                            if let Statement::IfStatement(inner) = source {
                                next = Some(Next::Enter {
                                    source: inner,
                                    output,
                                    depth,
                                    charge: source_control::SourceCharge::Uncharged,
                                });
                            } else {
                                self.statement_depth =
                                    pending.last().map_or(original_depth, |frame| frame.depth);
                                let completion = self.statement(source, &mut output, depth)?;
                                completed = Some((output, completion));
                            }
                        }
                    }
                    continue;
                }

                let (finished_region, completion) =
                    completed.take().expect("scheduled source If child");
                let Some(mut frame) = pending.pop() else {
                    *region = finished_region;
                    return Ok(completion);
                };

                self.statement_depth = frame.depth;
                self.bindings.truncate(frame.visible);
                if let Some(consequent) = frame.consequent.take() {
                    // Both sides are now lowered. Each side owns a distinct
                    // generated Region and the original completion union.
                    let consequent_returns = frame
                        .consequent_returns
                        .expect("scheduled consequent completion");
                    Self::push(
                        &mut frame.output,
                        frame.source,
                        Op::If {
                            result: None,
                            condition: frame.condition,
                            consequent,
                            alternate: finished_region,
                        },
                    );
                    completed = Some((frame.output, consequent_returns.union(completion)));
                    continue;
                }

                // Consequent finished: restore its lexical scope before
                // allocating and evaluating the alternate, even when absent.
                frame.consequent = Some(finished_region);
                frame.consequent_returns = Some(completion);
                let mut alternate = self.region(frame.source);
                if let Some(source) = &frame.source.alternate {
                    alternate.span = source.span();
                    let child_depth = frame.depth + 1;
                    pending.push(frame);
                    next = Some(Next::Statement {
                        source,
                        output: alternate,
                        depth: child_depth,
                    });
                } else {
                    Self::push(
                        &mut frame.output,
                        frame.source,
                        Op::If {
                            result: None,
                            condition: frame.condition,
                            consequent: frame.consequent.take().expect("consequent Region"),
                            alternate,
                        },
                    );
                    completed = Some((frame.output, completion.union(Completion::NORMAL)));
                }
            }
        })();
        self.statement_depth = original_depth;
        result
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn nested_consequent_and_alternate_ifs_respect_two_mib_stack_and_depth_budget() {
        for (depth, admitted) in [(1, true), (16, true), (64, true), (65, false)] {
            let mut body = String::from("return a;");
            for _ in 0..depth {
                body = format!("if(a){body}else return b;");
            }
            let source = format!("export function run(a,b){{{body}}}");
            for typescript in [false, true] {
                let source = source.clone();
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile(&source, typescript))
                    .expect("native small-stack compiler thread")
                    .join()
                    .expect("nested source If must not overflow native stack");
                if admitted {
                    let artifact = result.unwrap_or_else(|error| panic!("{depth}: {error}"));
                    assert!(!artifact.wasm.is_empty());
                } else {
                    let error = result.err().expect("over-depth source must be rejected");
                    assert!(
                        error.contains("statement nesting exceeds 64"),
                        "{depth} if statements: {error}"
                    );
                }
            }
        }
    }

    #[test]
    fn nested_else_if_regions_keep_both_completion_paths() {
        let mut body = String::from("return b;");
        for _ in 0..24 {
            body = format!("if(a)return a;else {body}");
        }
        let source = format!("export function run(a,b){{{body}}}");
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                crate::compile(&source, false).expect("nested else-if branches");
            })
            .expect("small stack else-if thread")
            .join()
            .expect("else-if compiler must not overflow");
    }
}
