//! Closed lexical exceptions do not change an ordinary function's result lane.
use super::*;

struct Fixture {
    module: Module,
    input: ValueId,
    thrown: ValueId,
    reason: ValueId,
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

fn numeric_catch() -> Fixture {
    let mut module = crate::lower_source("export function run(a,b){return a;}", false).unwrap();
    let function = &mut module.functions[0];
    let Exit::Return(input) = function.body.exit else {
        panic!("numeric fixture return");
    };
    let span = function.body.span;
    let thrown = ValueId(function.values.len());
    function.values.push(Type::General);
    let reason = ValueId(function.values.len());
    function.values.push(Type::General);
    let returned = ValueId(function.values.len());
    function.values.push(Type::Number);
    function.body.operations.push(Operation {
        span,
        kind: Op::TryCatch {
            body: region(
                1,
                span,
                vec![Op::Pack {
                    result: thrown,
                    input,
                }],
                Exit::Throw(thrown),
            ),
            reason,
            handler: region(
                2,
                span,
                vec![Op::NumberFromGeneral {
                    result: returned,
                    input: reason,
                }],
                Exit::Return(returned),
            ),
        },
    });
    function.body.exit = Exit::Fallthrough;
    Fixture {
        module,
        input,
        thrown,
        reason,
    }
}

#[test]
fn numeric_result_functions_can_close_primitive_throws_locally() {
    let fixture = numeric_catch();
    verify_module(&fixture.module).unwrap();
    let output = crate::emit::emit(&fixture.module).unwrap();
    wat::parse_str(output).expect("closed throw uses the ordinary Wasm emitter");
}

#[test]
fn escaping_general_throws_still_require_the_async_emitter() {
    let mut module = crate::lower_source_with_entry_type(
        "export function run(a,b){return a;}",
        false,
        Type::General,
    )
    .unwrap();
    let function = &mut module.functions[0];
    let Exit::Return(reason) = function.body.exit else {
        panic!("primitive fixture return");
    };
    assert_eq!(function.values[reason.0], Type::General);
    function.body.exit = Exit::Throw(reason);
    // General exits retain their existing async-compatible verifier contract;
    // the ordinary emitter separately requires every throw to be closed.
    verify_module_with_entry_type(&module, Type::General).unwrap();
    assert_eq!(
        crate::emit::emit_with_entry_type(&module, Type::General).unwrap_err(),
        "await IR requires the async emitter"
    );
}

#[test]
fn local_catches_keep_reason_type_and_visibility_guards() {
    for fault in 0..5 {
        let mut fixture = numeric_catch();
        verify_module(&fixture.module).unwrap();
        let function = &mut fixture.module.functions[0];
        let Op::TryCatch {
            body,
            reason,
            handler,
        } = &mut function.body.operations.last_mut().unwrap().kind
        else {
            panic!("fixture catch");
        };
        match fault {
            0 => function.values[reason.0] = Type::Number,
            1 => body.exit = Exit::Throw(fixture.input),
            2 => body.exit = Exit::Throw(*reason),
            3 => {
                let Op::NumberFromGeneral { input, .. } = &mut handler.operations[0].kind else {
                    panic!("fixture reason consumer");
                };
                *input = fixture.thrown;
            }
            _ => *reason = ValueId(usize::MAX),
        }
        assert!(verify_module(&fixture.module).is_err(), "fault {fault}");
    }
}

#[test]
fn a_handler_rethrow_needs_an_outer_catch() {
    let mut fixture = numeric_catch();
    let function = &mut fixture.module.functions[0];
    let Op::TryCatch { handler, .. } = &mut function.body.operations.last_mut().unwrap().kind
    else {
        panic!("fixture catch");
    };
    handler.exit = Exit::Throw(fixture.reason);
    assert!(verify_module(&fixture.module).is_err());

    let function = &mut fixture.module.functions[0];
    let inner = function.body.operations.pop().unwrap().kind;
    let span = function.body.span;
    let reason = ValueId(function.values.len());
    function.values.push(Type::General);
    let returned = ValueId(function.values.len());
    function.values.push(Type::Number);
    function.body.operations.push(Operation {
        span,
        kind: Op::TryCatch {
            body: region(3, span, vec![inner], Exit::Fallthrough),
            reason,
            handler: region(
                4,
                span,
                vec![Op::NumberFromGeneral {
                    result: returned,
                    input: reason,
                }],
                Exit::Return(returned),
            ),
        },
    });
    verify_module(&fixture.module).unwrap();
    wat::parse_str(crate::emit::emit(&fixture.module).unwrap()).unwrap();
}

#[test]
fn a_caller_catch_does_not_change_an_inline_function_throw_contract() {
    let mut fixture = numeric_catch();
    let function = &mut fixture.module.functions[0];
    let result = ValueId(function.values.len());
    function.values.push(Type::Number);
    let Op::TryCatch { body, .. } = &mut function.body.operations.last_mut().unwrap().kind else {
        panic!("fixture catch");
    };
    let span = body.span;
    let inline = std::mem::replace(body, region(3, span, Vec::new(), Exit::Return(result)));
    body.operations.push(Operation {
        span,
        kind: Op::InlineCall {
            result,
            body: inline,
        },
    });
    assert!(
        verify_module(&fixture.module)
            .unwrap_err()
            .contains("invalid region exit contract")
    );
}
