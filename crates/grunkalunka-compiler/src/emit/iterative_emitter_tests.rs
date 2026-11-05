//! Synthetic IR depth probes the emitter, not the source admission policy.
use super::*;
use crate::ir::{Operation, RegionId};
use oxc_span::Span;

#[test]
fn structured_wat_emission_keeps_deep_blocks_off_native_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let span = Span::new(0, 0);
            const DEPTH: usize = 4096;
            let mut body = Region {
                id: RegionId(0),
                span,
                operations: Vec::new(),
                exit: Exit::Fallthrough,
            };
            for _ in 0..DEPTH {
                body = Region {
                    id: RegionId(0),
                    span,
                    operations: vec![Operation {
                        span,
                        kind: Op::Block(body),
                    }],
                    exit: Exit::Fallthrough,
                };
            }
            let function = Function {
                return_type: Type::Number,
                parameter_count: 0,
                slots: Vec::new(),
                values: Vec::new(),
                body,
            };
            let mut emitter = Emitter {
                function: &function,
                slot_offsets: Vec::new(),
                value_offsets: Vec::new(),
                cell_offsets: Vec::new(),
                arena_temp: None,
                call_boundary: None,
                output: String::new(),
                loop_to: None,
                return_to: None,
                throw_to: None,
                finally_owner: 0,
                needs_remainder: false,
                errors: false,
                error_kinds: false,
                gc_closures: false,
                function_index_offset: 0,
            };
            emitter.region(&function.body);
            assert!(emitter.output.is_empty());
            drop(emitter);

            let mut current = function.body;
            for _ in 0..DEPTH {
                let op = current.operations.pop().expect("nested block");
                let Op::Block(child) = op.kind else {
                    panic!("expected nested block")
                };
                current = child;
            }
            assert!(current.operations.is_empty());
        })
        .expect("stack probe thread")
        .join()
        .expect("stack probe completed");
}

#[test]
fn structured_finally_emission_is_linear_on_a_small_native_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let span = Span::new(0, 0);
            const DEPTH: usize = 4096;
            let mut values = vec![Type::Number];
            let mut body = Region {
                id: RegionId(0),
                span,
                operations: vec![Operation {
                    span,
                    kind: Op::Constant {
                        result: ValueId(0),
                        value: 1.0,
                    },
                }],
                exit: Exit::Return(ValueId(0)),
            };
            for level in 0..DEPTH {
                let base = values.len();
                values.extend([Type::Number, Type::General, Type::Number]);
                let returned = ValueId(base);
                let reason = ValueId(base + 1);
                let marker = ValueId(base + 2);
                body = Region {
                    id: RegionId(3 * level + 3),
                    span,
                    operations: vec![Operation {
                        span,
                        kind: Op::TryFinally {
                            body,
                            finalizer: Region {
                                id: RegionId(3 * level + 1),
                                span,
                                operations: vec![Operation {
                                    span,
                                    kind: Op::Constant {
                                        result: marker,
                                        value: 173.25,
                                    },
                                }],
                                exit: Exit::Fallthrough,
                            },
                            returned,
                            reason,
                            continuation: Box::new(Region {
                                id: RegionId(3 * level + 2),
                                span,
                                operations: Vec::new(),
                                exit: Exit::Return(returned),
                            }),
                        },
                    }],
                    exit: Exit::Fallthrough,
                };
            }
            let function = Function {
                return_type: Type::Number,
                parameter_count: 0,
                slots: Vec::new(),
                values,
                body,
            };
            assert!(!region_has_control(&function.body, true));
            let (output, _) = emit_function(
                &function,
                "(func (result f64)\n".into(),
                false,
                false,
                false,
                0,
            );
            assert_eq!(output.matches("f64.const 173.25\n").count(), DEPTH);
            assert!(output.len() < DEPTH * 2048);

            // The synthetic depth exceeds source limits. Drain the one deep
            // child explicitly so destruction is not part of this probe.
            let mut current = function.body;
            for _ in 0..DEPTH {
                let operation = current.operations.pop().expect("nested finally");
                let Op::TryFinally { body, .. } = operation.kind else {
                    panic!("nested finally");
                };
                current = body;
            }
            assert!(matches!(current.exit, Exit::Return(ValueId(0))));
        })
        .expect("small-stack finally emitter thread")
        .join()
        .expect("small-stack finally emitter completed");
}
