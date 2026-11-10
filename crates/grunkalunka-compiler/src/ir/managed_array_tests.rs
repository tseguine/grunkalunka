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
fn array_at_checks_its_receiver_and_index_before_emission() {
    let source = "export function run(a,b){const x=[a,b];return x.at(a)===b?1:0;}";
    for wrong_index in [false, true] {
        let mut module = crate::lower_source(source, false).unwrap();
        let function = &mut module.functions[0];
        let operation = function
            .body
            .operations
            .iter_mut()
            .find_map(|operation| {
                if let Op::ManagedArrayAt { array, index, .. } = &mut operation.kind {
                    Some((array, index))
                } else {
                    None
                }
            })
            .unwrap();
        if wrong_index {
            *operation.1 = *operation.0;
        } else {
            *operation.0 = *operation.1;
        }
        assert!(verify_module(&module).unwrap_err().contains("value type"));
    }
}

#[test]
fn array_at_helpers_keep_boxed_results_after_string_at_recognition() {
    for source in [
        "function pick(x,i){return x.at(i);} export function run(a,b){const x=[a,b];return pick(x,1)===b?1:0;}",
        "function pick(x,i){return x['at'](i);} export function run(a,b){const o={value:a};return pick([o],0)===o?1:0;}",
        "function pick(x,i){return x.at(i);} function forward(x,i){return pick(x,i);} export function run(a,b){return forward([a],1)===undefined?1:0;}",
        "function pick(x,i){return x.at(i);} export function run(a,b){const choose=()=>pick([a,b],1);return choose()===b?1:0;}",
        "function pick(x,i){return x.at(i);} export function run(a,b){return pick('abc',a)==='b'?1:0;}",
        "function pick(x,i){return x.at(i);} export function run(a,b){const o={at:i=>i};return pick(o,a);}",
    ] {
        for typescript in [false, true] {
            let result = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || crate::compile(source, typescript))
                .unwrap()
                .join()
                .unwrap();
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
}

#[test]
fn at_helper_results_keep_receiver_and_missing_value_guards() {
    for source in [
        "function pick(x,i){return x.at(i);} export function run(a,b){return pick(['abc'],a).trim().length;}",
        "function pick(x,i){return x.at(i);} export function run(a,b){return pick('abc',a).trim().length;}",
        "function pick(x,i){return x.at(i);} export function run(a,b){const x=a?['abc']:null;return pick(x,0)===undefined?1:0;}",
        "function pick(x,i){return x.at(i);} export function run(a,b){return pick([a,b],'0')===a?1:0;}",
    ] {
        for typescript in [false, true] {
            let error = crate::compile(source, typescript).err().unwrap();
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

#[test]
fn array_pop_checks_its_receiver_and_boxed_result_before_emission() {
    let source = "export function run(a,b){const x=[a,b];return x.pop()===b?1:0;}";
    for wrong_result in [false, true] {
        let mut module = crate::lower_source(source, false).unwrap();
        let function = &mut module.functions[0];
        let number = ValueId(
            function
                .values
                .iter()
                .position(|ty| *ty == Type::Number)
                .unwrap(),
        );
        let operation = function
            .body
            .operations
            .iter_mut()
            .find_map(|operation| {
                if let Op::ManagedArrayPop { result, array } = &mut operation.kind {
                    Some((result, array))
                } else {
                    None
                }
            })
            .unwrap();
        if wrong_result {
            function.values[operation.0.0] = Type::Number;
        } else {
            *operation.1 = number;
        }
        let error = verify_module(&module).unwrap_err();
        assert!(
            error.contains(if wrong_result {
                "result value"
            } else {
                "value type"
            }),
            "{error}"
        );
    }
}

#[test]
fn array_rest_checks_its_receiver_and_array_result_before_emission() {
    for invalid in 0..3 {
        let mut m = module();
        let f = &mut m.functions[0];
        let source = value(f, Type::ManagedArray, |result| Op::ManagedArrayNew {
            result,
            length: 2,
        });
        let number = value(f, Type::Number, |result| Op::Constant {
            result,
            value: 0.0,
        });
        let rest = value(f, Type::ManagedArray, |result| Op::ManagedArrayRest {
            result,
            array: match invalid {
                0 => number,
                1 => ValueId(usize::MAX),
                _ => source,
            },
            start: 0,
        });
        if invalid == 2 {
            f.values[rest.0] = Type::ManagedValue;
        }
        f.body.exit = Exit::Return(number);
        assert!(verify_module(&m).is_err());
    }
}

#[test]
fn array_rest_densifies_holes_and_excludes_named_properties() {
    for (length, start) in [(0, 0), (4, 0), (4, 1), (4, 4), (4, u32::MAX)] {
        let mut m = module();
        let f = &mut m.functions[0];
        let source = value(f, Type::ManagedArray, |result| Op::ManagedArrayNew {
            result,
            length,
        });
        let a = primitive(f, 0);
        if length != 0 {
            set(f, source, "2", a);
        }
        set(f, source, "named", a);
        let rest = value(f, Type::ManagedArray, |result| Op::ManagedArrayRest {
            result,
            array: source,
            start,
        });
        let keys = value(f, Type::ManagedArray, |result| Op::ManagedEnumerateOwn {
            kind: OwnEnumeration::Keys,
            result,
            input: rest,
        });
        let count = value(f, Type::Number, |result| Op::ManagedArrayLength {
            result,
            array: keys,
        });
        f.body.exit = Exit::Return(count);
        execute(
            &m,
            &format!(
                "(()=>{{const source=new Array({length});if({length})source[2]=a;source.named=a;const rest=Array.from(source).slice({start});return Object.keys(rest).length;}})()"
            ),
        );
    }
}

#[test]
fn array_rest_copies_entries_without_aliasing_the_source_slots() {
    let mut m = module();
    let f = &mut m.functions[0];
    let source = value(f, Type::ManagedArray, |result| Op::ManagedArrayNew {
        result,
        length: 1,
    });
    let a = primitive(f, 0);
    let b = primitive(f, 1);
    set(f, source, "0", a);
    let rest = value(f, Type::ManagedArray, |result| Op::ManagedArrayRest {
        result,
        array: source,
        start: 0,
    });
    set(f, rest, "0", b);
    let original = get(f, source, "0");
    let result = equal(f, original, a);
    f.body.exit = Exit::Return(result);
    execute(
        &m,
        "(()=>{const source=[a];const [...rest]=source;rest[0]=b;return source[0]===a?1:0;})()",
    );
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

#[test]
fn array_search_requires_array_boxed_needle_and_number_position() {
    for (method, expected) in [
        ("includes", Type::Predicate),
        ("indexOf", Type::Number),
        ("lastIndexOf", Type::Number),
    ] {
        let source =
            format!("export function run(a,b){{const xs=[a];return xs.{method}(b,a)?1:0;}}");
        let mut module = crate::lower_source(&source, false).unwrap();
        verify_module(&module).unwrap();
        let (position, kind, result, array, needle, from) = module.functions[0]
            .body
            .operations
            .iter()
            .enumerate()
            .find_map(|(position, op)| match op.kind {
                Op::ManagedArraySearch {
                    kind,
                    result,
                    array,
                    needle: Some(needle),
                    from: Some(from),
                } => Some((position, kind, result, array, needle, from)),
                _ => None,
            })
            .unwrap();
        let wrong_result = if expected == Type::Number {
            Type::Predicate
        } else {
            Type::Number
        };
        for (array, needle, from, ty) in [
            (from, needle, from, expected),
            (array, from, from, expected),
            (array, needle, needle, expected),
            (array, needle, from, wrong_result),
        ] {
            let function = &mut module.functions[0];
            function.body.operations[position].kind = Op::ManagedArraySearch {
                kind,
                result,
                array,
                needle: Some(needle),
                from: Some(from),
            };
            function.values[result.0] = ty;
            assert!(verify_module(&module).is_err());
        }
    }
}

#[test]
fn array_slice_checks_its_array_and_number_operands() {
    let source = "export function run(a,b){const xs=[a].slice(a,b);return xs.length;}";
    let mut module = crate::lower_source(source, false).unwrap();
    verify_module(&module).unwrap();
    let (position, result, array, start, end) = module.functions[0]
        .body
        .operations
        .iter()
        .enumerate()
        .find_map(|(i, op)| match op.kind {
            Op::ManagedArraySlice {
                result,
                array,
                start,
                end,
            } => Some((i, result, array, start, end)),
            _ => None,
        })
        .unwrap();
    for (array, start, end, ty) in [
        (start, start, end, Type::ManagedArray),
        (array, array, end, Type::ManagedArray),
        (array, start, array, Type::ManagedArray),
        (array, start, end, Type::ManagedValue),
    ] {
        let f = &mut module.functions[0];
        f.body.operations[position].kind = Op::ManagedArraySlice {
            result,
            array,
            start,
            end,
        };
        f.values[result.0] = ty;
        assert!(verify_module(&module).is_err());
    }
}

#[test]
fn array_slice_keeps_unsupported_receivers_coercions_and_calls_explicit() {
    for body in [
        "return a.slice().length;",
        "const xs=a?[b]:null;return xs.slice().length;",
        "const xs=a?[b]:'text';return xs.slice().length;",
        "let xs=[b];const copy=xs.slice();xs=null;return copy.length;",
        "const xs=[b];return xs.slice('1').length;",
        "const xs=[b];return xs.slice(0,{}).length;",
        "const xs=[b],args=[0];return xs.slice(...args).length;",
        "const xs=[b];return xs?.slice().length;",
        "const xs=[b];return xs.slice?.().length;",
        "const xs=[b];xs.slice=()=>[];return xs.slice().length;",
        "const xs=[b];return xs.slice.call(xs).length;",
        "const xs=['s'].slice();return xs[0].trim().length;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let err = crate::lower_source(&source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(
                err.contains("unsupported") && !err.contains("internal IR"),
                "{source}: {err}"
            );
        }
    }
}

#[test]
fn array_reverse_requires_a_proven_array_and_invalidates_position_facts() {
    for body in [
        "return a.reverse().length;",
        "const xs=a?[b]:null;return xs.reverse().length;",
        "const xs=[b];return xs.reverse?.().length;",
        "const xs=[b];return xs?.reverse().length;",
        "const xs=[b];return xs.reverse(...[]).length;",
        "const xs=[b];return xs.reverse.call(xs).length;",
        "const xs=[b];xs.reverse=()=>[];return xs.reverse().length;",
        "const xs=['x',b];xs.reverse();return xs[0].trim().length;",
        "const xs=['x',b];const alias=xs;alias.reverse();return xs[0].trim().length;",
        "const xs=['x'];const alias=xs.reverse();alias[0]=b;return xs[0].trim().length;",
        "const xs=[()=>a,b];xs.reverse();return xs[0]();",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let err = crate::lower_source(&source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(
                err.contains("unsupported") && !err.contains("internal IR"),
                "{source}: {err}"
            );
        }
    }
    let mut module = crate::lower_source(
        "export function run(a,b){return [a,b].reverse().length;}",
        false,
    )
    .unwrap();
    verify_module(&module).unwrap();
    let (position, result, array) = module.functions[0]
        .body
        .operations
        .iter()
        .enumerate()
        .find_map(|(i, o)| match o.kind {
            Op::ManagedArrayReverse { result, array } => Some((i, result, array)),
            _ => None,
        })
        .unwrap();
    module.functions[0].values[result.0] = Type::ManagedValue;
    assert!(verify_module(&module).is_err());
    module.functions[0].values[result.0] = Type::ManagedArray;
    module.functions[0].body.operations[position].kind = Op::ManagedArrayReverse {
        result,
        array: result,
    };
    assert!(verify_module(&module).is_err());
    module.functions[0].body.operations[position].kind = Op::ManagedArrayReverse { result, array };
    verify_module(&module).unwrap();
}

#[test]
fn array_shift_keeps_unknown_receivers_and_invalidated_positions_explicit() {
    for body in [
        "return a.shift();",
        "const xs=a?[b]:null;return xs.shift()===b?1:0;",
        "const xs=[b];return xs.shift?.()===b?1:0;",
        "const xs=[b];return xs.shift(...[])===b?1:0;",
        "const xs=[b];return xs.shift.call(xs)===b?1:0;",
        "const xs=[b];xs.shift=()=>b;return xs.shift()===b?1:0;",
        "const xs=['x',b];xs.shift();return xs[0].trim().length;",
        "const xs=['x',b];const alias=xs;alias.shift();return xs[0].trim().length;",
        "const xs=[()=>a,b];xs.shift();return xs[0]();",
        "const xs=['x'];return xs.shift().trim().length;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typed in [false, true] {
            let err = crate::lower_source(&source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(
                err.contains("unsupported") && !err.contains("internal IR"),
                "{source}: {err}"
            );
        }
    }
    let mut module = crate::lower_source(
        "export function run(a,b){return [a,b].shift()===a?1:0;}",
        false,
    )
    .unwrap();
    verify_module(&module).unwrap();
    let (position, result, array) = module.functions[0]
        .body
        .operations
        .iter()
        .enumerate()
        .find_map(|(i, o)| match o.kind {
            Op::ManagedArrayShift { result, array } => Some((i, result, array)),
            _ => None,
        })
        .unwrap();
    module.functions[0].values[result.0] = Type::ManagedArray;
    assert!(verify_module(&module).is_err());
    module.functions[0].values[result.0] = Type::ManagedValue;
    module.functions[0].body.operations[position].kind = Op::ManagedArrayShift {
        result,
        array: result,
    };
    assert!(verify_module(&module).is_err());
    module.functions[0].body.operations[position].kind = Op::ManagedArrayShift { result, array };
    verify_module(&module).unwrap();
}

#[test]
fn array_at_defaults_admit_typed_consumers_on_small_stacks() {
    for source in [
        "export function run(a,b){const {text=' ? '}={text:[' first ',' second '].at(a)};return text.trim().length;}",
        "function pick(xs,i){return xs.at(i);}export function run(a,b){const rows=[{label:' first '},{label:' second '}];const {item={label:' missing '}}={item:pick(rows,a)};return item.label.trim().length;}",
        "export function run(a,b){const {item=[]}={item:[[a],[a,b]].at(a)};return item.length;}",
        "export function run(a,b){const xs=[' old '];const alias=xs;alias[0]=' new ';const {text=' ? '}={text:xs.at(a)};return text.trim().length;}",
        "export function run(a,b){const xs=[' first ',' second '];const [...copy]=xs.slice();const {text=' ? '}={text:copy.at(a)};return text.trim().length;}",
    ] {
        for typescript in [false, true] {
            let result = std::thread::Builder::new()
                .stack_size(2 * 1024 * 1024)
                .spawn(move || crate::compile(source, typescript))
                .unwrap()
                .join()
                .unwrap();
            assert!(result.is_ok(), "{source}: {:?}", result.err());
        }
    }
}

#[test]
fn array_at_defaults_do_not_establish_presence_or_erase_other_kinds() {
    for body in [
        "return ['text'].at(0).trim().length;",
        "return ['text'].at(a).trim().length;",
        "const {text='?'}={text:['text',b].at(a)};return text.trim().length;",
        "const xs=['text'];const alias=xs;alias[0]=null;const {text='?'}={text:xs.at(a)};return text.trim().length;",
        "const xs=[{label:'text'}];xs[0].label=b;const {item={label:'?'}}={item:xs.at(a)};return item.label.trim().length;",
        "const {text='?'}={text:[,'text'].at(a)};return text.trim().length;",
        "const xs=['text'];xs.length=2;const {text='?'}={text:xs.at(a)};return text.trim().length;",
        "const {text=null}={text:['text'].at(a)};return text.trim().length;",
        "const xs=['text'];delete xs[0];const {text='?'}={text:xs.at(a)};return text.trim().length;",
        "const {f=()=>b}={f:[()=>a].at(a)};return f();",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typescript in [false, true] {
            let error = crate::compile(&source, typescript).err().unwrap();
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

#[test]
fn runtime_resize_requires_a_visible_number_and_invalidates_elements() {
    let src = "export function run(a,b){const xs=[a];xs.length=b>>>0;return xs.length;}";
    let mut module = crate::lower_source(src, false).unwrap();
    verify_module(&module).unwrap();
    let op = module.functions[0]
        .body
        .operations
        .iter_mut()
        .find_map(|op| match &mut op.kind {
            Op::ManagedArrayResizeDynamic { array, length } => Some((*array, length)),
            _ => None,
        })
        .unwrap();
    *op.1 = op.0;
    assert!(verify_module(&module).is_err());
    let src = "export function run(a,b){const xs=['safe'];const alias=xs;alias.length=a>>>0;return xs[0].trim().length;}";
    assert!(crate::compile(src, false).is_err());
}
