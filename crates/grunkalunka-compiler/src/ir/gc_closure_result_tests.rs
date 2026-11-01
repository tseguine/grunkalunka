//! Typed synchronous managed results; source result inference is separate.
use super::managed_value_tests::{effect, value};
use super::*;

fn function(parameters: &[Type], result: Type) -> Function {
    let span = Span::new(0, 0);
    Function {
        return_type: result,
        parameter_count: parameters.len(),
        slots: parameters
            .iter()
            .map(|ty| Slot {
                ty: *ty,
                owner: RegionId(0),
                mutable: true,
                parameter: true,
                span,
            })
            .collect(),
        values: vec![],
        body: Region {
            id: RegionId(0),
            span,
            operations: vec![],
            exit: Exit::Fallthrough,
        },
    }
}
fn load(f: &mut Function, index: usize) -> ValueId {
    value(f, f.slots[index].ty, |result| Op::Load {
        result,
        slot: SlotId(index),
    })
}
fn boxed(f: &mut Function, input: ValueId) -> ValueId {
    match f.values[input.0] {
        Type::ManagedObject => value(f, Type::ManagedValue, |result| Op::ManagedFromObject {
            result,
            input,
        }),
        Type::ManagedArray => value(f, Type::ManagedValue, |result| Op::ManagedFromArray {
            result,
            input,
        }),
        Type::ManagedValue => input,
        _ => panic!("managed fixture value"),
    }
}
fn observe_same(f: &mut Function, left: ValueId, right: ValueId) {
    let left = boxed(f, left);
    let right = boxed(f, right);
    let same = value(f, Type::Predicate, |result| Op::ManagedEqual {
        result,
        left,
        right,
    });
    let result = value(f, Type::Number, |result| Op::NumberFromPredicate {
        result,
        input: same,
    });
    f.body.exit = Exit::Return(result);
}
fn round_trip(ty: Type) -> Module {
    let mut m = crate::lower_source("export function run(a,b){return a;}", false).unwrap();
    let signature = m.closure_signatures.intern(vec![ty], ty);
    let f = &mut m.functions[0];
    f.values.clear();
    f.body.operations.clear();
    let object = value(f, ty, |result| match ty {
        Type::ManagedObject => Op::ManagedObjectNew { result },
        Type::ManagedArray => Op::ManagedArrayNew { result, length: 1 },
        Type::ManagedValue => Op::ManagedString {
            result,
            units: vec![0xd800, 0, 0xdc00],
        },
        _ => unreachable!(),
    });
    let callee = value(f, Type::Closure(signature), |result| Op::MakeClosure {
        result,
        function: FunctionId(1),
        captures: vec![],
    });
    let returned = value(f, ty, |result| Op::ClosureCall {
        result,
        callee,
        arguments: vec![object],
    });
    observe_same(f, object, returned);
    let mut identity = function(&[ty], ty);
    effect(&mut identity, Op::ClosureEntry { captures: vec![] });
    let result = load(&mut identity, 0);
    identity.body.exit = Exit::Return(result);
    m.functions.push(identity);
    m
}
fn check(m: &Module, native: &str) {
    verify_module(m).unwrap();
    let wat = crate::emit::emit(m).unwrap();
    assert!(wat.contains("$gc_frame"));
    assert!(!wat.contains("$cell_top"));
    super::managed_value_tests::node(&wat::parse_str(wat).unwrap(), native);
}
#[test]
fn managed_object_results_preserve_private_argument_identity() {
    check(
        &round_trip(Type::ManagedObject),
        "(()=>{const x={};const f=x=>x;return Number(f(x)===x);})()",
    );
}

fn number(f: &mut Function, n: ValueId) -> ValueId {
    let general = value(f, Type::General, |result| Op::Pack { result, input: n });
    value(f, Type::ManagedValue, |result| Op::ManagedPrimitive {
        result,
        input: general,
    })
}
#[test]
fn managed_array_results_preserve_alias_mutation_and_length() {
    let mut m = round_trip(Type::ManagedArray);
    let f = &mut m.functions[0];
    let same = match f.body.exit {
        Exit::Return(v) => v,
        _ => unreachable!(),
    };
    let returned = f
        .body
        .operations
        .iter()
        .find_map(|o| match o.kind {
            Op::ClosureCall { result, .. } => Some(result),
            _ => None,
        })
        .unwrap();
    let b = load(f, 1);
    let b = number(f, b);
    effect(
        f,
        Op::ManagedArraySetOwn {
            array: returned,
            key: vec![50],
            value: b,
        },
    );
    let length = value(f, Type::Number, |result| Op::ManagedArrayLength {
        result,
        array: ValueId(0),
    });
    let result = value(f, Type::Number, |result| Op::Binary {
        result,
        operator: Binary::Add,
        left: same,
        right: length,
    });
    f.body.exit = Exit::Return(result);
    check(
        &m,
        "(()=>{const x=new Array(1);const f=x=>x;const y=f(x);y[2]=b;return Number(x===y)+x.length;})()",
    );
}
#[test]
fn managed_result_carriers_keep_lossless_text_and_boxed_object_identity() {
    check(
        &round_trip(Type::ManagedValue),
        "(()=>{const x=String.fromCharCode(0xd800,0,0xdc00);const f=x=>x;return Number(f(x)===x);})()",
    );
    let mut m = round_trip(Type::ManagedValue);
    let f = &mut m.functions[0];
    let object = value(f, Type::ManagedObject, |result| Op::ManagedObjectNew {
        result,
    });
    let allocation = f.body.operations.pop().unwrap();
    f.body.operations.insert(0, allocation);
    let origin = &mut f.body.operations[1].kind;
    let Op::ManagedString { result, .. } = origin else {
        unreachable!()
    };
    *origin = Op::ManagedFromObject {
        result: *result,
        input: object,
    };
    check(
        &m,
        "(()=>{const x={};const f=x=>x;return Number(f(x)===x);})()",
    );
}
#[test]
fn managed_result_carriers_keep_number_kind_and_nan_equality() {
    let mut m = round_trip(Type::ManagedValue);
    let f = &mut m.functions[0];
    let a = load(f, 0);
    let packed = value(f, Type::General, |result| Op::Pack { result, input: a });
    let start = f.body.operations.len() - 2;
    let prefix = f.body.operations.split_off(start);
    f.body.operations.splice(0..0, prefix);
    let Op::ManagedString { result, .. } = f.body.operations[2].kind else {
        unreachable!()
    };
    f.body.operations[2].kind = Op::ManagedPrimitive {
        result,
        input: packed,
    };
    check(&m, "Number(a===a)");
}
#[test]
fn fresh_managed_results_select_gc_without_managed_parameters_or_captures() {
    let mut m = round_trip(Type::ManagedObject);
    m.closure_signatures.higher[0].parameters.clear();
    let mut factory = function(&[], Type::ManagedObject);
    effect(&mut factory, Op::ClosureEntry { captures: vec![] });
    let object = value(&mut factory, Type::ManagedObject, |result| {
        Op::ManagedObjectNew { result }
    });
    let one = value(&mut factory, Type::Number, |result| Op::Constant {
        result,
        value: 1.0,
    });
    let one = number(&mut factory, one);
    effect(
        &mut factory,
        Op::ManagedSetOwn {
            object,
            key: vec![120],
            value: one,
        },
    );
    factory.body.exit = Exit::Return(object);
    m.functions[1] = factory;
    let f = &mut m.functions[0];
    let same = match f.body.exit {
        Exit::Return(v) => v,
        _ => unreachable!(),
    };
    let mut returned = None;
    for op in &mut f.body.operations {
        if let Op::ClosureCall {
            result, arguments, ..
        } = &mut op.kind
        {
            arguments.clear();
            returned = Some(*result);
        }
    }
    let read = value(f, Type::ManagedValue, |result| Op::ManagedGetOwn {
        result,
        object: returned.unwrap(),
        key: vec![120],
    });
    let one = value(f, Type::Number, |result| Op::Constant {
        result,
        value: 1.0,
    });
    let one = number(f, one);
    let equal = value(f, Type::Predicate, |result| Op::ManagedEqual {
        result,
        left: read,
        right: one,
    });
    let equal = value(f, Type::Number, |result| Op::NumberFromPredicate {
        result,
        input: equal,
    });
    let result = value(f, Type::Number, |result| Op::Binary {
        result,
        operator: Binary::Add,
        left: same,
        right: equal,
    });
    f.body.exit = Exit::Return(result);
    check(
        &m,
        "(()=>{const old={};const make=()=>({x:1});const x=make();return Number(old===x)+Number(x.x===1);})()",
    );
}

#[test]
fn managed_result_functions_pass_through_callbacks() {
    let mut m = round_trip(Type::ManagedObject);
    let leaf = 1001;
    let wrapper_signature = m.closure_signatures.intern(
        vec![Type::Closure(leaf), Type::ManagedObject],
        Type::ManagedObject,
    );
    let mut wrapper = function(
        &[Type::Closure(leaf), Type::ManagedObject],
        Type::ManagedObject,
    );
    effect(&mut wrapper, Op::ClosureEntry { captures: vec![] });
    let callback = load(&mut wrapper, 0);
    let object = load(&mut wrapper, 1);
    let result = value(&mut wrapper, Type::ManagedObject, |result| {
        Op::ClosureCall {
            result,
            callee: callback,
            arguments: vec![object],
        }
    });
    wrapper.body.exit = Exit::Return(result);
    m.functions.push(wrapper);
    let f = &mut m.functions[0];
    let wrapped = value(f, Type::Closure(wrapper_signature), |result| {
        Op::MakeClosure {
            result,
            function: FunctionId(2),
            captures: vec![],
        }
    });
    let make = f.body.operations.pop().unwrap();
    let index = f
        .body
        .operations
        .iter()
        .position(|o| matches!(o.kind, Op::ClosureCall { .. }))
        .unwrap();
    if let Op::ClosureCall {
        callee, arguments, ..
    } = &mut f.body.operations[index].kind
    {
        arguments.insert(0, *callee);
        *callee = wrapped;
    }
    f.body.operations.insert(index, make);
    check(
        &m,
        "(()=>{const x={};const apply=(fn,x)=>fn(x);return Number(apply(x=>x,x)===x);})()",
    );
}
#[test]
fn returned_functions_retain_managed_result_signatures() {
    let mut m = round_trip(Type::ManagedObject);
    let leaf = 1001;
    let factory_signature = m.closure_signatures.intern(vec![], Type::Closure(leaf));
    let mut factory = function(&[], Type::Closure(leaf));
    effect(&mut factory, Op::ClosureEntry { captures: vec![] });
    let result = value(&mut factory, Type::Closure(leaf), |result| {
        Op::MakeClosure {
            result,
            function: FunctionId(1),
            captures: vec![],
        }
    });
    factory.body.exit = Exit::Return(result);
    m.functions.push(factory);
    let f = &mut m.functions[0];
    let index = f
        .body
        .operations
        .iter()
        .position(|o| matches!(o.kind, Op::ClosureCall { .. }))
        .unwrap();
    let original = match f.body.operations[index].kind {
        Op::ClosureCall { callee, .. } => callee,
        _ => unreachable!(),
    };
    f.values[original.0] = Type::Closure(factory_signature);
    for op in &mut f.body.operations {
        if let Op::MakeClosure { function, .. } = &mut op.kind {
            *function = FunctionId(2);
        }
    }
    let returned = value(f, Type::Closure(leaf), |result| Op::ClosureCall {
        result,
        callee: original,
        arguments: vec![],
    });
    let call = f.body.operations.pop().unwrap();
    if let Op::ClosureCall { callee, .. } = &mut f.body.operations[index].kind {
        *callee = returned;
    }
    f.body.operations.insert(index, call);
    check(
        &m,
        "(()=>{const x={};const make=()=>x=>x;return Number(make()(x)===x);})()",
    );
}
#[test]
fn managed_result_signatures_keep_argument_limits_and_precise_validation() {
    let mut m = round_trip(Type::ManagedObject);
    m.closure_signatures.higher[0].parameters = vec![Type::ManagedObject; 1000];
    m.functions[1].slots = vec![m.functions[1].slots[0].clone(); 1000];
    m.functions[1].parameter_count = 1000;
    for op in &mut m.functions[0].body.operations {
        if let Op::ClosureCall { arguments, .. } = &mut op.kind {
            *arguments = vec![arguments[0]; 1000];
        }
    }
    check(
        &m,
        "(()=>{const x={};const f=(...args)=>args[0];return Number(f(...Array(1000).fill(x))===x);})()",
    );
    m.closure_signatures.higher[0]
        .parameters
        .push(Type::ManagedObject);
    assert!(verify_module(&m).is_err());
    for change in 0..7 {
        let mut m = round_trip(Type::ManagedObject);
        match change {
            0 => m.closure_signatures.higher[0].result = Type::ManagedArray,
            1 => m.closure_signatures.higher[0].asynchronous = true,
            2 => m.functions[1].return_type = Type::ManagedValue,
            3 => m.closure_signatures.higher[0].result = Type::Closure(1001),
            4 => m.closure_signatures.higher[0].parameters.push(Type::Number),
            5 => m.functions[1].body.exit = Exit::Return(ValueId(999)),
            _ => {
                m.functions[0].values[2] = Type::ManagedValue;
            }
        }
        assert!(verify_module(&m).is_err(), "change {change}");
    }
    let mut asynchronous = round_trip(Type::ManagedObject);
    let entry = &mut asynchronous.functions[0];
    let input = match entry.body.exit {
        Exit::Return(value) => value,
        _ => unreachable!(),
    };
    let result = value(entry, Type::General, |result| Op::Pack { result, input });
    entry.return_type = Type::General;
    entry.body.exit = Exit::Return(result);
    wat::parse_str(crate::emit::emit_async(&asynchronous).unwrap()).unwrap();
}
#[test]
fn source_managed_results_keep_numeric_exports_coercion_and_async_guards() {
    for source in [
        "export function run(a,b){let f=()=>({x:a});return f();}",
        "export function run(a,b){let f=()=>({x:a});return f()+a;}",
        "export async function run(a,b){const o={};let f=()=>o;await 0;return f()===o?a:b;}",
    ] {
        for ts in [false, true] {
            let error = crate::compile(source, ts)
                .err()
                .unwrap_or_else(|| panic!("source result admitted: {source}"));
            assert!(
                error.contains("unsupported") && !error.contains("internal IR"),
                "{source}: {error}"
            );
        }
    }
}

#[test]
fn managed_inline_body_results_keep_exact_private_types_and_await_guards() {
    for ty in [Type::ManagedObject, Type::ManagedArray, Type::ManagedValue] {
        let mut m = round_trip(ty);
        let f = &mut m.functions[1];
        let body = Region {
            id: RegionId(1),
            span: f.body.span,
            operations: f.body.operations.split_off(1),
            exit: std::mem::replace(&mut f.body.exit, Exit::Fallthrough),
        };
        let result = value(f, ty, |result| Op::InlineCall { result, body });
        f.body.exit = Exit::Return(result);
        let native = match ty {
            Type::ManagedObject => {
                "(()=>{const x={};const f=x=>{return x;};return Number(f(x)===x);})()"
            }
            Type::ManagedArray => {
                "(()=>{const x=[];const f=x=>{return x;};return Number(f(x)===x);})()"
            }
            _ => {
                "(()=>{const x=String.fromCharCode(0xd800,0,0xdc00);const f=x=>{return x;};return Number(f(x)===x);})()"
            }
        };
        check(&m, native);
        m.functions[1].values[result.0] = Type::Number;
        assert!(verify_module(&m).is_err(), "mismatched inline return kind");
        m.functions[1].values[result.0] = ty;
        let op = &mut m.functions[1].body.operations[1].kind;
        let Op::InlineCall { result, body } =
            std::mem::replace(op, Op::CaptureCells { cells: vec![] })
        else {
            unreachable!()
        };
        *op = Op::AwaitCall { result, body };
        let error = verify_module(&m).unwrap_err();
        assert!(error.contains("awaited calls require General"), "{error}");
    }
}
