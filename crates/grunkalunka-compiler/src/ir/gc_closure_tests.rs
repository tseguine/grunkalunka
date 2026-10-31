//! Typed-IR storage checks and source capture boundary regressions.
use super::*;

fn value(f: &mut Function, ty: Type, op: impl FnOnce(ValueId) -> Op) -> ValueId {
    let id = ValueId(f.values.len());
    f.values.push(ty);
    f.body.operations.push(Operation {
        span: f.body.span,
        kind: op(id),
    });
    id
}
fn push(f: &mut Function, op: Op) {
    f.body.operations.push(Operation {
        span: f.body.span,
        kind: op,
    });
}
fn fixture() -> Module {
    let mut m = crate::lower_source("export function run(a,b){return a;}", false).unwrap();
    let root = &mut m.functions[0];
    root.slots.truncate(2);
    root.values.clear();
    root.body.operations.clear();
    for _ in 0..2 {
        root.slots.push(Slot {
            ty: Type::ManagedObject,
            owner: root.body.id,
            mutable: true,
            parameter: false,
            span: root.body.span,
        });
    }
    let old = value(root, Type::ManagedObject, |result| Op::ManagedObjectNew {
        result,
    });
    push(
        root,
        Op::Initialize {
            slot: SlotId(2),
            value: old,
        },
    );
    push(
        root,
        Op::Initialize {
            slot: SlotId(3),
            value: old,
        },
    );
    let get = value(root, Type::Closure(0), |result| Op::MakeClosure {
        result,
        function: FunctionId(1),
        captures: vec![SlotId(2), SlotId(3)],
    });
    let before = value(root, Type::Number, |result| Op::ClosureCall {
        result,
        callee: get,
        arguments: vec![],
    });
    let fresh = value(root, Type::ManagedObject, |result| Op::ManagedObjectNew {
        result,
    });
    push(
        root,
        Op::Store {
            slot: SlotId(2),
            value: fresh,
        },
    );
    let after = value(root, Type::Number, |result| Op::ClosureCall {
        result,
        callee: get,
        arguments: vec![],
    });
    let result = value(root, Type::Number, |result| Op::Binary {
        result,
        operator: Binary::Subtract,
        left: before,
        right: after,
    });
    root.body.exit = Exit::Return(result);
    let span = root.body.span;
    m.functions.push(Function {
        return_type: Type::Number,
        parameter_count: 0,
        slots: vec![],
        values: vec![],
        body: Region {
            id: RegionId(0),
            span,
            operations: vec![],
            exit: Exit::Fallthrough,
        },
    });
    let f = &mut m.functions[1];
    f.values.clear();
    f.body.operations.clear();
    f.slots.clear();
    for _ in 0..2 {
        f.slots.push(Slot {
            ty: Type::ManagedObject,
            owner: f.body.id,
            mutable: true,
            parameter: false,
            span: f.body.span,
        });
    }
    push(
        f,
        Op::ClosureEntry {
            captures: vec![SlotId(0), SlotId(1)],
        },
    );
    let x = value(f, Type::ManagedObject, |result| Op::Load {
        result,
        slot: SlotId(0),
    });
    let old = value(f, Type::ManagedObject, |result| Op::Load {
        result,
        slot: SlotId(1),
    });
    let x = value(f, Type::ManagedValue, |result| Op::ManagedFromObject {
        result,
        input: x,
    });
    let old = value(f, Type::ManagedValue, |result| Op::ManagedFromObject {
        result,
        input: old,
    });
    let same = value(f, Type::Predicate, |result| Op::ManagedEqual {
        result,
        left: x,
        right: old,
    });
    let n = value(f, Type::Number, |result| Op::NumberFromPredicate {
        result,
        input: same,
    });
    f.body.exit = Exit::Return(n);
    m
}

#[test]
fn gc_capture_shares_the_binding_instead_of_copying_its_value() {
    let m = fixture();
    verify_module(&m).unwrap();
    let wat = crate::emit::emit(&m).unwrap();
    assert!(wat.contains("$gc_cell"));
    assert!(!wat.contains("$cell_top"));
    super::managed_value_tests::node(
        &wat::parse_str(wat).unwrap(),
        "(()=>{let x={};const old=x;const get=()=>Number(x===old);const before=get();x={};return before-get();})()",
    );
}

fn check(m: &Module, native: &str) {
    verify_module(m).unwrap();
    let wat = crate::emit::emit(m).unwrap();
    super::managed_value_tests::node(&wat::parse_str(wat).unwrap(), native);
}
fn force_gc_storage(m: &mut Module) {
    let f = &mut m.functions[0];
    let slot = SlotId(f.slots.len());
    f.slots.push(Slot {
        ty: Type::ManagedValue,
        owner: f.body.id,
        mutable: false,
        parameter: false,
        span: f.body.span,
    });
    let text = value(f, Type::ManagedValue, |result| Op::ManagedString {
        result,
        units: vec![0, 0xd800],
    });
    push(f, Op::Initialize { slot, value: text });
    push(f, Op::CaptureCells { cells: vec![slot] });
}
#[test]
fn gc_capture_arrays_and_strings_preserve_rebinding() {
    for ty in [Type::ManagedArray, Type::ManagedValue] {
        let mut m = fixture();
        for f in &mut m.functions {
            for slot in &mut f.slots {
                if slot.ty == Type::ManagedObject {
                    slot.ty = ty;
                }
            }
            for v in &mut f.values {
                if *v == Type::ManagedObject {
                    *v = ty;
                }
            }
            let mut first = true;
            for op in &mut f.body.operations {
                match &op.kind {
                    Op::ManagedObjectNew { result } => {
                        op.kind = if ty == Type::ManagedArray {
                            Op::ManagedArrayNew {
                                result: *result,
                                length: 0,
                            }
                        } else {
                            Op::ManagedString {
                                result: *result,
                                units: if first {
                                    vec![0, 0xd800]
                                } else {
                                    vec![0, 0xdc00]
                                },
                            }
                        };
                        first = false;
                    }
                    Op::ManagedFromObject { result, input } => {
                        op.kind = if ty == Type::ManagedArray {
                            Op::ManagedFromArray {
                                result: *result,
                                input: *input,
                            }
                        } else {
                            Op::Load {
                                result: *result,
                                slot: if input.0 == 0 { SlotId(0) } else { SlotId(1) },
                            }
                        };
                    }
                    _ => {}
                }
            }
        }
        check(&m, "1");
    }
}
#[test]
fn gc_cells_preserve_numeric_general_callback_and_returned_closure_lanes() {
    for body in [
        "let n=a;const f=a?()=>++n:()=>--n;const old=n;return f()+n-old;",
        "const h=a?(x,fn,z)=>fn(+x)+(+z):(x,fn,z)=>fn(-x)-(+z);return h(null,y=>y+b,true);",
        "const one=x=>()=>++x;const two=x=>()=>x+b;const make=a?one:two;const f=make(a);const g=make(b);return f()*100+g()*10+f();",
        "let current=()=>a;let make=x=>(current=()=>x,current);const old=make(b);make(a);return old()+current();",
        "const factory=a?(fn,x)=>()=>fn(x):(fn,x)=>()=>fn(x)+b;const f=factory(x=>x+a,b);return f()+f();",
        "const f=a?x=>x:x=>null;return f(null)===null?a:b;",
        "const make=a?x=>()=>+x:x=>()=>-x;return make(null)();",
        "let make=(fn,x)=>()=>fn(x);const f=make(y=>y+a,b);return f();",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        let mut m = crate::lower_source(&source, false).unwrap();
        force_gc_storage(&mut m);
        check(&m, &format!("(()=>{{{body}}})()"));
    }
}
#[test]
fn gc_cell_reinitialization_preserves_per_iteration_environments() {
    let body = "let first=()=>a;let last=()=>b;for(let i=0;i<3;i++){let x=i;if(i===0)first=()=>x;last=()=>x;}return first()*10+last();";
    let mut m = crate::lower_source(&format!("export function run(a,b){{{body}}}"), false).unwrap();
    force_gc_storage(&mut m);
    check(&m, &format!("(()=>{{{body}}})()"));
}
#[test]
fn gc_capture_types_initialization_and_async_boundaries_remain_checked() {
    for change in 0..4 {
        let mut m = fixture();
        match change {
            0 => m.functions[1].slots[0].ty = Type::ManagedArray,
            1 => m.functions[1].slots[0].mutable = false,
            2 => {
                m.functions[0].body.operations.remove(1);
            }
            _ => {
                m.functions[0].body.operations[3].kind = Op::MakeClosure {
                    result: ValueId(1),
                    function: FunctionId(1),
                    captures: vec![SlotId(2)],
                };
            }
        }
        assert!(verify_module(&m).is_err(), "change {change}");
    }
    let mut m = fixture();
    let f = &mut m.functions[0];
    let input = match f.body.exit {
        Exit::Return(v) => v,
        _ => unreachable!(),
    };
    let result = value(f, Type::General, |result| Op::Pack { result, input });
    f.return_type = Type::General;
    f.body.exit = Exit::Return(result);
    assert!(crate::emit::emit_async(&m).unwrap_err().contains("managed"));
    for source in [
        "export function run(a,b){let x={};let f=()=>x===x?a:b;return f();}",
        "export function run(a,b){let x=[];let f=()=>x===x?a:b;return f();}",
        "export function run(a,b){let x='text';let f=()=>x==='text'?a:b;return f();}",
    ] {
        let artifact = crate::compile(source, false).unwrap();
        super::managed_value_tests::node(&artifact.wasm, "a");
    }
}
#[test]
fn gc_argument_frames_do_not_exhaust_the_legacy_bump_arena() {
    let body = "const f=a?x=>x:x=>-x;let n=0;for(let i=0;i<70000;i++)n=f(n);return n;";
    let mut m = crate::lower_source(&format!("export function run(a,b){{{body}}}"), false).unwrap();
    force_gc_storage(&mut m);
    check(&m, "0");
}

#[test]
fn stores_from_a_closure_update_the_same_managed_binding() {
    let mut m = fixture();
    push(
        &mut m.functions[1],
        Op::Store {
            slot: SlotId(0),
            value: ValueId(1),
        },
    );
    let f = &mut m.functions[0];
    let previous = match f.body.exit {
        Exit::Return(v) => v,
        _ => unreachable!(),
    };
    let third = value(f, Type::Number, |result| Op::ClosureCall {
        result,
        callee: ValueId(1),
        arguments: vec![],
    });
    let result = value(f, Type::Number, |result| Op::Binary {
        result,
        operator: Binary::Add,
        left: previous,
        right: third,
    });
    f.body.exit = Exit::Return(result);
    check(
        &m,
        "(()=>{let x={};const old=x;const get=()=>{const same=Number(x===old);x=old;return same;};const before=get();x={};return before-get()+get();})()",
    );
}
#[test]
fn gc_argument_frames_keep_the_existing_thousand_parameter_boundary() {
    let parameters = (0..1000)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let mut args = vec!["null"; 1000];
    args[999] = "true";
    let source = format!(
        "export function run(a,b){{const f=a?({parameters})=>+p0+(+p999):({parameters})=>+p0+(+p999);return f({});}}",
        args.join(",")
    );
    let mut m = crate::lower_source(&source, false).unwrap();
    force_gc_storage(&mut m);
    check(&m, "1");
}

#[test]
fn gc_captured_modules_retain_the_primitive_pair_export() {
    let mut m = fixture();
    let f = &mut m.functions[0];
    let input = match f.body.exit {
        Exit::Return(v) => v,
        _ => unreachable!(),
    };
    let result = value(f, Type::General, |result| Op::Pack { result, input });
    f.return_type = Type::General;
    f.body.exit = Exit::Return(result);
    let wat = crate::emit::emit_with_entry_type(&m, Type::General).unwrap();
    // Adapt only the test observation to the shared numeric Node harness. The
    // original export must still produce its independent Number tag and payload.
    let mut wat = wat.replace("(export \"run\")", "");
    let end = wat.rfind(')').unwrap();
    wat.truncate(end);
    wat.push_str(
        r#"(func (export "run") (param f64 f64) (result f64) (local $payload f64)
      local.get 0 local.get 1 call 0 local.set $payload
      i32.const 3 i32.ne if unreachable end local.get $payload))"#,
    );
    super::managed_value_tests::node(&wat::parse_str(wat).unwrap(), "1");
}
