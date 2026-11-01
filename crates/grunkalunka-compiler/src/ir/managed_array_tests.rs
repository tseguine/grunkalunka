use super::{
    managed_value_tests::{effect, node, value},
    *,
};

fn module() -> Module {
    let mut m = crate::lower_source("export function run(a,b){return a;}", false).unwrap();
    m.functions[0].values.clear();
    m.functions[0].body.operations.clear();
    m
}

#[test]
fn array_lengths_include_holes_without_dense_allocation() {
    for length in [0, 2, 1 << 31, u32::MAX] {
        let mut m = module();
        let f = &mut m.functions[0];
        let array = value(f, Type::ManagedArray, |result| Op::ManagedArrayNew {
            result,
            length,
        });
        let result = value(f, Type::Number, |result| Op::ManagedArrayLength {
            result,
            array,
        });
        f.body.exit = Exit::Return(result);
        let wasm = wat::parse_str(crate::emit::emit(&m).unwrap()).unwrap();
        node(&wasm, &format!("new Array({length}).length"));
    }
}

fn primitive(f: &mut Function, slot: usize) -> ValueId {
    let n = value(f, Type::Number, |result| Op::Load {
        result,
        slot: SlotId(slot),
    });
    let g = value(f, Type::General, |result| Op::Pack { result, input: n });
    value(f, Type::ManagedValue, |result| Op::ManagedPrimitive {
        result,
        input: g,
    })
}

fn get(f: &mut Function, array: ValueId, key: &str) -> ValueId {
    value(f, Type::ManagedValue, |result| Op::ManagedArrayGetOwn {
        result,
        array,
        key: key.encode_utf16().collect(),
    })
}

fn set(f: &mut Function, array: ValueId, key: &str, input: ValueId) {
    effect(
        f,
        Op::ManagedArraySetOwn {
            array,
            key: key.encode_utf16().collect(),
            value: input,
        },
    );
}

fn equal(f: &mut Function, left: ValueId, right: ValueId) -> ValueId {
    let p = value(f, Type::Predicate, |result| Op::ManagedEqual {
        result,
        left,
        right,
    });
    value(f, Type::Number, |result| Op::NumberFromPredicate {
        result,
        input: p,
    })
}

fn execute(m: &Module, expected: &str) {
    let wasm = wat::parse_str(crate::emit::emit(m).unwrap()).unwrap();
    node(&wasm, expected);
}

#[test]
fn array_index_writes_grow_length_but_named_properties_do_not() {
    for initial in [0, 3] {
        for key in [
            "0",
            "2",
            "4294967294",
            "4294967295",
            "4294967296",
            "01",
            "00",
            "-0",
            "-1",
            "+1",
            "1.0",
            "1e2",
            "",
            "2147483648",
            "99999999999",
            "١",
        ] {
            let mut m = module();
            let f = &mut m.functions[0];
            let array = value(f, Type::ManagedArray, |result| Op::ManagedArrayNew {
                result,
                length: initial,
            });
            let a = primitive(f, 0);
            set(f, array, key, a);
            let length = value(f, Type::Number, |result| Op::ManagedArrayLength {
                result,
                array,
            });
            f.body.exit = Exit::Return(length);
            execute(
                &m,
                &format!("(()=>{{const x=new Array({initial});x[{key:?}]=a;return x.length}})()"),
            );
        }
    }
}

#[test]
fn array_reads_keep_snapshots_and_length_is_a_number_value() {
    let mut m = module();
    let f = &mut m.functions[0];
    let array = value(f, Type::ManagedArray, |result| Op::ManagedArrayNew {
        result,
        length: 1,
    });
    let a = primitive(f, 0);
    let b = primitive(f, 1);
    set(f, array, "0", a);
    let old = get(f, array, "0");
    set(f, array, "0", b);
    let new = get(f, array, "0");
    let x = equal(f, old, a);
    let y = equal(f, new, b);
    let result = value(f, Type::Number, |result| Op::Binary {
        result,
        operator: Binary::Subtract,
        left: x,
        right: y,
    });
    f.body.exit = Exit::Return(result);
    execute(
        &m,
        "(()=>{const x=[a],old=x[0];x[0]=b;return Number(old===a)-Number(x[0]===b)})()",
    );
    let f = &mut m.functions[0];
    let got = get(f, array, "length");
    let a = primitive(f, 0);
    let result = equal(f, got, a);
    f.body.exit = Exit::Return(result);
    execute(&m, "Number([b].length===a)");
}

#[test]
fn array_shrink_removes_elements_and_growth_does_not_restore_them() {
    let mut m = module();
    let f = &mut m.functions[0];
    let array = value(f, Type::ManagedArray, |result| Op::ManagedArrayNew {
        result,
        length: 0,
    });
    let a = primitive(f, 0);
    let b = primitive(f, 1);
    set(f, array, "0", a);
    set(f, array, "2", b);
    set(f, array, "01", b);
    let old = get(f, array, "2");
    effect(f, Op::ManagedArrayResize { array, length: 1 });
    effect(f, Op::ManagedArrayResize { array, length: 3 });
    let hole = get(f, array, "2");
    let missing = get(f, array, "missing");
    let named = get(f, array, "01");
    let x = equal(f, hole, missing);
    let y = equal(f, named, b);
    let z = equal(f, old, b);
    let xy = value(f, Type::Number, |result| Op::Binary {
        result,
        operator: Binary::Add,
        left: x,
        right: y,
    });
    let result = value(f, Type::Number, |result| Op::Binary {
        result,
        operator: Binary::Add,
        left: xy,
        right: z,
    });
    f.body.exit = Exit::Return(result);
    execute(
        &m,
        "(()=>{const x=[];x[0]=a;x[2]=b;x['01']=b;const old=x[2];x.length=1;x.length=3;return Number(x[2]===x.missing)+Number(x['01']===b)+Number(old===b)})()",
    );
}

#[test]
fn arrays_box_as_object_identities_and_remain_truthy() {
    let mut m = module();
    let f = &mut m.functions[0];
    let array = value(f, Type::ManagedArray, |result| Op::ManagedArrayNew {
        result,
        length: 0,
    });
    let fresh = value(f, Type::ManagedArray, |result| Op::ManagedArrayNew {
        result,
        length: 0,
    });
    let object = value(f, Type::ManagedObject, |result| Op::ManagedObjectNew {
        result,
    });
    let boxed = value(f, Type::ManagedValue, |result| Op::ManagedFromArray {
        result,
        input: array,
    });
    let alias = value(f, Type::ManagedValue, |result| Op::ManagedFromArray {
        result,
        input: array,
    });
    let other = value(f, Type::ManagedValue, |result| Op::ManagedFromArray {
        result,
        input: fresh,
    });
    let obj = value(f, Type::ManagedValue, |result| Op::ManagedFromObject {
        result,
        input: object,
    });
    let x = equal(f, boxed, alias);
    let y = equal(f, boxed, other);
    let z = equal(f, boxed, obj);
    let p = value(f, Type::Predicate, |result| Op::ManagedTruthy {
        result,
        input: boxed,
    });
    let t = value(f, Type::Number, |result| Op::NumberFromPredicate {
        result,
        input: p,
    });
    let xy = value(f, Type::Number, |result| Op::Binary {
        result,
        operator: Binary::Subtract,
        left: x,
        right: y,
    });
    let xyz = value(f, Type::Number, |result| Op::Binary {
        result,
        operator: Binary::Subtract,
        left: xy,
        right: z,
    });
    let result = value(f, Type::Number, |result| Op::Binary {
        result,
        operator: Binary::Add,
        left: xyz,
        right: t,
    });
    f.body.exit = Exit::Return(result);
    execute(
        &m,
        "(()=>{const x=[],alias=x;return Number(x===alias)-Number(x===[])-Number(x==={})+Number(!!x)})()",
    );
}

#[test]
fn typed_helpers_pass_and_return_array_references() {
    let mut m = module();
    let f = &mut m.functions[0];
    let span = f.body.span;
    let array = value(f, Type::ManagedArray, |result| Op::ManagedArrayNew {
        result,
        length: 7,
    });
    let alias = value(f, Type::ManagedArray, |result| Op::Call {
        result,
        function: FunctionId(1),
        arguments: vec![array],
    });
    let boxed = value(f, Type::ManagedValue, |result| Op::ManagedFromArray {
        result,
        input: array,
    });
    let other = value(f, Type::ManagedValue, |result| Op::ManagedFromArray {
        result,
        input: alias,
    });
    let result = equal(f, boxed, other);
    f.body.exit = Exit::Return(result);
    m.functions.push(Function {
        return_type: Type::ManagedArray,
        parameter_count: 1,
        slots: vec![Slot {
            ty: Type::ManagedArray,
            owner: RegionId(0),
            mutable: true,
            parameter: true,
            span,
        }],
        values: vec![Type::ManagedArray],
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
    execute(
        &m,
        "(()=>{function id(v){return v}const a=new Array(7);return Number(a===id(a))})()",
    );
}

#[test]
fn array_named_keys_share_the_large_utf16_literal_path() {
    let mut m = module();
    let f = &mut m.functions[0];
    let array = value(f, Type::ManagedArray, |result| Op::ManagedArrayNew {
        result,
        length: 0,
    });
    let a = primitive(f, 0);
    let key = "x".repeat(10_001);
    set(f, array, &key, a);
    let got = get(f, array, &key);
    let result = equal(f, got, a);
    f.body.exit = Exit::Return(result);
    execute(&m, "Number(a===a)");
}

#[test]
fn arrays_reject_wrong_types_and_unvalidated_length_writes() {
    for change in 0..7 {
        let mut m = module();
        let f = &mut m.functions[0];
        let array = value(f, Type::ManagedArray, |result| Op::ManagedArrayNew {
            result,
            length: 0,
        });
        let a = primitive(f, 0);
        let op = match change {
            0 => Op::ManagedArraySetOwn {
                array,
                key: "length".encode_utf16().collect(),
                value: a,
            },
            1 => Op::ManagedArrayResize {
                array: a,
                length: 1,
            },
            2 => Op::ManagedArraySetOwn {
                array: a,
                key: vec![48],
                value: a,
            },
            3 => Op::ManagedArraySetOwn {
                array,
                key: vec![48],
                value: ValueId(1),
            },
            4 => Op::ManagedFromArray {
                result: ValueId(f.values.len()),
                input: a,
            },
            5 => Op::ManagedArrayGetOwn {
                result: ValueId(f.values.len()),
                array: a,
                key: vec![48],
            },
            _ => Op::ManagedArrayLength {
                result: ValueId(f.values.len()),
                array: a,
            },
        };
        f.values.push(Type::ManagedValue);
        effect(f, op);
        let result = value(f, Type::Number, |result| Op::Constant {
            result,
            value: 0.0,
        });
        f.body.exit = Exit::Return(result);
        let error = verify_module(&m).unwrap_err();
        if change == 0 {
            assert!(error.contains("validated resize"), "{error}");
        }
    }
    let mut m = module();
    let f = &mut m.functions[0];
    let result = value(f, Type::Number, |result| Op::ManagedArrayNew {
        result,
        length: 0,
    });
    f.body.exit = Exit::Return(result);
    assert!(verify_module(&m).is_err());
}

#[test]
fn arrays_use_traced_capture_cells_and_single_coroutine_globals() {
    let mut m = module();
    let f = &mut m.functions[0];
    let array = value(f, Type::ManagedArray, |result| Op::ManagedArrayNew {
        result,
        length: 0,
    });
    let result = value(f, Type::Number, |result| Op::Constant {
        result,
        value: 0.0,
    });
    f.body.exit = Exit::Return(result);
    let span = f.body.span;
    f.slots.push(Slot {
        ty: Type::ManagedArray,
        owner: RegionId(0),
        mutable: false,
        parameter: false,
        span,
    });
    effect(
        f,
        Op::Initialize {
            slot: SlotId(2),
            value: array,
        },
    );
    effect(
        f,
        Op::CaptureCells {
            cells: vec![SlotId(2)],
        },
    );
    verify_module(&m).unwrap();
    let wat = crate::emit::emit(&m).unwrap();
    assert!(wat.contains("$gc_cell"));
    assert!(!wat.contains("$cell_top"));
    wat::parse_str(wat).unwrap();
    m.functions[0].body.operations.pop();
    verify_module(&m).unwrap();
    let f = &mut m.functions[0];
    f.return_type = Type::General;
    let general = value(f, Type::General, |id| Op::Pack {
        result: id,
        input: result,
    });
    f.body.exit = Exit::Return(general);
    verify_async_module(&m).unwrap();
    wat::parse_str(crate::emit::emit_async(&m).unwrap()).unwrap();
}
