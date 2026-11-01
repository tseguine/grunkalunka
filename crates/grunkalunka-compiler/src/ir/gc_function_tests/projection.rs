use super::*;

fn artifact(module: &Module, name: &str, native: Option<&str>) -> Vec<u8> {
    verify_module(module).unwrap();
    let wat = crate::emit::emit(module).unwrap();
    let wasm = wat::parse_str(&wat).unwrap();
    if let Some(directory) = std::env::var_os("GRUNKALUNKA_IR_TEST_ARTIFACTS") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join(format!("projection-{name}"));
        std::fs::write(path.with_extension("wat"), wat).unwrap();
        std::fs::write(path.with_extension("wasm"), &wasm).unwrap();
        std::fs::write(path.with_extension("expect"), native.unwrap_or("TRAP")).unwrap();
    }
    wasm
}

fn compare(module: &Module, name: &str, native: &str) {
    super::super::managed_value_tests::node(&artifact(module, name, Some(native)), native);
}

fn number_parameter() -> (Module, ValueId) {
    let (mut module, callee) = fixture(false);
    module.functions[0].values[callee.0] = Type::Closure(1);
    let child = &mut module.functions[1];
    child.parameter_count = 1;
    child.slots.push(Slot {
        ty: Type::Number,
        owner: RegionId(0),
        mutable: true,
        parameter: true,
        span: child.body.span,
    });
    child.body.operations[1].kind = Op::Load {
        result: ValueId(0),
        slot: SlotId(0),
    };
    (module, callee)
}

fn traps(module: &Module, name: &str) {
    use std::{
        io::Write,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let wasm = artifact(module, name, None);
    let script = format!(
        "const assert=require('node:assert/strict');const run=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({wasm:?}))).exports.run;assert.throws(()=>run(17,9),WebAssembly.RuntimeError);"
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
                "projection trap check timed out: {}",
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
fn boxed_function_can_be_projected_and_called() {
    for capture in [false, true] {
        let (mut module, callee) = fixture(capture);
        let f = &mut module.functions[0];
        let input = boxed(f, callee);
        let projected = value(f, Type::Closure(0), |result| Op::ManagedToClosure {
            result,
            input,
        });
        let result = value(f, Type::Number, |result| Op::ClosureCall {
            result,
            callee: projected,
            arguments: vec![],
        });
        f.body.exit = Exit::Return(result);
        compare(
            &module,
            &format!("call-{capture}"),
            if capture {
                "(()=>{const f=()=>a;const o={f};return o.f();})()"
            } else {
                "(()=>{const f=()=>0;const o={f};return o.f();})()"
            },
        );
    }
}

#[test]
fn projection_checks_the_full_parameter_signature() {
    let (mut module, callee) = number_parameter();
    let requested = module
        .closure_signatures
        .intern(vec![Type::General], Type::Number);
    let f = &mut module.functions[0];
    let input = boxed(f, callee);
    value(f, Type::Closure(requested), |result| Op::ManagedToClosure {
        result,
        input,
    });
    let result = value(f, Type::Number, |result| Op::Constant {
        result,
        value: 0.0,
    });
    f.body.exit = Exit::Return(result);
    traps(&module, "parameter-general");
}

#[test]
fn projected_functions_accept_original_number_arguments() {
    let (mut module, callee) = number_parameter();
    let f = &mut module.functions[0];
    let input = boxed(f, callee);
    let callee = value(f, Type::Closure(1), |result| Op::ManagedToClosure {
        result,
        input,
    });
    let argument = value(f, Type::Number, |result| Op::Load {
        result,
        slot: SlotId(0),
    });
    let result = value(f, Type::Number, |result| Op::ClosureCall {
        result,
        callee,
        arguments: vec![argument],
    });
    f.body.exit = Exit::Return(result);
    compare(
        &module,
        "number-argument",
        "(()=>{const o={f:x=>x};return o.f(a);})()",
    );
}

#[test]
fn reboxing_a_projected_function_preserves_its_original_identity() {
    let (mut module, callee) = fixture(true);
    let f = &mut module.functions[0];
    let input = boxed(f, callee);
    let projected = value(f, Type::Closure(0), |result| Op::ManagedToClosure {
        result,
        input,
    });
    let again = boxed(f, projected);
    equality(f, input, again);
    compare(
        &module,
        "identity",
        "(()=>{const f=()=>a;const o={f};return Number(o.f===f);})()",
    );
}

#[test]
fn projected_property_snapshots_stay_callable_after_overwrite() {
    for (name, body) in [
        (
            "object-property",
            "const f=()=>a;const o={f};const saved=o.f;o.f=b;",
        ),
        (
            "array-property",
            "const f=()=>a;const o=[f];const saved=o[0];o[0]=b;",
        ),
    ] {
        let source = format!("export function run(a,b){{{body}return saved===f?a:b;}}");
        let mut module = crate::lower_source(&source, false).unwrap();
        let f = &mut module.functions[0];
        let reads = f
            .body
            .operations
            .iter()
            .filter_map(|operation| match operation.kind {
                Op::ManagedGetOwn { result, .. } | Op::ManagedArrayGetOwn { result, .. } => {
                    Some(result)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(reads.len(), 1);
        let callee = value(f, Type::Closure(0), |result| Op::ManagedToClosure {
            result,
            input: reads[0],
        });
        let result = value(f, Type::Number, |result| Op::ClosureCall {
            result,
            callee,
            arguments: vec![],
        });
        f.body.exit = Exit::Return(result);
        compare(&module, name, &format!("(()=>{{{body}return saved();}})()"));
    }
}

#[test]
fn complete_signature_checks_include_arity_and_result_types() {
    for (name, parameters, result_type) in [
        ("arity", vec![], Type::Number),
        ("result", vec![Type::Number], Type::General),
        ("managed-parameter", vec![Type::ManagedObject], Type::Number),
        ("callback-parameter", vec![Type::Closure(0)], Type::Number),
    ] {
        let (mut module, callee) = number_parameter();
        let requested = module.closure_signatures.intern(parameters, result_type);
        let f = &mut module.functions[0];
        let input = boxed(f, callee);
        value(f, Type::Closure(requested), |result| Op::ManagedToClosure {
            result,
            input,
        });
        let result = value(f, Type::Number, |result| Op::Constant {
            result,
            value: 0.0,
        });
        f.body.exit = Exit::Return(result);
        traps(&module, name);
    }
}

#[test]
fn nonfunctions_cannot_be_projected_to_callable_values() {
    for kind in 0..7 {
        let (mut module, _) = fixture(false);
        if kind == 0 {
            // Projection itself must select GC storage, even with no function
            // construction, closure table or environment anywhere in the module.
            module.functions.truncate(1);
            module.functions[0].values.clear();
            module.functions[0].body.operations.clear();
        }
        let f = &mut module.functions[0];
        let input = match kind {
            0 | 1 => {
                let primitive = value(f, Type::General, |result| Op::Nullish {
                    result,
                    value: if kind == 0 {
                        Nullish::Undefined
                    } else {
                        Nullish::Null
                    },
                });
                value(f, Type::ManagedValue, |result| Op::ManagedPrimitive {
                    result,
                    input: primitive,
                })
            }
            2 | 3 => {
                let number = if kind == 2 {
                    value(f, Type::Predicate, |result| Op::Boolean {
                        result,
                        value: true,
                    })
                } else {
                    value(f, Type::Number, |result| Op::Constant {
                        result,
                        value: 17.0,
                    })
                };
                let primitive = value(f, Type::General, |result| Op::Pack {
                    result,
                    input: number,
                });
                value(f, Type::ManagedValue, |result| Op::ManagedPrimitive {
                    result,
                    input: primitive,
                })
            }
            4 => value(f, Type::ManagedValue, |result| Op::ManagedString {
                result,
                units: vec![102],
            }),
            5 => {
                let object = value(f, Type::ManagedObject, |result| Op::ManagedObjectNew {
                    result,
                });
                value(f, Type::ManagedValue, |result| Op::ManagedFromObject {
                    result,
                    input: object,
                })
            }
            _ => {
                let array = value(f, Type::ManagedArray, |result| Op::ManagedArrayNew {
                    result,
                    length: 0,
                });
                value(f, Type::ManagedValue, |result| Op::ManagedFromArray {
                    result,
                    input: array,
                })
            }
        };
        value(f, Type::Closure(0), |result| Op::ManagedToClosure {
            result,
            input,
        });
        let result = value(f, Type::Number, |result| Op::Constant {
            result,
            value: 0.0,
        });
        f.body.exit = Exit::Return(result);
        traps(&module, &format!("nonfunction-{kind}"));
    }
}

// Instrument the typed IR boundary, leaving the original source evaluation and
// all call operands unchanged. Every root call must pass through boxed storage.
#[test]
fn projection_retains_rich_signatures_and_callee_snapshots() {
    for (name, body) in [
        (
            "object",
            "const f=x=>x;const box={f};const o={x:a};return f(o)===o?1:0;",
        ),
        (
            "array",
            "const f=x=>x;const box={f};const o=[a,b];return f(o)===o?1:0;",
        ),
        (
            "string",
            "const f=x=>x;const box={f};return f('text')==='text'?1:0;",
        ),
        (
            "callback",
            "const f=fn=>fn(a);const box={f};const g=x=>x;return f(g);",
        ),
        (
            "returned",
            "const f=()=>()=>a;const box={f};const g=f();return g();",
        ),
        (
            "snapshot",
            "let f=x=>x;const old=f;const box={f};return f((f=x=>b,a));",
        ),
        ("declaration", "const f=id;const box={f};return f(a);"),
    ] {
        let helpers = if name == "declaration" {
            "function id(x){return x;}"
        } else {
            ""
        };
        let source = format!("{helpers}export function run(a,b){{{body}}}");
        let mut module =
            crate::lower_source(&source, false).unwrap_or_else(|error| panic!("{name}: {error}"));
        let f = &mut module.functions[0];
        let calls = (0..f.body.operations.len())
            .filter(|index| matches!(f.body.operations[*index].kind, Op::ClosureCall { .. }))
            .collect::<Vec<_>>();
        assert!(!calls.is_empty(), "{name}");
        for index in calls.into_iter().rev() {
            let Op::ClosureCall { callee, .. } = &mut f.body.operations[index].kind else {
                unreachable!()
            };
            let original = *callee;
            let input = ValueId(f.values.len());
            f.values.push(Type::ManagedValue);
            let projected = ValueId(f.values.len());
            f.values.push(f.values[original.0]);
            *callee = projected;
            let span = f.body.operations[index].span;
            f.body.operations.splice(
                index..index,
                [
                    Operation {
                        span,
                        kind: Op::ManagedFromClosure {
                            result: input,
                            input: original,
                        },
                    },
                    Operation {
                        span,
                        kind: Op::ManagedToClosure {
                            result: projected,
                            input,
                        },
                    },
                ],
            );
        }
        compare(&module, name, &format!("(()=>{{{helpers}{body}}})()"));
    }
}

#[test]
fn projected_general_parameter_and_result_preserve_boolean_kinds() {
    let (mut module, callee) = number_parameter();
    let signature = module
        .closure_signatures
        .intern(vec![Type::General], Type::General);
    let child = &mut module.functions[1];
    child.slots[0].ty = Type::General;
    child.values[0] = Type::General;
    child.return_type = Type::General;
    let f = &mut module.functions[0];
    f.values[callee.0] = Type::Closure(signature);
    let input = boxed(f, callee);
    let callee = value(f, Type::Closure(signature), |result| Op::ManagedToClosure {
        result,
        input,
    });
    let flag = value(f, Type::Predicate, |result| Op::Boolean {
        result,
        value: true,
    });
    let argument = value(f, Type::General, |result| Op::Pack {
        result,
        input: flag,
    });
    let actual = value(f, Type::General, |result| Op::ClosureCall {
        result,
        callee,
        arguments: vec![argument],
    });
    let equal = value(f, Type::Predicate, |result| Op::PrimitiveStrictEqual {
        result,
        left: actual,
        right: argument,
        negated: false,
    });
    let result = value(f, Type::Number, |result| Op::NumberFromPredicate {
        result,
        input: equal,
    });
    f.body.exit = Exit::Return(result);
    compare(
        &module,
        "general-boolean",
        "(()=>{const o={f:x=>x};return Number(o.f(true)===true);})()",
    );
}

#[test]
fn projection_does_not_bypass_signature_registry_invariants() {
    let (mut module, callee) = fixture(false);
    module.closure_signatures.higher.push(ClosureSignature {
        parameters: vec![],
        result: Type::Number,
        asynchronous: false,
    });
    let f = &mut module.functions[0];
    let input = boxed(f, callee);
    let projected = value(f, Type::Closure(1001), |result| Op::ManagedToClosure {
        result,
        input,
    });
    let result = value(f, Type::Number, |result| Op::ClosureCall {
        result,
        callee: projected,
        arguments: vec![],
    });
    f.body.exit = Exit::Return(result);
    assert!(
        verify_module(&module)
            .unwrap_err()
            .contains("invalid closure signature")
    );
}

#[test]
fn projection_queries_and_emission_fit_a_two_megabyte_native_stack() {
    let (mut module, callee) = fixture(false);
    let f = &mut module.functions[0];
    let input = boxed(f, callee);
    let callee = value(f, Type::Closure(0), |result| Op::ManagedToClosure {
        result,
        input,
    });
    let result = value(f, Type::Number, |result| Op::ClosureCall {
        result,
        callee,
        arguments: vec![],
    });
    f.body.exit = Exit::Return(result);
    let span = f.body.span;
    let mut body = std::mem::replace(
        &mut f.body,
        Region {
            id: RegionId(0),
            span,
            operations: vec![],
            exit: Exit::Fallthrough,
        },
    );
    body.id = RegionId(256);
    for index in (0..256).rev() {
        body = Region {
            id: RegionId(index),
            span,
            operations: vec![Operation {
                span,
                kind: Op::Block(body),
            }],
            exit: Exit::Fallthrough,
        };
    }
    f.body = body;
    let wat = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            verify_module(&module).unwrap();
            crate::emit::emit(&module).unwrap()
        })
        .unwrap()
        .join()
        .unwrap();
    let wasm = wat::parse_str(wat).unwrap();
    super::super::managed_value_tests::node(&wasm, "0");
}

#[test]
fn projection_does_not_enter_numeric_async_spill_frames() {
    let (mut module, callee) = fixture(false);
    let f = &mut module.functions[0];
    f.return_type = Type::General;
    let input = boxed(f, callee);
    value(f, Type::Closure(0), |result| Op::ManagedToClosure {
        result,
        input,
    });
    let number = value(f, Type::Number, |result| Op::Constant {
        result,
        value: 0.0,
    });
    let result = value(f, Type::General, |result| Op::Pack {
        result,
        input: number,
    });
    f.body.exit = Exit::Return(result);
    verify_async_module(&module).unwrap();
    assert_eq!(
        crate::emit::emit_async(&module).unwrap_err(),
        "managed values require reference-aware async frames"
    );
}

#[test]
fn projection_requires_visible_managed_input_and_known_synchronous_result() {
    for result_type in [Type::Number, Type::ManagedValue, Type::Closure(usize::MAX)] {
        let (mut module, callee) = fixture(false);
        let f = &mut module.functions[0];
        let input = boxed(f, callee);
        value(f, result_type, |result| Op::ManagedToClosure {
            result,
            input,
        });
        let result = value(f, Type::Number, |result| Op::Constant {
            result,
            value: 0.0,
        });
        f.body.exit = Exit::Return(result);
        assert!(
            verify_module(&module)
                .unwrap_err()
                .contains("managed function projection requires")
        );
    }
    let (mut module, callee) = fixture(false);
    let asynchronous = module.closure_signatures.intern_async(vec![]);
    let f = &mut module.functions[0];
    let input = boxed(f, callee);
    value(f, Type::Closure(asynchronous), |result| {
        Op::ManagedToClosure { result, input }
    });
    let result = value(f, Type::Number, |result| Op::Constant {
        result,
        value: 0.0,
    });
    f.body.exit = Exit::Return(result);
    assert!(
        verify_module(&module)
            .unwrap_err()
            .contains("known synchronous signature")
    );
    for later in [false, true] {
        let (mut module, callee) = fixture(false);
        let f = &mut module.functions[0];
        let input = if later {
            ValueId(f.values.len() + 1)
        } else {
            callee
        };
        value(f, Type::Closure(0), |result| Op::ManagedToClosure {
            result,
            input,
        });
        if later {
            boxed(f, callee);
        }
        let result = value(f, Type::Number, |result| Op::Constant {
            result,
            value: 0.0,
        });
        f.body.exit = Exit::Return(result);
        let error = verify_module(&module).unwrap_err();
        if later {
            assert!(error.contains("not visible"), "{error}");
        }
    }
}
