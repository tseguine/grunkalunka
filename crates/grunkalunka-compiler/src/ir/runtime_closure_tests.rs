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

#[test]
fn higher_order_signatures_and_callback_arguments_are_verified_logically() {
    let source =
        "export function run(a,b){const h=a?(fn,x)=>fn(x):(fn,x)=>fn(fn(x));return h(y=>y+b,a);}";
    for change in 0..10 {
        let mut module = crate::lower_source(source, false).unwrap();
        verify_module(&module).unwrap();
        match change {
            0 => module.closure_signatures.higher.clear(),
            1 => module.closure_signatures.higher[0].parameters[0] = Type::Number,
            2 => {
                let original = &module.closure_signatures.higher[0];
                let duplicate = ClosureSignature {
                    parameters: original.parameters.clone(),
                    result: original.result,
                };
                module.closure_signatures.higher.push(duplicate);
            }
            3 => module.closure_signatures.higher[0].parameters[0] = Type::Closure(1001),
            4 => module.closure_signatures.higher[0]
                .parameters
                .resize(1001, Type::Number),
            5 => module.functions[1].slots[0].ty = Type::Number,
            6 => module.functions[1].slots[0].ty = Type::Closure(2),
            _ => {
                let entry = &mut module.functions[0];
                let operation = entry
                    .body
                    .operations
                    .iter_mut()
                    .find(|op| matches!(op.kind, Op::ClosureCall { .. }))
                    .unwrap();
                let Op::ClosureCall {
                    callee, arguments, ..
                } = &mut operation.kind
                else {
                    unreachable!()
                };
                match change {
                    7 => arguments[0] = arguments[1],
                    8 => *callee = arguments[0],
                    _ => arguments.swap(0, 1),
                }
            }
        }
        assert!(
            verify_module(&module).is_err(),
            "signature/argument mutation {change}"
        );
    }
}

#[test]
fn runtime_factory_result_metadata_and_pair_calls_are_verified() {
    let source = "export function run(a,b){let make=x=>()=>x;return make(a)();}";
    for change in 0..9 {
        let mut module = crate::lower_source(source, false).unwrap();
        verify_module(&module).unwrap();
        let factory = module
            .functions
            .iter()
            .position(|f| matches!(f.return_type, Type::Closure(_)))
            .unwrap();
        match change {
            0 => module.functions[factory].return_type = Type::Number,
            1 => module.functions[factory].return_type = Type::Closure(1001),
            2 => module.closure_signatures.higher[0].result = Type::Number,
            3 => module.closure_signatures.higher[0].result = Type::Closure(1),
            4 => module.closure_signatures.higher[0].result = Type::Closure(1001),
            5 => module.functions[0].return_type = Type::Closure(0),
            6 => {
                let entry = &mut module.functions[0];
                let result = entry
                    .body
                    .operations
                    .iter()
                    .find_map(|op| {
                        if let Op::ClosureCall { result, .. } = op.kind {
                            Some(result)
                        } else {
                            None
                        }
                    })
                    .unwrap();
                entry.values[result.0] = Type::Number;
            }
            7 => {
                let f = &mut module.functions[factory];
                let value = f
                    .body
                    .operations
                    .iter()
                    .find_map(|op| {
                        if let Op::MakeClosure { result, .. } = op.kind {
                            Some(result)
                        } else {
                            None
                        }
                    })
                    .unwrap();
                f.values[value.0] = Type::Closure(1);
            }
            _ => {
                let leaf = module
                    .functions
                    .iter_mut()
                    .find(|f| closure_environment(f).is_some() && f.return_type == Type::Number)
                    .unwrap();
                leaf.return_type = Type::General;
            }
        }
        assert!(
            verify_module(&module).is_err(),
            "factory result mutation {change}"
        );
    }
}
