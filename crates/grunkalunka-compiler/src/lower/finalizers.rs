//! Source finally scopes retain raw completion values until cleanup has run.
use super::*;
use oxc_ast::ast::TryStatement;

impl Lowering<'_, '_> {
    pub(super) fn try_statement(
        &mut self,
        statement: &TryStatement<'_>,
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String> {
        if !self.allow_await {
            return Err(unsupported(
                statement,
                "statement is outside the numeric experiment",
            ));
        }
        let Some(finalizer) = &statement.finalizer else {
            return self.try_catch(statement, region, depth);
        };
        let outer_type = self.return_type;
        if !matches!(outer_type, Type::General | Type::AsyncValue) {
            return Err(unsupported(
                statement,
                "finally requires an async primitive completion",
            ));
        }
        let visible = self.bindings.len();
        self.return_type = Type::AsyncValue;
        let parts: Result<_, String> = (|| {
            let mut body = self.region(statement.block.as_ref());
            let body_completion = if statement.handler.is_some() {
                self.try_catch(statement, &mut body, depth)?
            } else {
                self.charge_statement(statement.block.as_ref())?;
                self.sequence(&statement.block.body, &mut body, depth + 2)?
            };
            self.bindings.truncate(visible);
            let mut cleanup = self.region(finalizer.as_ref());
            self.charge_statement(finalizer.as_ref())?;
            let cleanup_completion = self.sequence(&finalizer.body, &mut cleanup, depth + 2)?;
            Ok((body, body_completion, cleanup, cleanup_completion))
        })();
        self.return_type = outer_type;
        self.bindings.truncate(visible);
        let (body, body_completion, finalizer, finalizer_completion) = parts?;
        let returned = self.value(Type::AsyncValue);
        let reason = self.value(Type::General);
        let mut continuation = self.region(statement);
        let result = if outer_type == Type::AsyncValue {
            returned
        } else {
            self.resolve_async_value(statement, returned, &mut continuation)
        };
        continuation.exit = Exit::Return(result);
        Self::push(
            region,
            statement,
            Op::TryFinally {
                body,
                finalizer,
                returned,
                reason,
                continuation: Box::new(continuation),
            },
        );
        Ok(body_completion.finalized_by(finalizer_completion))
    }

    fn try_catch(
        &mut self,
        statement: &TryStatement<'_>,
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String> {
        let handler = statement
            .handler
            .as_ref()
            .ok_or_else(|| unsupported(statement, "try requires a catch handler"))?;
        let visible = self.bindings.len();
        let mut body = self.region(statement.block.as_ref());
        self.charge_statement(statement.block.as_ref())?;
        let body_completion = self.sequence(&statement.block.body, &mut body, depth + 2)?;
        self.bindings.truncate(visible);
        let mut caught = self.region(handler.body.as_ref());
        self.charge_statement(handler.body.as_ref())?;
        let reason = self.value(Type::General);
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
                ty: Type::General,
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
        let handler_completion = self.sequence(&handler.body.body, &mut caught, depth + 2)?;
        self.bindings.truncate(visible);
        Self::push(
            region,
            statement,
            Op::TryCatch {
                body,
                reason,
                handler: caught,
            },
        );
        Ok(body_completion.caught_by(handler_completion))
    }
}
