//! Release owned structured IR without native call depth proportional to
//! region nesting. Verification and code generation already have separate
//! iterative traversals; they cannot protect the implicit Rust drop glue.
use crate::ir::{Op, Operation, Region};

impl Drop for Region {
    fn drop(&mut self) {
        // Moving the entire operation vector out leaves this Region empty for
        // its automatic field drop. Every nested Region is similarly emptied
        // *before* dropping the Operation that owns it.
        let mut pending = std::mem::take(&mut self.operations);
        while let Some(mut operation) = pending.pop() {
            match &mut operation.kind {
                Op::Block(child)
                | Op::InlineCall { body: child, .. }
                | Op::AwaitCall { body: child, .. }
                | Op::SettlePromise { body: child, .. } => {
                    pending.append(&mut child.operations);
                }
                Op::If {
                    consequent,
                    alternate,
                    ..
                } => {
                    pending.append(&mut consequent.operations);
                    pending.append(&mut alternate.operations);
                }
                Op::While {
                    condition,
                    body,
                    step,
                } => {
                    pending.append(&mut condition.operations);
                    pending.append(&mut body.operations);
                    if let Some(step) = step {
                        pending.append(&mut step.operations);
                    }
                }
                Op::DoWhile { body, condition } => {
                    pending.append(&mut body.operations);
                    pending.append(&mut condition.operations);
                }
                Op::TryCatch { body, handler, .. } => {
                    pending.append(&mut body.operations);
                    pending.append(&mut handler.operations);
                }
                Op::TryFinally {
                    body,
                    finalizer,
                    continuation,
                    ..
                } => {
                    pending.append(&mut body.operations);
                    pending.append(&mut finalizer.operations);
                    pending.append(&mut continuation.operations);
                }
                // Leaf operations have no owned child Regions.
                _ => (),
            }
            // Each child Region in this operation is now empty. Dropping the
            // operation cannot recursively traverse deeper structured IR.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Exit, RegionId, ValueId};
    use oxc_span::Span;

    fn empty() -> Region {
        Region {
            id: RegionId(0),
            span: Span::new(0, 0),
            operations: Vec::new(),
            exit: Exit::Fallthrough,
        }
    }

    fn operation_with_child(form: usize, child: Region) -> Op {
        let value = ValueId(0);
        match form {
            0 => Op::Block(child),
            1 => Op::InlineCall {
                result: value,
                body: child,
            },
            2 => Op::AwaitCall {
                result: value,
                body: child,
            },
            3 => Op::SettlePromise {
                result: value,
                body: child,
            },
            4 => Op::If {
                result: None,
                condition: value,
                consequent: child,
                alternate: empty(),
            },
            5 => Op::If {
                result: None,
                condition: value,
                consequent: empty(),
                alternate: child,
            },
            6 => Op::While {
                condition: child,
                body: empty(),
                step: None,
            },
            7 => Op::While {
                condition: empty(),
                body: child,
                step: None,
            },
            8 => Op::While {
                condition: empty(),
                body: empty(),
                step: Some(child),
            },
            9 => Op::DoWhile {
                body: child,
                condition: empty(),
            },
            10 => Op::DoWhile {
                body: empty(),
                condition: child,
            },
            11 => Op::TryCatch {
                body: child,
                reason: value,
                handler: empty(),
            },
            12 => Op::TryCatch {
                body: empty(),
                reason: value,
                handler: child,
            },
            13 => Op::TryFinally {
                body: child,
                finalizer: empty(),
                returned: value,
                reason: value,
                continuation: Box::new(empty()),
            },
            14 => Op::TryFinally {
                body: empty(),
                finalizer: child,
                returned: value,
                reason: value,
                continuation: Box::new(empty()),
            },
            15 => Op::TryFinally {
                body: empty(),
                finalizer: empty(),
                returned: value,
                reason: value,
                continuation: Box::new(child),
            },
            _ => unreachable!("all nested Region variants"),
        }
    }

    #[test]
    fn all_region_child_positions_drop_on_two_megabyte_native_stack() {
        // Intentionally far beyond source admission and verifier budgets.
        // Each shape exercises a distinct nested Region ownership path.
        const DEPTH: usize = 4096;
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                for form in 0..16 {
                    let mut root = empty();
                    for level in 0..DEPTH {
                        root = Region {
                            id: RegionId(level + 1),
                            span: Span::new(0, 0),
                            operations: vec![Operation {
                                span: Span::new(0, 0),
                                kind: operation_with_child(form, root),
                            }],
                            exit: Exit::Fallthrough,
                        };
                    }
                    // Unlike existing inspector tests, no manual tree
                    // disassembly is needed before ownership is released.
                    drop(root);
                }
            })
            .expect("small-stack IR owner thread")
            .join()
            .expect("iterative structured IR destructor completed");
    }
}
