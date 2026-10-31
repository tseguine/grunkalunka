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

fn nested_verifier_module(depth: usize) -> Module {
    let span = Span::new(0, 0);
    let mut body = Region {
        id: RegionId(depth),
        span,
        operations: vec![Operation {
            span,
            kind: marker(0),
        }],
        exit: Exit::Return(ValueId(0)),
    };
    for level in (0..depth).rev() {
        body = Region {
            id: RegionId(level),
            span,
            operations: vec![Operation {
                span,
                kind: Op::Block(body),
            }],
            exit: Exit::Fallthrough,
        };
    }
    Module {
        closure_signatures: ClosureSignatures::default(),
        functions: vec![Function {
            return_type: Type::Number,
            parameter_count: 2,
            slots: (0..2)
                .map(|_| Slot {
                    ty: Type::Number,
                    owner: RegionId(0),
                    mutable: true,
                    parameter: true,
                    span,
                })
                .collect(),
            values: vec![Type::Number],
            body,
        }],
    }
}

fn discard_nested_module(mut module: Module, depth: usize) {
    // Deep synthetic IR destruction is deliberately not the subject of this
    // verifier test. Unwrap one owned child per iteration rather than dropping
    // a linked chain through Rust's derived Drop recursion.
    let mut current = module.functions.pop().expect("entry").body;
    for _ in 0..depth {
        let operation = current.operations.pop().expect("nested block");
        let Op::Block(child) = operation.kind else {
            panic!("expected nested block");
        };
        current = child;
    }
    assert!(matches!(
        current.operations.pop().expect("leaf").kind,
        Op::Constant { .. }
    ));
}

#[test]
fn deep_synthetic_verification_is_stack_independent_and_rejects_duplicate_regions() {
    // The parser/lowerer retain their own smaller admission budgets. This
    // deliberately non-source-originating region chain tests only the IR
    // verifier's iterative continuation algorithm under a small native stack.
    const DEPTH: usize = 1024;
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let module = nested_verifier_module(DEPTH);
            verify_module(&module).expect("deep, structurally valid numeric IR");
            discard_nested_module(module, DEPTH);

            let mut malformed = nested_verifier_module(DEPTH);
            let Op::Block(first) = &mut malformed.functions[0].body.operations[0].kind else {
                panic!("expected outer block");
            };
            first.id = RegionId(0);
            let error = verify_module(&malformed).expect_err("repeated region ID");
            assert!(error.contains("region ID is not unique"), "{error}");
            discard_nested_module(malformed, DEPTH);
        })
        .expect("small-stack verifier thread")
        .join()
        .expect("small-stack verifier completed");
}

#[test]
fn verifier_scope_paths_preserve_ancestor_access_and_reject_siblings() {
    let span = Span::new(0, 0);
    for (read_slot, accepted) in [(SlotId(0), true), (SlotId(2), false)] {
        let body = Region {
            id: RegionId(0),
            span,
            operations: vec![
                Operation {
                    span,
                    kind: Op::Load {
                        result: ValueId(0),
                        slot: SlotId(0),
                    },
                },
                Operation {
                    span,
                    kind: Op::Block(Region {
                        id: RegionId(1),
                        span,
                        operations: vec![Operation {
                            span,
                            kind: Op::Initialize {
                                slot: SlotId(2),
                                value: ValueId(0),
                            },
                        }],
                        exit: Exit::Fallthrough,
                    }),
                },
                Operation {
                    span,
                    kind: Op::Block(Region {
                        id: RegionId(2),
                        span,
                        operations: vec![Operation {
                            span,
                            kind: Op::Load {
                                result: ValueId(1),
                                slot: read_slot,
                            },
                        }],
                        exit: Exit::Fallthrough,
                    }),
                },
            ],
            exit: Exit::Return(ValueId(0)),
        };
        let mut slots = (0..2)
            .map(|_| Slot {
                ty: Type::Number,
                owner: RegionId(0),
                mutable: true,
                parameter: true,
                span,
            })
            .collect::<Vec<_>>();
        slots.push(Slot {
            ty: Type::Number,
            owner: RegionId(1),
            mutable: false,
            parameter: false,
            span,
        });
        let module = Module {
            closure_signatures: ClosureSignatures::default(),
            functions: vec![Function {
                return_type: Type::Number,
                parameter_count: 2,
                slots,
                values: vec![Type::Number, Type::Number],
                body,
            }],
        };
        let result = verify_module(&module);
        if accepted {
            result.expect("root-owned parameter remains visible in sibling region");
        } else {
            assert!(
                result.unwrap_err().contains("slot owner is outside the lexical region"),
                "sibling-owned cell must be rejected"
            );
        }
    }
}
