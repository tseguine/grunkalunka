//! Direct nested source try/finally completion lives on heap continuations.
//! The compiler's IR async splitter is independently iterative; source
//! lowering must also avoid a Rust frame for each lexical cleanup layer.
use super::*;
use oxc_ast::ast::TryStatement;

impl Lowering<'_, '_> {
    #[inline(never)]
    pub(super) fn try_finally_chain<'n, 'a>(
        &mut self,
        root: &'n TryStatement<'a>,
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String> {
        struct Frame<'n, 'a> {
            source: &'n TryStatement<'a>,
            body: Region,
            depth: usize,
            visible: usize,
            outer_type: Type,
        }

        let initial_type = self.return_type;
        let initial_visible = self.bindings.len();
        let initial_depth = self.statement_depth;
        let result = (|| {
            let mut pending: Vec<Frame<'n, 'a>> = Vec::new();
            let mut current = root;
            let mut current_depth = depth;
            // Each body's only source statement is the next TryStatement.
            // Admit that statement exactly once when we advance to it.
            while current.handler.is_none() && current.finalizer.is_some() {
                let [Statement::TryStatement(inner)] = current.block.body.as_slice() else {
                    break;
                };
                if !self.allow_await {
                    return Err(unsupported(
                        current,
                        "statement is outside the numeric experiment",
                    ));
                }
                let outer_type = self.return_type;
                if !matches!(outer_type, Type::General | Type::AsyncValue) {
                    return Err(unsupported(
                        current,
                        "finally requires an async primitive completion",
                    ));
                }
                let visible = self.bindings.len();
                self.return_type = Type::AsyncValue;
                let body = self.region(current.block.as_ref());
                self.charge_statement(current.block.as_ref())?;
                pending.push(Frame {
                    source: current,
                    body,
                    depth: current_depth,
                    visible,
                    outer_type,
                });

                current = inner;
                current_depth += 2;
                self.statement_depth = current_depth;
                if self.active_block_calls != 0 && self.inline_depth + current_depth > 128 {
                    return Err(unsupported(
                        current,
                        "block-arrow expansion exceeds 128 nested regions",
                    ));
                }
                if current_depth > 64 {
                    return Err(unsupported(current, "statement nesting exceeds 64"));
                }
                self.charge_statement(current)?;
            }

            // The final node is lowered once through the existing trusted
            // implementation. Every peeled parent finishes by unwinding
            // its own cleanup, return-kind and completion continuation.
            let body = &mut pending
                .last_mut()
                .expect("direct nested finalizer has a parent")
                .body;
            let mut completion = self.try_statement(current, body, current_depth)?;

            while let Some(frame) = pending.pop() {
                let statement = frame.source;
                self.statement_depth = frame.depth;
                self.bindings.truncate(frame.visible);
                let finalizer = statement
                    .finalizer
                    .as_ref()
                    .expect("peeled try/finally has cleanup");
                let mut cleanup = self.region(finalizer.as_ref());
                self.charge_statement(finalizer.as_ref())?;
                let cleanup_completion =
                    self.sequence(&finalizer.body, &mut cleanup, frame.depth + 2)?;
                self.return_type = frame.outer_type;
                self.bindings.truncate(frame.visible);

                let returned = self.value(Type::AsyncValue);
                let reason = self.value(self.promise_parameters.finally_type(statement.span.start));
                let mut continuation = self.region(statement);
                let result = if frame.outer_type == Type::AsyncValue {
                    returned
                } else {
                    self.resolve_async_value(statement, returned, &mut continuation)
                };
                continuation.exit = Exit::Return(result);
                let operation = Op::TryFinally {
                    body: frame.body,
                    finalizer: cleanup,
                    returned,
                    reason,
                    continuation: Box::new(continuation),
                };
                if let Some(parent) = pending.last_mut() {
                    Self::push(&mut parent.body, statement, operation);
                } else {
                    Self::push(region, statement, operation);
                }
                completion = completion.finalized_by(cleanup_completion);
            }
            Ok(completion)
        })();
        self.return_type = initial_type;
        self.bindings.truncate(initial_visible);
        self.statement_depth = initial_depth;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_nested_finalizers_keep_small_stack_and_original_depth_guards() {
        for depth in [1, 2, 4, 8, 33] {
            let mut body = String::from("return a;");
            for _ in 0..depth {
                body = format!("try{{{body}}}finally{{await b;}}");
            }
            let source = format!("export async function run(a,b){{{body}}}");
            for typescript in [false, true] {
                let source = source.clone();
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile_async(&source, typescript))
                    .expect("2 MiB compiler thread")
                    .join()
                    .expect("source cleanup nesting must not overflow");
                if depth <= 4 {
                    let artifact = result.unwrap_or_else(|error| panic!("{depth}: {error}"));
                    assert!(!artifact.wasm.is_empty());
                } else if let Err(error) = result {
                    assert!(
                        error.contains("64")
                            || error.contains("128")
                            || error.contains("4096")
                            || error.contains("nest"),
                        "{depth} nested finalizers: {error}",
                    );
                }
            }
        }
    }
}
