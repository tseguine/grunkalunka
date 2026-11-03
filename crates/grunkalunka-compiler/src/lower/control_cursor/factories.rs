//! Returned-function factory transitions on the shared source scheduler.
use super::*;

impl<'s, 'ast> Lowering<'s, 'ast> {
    /// The callee consumer alone owns an extra function-value admission.
    /// Both consumers restore the same aggregate generated-region depth.
    pub(super) fn restore_factory_expression(&mut self, state: &FactoryExpression<'_, '_>) {
        if matches!(state.resume, FactoryResume::Callee(_)) {
            self.leave_function_value();
        }
        self.restore_function_result_depth(state.previous_result_depth);
    }

    pub(super) fn finish_factory_body<'n>(
        &mut self,
        mut state: FactoryExpression<'n, 'ast>,
        activation: closures::BlockArrowActivation,
        factory: function_values::ZeroBlockFactory,
        finished_region: Region,
        completion: Completion,
        cursor: &mut Cursor<'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        // Complete the body in the callee environment, emit its inline call,
        // then restore the caller before any outer-call arguments run.
        let body_result =
            self.finish_arrow_block_activation(activation, finished_region, Ok(completion));
        let emitted = body_result.map(|body| {
            self.resume_zero_block_factory(
                state.expression.as_mut(),
                state.node,
                state.signature,
                body,
            );
        });
        self.finish_zero_block_factory(factory);
        self.restore_factory_expression(&state);
        emitted?;

        if let FactoryResume::Callee(continuation) = state.resume {
            let callee = state
                .expression
                .pop_function_value()
                .expect("returned factory callee function");
            self.resume_sync_closure_callee(state.expression.as_mut(), continuation, callee)?;
        }
        self.advance_owned_expression(*state.expression, state.mode, cursor)
    }
}
