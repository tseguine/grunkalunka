//! Typed finally-continuation ordering without recursive native region calls.
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

fn finally_region(body: Region, finalizer: Region, continuation: Region) -> Region {
    region(
        0,
        vec![Op::TryFinally {
            body,
            finalizer,
            returned: ValueId(3),
            reason: ValueId(4),
            continuation: Box::new(continuation),
        }],
        Exit::Fallthrough,
    )
}

#[test]
fn finalizer_preserves_pending_return_and_throw_channels() {
    for (exit, expected_result) in [
        (Exit::Return(ValueId(7)), ValueId(3)),
        (Exit::Throw(ValueId(7)), ValueId(4)),
    ] {
        let source = finally_region(
            region(1, vec![], exit),
            region(2, vec![], Exit::Fallthrough),
            region(3, vec![], Exit::Return(ValueId(3))),
        );
        let mut split = Splitter::default();
        let normal = split.terminal(End::Trap).unwrap();
        let entry = split.region(&source, normal, None, None).unwrap();
        let target = match &split.states[entry].end {
            End::Copy {
                input,
                result,
                next,
            } => {
                assert_eq!((*input, *result), (ValueId(7), expected_result));
                *next
            }
            _ => panic!("protected completion must save the original value"),
        };
        if matches!(exit, Exit::Return(_)) {
            assert!(matches!(&split.states[target].end, End::Return(ValueId(3))));
        } else {
            assert!(matches!(&split.states[target].end, End::Reject(ValueId(4))));
        }
        assert!(split.return_to.is_none());
        assert!(split.catch_to.is_none());
    }
}

#[test]
fn finalizer_loop_exit_targets_preserve_nearest_break_and_continue() {
    for (exit, expected) in [(Exit::Break, 8), (Exit::Continue, 9)] {
        let source = finally_region(
            region(1, vec![], exit),
            region(2, vec![], Exit::Fallthrough),
            region(3, vec![], Exit::Fallthrough),
        );
        let mut split = Splitter::default();
        let normal = split.terminal(End::Trap).unwrap();
        let entry = split
            .region(
                &source,
                normal,
                None,
                Some(LoopTargets { exit: 8, next: 9 }),
            )
            .unwrap();
        assert_eq!(entry, expected);
        assert!(split.return_to.is_none());
        assert!(split.catch_to.is_none());
    }
}

#[test]
fn nested_finally_regions_use_bounded_native_stack() {
    const DEPTH: usize = 2048;
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut source = region(DEPTH, vec![], Exit::Return(ValueId(7)));
            for level in 0..DEPTH {
                source = region(
                    level,
                    vec![Op::TryFinally {
                        body: source,
                        finalizer: region(DEPTH + level + 1, vec![], Exit::Fallthrough),
                        returned: ValueId(3),
                        reason: ValueId(4),
                        continuation: Box::new(region(
                            DEPTH * 2 + level + 1,
                            vec![],
                            Exit::Fallthrough,
                        )),
                    }],
                    Exit::Fallthrough,
                );
            }
            let mut split = Splitter::default();
            let normal = split.terminal(End::Trap).unwrap();
            let entry = split.region(&source, normal, None, None).unwrap();
            assert_eq!(split.states.len(), DEPTH + 2);
            assert!(matches!(&split.states[entry].end, End::Copy { .. }));
            assert!(split.return_to.is_none());
            assert!(split.catch_to.is_none());
        })
        .expect("small-stack nested-finally thread")
        .join()
        .expect("iterative finalizer expansions completed");
}

#[test]
fn invalid_finalizer_completion_does_not_leak_lexical_targets() {
    let source = finally_region(
        region(1, vec![], Exit::Return(ValueId(7))),
        region(2, vec![], Exit::Yield(ValueId(8))),
        region(3, vec![], Exit::Return(ValueId(3))),
    );
    let mut split = Splitter::default();
    let normal = split.terminal(End::Trap).unwrap();
    let error = split.region(&source, normal, None, None).unwrap_err();
    assert!(error.contains("async yield lacks a target"), "{error}");
    assert!(split.return_to.is_none());
    assert!(split.catch_to.is_none());
}
