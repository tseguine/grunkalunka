//! Source-ordered lexical handler continuations with bounded native stack depth.
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
        _ => panic!("expected atomic operation and continuation"),
    }
}

#[test]
fn protected_throw_enters_its_handler_before_the_parent_suffix() {
    let source = region(
        0,
        vec![
            constant(0),
            Op::TryCatch {
                body: region(1, vec![constant(1)], Exit::Throw(ValueId(11))),
                reason: ValueId(4),
                handler: region(2, vec![constant(2)], Exit::Fallthrough),
            },
            constant(3),
        ],
        Exit::Return(ValueId(7)),
    );
    let mut split = Splitter::default();
    let normal = split.terminal(End::Trap).unwrap();
    let entry = split.region(&source, normal, None, None).unwrap();
    let body = constant_then(&split, constant_then(&split, entry, 0), 1);
    let handler = match &split.states[body].end {
        End::Copy {
            input,
            result,
            next,
        } => {
            assert_eq!((*input, *result), (ValueId(11), ValueId(4)));
            *next
        }
        _ => panic!("protected throw must copy the original reason"),
    };
    let suffix = constant_then(&split, constant_then(&split, handler, 2), 3);
    assert!(matches!(&split.states[suffix].end, End::Return(ValueId(7))));
    assert!(split.catch_to.is_none());
}

#[test]
fn deep_lexical_catches_are_not_native_recursive() {
    const DEPTH: usize = 2048;
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut source = region(DEPTH, vec![], Exit::Return(ValueId(7)));
            for level in 0..DEPTH {
                source = region(
                    level,
                    vec![Op::TryCatch {
                        body: source,
                        reason: ValueId(5),
                        handler: region(DEPTH + level + 1, vec![], Exit::Fallthrough),
                    }],
                    Exit::Fallthrough,
                );
            }
            let mut split = Splitter::default();
            let normal = split.terminal(End::Trap).unwrap();
            let entry = split.region(&source, normal, None, None).unwrap();
            assert_eq!(split.states.len(), 2);
            assert!(matches!(&split.states[entry].end, End::Return(ValueId(7))));
            assert!(split.catch_to.is_none());
        })
        .expect("small-stack nested-catch thread")
        .join()
        .expect("nested catch continuations completed without recursion");
}

#[test]
fn failed_protected_body_split_does_not_leak_a_handler_binding() {
    let source = region(
        0,
        vec![Op::TryCatch {
            body: region(1, vec![], Exit::Yield(ValueId(3))),
            reason: ValueId(4),
            handler: region(2, vec![], Exit::Fallthrough),
        }],
        Exit::Fallthrough,
    );
    let valid = region(3, vec![], Exit::Return(ValueId(7)));
    let mut split = Splitter::default();
    let normal = split.terminal(End::Trap).unwrap();
    let error = split.region(&source, normal, None, None).unwrap_err();
    assert!(error.contains("async yield lacks a target"), "{error}");
    assert!(split.catch_to.is_none());
    let entry = split.region(&valid, normal, None, None).unwrap();
    assert!(matches!(&split.states[entry].end, End::Return(ValueId(7))));
}
