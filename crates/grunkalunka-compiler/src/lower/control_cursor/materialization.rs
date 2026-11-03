//! Real lifted bodies use the same owned child switch as disposable checks,
//! but publish their nested functions and enclosing function into the parent.
use super::*;

impl<'s, 'ast> Lowering<'s, 'ast> {
    pub(super) fn start_function_materialization<'n>(
        &mut self,
        mut state: FactoryExpression<'n, 'ast>,
        closure: closures::LocalClosure,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        let node = state.node;
        let signature = state.signature;
        let output = Self::take_cursor_region(state.expression.current_region());
        // The caller's result context remains owned even if initial lifting
        // admission or the disposable definition check fails immediately.
        cursor.pending.push(Frame::MaterializedFunction {
            state,
            depth: self.statement_depth,
        });
        self.start_closure_materialization(closure, output, node, signature, cursor)
    }

    /// The caller publishes its own value consumer before transferring this
    /// Region. Direct declaration consumers need no extra function-value guard;
    /// expression consumers retain whichever admission they already own.
    pub(super) fn start_closure_materialization<'n>(
        &mut self,
        closure: closures::LocalClosure,
        output: Region,
        node: &'n Expression<'ast>,
        signature: usize,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        self.admit_lifted_closure(node)?;
        if !closure.checked {
            cursor.pending.push(Frame::CheckedMaterialization {
                output,
                node,
                signature,
                depth: self.statement_depth,
            });
            return self.start_closure_check(closure, 0, cursor);
        }
        self.start_checked_materialization(closure, output, node, signature, cursor)
    }

    pub(super) fn start_checked_materialization<'n>(
        &mut self,
        closure: closures::LocalClosure,
        output: Region,
        node: &'n Expression<'ast>,
        signature: usize,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        let function_values::PreparedLiftedClosure {
            lowering,
            body,
            completion,
        } = self.prepare_lifted_closure(closure, node, signature)?;
        let depth = self.statement_depth;
        let child = cursor.enter_lowering_child(self, lowering);
        cursor.pending.push(Frame::ChildBoundary {
            child,
            completion: ChildCompletion::LiftedBody {
                completion,
                output,
                node,
            },
            depth,
        });
        #[cfg(test)]
        BRIDGED_LIFTED_BODIES.with(|count| count.set(count.get() + 1));

        let Frame::ChildBoundary {
            completion: ChildCompletion::LiftedBody { completion, .. },
            ..
        } = cursor.pending.last().expect("lifted child boundary")
        else {
            unreachable!("just published lifted child boundary");
        };
        let (completion, request) = self.prepare_lifted_body(completion, body);
        match request {
            function_values::LiftedBodyRequest::Expression {
                source,
                output,
                mode,
            } => {
                let function_value = matches!(mode, evaluate::Mode::Function(_));
                if function_value {
                    self.enter_function_value(source)?;
                }
                cursor.pending.push(Frame::LiftedExpression {
                    completion,
                    function_value,
                    depth: self.statement_depth,
                });
                cursor.next = Some(Next::Expression {
                    source,
                    output,
                    depth: 0,
                    mode,
                });
            }
            function_values::LiftedBodyRequest::Block { arrow, result_type } => {
                let (activation, request) =
                    self.begin_arrow_block_activation(arrow, 0, result_type)?;
                cursor.pending.push(Frame::LiftedBlock {
                    completion,
                    activation,
                    depth: 0,
                });
                cursor.next = Some(Next::Enter {
                    source: ControlNode::Sequence(request.statements),
                    output: request.region,
                    depth: 0,
                    charge: source_control::SourceCharge::Charged,
                });
            }
        }
        Ok(())
    }

    pub(super) fn finish_materializing_child<'n>(
        &mut self,
        frame: Frame<'n, 'ast>,
        body: Region,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String> {
        let Frame::ChildBoundary {
            child,
            completion:
                ChildCompletion::LiftedBody {
                    completion,
                    mut output,
                    node,
                },
            depth,
        } = frame
        else {
            unreachable!("lifted body needs its materialization boundary");
        };
        let completed = cursor.leave_lowering_child(self, child);
        self.statement_depth = depth;
        let value = self.finish_lifted_closure(completion, completed, body, &mut output, node)?;
        cursor.completed = Some(Outcome::Expression(output, Some(value)));
        Ok(())
    }

    pub(super) fn resume_materialized_function<'n>(
        &mut self,
        mut state: FactoryExpression<'n, 'ast>,
        output: Region,
        value: ValueId,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        *state.expression.current_region() = output;
        self.restore_factory_expression(&state);
        state.expression.resume_function_value(value);
        self.resume_factory_expression(state, cursor)
    }
}
