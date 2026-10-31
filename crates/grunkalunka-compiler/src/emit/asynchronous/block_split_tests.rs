//! Owned-region synthetic probes for the async splitter's explicit Block stack.
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

fn constant(value: usize) -> Op {
    Op::Constant {
        result: ValueId(value),
        value: value as f64,
    }
}

fn assert_constant_chain(split: &Splitter<'_>, mut entry: usize, values: &[usize]) {
    for value in values {
        let state = &split.states[entry];
        assert_eq!(state.operations.len(), 1);
        assert!(matches!(
            state.operations[0].kind,
            Op::Constant {
                result,
                ..
            } if result == ValueId(*value)
        ));
        let End::Jump(next) = state.end else {
            panic!("expected atomic operation followed by a jump");
        };
        entry = next;
    }
    assert!(matches!(split.states[entry].end, End::Return(ValueId(7))));
}

#[test]
fn mixed_nested_blocks_preserve_prefix_child_suffix_order() {
    let source = region(
        0,
        vec![
            constant(0),
            Op::Block(region(1, vec![constant(1)], Exit::Fallthrough)),
            constant(2),
        ],
        Exit::Return(ValueId(7)),
    );
    let mut split = Splitter::default();
    let normal = split.terminal(End::Trap).unwrap();
    let first = split.region(&source, normal, None, None).unwrap();
    assert_eq!(split.states.len(), 5);
    assert_constant_chain(&split, first, &[0, 1, 2]);
}

#[test]
fn deeply_nested_blocks_with_prefix_effects_do_not_use_native_recursion() {
    // Unlike the transparent-block test, every parent contains a live atomic
    // prefix, so the old splitter consumed a native frame for each level.
    const DEPTH: usize = 2048;
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut source = region(DEPTH, vec![], Exit::Return(ValueId(7)));
            for level in 0..DEPTH {
                source = region(
                    level,
                    vec![constant(level), Op::Block(source)],
                    Exit::Fallthrough,
                );
            }
            let mut split = Splitter::default();
            let normal = split.terminal(End::Trap).unwrap();
            let first = split.region(&source, normal, None, None).unwrap();
            assert_eq!(split.states.len(), DEPTH + 2);
            let expected = (0..DEPTH).rev().collect::<Vec<_>>();
            assert_constant_chain(&split, first, &expected);
        })
        .expect("small-stack region splitting thread")
        .join()
        .expect("iterative mixed block splitting finished");
}

#[test]
fn too_many_synthetic_block_states_fail_with_the_existing_diagnostic() {
    // The stack fix does not expand the fixed async continuation-state budget.
    const DEPTH: usize = MAX_STATES;
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut source = region(DEPTH, vec![], Exit::Return(ValueId(7)));
            for level in 0..DEPTH {
                source = region(
                    level,
                    vec![constant(level), Op::Block(source)],
                    Exit::Fallthrough,
                );
            }
            let mut split = Splitter::default();
            let normal = split.terminal(End::Trap).unwrap();
            let error = split
                .region(&source, normal, None, None)
                .expect_err("synthetic source must exceed the unchanged state budget");
            assert!(error.contains("4096 states"), "{error}");
        })
        .expect("bounded-state region splitting thread")
        .join()
        .expect("state-budget failure reported without stack overflow");
}
