use super::*;

fn value(f: &mut Function, ty: Type) -> ValueId {
    let id = ValueId(f.values.len());
    f.values.push(ty);
    id
}

fn seed(promise: bool) -> (Module, ValueId) {
    let mut module = crate::lower_source_profile(
        "export async function run(a,b){return a;}",
        false,
        Type::General,
        true,
    )
    .unwrap();
    let f = &mut module.functions[0];
    let Exit::Return(mut input) = f.body.exit else {
        unreachable!()
    };
    let span = f.body.span;
    if promise {
        let snapshot = value(f, Type::FulfilledPromise);
        let id = value(f, Type::Promise);
        f.body.operations.extend([
            Operation {
                span,
                kind: Op::FulfillPromise {
                    result: snapshot,
                    input,
                },
            },
            Operation {
                span,
                kind: Op::MaterializePromise {
                    result: id,
                    input: snapshot,
                },
            },
        ]);
        input = id;
    }
    let carrier = value(f, Type::AsyncValue);
    f.body.operations.push(Operation {
        span,
        kind: Op::ToAsyncValue {
            result: carrier,
            input,
        },
    });
    (module, carrier)
}

#[test]
fn async_value_lifts_and_projections_check_types_and_visibility() {
    for promise in [false, true] {
        let (module, _) = seed(promise);
        verify_module_with_entry_type(&module, Type::General).unwrap();
        for change in 0..5 {
            let (mut bad, carrier) = seed(promise);
            let f = &mut bad.functions[0];
            let Op::ToAsyncValue { result, input } =
                &mut f.body.operations.last_mut().unwrap().kind
            else {
                unreachable!()
            };
            match change {
                0 => *input = ValueId(9999),
                1 => *input = ValueId(0),
                2 => *input = carrier,
                3 => f.values[result.0] = Type::General,
                _ => *result = ValueId(9999),
            }
            assert!(verify_module_with_entry_type(&bad, Type::General).is_err());
        }
        for kind in [0, 1, 2] {
            for bad in [false, true] {
                let (mut module, carrier) = seed(promise);
                let f = &mut module.functions[0];
                let ty = [Type::Predicate, Type::General, Type::Promise][kind];
                let result = value(f, ty);
                let input = if bad { ValueId(0) } else { carrier };
                let op = match kind {
                    0 => Op::AsyncValueIsPromise { result, input },
                    1 => Op::AsyncValuePrimitive { result, input },
                    _ => Op::AsyncValuePromise { result, input },
                };
                f.body.operations.push(Operation {
                    span: f.body.span,
                    kind: op,
                });
                assert_eq!(
                    verify_module_with_entry_type(&module, Type::General).is_ok(),
                    !bad
                );
            }
        }
        let (mut escaped, carrier) = seed(promise);
        escaped.functions[0].body.exit = Exit::Return(carrier);
        assert!(verify_module_with_entry_type(&escaped, Type::General).is_err());
        escaped.functions[0].return_type = Type::AsyncValue;
        assert!(verify_module_with_entry_type(&escaped, Type::General).is_err());
        assert!(verify_module_with_entry_type(&escaped, Type::AsyncValue).is_err());
        for signature in [
            ClosureSignature {
                parameters: vec![Type::AsyncValue],
                result: Type::General,
                asynchronous: false,
            },
            ClosureSignature {
                parameters: vec![Type::General],
                result: Type::AsyncValue,
                asynchronous: false,
            },
        ] {
            let (mut escaped, _) = seed(promise);
            let carrier_result = signature.result == Type::AsyncValue;
            escaped.closure_signatures.higher.push(signature);
            verify_module_with_entry_type(&escaped, Type::General).unwrap();
            escaped.closure_signatures.higher[0].asynchronous = true;
            assert_eq!(
                verify_module_with_entry_type(&escaped, Type::General).is_err(),
                carrier_result,
                "async completion remains General; its private inputs may be carriers"
            );
        }
    }
}

#[test]
fn async_value_projections_execute_after_suspension_and_preserve_identity() {
    let mut script = async_script();
    for promise in [false, true] {
        let (mut module, carrier) = seed(promise);
        let f = &mut module.functions[0];
        let span = f.body.span;
        let Exit::Return(pause_input) = f.body.exit else {
            unreachable!()
        };
        let awaited = value(f, Type::General);
        let result = value(f, Type::General);
        f.body.operations.extend([Operation {
            span,
            kind: Op::Await {
                result: awaited,
                input: pause_input,
                rejected: false,
            },
        }]);
        if promise {
            let id = value(f, Type::Promise);
            let original = f
                .body
                .operations
                .iter()
                .find_map(|op| match op.kind {
                    Op::MaterializePromise { result, .. } => Some(result),
                    _ => None,
                })
                .unwrap();
            let equal = value(f, Type::Predicate);
            f.body.operations.extend([
                Operation {
                    span,
                    kind: Op::AsyncValuePromise {
                        result: id,
                        input: carrier,
                    },
                },
                Operation {
                    span,
                    kind: Op::PromiseIdentityEqual {
                        result: equal,
                        left: original,
                        right: id,
                        negated: false,
                    },
                },
                Operation {
                    span,
                    kind: Op::Pack {
                        result,
                        input: equal,
                    },
                },
            ]);
        } else {
            f.body.operations.push(Operation {
                span,
                kind: Op::AsyncValuePrimitive {
                    result,
                    input: carrier,
                },
            });
        }
        f.body.exit = Exit::Return(result);
        async_case(
            &mut script,
            &format!("async-value-{promise}"),
            &module,
            if promise {
                "const p=Promise.resolve(a);await a;return p===p;"
            } else {
                "await a;return a;"
            },
        );
    }
    finish_async(script);
}

#[test]
fn wrong_async_value_projection_traps_instead_of_leaking_a_completion_tag() {
    let mut script = async_script();
    for promise in [false, true] {
        let (mut module, carrier) = seed(promise);
        let f = &mut module.functions[0];
        let result = value(
            f,
            if promise {
                Type::General
            } else {
                Type::Promise
            },
        );
        f.body.operations.push(Operation {
            span: f.body.span,
            kind: if promise {
                Op::AsyncValuePrimitive {
                    result,
                    input: carrier,
                }
            } else {
                Op::AsyncValuePromise {
                    result,
                    input: carrier,
                }
            },
        });
        verify_module_with_entry_type(&module, Type::General).unwrap();
        let bytes = wat::parse_str(crate::emit::emit_async(&module).unwrap()).unwrap();
        script.push_str(&format!("assert.throws(()=>new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from({bytes:?}))).exports.start(4,2),WebAssembly.RuntimeError);\n"));
    }
    finish_async(script);
}

#[test]
fn async_value_carriers_survive_sibling_coroutine_frames() {
    let source =
        "let f=async x=>{await x;return x;};const p=f(a),q=f(b);return (await p)+(await q)*100;";
    let mut module = crate::lower_source_profile(
        &format!("export async function run(a,b){{{source}}}"),
        false,
        Type::General,
        true,
    )
    .unwrap();
    let f = module
        .functions
        .iter_mut()
        .find(|f| is_async_closure(f))
        .unwrap();
    let carrier = value(f, Type::AsyncValue);
    let packed = value(f, Type::General);
    let result = value(f, Type::General);
    let body = f
        .body
        .operations
        .iter_mut()
        .find_map(|op| match &mut op.kind {
            Op::InlineCall { body, .. } => Some(body),
            _ => None,
        })
        .unwrap();
    let index = body
        .operations
        .iter()
        .position(|op| matches!(op.kind, Op::Await { .. }))
        .unwrap();
    let Op::Await { input, .. } = body.operations[index].kind else {
        unreachable!()
    };
    let span = body.span;
    let before = if index > 0 && matches!(body.operations[index - 1].kind, Op::AwaitSite { .. }) {
        index - 1
    } else {
        index
    };
    body.operations.insert(
        before,
        Operation {
            span,
            kind: Op::Pack {
                result: packed,
                input,
            },
        },
    );
    body.operations.insert(
        before + 1,
        Operation {
            span,
            kind: Op::ToAsyncValue {
                result: carrier,
                input: packed,
            },
        },
    );
    body.operations.push(Operation {
        span,
        kind: Op::AsyncValuePrimitive {
            result,
            input: carrier,
        },
    });
    body.exit = Exit::Return(result);
    let mut script = async_script();
    async_case(&mut script, "async-value-sibling-frames", &module, source);
    finish_async(script);
}
