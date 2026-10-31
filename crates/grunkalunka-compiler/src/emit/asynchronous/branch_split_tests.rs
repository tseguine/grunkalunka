//! Synthetic If control-flow checks for the iterative async continuation splitter.
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

fn constant_then(split: &Splitter<'_>, state: usize, value: usize) -> usize {
    let state = &split.states[state];
    assert_eq!(state.operations.len(), 1);
    assert!(matches!(
        &state.operations[0].kind,
        Op::Constant { result, .. } if *result == ValueId(value)
    ));
    match &state.end {
        End::Jump(next) => *next,
        _ => panic!("constant state must jump to its successor"),
    }
}

#[test]
fn if_branches_preserve_snapshot_order_and_yield_continuations() {
    let source = region(
        0,
        vec![
            constant(0),
            Op::If {
                result: Some(ValueId(4)),
                condition: ValueId(3),
                consequent: region(1, vec![constant(1)], Exit::Yield(ValueId(7))),
                alternate: region(2, vec![constant(2)], Exit::Yield(ValueId(8))),
            },
            constant(9),
        ],
        Exit::Return(ValueId(4)),
    );
    let mut split = Splitter::default();
    let unreachable = split.terminal(End::Trap).unwrap();
    let first = split.region(&source, unreachable, None, None).unwrap();
    let selected = constant_then(&split, first, 0);
    let (yes, no) = match &split.states[selected].end {
        End::Test { condition, yes, no } => {
            assert_eq!(*condition, ValueId(3));
            (*yes, *no)
        }
        _ => panic!("If must dispatch after its source prefix"),
    };
    let consequent_copy = constant_then(&split, yes, 1);
    let suffix_state = match &split.states[consequent_copy].end {
        End::Copy { input, result, next } => {
            assert_eq!((*input, *result), (ValueId(7), ValueId(4)));
            *next
        }
        _ => panic!("consequent must copy its yielded result"),
    };
    let returned = constant_then(&split, suffix_state, 9);
    assert!(matches!(&split.states[returned].end, End::Return(ValueId(4))));

    let alternate_copy = constant_then(&split, no, 2);
    match &split.states[alternate_copy].end {
        End::Copy { input, result, next } => {
            assert_eq!((*input, *result), (ValueId(8), ValueId(4)));
            assert_eq!(*next, suffix_state);
        }
        _ => panic!("alternate must copy its yielded result"),
    }
}

#[test]
fn nested_if_region_graphs_use_a_bounded_native_stack() {
    const DEPTH: usize = 2048;
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut source = region(DEPTH, vec![], Exit::Return(ValueId(7)));
            for level in 0..DEPTH {
                source = region(
                    level,
                    vec![Op::If {
                        result: None,
                        condition: ValueId(1),
                        consequent: source,
                        alternate: region(DEPTH + level + 1, vec![], Exit::Fallthrough),
                    }],
                    Exit::Fallthrough,
                );
            }
            let mut split = Splitter::default();
            let normal = split.terminal(End::Trap).unwrap();
            let mut entry = split.region(&source, normal, None, None).unwrap();
            assert_eq!(split.states.len(), DEPTH + 2);
            for _ in 0..DEPTH {
                let End::Test { condition, yes, no } = &split.states[entry].end else {
                    panic!("expected nested If dispatch");
                };
                assert_eq!(*condition, ValueId(1));
                assert_eq!(*no, normal);
                entry = *yes;
            }
            assert!(matches!(&split.states[entry].end, End::Return(ValueId(7))));
        })
        .expect("small-stack nested If thread")
        .join()
        .expect("iterative nested If splitting completed");
}
