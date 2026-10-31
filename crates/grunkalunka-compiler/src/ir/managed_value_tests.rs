use super::*;

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
fn managed_values_require_reference_aware_async_frames() {
    let mut module = primitive_comparison();
    module.functions[0].return_type = Type::General;
    module.functions[0].body.exit = Exit::Return(ValueId(1));
    verify_async_module(&module).unwrap();
    assert!(
        crate::emit::emit_async(&module)
            .unwrap_err()
            .contains("managed")
    );
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
fn managed_references_stay_out_of_exported_and_private_closure_signatures() {
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
        assert!(verify_module(&module).is_err());
        let mut module = object_identity();
        module.closure_signatures.higher.push(ClosureSignature {
            parameters: vec![Type::Number],
            result: ty,
            asynchronous: false,
        });
        assert!(verify_module(&module).is_err());
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
