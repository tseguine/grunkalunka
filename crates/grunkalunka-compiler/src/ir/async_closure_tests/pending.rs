use super::*;

fn siblings(body: &str) -> Module {
    let mut module = call_module(body);
    let f = &mut module.functions[0];
    let span = f.body.span;
    f.values
        .extend([Type::Promise, Type::Promise, Type::General]);
    f.body.operations.pop().unwrap();
    for (result, arguments) in [
        (ValueId(4), vec![ValueId(0), ValueId(1)]),
        (ValueId(5), vec![ValueId(1), ValueId(0)]),
    ] {
        f.body.operations.push(Operation {
            span,
            kind: Op::StartAsyncClosure {
                result,
                callee: ValueId(2),
                arguments,
            },
        });
    }
    for (result, input) in [(ValueId(3), ValueId(4)), (ValueId(6), ValueId(5))] {
        f.body.operations.push(Operation {
            span,
            kind: Op::AwaitPromise { result, input },
        });
    }
    f.body.exit = Exit::Return(ValueId(6));
    module
}

#[test]
fn pending_promise_ir_sibling_activations_start_eagerly_and_await_later() {
    let module = siblings("await a;return b;");
    verify_module_with_entry_type(&module, Type::General).unwrap();
    let mut script = script();
    case(
        &mut script,
        "pending-siblings",
        &module,
        "const f=async(a,b)=>{await a;return b;};const p=f(a,b);const q=f(b,a);await p;return await q;",
    );
    finish(script);
}

fn pending_call(module: &mut Module) {
    fn replace(region: &mut Region, values: &mut Vec<Type>) {
        let mut operations = Vec::new();
        for mut operation in region.operations.drain(..) {
            match &mut operation.kind {
                Op::AwaitClosureCall {
                    result,
                    callee,
                    arguments,
                } => {
                    let handle = ValueId(values.len());
                    values.push(Type::Promise);
                    operations.push(Operation {
                        span: operation.span,
                        kind: Op::StartAsyncClosure {
                            result: handle,
                            callee: *callee,
                            arguments: std::mem::take(arguments),
                        },
                    });
                    operation.kind = Op::AwaitPromise {
                        result: *result,
                        input: handle,
                    };
                }
                Op::TryCatch { body, handler, .. } => {
                    replace(body, values);
                    replace(handler, values);
                }
                _ => (),
            }
            operations.push(operation);
        }
        region.operations = operations;
    }
    let f = &mut module.functions[0];
    replace(&mut f.body, &mut f.values);
}

#[test]
fn pending_promise_ir_keeps_primitive_completions_and_rejection_reactions() {
    let mut script = script();
    for (index, body) in [
        "return b;",
        "await a;return b;",
        "await a;await b;return a-b;",
        "let x=a;await b;return x+b;",
        "if(a){await b;return null;}return false;",
        "await a;return void 0;",
        "throw b;",
        "await a;throw b;",
        "return await Promise.reject(b);",
        "try{await Promise.reject(b);}catch(e){await a;return e;}",
        "const g=async()=>{await b;return a;};return await g();",
        "const g=async()=>{await a;throw b;};try{return await g();}catch(e){return e;}",
        "let x=0;for(let i=0;i<3;i++)x+=await b;return x;",
    ]
    .iter()
    .enumerate()
    {
        for caught in [false, true] {
            let mut module = call_module(body);
            if caught {
                protect(&mut module);
            }
            pending_call(&mut module);
            let source = format!("const p=(async(a,b)=>{{{body}}})(a,b);return await p;");
            let source = if caught {
                format!("try{{{source}}}catch(e){{return e;}}")
            } else {
                source
            };
            case(
                &mut script,
                &format!("pending-body-{index}-{caught}"),
                &module,
                &source,
            );
        }
    }
    finish(script);
}

#[test]
fn pending_promise_ir_independent_frames_keep_parameters_and_saved_values() {
    let mut script = script();
    for (index, body) in [
        "return b;",
        "await a;return b;",
        "await a;await b;return a-b;",
        "let x=a;await b;return x+b;",
        "if(a){await b;return null;}return false;",
        "let x=a;const g=()=>++x;await b;return g()+x;",
        "let x=0;for(let i=0;i<2;i++)x+=await b;return x+a;",
    ]
    .iter()
    .enumerate()
    {
        let module = siblings(body);
        case(
            &mut script,
            &format!("pending-independent-{index}"),
            &module,
            &format!(
                "const f=async(a,b)=>{{{body}}};const p=f(a,b);const q=f(b,a);await p;return await q;"
            ),
        );
    }
    finish(script);
}

#[test]
fn pending_promise_ir_repeated_and_late_awaits_keep_identity() {
    let mut script = script();
    for (index, body) in [
        "return b;",
        "await a;return b;",
        "await a;await b;return b;",
    ]
    .iter()
    .enumerate()
    {
        let mut module = siblings(body);
        let f = &mut module.functions[0];
        let span = f.body.span;
        let last = f.body.operations.pop().unwrap();
        let again = ValueId(f.values.len());
        f.values.push(Type::General);
        f.body.operations.push(Operation {
            span,
            kind: Op::AwaitPromise {
                result: again,
                input: ValueId(4),
            },
        });
        f.body.operations.push(last);
        case(
            &mut script,
            &format!("pending-repeat-{index}"),
            &module,
            &format!(
                "const f=async(a,b)=>{{{body}}};const p=f(a,b);const q=f(b,a);await p;await p;return await q;"
            ),
        );
    }
    finish(script);
}

#[test]
fn pending_promise_ir_nested_recursive_activations_and_rejections_share_fifo() {
    let mut script = script();
    for captured in [false, true] {
        let mut module = recursive_module(captured);
        pending_call(&mut module);
        case(
            &mut script,
            &format!("pending-recursive-{captured}"),
            &module,
            "const f=async(x,y)=>{if(x>0){const r=await f(x-1,y);return +r+x;}return await y;};return await f(a,b);",
        );
    }
    finish(script);
}

#[test]
fn pending_promise_ir_verifier_keeps_handles_separate_from_completions() {
    for wrong in [
        Type::General,
        Type::FulfilledPromise,
        Type::SettledPromise,
        Type::Number,
        Type::Closure(0),
    ] {
        let mut module = siblings("return b;");
        module.functions[0].values[4] = wrong;
        assert!(verify_module_with_entry_type(&module, Type::General).is_err());
    }
    let mut module = siblings("return b;");
    let f = &mut module.functions[0];
    let Op::StartAsyncClosure { callee, .. } = &mut f.body.operations[3].kind else {
        panic!("start");
    };
    *callee = ValueId(0);
    assert!(verify_module_with_entry_type(&module, Type::General).is_err());
}

#[test]
fn pending_promise_ir_handles_survive_cell_writes_captures_and_private_transport() {
    let mut module = siblings("let saved=a;await b;return saved+b;");
    let getter_signature = module.closure_signatures.intern(vec![], Type::Promise);
    let identity_signature = module.closure_signatures.intern(
        vec![Type::General, Type::Promise, Type::Number],
        Type::Promise,
    );
    let f = &mut module.functions[0];
    let span = f.body.span;
    let op = |kind| Operation { span, kind };
    f.slots.push(Slot {
        ty: Type::Promise,
        owner: RegionId(0),
        mutable: true,
        parameter: false,
        span,
    });
    let getter = ValueId(f.values.len());
    f.values.push(Type::Closure(getter_signature));
    let first = ValueId(f.values.len());
    f.values.push(Type::Promise);
    let second = ValueId(f.values.len());
    f.values.push(Type::Promise);
    let identity = ValueId(f.values.len());
    f.values.push(Type::Closure(identity_signature));
    let packed = ValueId(f.values.len());
    f.values.push(Type::General);
    let transported = ValueId(f.values.len());
    f.values.push(Type::Promise);
    let operations = vec![
        op(Op::Initialize {
            slot: SlotId(2),
            value: ValueId(4),
        }),
        op(Op::CaptureCells {
            cells: vec![SlotId(2)],
        }),
        op(Op::MakeClosure {
            result: getter,
            function: FunctionId(2),
            captures: vec![SlotId(2)],
        }),
        op(Op::ClosureCall {
            result: first,
            callee: getter,
            arguments: vec![],
        }),
        op(Op::Store {
            slot: SlotId(2),
            value: ValueId(5),
        }),
        op(Op::ClosureCall {
            result: second,
            callee: getter,
            arguments: vec![],
        }),
        op(Op::MakeClosure {
            result: identity,
            function: FunctionId(3),
            captures: vec![],
        }),
        op(Op::Pack {
            result: packed,
            input: ValueId(0),
        }),
        op(Op::ClosureCall {
            result: transported,
            callee: identity,
            arguments: vec![packed, first, ValueId(1)],
        }),
    ];
    f.body.operations.splice(5..5, operations);
    let index = f.body.operations.len() - 2;
    let Op::AwaitPromise { input, .. } = &mut f.body.operations[index].kind else {
        panic!("await");
    };
    *input = transported;
    let Op::AwaitPromise { input, .. } = &mut f.body.operations[index + 1].kind else {
        panic!("await");
    };
    *input = second;
    let slot = |ty, parameter| Slot {
        ty,
        owner: RegionId(0),
        mutable: true,
        parameter,
        span,
    };
    module.functions.push(Function {
        return_type: Type::Promise,
        parameter_count: 0,
        slots: vec![slot(Type::Promise, false)],
        values: vec![Type::Promise],
        body: Region {
            id: RegionId(0),
            span,
            operations: vec![
                op(Op::ClosureEntry {
                    captures: vec![SlotId(0)],
                }),
                op(Op::Load {
                    result: ValueId(0),
                    slot: SlotId(0),
                }),
            ],
            exit: Exit::Return(ValueId(0)),
        },
    });
    module.functions.push(Function {
        return_type: Type::Promise,
        parameter_count: 3,
        slots: vec![
            slot(Type::General, true),
            slot(Type::Promise, true),
            slot(Type::Number, true),
        ],
        values: vec![Type::Promise],
        body: Region {
            id: RegionId(0),
            span,
            operations: vec![
                op(Op::ClosureEntry { captures: vec![] }),
                op(Op::Load {
                    result: ValueId(0),
                    slot: SlotId(1),
                }),
            ],
            exit: Exit::Return(ValueId(0)),
        },
    });
    let mut script = script();
    case(
        &mut script,
        "pending-private-transport",
        &module,
        "const f=async(a,b)=>{let saved=a;await b;return saved+b;};let p=f(a,b);const q=f(b,a);const read=()=>p;const alias=read();p=q;const latest=read();const id=(x,p,y)=>p;await id(a,alias,b);return await latest;",
    );
    finish(script);
}

#[test]
fn pending_promise_ir_root_settlement_does_not_cancel_queued_activations() {
    let mut module = siblings("await a;await b;return b;");
    let f = &mut module.functions[0];
    f.body.operations.truncate(5);
    f.values.pop();
    f.body.operations.push(Operation {
        span: f.body.span,
        kind: Op::Pack {
            result: ValueId(3),
            input: ValueId(0),
        },
    });
    f.body.exit = Exit::Return(ValueId(3));
    let mut script = script();
    case(
        &mut script,
        "pending-root-settled",
        &module,
        "const f=async(a,b)=>{await a;await b;return b;};f(a,b);f(b,a);return a;",
    );
    let bytes = wat::parse_str(crate::emit::emit_async(&module).unwrap()).unwrap();
    script.push_str(&format!("{{const g=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from({bytes:?}))).exports;assert.deepEqual(g.start(4,2),[0,3,4]);assert.equal(g.take_jobs(),2);for(const issued of[1,1,0,0]){{assert.deepEqual(g.resume(0,0),[0,3,4]);assert.equal(g.take_jobs(),issued);}}assert.throws(()=>g.resume(0,0),WebAssembly.RuntimeError);assert.throws(()=>g.start(4,2),WebAssembly.RuntimeError);}}"));
    finish(script);
}

#[test]
fn pending_promise_ir_async_callbacks_and_different_signatures_share_the_queue() {
    let mut script = script();
    for (index, (mut module, source)) in [
        (callback_module(false), "let x=a;const g=()=>x;const f=async(fn,y)=>{await y;return fn();};x=b;return await f(g,b);"),
        (callback_module(true), "let x=a;const g=async()=>await x;const f=async(fn,y)=>{await y;return await fn();};x=b;return await f(g,b);"),
    ].into_iter().enumerate() {
        pending_call(&mut module);
        case(&mut script, &format!("pending-callback-{index}"), &module, source);
    }
    finish(script);
}

#[test]
fn pending_promise_ir_site_limits_include_the_caller_reaction() {
    let mut script = script();
    for count in [31, 32] {
        let mut module = call_module("return await b;");
        pending_call(&mut module);
        let f = &mut module.functions[0];
        let span = f.body.span;
        let mut last = ValueId(3);
        for _ in 1..count {
            last = ValueId(f.values.len());
            f.values.push(Type::General);
            f.body.operations.push(Operation {
                span,
                kind: Op::AwaitPromise {
                    result: last,
                    input: ValueId(4),
                },
            });
        }
        f.body.exit = Exit::Return(last);
        if count == 32 {
            assert!(
                crate::emit::emit_async(&module)
                    .unwrap_err()
                    .contains("32 await sites")
            );
        } else {
            case(
                &mut script,
                "pending-32-jobs",
                &module,
                &format!(
                    "const f=async()=>await b;const p=f();{}return await p;",
                    "await p;".repeat(30)
                ),
            );
        }
    }
    finish(script);
}

#[test]
fn pending_promise_ir_does_not_project_handles_or_start_inside_closed_settlement() {
    let mut module = siblings("return b;");
    let f = &mut module.functions[0];
    let Op::AwaitPromise { input, .. } = &mut f.body.operations[5].kind else {
        panic!("await");
    };
    *input = ValueId(0);
    assert!(verify_module_with_entry_type(&module, Type::General).is_err());

    let mut module = siblings("return b;");
    let f = &mut module.functions[0];
    let operation = f.body.operations.remove(3);
    f.body.operations.insert(
        3,
        Operation {
            span: f.body.span,
            kind: Op::SettlePromise {
                result: ValueId(f.values.len()),
                body: Region {
                    id: RegionId(1),
                    span: f.body.span,
                    operations: vec![operation],
                    exit: Exit::Throw(ValueId(0)),
                },
            },
        },
    );
    f.values.push(Type::SettledPromise);
    assert!(
        verify_module_with_entry_type(&module, Type::General)
            .unwrap_err()
            .contains("settlement bodies cannot start")
    );
}

#[test]
fn pending_promise_ir_shared_capture_observes_both_eager_prefixes() {
    let prefix = "let x=0;let f=async()=>{x+=a;await b;x+=b;return x;};";
    let mut module = crate::lower_source_profile(
        &format!("export async function run(a,b){{{prefix}return await(a?f:f)();}}"),
        false,
        Type::General,
        true,
    )
    .unwrap();
    let f = &mut module.functions[0];
    let index = f
        .body
        .operations
        .iter()
        .rposition(|op| matches!(op.kind, Op::AwaitClosureCall { .. }))
        .unwrap();
    let call = f.body.operations.remove(index);
    let suffix = f.body.operations.split_off(index);
    if f.body
        .operations
        .last()
        .is_some_and(|op| matches!(op.kind, Op::AwaitSite { .. }))
    {
        f.body.operations.pop();
    }
    let Op::AwaitClosureCall {
        result: first,
        callee,
        arguments,
    } = call.kind
    else {
        panic!("runtime async call: {:?}", call.kind);
    };
    let span = f.body.span;
    let op = |kind| Operation { span, kind };
    let mut value = |ty| {
        let id = ValueId(f.values.len());
        f.values.push(ty);
        id
    };
    let p = value(Type::Promise);
    let q = value(Type::Promise);
    let second = value(Type::General);
    let left = value(Type::Number);
    let right = value(Type::Number);
    let scale = value(Type::Number);
    let product = value(Type::Number);
    let sum = value(Type::Number);
    let completion = value(Type::General);
    f.body.operations.extend([
        op(Op::StartAsyncClosure {
            result: p,
            callee,
            arguments: arguments.clone(),
        }),
        op(Op::StartAsyncClosure {
            result: q,
            callee,
            arguments,
        }),
        op(Op::AwaitPromise {
            result: first,
            input: p,
        }),
        op(Op::AwaitPromise {
            result: second,
            input: q,
        }),
    ]);
    f.body.operations.extend(suffix);
    f.body.operations.extend([
        op(Op::NumberFromGeneral {
            result: left,
            input: first,
        }),
        op(Op::NumberFromGeneral {
            result: right,
            input: second,
        }),
        op(Op::Constant {
            result: scale,
            value: 1000.0,
        }),
        op(Op::Binary {
            result: product,
            operator: Binary::Multiply,
            left,
            right: scale,
        }),
        op(Op::Binary {
            result: sum,
            operator: Binary::Add,
            left: product,
            right,
        }),
        op(Op::Pack {
            result: completion,
            input: sum,
        }),
    ]);
    f.body.exit = Exit::Return(completion);
    let mut script = script();
    case(
        &mut script,
        "pending-eager-shared-capture",
        &module,
        &format!(
            "{prefix}const p=(a?f:f)();const q=(a?f:f)();const left=await p;const right=await q;return +left*1000+(+right);"
        ),
    );
    finish(script);
}

#[test]
fn pending_promise_ir_async_parameters_retain_a_shared_pending_handle() {
    let mut module = siblings("await a;await b;return b;");
    let signature = module.closure_signatures.intern_async(vec![Type::Promise]);
    let f = &mut module.functions[0];
    let span = f.body.span;
    let op = |kind| Operation { span, kind };
    let fun = ValueId(f.values.len());
    f.values.push(Type::Closure(signature));
    let p = ValueId(f.values.len());
    f.values.push(Type::Promise);
    let q = ValueId(f.values.len());
    f.values.push(Type::Promise);
    f.body.operations.splice(
        5..5,
        [
            op(Op::MakeClosure {
                result: fun,
                function: FunctionId(2),
                captures: vec![],
            }),
            op(Op::StartAsyncClosure {
                result: p,
                callee: fun,
                arguments: vec![ValueId(4)],
            }),
            op(Op::StartAsyncClosure {
                result: q,
                callee: fun,
                arguments: vec![ValueId(4)],
            }),
        ],
    );
    let index = f.body.operations.len() - 2;
    let Op::AwaitPromise { input, .. } = &mut f.body.operations[index].kind else {
        panic!("await")
    };
    *input = p;
    let Op::AwaitPromise { input, .. } = &mut f.body.operations[index + 1].kind else {
        panic!("await")
    };
    *input = q;
    module.functions.push(Function {
        return_type: Type::General,
        parameter_count: 1,
        slots: vec![Slot {
            ty: Type::Promise,
            owner: RegionId(0),
            mutable: true,
            parameter: true,
            span,
        }],
        values: vec![Type::Promise, Type::General],
        body: Region {
            id: RegionId(0),
            span,
            operations: vec![
                op(Op::AsyncClosureEntry { captures: vec![] }),
                op(Op::Load {
                    result: ValueId(0),
                    slot: SlotId(0),
                }),
                op(Op::AwaitPromise {
                    result: ValueId(1),
                    input: ValueId(0),
                }),
            ],
            exit: Exit::Return(ValueId(1)),
        },
    });
    let mut script = script();
    case(
        &mut script,
        "pending-shared-waiters",
        &module,
        "const f=async(a,b)=>{await a;await b;return b;};const p=f(a,b);const q=f(b,a);const wait=async p=>await p;const x=wait(p);const y=wait(p);await x;return await y;",
    );
    finish(script);
}
