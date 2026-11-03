//! Owned Lowering children share the source cursor while retaining distinct
//! completion policies. Disposable checks publish metadata and source costs;
//! they never publish the child's generated IR or lifted function pool.
use super::*;

impl<'s, 'n, 'a> Cursor<'s, 'n, 'a> {
    pub(super) fn enter_lowering_child(
        &mut self,
        active: &mut Lowering<'s, 'a>,
        mut child: Box<Lowering<'s, 'a>>,
    ) -> LoweringChildId {
        std::mem::swap(active, child.as_mut());
        let id = LoweringChildId(self.parents.len());
        self.parents.push(child);
        id
    }

    pub(super) fn leave_lowering_child(
        &mut self,
        active: &mut Lowering<'s, 'a>,
        child: LoweringChildId,
    ) -> Box<Lowering<'s, 'a>> {
        assert_eq!(child.0 + 1, self.parents.len(), "LIFO child Lowering");
        let mut parent = self.parents.pop().expect("owned parent Lowering");
        std::mem::swap(active, parent.as_mut());
        // The same box now owns the completed child. Its caller decides which
        // metadata or real IR, if any, may be committed to the restored parent.
        parent
    }
}

impl<'s, 'ast> Lowering<'s, 'ast> {
    pub(super) fn start_closure_check<'n>(
        &mut self,
        mut closure: closures::LocalClosure,
        mut factory_depth: usize,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        // Direct returned-arrow chains enter more owned children without
        // recursively invoking this transition or another source dispatcher.
        loop {
            let arrow = closure.arrow;
            let closures::PreparedClosureCheck {
                lowering,
                mut region,
            } = self.prepare_closure_check(&closure, factory_depth)?;
            let depth = self.statement_depth;
            let child = cursor.enter_lowering_child(self, lowering);
            cursor.pending.push(Frame::ChildBoundary {
                child,
                completion: ChildCompletion::DefinitionCheck { closure },
                depth,
            });
            #[cfg(test)]
            BRIDGED_CLOSURE_CHECKS.with(|count| count.set(count.get() + 1));

            // Publish the boundary before body classification or admission can
            // fail, so diagnostic unwind always restores the correct parent.
            let Frame::ChildBoundary {
                completion: ChildCompletion::DefinitionCheck { closure: checking },
                ..
            } = cursor.pending.last_mut().expect("checking boundary")
            else {
                unreachable!("just published checking boundary");
            };
            match self.prepare_closure_check_body(checking)? {
                closures::ClosureCheckBody::Returned(returned) => {
                    if returned.checked {
                        self.capture_cells(&returned, &mut region);
                        cursor.completed = Some(Outcome::CheckBody {
                            region,
                            factory: true,
                        });
                        return Ok(());
                    }
                    cursor.pending.push(Frame::CheckReturnedClosure {
                        output: region,
                        depth: self.statement_depth,
                    });
                    closure = returned;
                    factory_depth += 1;
                }
                closures::ClosureCheckBody::Expression { source, mode } => {
                    let function_value = matches!(mode, evaluate::Mode::Function(_));
                    if function_value {
                        self.enter_function_value(source)?;
                    }
                    cursor.pending.push(Frame::CheckExpression {
                        function_value,
                        depth: self.statement_depth,
                    });
                    cursor.next = Some(Next::Expression {
                        source,
                        output: region,
                        depth: 0,
                        mode,
                    });
                    return Ok(());
                }
                closures::ClosureCheckBody::Block {
                    result_type,
                    factory,
                } => {
                    let arrow = self.arrow(arrow);
                    let (activation, request) =
                        self.begin_arrow_block_activation(arrow, 0, result_type)?;
                    cursor.pending.push(Frame::CheckBlock {
                        activation,
                        factory,
                        depth: 0,
                    });
                    cursor.next = Some(Next::Enter {
                        source: ControlNode::Sequence(request.statements),
                        output: request.region,
                        depth: 0,
                        charge: source_control::SourceCharge::Charged,
                    });
                    return Ok(());
                }
            }
        }
    }

    pub(super) fn finish_checking_child<'n>(
        &mut self,
        frame: Frame<'n, 'ast>,
        region: Region,
        factory: bool,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) {
        let Frame::ChildBoundary {
            child,
            completion: ChildCompletion::DefinitionCheck { mut closure },
            depth,
        } = frame
        else {
            unreachable!("checking body needs its child boundary");
        };
        if factory {
            closure.factory = true;
        }
        let completed = cursor.leave_lowering_child(self, child);
        self.statement_depth = depth;
        self.finish_closure_check(&completed, &mut closure, &region);
        cursor.completed = Some(Outcome::CheckedClosure(closure));
    }

    pub(super) fn resume_checked_closure<'n>(
        &mut self,
        frame: Frame<'n, 'ast>,
        closure: closures::LocalClosure,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        match frame {
            Frame::CheckedDeclarationClosure {
                sequence,
                local,
                output,
            } => self.resume_declaration_closure(sequence, local, closure, output, cursor),
            Frame::CheckedMaterialization {
                output,
                node,
                signature,
                ..
            } => self.start_checked_materialization(closure, output, node, signature, cursor),
            Frame::CheckedLocalCall {
                mut expression,
                mode,
                continuation,
                ..
            } => {
                self.resume_checked_local_call(expression.as_mut(), continuation, closure)?;
                self.queue_owned_expression(expression, mode, cursor);
                Ok(())
            }
            Frame::CheckReturnedClosure { mut output, .. } => {
                self.capture_cells(&closure, &mut output);
                cursor.completed = Some(Outcome::CheckBody {
                    region: output,
                    factory: true,
                });
                Ok(())
            }
            _ => unreachable!("checked closure needs a metadata consumer"),
        }
    }
}
