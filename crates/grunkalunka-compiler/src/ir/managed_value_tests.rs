use super::*;

#[test]
fn primitive_projection_checks_visible_input_and_general_result() {
    for invalid in 0..4 {
        let mut module = crate::lower_source(
            "export function run(a,b){const o={v:null};return +o.v;}",
            false,
        )
        .unwrap();
        verify_module(&module).unwrap();
        let function = &mut module.functions[0];
        let (result, input) = function
            .body
            .operations
            .iter_mut()
            .find_map(|op| {
                if let Op::ManagedToPrimitive { result, input } = &mut op.kind {
                    Some((result, input))
                } else {
                    None
                }
            })
            .unwrap();
        match invalid {
            0 => *input = ValueId(usize::MAX),
            1 => *input = *result,
            2 => function.values[result.0] = Type::Number,
            _ => function.values[input.0] = Type::General,
        }
        assert!(verify_module(&module).is_err(), "mutation {invalid}");
    }
}

#[test]
fn string_repeat_verifier_checks_receiver_count_and_result_types() {
    for invalid in 0..6 {
        let mut module = crate::lower_source(
            "export function run(a,b){return 'xy'.repeat(3).length;}",
            false,
        )
        .unwrap();
        verify_module(&module).unwrap();
        let function = &mut module.functions[0];
        let (result, input, count) = function
            .body
            .operations
            .iter_mut()
            .find_map(|op| {
                if let Op::ManagedStringRepeat {
                    result,
                    input,
                    count,
                } = &mut op.kind
                {
                    Some((result, input, count))
                } else {
                    None
                }
            })
            .expect("repeat operation");
        match invalid {
            0 => *input = ValueId(usize::MAX),
            1 => *count = ValueId(usize::MAX),
            2 => *input = *result,
            3 => *count = *result,
            4 => function.values[result.0] = Type::Number,
            _ => function.values[count.0] = Type::ManagedValue,
        }
        assert!(verify_module(&module).is_err(), "mutation {invalid}");
    }
    let numeric = crate::compile("export function run(a,b){return a+b;}", false).unwrap();
    assert!(!numeric.wat.contains("$gc_string_repeat"));
}

#[test]
fn string_comparison_checks_visible_managed_operands_and_predicate_result() {
    for invalid in 0..7 {
        let mut module =
            crate::lower_source("export function run(a,b){return 'a'<'b'?1:0;}", false).unwrap();
        verify_module(&module).unwrap();
        let function = &mut module.functions[0];
        let (result, left, right) = function
            .body
            .operations
            .iter_mut()
            .find_map(|op| {
                if let Op::ManagedStringCompare {
                    result,
                    left,
                    right,
                    ..
                } = &mut op.kind
                {
                    Some((result, left, right))
                } else {
                    None
                }
            })
            .unwrap();
        match invalid {
            0 => *left = ValueId(usize::MAX),
            1 => *right = ValueId(usize::MAX),
            2 => *left = *result,
            3 => *right = *result,
            4 => function.values[result.0] = Type::Number,
            5 => function.values[left.0] = Type::Number,
            _ => function.values[right.0] = Type::General,
        }
        assert!(verify_module(&module).is_err(), "mutation {invalid}");
    }
    let numeric = crate::compile("export function run(a,b){return a<b?1:0;}", false).unwrap();
    assert!(!numeric.wat.contains("$gc_string_compare"));
}

#[test]
fn property_delete_checks_receiver_key_result_and_nonconfigurable_length() {
    for invalid in 0..5 {
        let mut module = crate::lower_source(
            "export function run(a,b){const xs=[a];return delete xs[0]?1:0;}",
            false,
        )
        .unwrap();
        verify_module(&module).unwrap();
        let function = &mut module.functions[0];
        let (input, key, result) = function
            .body
            .operations
            .iter_mut()
            .find_map(|op| {
                if let Op::ManagedDeleteOwn { input, key, result } = &mut op.kind {
                    Some((input, key, result))
                } else {
                    None
                }
            })
            .unwrap();
        match invalid {
            0 => *input = ValueId(usize::MAX),
            1 => *key = ManagedKey::Literal("length".encode_utf16().collect()),
            2 => *key = ManagedKey::String(*input),
            3 => function.values[result.0] = Type::Number,
            _ => *input = *result,
        }
        assert!(verify_module(&module).is_err(), "mutation {invalid}");
    }
}

#[test]
fn dictionary_dynamic_write_checks_every_operand_before_emission() {
    let source = "export function run(a,b){const d=Object.create(null);const key='x';d[key]=a;return d.x===a?1:0;}";
    for invalid in 0..4 {
        let mut module = crate::lower_source(source, false).unwrap();
        let function = &mut module.functions[0];
        let (object, key, value) = function
            .body
            .operations
            .iter_mut()
            .find_map(|operation| {
                if let Op::ManagedSetOwnDynamic { object, key, value } = &mut operation.kind {
                    Some((object, key, value))
                } else {
                    None
                }
            })
            .unwrap();
        let ManagedKey::String(key) = key else {
            panic!("dictionary String key");
        };
        match invalid {
            0 => *object = *key,
            1 => *key = *object,
            2 => *value = *object,
            _ => *value = ValueId(usize::MAX),
        }
        assert!(verify_module(&module).is_err());
    }
}

#[test]
fn dictionary_dynamic_read_checks_operands_and_result_before_emission() {
    let source = "export function run(a,b){const d=Object.create(null);d.x=a;const key='x';return d[key]===a?1:0;}";
    for invalid in 0..4 {
        let mut module = crate::lower_source(source, false).unwrap();
        let function = &mut module.functions[0];
        let (result, object, key) = function
            .body
            .operations
            .iter_mut()
            .find_map(|operation| {
                if let Op::ManagedGetOwnDynamic {
                    result,
                    object,
                    key,
                } = &mut operation.kind
                {
                    Some((result, object, key))
                } else {
                    None
                }
            })
            .unwrap();
        let ManagedKey::String(key) = key else {
            panic!("String key");
        };
        match invalid {
            0 => *object = *key,
            1 => *key = *object,
            2 => function.values[result.0] = Type::Number,
            _ => *key = ValueId(usize::MAX),
        }
        assert!(verify_module(&module).is_err());
    }
}

fn primitive_comparison() -> Module {
    let mut module = crate::lower_source("export function run(a,b){return a;}", false).unwrap();
    let function = &mut module.functions[0];
    function.values = vec![
        Type::Number,
        Type::General,
        Type::Number,
        Type::General,
        Type::ManagedValue,
        Type::ManagedValue,
        Type::Predicate,
        Type::Number,
    ];
    let span = function.body.span;
    function.body.operations = vec![
        Op::Load {
            result: ValueId(0),
            slot: SlotId(0),
        },
        Op::Pack {
            result: ValueId(1),
            input: ValueId(0),
        },
        Op::Load {
            result: ValueId(2),
            slot: SlotId(1),
        },
        Op::Pack {
            result: ValueId(3),
            input: ValueId(2),
        },
        Op::ManagedPrimitive {
            result: ValueId(4),
            input: ValueId(1),
        },
        Op::ManagedPrimitive {
            result: ValueId(5),
            input: ValueId(3),
        },
        Op::ManagedEqual {
            result: ValueId(6),
            left: ValueId(4),
            right: ValueId(5),
        },
        Op::NumberFromPredicate {
            result: ValueId(7),
            input: ValueId(6),
        },
    ]
    .into_iter()
    .map(|kind| Operation { span, kind })
    .collect();
    function.body.exit = Exit::Return(ValueId(7));
    module
}

fn object_identity() -> Module {
    let mut module = crate::lower_source("export function run(a,b){return a;}", false).unwrap();
    let function = &mut module.functions[0];
    function.values = vec![
        Type::ManagedObject,
        Type::ManagedObject,
        Type::ManagedValue,
        Type::ManagedValue,
        Type::ManagedValue,
        Type::Predicate,
        Type::Predicate,
        Type::Number,
        Type::Number,
        Type::Number,
    ];
    let span = function.body.span;
    function.body.operations = vec![
        Op::ManagedObjectNew { result: ValueId(0) },
        Op::ManagedObjectNew { result: ValueId(1) },
        Op::ManagedFromObject {
            result: ValueId(2),
            input: ValueId(0),
        },
        Op::ManagedFromObject {
            result: ValueId(3),
            input: ValueId(0),
        },
        Op::ManagedFromObject {
            result: ValueId(4),
            input: ValueId(1),
        },
        Op::ManagedEqual {
            result: ValueId(5),
            left: ValueId(2),
            right: ValueId(3),
        },
        Op::ManagedEqual {
            result: ValueId(6),
            left: ValueId(2),
            right: ValueId(4),
        },
        Op::NumberFromPredicate {
            result: ValueId(7),
            input: ValueId(5),
        },
        Op::NumberFromPredicate {
            result: ValueId(8),
            input: ValueId(6),
        },
        Op::Binary {
            result: ValueId(9),
            operator: Binary::Subtract,
            left: ValueId(7),
            right: ValueId(8),
        },
    ]
    .into_iter()
    .map(|kind| Operation { span, kind })
    .collect();
    function.body.exit = Exit::Return(ValueId(9));
    module
}

#[test]
fn managed_object_identity_survives_reboxing_without_merging_fresh_objects() {
    let wasm = wat::parse_str(crate::emit::emit(&object_identity()).unwrap()).unwrap();
    node(&wasm, "1");
}

#[test]
fn ordinary_helpers_pass_and_return_managed_object_references() {
    let mut module = object_identity();
    let function = &mut module.functions[0];
    let span = function.body.span;
    function.values.push(Type::ManagedObject);
    function.body.operations.insert(
        2,
        Operation {
            span,
            kind: Op::Call {
                result: ValueId(10),
                function: FunctionId(1),
                arguments: vec![ValueId(0)],
            },
        },
    );
    function.body.operations[4].kind = Op::ManagedFromObject {
        result: ValueId(3),
        input: ValueId(10),
    };
    module.functions.push(Function {
        return_type: Type::ManagedObject,
        parameter_count: 1,
        slots: vec![Slot {
            ty: Type::ManagedObject,
            owner: RegionId(0),
            mutable: true,
            parameter: true,
            span,
        }],
        values: vec![Type::ManagedObject],
        body: Region {
            id: RegionId(0),
            span,
            operations: vec![Operation {
                span,
                kind: Op::Load {
                    result: ValueId(0),
                    slot: SlotId(0),
                },
            }],
            exit: Exit::Return(ValueId(0)),
        },
    });
    let wasm = wat::parse_str(crate::emit::emit(&module).unwrap()).unwrap();
    node(&wasm, "1");
}

pub(super) fn value(
    function: &mut Function,
    ty: Type,
    operation: impl FnOnce(ValueId) -> Op,
) -> ValueId {
    let id = ValueId(function.values.len());
    function.values.push(ty);
    effect(function, operation(id));
    id
}

pub(super) fn effect(function: &mut Function, kind: Op) {
    function.body.operations.push(Operation {
        span: function.body.span,
        kind,
    });
}

fn property_snapshots() -> Module {
    let mut module = crate::lower_source("export function run(a,b){return a;}", false).unwrap();
    let f = &mut module.functions[0];
    f.values.clear();
    f.body.operations.clear();
    let a = value(f, Type::Number, |result| Op::Load {
        result,
        slot: SlotId(0),
    });
    let b = value(f, Type::Number, |result| Op::Load {
        result,
        slot: SlotId(1),
    });
    let ga = value(f, Type::General, |result| Op::Pack { result, input: a });
    let gb = value(f, Type::General, |result| Op::Pack { result, input: b });
    let va = value(f, Type::ManagedValue, |result| Op::ManagedPrimitive {
        result,
        input: ga,
    });
    let vb = value(f, Type::ManagedValue, |result| Op::ManagedPrimitive {
        result,
        input: gb,
    });
    let object = value(f, Type::ManagedObject, |result| Op::ManagedObjectNew {
        result,
    });
    let key: Vec<u16> = "x".encode_utf16().collect();
    effect(
        f,
        Op::ManagedSetOwn {
            object,
            key: key.clone(),
            value: va,
        },
    );
    let old = value(f, Type::ManagedValue, |result| Op::ManagedGetOwn {
        result,
        object,
        key: key.clone(),
    });
    effect(
        f,
        Op::ManagedSetOwn {
            object,
            key: key.clone(),
            value: vb,
        },
    );
    let new = value(f, Type::ManagedValue, |result| Op::ManagedGetOwn {
        result,
        object,
        key,
    });
    let old_matches = value(f, Type::Predicate, |result| Op::ManagedEqual {
        result,
        left: old,
        right: va,
    });
    let new_matches = value(f, Type::Predicate, |result| Op::ManagedEqual {
        result,
        left: new,
        right: vb,
    });
    let old_number = value(f, Type::Number, |result| Op::NumberFromPredicate {
        result,
        input: old_matches,
    });
    let new_number = value(f, Type::Number, |result| Op::NumberFromPredicate {
        result,
        input: new_matches,
    });
    let result = value(f, Type::Number, |result| Op::Binary {
        result,
        operator: Binary::Subtract,
        left: old_number,
        right: new_number,
    });
    f.body.exit = Exit::Return(result);
    module
}

#[test]
fn managed_property_updates_preserve_prior_value_snapshots() {
    let wasm = wat::parse_str(crate::emit::emit(&property_snapshots()).unwrap()).unwrap();
    node(
        &wasm,
        "(()=>{const o={x:a},old=o.x;o.x=b;return Number(old===a)-Number(o.x===b);})()",
    );
}

#[test]
fn managed_properties_distinguish_utf16_keys_and_missing_values() {
    for (first, second) in [
        (vec![120], vec![121]),
        (vec![], vec![0]),
        (vec![0xd800, 0], vec![0xfffd, 0]),
        (vec![0xd800], vec![0xd800, 0]),
        (vec![49], vec![48, 49]),
    ] {
        let missing = vec![12345, 0, 0xd800];
        let mut module = crate::lower_source("export function run(a,b){return a;}", false).unwrap();
        let f = &mut module.functions[0];
        f.values.clear();
        f.body.operations.clear();
        let mut actuals = Vec::new();
        for slot in [0, 1] {
            let number = value(f, Type::Number, |result| Op::Load {
                result,
                slot: SlotId(slot),
            });
            let general = value(f, Type::General, |result| Op::Pack {
                result,
                input: number,
            });
            actuals.push(value(f, Type::ManagedValue, |result| {
                Op::ManagedPrimitive {
                    result,
                    input: general,
                }
            }));
        }
        let object = value(f, Type::ManagedObject, |result| Op::ManagedObjectNew {
            result,
        });
        for (key, &actual) in [&first, &second].into_iter().zip(&actuals) {
            effect(
                f,
                Op::ManagedSetOwn {
                    object,
                    key: key.clone(),
                    value: actual,
                },
            );
        }
        let mut numbers = Vec::new();
        for (key, &actual) in [&first, &second].into_iter().zip(&actuals) {
            let got = value(f, Type::ManagedValue, |result| Op::ManagedGetOwn {
                result,
                object,
                key: key.clone(),
            });
            let equal = value(f, Type::Predicate, |result| Op::ManagedEqual {
                result,
                left: got,
                right: actual,
            });
            numbers.push(value(f, Type::Number, |result| Op::NumberFromPredicate {
                result,
                input: equal,
            }));
        }
        let undefined = value(f, Type::General, |result| Op::Nullish {
            result,
            value: Nullish::Undefined,
        });
        let undefined = value(f, Type::ManagedValue, |result| Op::ManagedPrimitive {
            result,
            input: undefined,
        });
        let absent = value(f, Type::ManagedValue, |result| Op::ManagedGetOwn {
            result,
            object,
            key: missing.clone(),
        });
        let absent_equal = value(f, Type::Predicate, |result| Op::ManagedEqual {
            result,
            left: absent,
            right: undefined,
        });
        let absent_number = value(f, Type::Number, |result| Op::NumberFromPredicate {
            result,
            input: absent_equal,
        });
        let difference = value(f, Type::Number, |result| Op::Binary {
            result,
            operator: Binary::Subtract,
            left: numbers[0],
            right: numbers[1],
        });
        let result = value(f, Type::Number, |result| Op::Binary {
            result,
            operator: Binary::Add,
            left: difference,
            right: absent_number,
        });
        f.body.exit = Exit::Return(result);
        let key = |units: &[u16]| {
            format!(
                "String.fromCharCode({})",
                units
                    .iter()
                    .map(u16::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        let expected = format!(
            "(()=>{{const o={{}},ka={},kb={};o[ka]=a;o[kb]=b;return Number(o[ka]===a)-Number(o[kb]===b)+Number(o[{}]===undefined);}})()",
            key(&first),
            key(&second),
            key(&missing)
        );
        let wasm = wat::parse_str(crate::emit::emit(&module).unwrap()).unwrap();
        node(&wasm, &expected);
    }
}

#[test]
fn managed_primitive_equality_matches_js_number_semantics() {
    let module = primitive_comparison();
    verify_module(&module).unwrap();
    let wat = crate::emit::emit(&module).unwrap();
    let wasm = wat::parse_str(wat).unwrap();
    node(&wasm, "Number(a===b)");
}

#[test]
fn managed_primitives_keep_nullish_boolean_and_number_kinds() {
    for left in ["undefined", "null", "false", "true", "a"] {
        for right in ["undefined", "null", "false", "true", "b"] {
            let mut module = primitive_comparison();
            let function = &mut module.functions[0];
            for (name, value, packed) in [(left, 0, 1), (right, 2, 3)] {
                match name {
                    "undefined" | "null" => {
                        function.body.operations[packed].kind = Op::Nullish {
                            result: ValueId(packed),
                            value: if name == "undefined" {
                                Nullish::Undefined
                            } else {
                                Nullish::Null
                            },
                        }
                    }
                    "false" | "true" => {
                        function.values[value] = Type::Predicate;
                        function.body.operations[value].kind = Op::Boolean {
                            result: ValueId(value),
                            value: name == "true",
                        };
                    }
                    _ => (),
                }
            }
            let wat = crate::emit::emit(&module).unwrap();
            node(
                &wat::parse_str(wat).unwrap(),
                &format!("Number({left}==={right})"),
            );
        }
    }
}

#[test]
fn managed_values_use_traced_capture_cells() {
    let mut module = primitive_comparison();
    let function = &mut module.functions[0];
    let span = function.body.span;
    function.slots.push(Slot {
        ty: Type::ManagedValue,
        owner: function.body.id,
        mutable: false,
        parameter: false,
        span,
    });
    function.body.operations.push(Operation {
        span,
        kind: Op::Initialize {
            slot: SlotId(2),
            value: ValueId(4),
        },
    });
    function.body.operations.push(Operation {
        span,
        kind: Op::CaptureCells {
            cells: vec![SlotId(2)],
        },
    });
    verify_module(&module).unwrap();
    let wat = crate::emit::emit(&module).unwrap();
    assert!(wat.contains("$gc_cell"));
    assert!(!wat.contains("$cell_top"));
    wat::parse_str(wat).unwrap();
}

#[test]
fn managed_values_use_reference_globals_in_single_coroutine_frames() {
    let mut module = primitive_comparison();
    module.functions[0].return_type = Type::General;
    module.functions[0].body.exit = Exit::Return(ValueId(1));
    verify_async_module(&module).unwrap();
    wat::parse_str(crate::emit::emit_async(&module).unwrap()).unwrap();
}

#[test]
fn managed_operations_reject_wrong_receivers_values_and_results() {
    for change in 0..4 {
        let mut module = property_snapshots();
        let function = &mut module.functions[0];
        match change {
            0 => {
                let Op::ManagedGetOwn { object, .. } = &mut function
                    .body
                    .operations
                    .iter_mut()
                    .find(|op| matches!(op.kind, Op::ManagedGetOwn { .. }))
                    .unwrap()
                    .kind
                else {
                    unreachable!()
                };
                *object = ValueId(4);
            }
            1 => {
                let Op::ManagedSetOwn { value, .. } = &mut function
                    .body
                    .operations
                    .iter_mut()
                    .find(|op| matches!(op.kind, Op::ManagedSetOwn { .. }))
                    .unwrap()
                    .kind
                else {
                    unreachable!()
                };
                *value = ValueId(6);
            }
            2 => {
                let Op::ManagedGetOwn { result, .. } = function
                    .body
                    .operations
                    .iter()
                    .find(|op| matches!(op.kind, Op::ManagedGetOwn { .. }))
                    .unwrap()
                    .kind
                else {
                    unreachable!()
                };
                function.values[result.0] = Type::ManagedObject;
            }
            _ => function.body.operations.swap(0, 7),
        }
        assert!(verify_module(&module).is_err(), "change {change}");
    }
}

#[test]
fn managed_exports_remain_guarded_while_private_signatures_are_typed() {
    for ty in [Type::ManagedValue, Type::ManagedObject, Type::ManagedArray] {
        let mut module = object_identity();
        module.functions[0].slots[0].ty = ty;
        assert!(verify_module(&module).is_err());
        assert!(verify_module_with_entry_type(&object_identity(), ty).is_err());
        let mut module = object_identity();
        module.closure_signatures.higher.push(ClosureSignature {
            parameters: vec![ty],
            result: Type::Number,
            asynchronous: false,
        });
        verify_module(&module).unwrap();
        assert!(module.closure_signatures.is_callback_signature(1001));
        assert!(module.closure_signatures.is_returned_function(1001));
        let mut module = object_identity();
        module.closure_signatures.higher.push(ClosureSignature {
            parameters: vec![Type::Number],
            result: ty,
            asynchronous: false,
        });
        verify_module(&module).unwrap();
        assert!(module.closure_signatures.is_callback_signature(1001));
        assert!(module.closure_signatures.is_returned_function(1001));
    }
}

#[test]
fn managed_object_source_cells_preserve_load_snapshots() {
    let mut module = object_identity();
    let function = &mut module.functions[0];
    let span = function.body.span;
    function.slots.push(Slot {
        ty: Type::ManagedObject,
        owner: function.body.id,
        mutable: true,
        parameter: false,
        span,
    });
    function.values.push(Type::ManagedObject);
    function.body.operations.insert(
        2,
        Operation {
            span,
            kind: Op::Initialize {
                slot: SlotId(2),
                value: ValueId(0),
            },
        },
    );
    function.body.operations.insert(
        3,
        Operation {
            span,
            kind: Op::Load {
                result: ValueId(10),
                slot: SlotId(2),
            },
        },
    );
    function.body.operations.insert(
        4,
        Operation {
            span,
            kind: Op::Store {
                slot: SlotId(2),
                value: ValueId(1),
            },
        },
    );
    function.body.operations[6].kind = Op::ManagedFromObject {
        result: ValueId(3),
        input: ValueId(10),
    };
    let current = value(function, Type::ManagedObject, |result| Op::Load {
        result,
        slot: SlotId(2),
    });
    let current = value(function, Type::ManagedValue, |result| {
        Op::ManagedFromObject {
            result,
            input: current,
        }
    });
    let matches = value(function, Type::Predicate, |result| Op::ManagedEqual {
        result,
        left: current,
        right: ValueId(4),
    });
    let number = value(function, Type::Number, |result| Op::NumberFromPredicate {
        result,
        input: matches,
    });
    let result = value(function, Type::Number, |result| Op::Binary {
        result,
        operator: Binary::Add,
        left: ValueId(9),
        right: number,
    });
    function.body.exit = Exit::Return(result);
    let wasm = wat::parse_str(crate::emit::emit(&module).unwrap()).unwrap();
    node(&wasm, "2");
}

#[test]
fn mixed_reference_and_number_helper_parameters_preserve_mutation() {
    let mut module = property_snapshots();
    let f = &mut module.functions[0];
    let span = f.body.span;
    f.values.push(Type::ManagedObject);
    f.body.operations[9].kind = Op::Call {
        result: ValueId(14),
        function: FunctionId(1),
        arguments: vec![ValueId(6), ValueId(5), ValueId(1)],
    };
    let n = value(f, Type::ManagedValue, |result| Op::ManagedGetOwn {
        result,
        object: ValueId(6),
        key: vec![110],
    });
    let equal = value(f, Type::Predicate, |result| Op::ManagedEqual {
        result,
        left: n,
        right: ValueId(5),
    });
    let number = value(f, Type::Number, |result| Op::NumberFromPredicate {
        result,
        input: equal,
    });
    let two = value(f, Type::Number, |result| Op::Constant {
        result,
        value: 2.0,
    });
    let weighted = value(f, Type::Number, |result| Op::Binary {
        result,
        operator: Binary::Multiply,
        left: number,
        right: two,
    });
    let result = value(f, Type::Number, |result| Op::Binary {
        result,
        operator: Binary::Add,
        left: ValueId(13),
        right: weighted,
    });
    f.body.exit = Exit::Return(result);
    let types = [Type::ManagedObject, Type::ManagedValue, Type::Number];
    let mut helper = Function {
        return_type: Type::ManagedObject,
        parameter_count: 3,
        slots: types
            .into_iter()
            .map(|ty| Slot {
                ty,
                owner: RegionId(0),
                mutable: true,
                parameter: true,
                span,
            })
            .collect(),
        values: vec![],
        body: Region {
            id: RegionId(0),
            span,
            operations: vec![],
            exit: Exit::Fallthrough,
        },
    };
    let object = value(&mut helper, Type::ManagedObject, |result| Op::Load {
        result,
        slot: SlotId(0),
    });
    let input = value(&mut helper, Type::ManagedValue, |result| Op::Load {
        result,
        slot: SlotId(1),
    });
    let number = value(&mut helper, Type::Number, |result| Op::Load {
        result,
        slot: SlotId(2),
    });
    let general = value(&mut helper, Type::General, |result| Op::Pack {
        result,
        input: number,
    });
    let managed = value(&mut helper, Type::ManagedValue, |result| {
        Op::ManagedPrimitive {
            result,
            input: general,
        }
    });
    effect(
        &mut helper,
        Op::ManagedSetOwn {
            object,
            key: vec![120],
            value: input,
        },
    );
    effect(
        &mut helper,
        Op::ManagedSetOwn {
            object,
            key: vec![110],
            value: managed,
        },
    );
    helper.body.exit = Exit::Return(object);
    module.functions.push(helper);
    let wasm = wat::parse_str(crate::emit::emit(&module).unwrap()).unwrap();
    node(
        &wasm,
        "(()=>{function set(o,v,n){o.x=v;o.n=n;return o;}const o={x:a},old=o.x;set(o,b,b);return Number(old===a)-Number(o.x===b)+2*Number(o.n===b);})()",
    );
}

pub(super) fn node(wasm: &[u8], expected: &str) {
    use std::{
        io::Write,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let script = format!(
        r#"const assert=require('node:assert/strict');
const run=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({wasm:?}))).exports.run;
const native=(a,b)=>{expected},values=[0,-0,1,-7,Infinity,-Infinity,NaN,Number.MIN_VALUE];
for(const a of values)for(const b of values)assert.ok(Object.is(run(a,b),native(a,b)),String([a,b]));"#
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
    let started = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() > Duration::from_secs(30) {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!(
                "managed IR Node test timed out: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn string_construction_requires_a_managed_result() {
    let mut module = crate::lower_source("export function run(a,b){return a;}", false).unwrap();
    let function = &mut module.functions[0];
    function.values = vec![Type::Number];
    function.body.operations = vec![Operation {
        span: function.body.span,
        kind: Op::ManagedString {
            result: ValueId(0),
            units: vec![120],
        },
    }];
    function.body.exit = Exit::Return(ValueId(0));
    assert!(verify_module(&module).is_err());
}

#[test]
fn string_length_requires_a_visible_managed_input_and_number_result() {
    let mut module =
        crate::lower_source("export function run(a,b){return 'abc'.length;}", false).unwrap();
    let function = &mut module.functions[0];
    let index = function
        .body
        .operations
        .iter()
        .position(|op| matches!(op.kind, Op::ManagedStringLength { .. }))
        .unwrap();
    let Op::ManagedStringLength { result, input } = function.body.operations[index].kind else {
        unreachable!()
    };
    verify_module(&module).unwrap();
    module.functions[0].values[result.0] = Type::ManagedValue;
    assert!(verify_module(&module).is_err());
    module.functions[0].values[result.0] = Type::Number;
    module.functions[0].body.operations[index].kind = Op::ManagedStringLength {
        result,
        input: result,
    };
    assert!(verify_module(&module).is_err());
    module.functions[0].body.operations[index].kind = Op::ManagedStringLength { result, input };
    module.functions[0].values[input.0] = Type::Number;
    assert!(verify_module(&module).is_err());
}

#[test]
fn string_concat_requires_visible_managed_operands_and_result() {
    let mut module =
        crate::lower_source("export function run(a,b){return ('a'+'b').length;}", false).unwrap();
    let function = &mut module.functions[0];
    let index = function
        .body
        .operations
        .iter()
        .position(|op| matches!(op.kind, Op::ManagedStringConcat { .. }))
        .unwrap();
    let Op::ManagedStringConcat {
        result,
        left,
        right,
    } = function.body.operations[index].kind
    else {
        unreachable!()
    };
    verify_module(&module).unwrap();
    module.functions[0].values[result.0] = Type::Number;
    assert!(verify_module(&module).is_err());
    module.functions[0].values[result.0] = Type::ManagedValue;
    for (left, right) in [(result, right), (left, result)] {
        module.functions[0].body.operations[index].kind = Op::ManagedStringConcat {
            result,
            left,
            right,
        };
        assert!(verify_module(&module).is_err());
    }
}

#[test]
fn string_trimming_requires_managed_input_and_result() {
    let mut module = crate::lower_source(
        "export function run(a,b){return ' text '.trim(a=1).length;}",
        false,
    )
    .unwrap();
    let function = &mut module.functions[0];
    let number = function
        .body
        .operations
        .iter()
        .find_map(|op| match op.kind {
            Op::Constant { result, .. } => Some(result),
            _ => None,
        })
        .unwrap();
    let index = function
        .body
        .operations
        .iter()
        .position(|op| matches!(op.kind, Op::ManagedStringTrim { .. }))
        .unwrap();
    let Op::ManagedStringTrim {
        result,
        input: _,
        ends,
    } = function.body.operations[index].kind
    else {
        unreachable!()
    };
    verify_module(&module).unwrap();
    module.functions[0].values[result.0] = Type::Number;
    assert!(verify_module(&module).is_err());
    module.functions[0].values[result.0] = Type::ManagedValue;
    for input in [number, result] {
        module.functions[0].body.operations[index].kind = Op::ManagedStringTrim {
            result,
            input,
            ends,
        };
        assert!(verify_module(&module).is_err());
    }
}

#[test]
fn string_search_requires_managed_operands_number_position_and_predicate_result() {
    let mut module = crate::lower_source(
        "export function run(a,b){return 'abcdef'.includes('bc',a)?1:0;}",
        false,
    )
    .unwrap();
    let function = &mut module.functions[0];
    let index = function
        .body
        .operations
        .iter()
        .position(|op| matches!(op.kind, Op::ManagedStringSearch { .. }))
        .unwrap();
    let Op::ManagedStringSearch {
        result,
        input,
        search,
        position,
        kind,
    } = function.body.operations[index].kind
    else {
        unreachable!()
    };
    verify_module(&module).unwrap();
    module.functions[0].values[result.0] = Type::Number;
    assert!(verify_module(&module).is_err());
    module.functions[0].values[result.0] = Type::Predicate;
    for (input, search, position) in [
        (position, search, position),
        (input, position, position),
        (input, search, input),
        (input, search, result),
    ] {
        module.functions[0].body.operations[index].kind = Op::ManagedStringSearch {
            result,
            input,
            search,
            position,
            kind,
        };
        assert!(verify_module(&module).is_err());
    }
}

#[test]
fn string_indices_require_managed_operands_and_number_position_and_result() {
    for method in ["indexOf", "lastIndexOf"] {
        let mut module = crate::lower_source(
            &format!("export function run(a,b){{return 'abcdef'.{method}('bc',a);}}"),
            false,
        )
        .unwrap();
        let function = &mut module.functions[0];
        let index = function
            .body
            .operations
            .iter()
            .position(|op| matches!(op.kind, Op::ManagedStringIndex { .. }))
            .unwrap();
        let Op::ManagedStringIndex {
            result,
            input,
            search,
            position,
            backwards,
        } = function.body.operations[index].kind
        else {
            unreachable!()
        };
        verify_module(&module).unwrap();
        module.functions[0].values[result.0] = Type::Predicate;
        assert!(verify_module(&module).is_err());
        module.functions[0].values[result.0] = Type::Number;
        for (input, search, position) in [
            (position, search, position),
            (input, position, position),
            (input, search, input),
            (input, search, result),
        ] {
            module.functions[0].body.operations[index].kind = Op::ManagedStringIndex {
                result,
                input,
                search,
                position,
                backwards,
            };
            assert!(verify_module(&module).is_err());
        }
    }
}

#[test]
fn string_character_operations_keep_text_index_and_result_types() {
    for (expression, expected) in [
        ("'abc'.charAt(a).length", Type::ManagedValue),
        ("'abc'.at(a)===undefined?1:0", Type::ManagedValue),
        ("'abc'.codePointAt(a)===undefined?1:0", Type::ManagedValue),
        ("'abc'.charCodeAt(a)", Type::Number),
    ] {
        let mut module = crate::lower_source(
            &format!("export function run(a,b){{return {expression};}}"),
            false,
        )
        .unwrap();
        let function = &mut module.functions[0];
        let operation = function
            .body
            .operations
            .iter()
            .find(|op| {
                matches!(
                    op.kind,
                    Op::ManagedStringCharAt { .. }
                        | Op::ManagedStringCodePointAt { .. }
                        | Op::ManagedStringAt { .. }
                        | Op::ManagedStringCharCodeAt { .. }
                )
            })
            .unwrap();
        let (result, input, index) = match operation.kind {
            Op::ManagedStringCharAt {
                result,
                input,
                index,
            }
            | Op::ManagedStringCodePointAt {
                result,
                input,
                index,
            }
            | Op::ManagedStringAt {
                result,
                input,
                index,
            }
            | Op::ManagedStringCharCodeAt {
                result,
                input,
                index,
            } => (result, input, index),
            _ => unreachable!(),
        };
        verify_module(&module).unwrap();
        module.functions[0].values[result.0] = Type::Predicate;
        assert!(verify_module(&module).is_err());
        module.functions[0].values[result.0] = expected;
        module.functions[0].values[input.0] = Type::Number;
        assert!(verify_module(&module).is_err());
        module.functions[0].values[input.0] = Type::ManagedValue;
        module.functions[0].values[index.0] = Type::ManagedValue;
        assert!(verify_module(&module).is_err());
    }
}

#[test]
fn string_ranges_require_managed_text_and_visible_number_endpoints() {
    let mut module = crate::lower_source(
        "export function run(a,b){return 'abcdef'.slice(a,b).length;}",
        false,
    )
    .unwrap();
    let function = &mut module.functions[0];
    let index = function
        .body
        .operations
        .iter()
        .position(|op| matches!(op.kind, Op::ManagedStringRange { .. }))
        .unwrap();
    let Op::ManagedStringRange {
        result,
        input,
        start,
        end,
        substring,
    } = function.body.operations[index].kind
    else {
        unreachable!()
    };
    verify_module(&module).unwrap();
    module.functions[0].values[result.0] = Type::Number;
    assert!(verify_module(&module).is_err());
    module.functions[0].values[result.0] = Type::ManagedValue;
    for (input, start, end) in [
        (start, start, end),
        (input, input, end),
        (input, start, result),
    ] {
        module.functions[0].body.operations[index].kind = Op::ManagedStringRange {
            result,
            input,
            start,
            end,
            substring,
        };
        assert!(verify_module(&module).is_err());
    }
}

#[test]
fn managed_nullish_predicate_keeps_the_carrier_type_boundary() {
    let mut module = primitive_comparison();
    module.functions[0].body.operations[6].kind = Op::ManagedIsNullish {
        result: ValueId(6),
        input: ValueId(4),
    };
    verify_module(&module).unwrap();
    let wasm = wat::parse_str(crate::emit::emit(&module).unwrap()).unwrap();
    node(&wasm, "Number(a===null||a===undefined)");
    module.functions[0].body.operations[6].kind = Op::ManagedIsNullish {
        result: ValueId(6),
        input: ValueId(1),
    };
    assert!(verify_module(&module).is_err());
    module.functions[0].body.operations[6].kind = Op::ManagedIsNullish {
        result: ValueId(6),
        input: ValueId(4),
    };
    module.functions[0].values[6] = Type::Number;
    assert!(verify_module(&module).is_err());
}

#[test]
fn array_brand_requires_a_boxed_input_and_a_predicate_result() {
    let mut module = primitive_comparison();
    let check = |input| Op::ManagedTest {
        test: ManagedTest::Array,
        result: ValueId(6),
        input,
    };
    module.functions[0].body.operations[6].kind = check(ValueId(4));
    verify_module(&module).unwrap();
    module.functions[0].body.operations[6].kind = check(ValueId(1));
    assert!(verify_module(&module).is_err());
    module.functions[0].body.operations[6].kind = check(ValueId(4));
    module.functions[0].values[6] = Type::Number;
    assert!(verify_module(&module).is_err());
}

#[test]
fn managed_typeof_requires_a_managed_input_and_managed_text_result() {
    let mut module = primitive_comparison();
    module.functions[0].body.operations[5].kind = Op::ManagedTypeof {
        result: ValueId(5),
        input: ValueId(4),
    };
    verify_module(&module).unwrap();
    let wasm = wat::parse_str(crate::emit::emit(&module).unwrap()).unwrap();
    node(&wasm, "Number(a===typeof a)");
    module.functions[0].body.operations[5].kind = Op::ManagedTypeof {
        result: ValueId(5),
        input: ValueId(1),
    };
    assert!(verify_module(&module).is_err());
    module.functions[0].body.operations[5].kind = Op::ManagedTypeof {
        result: ValueId(5),
        input: ValueId(4),
    };
    module.functions[0].values[5] = Type::Predicate;
    assert!(verify_module(&module).is_err());
}

#[test]
fn string_iteration_emits_its_reader_only_for_a_proven_string_loop() {
    for (input, needs_reader) in [("'a'", true), ("['a']", false)] {
        let source = format!(
            "export function run(a,b){{let n=0;for(const ch of {input})n+=ch.length;return n;}}"
        );
        let module = crate::lower_source(&source, false).unwrap();
        verify_module(&module).unwrap();
        assert_eq!(
            any_operation(&module.functions[0].body, &mut |op| matches!(
                op,
                Op::ManagedStringIterValue { .. }
            )),
            needs_reader
        );
        let wat = crate::emit::emit(&module).unwrap();
        assert_eq!(wat.contains("(func $gc_string_iter_value"), needs_reader);
    }
}

#[test]
fn string_spread_and_collection_helpers_share_one_index_formatter() {
    for body in [
        "return [...'ab'].length;",
        "const xs=[...'ab'];const o={...xs};const rows=Object.entries(o);const copy=[...rows];copy.pop();return copy.at(0)===rows[0]?1:0;",
    ] {
        let module =
            crate::lower_source(&format!("export function run(a,b){{{body}}}"), false).unwrap();
        verify_module(&module).unwrap();
        let wat = crate::emit::emit(&module).unwrap();
        assert_eq!(wat.matches("(func $gc_spread_index ").count(), 1);
        assert_eq!(wat.matches("(func $gc_string_to_array ").count(), 1);
        wat::parse_str(&wat).unwrap();
    }
}

#[test]
fn string_at_does_not_prove_a_present_string_or_coerce_arguments() {
    for body in [
        "return 'abc'.at(a).length;",
        "return 'abc'.at(a).trim().length;",
        "const c='abc'.at(a);return c.charAt(0).length;",
        "return 'abc'.at('1')===undefined?1:0;",
        "return 'abc'.at(null)===undefined?1:0;",
        "return 'abc'.at(true)===undefined?1:0;",
        "const s=a?'abc':null;return s.at(0)===undefined?1:0;",
        "return 'abc'.at(...[0])===undefined?1:0;",
        "return 'abc'?.at(0)===undefined?1:0;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(crate::lower_source(&source, false).is_err(), "{source}");
    }
}

#[test]
fn code_point_at_keeps_boxed_results_and_requires_proven_inputs() {
    for body in [
        "const c='abc'.codePointAt(a);return +c;",
        "return 'abc'.codePointAt(a).length;",
        "return 'abc'.codePointAt(a).trim().length;",
        "return 'abc'.codePointAt('1')===undefined?1:0;",
        "return 'abc'.codePointAt(null)===undefined?1:0;",
        "return 'abc'.codePointAt(true)===undefined?1:0;",
        "const s=a?'abc':null;return s.codePointAt(0)===undefined?1:0;",
        "return 'abc'.codePointAt(...[0])===undefined?1:0;",
        "return 'abc'?.codePointAt(0)===undefined?1:0;",
        "const f='abc'.codePointAt;return f(0);",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(crate::lower_source(&source, false).is_err(), "{source}");
    }
}

#[test]
fn string_at_assignment_defaults_prove_text_without_admitting_unproven_reads() {
    let source =
        "export function run(a,b){let c='';({c='?'}={c:'abc'.at(a)});return c.charCodeAt(0);}";
    verify_module(&crate::lower_source(source, false).unwrap()).unwrap();
    for source in [
        "export function run(a,b){let c='';({c}={c:'abc'.at(a)});return c.charCodeAt(0);}",
        "export function run(a,b){let c='';({c=1}={c:'abc'.at(a)});return c.charCodeAt(0);}",
    ] {
        assert!(crate::lower_source(source, false).is_err(), "{source}");
    }
}

#[test]
fn well_formed_validation_requires_text_and_boolean_result() {
    let mut m = crate::lower_source(
        "export function run(a,b){return 'abc'.isWellFormed()?a:b;}",
        false,
    )
    .unwrap();
    let op = m.functions[0]
        .body
        .operations
        .iter()
        .find(|o| matches!(o.kind, Op::ManagedStringIsWellFormed { .. }))
        .unwrap();
    let Op::ManagedStringIsWellFormed { result, input } = op.kind else {
        unreachable!()
    };
    verify_module(&m).unwrap();
    m.functions[0].values[result.0] = Type::Number;
    assert!(verify_module(&m).is_err());
    m.functions[0].values[result.0] = Type::Predicate;
    m.functions[0].values[input.0] = Type::Number;
    assert!(verify_module(&m).is_err());
    for body in [
        "const s=a?'abc':null;return s.isWellFormed()?1:0;",
        "return 'abc'?.isWellFormed()?1:0;",
        "return 'abc'.isWellFormed(...[])?1:0;",
    ] {
        assert!(
            crate::lower_source(&format!("export function run(a,b){{{body}}}"), false).is_err()
        );
    }
}

#[test]
fn well_formed_repair_requires_managed_text_and_result() {
    let mut m = crate::lower_source(
        "export function run(a,b){return 'abc'.toWellFormed().length;}",
        false,
    )
    .unwrap();
    let op = m.functions[0]
        .body
        .operations
        .iter()
        .find(|o| matches!(o.kind, Op::ManagedStringToWellFormed { .. }))
        .unwrap();
    let Op::ManagedStringToWellFormed { result, input } = op.kind else {
        unreachable!()
    };
    verify_module(&m).unwrap();
    m.functions[0].values[result.0] = Type::Number;
    assert!(verify_module(&m).is_err());
    m.functions[0].values[result.0] = Type::ManagedValue;
    m.functions[0].values[input.0] = Type::Number;
    assert!(verify_module(&m).is_err());
    for body in [
        "const s=a?'abc':null;return s.toWellFormed().length;",
        "return 'abc'?.toWellFormed().length;",
        "return 'abc'.toWellFormed(...[]).length;",
    ] {
        assert!(
            crate::lower_source(&format!("export function run(a,b){{{body}}}"), false).is_err()
        );
    }
}

#[test]
fn string_concat_method_requires_string_arguments_and_bounded_arity() {
    for body in [
        "return ''.concat(a).length;",
        "return ''.concat(undefined).length;",
        "return ''.concat(null).length;",
        "return ''.concat(true).length;",
        "return ''.concat({}).length;",
        "return ''.concat(...['a']).length;",
        "return ''?.concat('a').length;",
        "const s=a?'a':null;return s.concat('b').length;",
    ] {
        assert!(
            crate::lower_source(&format!("export function run(a,b){{{body}}}"), false).is_err(),
            "{body}"
        );
    }
    for (n, admitted) in [(0, true), (3, true), (1000, true), (1001, false)] {
        let arguments = vec!["''"; n].join(",");
        let source = format!("export function run(a,b){{return 'x'.concat({arguments}).length;}}");
        assert_eq!(crate::lower_source(&source, false).is_ok(), admitted, "{n}");
    }
}

#[test]
fn templates_require_supported_primitives_and_bound_substitution_count() {
    for body in [
        "return `${a?b:[]}`.length;",
        "return `${{}}`.length;",
        "return `${a?'a':{}}`.length;",
        "return `${()=>a}`.length;",
        "return tag`hello`.length;",
    ] {
        assert!(
            crate::lower_source(&format!("export function run(a,b){{{body}}}"), false).is_err(),
            "{body}"
        );
    }
    for (operand, needs_formatter) in [
        ("a", true),
        ("a?'text':b", true),
        ("a>0", false),
        ("12.75", false),
        ("'only text'", false),
    ] {
        let source = format!("export function run(a,b){{return `${{{operand}}}`.length;}}");
        let wat = crate::compile(&source, false).unwrap().wat;
        assert_eq!(wat.contains("(global $inv5"), needs_formatter, "{source}");
    }
    for (n, admitted) in [(0, true), (1000, true), (1001, false)] {
        let parts = "${''}".repeat(n);
        let source = format!("export function run(a,b){{return `{parts}`.length;}}");
        assert_eq!(crate::lower_source(&source, false).is_ok(), admitted, "{n}");
    }
}

#[test]
fn numeric_string_index_requires_number_and_keeps_missing_results() {
    let mut m = crate::lower_source(
        "export function run(a,b){return 'abc'[a]===undefined?1:0;}",
        false,
    )
    .unwrap();
    let op = m.functions[0]
        .body
        .operations
        .iter()
        .find(|o| matches!(o.kind, Op::ManagedStringGetIndex { .. }))
        .unwrap();
    let Op::ManagedStringGetIndex {
        result,
        input,
        index,
    } = op.kind
    else {
        unreachable!()
    };
    verify_module(&m).unwrap();
    for value in [result, input] {
        m.functions[0].values[value.0] = Type::Number;
        assert!(verify_module(&m).is_err());
        m.functions[0].values[value.0] = Type::ManagedValue;
    }
    m.functions[0].values[index.0] = Type::ManagedValue;
    assert!(verify_module(&m).is_err());
    assert!(
        crate::lower_source(
            "export function run(a,b){return 'abc'?.[a]===undefined?1:0;}",
            false
        )
        .is_ok()
    );
    for body in [
        "return 'abc'[a].length;",
        "return 'abc'[a].trim().length;",
        "return 'abc'[null]===undefined?1:0;",
        "return 'abc'[true]===undefined?1:0;",
        "const s=a?'abc':null;return s[0]===undefined?1:0;",
    ] {
        assert!(
            crate::lower_source(&format!("export function run(a,b){{{body}}}"), false).is_err(),
            "{body}"
        );
    }
}

#[test]
fn nullish_results_preserve_source_receiver_origins() {
    for source in [
        "export function run(a,b){const label=[' ready '].at(a)??' missing ';return label.trim().length;}",
        "export function run(a,b){const label=(a?null:' ready ')??' missing ';return label.trim().length;}",
        "function choose(x,y){return x??y;}export function run(a,b){const label=choose(a?null:' ready ',' missing ');return label.trim().length;}",
        "export function run(a,b){const rows=[{label:' ready '}];const read=()=>rows.at(a)??{label:' missing '};return read().label.trim().length;}",
        "export function run(a,b){const row={label:' ready '}??a;return row.label.trim().length;}",
        "export function run(a,b){const row=(a?null:[a,b])??[];return row.length;}",
        "export function run(a,b){const label='abc'.at(a)??'?';return label.trim().length;}",
        "export function run(a,b){const box={label:null};const alias=box;alias.label=' ready ';const label=box.label??' missing ';return label.trim().length;}",
    ] {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                for typescript in [false, true] {
                    let result = crate::compile(source, typescript);
                    assert!(result.is_ok(), "{source}: {:?}", result.err());
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}

#[test]
fn nullish_result_origins_keep_receiver_and_unselected_source_guards() {
    for body in [
        "const label=a??'fallback';return label.trim().length;",
        "const label=(a?'text':false)??'fallback';return label.trim().length;",
        "const label=(a?'text':0)??'fallback';return label.trim().length;",
        "const label=(a?'text':0/0)??'fallback';return label.trim().length;",
        "const label=['text',a].at(a)??'fallback';return label.trim().length;",
        "const source={label:'text'};const alias=source;alias.label=a;const label=source.label??'fallback';return label.trim().length;",
        "const source=[{label:'text'}];const row=source.at(a)??{label:'fallback'};source[0].label=b;return row.label.trim().length;",
        "const label=(a?'text':undefined)??null;return label.trim().length;",
        "const label='text'??missing();return label.trim().length;",
        "const row={label:'text'}??{[{}]:b};return row.label.trim().length;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typescript in [false, true] {
            let error = crate::compile(&source, typescript).err().unwrap();
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

#[test]
fn logical_result_origins_preserve_selected_receivers() {
    for source in [
        "export function run(a,b){const label=(a?null:' ready ')||' missing ';return label.trim().length;}",
        "export function run(a,b){const row={label:' ready '}||a;return row.label.trim().length;}",
        "export function run(a,b){const row=[]&&{label:' ready '};return row.label.trim().length;}",
        "export function run(a,b){const label=(null&&a)??' missing ';return label.trim().length;}",
        "function choose(x,y){return x||y;}export function run(a,b){const label=choose(a?null:' ready ',' missing ');return label.trim().length;}",
        "export function run(a,b){const rows=[{label:' ready '}]||a;const read=()=>rows.at(a)??{label:' missing '};return read().label.trim().length;}",
        "export function run(a,b){const marker=()=>a;const row=marker&&{label:' ready '};return row.label.trim().length;}",
        "function marker(x){return x;}export function run(a,b){const row=marker&&{label:' ready '};return row.label.trim().length;}",
        "export function run(a,b){const row=Object.create(null);row.label=' ready ';const selected=row||a;const label=selected.label??' missing ';return label.trim().length;}",
    ] {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                for typescript in [false, true] {
                    let result = crate::compile(source, typescript);
                    assert!(result.is_ok(), "{source}: {:?}", result.err());
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}

#[test]
fn logical_result_origins_keep_unknown_truthiness_and_unselected_source_guards() {
    for body in [
        "const label=(a?null:b)||'fallback';return label.trim().length;",
        "const label=(a?'text':false)||'fallback';return label.trim().length;",
        "const label=0||'fallback';return label.trim().length;",
        "const label='text'||a;return label.trim().length;",
        "const row='text'&&{label:'ready'};return row.label.trim().length;",
        "const label=a&&'text';return label.trim().length;",
        "const label=(a?{}:null)&&'text';return label.trim().length;",
        "const source={label:'text'};const row=source||{label:'fallback'};source.label=b;return row.label.trim().length;",
        "const row={label:'text'}||missing();return row.label.trim().length;",
        "const label=(null&&missing())??'fallback';return label.trim().length;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typescript in [false, true] {
            let error = crate::compile(&source, typescript).err().unwrap();
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

#[test]
fn scalar_array_keys_cannot_be_replaced_by_unchecked_string_values() {
    let source = "export function run(a,b){const xs=[];xs[a]=b;return xs.length;}";
    let mut module = crate::lower_source(source, false).unwrap();
    verify_module(&module).unwrap();
    let key = module.functions[0]
        .body
        .operations
        .iter_mut()
        .find_map(|op| match &mut op.kind {
            Op::ManagedSetOwnDynamic { key, .. } => Some(key),
            _ => None,
        })
        .unwrap();
    let ManagedKey::Scalar {
        input,
        numbers: true,
    } = key
    else {
        panic!("expected primitive key")
    };
    *key = ManagedKey::String(*input);
    assert!(verify_module(&module).is_err());
}

#[test]
fn uint32_delete_key_requires_a_visible_number_snapshot() {
    let source = "export function run(a,b){const xs=[a];return delete xs[b>>>0]?1:0;}";
    for invalid in 0..4 {
        let mut module = crate::lower_source(source, false).unwrap();
        verify_module(&module).unwrap();
        let function = &mut module.functions[0];
        let (result, input, key) = function
            .body
            .operations
            .iter_mut()
            .find_map(|operation| {
                if let Op::ManagedDeleteOwn { result, input, key } = &mut operation.kind {
                    Some((result, input, key))
                } else {
                    None
                }
            })
            .unwrap();
        let ManagedKey::Uint32(value) = key else {
            panic!("numeric key");
        };
        match invalid {
            0 => *value = ValueId(usize::MAX),
            1 => *value = *result,
            2 => *value = *input,
            _ => function.values[value.0] = Type::ManagedValue,
        }
        assert!(verify_module(&module).is_err(), "mutation {invalid}");
    }
}

#[test]
fn unsigned_object_definition_validates_receiver_key_and_value() {
    let source = "export function run(a,b){const o={[a>>>0]:b};return Object.keys(o).length;}";
    for invalid in 0..6 {
        let mut module = crate::lower_source(source, false).unwrap();
        verify_module(&module).unwrap();
        let function = &mut module.functions[0];
        let (object, key, value) = function
            .body
            .operations
            .iter_mut()
            .find_map(|operation| {
                if let Op::ManagedSetOwnDynamic { object, key, value } = &mut operation.kind {
                    Some((object, key, value))
                } else {
                    None
                }
            })
            .unwrap();
        let ManagedKey::Uint32(number) = key else {
            panic!("numeric key");
        };
        let number = *number;
        match invalid {
            0 => *object = number,
            1 => *key = ManagedKey::Uint32(*value),
            2 => *key = ManagedKey::Uint32(ValueId(usize::MAX)),
            3 => *value = *object,
            4 => *key = ManagedKey::String(number),
            _ => *object = ValueId(usize::MAX),
        }
        assert!(verify_module(&module).is_err(), "mutation {invalid}");
    }
}

#[test]
fn generic_object_definition_supports_large_literal_key_carriers() {
    let source = "export function run(a,b){const o={[a>>>0]:b};return Object.keys(o).length;}";
    let mut module = crate::lower_source(source, false).unwrap();
    for operation in &mut module.functions[0].body.operations {
        if let Op::ManagedSetOwnDynamic { key, .. } = &mut operation.kind {
            *key = ManagedKey::Literal(vec![u16::from(b'x'); 10001]);
        }
    }
    verify_module(&module).unwrap();
    let wasm = wat::parse_str(crate::emit::emit(&module).unwrap()).unwrap();
    node(&wasm, "1");
}

#[test]
fn unsigned_array_definition_checks_length_key_and_operands() {
    let source = "export function run(a,b){const xs=[];xs[a>>>0]=b;return xs.length;}";
    for invalid in 0..6 {
        let mut module = crate::lower_source(source, false).unwrap();
        verify_module(&module).unwrap();
        let function = &mut module.functions[0];
        let (object, key, value) = function
            .body
            .operations
            .iter_mut()
            .find_map(|operation| {
                if let Op::ManagedSetOwnDynamic { object, key, value } = &mut operation.kind {
                    Some((object, key, value))
                } else {
                    None
                }
            })
            .unwrap();
        let ManagedKey::Uint32(number) = key else {
            panic!("numeric key");
        };
        let number = *number;
        match invalid {
            0 => *key = ManagedKey::Literal("length".encode_utf16().collect()),
            1 => *key = ManagedKey::String(*value),
            2 => *key = ManagedKey::Uint32(ValueId(usize::MAX)),
            3 => *value = *object,
            4 => *object = number,
            _ => *key = ManagedKey::Uint32(*value),
        }
        assert!(verify_module(&module).is_err(), "mutation {invalid}");
    }
}

#[test]
fn unsigned_array_reads_validate_key_receiver_and_result() {
    for bad in 0..5 {
        let mut m = crate::lower_source(
            "export function run(a,b){const xs=[a];return +xs[b>>>0];}",
            false,
        )
        .unwrap();
        verify_module(&m).unwrap();
        let f = &mut m.functions[0];
        let (r, o, k) = f
            .body
            .operations
            .iter_mut()
            .find_map(|op| {
                if let Op::ManagedGetOwnDynamic {
                    result,
                    object,
                    key,
                } = &mut op.kind
                {
                    Some((result, object, key))
                } else {
                    None
                }
            })
            .unwrap();
        match bad {
            0 => *k = ManagedKey::Uint32(ValueId(usize::MAX)),
            1 => *k = ManagedKey::String(*r),
            2 => *k = ManagedKey::Literal("length".encode_utf16().collect()),
            3 => *o = *r,
            _ => f.values[r.0] = Type::Number,
        }
        assert!(verify_module(&m).is_err());
    }
}
