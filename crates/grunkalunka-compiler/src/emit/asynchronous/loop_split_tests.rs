//! Synthetic loop continuations under the explicit async splitter work stack.
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
fn while_step_and_nearest_loop_break_continue_targets_are_preserved() {
    for (exit, continued) in [(Exit::Continue, true), (Exit::Break, false)] {
        let source = region(
            0,
            vec![Op::While {
                condition: region(1, vec![], Exit::Yield(ValueId(8))),
                body: region(2, vec![], exit),
                step: Some(region(
                    3,
                    vec![Op::Constant {
                        result: ValueId(9),
                        value: 42.0,
                    }],
                    Exit::Fallthrough,
                )),
            }],
            Exit::Return(ValueId(7)),
        );
        let mut split = Splitter::default();
        let normal = split.terminal(End::Trap).unwrap();
        let first = split.region(&source, normal, None, None).unwrap();
        let condition = match &split.states[first].end {
            End::Jump(target) => *target,
            _ => panic!("while test must jump to its condition"),
        };
        let (body, done) = match &split.states[condition].end {
            End::Test { condition, yes, no } => {
                assert_eq!(*condition, ValueId(8));
                (*yes, *no)
            }
            _ => panic!("condition should branch between body and loop exit"),
        };
        assert!(matches!(&split.states[done].end, End::Return(ValueId(7))));
        let step = split
            .states
            .iter()
            .position(|state| {
                state.operations.len() == 1
                    && matches!(state.operations[0].kind, Op::Constant { .. })
            })
            .expect("step atomic state");
        assert_eq!(body, if continued { step } else { done });
        assert!(matches!(&split.states[step].end, End::Jump(target) if *target == first));
    }
}

#[test]
fn do_while_continue_and_break_preserve_first_iteration_and_test_targets() {
    for (exit, continued) in [(Exit::Continue, true), (Exit::Break, false)] {
        let source = region(
            0,
            vec![Op::DoWhile {
                body: region(1, vec![], exit),
                condition: region(2, vec![], Exit::Yield(ValueId(8))),
            }],
            Exit::Return(ValueId(7)),
        );
        let mut split = Splitter::default();
        let normal = split.terminal(End::Trap).unwrap();
        let first = split.region(&source, normal, None, None).unwrap();
        let target = match &split.states[first].end {
            End::Jump(target) => *target,
            _ => panic!("do loop must enter the body before testing"),
        };
        if continued {
            let End::Test { condition, yes, no } = &split.states[target].end else {
                panic!("continue should run the do-while condition");
            };
            assert_eq!(*condition, ValueId(8));
            assert_eq!(*yes, first);
            assert!(matches!(&split.states[*no].end, End::Return(ValueId(7))));
        } else {
            assert!(matches!(&split.states[target].end, End::Return(ValueId(7))));
        }
    }
}

#[test]
fn deeply_nested_while_bodies_split_without_native_recursion() {
    const DEPTH: usize = 1024;
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut source = region(DEPTH, vec![], Exit::Return(ValueId(7)));
            for level in 0..DEPTH {
                source = region(
                    level,
                    vec![Op::While {
                        condition: region(
                            DEPTH + level + 1,
                            vec![],
                            Exit::Yield(ValueId(1)),
                        ),
                        body: source,
                        step: None,
                    }],
                    Exit::Fallthrough,
                );
            }
            let mut split = Splitter::default();
            let normal = split.terminal(End::Trap).unwrap();
            let mut entry = split.region(&source, normal, None, None).unwrap();
            assert_eq!(split.states.len(), DEPTH * 2 + 2);
            for _ in 0..DEPTH {
                let End::Jump(test) = &split.states[entry].end else {
                    panic!("while entry must branch through its loop condition");
                };
                let End::Test { condition, yes, no } = &split.states[*test].end else {
                    panic!("while condition must select the next nested body");
                };
                assert_eq!(*condition, ValueId(1));
                assert_eq!(*no, normal);
                entry = *yes;
            }
            assert!(matches!(&split.states[entry].end, End::Return(ValueId(7))));
        })
        .expect("small-stack nested loops thread")
        .join()
        .expect("iterative async loop splitting completed");
}
