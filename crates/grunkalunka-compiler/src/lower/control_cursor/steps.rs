//! Typed transitions for the shared heap-owned source-control cursor.
//!
//! A transition can only schedule a child or complete a region; it never calls
//! another source-control dispatcher through the Rust call stack. The small
//! scheduler in the parent module owns the worklist and continuation stack.
use super::*;
mod for_loop;

impl Lowering<'_, '_> {
    pub(super) fn begin_control<'n, 'a>(
        &mut self,
        source: ControlNode<'n, 'a>,
        output: Region,
        depth: usize,
        cursor: &mut Cursor<'n, 'a>,
    ) -> Result<(), String> {
        match source {
            ControlNode::TryFinally(source) => {
                if !self.allow_await {
                    return Err(unsupported(
                        source,
                        "statement is outside the numeric experiment",
                    ));
                }
                let outer_type = self.return_type;
                if !matches!(outer_type, Type::General | Type::AsyncValue) {
                    return Err(unsupported(
                        source,
                        "finally requires an async primitive completion",
                    ));
                }
                let visible = self.bindings.len();
                self.return_type = Type::AsyncValue;
                let body = self.region(source.block.as_ref());
                let has_handler = source.handler.is_some();
                if !has_handler {
                    self.charge_statement(source.block.as_ref())?;
                }
                cursor.pending.push(Frame::TryFinallyBody {
                    source,
                    output,
                    depth,
                    visible,
                    outer_type,
                });
                cursor.next = Some(Next::Enter {
                    // Catch and finally have distinct
                    // completions. The protected body
                    // remains a raw source sequence.
                    source: if has_handler {
                        ControlNode::TryCatch(source)
                    } else {
                        ControlNode::Sequence(&source.block.body)
                    },
                    output: body,
                    depth: if has_handler { depth } else { depth + 2 },
                    charge: source_control::SourceCharge::Charged,
                });
            }
            ControlNode::TryCatch(source) => {
                if !self.allow_await {
                    return Err(unsupported(
                        source,
                        "statement is outside the numeric experiment",
                    ));
                }
                source
                    .handler
                    .as_ref()
                    .ok_or_else(|| unsupported(source, "try requires a catch handler"))?;
                let visible = self.bindings.len();
                let body = self.region(source.block.as_ref());
                // The try body is a source block, but its
                // region is already the protected IR body.
                // Do not wrap it in another Op::Block.
                self.charge_statement(source.block.as_ref())?;
                cursor.pending.push(Frame::TryCatchBody {
                    source,
                    output,
                    depth,
                    visible,
                });
                cursor.next = Some(Next::Enter {
                    source: ControlNode::Sequence(&source.block.body),
                    output: body,
                    depth: depth + 2,
                    charge: source_control::SourceCharge::Charged,
                });
            }
            ControlNode::Sequence(statements) => {
                if let Some(statement) = statements.first() {
                    cursor.pending.push(Frame::Sequence {
                        statements,
                        index: 0,
                        depth,
                        completion: Completion::NORMAL,
                    });
                    cursor.next = Some(Next::Statement {
                        source: statement,
                        output,
                        depth,
                    });
                } else {
                    cursor.completed = Some(Outcome::Statement(output, Completion::NORMAL));
                }
            }
            ControlNode::Block(source) => {
                // The raw sequence cursor owns sibling
                // ordering. This frame owns only the lexical
                // boundary and the generated Block region.
                let child = self.region(source);
                let visible = self.bindings.len();
                cursor.pending.push(Frame::Block {
                    source,
                    output,
                    depth,
                    visible,
                });
                cursor.next = Some(Next::Enter {
                    source: ControlNode::Sequence(&source.body),
                    output: child,
                    depth: depth + 1,
                    charge: source_control::SourceCharge::Charged,
                });
            }
            ControlNode::If(source) => {
                // The condition is evaluated before its lexical branch Region
                // exists. The source frame suspends without holding a second
                // mutable borrow of the output Region.
                cursor.pending.push(Frame::IfPredicate { source, depth });
                cursor.next = Some(Next::Expression {
                    source: &source.test,
                    output,
                    depth: 0,
                    mode: evaluate::Mode::Predicate,
                });
            }
            ControlNode::While(source) => {
                // While tests execute before their bodies.
                let mut condition = self.region(&source.test);
                let predicate = self.predicate(&source.test, &mut condition, 0)?;
                condition.exit = Exit::Yield(predicate);
                let body = self.region(&source.body);
                let visible = self.bindings.len();
                let old_loop = self.in_loop_body;
                cursor.pending.push(Frame::While {
                    source,
                    output,
                    depth,
                    visible,
                    old_loop,
                    condition,
                });
                self.in_loop_body = true;
                cursor.next = Some(Next::Statement {
                    source: &source.body,
                    output: body,
                    depth: depth + 1,
                });
            }
            ControlNode::For(source) => {
                self.begin_for_loop(source, output, depth, cursor)?;
            }
            ControlNode::DoWhile(source) => {
                // Do-while tests are admitted only after the body.
                let body = self.region(&source.body);
                let visible = self.bindings.len();
                let old_loop = self.in_loop_body;
                cursor.pending.push(Frame::DoWhile {
                    source,
                    output,
                    depth,
                    visible,
                    old_loop,
                });
                self.in_loop_body = true;
                cursor.next = Some(Next::Statement {
                    source: &source.body,
                    output: body,
                    depth: depth + 1,
                });
            }
        }
        Ok(())
    }

    /// Resume one source-level consumer after the existing expression
    /// evaluator has delivered its typed result. No source node is charged
    /// twice and no child Region is allocated before the source expression.
    pub(super) fn resume_expression<'n, 'a>(
        &mut self,
        frame: Frame<'n, 'a>,
        output: Region,
        value: Option<ValueId>,
        cursor: &mut Cursor<'n, 'a>,
    ) -> Result<(), String> {
        match frame {
            Frame::IfPredicate { source, depth } => {
                let condition = value.expect("predicate mode yields a value");
                let consequent = self.region(&source.consequent);
                let visible = self.bindings.len();
                cursor.pending.push(Frame::If {
                    source,
                    output,
                    depth,
                    visible,
                    condition,
                    consequent: None,
                    consequent_returns: None,
                });
                cursor.next = Some(Next::Statement {
                    source: &source.consequent,
                    output: consequent,
                    depth: depth + 1,
                });
                Ok(())
            }
            _ => unreachable!("only expression continuations may receive value results"),
        }
    }

    pub(super) fn resume_control<'n, 'a>(
        &mut self,
        frame: Frame<'n, 'a>,
        finished_region: Region,
        completion: Completion,
        cursor: &mut Cursor<'n, 'a>,
    ) -> Result<(), String> {
        match frame {
            Frame::IfPredicate { .. } => {
                unreachable!("predicate continuation is resumed by a value result")
            }
            Frame::TryFinallyBody {
                source,
                output,
                depth,
                visible,
                outer_type,
            } => {
                self.bindings.truncate(visible);
                let finalizer = source
                    .finalizer
                    .as_ref()
                    .expect("TryFinally has a source finalizer");
                let cleanup = self.region(finalizer.as_ref());
                self.charge_statement(finalizer.as_ref())?;
                cursor.pending.push(Frame::TryFinallyCleanup {
                    source,
                    output,
                    depth,
                    visible,
                    outer_type,
                    body: finished_region,
                    body_completion: completion,
                });
                cursor.next = Some(Next::Enter {
                    source: ControlNode::Sequence(&finalizer.body),
                    output: cleanup,
                    depth: depth + 2,
                    charge: source_control::SourceCharge::Charged,
                });
            }
            Frame::TryFinallyCleanup {
                source,
                mut output,
                visible,
                outer_type,
                body,
                body_completion,
                ..
            } => {
                self.return_type = outer_type;
                self.bindings.truncate(visible);
                let returned = self.value(Type::AsyncValue);
                let reason = self.value(self.promise_parameters.finally_type(source.span.start));
                let mut continuation = self.region(source);
                let result = if outer_type == Type::AsyncValue {
                    returned
                } else {
                    self.resolve_async_value(source, returned, &mut continuation)
                };
                continuation.exit = Exit::Return(result);
                Self::push(
                    &mut output,
                    source,
                    Op::TryFinally {
                        body,
                        finalizer: finished_region,
                        returned,
                        reason,
                        continuation: Box::new(continuation),
                    },
                );
                cursor.completed = Some(Outcome::Statement(
                    output,
                    body_completion.finalized_by(completion),
                ));
            }
            Frame::TryCatchBody {
                source,
                output,
                depth,
                visible,
            } => {
                self.bindings.truncate(visible);
                let handler = source
                    .handler
                    .as_ref()
                    .expect("try/catch handler validated at entry");
                let mut caught = self.region(handler.body.as_ref());
                self.charge_statement(handler.body.as_ref())?;
                let reason_type = self.promise_parameters.catch_type(handler);
                let reason = self.value(reason_type);
                if let Some(parameter) = &handler.param {
                    let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern else {
                        return Err(unsupported(
                            parameter,
                            "catch destructuring is not admitted",
                        ));
                    };
                    if self.source_charging && self.local_count >= 128 {
                        return Err(unsupported(
                            parameter,
                            "at most 128 source locals are admitted",
                        ));
                    }
                    if self.source_charging {
                        self.local_count += 1;
                    }
                    let slot = SlotId(self.slots.len());
                    self.slots.push(Slot {
                        ty: reason_type,
                        owner: caught.id,
                        mutable: true,
                        parameter: false,
                        span: parameter.span(),
                    });
                    Self::push(
                        &mut caught,
                        parameter,
                        Op::Initialize {
                            slot,
                            value: reason,
                        },
                    );
                    self.bindings.push(NumericBinding {
                        symbol: identifier.symbol_id.get().ok_or("missing catch binding")?,
                        slot,
                        mutable: true,
                    });
                }
                cursor.pending.push(Frame::TryCatchHandler {
                    source,
                    output,
                    depth,
                    visible,
                    body: finished_region,
                    body_completion: completion,
                    reason,
                });
                cursor.next = Some(Next::Enter {
                    source: ControlNode::Sequence(&handler.body.body),
                    output: caught,
                    depth: depth + 2,
                    charge: source_control::SourceCharge::Charged,
                });
            }
            Frame::TryCatchHandler {
                source,
                mut output,
                visible,
                body,
                body_completion,
                reason,
                ..
            } => {
                self.bindings.truncate(visible);
                Self::push(
                    &mut output,
                    source,
                    Op::TryCatch {
                        body,
                        reason,
                        handler: finished_region,
                    },
                );
                cursor.completed = Some(Outcome::Statement(
                    output,
                    body_completion.caught_by(completion),
                ));
            }
            Frame::Sequence {
                statements,
                index,
                depth,
                completion: prefix,
            } => {
                let completion = prefix.then(completion);
                let next_index = index + 1;
                if let Some(statement) = statements.get(next_index) {
                    // Mirror the original source sequence's first
                    // diagnostic, while allowing erased TS declarations
                    // to follow an otherwise terminal completion.
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
                    cursor.pending.push(Frame::Sequence {
                        statements,
                        index: next_index,
                        depth,
                        completion,
                    });
                    cursor.next = Some(Next::Statement {
                        source: statement,
                        output: finished_region,
                        depth,
                    });
                } else {
                    cursor.completed = Some(Outcome::Statement(finished_region, completion));
                }
            }
            Frame::Block {
                source,
                mut output,
                visible,
                ..
            } => {
                self.bindings.truncate(visible);
                Self::push(&mut output, source, Op::Block(finished_region));
                cursor.completed = Some(Outcome::Statement(output, completion));
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
                    cursor.completed = Some(Outcome::Statement(output, first.union(completion)));
                    return Ok(());
                }

                // The consequent has finished. Its lexical declarations
                // must not be visible while lowering the alternate.
                let mut alternate = self.region(source);
                if let Some(alternate_source) = &source.alternate {
                    alternate.span = alternate_source.span();
                    cursor.pending.push(Frame::If {
                        source,
                        output,
                        depth,
                        visible,
                        condition,
                        consequent: Some(finished_region),
                        consequent_returns: Some(completion),
                    });
                    cursor.next = Some(Next::Statement {
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
                    cursor.completed = Some(Outcome::Statement(
                        output,
                        completion.union(Completion::NORMAL),
                    ));
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
                cursor.completed = Some(Outcome::Statement(output, completion.pretest_loop()));
            }
            Frame::For(frame) => {
                self.resume_for_loop(frame, finished_region, completion, cursor)?;
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
                cursor.completed = Some(Outcome::Statement(output, completion.do_loop()));
            }
        }
        Ok(())
    }
}
