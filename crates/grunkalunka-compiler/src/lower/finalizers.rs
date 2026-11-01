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
            return self.control_statement_iterative(
                control_cursor::ControlNode::TryCatch(statement),
                region,
                depth,
            );
        };
        let outer_type = self.return_type;
        if !matches!(outer_type, Type::General | Type::AsyncValue) {
            return Err(unsupported(
                statement,
                "finally requires an async primitive completion",
            ));
        }
        // A singleton nested source try inside a finally body is a
        // source-dependent continuation, not a reason to reenter the large
        // native finalizer frame. The iterative checker preserves the same
        // return-kind and source budgets as the ordinary path.
        if statement.handler.is_none()
            && matches!(
                statement.block.body.as_slice(),
                [Statement::TryStatement(_)]
            )
        {
            return self.try_finally_chain(statement, region, depth);
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
        let reason = self.value(self.promise_parameters.finally_type(statement.span.start));
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

#[cfg(test)]
mod stack_hygiene_tests {
    #[test]
    fn alternating_try_catch_and_if_blocks_stay_on_heap_continuations() {
        for depth in [2, 8, 16, 32] {
            let mut body = String::from("return a;");
            for index in 0..depth {
                body = if index % 2 == 0 {
                    format!("try{{{body}}}catch(e){{return b;}}")
                } else {
                    format!("if(a){{{body}}}else return b;")
                };
            }
            let source = format!("export async function run(a,b){{{body}}}");
            for ts in [false, true] {
                let source = source.clone();
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile_async(&source, ts))
                    .expect("small-stack mixed try/if compiler")
                    .join()
                    .expect("alternating try/catch source cannot overflow native stack");
                if depth <= 8 {
                    assert!(result.is_ok(), "{depth} controls: {:?}", result.err());
                } else if let Err(error) = result {
                    assert!(
                        error.contains("64")
                            || error.contains("128")
                            || error.contains("256")
                            || error.contains("nested"),
                        "{depth} controls: {error}"
                    );
                }
            }
        }
    }

    #[test]
    fn direct_nested_try_catch_chain_obeys_small_stack_and_source_limits() {
        for (depth, admitted) in [(1, true), (8, true), (16, true), (34, false)] {
            let mut body = String::from("return a;");
            for _ in 0..depth {
                body = format!("try{{{body}}}catch(e){{return b;}}");
            }
            let source = format!("export async function run(a,b){{{body}}}");
            for typescript in [false, true] {
                let source = source.clone();
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile_async(&source, typescript))
                    .expect("small-stack source try/catch thread")
                    .join()
                    .expect("nested try/catch must not overflow native stack");
                if admitted {
                    let artifact = result.unwrap_or_else(|error| panic!("{depth}: {error}"));
                    assert!(!artifact.wasm.is_empty());
                } else {
                    let error = result.err().expect("excessive source nesting must reject");
                    assert!(
                        error.contains("64") || error.contains("128"),
                        "{depth} nested try/catch: {error}"
                    );
                }
            }
        }
    }
}
