use super::*;

#[test]
fn raw_rejections_require_the_async_profile_and_compatible_lexical_catches() {
    for source in [
        "export async function run(a,b){throw Promise.resolve(a);}",
        "export async function run(a,b){const f=async()=>{throw Promise.resolve(a);};return await f();}",
        "async function f(p){throw p;}export async function run(a,b){return await f(Promise.resolve(a));}",
    ] {
        let module = crate::lower_source_profile(source, false, Type::General, true).unwrap();
        verify_async_module(&module).unwrap();
        assert!(
            verify_module_with_entry_type(&module, Type::General)
                .unwrap_err()
                .contains("raw throw requires a raw lexical handler"),
            "{source}"
        );
    }
    let source =
        "export async function run(a,b){try{throw Promise.resolve(a);}catch(e){return e===e;}}";
    let mut module = crate::lower_source_profile(source, false, Type::General, true).unwrap();
    verify_async_module(&module).unwrap();
    let (_, reason, _) = catch_regions(&mut module.functions[0].body.operations);
    let reason = *reason;
    module.functions[0].values[reason.0] = Type::General;
    assert!(
        verify_async_module(&module)
            .unwrap_err()
            .contains("raw throw requires a raw lexical handler"),
        "a primitive handler cannot receive a raw reason"
    );
}

fn value(f: &mut Function, ty: Type) -> ValueId {
    let id = ValueId(f.values.len());
    f.values.push(ty);
    id
}

fn catch_regions(operations: &mut [Operation]) -> (&mut Region, &mut ValueId, &mut Region) {
    operations
        .iter_mut()
        .find_map(|op| match &mut op.kind {
            Op::TryCatch {
                body,
                reason,
                handler,
            } => Some((body, reason, handler)),
            _ => None,
        })
        .unwrap()
}

fn caught(promise: bool, primitive_throw: bool) -> Module {
    caught_with_input(promise, primitive_throw, "a")
}

fn caught_with_input(promise: bool, primitive_throw: bool, input: &str) -> Module {
    let mut module = crate::lower_source_profile(
        &format!("export async function run(a,b){{const p=Promise.resolve({input});try{{await b;throw a;}}catch(e){{await a;return e;}}}}"),
        false, Type::General, true,
    ).unwrap();
    let f = &mut module.functions[0];
    let span = f.body.span;
    let primitive = f
        .body
        .operations
        .iter()
        .find_map(|op| match op.kind {
            Op::FulfillPromise { input, .. } => Some(input),
            _ => None,
        })
        .unwrap();
    let original = f
        .body
        .operations
        .iter()
        .find_map(|op| match op.kind {
            Op::MaterializePromise { result, .. } => Some(result),
            _ => None,
        })
        .unwrap();
    let carrier = value(f, Type::AsyncValue);
    let projected = value(
        f,
        if promise {
            Type::Promise
        } else {
            Type::General
        },
    );
    let comparison = promise.then(|| (value(f, Type::Predicate), value(f, Type::General)));
    let index = f
        .body
        .operations
        .iter()
        .position(|op| matches!(op.kind, Op::TryCatch { .. }))
        .unwrap();
    f.body.operations.insert(
        index,
        Operation {
            span,
            kind: Op::ToAsyncValue {
                result: carrier,
                input: if promise { original } else { primitive },
            },
        },
    );
    let Op::TryCatch {
        body,
        reason,
        handler,
    } = &mut f.body.operations[index + 1].kind
    else {
        unreachable!()
    };
    body.exit = Exit::Throw(if primitive_throw { primitive } else { carrier });
    f.values[reason.0] = Type::AsyncValue;
    for operation in &handler.operations {
        if let Op::Initialize { slot, value } = operation.kind
            && value == *reason
        {
            f.slots[slot.0].ty = Type::AsyncValue;
        }
    }
    let Exit::Return(caught) = handler.exit else {
        unreachable!()
    };
    f.values[caught.0] = Type::AsyncValue;
    handler.operations.push(Operation {
        span,
        kind: if promise {
            Op::AsyncValuePromise {
                result: projected,
                input: caught,
            }
        } else {
            Op::AsyncValuePrimitive {
                result: projected,
                input: caught,
            }
        },
    });
    if promise {
        let (equal, result) = comparison.unwrap();
        handler.operations.extend([
            Operation {
                span,
                kind: Op::PromiseIdentityEqual {
                    result: equal,
                    left: original,
                    right: projected,
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
        handler.exit = Exit::Return(result);
    } else {
        handler.exit = Exit::Return(projected);
    }
    module
}

#[test]
fn raw_catch_reasons_preserve_identity_and_primitive_values_across_awaits() {
    let mut script = async_script();
    for (promise, primitive_throw) in [(true, false), (false, false), (false, true)] {
        let module = caught(promise, primitive_throw);
        verify_module_with_entry_type(&module, Type::General).unwrap();
        async_case(
            &mut script,
            &format!("raw-catch-{promise}-{primitive_throw}"),
            &module,
            if promise {
                "const p=Promise.resolve(a);try{await b;throw p;}catch(e){await a;return e===p;}"
            } else {
                "const p=Promise.resolve(a);try{await b;throw a;}catch(e){await a;return e;}"
            },
        );
    }
    finish_async(script);
}

#[test]
fn raw_catch_reasons_preserve_undefined_null_and_boolean_tags() {
    let mut script = async_script();
    for input in ["undefined", "null", "false", "true"] {
        let module = caught_with_input(false, false, input);
        verify_module_with_entry_type(&module, Type::General).unwrap();
        async_case(
            &mut script,
            &format!("raw-catch-kind-{input}"),
            &module,
            &format!(
                "const p=Promise.resolve({input});try{{await b;throw {input};}}catch(e){{await a;return e;}}"
            ),
        );
    }
    finish_async(script);
}

#[test]
fn raw_catch_reasons_receive_primitive_await_rejections() {
    let mut module = caught(false, false);
    let (body, _, _) = catch_regions(&mut module.functions[0].body.operations);
    let Op::Await { rejected, .. } = &mut body
        .operations
        .iter_mut()
        .find(|op| matches!(op.kind, Op::Await { .. }))
        .unwrap()
        .kind
    else {
        unreachable!()
    };
    *rejected = true;
    verify_module_with_entry_type(&module, Type::General).unwrap();
    let mut script = async_script();
    async_case(
        &mut script,
        "raw-catch-await-rejection",
        &module,
        "const p=Promise.resolve(a);try{await Promise.reject(b);throw a;}catch(e){await a;return e;}",
    );
    finish_async(script);
}

fn with_finally(mut module: Module, override_reason: bool) -> Module {
    let f = &mut module.functions[0];
    let span = f.body.span;
    let input = f
        .body
        .operations
        .iter()
        .find_map(|op| match op.kind {
            Op::FulfillPromise { input, .. } => Some(input),
            _ => None,
        })
        .unwrap();
    let awaited = value(f, Type::General);
    let returned = value(f, Type::AsyncValue);
    let reason = value(f, Type::AsyncValue);
    let replacement = override_reason.then(|| (value(f, Type::Number), value(f, Type::General)));
    let (body, _, handler) = catch_regions(&mut f.body.operations);
    let next = body.id.0.max(handler.id.0) + 1;
    let original = std::mem::replace(
        body,
        Region {
            id: RegionId(next),
            span,
            operations: Vec::new(),
            exit: Exit::Fallthrough,
        },
    );
    let mut finalizer = Region {
        id: RegionId(next + 1),
        span,
        operations: vec![Operation {
            span,
            kind: Op::Await {
                result: awaited,
                input,
                rejected: false,
            },
        }],
        exit: Exit::Fallthrough,
    };
    if let Some((input, result)) = replacement {
        finalizer.operations.push(Operation {
            span,
            kind: Op::Load {
                result: input,
                slot: SlotId(1),
            },
        });
        finalizer.operations.push(Operation {
            span,
            kind: Op::Pack { result, input },
        });
        finalizer.exit = Exit::Throw(result);
    }
    body.operations.push(Operation {
        span,
        kind: Op::TryFinally {
            body: original,
            finalizer,
            returned,
            reason,
            continuation: Box::new(Region {
                id: RegionId(next + 2),
                span,
                operations: Vec::new(),
                exit: Exit::Return(input),
            }),
        },
    });
    module
}

#[test]
fn raw_catch_reasons_survive_finally_cleanup_and_abrupt_override() {
    let mut script = async_script();
    for (promise, replace) in [(true, false), (false, false), (false, true)] {
        let module = with_finally(caught(promise, false), replace);
        verify_module_with_entry_type(&module, Type::General).unwrap();
        async_case(
            &mut script,
            &format!("raw-finally-{promise}-{replace}"),
            &module,
            match (promise, replace) {
                (true, false) => {
                    "const p=Promise.resolve(a);try{try{await b;throw p;}finally{await a;}}catch(e){await a;return e===p;}"
                }
                (false, false) => {
                    "const p=Promise.resolve(a);try{try{await b;throw a;}finally{await a;}}catch(e){await a;return e;}"
                }
                _ => {
                    "const p=Promise.resolve(a);try{try{await b;throw a;}finally{await a;throw b;}}catch(e){await a;return e;}"
                }
            },
        );
    }
    finish_async(script);
}

#[test]
fn raw_catch_reasons_rethrow_to_the_outer_handler() {
    let mut script = async_script();
    for promise in [false, true] {
        let mut module = caught(promise, false);
        let f = &mut module.functions[0];
        let reason = value(
            f,
            if promise {
                Type::AsyncValue
            } else {
                Type::General
            },
        );
        let primitive = f
            .body
            .operations
            .iter()
            .find_map(|op| match op.kind {
                Op::FulfillPromise { input, .. } => Some(input),
                _ => None,
            })
            .unwrap();
        let (body, _, handler) = catch_regions(&mut f.body.operations);
        let span = body.span;
        let next = body.id.0.max(handler.id.0) + 1;
        if !promise {
            body.exit = Exit::Throw(primitive);
        }
        let protected = std::mem::replace(
            body,
            Region {
                id: RegionId(next),
                span,
                operations: Vec::new(),
                exit: Exit::Fallthrough,
            },
        );
        body.operations.push(Operation {
            span,
            kind: Op::TryCatch {
                body: protected,
                reason,
                handler: Region {
                    id: RegionId(next + 1),
                    span,
                    operations: Vec::new(),
                    exit: Exit::Throw(reason),
                },
            },
        });
        verify_module_with_entry_type(&module, Type::General).unwrap();
        async_case(
            &mut script,
            &format!("raw-rethrow-{promise}"),
            &module,
            if promise {
                "const p=Promise.resolve(a);try{try{await b;throw p;}catch(inner){throw inner;}}catch(e){await a;return e===p;}"
            } else {
                "const p=Promise.resolve(a);try{try{await b;throw a;}catch(inner){throw inner;}}catch(e){await a;return e;}"
            },
        );
    }
    finish_async(script);
}

#[test]
fn raw_catch_reasons_are_not_visible_in_the_protected_body() {
    let mut module = caught(true, false);
    let (body, reason, _) = catch_regions(&mut module.functions[0].body.operations);
    body.exit = Exit::Throw(*reason);
    assert!(
        verify_module_with_entry_type(&module, Type::General)
            .unwrap_err()
            .contains("value is not visible")
    );
}

#[test]
fn raw_catch_reasons_cannot_escape_their_typed_lexical_handler() {
    for change in 0..5 {
        let mut module = caught(true, false);
        let f = &mut module.functions[0];
        let (body, reason, handler) = catch_regions(&mut f.body.operations);
        match change {
            0 => f.values[reason.0] = Type::General,
            1 => f.values[reason.0] = Type::Promise,
            2 => {
                let caught = handler
                    .operations
                    .iter()
                    .find_map(|op| match op.kind {
                        Op::Load { result, .. } if f.values[result.0] == Type::AsyncValue => {
                            Some(result)
                        }
                        _ => None,
                    })
                    .unwrap();
                handler.exit = Exit::Throw(caught);
            }
            3 => {
                let Exit::Throw(input) = body.exit else {
                    unreachable!()
                };
                let result = ValueId(f.values.len());
                f.values.push(Type::SettledPromise);
                body.operations.push(Operation {
                    span: body.span,
                    kind: Op::SettlePromise {
                        result,
                        body: Region {
                            id: RegionId(999),
                            span: body.span,
                            operations: Vec::new(),
                            exit: Exit::Throw(input),
                        },
                    },
                });
            }
            _ => {
                let Exit::Throw(input) = body.exit else {
                    unreachable!()
                };
                let result = ValueId(f.values.len());
                f.values.push(Type::General);
                body.operations.push(Operation {
                    span: body.span,
                    kind: Op::AwaitCall {
                        result,
                        body: Region {
                            id: RegionId(999),
                            span: body.span,
                            operations: Vec::new(),
                            exit: Exit::Throw(input),
                        },
                    },
                });
            }
        }
        let error = verify_module_with_entry_type(&module, Type::General).unwrap_err();
        assert!(
            error.contains(if change == 1 {
                "exception reason must be General or AsyncValue"
            } else {
                "raw throw requires a raw lexical handler"
            }),
            "case{change}: {error}"
        );
    }
    let mut module = with_finally(caught(true, false), false);
    let f = &mut module.functions[0];
    let (_, reason, _) = catch_regions(&mut f.body.operations);
    f.values[reason.0] = Type::General;
    assert!(
        verify_module_with_entry_type(&module, Type::General)
            .unwrap_err()
            .contains("raw finally reason requires a raw outer handler")
    );
}
