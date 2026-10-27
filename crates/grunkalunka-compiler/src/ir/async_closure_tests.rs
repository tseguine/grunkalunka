use super::*;

#[test]
fn await_site_provenance_is_explicit_and_requires_an_adjacent_await() {
    for shared in [false, true] {
        let mut module = crate::lower_source_profile(
            "export async function run(a,b){return a;}",
            false,
            Type::General,
            true,
        )
        .unwrap();
        let function = &mut module.functions[0];
        let Exit::Return(input) = function.body.exit else {
            unreachable!()
        };
        let span = function.body.span;
        for _ in 0..33 {
            if shared {
                function.body.operations.push(Operation {
                    span,
                    kind: Op::AwaitSite { site: 7 },
                });
            }
            let result = ValueId(function.values.len());
            function.values.push(Type::General);
            function.body.operations.push(Operation {
                span,
                kind: Op::Await {
                    result,
                    input,
                    rejected: false,
                },
            });
        }
        verify_module_with_entry_type(&module, Type::General).unwrap();
        let emitted = crate::emit::emit_async(&module);
        if shared {
            wat::parse_str(emitted.unwrap()).unwrap();
        } else {
            assert!(emitted.unwrap_err().contains("32 await sites"));
        }
        module.functions[0].body.operations.push(Operation {
            span,
            kind: Op::AwaitSite { site: 9 },
        });
        assert!(
            verify_module_with_entry_type(&module, Type::General)
                .unwrap_err()
                .contains("immediately precede an await")
        );
    }
}

#[test]
fn await_site_names_are_local_to_each_compiled_function() {
    let mut module = call_module(&format!("{}return b;", "await a;".repeat(32)));
    for function in &mut module.functions {
        let index = function
            .body
            .operations
            .iter()
            .position(|operation| {
                matches!(
                    operation.kind,
                    Op::Await { .. } | Op::AwaitClosureCall { .. }
                )
            })
            .unwrap();
        function.body.operations.insert(
            index,
            Operation {
                span: function.body.span,
                kind: Op::AwaitSite { site: 7 },
            },
        );
    }
    verify_module_with_entry_type(&module, Type::General).unwrap();
    assert!(
        crate::emit::emit_async(&module)
            .unwrap_err()
            .contains("32 await sites")
    );
}

#[test]
fn async_closure_ir_signatures_distinguish_call_effects() {
    let mut signatures = ClosureSignatures::default();
    let normal = signatures.intern(vec![Type::General], Type::General);
    let asynchronous = signatures.intern_async(vec![Type::General]);
    assert_ne!(normal, asynchronous);
    assert!(!signatures.is_async(normal));
    assert!(signatures.is_async(asynchronous));
    assert_eq!(signatures.result(asynchronous), Some(Type::General));
    assert_eq!(signatures.intern_async(vec![Type::General]), asynchronous);
    assert!(signatures.is_callback_signature(asynchronous));
    assert!(signatures.is_returned_function(asynchronous));
}

fn call_module(body: &str) -> Module {
    let mut module = crate::lower_source_profile(
        &format!("export async function run(a,b){{{body}}}"),
        false,
        Type::General,
        true,
    )
    .unwrap();
    for function in &mut module.functions {
        shift_functions(&mut function.body);
    }
    let span = module.functions[0].body.span;
    module.functions[0].body.operations.insert(
        0,
        Operation {
            span,
            kind: Op::AsyncClosureEntry { captures: vec![] },
        },
    );
    let signature = module
        .closure_signatures
        .intern_async(vec![Type::Number, Type::Number]);
    let function = Function {
        return_type: Type::General,
        parameter_count: 2,
        slots: (0..2)
            .map(|_| Slot {
                ty: Type::Number,
                owner: RegionId(0),
                mutable: true,
                parameter: true,
                span,
            })
            .collect(),
        values: vec![
            Type::Number,
            Type::Number,
            Type::Closure(signature),
            Type::General,
        ],
        body: Region {
            id: RegionId(0),
            span,
            operations: vec![
                Operation {
                    span,
                    kind: Op::Load {
                        result: ValueId(0),
                        slot: SlotId(0),
                    },
                },
                Operation {
                    span,
                    kind: Op::Load {
                        result: ValueId(1),
                        slot: SlotId(1),
                    },
                },
                Operation {
                    span,
                    kind: Op::MakeClosure {
                        result: ValueId(2),
                        function: FunctionId(1),
                        captures: vec![],
                    },
                },
                Operation {
                    span,
                    kind: Op::AwaitClosureCall {
                        result: ValueId(3),
                        callee: ValueId(2),
                        arguments: vec![ValueId(0), ValueId(1)],
                    },
                },
            ],
            exit: Exit::Return(ValueId(3)),
        },
    };
    module.functions.insert(0, function);
    module
}

fn shift_functions(region: &mut Region) {
    for operation in &mut region.operations {
        match &mut operation.kind {
            Op::MakeClosure { function, .. } | Op::Call { function, .. } => function.0 += 1,
            Op::InlineCall { body, .. }
            | Op::AwaitCall { body, .. }
            | Op::SettlePromise { body, .. }
            | Op::Block(body) => shift_functions(body),
            Op::TryCatch { body, handler, .. } => {
                shift_functions(body);
                shift_functions(handler);
            }
            Op::If {
                consequent,
                alternate,
                ..
            } => {
                shift_functions(consequent);
                shift_functions(alternate);
            }
            Op::While {
                condition,
                body,
                step,
            } => {
                shift_functions(condition);
                shift_functions(body);
                if let Some(step) = step {
                    shift_functions(step);
                }
            }
            Op::DoWhile { body, condition } => {
                shift_functions(body);
                shift_functions(condition);
            }
            _ => (),
        }
    }
}

#[test]
fn async_closure_ir_call_emits_a_suspending_private_body() {
    let module = call_module("return await b;");
    verify_module_with_entry_type(&module, Type::General).unwrap();
    let wat = crate::emit::emit_async(&module).unwrap();
    wat::parse_str(wat).unwrap();
}

fn proof() -> bool {
    std::env::var_os("GRUNKALUNKA_FRAME_PROOFS").is_some()
}

fn script() -> String {
    let adapter = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("grunkalunka-runtime/js/async.mjs");
    format!(
        r#"const assert=require('node:assert/strict');
(async()=>{{
const{{createAsyncFunction}}=await import(require('node:url').pathToFileURL({:?}));
const view=new DataView(new ArrayBuffer(8));
function encode(v){{if(v===undefined)return'undefined';if(v===null)return'null';if(typeof v==='boolean')return'boolean:'+v;assert.equal(typeof v,'number');if(Number.isNaN(v))return'nan';view.setFloat64(0,v);return view.getBigUint64(0).toString(16);}}
async function observe(run,a,b){{const trace=[];const p=run(a,b);trace.push('returned');const done=p.then(v=>trace.push('fulfilled:'+encode(v)),e=>trace.push('rejected:'+encode(e)));const markers=new Promise(resolve=>{{let n=0;function tick(){{trace.push('tick'+(++n));if(n<34)queueMicrotask(tick);else resolve();}}queueMicrotask(tick);}});await Promise.all([done,markers]);return trace;}}
function receipt(name,index,trace){{const at=trace.findIndex(x=>x.startsWith('fulfilled:')||x.startsWith('rejected:'));const line=trace[at],cut=line.indexOf(':'),kind=line.slice(0,cut),v=line.slice(cut+1);let tag,bits;if(v==='undefined'){{tag=0;bits='0000000000000000';}}else if(v==='null'){{tag=1;bits='0000000000000000';}}else if(v.startsWith('boolean:')){{tag=2;view.setFloat64(0,v==='boolean:true'?1:0);bits=view.getBigUint64(0).toString(16).padStart(16,'0');}}else{{tag=3;bits=v==='nan'?'nan':v.padStart(16,'0');}}console.log('IR_FRAME_OBS|'+name+'|'+index+'|'+kind+'|'+tag+'|'+bits+'|'+trace.slice(0,at).filter(x=>x.startsWith('tick')).length);}}
"#,
        adapter.to_string_lossy()
    )
}

fn case(script: &mut String, name: &str, module: &Module, source: &str) {
    let bytes = wat::parse_str(crate::emit::emit_async(module).unwrap()).unwrap();
    if proof() {
        println!(
            "IR_FRAME_WASM|{name}|async|{}",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        );
    }
    script.push_str(&format!("{{const native=async(a,b)=>{{{source}}};const wasm=createAsyncFunction(Uint8Array.from({bytes:?}));let i=0;for(const[a,b]of[[4,2],[0,-0],[-0,-2],[NaN,Infinity]]){{const expected=await observe(native,a,b);assert.deepEqual(await observe(wasm,a,b),expected,{source:?});if({})receipt({name:?},i,expected);i++;}}assert.deepEqual(await Promise.allSettled([wasm(4,2),wasm(-0,-2)]),await Promise.allSettled([native(4,2),native(-0,-2)]));}}\n",proof()));
}

fn finish(mut script: String) {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    script.push_str("})().catch(e=>{console.error(e);process.exitCode=1;});\n");
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
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    if proof() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
}

#[test]
fn async_closure_ir_private_body_values_and_reactions_match_native() {
    let mut script = script();
    for (index, body) in [
        "return b;",
        "return await b;",
        "await a;return b;",
        "await a;",
        "if(a)return await b;return await null;",
        "throw b;",
        "await a;throw b;",
        "return await Promise.reject(b);",
        "try{return await Promise.reject(b);}catch(e){throw e;}",
        "try{throw await a;}catch(e){return await b;}",
        "let x=0;for(let i=0;i<3;i++)x+=await b;return x;",
        "const g=async()=>{throw b;};return await g();",
        "let g=async()=>{throw b;};return await g();",
        "let x=a;const g=()=>++x;await b;return g()+x;",
        "return await(a?true:false);",
        "throw null;",
        "return await Promise.reject(undefined);",
        "return await Promise.reject(a?false:true);",
        "const g=()=>++a;await b;return g()+a;",
        "await a;try{throw b;}catch(e){await e;return e;}",
    ]
    .iter()
    .enumerate()
    {
        for caught in [false, true] {
            let mut module = call_module(body);
            let source = format!("return await(async()=>{{{body}}})();");
            let source = if caught {
                protect(&mut module);
                format!("try{{{source}}}catch(e){{return e;}}")
            } else {
                source
            };
            case(
                &mut script,
                &format!("body-{index}-{caught}"),
                &module,
                &source,
            );
        }
    }
    finish(script);
}

fn protect(module: &mut Module) {
    let f = &mut module.functions[0];
    let span = f.body.span;
    let call = f.body.operations.pop().unwrap();
    let Op::AwaitClosureCall { result, .. } = call.kind else {
        unreachable!()
    };
    let reason = ValueId(f.values.len());
    f.values.push(Type::General);
    f.body.operations.push(Operation {
        span,
        kind: Op::TryCatch {
            body: Region {
                id: RegionId(1),
                span,
                operations: vec![call],
                exit: Exit::Return(result),
            },
            reason,
            handler: Region {
                id: RegionId(2),
                span,
                operations: vec![],
                exit: Exit::Return(reason),
            },
        },
    });
    f.body.exit = Exit::Fallthrough;
}

fn selected_module(left: &str, right: &str) -> Module {
    let mut module = call_module(left);
    let mut other = call_module(right);
    assert_eq!(module.functions.len(), 2);
    assert_eq!(other.functions.len(), 2);
    module.functions.push(other.functions.remove(1));
    let f = &mut module.functions[0];
    let span = f.body.span;
    let ty = f.values[2];
    f.values.extend([ty, ty, Type::Predicate]);
    f.body.operations.remove(2);
    f.body.operations.insert(
        2,
        Operation {
            span,
            kind: Op::Truthy {
                result: ValueId(6),
                input: ValueId(0),
            },
        },
    );
    f.body.operations.insert(
        3,
        Operation {
            span,
            kind: Op::If {
                result: Some(ValueId(2)),
                condition: ValueId(6),
                consequent: Region {
                    id: RegionId(10),
                    span,
                    operations: vec![Operation {
                        span,
                        kind: Op::MakeClosure {
                            result: ValueId(4),
                            function: FunctionId(1),
                            captures: vec![],
                        },
                    }],
                    exit: Exit::Yield(ValueId(4)),
                },
                alternate: Region {
                    id: RegionId(11),
                    span,
                    operations: vec![Operation {
                        span,
                        kind: Op::MakeClosure {
                            result: ValueId(5),
                            function: FunctionId(2),
                            captures: vec![],
                        },
                    }],
                    exit: Exit::Yield(ValueId(5)),
                },
            },
        },
    );
    module
}

#[test]
fn async_closure_ir_dynamic_targets_preserve_child_and_caller_catches() {
    let mut script = script();
    for (index, (left, right)) in [
        ("await a;return b;", "throw b;"),
        (
            "try{throw await a;}catch(e){return e;}",
            "return await(a?true:false);",
        ),
    ]
    .iter()
    .enumerate()
    {
        for caught in [false, true] {
            let mut module = selected_module(left, right);
            let source = format!(
                "const f=a?async(x,y)=>{{let a=x,b=y;{left}}}:async(x,y)=>{{let a=x,b=y;{right}}};return await f(a,b);"
            );
            let source = if caught {
                protect(&mut module);
                format!("try{{{source}}}catch(e){{return e;}}")
            } else {
                source
            };
            case(
                &mut script,
                &format!("selected-{index}-{caught}"),
                &module,
                &source,
            );
        }
    }
    finish(script);
}

fn mixed_module(kind: usize) -> Module {
    let mut module = call_module("return await b;");
    let signature = module
        .closure_signatures
        .intern_async(vec![Type::General, Type::Number]);
    let f = &mut module.functions[1];
    let span = f.body.span;
    f.slots[0].ty = Type::General;
    f.values = vec![Type::Number, Type::Number, Type::General];
    f.body.operations = vec![
        Operation {
            span,
            kind: Op::AsyncClosureEntry { captures: vec![] },
        },
        Operation {
            span,
            kind: Op::Load {
                result: ValueId(0),
                slot: SlotId(1),
            },
        },
        Operation {
            span,
            kind: Op::Await {
                result: ValueId(1),
                input: ValueId(0),
                rejected: false,
            },
        },
        Operation {
            span,
            kind: Op::Load {
                result: ValueId(2),
                slot: SlotId(0),
            },
        },
    ];
    f.body.exit = Exit::Return(ValueId(2));
    let f = &mut module.functions[0];
    f.values[2] = Type::Closure(signature);
    f.values.push(Type::General);
    let operations = match kind {
        0 | 1 => vec![Operation {
            span,
            kind: Op::Nullish {
                result: ValueId(4),
                value: if kind == 0 {
                    Nullish::Undefined
                } else {
                    Nullish::Null
                },
            },
        }],
        2 => {
            f.values.push(Type::Predicate);
            vec![
                Operation {
                    span,
                    kind: Op::Truthy {
                        result: ValueId(5),
                        input: ValueId(0),
                    },
                },
                Operation {
                    span,
                    kind: Op::Pack {
                        result: ValueId(4),
                        input: ValueId(5),
                    },
                },
            ]
        }
        _ => vec![Operation {
            span,
            kind: Op::Pack {
                result: ValueId(4),
                input: ValueId(0),
            },
        }],
    };
    f.body.operations.splice(2..2, operations);
    let Op::AwaitClosureCall { arguments, .. } = &mut f.body.operations.last_mut().unwrap().kind
    else {
        unreachable!()
    };
    arguments[0] = ValueId(4);
    module
}

#[test]
fn async_closure_ir_typed_argument_frames_keep_general_payload_lanes() {
    let mut script = script();
    for (kind, input) in ["undefined", "null", "!!a", "a"].iter().enumerate() {
        case(
            &mut script,
            &format!("mixed-{kind}"),
            &mixed_module(kind),
            &format!("const f=async(x,y)=>{{await y;return x;}};return await f({input},b);"),
        );
    }
    finish(script);
}

fn promise_argument_module(kind: usize, settled: Option<bool>) -> Module {
    let mut module = mixed_module(kind);
    let ty = if settled.is_some() {
        Type::SettledPromise
    } else {
        Type::FulfilledPromise
    };
    let signature = module
        .closure_signatures
        .intern_async(vec![ty, Type::Number]);
    let f = &mut module.functions[1];
    let span = f.body.span;
    f.slots[0].ty = ty;
    f.values[2] = ty;
    if settled.is_some() {
        f.values.push(Type::General);
        f.body.operations.push(Operation {
            span,
            kind: Op::AwaitSettled {
                result: ValueId(3),
                input: ValueId(2),
            },
        });
    } else {
        f.values.extend([Type::General, Type::General]);
        f.body.operations.extend([
            Operation {
                span,
                kind: Op::PromiseValue {
                    result: ValueId(3),
                    input: ValueId(2),
                },
            },
            Operation {
                span,
                kind: Op::Await {
                    result: ValueId(4),
                    input: ValueId(3),
                    rejected: false,
                },
            },
        ]);
    }
    f.body.exit = Exit::Return(ValueId(if settled.is_some() { 3 } else { 4 }));
    let f = &mut module.functions[0];
    f.values[2] = Type::Closure(signature);
    let result = ValueId(f.values.len());
    f.values.push(ty);
    let operation = if let Some(rejected) = settled {
        Op::SettlePromise {
            result,
            body: Region {
                id: RegionId(20),
                span,
                operations: vec![],
                exit: if rejected {
                    Exit::Throw(ValueId(4))
                } else {
                    Exit::Return(ValueId(4))
                },
            },
        }
    } else {
        Op::FulfillPromise {
            result,
            input: ValueId(4),
        }
    };
    let at = f.body.operations.len() - 1;
    f.body.operations.insert(
        at,
        Operation {
            span,
            kind: operation,
        },
    );
    let Op::AwaitClosureCall { arguments, .. } = &mut f.body.operations.last_mut().unwrap().kind
    else {
        unreachable!()
    };
    arguments[0] = result;
    module
}

#[test]
fn async_closure_ir_promise_argument_frames_preserve_logical_kinds_and_status() {
    let mut script = script();
    for (kind, input) in ["undefined", "null", "!!a", "a"].iter().enumerate() {
        for settled in [None, Some(false), Some(true)] {
            let constructor = if settled == Some(true) {
                "reject"
            } else {
                "resolve"
            };
            case(
                &mut script,
                &format!("promise-{kind}-{settled:?}"),
                &promise_argument_module(kind, settled),
                &format!(
                    "const f=async(p,y)=>{{await y;return await p;}};return await f(Promise.{constructor}({input}),b);"
                ),
            );
        }
    }
    finish(script);
}

fn callback_module(asynchronous: bool) -> Module {
    let mut module = call_module("return await b;");
    let inner = if asynchronous {
        module.closure_signatures.intern_async(vec![])
    } else {
        0
    };
    let signature = module
        .closure_signatures
        .intern_async(vec![Type::Closure(inner), Type::Number]);
    let span = module.functions[0].body.span;
    let op = |kind| Operation { span, kind };
    let slot = |ty, parameter| Slot {
        ty,
        owner: RegionId(0),
        mutable: true,
        parameter,
        span,
    };
    let mut operations = vec![
        op(Op::AsyncClosureEntry { captures: vec![] }),
        op(Op::Load {
            result: ValueId(0),
            slot: SlotId(0),
        }),
        op(Op::Load {
            result: ValueId(1),
            slot: SlotId(1),
        }),
        op(Op::Await {
            result: ValueId(2),
            input: ValueId(1),
            rejected: false,
        }),
    ];
    let (values, exit) = if asynchronous {
        operations.push(op(Op::AwaitClosureCall {
            result: ValueId(3),
            callee: ValueId(0),
            arguments: vec![],
        }));
        (
            vec![
                Type::Closure(inner),
                Type::Number,
                Type::Number,
                Type::General,
            ],
            Exit::Return(ValueId(3)),
        )
    } else {
        operations.extend([
            op(Op::ClosureCall {
                result: ValueId(3),
                callee: ValueId(0),
                arguments: vec![],
            }),
            op(Op::Pack {
                result: ValueId(4),
                input: ValueId(3),
            }),
        ]);
        (
            vec![
                Type::Closure(inner),
                Type::Number,
                Type::Number,
                Type::Number,
                Type::General,
            ],
            Exit::Return(ValueId(4)),
        )
    };
    module.functions[1] = Function {
        return_type: Type::General,
        parameter_count: 2,
        slots: vec![slot(Type::Closure(inner), true), slot(Type::Number, true)],
        values,
        body: Region {
            id: RegionId(0),
            span,
            operations,
            exit,
        },
    };
    let (operations, values, exit, return_type) = if asynchronous {
        (
            vec![
                op(Op::AsyncClosureEntry {
                    captures: vec![SlotId(0)],
                }),
                op(Op::Load {
                    result: ValueId(0),
                    slot: SlotId(0),
                }),
                op(Op::Await {
                    result: ValueId(1),
                    input: ValueId(0),
                    rejected: false,
                }),
                op(Op::Pack {
                    result: ValueId(2),
                    input: ValueId(1),
                }),
            ],
            vec![Type::Number, Type::Number, Type::General],
            Exit::Return(ValueId(2)),
            Type::General,
        )
    } else {
        (
            vec![
                op(Op::ClosureEntry {
                    captures: vec![SlotId(0)],
                }),
                op(Op::Load {
                    result: ValueId(0),
                    slot: SlotId(0),
                }),
            ],
            vec![Type::Number],
            Exit::Return(ValueId(0)),
            Type::Number,
        )
    };
    module.functions.push(Function {
        return_type,
        parameter_count: 0,
        slots: vec![slot(Type::Number, false)],
        values,
        body: Region {
            id: RegionId(0),
            span,
            operations,
            exit,
        },
    });
    let root = &mut module.functions[0];
    root.values[2] = Type::Closure(signature);
    root.values.push(Type::Closure(inner));
    root.body.operations.insert(
        2,
        op(Op::MakeClosure {
            result: ValueId(4),
            function: FunctionId(2),
            captures: vec![SlotId(0)],
        }),
    );
    root.body.operations.insert(
        4,
        op(Op::Store {
            slot: SlotId(0),
            value: ValueId(1),
        }),
    );
    let Op::AwaitClosureCall { arguments, .. } = &mut root.body.operations.last_mut().unwrap().kind
    else {
        unreachable!()
    };
    arguments[0] = ValueId(4);
    module
}

#[test]
fn async_closure_ir_callback_parameters_keep_code_environment_and_effect() {
    let mut script = script();
    for asynchronous in [false, true] {
        let source = if asynchronous {
            "let x=a;const g=async()=>await x;const f=async(fn,y)=>{await y;return await fn();};x=b;return await f(g,b);"
        } else {
            "let x=a;const g=()=>x;const f=async(fn,y)=>{await y;return fn();};x=b;return await f(g,b);"
        };
        case(
            &mut script,
            &format!("callback-{asynchronous}"),
            &callback_module(asynchronous),
            source,
        );
    }
    finish(script);
}

#[test]
fn async_closure_ir_synchronous_factory_returns_an_async_function_pair() {
    let mut module = call_module("return await b;");
    let Type::Closure(inner) = module.functions[0].values[2] else {
        unreachable!()
    };
    let factory = module
        .closure_signatures
        .intern(vec![], Type::Closure(inner));
    let span = module.functions[0].body.span;
    module.functions.push(Function {
        return_type: Type::Closure(inner),
        parameter_count: 0,
        slots: vec![],
        values: vec![Type::Closure(inner)],
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
                    kind: Op::MakeClosure {
                        result: ValueId(0),
                        function: FunctionId(1),
                        captures: vec![],
                    },
                },
            ],
            exit: Exit::Return(ValueId(0)),
        },
    });
    let root = &mut module.functions[0];
    root.values.push(Type::Closure(factory));
    root.body.operations.splice(
        2..3,
        [
            Operation {
                span,
                kind: Op::MakeClosure {
                    result: ValueId(4),
                    function: FunctionId(2),
                    captures: vec![],
                },
            },
            Operation {
                span,
                kind: Op::ClosureCall {
                    result: ValueId(2),
                    callee: ValueId(4),
                    arguments: vec![],
                },
            },
        ],
    );
    let mut script = script();
    case(
        &mut script,
        "factory",
        &module,
        "const make=()=>async(x,y)=>await y;const f=make();return await f(a,b);",
    );
    finish(script);
}

fn wide_module(count: usize, first: bool) -> Module {
    let mut module = call_module("return await b;");
    let signature = module
        .closure_signatures
        .intern_async(vec![Type::Number; count]);
    let f = &mut module.functions[1];
    let span = f.body.span;
    f.parameter_count = count;
    f.slots.resize_with(count, || Slot {
        ty: Type::Number,
        owner: RegionId(0),
        mutable: true,
        parameter: true,
        span,
    });
    for operation in &mut f.body.operations {
        if let Op::Load { slot, .. } = &mut operation.kind
            && *slot == SlotId(1)
        {
            *slot = SlotId(if first { 0 } else { count - 1 });
        }
    }
    let f = &mut module.functions[0];
    f.values[2] = Type::Closure(signature);
    let Op::AwaitClosureCall { arguments, .. } = &mut f.body.operations.last_mut().unwrap().kind
    else {
        unreachable!()
    };
    *arguments = vec![ValueId(0); count];
    arguments[count - 1] = ValueId(1);
    module
}

#[test]
fn async_closure_ir_frame_limits_count_logical_sites_and_parameters() {
    let mut script = script();
    for first in [false, true] {
        case(
            &mut script,
            &format!("wide-{first}"),
            &wide_module(1000, first),
            &format!(
                "const values=Array(1000).fill(a);values[999]=b;const f=async(...p)=>await p[{}];return await f(...values);",
                if first { 0 } else { 999 }
            ),
        );
    }
    assert!(verify_module_with_entry_type(&wide_module(1001, false), Type::General).is_err());
    let body = format!("{}return b;", "await a;".repeat(31));
    case(
        &mut script,
        "await-32",
        &call_module(&body),
        &format!("return await(async()=>{{{body}}})();"),
    );
    let body = format!("{}return b;", "await a;".repeat(32));
    assert!(
        crate::emit::emit_async(&call_module(&body))
            .unwrap_err()
            .contains("32 await sites")
    );
    finish(script);
}

#[test]
fn async_closure_ir_arena_quota_stops_unbounded_frame_growth() {
    let bytes = wat::parse_str(crate::emit::emit_async(&recursive_module(false)).unwrap()).unwrap();
    finish(format!(
        "{}{{const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from({bytes:?}))).exports.start;assert.throws(()=>run(1000000,2),WebAssembly.RuntimeError);}}\n",
        script()
    ));
}

#[test]
fn async_closure_ir_malformed_effects_inputs_and_bodies_reject() {
    for change in 0..10 {
        let mut module = call_module("return await b;");
        match change {
            0 => {
                module
                    .closure_signatures
                    .higher
                    .last_mut()
                    .unwrap()
                    .asynchronous = false
            }
            1 => module.closure_signatures.higher.last_mut().unwrap().result = Type::Number,
            2 => module.functions[1].return_type = Type::Number,
            3 => {
                module.functions[1].body.operations[0].kind = Op::ClosureEntry { captures: vec![] }
            }
            4 => {
                let f = &mut module.functions[0];
                let operation = f.body.operations.last_mut().unwrap();
                let Op::AwaitClosureCall {
                    result,
                    callee,
                    arguments,
                } = &operation.kind
                else {
                    unreachable!()
                };
                operation.kind = Op::ClosureCall {
                    result: *result,
                    callee: *callee,
                    arguments: arguments.clone(),
                };
            }
            5 => {
                let f = &mut module.functions[0];
                let Op::AwaitClosureCall { result, .. } = f.body.operations.last().unwrap().kind
                else {
                    unreachable!()
                };
                f.values[result.0] = Type::Number;
            }
            6 => {
                let Op::AwaitClosureCall { arguments, .. } =
                    &mut module.functions[0].body.operations.last_mut().unwrap().kind
                else {
                    unreachable!()
                };
                arguments.clear();
            }
            7 => {
                let Op::AwaitClosureCall { callee, .. } =
                    &mut module.functions[0].body.operations.last_mut().unwrap().kind
                else {
                    unreachable!()
                };
                *callee = ValueId(0);
            }
            8 => {
                let f = &mut module.functions[1];
                f.body.operations.swap(0, 1);
            }
            _ => {
                let f = &mut module.functions[0];
                let operation = f.body.operations.pop().unwrap();
                let span = f.body.span;
                let result = ValueId(f.values.len());
                f.values.push(Type::SettledPromise);
                f.body.operations.push(Operation {
                    span,
                    kind: Op::SettlePromise {
                        result,
                        body: Region {
                            id: RegionId(10),
                            span,
                            operations: vec![operation],
                            exit: Exit::Return(ValueId(3)),
                        },
                    },
                });
            }
        }
        let error = verify_module_with_entry_type(&module, Type::General).unwrap_err();
        if change == 9 {
            assert!(
                error.contains("settlement bodies cannot suspend"),
                "{error}"
            );
        }
    }
    let mut normal = call_module("return b;");
    let signature = normal
        .closure_signatures
        .intern(vec![Type::Number, Type::Number], Type::General);
    normal.functions[0].values[2] = Type::Closure(signature);
    normal.functions[1].body.operations[0].kind = Op::ClosureEntry { captures: vec![] };
    assert!(
        verify_module_with_entry_type(&normal, Type::General)
            .unwrap_err()
            .contains("closure call effect does not match signature")
    );
    assert!(crate::emit::emit_with_entry_type(&call_module("return b;"), Type::General).is_err());
}

fn recursive_module(captured: bool) -> Module {
    let mut module = call_module("return await b;");
    let signature = module
        .closure_signatures
        .intern_async(vec![Type::Number, Type::Number]);
    let span = module.functions[1].body.span;
    let op = |kind| Operation { span, kind };
    let mut operations = vec![op(Op::AsyncClosureEntry { captures: vec![] })];
    if captured {
        operations.push(op(Op::CaptureCells {
            cells: vec![SlotId(0)],
        }));
    }
    operations.extend([
        op(Op::Load {
            result: ValueId(0),
            slot: SlotId(0),
        }),
        op(Op::Load {
            result: ValueId(1),
            slot: SlotId(1),
        }),
        op(Op::Constant {
            result: ValueId(2),
            value: 0.0,
        }),
        op(Op::Compare {
            result: ValueId(3),
            operator: Compare::Greater,
            left: ValueId(0),
            right: ValueId(2),
        }),
        op(Op::If {
            result: None,
            condition: ValueId(3),
            consequent: Region {
                id: RegionId(1),
                span,
                operations: vec![
                    op(Op::Constant {
                        result: ValueId(4),
                        value: 1.0,
                    }),
                    op(Op::Binary {
                        result: ValueId(5),
                        operator: Binary::Subtract,
                        left: ValueId(0),
                        right: ValueId(4),
                    }),
                    op(Op::MakeClosure {
                        result: ValueId(6),
                        function: FunctionId(1),
                        captures: vec![],
                    }),
                    op(Op::AwaitClosureCall {
                        result: ValueId(7),
                        callee: ValueId(6),
                        arguments: vec![ValueId(5), ValueId(1)],
                    }),
                    op(Op::NumberFromGeneral {
                        result: ValueId(8),
                        input: ValueId(7),
                    }),
                    op(Op::Load {
                        result: ValueId(9),
                        slot: SlotId(0),
                    }),
                    op(Op::Binary {
                        result: ValueId(10),
                        operator: Binary::Add,
                        left: ValueId(8),
                        right: ValueId(if captured { 9 } else { 0 }),
                    }),
                    op(Op::Pack {
                        result: ValueId(11),
                        input: ValueId(10),
                    }),
                ],
                exit: Exit::Return(ValueId(11)),
            },
            alternate: Region {
                id: RegionId(2),
                span,
                operations: vec![
                    op(Op::Pack {
                        result: ValueId(12),
                        input: ValueId(1),
                    }),
                    op(Op::Await {
                        result: ValueId(13),
                        input: ValueId(12),
                        rejected: false,
                    }),
                ],
                exit: Exit::Return(ValueId(13)),
            },
        }),
    ]);
    let f = &mut module.functions[1];
    f.values = vec![
        Type::Number,
        Type::Number,
        Type::Number,
        Type::Predicate,
        Type::Number,
        Type::Number,
        Type::Closure(signature),
        Type::General,
        Type::Number,
        Type::Number,
        Type::Number,
        Type::General,
        Type::General,
        Type::General,
    ];
    f.body.operations = operations;
    f.body.exit = Exit::Fallthrough;
    module
}

#[test]
fn async_closure_ir_recursive_activations_restore_snapshots_and_cell_pointers() {
    let mut script = script();
    for captured in [false, true] {
        case(
            &mut script,
            &format!("recursive-{captured}"),
            &recursive_module(captured),
            "const f=async(x,y)=>{if(x>0){const r=await f(x-1,y);return +r+x;}return await y;};return await f(a,b);",
        );
    }
    finish(script);
}

#[test]
fn async_closure_ir_recursive_rejection_reaches_each_caller_reaction() {
    let mut script = script();
    for caught in [false, true] {
        let mut module = recursive_module(true);
        let Op::If { alternate, .. } =
            &mut module.functions[1].body.operations.last_mut().unwrap().kind
        else {
            unreachable!()
        };
        for operation in &mut alternate.operations {
            if let Op::Await { rejected, .. } = &mut operation.kind {
                *rejected = true;
            }
        }
        let source = "const f=async(x,y)=>{if(x>0)return await f(x-1,y);return await Promise.reject(y);};return await f(a,b);";
        let source = if caught {
            protect(&mut module);
            format!("try{{{source}}}catch(e){{return e;}}")
        } else {
            source.to_string()
        };
        case(
            &mut script,
            &format!("recursive-rejection-{caught}"),
            &module,
            &source,
        );
    }
    finish(script);
}

#[test]
fn async_closure_ir_nested_rejection_keeps_the_primitive_reason() {
    let mut module = call_module("return await Promise.reject(undefined);");
    let mut wrapper = call_module("return await b;").functions.remove(0);
    let span = wrapper.body.span;
    wrapper.body.operations.insert(
        0,
        Operation {
            span,
            kind: Op::AsyncClosureEntry { captures: vec![] },
        },
    );
    for operation in &mut wrapper.body.operations {
        if let Op::MakeClosure { function, .. } = &mut operation.kind {
            *function = FunctionId(2);
        }
    }
    module.functions.insert(1, wrapper);
    let mut script = script();
    case(
        &mut script,
        "nested-rejection",
        &module,
        "return await(async(x,y)=>{return await(async()=>await Promise.reject(undefined))();})(a,b);",
    );
    finish(script);
}

#[test]
fn async_closure_ir_nested_same_signature_calls_keep_separate_frames() {
    let mut module = call_module("return await b;");
    let mut wrapper = call_module("return await b;").functions.remove(0);
    let span = wrapper.body.span;
    wrapper.body.operations.insert(
        0,
        Operation {
            span,
            kind: Op::AsyncClosureEntry { captures: vec![] },
        },
    );
    for op in &mut wrapper.body.operations {
        if let Op::MakeClosure { function, .. } = &mut op.kind {
            *function = FunctionId(2);
        }
    }
    module.functions.insert(1, wrapper);
    let mut script = script();
    case(
        &mut script,
        "nested",
        &module,
        "return await(async(x,y)=>{return await(async()=>{return await y;})();})(a,b);",
    );
    finish(script);
}

#[test]
fn async_closure_ir_environment_imports_alias_the_call_time_cell() {
    let mut module = call_module("await a;return b;");
    let signature = module.closure_signatures.intern_async(vec![Type::Number]);
    let f = &mut module.functions[1];
    f.parameter_count = 1;
    f.slots[1].parameter = false;
    let Op::AsyncClosureEntry { captures } = &mut f.body.operations[0].kind else {
        unreachable!()
    };
    *captures = vec![SlotId(1)];
    let root = &mut module.functions[0];
    let span = root.body.span;
    root.slots.push(Slot {
        ty: Type::Number,
        owner: RegionId(0),
        mutable: true,
        parameter: false,
        span,
    });
    root.values[2] = Type::Closure(signature);
    root.body.operations.insert(
        2,
        Operation {
            span,
            kind: Op::Initialize {
                slot: SlotId(2),
                value: ValueId(0),
            },
        },
    );
    let Op::MakeClosure { captures, .. } = &mut root.body.operations[3].kind else {
        unreachable!()
    };
    *captures = vec![SlotId(2)];
    root.body.operations.insert(
        4,
        Operation {
            span,
            kind: Op::Store {
                slot: SlotId(2),
                value: ValueId(1),
            },
        },
    );
    let Op::AwaitClosureCall { arguments, .. } = &mut root.body.operations[5].kind else {
        unreachable!()
    };
    *arguments = vec![ValueId(0)];
    let mut script = script();
    case(
        &mut script,
        "external-capture",
        &module,
        "let x=a;const f=async y=>{await y;return x;};x=b;return await f(a);",
    );
    finish(script);
}
