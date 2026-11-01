//! Typed managed arguments; source inference and managed results are separate.
use super::*;

fn effect(f: &mut Function, kind: Op) {
    f.body.operations.push(Operation {
        span: f.body.span,
        kind,
    });
}
fn value(f: &mut Function, ty: Type, kind: impl FnOnce(ValueId) -> Op) -> ValueId {
    let result = ValueId(f.values.len());
    f.values.push(ty);
    effect(f, kind(result));
    result
}
fn function(parameters: &[Type]) -> Function {
    let span = Span::new(0, 0);
    Function {
        return_type: Type::Number,
        parameter_count: parameters.len(),
        slots: parameters
            .iter()
            .map(|ty| Slot {
                ty: *ty,
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
    }
}
fn managed(f: &mut Function, n: ValueId) -> ValueId {
    let packed = value(f, Type::General, |result| Op::Pack { result, input: n });
    value(f, Type::ManagedValue, |result| Op::ManagedPrimitive {
        result,
        input: packed,
    })
}
fn load(f: &mut Function, index: usize) -> ValueId {
    value(f, f.slots[index].ty, |result| Op::Load {
        result,
        slot: SlotId(index),
    })
}
fn object_alias() -> Module {
    let mut m = crate::lower_source("export function run(a,b){return a;}", false).unwrap();
    let signature = m
        .closure_signatures
        .intern(vec![Type::ManagedObject, Type::Number], Type::Number);
    let f = &mut m.functions[0];
    f.values.clear();
    f.body.operations.clear();
    let a = load(f, 0);
    let a = managed(f, a);
    let object = value(f, Type::ManagedObject, |result| Op::ManagedObjectNew {
        result,
    });
    effect(
        f,
        Op::ManagedSetOwn {
            object,
            key: vec![120],
            value: a,
        },
    );
    let old = value(f, Type::ManagedValue, |result| Op::ManagedGetOwn {
        result,
        object,
        key: vec![120],
    });
    let callee = value(f, Type::Closure(signature), |result| Op::MakeClosure {
        result,
        function: FunctionId(1),
        captures: vec![],
    });
    let b = load(f, 1);
    value(f, Type::Number, |result| Op::ClosureCall {
        result,
        callee,
        arguments: vec![object, b],
    });
    let new = value(f, Type::ManagedValue, |result| Op::ManagedGetOwn {
        result,
        object,
        key: vec![120],
    });
    let b = managed(f, b);
    let old_ok = value(f, Type::Predicate, |result| Op::ManagedEqual {
        result,
        left: old,
        right: a,
    });
    let new_ok = value(f, Type::Predicate, |result| Op::ManagedEqual {
        result,
        left: new,
        right: b,
    });
    let old_ok = value(f, Type::Number, |result| Op::NumberFromPredicate {
        result,
        input: old_ok,
    });
    let new_ok = value(f, Type::Number, |result| Op::NumberFromPredicate {
        result,
        input: new_ok,
    });
    let result = value(f, Type::Number, |result| Op::Binary {
        result,
        operator: Binary::Add,
        left: old_ok,
        right: new_ok,
    });
    f.body.exit = Exit::Return(result);
    let mut setter = function(&[Type::ManagedObject, Type::Number]);
    effect(&mut setter, Op::ClosureEntry { captures: vec![] });
    let object = load(&mut setter, 0);
    let n = load(&mut setter, 1);
    let n = managed(&mut setter, n);
    effect(
        &mut setter,
        Op::ManagedSetOwn {
            object,
            key: vec![120],
            value: n,
        },
    );
    let result = value(&mut setter, Type::Number, |result| Op::Constant {
        result,
        value: 0.0,
    });
    setter.body.exit = Exit::Return(result);
    m.functions.push(setter);
    m
}
fn check(m: &Module, native: &str) {
    verify_module(m).unwrap();
    let wat = crate::emit::emit(m).unwrap();
    assert!(wat.contains("$gc_frame"));
    assert!(!wat.contains("$cell_top"));
    super::managed_value_tests::node(&wat::parse_str(wat).unwrap(), native);
}
#[test]
fn closure_object_arguments_preserve_alias_mutation_and_prior_snapshots() {
    check(
        &object_alias(),
        "(()=>{const o={x:a};const old=o.x;const set=(o,n)=>{o.x=n;return 0;};set(o,b);return Number(old===a)+Number(o.x===b);})()",
    );
}

#[test]
fn closure_array_arguments_keep_their_reference_kind() {
    let mut m = object_alias();
    m.closure_signatures.higher[0].parameters[0] = Type::ManagedArray;
    for f in &mut m.functions {
        for slot in &mut f.slots {
            if slot.ty == Type::ManagedObject {
                slot.ty = Type::ManagedArray;
            }
        }
        for ty in &mut f.values {
            if *ty == Type::ManagedObject {
                *ty = Type::ManagedArray;
            }
        }
        for op in &mut f.body.operations {
            op.kind = match &op.kind {
                Op::ManagedObjectNew { result } => Op::ManagedArrayNew {
                    result: *result,
                    length: 2,
                },
                Op::ManagedSetOwn { object, key, value } => Op::ManagedArraySetOwn {
                    array: *object,
                    key: key.clone(),
                    value: *value,
                },
                Op::ManagedGetOwn {
                    result,
                    object,
                    key,
                } => Op::ManagedArrayGetOwn {
                    result: *result,
                    array: *object,
                    key: key.clone(),
                },
                _ => continue,
            };
        }
    }
    check(
        &m,
        "(()=>{const o=[];o.x=a;const old=o.x;const set=(o,n)=>{o.x=n;return 0;};set(o,b);return Number(old===a)+Number(o.x===b);})()",
    );
}
#[test]
fn closure_value_arguments_keep_lossless_strings_and_distinct_argument_entries() {
    for same in [false, true] {
        let mut m = crate::lower_source("export function run(a,b){return a;}", false).unwrap();
        let signature = m
            .closure_signatures
            .intern(vec![Type::ManagedValue, Type::ManagedValue], Type::Number);
        let f = &mut m.functions[0];
        f.values.clear();
        f.body.operations.clear();
        let left = value(f, Type::ManagedValue, |result| Op::ManagedString {
            result,
            units: vec![0, 0xd800],
        });
        let right = value(f, Type::ManagedValue, |result| Op::ManagedString {
            result,
            units: vec![0, if same { 0xd800 } else { 0xfffd }],
        });
        let callee = value(f, Type::Closure(signature), |result| Op::MakeClosure {
            result,
            function: FunctionId(1),
            captures: vec![],
        });
        let result = value(f, Type::Number, |result| Op::ClosureCall {
            result,
            callee,
            arguments: vec![left, right],
        });
        f.body.exit = Exit::Return(result);
        let mut equal = function(&[Type::ManagedValue, Type::ManagedValue]);
        effect(&mut equal, Op::ClosureEntry { captures: vec![] });
        let left = load(&mut equal, 0);
        let right = load(&mut equal, 1);
        let equal_value = value(&mut equal, Type::Predicate, |result| Op::ManagedEqual {
            result,
            left,
            right,
        });
        let result = value(&mut equal, Type::Number, |result| Op::NumberFromPredicate {
            result,
            input: equal_value,
        });
        equal.body.exit = Exit::Return(result);
        m.functions.push(equal);
        check(
            &m,
            if same {
                r"Number('\0\ud800'==='\0\ud800')"
            } else {
                r"Number('\0\ud800'==='\0\ufffd')"
            },
        );
    }
}
#[test]
fn private_managed_parameters_can_be_captured_by_nested_closures() {
    let mut m = object_alias();
    let mut inner = std::mem::replace(
        &mut m.functions[1],
        function(&[Type::ManagedObject, Type::Number]),
    );
    inner.parameter_count = 0;
    for slot in &mut inner.slots {
        slot.parameter = false;
    }
    inner.body.operations[0].kind = Op::ClosureEntry {
        captures: vec![SlotId(0), SlotId(1)],
    };
    let outer = &mut m.functions[1];
    effect(outer, Op::ClosureEntry { captures: vec![] });
    let callee = value(outer, Type::Closure(0), |result| Op::MakeClosure {
        result,
        function: FunctionId(2),
        captures: vec![SlotId(0), SlotId(1)],
    });
    let result = value(outer, Type::Number, |result| Op::ClosureCall {
        result,
        callee,
        arguments: vec![],
    });
    outer.body.exit = Exit::Return(result);
    m.functions.push(inner);
    check(
        &m,
        "(()=>{const o={x:a};const old=o.x;const set=(o,n)=>{const f=()=>{o.x=n;return 0;};return f();};set(o,b);return Number(old===a)+Number(o.x===b);})()",
    );
}
#[test]
fn managed_argument_signatures_work_as_callback_parameters() {
    let mut m = object_alias();
    let signature = m.closure_signatures.intern(
        vec![Type::Closure(1001), Type::ManagedObject, Type::Number],
        Type::Number,
    );
    let mut wrapper = function(&[Type::Closure(1001), Type::ManagedObject, Type::Number]);
    effect(&mut wrapper, Op::ClosureEntry { captures: vec![] });
    let callee = load(&mut wrapper, 0);
    let object = load(&mut wrapper, 1);
    let number = load(&mut wrapper, 2);
    let result = value(&mut wrapper, Type::Number, |result| Op::ClosureCall {
        result,
        callee,
        arguments: vec![object, number],
    });
    wrapper.body.exit = Exit::Return(result);
    m.functions.push(wrapper);
    let f = &mut m.functions[0];
    let index = f
        .body
        .operations
        .iter()
        .position(|op| matches!(op.kind, Op::ClosureCall { .. }))
        .unwrap();
    let new = ValueId(f.values.len());
    f.values.push(Type::Closure(signature));
    let Op::ClosureCall {
        callee, arguments, ..
    } = &mut f.body.operations[index].kind
    else {
        unreachable!()
    };
    arguments.insert(0, *callee);
    *callee = new;
    f.body.operations.insert(
        index,
        Operation {
            span: f.body.span,
            kind: Op::MakeClosure {
                result: new,
                function: FunctionId(2),
                captures: vec![],
            },
        },
    );
    check(
        &m,
        "(()=>{const o={x:a};const old=o.x;const set=(o,n)=>{o.x=n;return 0;};const apply=(fn,o,n)=>fn(o,n);apply(set,o,b);return Number(old===a)+Number(o.x===b);})()",
    );
}
#[test]
fn returned_functions_retain_managed_parameter_signatures() {
    let mut m = object_alias();
    let signature = m.closure_signatures.intern(vec![], Type::Closure(1001));
    let mut factory = function(&[]);
    factory.return_type = Type::Closure(1001);
    effect(&mut factory, Op::ClosureEntry { captures: vec![] });
    let callee = value(&mut factory, Type::Closure(1001), |result| {
        Op::MakeClosure {
            result,
            function: FunctionId(1),
            captures: vec![],
        }
    });
    factory.body.exit = Exit::Return(callee);
    m.functions.push(factory);
    let f = &mut m.functions[0];
    let index = f
        .body
        .operations
        .iter()
        .position(|op| matches!(op.kind, Op::MakeClosure { .. }))
        .unwrap();
    let Op::MakeClosure { result, .. } = f.body.operations[index].kind else {
        unreachable!()
    };
    let new = ValueId(f.values.len());
    f.values.push(Type::Closure(signature));
    f.body.operations[index].kind = Op::ClosureCall {
        result,
        callee: new,
        arguments: vec![],
    };
    f.body.operations.insert(
        index,
        Operation {
            span: f.body.span,
            kind: Op::MakeClosure {
                result: new,
                function: FunctionId(2),
                captures: vec![],
            },
        },
    );
    check(
        &m,
        "(()=>{const o={x:a};const old=o.x;const make=()=>(o,n)=>{o.x=n;return 0;};make()(o,b);return Number(old===a)+Number(o.x===b);})()",
    );
}

#[test]
fn managed_parameters_keep_exact_types_counts_and_async_result_boundaries() {
    for change in 0..6 {
        let mut m = object_alias();
        match change {
            0 => {
                let f = &mut m.functions[0];
                let Op::ClosureCall { arguments, .. } = &mut f
                    .body
                    .operations
                    .iter_mut()
                    .find(|op| matches!(op.kind, Op::ClosureCall { .. }))
                    .unwrap()
                    .kind
                else {
                    unreachable!()
                };
                arguments[0] = arguments[1];
            }
            1 => {
                let f = &mut m.functions[0];
                let Op::ClosureCall { arguments, .. } = &mut f
                    .body
                    .operations
                    .iter_mut()
                    .find(|op| matches!(op.kind, Op::ClosureCall { .. }))
                    .unwrap()
                    .kind
                else {
                    unreachable!()
                };
                arguments.pop();
            }
            2 => m.functions[1].slots[0].ty = Type::ManagedArray,
            3 => m.closure_signatures.higher[0].parameters[0] = Type::Predicate,
            4 => {
                m.closure_signatures.higher[0].asynchronous = true;
                m.closure_signatures.higher[0].result = Type::General;
                // The input signature is now valid, but the unchanged target
                // is synchronous; module verification must still reject it.
                assert!(m.closure_signatures.is_callback_signature(1001));
                assert!(m.closure_signatures.is_returned_function(1001));
            }
            _ => m.closure_signatures.higher[0].result = Type::ManagedObject,
        }
        assert!(verify_module(&m).is_err(), "change {change}");
    }
    let mut m = object_alias();
    m.functions[0].return_type = Type::General;
    let f = &mut m.functions[0];
    let input = match f.body.exit {
        Exit::Return(v) => v,
        _ => unreachable!(),
    };
    let result = value(f, Type::General, |result| Op::Pack { result, input });
    f.body.exit = Exit::Return(result);
    wat::parse_str(crate::emit::emit_async(&m).unwrap()).unwrap();
    let promoted = crate::compile(
        "export function run(a,b){const o={x:a};const f=x=>x.x===a?a:b;return f(o);}",
        false,
    )
    .unwrap();
    super::managed_value_tests::node(&promoted.wasm, "a===a?a:b");
    let source = "export function run(a,b){const f=x=>x;return f('text')==='text'?a:b;}";
    let promoted = crate::compile(source, false).unwrap();
    super::managed_value_tests::node(
        &promoted.wasm,
        "(()=>{const f=x=>x;return f('text')==='text'?a:b;})()",
    );
}
#[test]
fn managed_parameter_frames_keep_the_thousand_argument_boundary() {
    for same in [false, true] {
        let mut m = crate::lower_source("export function run(a,b){return a;}", false).unwrap();
        let types = vec![Type::ManagedObject; 1000];
        let signature = m.closure_signatures.intern(types.clone(), Type::Number);
        let f = &mut m.functions[0];
        f.values.clear();
        f.body.operations.clear();
        let first = value(f, Type::ManagedObject, |result| Op::ManagedObjectNew {
            result,
        });
        let last = if same {
            first
        } else {
            value(f, Type::ManagedObject, |result| Op::ManagedObjectNew {
                result,
            })
        };
        let callee = value(f, Type::Closure(signature), |result| Op::MakeClosure {
            result,
            function: FunctionId(1),
            captures: vec![],
        });
        let mut arguments = vec![first; 1000];
        arguments[999] = last;
        let result = value(f, Type::Number, |result| Op::ClosureCall {
            result,
            callee,
            arguments,
        });
        f.body.exit = Exit::Return(result);
        let mut equal = function(&types);
        effect(&mut equal, Op::ClosureEntry { captures: vec![] });
        let first = load(&mut equal, 0);
        let last = load(&mut equal, 999);
        let first = value(&mut equal, Type::ManagedValue, |result| {
            Op::ManagedFromObject {
                result,
                input: first,
            }
        });
        let last = value(&mut equal, Type::ManagedValue, |result| {
            Op::ManagedFromObject {
                result,
                input: last,
            }
        });
        let equal_value = value(&mut equal, Type::Predicate, |result| Op::ManagedEqual {
            result,
            left: first,
            right: last,
        });
        let result = value(&mut equal, Type::Number, |result| Op::NumberFromPredicate {
            result,
            input: equal_value,
        });
        equal.body.exit = Exit::Return(result);
        m.functions.push(equal);
        check(&m, if same { "1" } else { "0" });
        m.closure_signatures.higher[0]
            .parameters
            .push(Type::ManagedObject);
        assert!(
            verify_module(&m)
                .unwrap_err()
                .contains("invalid closure signature")
        );
    }
}

#[test]
fn managed_value_parameters_preserve_number_kinds_payloads_and_nan_equality() {
    let mut m = crate::lower_source("export function run(a,b){return a;}", false).unwrap();
    let signature = m
        .closure_signatures
        .intern(vec![Type::ManagedValue, Type::Number], Type::Number);
    let f = &mut m.functions[0];
    f.values.clear();
    f.body.operations.clear();
    let a = load(f, 0);
    let boxed = managed(f, a);
    let b = load(f, 1);
    let callee = value(f, Type::Closure(signature), |result| Op::MakeClosure {
        result,
        function: FunctionId(1),
        captures: vec![],
    });
    let result = value(f, Type::Number, |result| Op::ClosureCall {
        result,
        callee,
        arguments: vec![boxed, b],
    });
    f.body.exit = Exit::Return(result);
    let mut equal = function(&[Type::ManagedValue, Type::Number]);
    effect(&mut equal, Op::ClosureEntry { captures: vec![] });
    let left = load(&mut equal, 0);
    let right = load(&mut equal, 1);
    let right = managed(&mut equal, right);
    let same = value(&mut equal, Type::Predicate, |result| Op::ManagedEqual {
        result,
        left,
        right,
    });
    let result = value(&mut equal, Type::Number, |result| Op::NumberFromPredicate {
        result,
        input: same,
    });
    equal.body.exit = Exit::Return(result);
    m.functions.push(equal);
    check(&m, "Number(a===b)");
}

#[test]
fn returned_closures_retain_their_managed_parameter_cells() {
    let mut m = object_alias();
    let mut inner = std::mem::replace(
        &mut m.functions[1],
        function(&[Type::ManagedObject, Type::Number]),
    );
    inner.parameter_count = 0;
    for slot in &mut inner.slots {
        slot.parameter = false;
    }
    inner.body.operations[0].kind = Op::ClosureEntry {
        captures: vec![SlotId(0), SlotId(1)],
    };
    let factory = &mut m.functions[1];
    factory.return_type = Type::Closure(0);
    effect(factory, Op::ClosureEntry { captures: vec![] });
    let callee = value(factory, Type::Closure(0), |result| Op::MakeClosure {
        result,
        function: FunctionId(2),
        captures: vec![SlotId(0), SlotId(1)],
    });
    factory.body.exit = Exit::Return(callee);
    m.functions.push(inner);
    m.closure_signatures.higher[0].result = Type::Closure(0);
    let f = &mut m.functions[0];
    let index = f
        .body
        .operations
        .iter()
        .position(|op| matches!(op.kind, Op::ClosureCall { .. }))
        .unwrap();
    let Op::ClosureCall { result, .. } = &mut f.body.operations[index].kind else {
        unreachable!()
    };
    let previous = *result;
    let returned = ValueId(f.values.len());
    f.values.push(Type::Closure(0));
    *result = returned;
    f.body.operations.insert(
        index + 1,
        Operation {
            span: f.body.span,
            kind: Op::ClosureCall {
                result: previous,
                callee: returned,
                arguments: vec![],
            },
        },
    );
    check(
        &m,
        "(()=>{const o={x:a};const old=o.x;const make=(o,n)=>()=>{o.x=n;return 0;};const set=make(o,b);set();return Number(old===a)+Number(o.x===b);})()",
    );
}
