//! Ordinary cleanup preserves typed results and existing completion ownership.
use super::*;

struct Fixture {
    module: Module,
    input: ValueId,
    returned: ValueId,
}

fn region(id: usize, span: Span, operations: Vec<Op>, exit: Exit) -> Region {
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

fn fixture(entry_type: Type) -> Fixture {
    let mut module = crate::lower_source_with_entry_type(
        "export function run(a,b){return a;}",
        false,
        entry_type,
    )
    .unwrap();
    let function = &mut module.functions[0];
    let Exit::Return(input) = function.body.exit else {
        panic!("fixture return");
    };
    let span = function.body.span;
    let returned = ValueId(function.values.len());
    function.values.push(entry_type);
    let reason = ValueId(function.values.len());
    function.values.push(Type::General);
    function.body.operations.push(Operation {
        span,
        kind: Op::TryFinally {
            body: region(1, span, vec![], Exit::Return(input)),
            finalizer: region(2, span, vec![], Exit::Fallthrough),
            returned,
            reason,
            continuation: Box::new(region(3, span, vec![], Exit::Return(returned))),
        },
    });
    function.body.exit = Exit::Fallthrough;
    Fixture {
        module,
        input,
        returned,
    }
}

fn run_numeric(module: &Module, expected: f64) {
    let bytes = wat::parse_str(crate::emit::emit(module).unwrap()).unwrap();
    let script = format!(
        "const assert=require('node:assert/strict');const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from({bytes:?}))).exports.run;assert.equal(run(2,7),{expected});"
    );
    let output = std::process::Command::new("node")
        .arg("-e")
        .arg(script)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn ordinary_finally_keeps_numeric_and_primitive_result_lanes() {
    for entry_type in [Type::Number, Type::General] {
        let fixture = fixture(entry_type);
        verify_module_with_entry_type(&fixture.module, entry_type).unwrap();
        wat::parse_str(crate::emit::emit_with_entry_type(&fixture.module, entry_type).unwrap())
            .unwrap();
    }
}

#[test]
fn ordinary_finally_captures_stay_private_and_typed() {
    for fault in 0..10 {
        let mut fixture = fixture(Type::Number);
        let function = &mut fixture.module.functions[0];
        let Op::TryFinally {
            body,
            finalizer,
            returned,
            reason,
            continuation,
        } = &mut function.body.operations.last_mut().unwrap().kind
        else {
            panic!("fixture finally");
        };
        match fault {
            0 => function.values[returned.0] = Type::General,
            1 => function.values[reason.0] = Type::Number,
            2 => body.exit = Exit::Return(*returned),
            3 => finalizer.exit = Exit::Return(*returned),
            4 => continuation.exit = Exit::Return(*reason),
            5 => continuation.exit = Exit::Fallthrough,
            6 => finalizer.id = body.id,
            7 => finalizer.exit = Exit::Continue,
            8 => *reason = *returned,
            _ => *returned = ValueId(usize::MAX),
        }
        assert!(verify_module(&fixture.module).is_err(), "fault {fault}");
    }
}

#[test]
fn ordinary_finally_does_not_authorize_numeric_throws() {
    for cleanup_throws in [false, true] {
        let mut fixture = fixture(Type::Number);
        let function = &mut fixture.module.functions[0];
        let span = function.body.span;
        let thrown = ValueId(function.values.len());
        function.values.push(Type::General);
        let index = function.body.operations.len() - 1;
        function.body.operations.insert(
            index,
            Operation {
                span,
                kind: Op::Pack {
                    result: thrown,
                    input: fixture.input,
                },
            },
        );
        let Op::TryFinally {
            body, finalizer, ..
        } = &mut function.body.operations.last_mut().unwrap().kind
        else {
            panic!("fixture finally");
        };
        if cleanup_throws {
            finalizer.exit = Exit::Throw(thrown);
        } else {
            body.exit = Exit::Throw(thrown);
            // Even an overriding return does not establish a catch contract.
            finalizer.exit = Exit::Return(fixture.input);
        }
        assert!(
            verify_module(&fixture.module)
                .unwrap_err()
                .contains("invalid region exit contract")
        );

        let function = &mut fixture.module.functions[0];
        let protected = function.body.operations.pop().unwrap().kind;
        let caught = ValueId(function.values.len());
        function.values.push(Type::General);
        function.body.operations.push(Operation {
            span,
            kind: Op::TryCatch {
                body: region(4, span, vec![protected], Exit::Fallthrough),
                reason: caught,
                handler: region(5, span, vec![], Exit::Return(fixture.input)),
            },
        });
        run_numeric(&fixture.module, 2.0);
    }
}

#[test]
fn both_return_paths_run_the_finally_continuation() {
    for cleanup_returns in [false, true] {
        let mut fixture = fixture(Type::Number);
        let function = &mut fixture.module.functions[0];
        let span = function.body.span;
        let base = function.values.len();
        function.values.extend([Type::Number; 3]);
        let [second, ten, sum] = std::array::from_fn(|i| ValueId(base + i));
        function.body.operations.insert(
            0,
            Operation {
                span,
                kind: Op::Load {
                    result: second,
                    slot: SlotId(1),
                },
            },
        );
        let Op::TryFinally {
            finalizer,
            continuation,
            ..
        } = &mut function.body.operations.last_mut().unwrap().kind
        else {
            panic!("fixture finally");
        };
        if cleanup_returns {
            finalizer.exit = Exit::Return(second);
        }
        continuation.operations.extend([
            Operation {
                span,
                kind: Op::Constant {
                    result: ten,
                    value: 10.0,
                },
            },
            Operation {
                span,
                kind: Op::Binary {
                    result: sum,
                    operator: Binary::Add,
                    left: fixture.returned,
                    right: ten,
                },
            },
        ]);
        continuation.exit = Exit::Return(sum);
        run_numeric(&fixture.module, if cleanup_returns { 17.0 } else { 12.0 });
    }
}

#[test]
fn a_typed_async_value_finally_does_not_inherit_a_callers_raw_contract() {
    let mut module = crate::lower_source_with_entry_type(
        "export function run(a,b){return a;}",
        false,
        Type::General,
    )
    .unwrap();
    let function = &mut module.functions[0];
    let Exit::Return(input) = function.body.exit else {
        panic!("primitive fixture return");
    };
    let span = function.body.span;
    let base = function.values.len();
    function.values.extend([
        Type::AsyncValue,
        Type::AsyncValue,
        Type::General,
        Type::AsyncValue,
        Type::AsyncValue,
        Type::General,
        Type::General,
    ]);
    let [
        raw,
        inner_returned,
        inner_reason,
        result,
        returned,
        reason,
        primitive,
    ] = std::array::from_fn(|i| ValueId(base + i));
    function.body.operations.push(Operation {
        span,
        kind: Op::ToAsyncValue { result: raw, input },
    });
    let inner = Op::TryFinally {
        body: region(1, span, vec![], Exit::Return(raw)),
        finalizer: region(2, span, vec![], Exit::Fallthrough),
        returned: inner_returned,
        reason: inner_reason,
        continuation: Box::new(region(3, span, vec![], Exit::Return(inner_returned))),
    };
    function.body.operations.push(Operation {
        span,
        kind: Op::TryFinally {
            body: region(
                5,
                span,
                vec![Op::InlineCall {
                    result,
                    body: region(4, span, vec![inner], Exit::Fallthrough),
                }],
                Exit::Return(result),
            ),
            finalizer: region(6, span, vec![], Exit::Fallthrough),
            returned,
            reason,
            continuation: Box::new(region(
                7,
                span,
                vec![Op::AsyncValuePrimitive {
                    result: primitive,
                    input: returned,
                }],
                Exit::Return(primitive),
            )),
        },
    });
    function.body.exit = Exit::Fallthrough;
    verify_module_with_entry_type(&module, Type::General).unwrap();
    wat::parse_str(crate::emit::emit_with_entry_type(&module, Type::General).unwrap()).unwrap();

    let Op::TryFinally { body, .. } =
        &mut module.functions[0].body.operations.last_mut().unwrap().kind
    else {
        panic!("outer finally");
    };
    let Op::InlineCall { body, .. } = &mut body.operations[0].kind else {
        panic!("inline helper");
    };
    let Op::TryFinally { body, .. } = &mut body.operations[0].kind else {
        panic!("typed helper finally");
    };
    body.exit = Exit::Throw(input);
    assert!(
        verify_module_with_entry_type(&module, Type::General)
            .unwrap_err()
            .contains("invalid region exit contract")
    );
}

#[test]
fn atomic_settlement_finally_has_scratch_in_both_emitters() {
    let mut fixture = fixture(Type::General);
    let function = &mut fixture.module.functions[0];
    let span = function.body.span;
    let mut body = std::mem::replace(
        &mut function.body,
        region(0, span, vec![], Exit::Fallthrough),
    );
    body.id = RegionId(100);
    let settled = ValueId(function.values.len());
    function.values.push(Type::SettledPromise);
    let result = ValueId(function.values.len());
    function.values.push(Type::General);
    function.body.operations.extend([
        Operation {
            span,
            kind: Op::SettlePromise {
                result: settled,
                body,
            },
        },
        Operation {
            span,
            kind: Op::SettledValue {
                result,
                input: settled,
            },
        },
    ]);
    function.body.exit = Exit::Return(result);
    verify_module_with_entry_type(&fixture.module, Type::General).unwrap();
    wat::parse_str(crate::emit::emit_with_entry_type(&fixture.module, Type::General).unwrap())
        .unwrap();
    wat::parse_str(crate::emit::emit_async(&fixture.module).unwrap()).unwrap();
}
