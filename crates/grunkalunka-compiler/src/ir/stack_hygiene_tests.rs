//! Characterize stack-independent IR operation inspection. Synthetic deep
//! regions are NOT admitted source programs or an IR verification allowance.
use super::*;

fn region(operations: Vec<Op>) -> Region {
    let span = Span::new(0, 0);
    Region {
        id: RegionId(0),
        span,
        operations: operations
            .into_iter()
            .map(|kind| Operation { span, kind })
            .collect(),
        exit: Exit::Fallthrough,
    }
}

fn marker(value: usize) -> Op {
    Op::Constant {
        result: ValueId(value),
        value: value as f64,
    }
}

#[test]
fn operation_walk_keeps_preorder_for_every_nested_control_region() {
    let root = region(vec![
        marker(0),
        Op::If {
            result: None,
            condition: ValueId(0),
            consequent: region(vec![
                marker(1),
                Op::InlineCall {
                    result: ValueId(0),
                    body: region(vec![marker(2)]),
                },
            ]),
            alternate: region(vec![marker(3)]),
        },
        Op::While {
            condition: region(vec![marker(4)]),
            body: region(vec![marker(5)]),
            step: Some(region(vec![marker(6)])),
        },
        Op::TryCatch {
            body: region(vec![marker(7)]),
            reason: ValueId(0),
            handler: region(vec![marker(8)]),
        },
        Op::TryFinally {
            body: region(vec![marker(9)]),
            finalizer: region(vec![marker(10)]),
            returned: ValueId(0),
            reason: ValueId(0),
            continuation: Box::new(region(vec![marker(11)])),
        },
        Op::DoWhile {
            body: region(vec![marker(12)]),
            condition: region(vec![marker(13)]),
        },
        Op::AwaitCall {
            result: ValueId(0),
            body: region(vec![marker(14)]),
        },
        Op::SettlePromise {
            result: ValueId(0),
            body: region(vec![marker(15)]),
        },
        Op::Block(region(vec![marker(16)])),
        marker(17),
    ]);
    let mut observed = Vec::new();
    visit_operations(&root, &mut |op| {
        if let Op::Constant { result, .. } = op {
            observed.push(result.0);
        }
    });
    assert_eq!(observed, (0..18).collect::<Vec<_>>());
}

#[test]
fn deep_operation_walk_stays_on_a_small_native_stack() {
    // No parser, verifier or emitter admission is inferred from this test.
    // Exceed the source-depth budget intentionally to prove the inspector
    // itself does not consume one native frame per synthetic IR block.
    const DEPTH: usize = 4096;
    let mut root = region(vec![marker(7)]);
    for _ in 0..DEPTH {
        root = region(vec![Op::Block(root)]);
    }
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let mut blocks = 0;
            let mut values = 0;
            visit_operations(&root, &mut |op| match op {
                Op::Block(_) => blocks += 1,
                Op::Constant {
                    result: ValueId(7), ..
                } => values += 1,
                _ => panic!("unexpected synthetic operation"),
            });
            assert_eq!((blocks, values), (DEPTH, 1));

            // Deliberately unwind this artificial chain iteratively as well.
            // Recursive *destruction* is a different native-stack seam and
            // must not accidentally become the subject of this walker test.
            let mut current = root;
            for _ in 0..DEPTH {
                let op = current.operations.pop().expect("nested block");
                let Op::Block(child) = op.kind else {
                    panic!("expected nested block");
                };
                current = child;
            }
            let op = current.operations.pop().expect("leaf operation");
            assert!(matches!(op.kind, Op::Constant { .. }));
            assert!(current.operations.is_empty());
        })
        .expect("stack probe thread")
        .join()
        .expect("stack probe completed");
}
