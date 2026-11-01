//! Module-lifetime identities use the existing private typed closure ABI.
use super::managed_value_tests::value;
use super::*;

fn fixture() -> Module {
    fixture_source(
        "function f(x){return x;}function g(x){return x;}export function run(a,b){return a;}",
    )
}

fn fixture_source(source: &str) -> Module {
    let mut module = crate::lower_source(source, false).unwrap();
    for function in module.functions.iter_mut().skip(1) {
        function.body.operations.insert(
            0,
            Operation {
                span: function.body.span,
                kind: Op::ClosureEntry { captures: vec![] },
            },
        );
    }
    module.functions[0].values.clear();
    module.functions[0].body.operations.clear();
    module
}

fn read(function: &mut Function, target: usize) -> ValueId {
    value(function, Type::Closure(1), |result| Op::ModuleClosure {
        result,
        function: FunctionId(target),
    })
}

fn equal(function: &mut Function, left: ValueId, right: ValueId) {
    let left = value(function, Type::ManagedValue, |result| {
        Op::ManagedFromClosure {
            result,
            input: left,
        }
    });
    let right = value(function, Type::ManagedValue, |result| {
        Op::ManagedFromClosure {
            result,
            input: right,
        }
    });
    let predicate = value(function, Type::Predicate, |result| Op::ManagedEqual {
        result,
        left,
        right,
    });
    let result = value(function, Type::Number, |result| Op::NumberFromPredicate {
        result,
        input: predicate,
    });
    function.body.exit = Exit::Return(result);
}

fn check(module: &Module, native: &str) {
    verify_module(module).unwrap();
    let wat = crate::emit::emit(module).unwrap();
    super::managed_value_tests::node(&wat::parse_str(wat).unwrap(), native);
}

#[test]
fn repeated_reads_of_a_module_function_keep_identity() {
    let mut module = fixture();
    let root = &mut module.functions[0];
    let first = read(root, 1);
    let again = read(root, 1);
    equal(root, first, again);
    check(
        &module,
        "(()=>{function f(x){return x;}return Number(f===f);})()",
    );
}

#[test]
fn emitted_module_identity_survives_distinct_call_activations() {
    let mut module = fixture();
    let root = &mut module.functions[0];
    let first = read(root, 1);
    let again = read(root, 1);
    equal(root, first, again);
    let wat = crate::emit::emit(&module).unwrap();
    assert_eq!(wat.matches("(export \"run\")").count(), 1);
    let mut observed = wat.replacen("(export \"run\")", "(export \"original_run\")", 1);
    observed.truncate(observed.rfind(')').unwrap());
    // Retain the prior read across calls without replacing generated bodies.
    // The original function stays at index zero; only its export is renamed.
    observed.push_str(
        r#"
      (global $test_saved (mut (ref null $gc_function)) (ref.null $gc_function))
      (func (export "run") (param f64 f64) (result f64)
        local.get 0 local.get 1 call 0 drop
        global.get $test_saved ref.is_null
        if (result f64) f64.const 1
        else global.get $test_saved global.get $gc_module_function1
          ref.eq f64.convert_i32_u end
        global.get $gc_module_function1 global.set $test_saved))"#,
    );
    super::managed_value_tests::node(&wat::parse_str(observed).unwrap(), "1");
}

#[test]
fn distinct_module_functions_and_fresh_closures_do_not_merge() {
    for fresh in [false, true] {
        let mut module = fixture();
        let root = &mut module.functions[0];
        let first = read(root, 1);
        let other = if fresh {
            value(root, Type::Closure(1), |result| Op::MakeClosure {
                result,
                function: FunctionId(1),
                captures: vec![],
            })
        } else {
            read(root, 2)
        };
        equal(root, first, other);
        check(
            &module,
            "(()=>{function f(x){return x;}function g(x){return x;}return Number(f===g);})()",
        );
    }
}

#[test]
fn typed_calls_activate_gc_without_a_managed_carrier() {
    let mut module = fixture();
    let root = &mut module.functions[0];
    let callee = read(root, 1);
    let argument = value(root, Type::Number, |result| Op::Load {
        result,
        slot: SlotId(0),
    });
    let result = value(root, Type::Number, |result| Op::ClosureCall {
        result,
        callee,
        arguments: vec![argument],
    });
    root.body.exit = Exit::Return(result);
    assert!(!crate::emit::module_has_managed(&module));
    check(&module, "(()=>{function f(x){return x;}return f(a);})()");
}

#[test]
fn module_getters_keep_zero_and_multiple_argument_signatures() {
    for count in [0, 2] {
        let source = if count == 0 {
            "function f(){return 42;}export function run(a,b){return a;}"
        } else {
            "function f(x,y){return x+y;}export function run(a,b){return a;}"
        };
        let mut module = fixture_source(source);
        let root = &mut module.functions[0];
        let callee = value(root, Type::Closure(count), |result| Op::ModuleClosure {
            result,
            function: FunctionId(1),
        });
        let arguments = (0..count)
            .map(|i| {
                value(root, Type::Number, |result| Op::Load {
                    result,
                    slot: SlotId(i),
                })
            })
            .collect();
        let result = value(root, Type::Number, |result| Op::ClosureCall {
            result,
            callee,
            arguments,
        });
        root.body.exit = Exit::Return(result);
        check(
            &module,
            if count == 0 {
                "(()=>{function f(){return 42;}return f();})()"
            } else {
                "(()=>{function f(x,y){return x+y;}return f(a,b);})()"
            },
        );
    }
}

#[test]
fn module_functions_pass_through_typed_callback_frames() {
    let mut module = fixture();
    let child = &mut module.functions[2];
    let span = child.body.span;
    child.parameter_count = 2;
    child.slots = vec![Type::Closure(1), Type::Number]
        .into_iter()
        .map(|ty| Slot {
            ty,
            owner: RegionId(0),
            mutable: true,
            parameter: true,
            span,
        })
        .collect();
    child.values.clear();
    child.body.operations.truncate(1);
    let callee = value(child, Type::Closure(1), |result| Op::Load {
        result,
        slot: SlotId(0),
    });
    let argument = value(child, Type::Number, |result| Op::Load {
        result,
        slot: SlotId(1),
    });
    let result = value(child, Type::Number, |result| Op::ClosureCall {
        result,
        callee,
        arguments: vec![argument],
    });
    child.body.exit = Exit::Return(result);
    let signature = module
        .closure_signatures
        .intern(vec![Type::Closure(1), Type::Number], Type::Number);
    let root = &mut module.functions[0];
    let function = read(root, 1);
    let apply = value(root, Type::Closure(signature), |result| Op::ModuleClosure {
        result,
        function: FunctionId(2),
    });
    let argument = value(root, Type::Number, |result| Op::Load {
        result,
        slot: SlotId(0),
    });
    let result = value(root, Type::Number, |result| Op::ClosureCall {
        result,
        callee: apply,
        arguments: vec![function, argument],
    });
    root.body.exit = Exit::Return(result);
    check(
        &module,
        "(()=>{function f(x){return x;}function apply(fn,x){return fn(x);}return apply(f,a);})()",
    );
}

#[test]
fn general_and_managed_signatures_keep_the_existing_typed_abi() {
    for ty in [Type::General, Type::ManagedObject] {
        let mut module = fixture();
        let child = &mut module.functions[1];
        assert_eq!(child.values.len(), 1);
        child.slots[0].ty = ty;
        child.values[0] = ty;
        child.return_type = ty;
        let signature = module.closure_signatures.intern(vec![ty], ty);
        let root = &mut module.functions[0];
        let callee = value(root, Type::Closure(signature), |result| Op::ModuleClosure {
            result,
            function: FunctionId(1),
        });
        let argument = if ty == Type::General {
            let number = value(root, Type::Number, |result| Op::Load {
                result,
                slot: SlotId(0),
            });
            value(root, ty, |result| Op::Pack {
                result,
                input: number,
            })
        } else {
            value(root, ty, |result| Op::ManagedObjectNew { result })
        };
        let result = value(root, ty, |result| Op::ClosureCall {
            result,
            callee,
            arguments: vec![argument],
        });
        if ty == Type::General {
            let result = value(root, Type::Number, |number| Op::NumberFromGeneral {
                result: number,
                input: result,
            });
            root.body.exit = Exit::Return(result);
            check(&module, "(()=>{function f(x){return x;}return f(a);})()");
        } else {
            let left = value(root, Type::ManagedValue, |result| Op::ManagedFromObject {
                result,
                input: argument,
            });
            let right = value(root, Type::ManagedValue, |r| Op::ManagedFromObject {
                result: r,
                input: result,
            });
            let predicate = value(root, Type::Predicate, |result| Op::ManagedEqual {
                result,
                left,
                right,
            });
            let result = value(root, Type::Number, |result| Op::NumberFromPredicate {
                result,
                input: predicate,
            });
            root.body.exit = Exit::Return(result);
            check(
                &module,
                "(()=>{function f(x){return x;}const o={};return Number(f(o)===o);})()",
            );
        }
    }
}

#[test]
fn different_bodies_read_the_same_module_binding() {
    let mut module = fixture();
    let child = &mut module.functions[2];
    child.values.clear();
    child.body.operations.truncate(1);
    child.return_type = Type::Closure(1);
    let f = read(child, 1);
    child.body.exit = Exit::Return(f);
    let signature = module
        .closure_signatures
        .intern(vec![Type::Number], Type::Closure(1));
    let root = &mut module.functions[0];
    let direct = read(root, 1);
    let get = value(root, Type::Closure(signature), |result| Op::ModuleClosure {
        result,
        function: FunctionId(2),
    });
    let argument = value(root, Type::Number, |result| Op::Load {
        result,
        slot: SlotId(1),
    });
    let returned = value(root, Type::Closure(1), |result| Op::ClosureCall {
        result,
        callee: get,
        arguments: vec![argument],
    });
    equal(root, direct, returned);
    check(
        &module,
        "(()=>{function f(x){return x;}function get(){return f;}return Number(get()===f);})()",
    );
}

#[test]
fn module_functions_reject_invalid_targets_environments_and_signatures() {
    for case in 0..6 {
        let mut module = fixture();
        let root = &mut module.functions[0];
        let callee = read(root, if case == 0 { 999 } else { 1 });
        equal(root, callee, callee);
        match case {
            1 => {
                module.functions[1].body.operations.remove(0);
            }
            2 => {
                module.functions[1].body.operations[0].kind = Op::ClosureEntry {
                    captures: vec![SlotId(0)],
                };
            }
            3 => {
                module.functions[1].body.operations[0].kind =
                    Op::AsyncClosureEntry { captures: vec![] };
            }
            4 => {
                module.functions[0].values[callee.0] = Type::Number;
            }
            5 => {
                module.functions[1].return_type = Type::ManagedObject;
            }
            _ => (),
        }
        let error = verify_module(&module).unwrap_err();
        assert!(
            error.contains(match case {
                0 => "invalid module function ID",
                1 => "requires a closure entry",
                2 | 3 => "capture-free synchronous",
                5 => "unknown module function signature",
                _ => "type",
            }),
            "{case}: {error}"
        );
    }
}

#[test]
fn module_function_references_use_traced_single_frame_globals() {
    let mut module = fixture();
    let root = &mut module.functions[0];
    let callee = read(root, 1);
    let argument = value(root, Type::Number, |result| Op::Load {
        result,
        slot: SlotId(0),
    });
    let number = value(root, Type::Number, |result| Op::ClosureCall {
        result,
        callee,
        arguments: vec![argument],
    });
    let result = value(root, Type::General, |result| Op::Pack {
        result,
        input: number,
    });
    root.return_type = Type::General;
    root.body.exit = Exit::Return(result);
    verify_async_module(&module).unwrap();
    assert!(!crate::emit::module_has_managed(&module));
    wat::parse_str(crate::emit::emit_async(&module).unwrap()).unwrap();
}

#[test]
fn module_identity_collection_keeps_a_two_megabyte_native_stack() {
    let wat = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut module = fixture();
            let root = &mut module.functions[0];
            let first = read(root, 1);
            let again = read(root, 1);
            equal(root, first, again);
            let span = root.body.span;
            let old = std::mem::replace(
                &mut root.body,
                Region {
                    id: RegionId(128),
                    span,
                    operations: vec![],
                    exit: Exit::Fallthrough,
                },
            );
            let mut body = old;
            body.id = RegionId(128);
            for id in (0..128).rev() {
                body = Region {
                    id: RegionId(id),
                    span,
                    operations: vec![Operation {
                        span,
                        kind: Op::Block(body),
                    }],
                    exit: Exit::Fallthrough,
                };
            }
            root.body = body;
            crate::emit::emit(&module).unwrap()
        })
        .unwrap()
        .join()
        .unwrap();
    super::managed_value_tests::node(&wat::parse_str(wat).unwrap(), "1");
}
