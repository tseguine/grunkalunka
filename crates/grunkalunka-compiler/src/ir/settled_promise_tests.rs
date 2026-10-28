fn async_script() -> String {
    let adapter = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("grunkalunka-runtime/js/async.mjs");
    let mut script = format!(
        r#"
const assert=require('node:assert/strict');
(async()=>{{
const{{createAsyncFunction}}=await import(require('node:url').pathToFileURL({:?}));
const view=new DataView(new ArrayBuffer(8));
function encode(value){{if(value===undefined)return'undefined';if(value===null)return'null';if(typeof value==='boolean')return'boolean:'+value;assert.equal(typeof value,'number');if(Number.isNaN(value))return'nan';view.setFloat64(0,value);return view.getBigUint64(0).toString(16);}}
async function observe(run,a,b){{const trace=[];const p=run(a,b);trace.push('returned');const done=p.then(v=>trace.push('fulfilled:'+encode(v)),e=>trace.push('rejected:'+encode(e)));const markers=new Promise(resolve=>{{let n=0;function tick(){{trace.push('tick'+(++n));if(n<34)queueMicrotask(tick);else resolve();}}queueMicrotask(tick);}});await Promise.all([done,markers]);return trace;}}
"#,
        adapter.to_string_lossy()
    );
    script.push_str(RECEIPT);
    script
}

fn async_case(script: &mut String, name: &str, module: &Module, source: &str) {
    let bytes = wat::parse_str(crate::emit::emit_async(module).unwrap()).unwrap();
    wasm_record(name, "async", &bytes);
    script.push_str(&format!("{{const native=async(a,b)=>{{{source}}};const wasm=createAsyncFunction(Uint8Array.from({bytes:?}));let index=0;for(const[a,b]of[[4,2],[0,-0],[-0,-2],[NaN,Infinity]]){{const expected=await observe(native,a,b);assert.deepEqual(await observe(wasm,a,b),expected,{source:?});if({})receipt({name:?},index,expected);index++;}}}}\n",proof()));
}

fn finish_async(mut script: String) {
    script.push_str("})().catch(e=>{console.error(e);process.exitCode=1;});\n");
    node(&script);
}

use super::*;

#[test]
fn settled_extended_kind_survives_status_and_payload_projection() {
    let cycle = "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();";
    let mut script = async_script();
    for rejected in [false, true] {
        let mut module = crate::lower_source_profile(
            &format!(
                "export async function run(a,b){{{cycle}try{{await p;}}catch(e){{return e;}}}}"
            ),
            false,
            Type::General,
            true,
        )
        .unwrap();
        let f = &mut module.functions[0];
        let base = f.values.len();
        f.values.extend([
            Type::SettledPromise,
            Type::General,
            Type::Predicate,
            Type::Predicate,
            Type::Predicate,
            Type::General,
            Type::Predicate,
            Type::General,
            Type::General,
        ]);
        let [
            snapshot,
            payload,
            equal,
            status,
            not_status,
            packed_equal,
            no,
            packed_no,
            result,
        ] = std::array::from_fn::<_, 9, _>(|i| ValueId(base + i));
        let handler = f
            .body
            .operations
            .iter_mut()
            .find_map(|op| {
                if let Op::TryCatch { handler, .. } = &mut op.kind {
                    Some(handler)
                } else {
                    None
                }
            })
            .unwrap();
        let Exit::Return(error) = handler.exit else {
            panic!("Error handler return");
        };
        let span = handler.span;
        let mut push = |kind| handler.operations.push(Operation { span, kind });
        push(Op::SettlePromise {
            result: snapshot,
            body: Region {
                id: RegionId(10000),
                span,
                operations: vec![],
                exit: if rejected {
                    Exit::Throw(error)
                } else {
                    Exit::Return(error)
                },
            },
        });
        push(Op::SettledValue {
            result: payload,
            input: snapshot,
        });
        push(Op::PrimitiveStrictEqual {
            result: equal,
            left: error,
            right: payload,
            negated: false,
        });
        push(Op::PromiseRejected {
            result: status,
            input: snapshot,
        });
        push(Op::Not {
            result: not_status,
            input: status,
        });
        push(Op::Pack {
            result: packed_equal,
            input: equal,
        });
        push(Op::Boolean {
            result: no,
            value: false,
        });
        push(Op::Pack {
            result: packed_no,
            input: no,
        });
        push(Op::If {
            result: Some(result),
            condition: if rejected { status } else { not_status },
            consequent: Region {
                id: RegionId(10001),
                span,
                operations: vec![],
                exit: Exit::Yield(packed_equal),
            },
            alternate: Region {
                id: RegionId(10002),
                span,
                operations: vec![],
                exit: Exit::Yield(packed_no),
            },
        });
        handler.exit = Exit::Return(result);
        let native = format!(
            "{cycle}try{{await p;}}catch(e){{try{{if({rejected})throw e;return e===e;}}catch(x){{return x===e;}}}}"
        );
        async_case(
            &mut script,
            &format!("settled-error-project-{rejected}"),
            &module,
            &native,
        );
    }
    finish_async(script);
}

#[test]
fn settled_promise_signatures_are_logically_distinct_private_values() {
    let mut signatures = ClosureSignatures::default();
    let settled = signatures.intern(vec![Type::SettledPromise], Type::SettledPromise);
    let fulfilled = signatures.intern(vec![Type::FulfilledPromise], Type::FulfilledPromise);
    let general = signatures.intern(vec![Type::General], Type::General);
    assert_ne!(settled, fulfilled);
    assert_ne!(settled, general);
    assert!(signatures.is_callback_signature(settled));
    assert!(signatures.is_returned_function(settled));
}

fn capture_module() -> Module {
    capture_source("return b;")
}

fn capture_source(source_body: &str) -> Module {
    let mut module = crate::lower_source_profile(
        &format!("export async function run(a,b){{{source_body}}}"),
        false,
        Type::General,
        true,
    )
    .unwrap();
    let function = &mut module.functions[0];
    let span = function.body.span;
    let mut body = std::mem::replace(
        &mut function.body,
        Region {
            id: RegionId(0),
            span,
            operations: vec![],
            exit: Exit::Fallthrough,
        },
    );
    body.id = RegionId(100);
    for slot in &mut function.slots {
        if slot.owner == RegionId(0) && !slot.parameter {
            slot.owner = body.id;
        }
    }
    let promise = ValueId(function.values.len());
    function.values.push(Type::SettledPromise);
    let payload = ValueId(function.values.len());
    function.values.push(Type::General);
    function.body.operations = vec![
        Operation {
            span,
            kind: Op::SettlePromise {
                result: promise,
                body,
            },
        },
        Operation {
            span,
            kind: Op::SettledValue {
                result: payload,
                input: promise,
            },
        },
    ];
    function.body.exit = Exit::Return(payload);
    module
}

fn status_result(module: &mut Module) {
    let f = &mut module.functions[0];
    let span = f.body.span;
    let Op::SettlePromise { result: input, .. } = f.body.operations[0].kind else {
        unreachable!()
    };
    let rejected = ValueId(f.values.len());
    f.values.push(Type::Predicate);
    let packed = ValueId(f.values.len());
    f.values.push(Type::General);
    f.body.operations.push(Operation {
        span,
        kind: Op::PromiseRejected {
            result: rejected,
            input,
        },
    });
    f.body.operations.push(Operation {
        span,
        kind: Op::Pack {
            result: packed,
            input: rejected,
        },
    });
    f.body.exit = Exit::Return(packed);
}

fn node(script: &str) {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
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

fn proof() -> bool {
    std::env::var_os("GRUNKALUNKA_SETTLED_PROOFS").is_some()
}

fn wasm_record(name: &str, profile: &str, bytes: &[u8]) {
    if proof() {
        let hex = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        println!("IR_SETTLED_WASM|{name}|{profile}|{hex}");
    }
}

const RECEIPT: &str = r#"
function receipt(name,index,trace){
 const at=trace.findIndex(x=>x.startsWith('fulfilled:')||x.startsWith('rejected:'));
 const line=trace[at],cut=line.indexOf(':'),kind=line.slice(0,cut),value=line.slice(cut+1);
 let tag,bits;const d=new DataView(new ArrayBuffer(8));
 if(value==='undefined'){tag=0;bits='0000000000000000';}
 else if(value==='null'){tag=1;bits='0000000000000000';}
 else if(value.startsWith('boolean:')){tag=2;d.setFloat64(0,value==='boolean:true'?1:0);bits=d.getBigUint64(0).toString(16).padStart(16,'0');}
 else{tag=3;bits=value==='nan'?'nan':value.padStart(16,'0');}
 console.log('IR_SETTLED_OBS|'+name+'|'+index+'|'+kind+'|'+tag+'|'+bits+'|'+trace.slice(0,at).filter(x=>x.startsWith('tick')).length);
}
"#;

#[test]
fn settled_promise_capture_executes_payload_status_and_lexical_catches() {
    let mut script = String::from("const assert=require('node:assert/strict');\n");
    for (case, body) in [
        "return b;",
        "return undefined;",
        "return null;",
        "return a?true:false;",
        "if(a)throw b;return null;",
        "try{if(a)throw b;return false;}catch(e){return e;}",
        "try{throw a;}catch(e){throw b;}",
        "let x=a;while(x>0){x--;if(x<2)throw b;}return x;",
        "let x=a;do{if(x)break;return b;}while(false);return x;",
        "const f=x=>x+b;return f(a);",
        "try{throw a;}catch(e){const f=()=>e;return f();}",
    ]
    .into_iter()
    .enumerate()
    {
        for status in [false, true] {
            let mut module = capture_source(body);
            if status {
                status_result(&mut module);
            }
            let wat = crate::emit::emit_with_entry_type(&module, Type::General).unwrap();
            let bytes = wat::parse_str(wat).unwrap();
            let name = format!("capture-{case}-{status}");
            wasm_record(&name, "general", &bytes);
            script.push_str(&format!("{{const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from({bytes:?}))).exports.run;let index=0;for(const[a,b]of[[4,2],[0,-0],[-0,-2],[NaN,Infinity]]){{let value,rejected=false;try{{value=(function(){{{body}}})();}}catch(e){{value=e;rejected=true;}}const expected={status}?rejected:value;const[tag,payload]=run(a,b);const actual=tag===0?undefined:tag===1?null:tag===2?!!payload:payload;assert(tag>=0&&tag<=3,'rejection status leaked into General');assert(Object.is(actual,expected),{body:?}+' status='+{status});if({}){{const d=new DataView(new ArrayBuffer(8)),tag=expected===undefined?0:expected===null?1:typeof expected==='boolean'?2:3;d.setFloat64(0,tag<2?0:tag===2?+expected:expected);const bits=Number.isNaN(expected)?'nan':d.getBigUint64(0).toString(16).padStart(16,'0');console.log('IR_SETTLED_OBS|'+{name:?}+'|'+index+'|fulfilled|'+tag+'|'+bits+'|0');}}index++;}}}}\n",proof()));
        }
    }
    node(&script);
}

#[test]
fn settled_promise_capture_rejects_suspension_inside_the_closed_body() {
    let mut module = capture_module();
    verify_module_with_entry_type(&module, Type::General).unwrap();
    let function = &mut module.functions[0];
    let result = ValueId(function.values.len());
    function.values.push(Type::General);
    let Op::SettlePromise { body, .. } = &mut function.body.operations[0].kind else {
        unreachable!()
    };
    let Exit::Return(input) = body.exit else {
        unreachable!()
    };
    body.operations.push(Operation {
        span: body.span,
        kind: Op::Await {
            result,
            input,
            rejected: false,
        },
    });
    assert!(verify_module_with_entry_type(&module, Type::General).is_err());
}

#[test]
fn settled_promise_capture_emits_a_checked_synchronous_boundary() {
    let mut module = capture_module();
    let function = &mut module.functions[0];
    let Op::SettlePromise { body, .. } = &mut function.body.operations[0].kind else {
        unreachable!()
    };
    let Exit::Return(value) = body.exit else {
        unreachable!()
    };
    body.exit = Exit::Throw(value);
    let wat = crate::emit::emit_with_entry_type(&module, Type::General).unwrap();
    wat::parse_str(wat).unwrap();
}

fn await_capture(body_source: &str, caught: bool) -> Module {
    let mut module = capture_source(body_source);
    let f = &mut module.functions[0];
    let span = f.body.span;
    let Op::SettlePromise { result: input, .. } = f.body.operations[0].kind else {
        unreachable!()
    };
    let result = ValueId(f.values.len());
    f.values.push(Type::General);
    let awaited = Operation {
        span,
        kind: Op::AwaitSettled { result, input },
    };
    if caught {
        let reason = ValueId(f.values.len());
        f.values.push(Type::General);
        f.body.operations.push(Operation {
            span,
            kind: Op::TryCatch {
                body: Region {
                    id: RegionId(200),
                    span,
                    operations: vec![awaited],
                    exit: Exit::Return(result),
                },
                reason,
                handler: Region {
                    id: RegionId(201),
                    span,
                    operations: vec![],
                    exit: Exit::Return(reason),
                },
            },
        });
        f.body.exit = Exit::Fallthrough;
    } else {
        f.body.operations.push(awaited);
        f.body.exit = Exit::Return(result);
    }
    module
}

#[test]
fn settled_promise_await_uses_the_existing_async_protocol() {
    let module = await_capture("if(a)throw b;return null;", false);
    let wat = crate::emit::emit_async(&module).unwrap();
    wat::parse_str(wat).unwrap();
}

#[test]
fn settled_promise_await_matches_native_values_rejections_and_reaction_order() {
    let mut script = async_script();
    for (case, body) in [
        "return b;",
        "return undefined;",
        "return null;",
        "return a?true:false;",
        "throw b;",
        "throw undefined;",
        "throw null;",
        "throw a?false:true;",
        "if(a)throw b;return null;",
        "try{if(a)throw b;return false;}catch(e){return e;}",
        "try{throw a;}catch(e){throw b;}",
    ]
    .into_iter()
    .enumerate()
    {
        for caught in [false, true] {
            let module = await_capture(body, caught);
            let name = format!("await-{case}-{caught}");
            let awaited = format!("return await(async()=>{{{body}}})();");
            let source = if caught {
                format!("try{{{awaited}}}catch(e){{return e;}}")
            } else {
                awaited
            };
            async_case(&mut script, &name, &module, &source);
        }
    }
    finish_async(script);
}

fn settled_region(
    region: &mut Region,
    rejected: bool,
    payloads: &std::collections::BTreeMap<ValueId, ValueId>,
    next_region: &mut usize,
) {
    for operation in &mut region.operations {
        match &mut operation.kind {
            Op::FulfillPromise { result, input } => {
                let (result, input) = (*result, *input);
                let id = RegionId(*next_region);
                *next_region += 1;
                operation.kind = Op::SettlePromise {
                    result,
                    body: Region {
                        id,
                        span: operation.span,
                        operations: vec![],
                        exit: if rejected {
                            Exit::Throw(input)
                        } else {
                            Exit::Return(input)
                        },
                    },
                };
            }
            Op::PromiseValue { result, input } => {
                operation.kind = Op::SettledValue {
                    result: *result,
                    input: *input,
                }
            }
            Op::Await {
                result,
                input,
                rejected: old_rejection,
            } if payloads.contains_key(input) => {
                assert!(!*old_rejection);
                operation.kind = Op::AwaitSettled {
                    result: *result,
                    input: payloads[input],
                };
            }
            Op::InlineCall { body, .. } | Op::AwaitCall { body, .. } | Op::Block(body) => {
                settled_region(body, rejected, payloads, next_region)
            }
            Op::TryCatch { body, handler, .. } => {
                settled_region(body, rejected, payloads, next_region);
                settled_region(handler, rejected, payloads, next_region);
            }
            Op::If {
                consequent,
                alternate,
                ..
            } => {
                settled_region(consequent, rejected, payloads, next_region);
                settled_region(alternate, rejected, payloads, next_region);
            }
            Op::While {
                condition,
                body,
                step,
            } => {
                settled_region(condition, rejected, payloads, next_region);
                settled_region(body, rejected, payloads, next_region);
                if let Some(step) = step {
                    settled_region(step, rejected, payloads, next_region);
                }
            }
            Op::DoWhile { body, condition } => {
                settled_region(body, rejected, payloads, next_region);
                settled_region(condition, rejected, payloads, next_region);
            }
            _ => (),
        }
    }
}

fn settled_seed(body: &str, rejected: bool, typescript: bool) -> Module {
    let mut module = crate::lower_source_profile(
        &format!("export async function run(a,b){{{body}}}"),
        typescript,
        Type::General,
        true,
    )
    .unwrap();
    let settle_type = |ty: &mut Type| {
        if *ty == Type::FulfilledPromise {
            *ty = Type::SettledPromise;
        }
    };
    for signature in &mut module.closure_signatures.higher {
        settle_type(&mut signature.result);
        for ty in &mut signature.parameters {
            settle_type(ty);
        }
    }
    for function in &mut module.functions {
        let mut payloads = std::collections::BTreeMap::new();
        visit_operations(&function.body, &mut |op| {
            if let Op::PromiseValue { result, input } = op {
                payloads.insert(*result, *input);
            }
        });
        settle_type(&mut function.return_type);
        for slot in &mut function.slots {
            settle_type(&mut slot.ty);
        }
        for ty in &mut function.values {
            settle_type(ty);
        }
        settled_region(&mut function.body, rejected, &payloads, &mut 10000);
    }
    module
}

#[test]
fn settled_promise_private_calls_frames_and_captures_match_native_reactions() {
    let mut script = async_script();
    for (case, body) in [
        "let f=x=>Promise.resolve(x);return await f(b);",
        "const apply=(fn,x)=>fn(x);let f=x=>Promise.resolve(x);return await apply(f,b);",
        "const take=a?p=>p:p=>p;return await take(Promise.resolve(b));",
        "let make=x=>()=>Promise.resolve(x);const f=make(b);return await f();",
        "const p=Promise.resolve(b);await a;return await p;",
        SNAPSHOT_BODY,
    ]
    .into_iter()
    .enumerate()
    {
        for rejected in [false, true] {
            for ts in [false, true] {
                let module = settled_seed(body, rejected, ts);
                let name = format!("private-{case}-{rejected}-{ts}");
                let settle = if rejected {
                    "const settle=x=>Promise.reject(x);"
                } else {
                    "const settle=x=>Promise.resolve(x);"
                };
                let source = format!("{settle}{}", body.replace("Promise.resolve", "settle"));
                async_case(&mut script, &name, &module, &source);
            }
        }
    }
    finish_async(script);
}

const SNAPSHOT_BODY: &str = "let n=0;let p=Promise.resolve(a);const old=p;const f=a?()=>p:()=>p;p=Promise.resolve(b);let x=0;try{x=await old;}catch(e){n+=1;x=e;}let y=0;try{y=await f();}catch(e){n+=10;y=e;}return +x+(+y)+n;";

#[test]
fn settled_promise_cell_stores_change_status_without_changing_loaded_snapshots() {
    let mut script = async_script();
    for first_rejected in [false, true] {
        let mut module = settled_seed(SNAPSHOT_BODY, !first_rejected, false);
        let mut changed = false;
        for op in &mut module.functions[0].body.operations {
            if let Op::SettlePromise { body, .. } = &mut op.kind {
                let input = match body.exit {
                    Exit::Return(v) | Exit::Throw(v) => v,
                    _ => unreachable!(),
                };
                body.exit = if first_rejected {
                    Exit::Throw(input)
                } else {
                    Exit::Return(input)
                };
                changed = true;
                break;
            }
        }
        assert!(changed);
        let prefix = if first_rejected {
            "let calls=0;const settle=x=>++calls===1?Promise.reject(x):Promise.resolve(x);"
        } else {
            "let calls=0;const settle=x=>++calls===1?Promise.resolve(x):Promise.reject(x);"
        };
        let source = format!(
            "{prefix}{}",
            SNAPSHOT_BODY.replace("Promise.resolve", "settle")
        );
        async_case(
            &mut script,
            &format!("mixed-cell-{first_rejected}"),
            &module,
            &source,
        );
    }
    finish_async(script);
}

fn wide_module(count: usize, rejected: bool, ts: bool) -> Module {
    let mut module = settled_seed("let f=p=>p;return await f(Promise.resolve(a));", false, ts);
    for signature in &mut module.closure_signatures.higher {
        if signature.parameters == [Type::SettledPromise] {
            signature.parameters = vec![Type::SettledPromise; count];
        }
    }
    for f in module.functions.iter_mut().skip(1) {
        assert_eq!(f.parameter_count, 1);
        assert_eq!(f.slots.len(), 1);
        f.parameter_count = count;
        while f.slots.len() < count {
            f.slots.push(Slot {
                ty: Type::SettledPromise,
                owner: f.body.id,
                mutable: true,
                parameter: true,
                span: f.body.span,
            });
        }
        for op in &mut f.body.operations {
            if let Op::Load { slot, .. } = &mut op.kind {
                assert_eq!(*slot, SlotId(0));
                *slot = SlotId(count - 1);
            }
        }
    }
    let f = &mut module.functions[0];
    let span = f.body.span;
    let call = f
        .body
        .operations
        .iter()
        .position(|op| matches!(op.kind, Op::ClosureCall { .. }))
        .unwrap();
    let Op::ClosureCall { arguments, .. } = &f.body.operations[call].kind else {
        unreachable!()
    };
    let first = arguments[0];
    let number = ValueId(f.values.len());
    f.values.push(Type::Number);
    let general = ValueId(f.values.len());
    f.values.push(Type::General);
    let last = ValueId(f.values.len());
    f.values.push(Type::SettledPromise);
    let extra = vec![
        Operation {
            span,
            kind: Op::Load {
                result: number,
                slot: SlotId(1),
            },
        },
        Operation {
            span,
            kind: Op::Pack {
                result: general,
                input: number,
            },
        },
        Operation {
            span,
            kind: Op::SettlePromise {
                result: last,
                body: Region {
                    id: RegionId(100000),
                    span,
                    operations: vec![],
                    exit: if rejected {
                        Exit::Throw(general)
                    } else {
                        Exit::Return(general)
                    },
                },
            },
        },
    ];
    let mut operations = Vec::new();
    let mut extra = Some(extra);
    for (index, mut op) in std::mem::take(&mut f.body.operations)
        .into_iter()
        .enumerate()
    {
        if index == call {
            operations.extend(extra.take().unwrap());
            let Op::ClosureCall { arguments, .. } = &mut op.kind else {
                unreachable!()
            };
            *arguments = vec![first; count];
            arguments[count - 1] = last;
        }
        operations.push(op);
    }
    f.body.operations = operations;
    module
}

#[test]
fn settled_promise_thousand_parameter_frames_preserve_last_payload_and_status() {
    let mut script = async_script();
    for rejected in [false, true] {
        for ts in [false, true] {
            let module = wide_module(1000, rejected, ts);
            let last = if rejected {
                "Promise.reject(b)"
            } else {
                "Promise.resolve(b)"
            };
            let source = format!(
                "const values=Array(1000).fill(Promise.resolve(a));values[999]={last};const f=(...p)=>p[999];return await f(...values);"
            );
            async_case(
                &mut script,
                &format!("wide-{rejected}-{ts}"),
                &module,
                &source,
            );
        }
    }
    for ts in [false, true] {
        let mut module = wide_module(1000, false, ts);
        for f in module.functions.iter_mut().skip(1) {
            for op in &mut f.body.operations {
                if let Op::Load { slot, .. } = &mut op.kind {
                    *slot = SlotId(0);
                }
            }
        }
        async_case(
            &mut script,
            &format!("wide-first-{ts}"),
            &module,
            "const values=Array(1000).fill(Promise.resolve(a));values[999]=Promise.resolve(b);const f=(...p)=>p[0];return await f(...values);",
        );
    }
    assert!(
        verify_module_with_entry_type(&wide_module(1001, false, false), Type::General).is_err()
    );
    finish_async(script);
}

fn repeated_await(count: usize) -> Module {
    let mut module = capture_module();
    let f = &mut module.functions[0];
    let span = f.body.span;
    let Op::SettlePromise { result: input, .. } = f.body.operations[0].kind else {
        unreachable!()
    };
    for _ in 0..count {
        let result = ValueId(f.values.len());
        f.values.push(Type::General);
        f.body.operations.push(Operation {
            span,
            kind: Op::AwaitSettled { result, input },
        });
        f.body.exit = Exit::Return(result);
    }
    module
}

#[test]
fn settled_promise_await_budget_counts_one_logical_site_for_both_settlements() {
    let module = repeated_await(32);
    let mut script = async_script();
    async_case(
        &mut script,
        "await-32",
        &module,
        "const p=Promise.resolve(b);for(let i=0;i<32;i++)await p;return b;",
    );
    finish_async(script);
    assert!(
        crate::emit::emit_async(&repeated_await(33))
            .unwrap_err()
            .contains("32 await sites")
    );
}

fn promoted_module() -> Module {
    let mut module = capture_module();
    let f = &mut module.functions[0];
    let span = f.body.span;
    let operation = f.body.operations.remove(0);
    let Op::SettlePromise { result, body } = operation.kind else {
        unreachable!()
    };
    let Exit::Return(input) = body.exit else {
        unreachable!()
    };
    let fulfilled = ValueId(f.values.len());
    f.values.push(Type::FulfilledPromise);
    let mut ops = body.operations;
    ops.push(Operation {
        span,
        kind: Op::FulfillPromise {
            result: fulfilled,
            input,
        },
    });
    ops.push(Operation {
        span,
        kind: Op::PromotePromise {
            result,
            input: fulfilled,
        },
    });
    ops.append(&mut f.body.operations);
    f.body.operations = ops;
    module
}

#[test]
fn settled_promise_kinds_scopes_and_private_interfaces_reject_malformed_ir() {
    verify_module_with_entry_type(&promoted_module(), Type::General).unwrap();
    for change in 0..11 {
        let mut module = capture_module();
        let f = &mut module.functions[0];
        match change {
            0 => {
                let Op::SettlePromise { result, .. } = f.body.operations[0].kind else {
                    unreachable!()
                };
                f.values[result.0] = Type::General;
            }
            1..=5 => {
                let Op::SettlePromise { body, .. } = &mut f.body.operations[0].kind else {
                    unreachable!()
                };
                match change {
                    1 => body.exit = Exit::Return(ValueId(0)),
                    2 => body.exit = Exit::Throw(ValueId(0)),
                    3 => body.exit = Exit::Fallthrough,
                    4 => body.exit = Exit::Break,
                    _ => body.id = RegionId(0),
                }
            }
            6 => {
                let Op::SettledValue { input, .. } = &mut f.body.operations[1].kind else {
                    unreachable!()
                };
                *input = ValueId(0);
            }
            7 => {
                let Op::SettlePromise { body, .. } = &mut f.body.operations[0].kind else {
                    unreachable!()
                };
                body.exit = Exit::Return(ValueId(3));
            }
            8 => {
                let result = ValueId(f.values.len());
                f.values.push(Type::Number);
                f.body.operations.push(Operation {
                    span: f.body.span,
                    kind: Op::PromiseRejected {
                        result,
                        input: ValueId(2),
                    },
                });
            }
            9 => {
                f.slots[0].ty = Type::SettledPromise;
            }
            _ => f.return_type = Type::SettledPromise,
        }
        assert!(
            verify_module_with_entry_type(&module, Type::General).is_err(),
            "malformed capture {change}"
        );
    }
    for change in 0..2 {
        let mut module = await_capture("return b;", false);
        let f = &mut module.functions[0];
        let Op::AwaitSettled { result, input } = &mut f.body.operations.last_mut().unwrap().kind
        else {
            unreachable!()
        };
        if change == 0 {
            f.values[result.0] = Type::Predicate;
        } else {
            *input = ValueId(0);
        }
        assert!(
            verify_module_with_entry_type(&module, Type::General).is_err(),
            "malformed await {change}"
        );
    }
    for change in 0..2 {
        let mut module = promoted_module();
        for op in &mut module.functions[0].body.operations {
            match &mut op.kind {
                Op::PromotePromise { input, .. } if change == 0 => *input = ValueId(1),
                Op::SettledValue { input, .. } if change == 1 => *input = ValueId(4),
                _ => (),
            }
        }
        assert!(
            verify_module_with_entry_type(&module, Type::General).is_err(),
            "malformed promotion {change}"
        );
    }
    let mut module = wide_module(1, false, false);
    let f = &mut module.functions[0];
    let primitive = f
        .body
        .operations
        .iter()
        .filter_map(|op| {
            if let Op::Pack { result, .. } = op.kind {
                Some(result)
            } else {
                None
            }
        })
        .next_back()
        .unwrap();
    for op in &mut f.body.operations {
        if let Op::ClosureCall { arguments, .. } = &mut op.kind {
            arguments[0] = primitive;
        }
    }
    assert!(verify_module_with_entry_type(&module, Type::General).is_err());
    assert!(verify_module_with_entry_type(&capture_module(), Type::SettledPromise).is_err());
    for change in 0..3 {
        let mut module = wide_module(1, false, false);
        match change {
            0 => module.functions[1].return_type = Type::General,
            1 => module.closure_signatures.higher[0].result = Type::General,
            _ => module.closure_signatures.higher[0].parameters[0] = Type::FulfilledPromise,
        }
        assert!(
            verify_module_with_entry_type(&module, Type::General).is_err(),
            "same-layout private kind mismatch {change}"
        );
    }
}

#[test]
fn settled_promise_capture_inherits_its_no_suspend_contract_into_branches() {
    let mut module = capture_source("if(a)return b;return null;");
    let f = &mut module.functions[0];
    let result = ValueId(f.values.len());
    f.values.push(Type::General);
    let Op::SettlePromise { body, .. } = &mut f.body.operations[0].kind else {
        unreachable!()
    };
    let branch = body
        .operations
        .iter_mut()
        .find_map(|op| {
            if let Op::If { consequent, .. } = &mut op.kind {
                Some(consequent)
            } else {
                None
            }
        })
        .unwrap();
    let Exit::Return(input) = branch.exit else {
        unreachable!()
    };
    branch.operations.push(Operation {
        span: branch.span,
        kind: Op::Await {
            result,
            input,
            rejected: false,
        },
    });
    assert!(
        verify_module_with_entry_type(&module, Type::General)
            .unwrap_err()
            .contains("settlement bodies cannot suspend")
    );
}

#[test]
fn settled_promise_fulfilled_conversion_preserves_payload_without_rejecting() {
    let module = promoted_module();
    let bytes =
        wat::parse_str(crate::emit::emit_with_entry_type(&module, Type::General).unwrap()).unwrap();
    wasm_record("promoted", "general", &bytes);
    node(&format!(
        "const assert=require('node:assert/strict'),d=new DataView(new ArrayBuffer(8)),run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from({bytes:?}))).exports.run;let i=0;for(const[a,b]of[[4,2],[0,-0],[-0,-2],[NaN,Infinity]]){{const[tag,value]=run(a,b);assert.equal(tag,3);assert(Object.is(value,b));if({}){{d.setFloat64(0,b);console.log('IR_SETTLED_OBS|promoted|'+i+'|fulfilled|3|'+d.getBigUint64(0).toString(16).padStart(16,'0')+'|0');}}i++;}}",
        proof()
    ));
}

#[test]
fn settled_promise_capture_closes_general_throw_from_numeric_inline_call() {
    let mut module = capture_source("return null;");
    let f = &mut module.functions[0];
    let number = ValueId(f.values.len());
    f.values.push(Type::Number);
    let primitive = ValueId(f.values.len());
    f.values.push(Type::General);
    let Op::SettlePromise { body, .. } = &mut f.body.operations[0].kind else {
        unreachable!()
    };
    let Exit::Return(reason) = body.exit else {
        unreachable!()
    };
    body.operations.push(Operation {
        span: body.span,
        kind: Op::InlineCall {
            result: number,
            body: Region {
                id: RegionId(300),
                span: body.span,
                operations: vec![],
                exit: Exit::Throw(reason),
            },
        },
    });
    body.operations.push(Operation {
        span: body.span,
        kind: Op::Pack {
            result: primitive,
            input: number,
        },
    });
    body.exit = Exit::Return(primitive);
    let bytes =
        wat::parse_str(crate::emit::emit_with_entry_type(&module, Type::General).unwrap()).unwrap();
    wasm_record("inline-throw", "general", &bytes);
    node(&format!(
        "const assert=require('node:assert/strict'),run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from({bytes:?}))).exports.run;let i=0;for(const[a,b]of[[4,2],[0,-0],[-0,-2],[NaN,Infinity]]){{const[tag,payload]=run(a,b);assert.equal(tag,1);assert(Object.is(payload,0));if({})console.log('IR_SETTLED_OBS|inline-throw|'+i+'|fulfilled|1|0000000000000000|0');i++;}}",
        proof()
    ));
}
