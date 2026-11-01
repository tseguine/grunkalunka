//! Canonical synchronous function identities inside the managed value domain.
use super::managed_value_tests::{effect, value};
use super::*;

fn fixture(capture: bool) -> (Module, ValueId) {
    let mut module = crate::lower_source("export function run(a,b){return a;}", false).unwrap();
    let root = &mut module.functions[0];
    root.values.clear();
    root.body.operations.clear();
    let callee = value(root, Type::Closure(0), |result| Op::MakeClosure {
        result,
        function: FunctionId(1),
        captures: if capture { vec![SlotId(0)] } else { vec![] },
    });
    let span = root.body.span;
    let mut child = Function {
        return_type: Type::Number,
        parameter_count: 0,
        slots: if capture {
            vec![Slot {
                ty: Type::Number,
                owner: RegionId(0),
                mutable: true,
                parameter: false,
                span,
            }]
        } else {
            vec![]
        },
        values: vec![],
        body: Region {
            id: RegionId(0),
            span,
            operations: vec![],
            exit: Exit::Fallthrough,
        },
    };
    effect(
        &mut child,
        Op::ClosureEntry {
            captures: if capture { vec![SlotId(0)] } else { vec![] },
        },
    );
    let returned = value(&mut child, Type::Number, |result| {
        if capture {
            Op::Load {
                result,
                slot: SlotId(0),
            }
        } else {
            Op::Constant { result, value: 0.0 }
        }
    });
    child.body.exit = Exit::Return(returned);
    module.functions.push(child);
    (module, callee)
}
fn boxed(f: &mut Function, input: ValueId) -> ValueId {
    value(f, Type::ManagedValue, |result| Op::ManagedFromClosure {
        result,
        input,
    })
}
fn equality(f: &mut Function, left: ValueId, right: ValueId) {
    let same = value(f, Type::Predicate, |result| Op::ManagedEqual {
        result,
        left,
        right,
    });
    let number = value(f, Type::Number, |result| Op::NumberFromPredicate {
        result,
        input: same,
    });
    f.body.exit = Exit::Return(number);
}
fn check(module: &Module, native: &str) {
    verify_module(module).unwrap();
    let wat = crate::emit::emit(module).unwrap();
    assert!(wat.contains("$gc_function"));
    assert!(!wat.contains("$cell_top"));
    super::managed_value_tests::node(&wat::parse_str(wat).unwrap(), native);
}

#[test]
fn reboxing_keeps_the_same_function_identity() {
    for capture in [false, true] {
        let (mut module, callee) = fixture(capture);
        let f = &mut module.functions[0];
        let first = boxed(f, callee);
        let again = boxed(f, callee);
        equality(f, first, again);
        check(&module, "(()=>{const f=()=>a;return Number(f===f);})()");
    }
}

#[test]
fn separate_creations_of_the_same_code_have_distinct_identity() {
    for capture in [false, true] {
        let (mut module, first) = fixture(capture);
        let f = &mut module.functions[0];
        let second = value(f, Type::Closure(0), |result| Op::MakeClosure {
            result,
            function: FunctionId(1),
            captures: if capture { vec![SlotId(0)] } else { vec![] },
        });
        let first = boxed(f, first);
        let second = boxed(f, second);
        equality(f, first, second);
        check(
            &module,
            "(()=>{const make=()=>()=>a;return Number(make()===make());})()",
        );
    }
}

#[test]
fn stored_function_values_keep_identity_and_old_property_snapshots() {
    for array in [false, true] {
        let (mut module, callee) = fixture(true);
        let f = &mut module.functions[0];
        let first = boxed(f, callee);
        let object = value(
            f,
            if array {
                Type::ManagedArray
            } else {
                Type::ManagedObject
            },
            |result| {
                if array {
                    Op::ManagedArrayNew { result, length: 0 }
                } else {
                    Op::ManagedObjectNew { result }
                }
            },
        );
        let store = |value| {
            if array {
                Op::ManagedArraySetOwn {
                    array: object,
                    key: vec![48],
                    value,
                }
            } else {
                Op::ManagedSetOwn {
                    object,
                    key: vec![102],
                    value,
                }
            }
        };
        effect(f, store(first));
        let saved = value(f, Type::ManagedValue, |result| {
            if array {
                Op::ManagedArrayGetOwn {
                    result,
                    array: object,
                    key: vec![48],
                }
            } else {
                Op::ManagedGetOwn {
                    result,
                    object,
                    key: vec![102],
                }
            }
        });
        let fresh = value(f, Type::Closure(0), |result| Op::MakeClosure {
            result,
            function: FunctionId(1),
            captures: vec![SlotId(0)],
        });
        let fresh = boxed(f, fresh);
        effect(f, store(fresh));
        let again = boxed(f, callee);
        equality(f, saved, again);
        check(
            &module,
            if array {
                "(()=>{const make=()=>()=>a;const f=make(),o=[f],old=o[0];o[0]=make();return Number(old===f);})()"
            } else {
                "(()=>{const make=()=>()=>a;const f=make(),o={f},old=o.f;o.f=make();return Number(old===f);})()"
            },
        );
    }
}

#[test]
fn functions_have_their_own_type_and_are_truthy_and_nonnullish() {
    for kind in 0..3 {
        let (mut module, callee) = fixture(false);
        let f = &mut module.functions[0];
        let input = boxed(f, callee);
        if kind == 0 {
            let result = value(f, Type::ManagedValue, |result| Op::ManagedTypeof {
                result,
                input,
            });
            let expected = value(f, Type::ManagedValue, |result| Op::ManagedString {
                result,
                units: "function".encode_utf16().collect(),
            });
            equality(f, result, expected);
        } else {
            let predicate = value(f, Type::Predicate, |result| {
                if kind == 1 {
                    Op::ManagedTruthy { result, input }
                } else {
                    Op::ManagedIsNullish { result, input }
                }
            });
            let result = value(f, Type::Number, |result| Op::NumberFromPredicate {
                result,
                input: predicate,
            });
            f.body.exit = Exit::Return(result);
        }
        check(
            &module,
            match kind {
                0 => "(()=>{const f=()=>a;return Number(typeof f==='function');})()",
                1 => "(()=>{const f=()=>a;return Number(Boolean(f));})()",
                _ => "(()=>{const f=()=>a;return Number(f==null);})()",
            },
        );
    }
}

#[test]
fn boxing_keeps_the_original_typed_function_callable() {
    let (mut module, callee) = fixture(true);
    let f = &mut module.functions[0];
    boxed(f, callee);
    let result = value(f, Type::Number, |result| Op::ClosureCall {
        result,
        callee,
        arguments: vec![],
    });
    f.body.exit = Exit::Return(result);
    check(&module, "(()=>{const f=()=>a;const o={f};return f();})()");
}

#[test]
fn boxing_requires_a_visible_known_synchronous_function_and_managed_result() {
    let (mut module, _) = fixture(false);
    let f = &mut module.functions[0];
    let number = value(f, Type::Number, |result| Op::Load {
        result,
        slot: SlotId(0),
    });
    let bad = boxed(f, number);
    equality(f, bad, bad);
    assert!(
        verify_module(&module)
            .unwrap_err()
            .contains("requires a function value")
    );

    let (mut module, callee) = fixture(false);
    let f = &mut module.functions[0];
    let bad = boxed(f, callee);
    equality(f, bad, bad);
    f.values[bad.0] = Type::ManagedObject;
    assert!(
        verify_module(&module)
            .unwrap_err()
            .contains("invalid result value ID or type")
    );

    let (mut module, callee) = fixture(false);
    let f = &mut module.functions[0];
    let bad = boxed(f, callee);
    let later = value(f, Type::Closure(0), |result| Op::MakeClosure {
        result,
        function: FunctionId(1),
        captures: vec![],
    });
    if let Op::ManagedFromClosure { input, .. } = &mut f.body.operations[1].kind {
        *input = later;
    } else {
        panic!("fixture box");
    }
    equality(f, bad, bad);
    assert!(verify_module(&module).unwrap_err().contains("not visible"));

    let (mut module, callee) = fixture(false);
    let signature = module.closure_signatures.intern_async(vec![]);
    module.functions[0].values[callee.0] = Type::Closure(signature);
    let child = &mut module.functions[1];
    child.return_type = Type::General;
    child.body.operations[0].kind = Op::AsyncClosureEntry { captures: vec![] };
    let input = match child.body.exit {
        Exit::Return(v) => v,
        _ => unreachable!(),
    };
    let result = value(child, Type::General, |result| Op::Pack { result, input });
    child.body.exit = Exit::Return(result);
    let f = &mut module.functions[0];
    let bad = boxed(f, callee);
    equality(f, bad, bad);
    assert!(
        verify_module(&module)
            .unwrap_err()
            .contains("known synchronous signature")
    );
}

#[test]
fn backend_bridge_does_not_silently_admit_source_function_containers() {
    for source in [
        "export function run(a,b){const f=()=>a;const o={f};return a;}",
        "export function run(a,b){const o=[()=>a];return a;}",
    ] {
        for ts in [false, true] {
            let error = crate::compile(source, ts).err().unwrap();
            assert!(
                error.contains("unsupported") && !error.contains("internal IR"),
                "{error}"
            );
        }
    }
}
