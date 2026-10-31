//! Mixed region ownership and dynamic target scopes across the async work stack.
use super::*;
use crate::ir::{Operation, RegionId};
use oxc_span::Span;

fn region(id: usize, operations: Vec<Op>, exit: Exit) -> Region {
    let span = Span::new(0, 0);
    Region {
        id: RegionId(id),
        span,
        operations: operations
            .into_iter()
            .map(|kind| Operation { span, kind })
            .collect(),
        exit,
    }
}

fn empty() -> Region {
    region(0, vec![], Exit::Fallthrough)
}

#[test]
fn interleaved_async_control_regions_split_without_native_stack_growth() {
    // Repeating distinct kinds matters: nested Block/If/try/call/loop/finally
    // changes which lexical target must be in force for the next inner kind.
    // No arbitrary-depth source admission follows from this synthetic IR.
    const DEPTH: usize = 1024;
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut inner = region(DEPTH + 1, vec![], Exit::Return(ValueId(7)));
            for level in 0..DEPTH {
                let kind = match level % 7 {
                    0 => Op::Block(inner),
                    1 => Op::If {
                        result: None,
                        condition: ValueId(1),
                        consequent: inner,
                        alternate: empty(),
                    },
                    2 => Op::TryCatch {
                        body: inner,
                        reason: ValueId(2),
                        handler: empty(),
                    },
                    3 => Op::InlineCall {
                        result: ValueId(3),
                        body: inner,
                    },
                    4 => Op::While {
                        condition: region(DEPTH + level + 2, vec![], Exit::Yield(ValueId(1))),
                        body: inner,
                        step: None,
                    },
                    5 => Op::DoWhile {
                        body: inner,
                        condition: region(DEPTH + level + 2, vec![], Exit::Yield(ValueId(1))),
                    },
                    6 => Op::TryFinally {
                        body: inner,
                        finalizer: empty(),
                        returned: ValueId(4),
                        reason: ValueId(5),
                        continuation: Box::new(empty()),
                    },
                    _ => unreachable!(),
                };
                inner = region(level, vec![kind], Exit::Fallthrough);
            }
            let source = region(
                DEPTH * 3,
                vec![Op::AwaitCall {
                    result: ValueId(6),
                    body: inner,
                }],
                Exit::Return(ValueId(6)),
            );
            let mut split = Splitter::default();
            let normal = split.terminal(End::Trap).unwrap();
            let entry = split.region(&source, normal, None, None).unwrap();
            assert!(entry < split.states.len());
            assert!(split.states.len() < MAX_STATES);
            assert_eq!(split.awaits, 1);
            assert!(split.return_to.is_none());
            assert!(split.catch_to.is_none());
        })
        .expect("small-stack composed async regions thread")
        .join()
        .expect("interleaved control regions completed without native recursion");
}
