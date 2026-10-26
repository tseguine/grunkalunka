use super::*;

fn module() -> Module {
    crate::lower_source(
        "export function run(a,b){const f=a?x=>x+a:x=>x-b;return f(b);}",
        false,
    )
    .unwrap()
}

fn constructor(module: &mut Module) -> &mut Op {
    let Op::If { consequent, .. } = &mut module.functions[0]
        .body
        .operations
        .iter_mut()
        .find(|op| matches!(op.kind, Op::If { .. }))
        .unwrap()
        .kind
    else {
        unreachable!()
    };
    &mut consequent
        .operations
        .iter_mut()
        .find(|op| matches!(op.kind, Op::MakeClosure { .. }))
        .unwrap()
        .kind
}

#[test]
fn environments_require_typed_initialized_captures_and_the_private_entry_abi() {
    for change in 0..12 {
        let mut module = module();
        verify_module(&module).unwrap();
        match change {
            0..=3 => {
                let Op::MakeClosure {
                    function, captures, ..
                } = constructor(&mut module)
                else {
                    unreachable!()
                };
                match change {
                    0 => *function = FunctionId(usize::MAX),
                    1 => *function = FunctionId(0),
                    2 => captures.clear(),
                    _ => captures[0] = SlotId(usize::MAX),
                }
            }
            4 => {
                module.functions[1].slots[1].mutable = false;
            }
            5 => {
                let Op::If {
                    result: Some(result),
                    ..
                } = &module.functions[0]
                    .body
                    .operations
                    .iter()
                    .find(|op| matches!(op.kind, Op::If { .. }))
                    .unwrap()
                    .kind
                else {
                    unreachable!()
                };
                let result = *result;
                module.functions[0].values[result.0] = Type::General;
            }
            6 => {
                let Op::ClosureEntry { captures } =
                    &mut module.functions[1].body.operations[0].kind
                else {
                    unreachable!()
                };
                captures[0] = SlotId(0);
            }
            7 => module.functions[1].body.operations.swap(0, 1),
            8 => module.functions[0].body.operations.insert(
                0,
                Operation {
                    span: Span::new(0, 1),
                    kind: Op::ClosureEntry {
                        captures: Vec::new(),
                    },
                },
            ),
            9 => {
                let operation = module.functions[0]
                    .body
                    .operations
                    .iter_mut()
                    .find(|op| matches!(op.kind, Op::ClosureCall { .. }))
                    .unwrap();
                let Op::ClosureCall {
                    result, arguments, ..
                } = &operation.kind
                else {
                    unreachable!()
                };
                operation.kind = Op::Call {
                    result: *result,
                    function: FunctionId(1),
                    arguments: arguments.clone(),
                };
            }
            10 => {
                let caller = &mut module.functions[0];
                let slot = SlotId(caller.slots.len());
                caller.slots.push(Slot {
                    ty: Type::Number,
                    owner: caller.body.id,
                    mutable: true,
                    parameter: false,
                    span: Span::new(0, 1),
                });
                let Op::MakeClosure { captures, .. } = constructor(&mut module) else {
                    unreachable!()
                };
                captures[0] = slot;
            }
            _ => module.functions[1].slots[1].owner = RegionId(usize::MAX),
        }
        assert!(verify_module(&module).is_err(), "mutation {change}");
    }
}

#[test]
fn function_values_cannot_be_called_with_wrong_types_or_packed_as_primitives() {
    for change in 0..5 {
        let mut module = module();
        verify_module(&module).unwrap();
        let function = &mut module.functions[0];
        let operation = function
            .body
            .operations
            .iter_mut()
            .find(|op| matches!(op.kind, Op::ClosureCall { .. }))
            .unwrap();
        let Op::ClosureCall {
            callee,
            arguments,
            result,
        } = &mut operation.kind
        else {
            unreachable!()
        };
        match change {
            0 => *callee = *result,
            1 => arguments.clear(),
            2 => arguments[0] = *callee,
            _ => {
                let input = *callee;
                let result = ValueId(function.values.len());
                function.values.push(if change == 3 {
                    Type::General
                } else {
                    Type::Closure(1)
                });
                function.body.operations.push(Operation {
                    span: Span::new(0, 1),
                    kind: if change == 3 {
                        Op::Pack { result, input }
                    } else {
                        Op::Await {
                            result,
                            input,
                            rejected: false,
                        }
                    },
                });
            }
        }
        assert!(verify_module(&module).is_err(), "mutation {change}");
    }
}
