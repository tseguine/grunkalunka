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
                loop_label: None,
                inline_return: None,
                throw_to: None,
                needs_remainder: false,
                errors: false,
                gc_closures: false,
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
