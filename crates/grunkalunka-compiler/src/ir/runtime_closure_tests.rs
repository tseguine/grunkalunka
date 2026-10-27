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
        let factory_signature = module
            .closure_signatures
            .id(&[Type::Number], Type::Closure(0))
            .unwrap()
            - 1001;
        match change {
            0 => module.functions[factory].return_type = Type::Number,
            1 => module.functions[factory].return_type = Type::Closure(1001),
            2 => module.closure_signatures.higher[factory_signature].result = Type::Number,
            3 => module.closure_signatures.higher[factory_signature].result = Type::Closure(1),
            4 => module.closure_signatures.higher[factory_signature].result = Type::Closure(1001),
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

#[test]
fn primitive_factory_leaf_signatures_and_metadata_are_verified() {
    let source = "export function run(a,b){let make=x=>()=>x?null:false;return make(a)();}";
    for change in 0..7 {
        let mut module = crate::lower_source_with_entry_type(source, false, Type::General).unwrap();
        verify_module_with_entry_type(&module, Type::General).unwrap();
        let factory = module
            .functions
            .iter()
            .position(|f| matches!(f.return_type, Type::Closure(_)))
            .unwrap();
        let Type::Closure(signature) = module.functions[factory].return_type else {
            unreachable!()
        };
        match change {
            0 => module.closure_signatures.higher[signature - 1001].result = Type::Number,
            1 => module.closure_signatures.higher[signature - 1001].result = Type::Closure(0),
            2 => module.closure_signatures.higher[signature - 1001]
                .parameters
                .push(Type::Closure(0)),
            3 => {
                module.closure_signatures.higher[signature - 1001].parameters =
                    vec![Type::Number; 1001]
            }
            4 => {
                module
                    .functions
                    .iter_mut()
                    .find(|f| closure_environment(f).is_some() && f.return_type == Type::General)
                    .unwrap()
                    .return_type = Type::Number;
            }
            5 => module.functions[factory].return_type = Type::Closure(usize::MAX),
            _ => {
                let entry = &mut module.functions[0];
                let result = entry
                    .body
                    .operations
                    .iter()
                    .find_map(|op| match op.kind {
                        Op::ClosureCall { result, .. }
                            if entry.values[result.0] == Type::General =>
                        {
                            Some(result)
                        }
                        _ => None,
                    })
                    .unwrap();
                entry.values[result.0] = Type::Number;
            }
        }
        assert!(
            verify_module_with_entry_type(&module, Type::General).is_err(),
            "primitive factory mutation {change}"
        );
    }
}

#[test]
fn nested_factory_result_chains_reject_cycles_parameters_and_wrong_pairs() {
    let source = "export function run(a,b){let make=x=>y=>()=>x+y;return make(a)(b)();}";
    for change in 0..10 {
        let mut module = crate::lower_source(source, false).unwrap();
        verify_module(&module).unwrap();
        let factory=module.functions.iter().position(|f| matches!(f.return_type,Type::Closure(id) if matches!(module.closure_signatures.result(id),Some(Type::Closure(_))))).unwrap();
        let Type::Closure(signature) = module.functions[factory].return_type else {
            unreachable!()
        };
        match change {
            0 => {
                module.closure_signatures.higher[signature - 1001].result = Type::Closure(signature)
            }
            1 => {
                module.closure_signatures.higher[signature - 1001].result =
                    Type::Closure(signature + 1)
            }
            2 => {
                module.closure_signatures.higher[signature - 1001].result =
                    Type::Closure(usize::MAX)
            }
            3 => {
                module.closure_signatures.higher[signature - 1001].parameters =
                    vec![Type::Closure(0)]
            }
            4 => {
                module.closure_signatures.higher[signature - 1001].parameters = vec![Type::General]
            }
            5 => {
                module.closure_signatures.higher[signature - 1001].parameters =
                    vec![Type::Number; 1001]
            }
            6 => {
                module
                    .functions
                    .iter_mut()
                    .find(|f| f.return_type == Type::Closure(0))
                    .unwrap()
                    .return_type = Type::General;
            }
            7 => module.functions[factory].return_type = Type::Closure(0),
            8 => {
                module.closure_signatures.higher[signature - 1001].parameters =
                    vec![Type::Closure(1001)]
            }
            _ => {
                let entry = &mut module.functions[0];
                let result = entry
                    .body
                    .operations
                    .iter()
                    .find_map(|op| match op.kind {
                        Op::ClosureCall { result, .. }
                            if matches!(entry.values[result.0], Type::Closure(_)) =>
                        {
                            Some(result)
                        }
                        _ => None,
                    })
                    .unwrap();
                entry.values[result.0] = Type::Closure(0);
            }
        }
        assert!(
            verify_module(&module).is_err(),
            "nested result mutation {change}"
        );
    }
}

#[test]
fn primitive_callback_result_kinds_match_calls_and_parameter_cells() {
    let source = "export function run(a,b){let apply=(fn,x)=>fn(x);return apply(y=>y?null:b,a);}";
    for change in 0..4 {
        let mut module = crate::lower_source_with_entry_type(source, false, Type::General).unwrap();
        verify_module_with_entry_type(&module, Type::General).unwrap();
        let callback = module
            .closure_signatures
            .id(&[Type::Number], Type::General)
            .unwrap();
        match change {
            0 => module.closure_signatures.higher[callback - 1001].result = Type::Number,
            1 => module.closure_signatures.higher[callback - 1001].parameters[0] = Type::General,
            2 => {
                let function = module
                    .functions
                    .iter_mut()
                    .find(|f| {
                        f.slots
                            .iter()
                            .any(|s| s.parameter && s.ty == Type::Closure(callback))
                    })
                    .unwrap();
                let slot = function
                    .slots
                    .iter_mut()
                    .find(|s| s.parameter && s.ty == Type::Closure(callback))
                    .unwrap();
                slot.ty = Type::Closure(1);
            }
            _ => {
                let function = module
                    .functions
                    .iter_mut()
                    .find(|f| {
                        f.slots
                            .iter()
                            .any(|s| s.parameter && s.ty == Type::Closure(callback))
                    })
                    .unwrap();
                let result = function
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
                function.values[result.0] = Type::Number;
            }
        }
        assert!(
            verify_module_with_entry_type(&module, Type::General).is_err(),
            "primitive callback mutation {change}"
        );
    }
}

#[test]
fn higher_callback_graphs_are_bounded_and_keep_scalar_results() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut module = module();
            let mut previous = 0;
            for _ in 0..512 {
                previous = module
                    .closure_signatures
                    .intern(vec![Type::Closure(previous)], Type::Number);
            }
            assert!(module.closure_signatures.is_callback_signature(previous));
            verify_module(&module).unwrap();
            let last = previous - 1001;
            module.closure_signatures.higher[last].parameters[0] = Type::Closure(previous);
            assert!(!module.closure_signatures.is_callback_signature(previous));
            assert!(verify_module(&module).is_err());
        })
        .unwrap()
        .join()
        .unwrap();
    let mut module = module();
    let primitive = module
        .closure_signatures
        .intern(vec![Type::Number], Type::General);
    let callback = module
        .closure_signatures
        .intern(vec![Type::Closure(primitive)], Type::Number);
    assert!(module.closure_signatures.is_callback_signature(callback));
    verify_module(&module).unwrap();
    module.closure_signatures.higher[primitive - 1001].result = Type::Predicate;
    assert!(!module.closure_signatures.is_callback_signature(callback));
    assert!(verify_module(&module).is_err());
}
