//! Typed transitions for the shared heap-owned source-control cursor.
//!
//! A transition can only schedule a child or complete a region; it never calls
//! another source-control dispatcher through the Rust call stack. The small
//! scheduler in the parent module owns the worklist and continuation stack.
use super::*;
mod for_loop;
mod for_of;

impl<'s, 'ast> Lowering<'s, 'ast> {
    pub(super) fn begin_control<'n>(
        &mut self,
        source: ControlNode<'n, 'ast>,
        output: Region,
        depth: usize,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String> {
        match source {
            ControlNode::TryFinally(source) => {
                let outer_type = self.return_type;
                if self.allow_await && !matches!(outer_type, Type::General | Type::AsyncValue) {
                    return Err(unsupported(
                        source,
                        "finally requires an async primitive completion",
                    ));
                }
                let visible = self.bindings.len();
                // Async return adoption happens after cleanup. An ordinary
                // function instead keeps its existing private result lane.
                self.return_type = if self.allow_await {
                    Type::AsyncValue
                } else {
                    outer_type
                };
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
                // The condition Region is allocated before evaluating its
                // expression, and the body exists only after the result.
                let condition = self.region(&source.test);
                cursor.pending.push(Frame::WhilePredicate {
                    source,
                    output,
                    depth,
                });
                cursor.next = Some(Next::Expression {
                    source: &source.test,
                    output: condition,
                    depth: 0,
                    mode: evaluate::Mode::Predicate,
                });
            }
            ControlNode::For(source) => {
                self.begin_for_loop(source, output, depth, cursor)?;
            }
            ControlNode::ForOf(source) => {
                self.begin_for_of(source, output, depth, cursor)?;
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
    /// twice; child Region allocation retains each source form's prior order.
    pub(super) fn resume_expression<'n>(
        &mut self,
        frame: Frame<'n, 'ast>,
        output_region: Region,
        value: Option<ValueId>,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        match frame {
            Frame::LiftedExpression {
                completion,
                function_value,
                ..
            } => {
                if function_value {
                    self.leave_function_value();
                }
                let body = self.finish_lifted_body(completion, output_region, value);
                cursor.completed = Some(Outcome::LiftedBody(body));
                Ok(())
            }
            Frame::MaterializedFunction { state, .. } => self.resume_materialized_function(
                state,
                output_region,
                value.expect("materialized function result"),
                cursor,
            ),
            Frame::CheckExpression { function_value, .. } => {
                if function_value {
                    self.leave_function_value();
                }
                cursor.completed = Some(Outcome::CheckBody {
                    region: output_region,
                    factory: false,
                });
                Ok(())
            }
            Frame::EvaluateFunctionValue {
                mut expression,
                mode,
                node,
                resume,
                previous_result_depth,
                ..
            } => {
                *expression.current_region() = output_region;
                self.leave_function_value();
                self.restore_function_result_depth(previous_result_depth);
                match resume {
                    FunctionValueResume::Effect(effect) => {
                        self.complete_function_effect(expression.as_mut(), node, effect);
                    }
                    FunctionValueResume::Callback | FunctionValueResume::Callee => {
                        expression.resume_function_value(value.expect("function child result"))
                    }
                }
                self.queue_owned_expression(expression, mode, cursor);
                Ok(())
            }
            Frame::EvaluateFactoryArgument {
                arguments, kind, ..
            } => self.resume_factory_argument(arguments, kind, output_region, value, cursor),
            Frame::EvaluateRuntimeFactoryCallee { state, factory, .. } => self
                .resume_runtime_factory_callee(
                    state,
                    factory,
                    output_region,
                    value.expect("runtime factory callee"),
                    cursor,
                ),
            Frame::EvaluateFactoryValue { state, factory, .. } => self.finish_factory_value(
                state,
                factory,
                output_region,
                value.expect("concise function-result body value"),
                cursor,
            ),
            Frame::DeclarationValue { sequence, active } => self.resume_declaration_value(
                sequence,
                active,
                output_region,
                value.expect("declaration initializer value"),
                cursor,
            ),
            Frame::PatternBindingSource { sequence, binding } => self
                .resume_pattern_binding_source(
                    sequence,
                    binding,
                    output_region,
                    value.expect("pattern binding source"),
                    cursor,
                ),
            Frame::PatternBindingKey {
                resume,
                state,
                source,
            } => self.resume_pattern_binding_key(
                resume,
                *state,
                source,
                output_region,
                value.expect("computed binding key"),
                cursor,
            ),
            Frame::PatternBindingDefault { resume, active } => self.resume_pattern_binding_default(
                resume,
                *active,
                output_region,
                value.expect("pattern binding default"),
                cursor,
            ),
            Frame::MaterializedDeclaration {
                mut sequence,
                local,
                mutable,
                signature,
            } => {
                let mut output = output_region;
                self.finish_declaration_materialization(
                    local,
                    mutable,
                    signature,
                    &mut output,
                    value.expect("materialized declaration initializer"),
                )?;
                sequence.index += 1;
                self.advance_declaration(sequence, output, cursor)
            }
            Frame::DiscardExpression { .. } => {
                // Mode::Discard already evaluates side effects and produces
                // no statement value. Resume sequential completion normally.
                cursor.completed = Some(Outcome::Statement(output_region, Completion::NORMAL));
                Ok(())
            }
            Frame::FunctionReturn { .. } => {
                self.leave_function_value();
                let mut output = output_region;
                output.exit = Exit::Return(value.expect("function return result"));
                cursor.completed = Some(Outcome::Statement(output, Completion::RETURN));
                Ok(())
            }
            Frame::ReturnValue { .. } => {
                let mut output = output_region;
                output.exit = Exit::Return(value.expect("typed return expression result"));
                cursor.completed = Some(Outcome::Statement(output, Completion::RETURN));
                Ok(())
            }
            Frame::ThrowValue { .. } => {
                let mut output = output_region;
                output.exit = Exit::Throw(value.expect("throw reason"));
                cursor.completed = Some(Outcome::Statement(output, Completion::THROW));
                Ok(())
            }
            Frame::IfPredicate { source, depth } => {
                let condition = value.expect("predicate mode yields a value");
                let consequent = self.region(&source.consequent);
                let visible = self.bindings.len();
                cursor.pending.push(Frame::If {
                    source,
                    output: output_region,
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
            Frame::WhilePredicate {
                source,
                output,
                depth,
            } => {
                let predicate = value.expect("predicate mode yields a value");
                let mut condition = output_region;
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
                Ok(())
            }
            Frame::ForInitializer {
                source,
                output,
                depth,
                outer,
            } => self.finish_for_initializer(
                ForPredicateFrame {
                    source,
                    output,
                    header: output_region,
                    depth,
                    outer,
                    header_bindings: self.bindings.len(),
                    let_slots: Vec::new(),
                },
                cursor,
            ),
            Frame::ForPredicate(frame) => {
                let predicate = value.expect("predicate mode yields a value");
                self.resume_for_predicate(frame, output_region, predicate, cursor)
            }
            Frame::ForOfSource {
                source,
                output,
                depth,
                outer,
            } => {
                let condition = self.region(&source.right);
                self.resume_for_of_source(
                    Box::new(ForOfFrame {
                        source,
                        output,
                        header: output_region,
                        condition,
                        depth,
                        outer,
                        old_loop: self.in_loop_body,
                    }),
                    value.expect("for-of iterable"),
                    cursor,
                )
            }
            Frame::ForUpdate {
                frame,
                body,
                completion,
            } => {
                // Discard-mode evaluation has populated the step Region.
                self.finish_for_loop(frame, body, completion, Some(output_region), cursor)
            }
            Frame::DoWhilePredicate {
                source,
                mut output,
                body,
                body_completion,
                ..
            } => {
                let predicate = value.expect("predicate mode yields a value");
                let mut condition = output_region;
                condition.exit = Exit::Yield(predicate);
                Self::push(&mut output, source, Op::DoWhile { body, condition });
                cursor.completed = Some(Outcome::Statement(output, body_completion.do_loop()));
                Ok(())
            }
            _ => unreachable!("only expression continuations may receive value results"),
        }
    }

    pub(super) fn resume_control<'n>(
        &mut self,
        frame: Frame<'n, 'ast>,
        finished_region: Region,
        completion: Completion,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        match frame {
            Frame::AdvanceDeclaration { sequence } => {
                self.advance_declaration(sequence, finished_region, cursor)?;
            }
            Frame::ForDeclarationInitializer {
                source,
                output,
                depth,
                outer,
                start,
                mutable,
            } => {
                let let_slots = if mutable {
                    self.bindings[start..]
                        .iter()
                        .map(|binding| binding.slot)
                        .collect()
                } else {
                    Vec::new()
                };
                self.finish_for_initializer(
                    ForPredicateFrame {
                        source,
                        output,
                        header: finished_region,
                        depth,
                        outer,
                        header_bindings: self.bindings.len(),
                        let_slots,
                    },
                    cursor,
                )?;
            }
            Frame::LiftedBlock {
                completion: body_completion,
                activation,
                ..
            } => {
                let region = self.finish_arrow_block_activation(
                    activation,
                    finished_region,
                    Ok(completion),
                )?;
                let body = self.finish_lifted_body(body_completion, region, None);
                cursor.completed = Some(Outcome::LiftedBody(body));
            }
            Frame::CheckBlock {
                activation,
                factory,
                ..
            } => {
                let region = self.finish_arrow_block_activation(
                    activation,
                    finished_region,
                    Ok(completion),
                )?;
                cursor.completed = Some(Outcome::CheckBody { region, factory });
            }
            Frame::EvaluateBody {
                mut expression,
                mode,
                activation,
                node,
                result_type,
                ..
            } => {
                let body = self.finish_arrow_block_activation(
                    activation,
                    finished_region,
                    Ok(completion),
                )?;
                self.resume_lowered_block_body(expression.as_mut(), node, result_type, body);
                self.queue_owned_expression(expression, mode, cursor);
            }
            Frame::EvaluateFactoryBody {
                state,
                activation,
                factory,
                ..
            } => {
                self.finish_factory_body(
                    state,
                    activation,
                    factory,
                    finished_region,
                    completion,
                    cursor,
                )?;
            }
            Frame::ChildBoundary { .. }
            | Frame::CheckedFactoryExpression { .. }
            | Frame::CheckedFactoryResult { .. }
            | Frame::CheckedDeclarationFactory { .. }
            | Frame::CheckedMaterialization { .. }
            | Frame::MaterializedFunction { .. }
            | Frame::LiftedExpression { .. }
            | Frame::CheckedLocalCall { .. }
            | Frame::CheckExpression { .. }
            | Frame::CheckReturnedClosure { .. }
            | Frame::DiscardExpression { .. }
            | Frame::EvaluateFunctionValue { .. }
            | Frame::EvaluateRuntimeFactoryCallee { .. }
            | Frame::EvaluateFactoryArgument { .. }
            | Frame::EvaluateFactoryValue { .. }
            | Frame::DeclarationValue { .. }
            | Frame::PatternBindingSource { .. }
            | Frame::PatternBindingKey { .. }
            | Frame::PatternBindingDefault { .. }
            | Frame::CheckedDeclarationClosure { .. }
            | Frame::MaterializedDeclaration { .. }
            | Frame::ReturnValue { .. }
            | Frame::FunctionReturn { .. }
            | Frame::ThrowValue { .. }
            | Frame::IfPredicate { .. }
            | Frame::WhilePredicate { .. }
            | Frame::DoWhilePredicate { .. }
            | Frame::ForInitializer { .. }
            | Frame::ForOfSource { .. }
            | Frame::ForPredicate(_)
            | Frame::ForUpdate { .. } => {
                unreachable!("expression continuation is resumed by a value result")
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
                let captured_type = self.return_type;
                self.return_type = outer_type;
                self.bindings.truncate(visible);
                let returned = self.value(captured_type);
                let reason_type = if self.allow_await {
                    self.promise_parameters.finally_type(source.span.start)
                } else {
                    Type::General
                };
                let reason = self.value(reason_type);
                let mut continuation = self.region(source);
                let result = if self.allow_await && outer_type != Type::AsyncValue {
                    self.resolve_async_value(source, returned, &mut continuation)
                } else {
                    returned
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
            Frame::ForOf(frame) => {
                self.finish_for_of(*frame, finished_region, completion, cursor);
            }
            Frame::DoWhile {
                source,
                output,
                visible,
                old_loop,
                depth,
            } => {
                self.in_loop_body = old_loop;
                self.bindings.truncate(visible);
                // The source depth limit was checked before descending.
                // The test is still lowered after the body, even if the body
                // already returned and cannot fall through at runtime.
                let condition = self.region(&source.test);
                cursor.pending.push(Frame::DoWhilePredicate {
                    source,
                    output,
                    depth,
                    body: finished_region,
                    body_completion: completion,
                });
                cursor.next = Some(Next::Expression {
                    source: &source.test,
                    output: condition,
                    depth: 0,
                    mode: evaluate::Mode::Predicate,
                });
            }
        }
        Ok(())
    }
}
