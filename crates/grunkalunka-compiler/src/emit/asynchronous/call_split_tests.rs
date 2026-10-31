//! Synthetic call-body continuation checks independent of source admission.
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

#[test]
fn inline_return_routes_to_its_caller_and_restores_loop_scope() {
    let inner = region(1, vec![], Exit::Return(ValueId(7)));
    let source = region(
        0,
        vec![Op::InlineCall {
            result: ValueId(3),
            body: inner,
        }],
        Exit::Return(ValueId(3)),
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
    let target = match &split.states[entry].end {
        End::Copy { input, result, next } => {
            assert_eq!((*input, *result), (ValueId(7), ValueId(3)));
            *next
        }
        _ => panic!("inline return must copy into the caller's result"),
    };
    assert!(matches!(&split.states[target].end, End::Return(ValueId(3))));
    assert!(split.return_to.is_none());

    let invalid = region(
        2,
        vec![Op::InlineCall {
            result: ValueId(3),
            body: region(3, vec![], Exit::Continue),
        }],
        Exit::Fallthrough,
    );
    let error = split
        .region(
            &invalid,
            normal,
            None,
            Some(LoopTargets { exit: 8, next: 9 }),
        )
        .expect_err("inline call must not inherit the source loop target");
    assert!(error.contains("async continue lacks a loop target"), "{error}");
    assert!(split.return_to.is_none());
}

#[test]
fn awaited_call_return_and_throw_keep_distinct_settlement_continuations() {
    for (exit, rejected) in [
        (Exit::Return(ValueId(7)), false),
        (Exit::Throw(ValueId(7)), true),
    ] {
        let source = region(
            0,
            vec![Op::AwaitCall {
                result: ValueId(3),
                body: region(1, vec![], exit),
            }],
            Exit::Return(ValueId(3)),
        );
        let mut split = Splitter::default();
        let normal = split.terminal(End::Trap).unwrap();
        let entry = split.region(&source, normal, None, None).unwrap();
        let suspended = match &split.states[entry].end {
            End::Copy { input, result, next } => {
                assert_eq!((*input, *result), (ValueId(7), ValueId(3)));
                *next
            }
            _ => panic!("awaited body completion must copy into its caller"),
        };
        let End::Suspend {
            input,
            rejected: observed,
            resume,
        } = &split.states[suspended].end
        else {
            panic!("awaited body completion must pass the caller's reaction boundary");
        };
        assert_eq!(*input, ValueId(3));
        assert_eq!(*observed, rejected);
        assert!(matches!(&split.states[*resume].end, End::Jump(_)));
        assert!(split.return_to.is_none());
        assert!(split.catch_to.is_none());
    }
}

#[test]
fn deep_inline_calls_split_on_two_megabyte_native_stacks() {
    // Every level owns its own return continuation. Keep the synthetic graph
    // below the real MAX_STATES bound while exceeding native recursive depth.
    const DEPTH: usize = 1800;
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut source = region(DEPTH, vec![], Exit::Return(ValueId(7)));
            for level in 0..DEPTH {
                source = region(
                    level,
                    vec![Op::InlineCall {
                        result: ValueId(3),
                        body: source,
                    }],
                    Exit::Return(ValueId(3)),
                );
            }
            let mut split = Splitter::default();
            let normal = split.terminal(End::Trap).unwrap();
            let entry = split.region(&source, normal, None, None).unwrap();
            assert!(matches!(&split.states[entry].end, End::Copy { .. }));
            assert_eq!(split.states.len(), DEPTH * 2 + 2);
            assert!(split.return_to.is_none());
            assert!(split.catch_to.is_none());
        })
        .expect("small-stack nested inline call thread")
        .join()
        .expect("iterative inline call bodies completed");
}
