use super::*;

fn value(f: &mut Function, ty: Type) -> ValueId {
    let result = ValueId(f.values.len());
    f.values.push(ty);
    result
}
fn fulfilled_seed(expression: &str) -> Module {
    let mut module = crate::lower_source_profile(
        &format!("export async function run(a,b){{return {expression};}}"),
        false,
        Type::General,
        true,
    )
    .unwrap();
    let f = &mut module.functions[0];
    let Exit::Return(input) = f.body.exit else {
        unreachable!()
    };
    let snapshot = value(f, Type::FulfilledPromise);
    let promise = value(f, Type::Promise);
    let completion = value(f, Type::General);
    let span = f.body.span;
    for kind in [
        Op::FulfillPromise {
            result: snapshot,
            input,
        },
        Op::MaterializePromise {
            result: promise,
            input: snapshot,
        },
        Op::AwaitPromise {
            result: completion,
            input: promise,
        },
    ] {
        f.body.operations.push(Operation { span, kind });
    }
    f.body.exit = Exit::Return(completion);
    module
}

#[test]
fn materialized_fulfilled_promise_awaits_its_snapshot() {
    let mut script = async_script();
    for (index, expression) in [
        "a",
        "b",
        "null",
        "undefined",
        "a<b",
        "-0",
        "NaN",
        "Infinity",
    ]
    .iter()
    .enumerate()
    {
        async_case(
            &mut script,
            &format!("materialized-fulfilled-{index}"),
            &fulfilled_seed(expression),
            &format!("const p=Promise.resolve({expression});return await p;"),
        );
    }
    finish_async(script);
}

fn settled_seed(expression: &str) -> Module {
    let mut module = capture_source(&format!("if(a)return {expression};throw {expression};"));
    let f = &mut module.functions[0];
    let Op::SettlePromise { result: input, .. } = f.body.operations[0].kind else {
        unreachable!()
    };
    let promise = value(f, Type::Promise);
    let completion = value(f, Type::General);
    let span = f.body.span;
    f.body.operations.extend([
        Operation {
            span,
            kind: Op::MaterializePromise {
                result: promise,
                input,
            },
        },
        Operation {
            span,
            kind: Op::AwaitPromise {
                result: completion,
                input: promise,
            },
        },
    ]);
    f.body.exit = Exit::Return(completion);
    module
}

#[test]
fn materialized_settlements_preserve_status_payload_and_reactions() {
    let mut script = async_script();
    for (index, expression) in [
        "a",
        "b",
        "null",
        "undefined",
        "a<b",
        "-0",
        "NaN",
        "Infinity",
    ]
    .iter()
    .enumerate()
    {
        async_case(
            &mut script,
            &format!("materialized-settled-{index}"),
            &settled_seed(expression),
            &format!(
                "const p=a?Promise.resolve({expression}):Promise.reject({expression});return await p;"
            ),
        );
    }
    finish_async(script);
}

fn private_creator(identity: bool) -> Module {
    let mut module = crate::lower_source_profile(
        "export async function run(a,b){return a;}",
        false,
        Type::General,
        true,
    )
    .unwrap();
    let signature = module
        .closure_signatures
        .intern(vec![Type::General], Type::Promise);
    let f = &mut module.functions[0];
    let Exit::Return(input) = f.body.exit else {
        unreachable!()
    };
    let span = f.body.span;
    let callee = value(f, Type::Closure(signature));
    let first = value(f, Type::Promise);
    f.body.operations.extend([
        Operation {
            span,
            kind: Op::MakeClosure {
                result: callee,
                function: FunctionId(1),
                captures: vec![],
            },
        },
        Operation {
            span,
            kind: Op::ClosureCall {
                result: first,
                callee,
                arguments: vec![input],
            },
        },
    ]);
    let result = if identity {
        let second = value(f, Type::Promise);
        let alias = value(f, Type::Predicate);
        let different = value(f, Type::Predicate);
        let no = value(f, Type::Predicate);
        let result = value(f, Type::Predicate);
        let packed = value(f, Type::General);
        for kind in [
            Op::ClosureCall {
                result: second,
                callee,
                arguments: vec![input],
            },
            Op::PromiseIdentityEqual {
                result: alias,
                left: first,
                right: first,
                negated: false,
            },
            Op::PromiseIdentityEqual {
                result: different,
                left: first,
                right: second,
                negated: true,
            },
            Op::Boolean {
                result: no,
                value: false,
            },
            Op::If {
                result: Some(result),
                condition: alias,
                consequent: Region {
                    id: RegionId(1000),
                    span,
                    operations: vec![],
                    exit: Exit::Yield(different),
                },
                alternate: Region {
                    id: RegionId(1001),
                    span,
                    operations: vec![],
                    exit: Exit::Yield(no),
                },
            },
            Op::Pack {
                result: packed,
                input: result,
            },
        ] {
            f.body.operations.push(Operation { span, kind });
        }
        packed
    } else {
        let result = value(f, Type::General);
        f.body.operations.push(Operation {
            span,
            kind: Op::AwaitPromise {
                result,
                input: first,
            },
        });
        result
    };
    f.body.exit = Exit::Return(result);
    module.functions.push(Function {
        return_type: Type::Promise,
        parameter_count: 1,
        slots: vec![Slot {
            ty: Type::General,
            owner: RegionId(0),
            mutable: true,
            parameter: true,
            span,
        }],
        values: vec![Type::General, Type::FulfilledPromise, Type::Promise],
        body: Region {
            id: RegionId(0),
            span,
            operations: vec![
                Operation {
                    span,
                    kind: Op::ClosureEntry { captures: vec![] },
                },
                Operation {
                    span,
                    kind: Op::Load {
                        result: ValueId(0),
                        slot: SlotId(0),
                    },
                },
                Operation {
                    span,
                    kind: Op::FulfillPromise {
                        result: ValueId(1),
                        input: ValueId(0),
                    },
                },
                Operation {
                    span,
                    kind: Op::MaterializePromise {
                        result: ValueId(2),
                        input: ValueId(1),
                    },
                },
            ],
            exit: Exit::Return(ValueId(2)),
        },
    });
    module
}

#[test]
fn materialized_private_creators_allocate_per_call_and_keep_aliases_stable() {
    let mut script = async_script();
    async_case(
        &mut script,
        "materialized-private-repeat",
        &private_creator(true),
        "const make=x=>Promise.resolve(x);const p=make(a),alias=p,q=make(a);return p===alias&&p!==q;",
    );
    async_case(
        &mut script,
        "materialized-private-await",
        &private_creator(false),
        "const make=x=>Promise.resolve(x);return await make(a);",
    );
    finish_async(script);
}

#[test]
fn materialized_promises_use_existing_async_return_adoption() {
    let mut script = async_script();
    for (name, mut module, native) in [
        (
            "fulfilled",
            fulfilled_seed("a"),
            "return Promise.resolve(a);",
        ),
        (
            "settled",
            settled_seed("b"),
            "return a?Promise.resolve(b):Promise.reject(b);",
        ),
    ] {
        let f = &mut module.functions[0];
        let last = f.body.operations.last_mut().unwrap();
        let Op::AwaitPromise { result, input } = last.kind else {
            unreachable!()
        };
        last.kind = Op::AdoptPromise { result, input };
        async_case(
            &mut script,
            &format!("materialized-adopt-{name}"),
            &module,
            native,
        );
    }
    finish_async(script);
}

fn replace_settled_awaits(region: &mut Region, values: &mut Vec<Type>) -> usize {
    let mut count = 0;
    for mut operation in std::mem::take(&mut region.operations) {
        if let Op::AwaitSettled { result, input } = operation.kind {
            let marker = if region
                .operations
                .last()
                .is_some_and(|o| matches!(o.kind, Op::AwaitSite { .. }))
            {
                region.operations.pop()
            } else {
                None
            };
            let promise = ValueId(values.len());
            values.push(Type::Promise);
            region.operations.push(Operation {
                span: operation.span,
                kind: Op::MaterializePromise {
                    result: promise,
                    input,
                },
            });
            region.operations.extend(marker);
            region.operations.push(Operation {
                span: operation.span,
                kind: Op::AwaitPromise {
                    result,
                    input: promise,
                },
            });
            count += 1;
            continue;
        }
        match &mut operation.kind {
            Op::InlineCall { body, .. }
            | Op::SettlePromise { body, .. }
            | Op::AwaitCall { body, .. }
            | Op::Block(body) => count += replace_settled_awaits(body, values),
            Op::TryCatch { body, handler, .. } => {
                count += replace_settled_awaits(body, values);
                count += replace_settled_awaits(handler, values);
            }
            Op::If {
                consequent,
                alternate,
                ..
            } => {
                count += replace_settled_awaits(consequent, values);
                count += replace_settled_awaits(alternate, values);
            }
            _ => (),
        }
        region.operations.push(operation);
    }
    count
}

#[test]
fn materialized_snapshots_keep_intrinsic_error_identity_and_settlement() {
    let cycle = "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();";
    let body = format!(
        "{cycle}try{{await p;}}catch(e){{const h=a?async x=>x:async x=>{{throw x;}};try{{const x=await h(e);return a?x===e:false;}}catch(x){{return a?false:x===e;}}}}"
    );
    let mut module = crate::lower_source_profile_with_identities(
        &format!("export async function run(a,b){{{body}}}"),
        false,
        Type::General,
        true,
        false,
        false,
    )
    .unwrap();
    let mut changed = 0;
    for f in &mut module.functions {
        changed += replace_settled_awaits(&mut f.body, &mut f.values);
    }
    assert!(changed > 0);
    let mut script = async_script();
    async_case(&mut script, "materialized-error", &module, &body);
    finish_async(script);
}

fn notification_seed(rejected: bool) -> Module {
    let mut module = capture_source(if rejected { "throw a;" } else { "return a;" });
    let f = &mut module.functions[0];
    let Op::SettlePromise { result: input, .. } = f.body.operations[0].kind else {
        unreachable!()
    };
    let span = f.body.span;
    let first = value(f, Type::Promise);
    let second = value(f, Type::Promise);
    let unequal = value(f, Type::Predicate);
    let result = value(f, Type::General);
    for kind in [
        Op::MaterializePromise {
            result: first,
            input,
        },
        Op::MaterializePromise {
            result: second,
            input,
        },
        Op::PromiseIdentityEqual {
            result: unequal,
            left: first,
            right: second,
            negated: true,
        },
        Op::Pack {
            result,
            input: unequal,
        },
    ] {
        f.body.operations.push(Operation { span, kind });
    }
    f.body.exit = Exit::Return(result);
    module
}

#[test]
fn materialized_construction_adds_no_jobs_and_preserves_rejection_tracking() {
    let mut script = async_script();
    script.push_str("const reports=[];const listener=(reason,p)=>reports.push({reason,p});process.on('unhandledRejection',listener);const flush=()=>new Promise(r=>setImmediate(r));try{\n");
    for rejected in [false, true] {
        let module = notification_seed(rejected);
        let bytes = wat::parse_str(crate::emit::emit_async(&module).unwrap()).unwrap();
        let name = format!("materialized-notify-{rejected}");
        wasm_record(&name, "async", &bytes);
        let method = if rejected { "reject" } else { "resolve" };
        script.push_str(&format!(r#"{{const bytes=Uint8Array.from({bytes:?}),wasm=createAsyncFunction(bytes),native=async(a,b)=>{{const p=Promise.{method}(a),q=Promise.{method}(a);return p!==q;}};
let index=0;for(const[a,b]of[[4,2],[0,-0],[-0,-2],[NaN,Infinity]]){{
 const raw=new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports;assert.deepEqual(raw.start(a,b),[0,2,1]);assert.equal(raw.take_jobs(),0);const events=[];for(;;){{const e=raw.take_promise_event();if(!e[0])break;events.push(e);}}assert.equal(events.length,{count});if({rejected}){{assert.notEqual(events[0][1],events[1][1]);for(const e of events){{assert.equal(e[0],1);assert.equal(e[2],3);assert(Object.is(e[3],a));}}}}
 reports.length=0;const expected=await observe(native,a,b);await flush();const reference=reports.splice(0);assert.equal(reference.length,{count});assert.equal(new Set(reference.map(e=>e.p)).size,{count});
 const actual=await observe(wasm,a,b);await flush();const observed=reports.splice(0);assert.deepEqual(actual,expected);assert.deepEqual(observed.map(e=>e.reason),reference.map(e=>e.reason));assert.equal(new Set(observed.map(e=>e.p)).size,{count});
 if({proof})receipt({name:?},index,expected);index++;
}}}}
"#,count=if rejected{2}else{0},proof=proof()));
    }
    script.push_str("}finally{process.off('unhandledRejection',listener);}\n");
    finish_async(script);
}

#[test]
fn materialization_checks_snapshot_visibility_result_and_runtime_profile() {
    for change in 0..7 {
        let mut module = fulfilled_seed("a");
        let f = &mut module.functions[0];
        let index = f
            .body
            .operations
            .iter()
            .position(|o| matches!(o.kind, Op::MaterializePromise { .. }))
            .unwrap();
        let Op::MaterializePromise { result, input } = &mut f.body.operations[index].kind else {
            unreachable!()
        };
        match change {
            0 => *input = ValueId(999),
            1 => *input = ValueId(0),
            2 => *input = ValueId(1),
            3 => *input = *result,
            4 => f.values[result.0] = Type::General,
            5 => {
                f.body.operations.swap(index - 1, index);
            }
            _ => *result = ValueId(999),
        }
        assert!(verify_module_with_entry_type(&module, Type::General).is_err());
    }
    let mut module = private_creator(true);
    verify_module_with_entry_type(&module, Type::General).unwrap();
    assert!(
        crate::emit::emit_with_entry_type(&module, Type::General)
            .unwrap_err()
            .contains("async runtime profile")
    );
    module.functions[0].return_type = Type::Number;
    module.functions[0].body.exit = Exit::Return(ValueId(0));
    verify_module(&module).unwrap();
    assert!(
        crate::emit::emit(&module)
            .unwrap_err()
            .contains("async runtime profile")
    );
}
