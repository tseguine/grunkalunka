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

#[test]
fn general_parameters_form_valid_private_closure_signatures() {
    let mut signatures = ClosureSignatures::default();
    let id = signatures.intern(
        vec![Type::General, Type::Number, Type::General],
        Type::Number,
    );
    assert!(signatures.is_callback_signature(id));
    assert!(signatures.is_returned_function(id));
    let predicate = signatures.intern(vec![Type::Predicate], Type::Number);
    assert!(!signatures.is_callback_signature(predicate));
    assert!(!signatures.is_returned_function(predicate));
}

fn primitive_argument_module(kind: u8, callback: bool, general_result: bool) -> Module {
    let span = Span::new(0, 1);
    let operation = |kind| Operation { span, kind };
    let parameter = |ty, owner| Slot {
        ty,
        owner: RegionId(owner),
        mutable: true,
        parameter: true,
        span,
    };
    let mut signatures = ClosureSignatures::default();
    let signature = signatures.intern(
        vec![Type::General, Type::Number, Type::General],
        Type::Number,
    );
    let first = match kind {
        0 => Op::Nullish {
            result: ValueId(2),
            value: Nullish::Undefined,
        },
        1 => Op::Nullish {
            result: ValueId(2),
            value: Nullish::Null,
        },
        2 | 4 => Op::Pack {
            result: ValueId(2),
            input: ValueId(6),
        },
        3 => Op::Pack {
            result: ValueId(2),
            input: ValueId(0),
        },
        _ => unreachable!(),
    };
    let entry = Function {
        return_type: Type::Number,
        parameter_count: 2,
        slots: vec![parameter(Type::Number, 0), parameter(Type::Number, 0)],
        values: vec![
            Type::Number,
            Type::Number,
            Type::General,
            Type::General,
            Type::Closure(signature),
            Type::Number,
            Type::Predicate,
        ],
        body: Region {
            id: RegionId(0),
            span,
            operations: vec![
                operation(Op::Load {
                    result: ValueId(0),
                    slot: SlotId(0),
                }),
                operation(Op::Load {
                    result: ValueId(1),
                    slot: SlotId(1),
                }),
                operation(Op::Boolean {
                    result: ValueId(6),
                    value: kind != 4,
                }),
                operation(first),
                operation(Op::Pack {
                    result: ValueId(3),
                    input: ValueId(6),
                }),
                operation(Op::MakeClosure {
                    result: ValueId(4),
                    function: FunctionId(1),
                    captures: vec![],
                }),
                operation(Op::ClosureCall {
                    result: ValueId(5),
                    callee: ValueId(4),
                    arguments: vec![ValueId(2), ValueId(1), ValueId(3)],
                }),
            ],
            exit: Exit::Return(ValueId(5)),
        },
    };
    let callee = Function {
        return_type: Type::Number,
        parameter_count: 3,
        slots: vec![
            parameter(Type::General, 1),
            parameter(Type::Number, 1),
            parameter(Type::General, 1),
        ],
        values: vec![
            Type::General,
            Type::Number,
            Type::General,
            Type::Number,
            Type::Number,
            Type::Number,
            Type::Number,
        ],
        body: Region {
            id: RegionId(1),
            span,
            operations: vec![
                operation(Op::ClosureEntry { captures: vec![] }),
                operation(Op::Load {
                    result: ValueId(0),
                    slot: SlotId(0),
                }),
                operation(Op::Load {
                    result: ValueId(1),
                    slot: SlotId(1),
                }),
                operation(Op::Load {
                    result: ValueId(2),
                    slot: SlotId(2),
                }),
                operation(Op::NumberFromGeneral {
                    result: ValueId(3),
                    input: ValueId(0),
                }),
                operation(Op::NumberFromGeneral {
                    result: ValueId(4),
                    input: ValueId(2),
                }),
                operation(Op::Binary {
                    result: ValueId(5),
                    operator: Binary::Add,
                    left: ValueId(3),
                    right: ValueId(1),
                }),
                operation(Op::Binary {
                    result: ValueId(6),
                    operator: Binary::Add,
                    left: ValueId(5),
                    right: ValueId(4),
                }),
            ],
            exit: Exit::Return(ValueId(6)),
        },
    };
    let mut module = Module {
        closure_signatures: signatures,
        functions: vec![entry, callee],
    };
    if callback {
        module.closure_signatures.higher[0].parameters[1] = Type::Closure(0);
        let caller = &mut module.functions[0];
        caller.values.push(Type::Closure(0));
        let at = caller.body.operations.len() - 1;
        caller.body.operations.insert(
            at,
            operation(Op::MakeClosure {
                result: ValueId(7),
                function: FunctionId(2),
                captures: vec![],
            }),
        );
        let Op::ClosureCall { arguments, .. } =
            &mut caller.body.operations.last_mut().unwrap().kind
        else {
            unreachable!()
        };
        arguments[1] = ValueId(7);
        let callee = &mut module.functions[1];
        callee.slots[1].ty = Type::Closure(0);
        callee.values[1] = Type::Closure(0);
        callee.values.push(Type::Number);
        callee.body.operations.insert(
            6,
            operation(Op::ClosureCall {
                result: ValueId(7),
                callee: ValueId(1),
                arguments: vec![],
            }),
        );
        let Op::Binary { right, .. } = &mut callee.body.operations[7].kind else {
            unreachable!()
        };
        *right = ValueId(7);
        callee.body.operations.insert(
            1,
            operation(Op::CaptureCells {
                cells: vec![SlotId(0)],
            }),
        );
        module.functions.push(Function {
            return_type: Type::Number,
            parameter_count: 0,
            slots: vec![],
            values: vec![Type::Number],
            body: Region {
                id: RegionId(2),
                span,
                operations: vec![
                    operation(Op::ClosureEntry { captures: vec![] }),
                    operation(Op::Constant {
                        result: ValueId(0),
                        value: 2.0,
                    }),
                ],
                exit: Exit::Return(ValueId(0)),
            },
        });
    }
    if general_result {
        module.closure_signatures.higher[0].result = Type::General;
        module.functions[0].return_type = Type::General;
        module.functions[0].values[5] = Type::General;
        module.functions[1].return_type = Type::General;
        module.functions[1].body.exit = Exit::Return(ValueId(0));
    }
    module
}

#[test]
fn mixed_primitive_closure_arguments_execute_without_overlapping_lanes() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    for kind in 0..=4 {
        for callback in [false, true] {
            for general_result in [false, true] {
                let module = primitive_argument_module(kind, callback, general_result);
                let wat = if general_result {
                    crate::emit::emit_with_entry_type(&module, Type::General)
                } else {
                    crate::emit::emit(&module)
                }
                .unwrap();
                let wasm = wat::parse_str(&wat).unwrap();
                let script = format!(
                    r#"const assert=require('node:assert/strict'),view=new DataView(new ArrayBuffer(8)),run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from({wasm:?}))).exports.run;
function bits(x){{if(Number.isNaN(x))return 'nan';view.setFloat64(0,x);return view.getBigUint64(0).toString(16);}}
function encode(x){{if(x===undefined)return [0,bits(0)];if(x===null)return [1,bits(0)];if(typeof x==='boolean')return [2,bits(+x)];return [3,bits(x)];}}
for(const[a,b]of[[4,2],[0,-0],[-0,-0],[-3.25,1.5],[NaN,Infinity],[Infinity,-Infinity],[5e-324,0]]){{const x=[undefined,null,true,a,false][{kind}],last={kind}===4?false:true,expected={general_result}?x:(+x+({callback}?2:b))+(+last),actual=run(a,b);if({general_result})assert.deepEqual([actual[0],bits(actual[1])],encode(expected));else assert.equal(bits(actual),bits(expected),'kind '+{kind}+' inputs '+a+','+b);}}
"#
                );
                let mut child = Command::new("node")
                    .arg("-")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .unwrap();
                child
                    .stdin
                    .take()
                    .unwrap()
                    .write_all(script.as_bytes())
                    .unwrap();
                let result = child.wait_with_output().unwrap();
                assert!(
                    result.status.success(),
                    "{}",
                    String::from_utf8_lossy(&result.stderr)
                );
                let hex = wasm
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>();
                println!("IR_GENERAL_ARGUMENT_WASM|{kind}-{callback}-{general_result}|{hex}");
            }
        }
    }
}

#[test]
fn general_closure_parameters_keep_private_abi_and_logical_type_guards() {
    for change in 0..7 {
        let mut module = primitive_argument_module(3, false, false);
        verify_module(&module).unwrap();
        match change {
            0 => module.functions[0].slots[0].ty = Type::General,
            1 => module.functions[1].slots[1].ty = Type::Predicate,
            2 => {
                let Op::ClosureCall { arguments, .. } =
                    &mut module.functions[0].body.operations.last_mut().unwrap().kind
                else {
                    unreachable!()
                };
                arguments[0] = ValueId(0);
            }
            3 => {
                let Op::ClosureCall { arguments, .. } =
                    &mut module.functions[0].body.operations.last_mut().unwrap().kind
                else {
                    unreachable!()
                };
                arguments.pop();
            }
            4 => module.functions[1].values[0] = Type::Number,
            5 => module.functions[1].slots[0].mutable = false,
            6 => module.closure_signatures.higher[0].parameters[0] = Type::Predicate,
            _ => unreachable!(),
        }
        assert!(
            verify_module(&module).is_err(),
            "invalid General parameter variant {change}"
        );
    }
    let mut ordinary = primitive_argument_module(3, false, false)
        .functions
        .remove(1);
    ordinary.body.operations.remove(0);
    assert!(
        verify(&ordinary)
            .unwrap_err()
            .contains("General source slots cannot be parameters")
    );
}

fn wide_general_argument_module(count: usize) -> Module {
    let mut module = primitive_argument_module(3, false, false);
    module.closure_signatures.higher[0].parameters = vec![Type::General; count];
    let callee = &mut module.functions[1];
    callee.parameter_count = count;
    let span = callee.body.span;
    callee.slots = (0..count)
        .map(|_| Slot {
            ty: Type::General,
            owner: callee.body.id,
            mutable: true,
            parameter: true,
            span,
        })
        .collect();
    for operation in &mut callee.body.operations {
        if let Op::Load { result, slot } = &mut operation.kind {
            if *result == ValueId(1) {
                operation.kind = Op::Constant {
                    result: ValueId(1),
                    value: 0.0,
                };
            } else if *result == ValueId(2) {
                *slot = SlotId(count - 1);
            }
        }
    }
    let Op::ClosureCall { arguments, .. } =
        &mut module.functions[0].body.operations.last_mut().unwrap().kind
    else {
        unreachable!()
    };
    *arguments = vec![ValueId(2); count];
    *arguments.last_mut().unwrap() = ValueId(3);
    module
}

#[test]
fn thousand_general_arguments_fit_the_private_frame_and_keep_the_existing_limit() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let module = wide_general_argument_module(1000);
    verify_module(&module).unwrap();
    let wasm = wat::parse_str(crate::emit::emit(&module).unwrap()).unwrap();
    let script = format!(
        "const assert=require('node:assert/strict'),run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from({wasm:?}))).exports.run;for(const a of[4,0,-0,-3.25,Infinity,-Infinity,5e-324])assert.ok(Object.is(run(a,2),a+1));assert.ok(Number.isNaN(run(NaN,2)));"
    );
    let mut child = Command::new("node")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let hex = wasm
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    println!("IR_GENERAL_ARGUMENT_WASM|wide|{hex}");
    assert!(verify_module(&wide_general_argument_module(1001)).is_err());
}
