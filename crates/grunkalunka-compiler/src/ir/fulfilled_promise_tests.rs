use super::*;

fn snapshot_profile(
    source: &str,
    typescript: bool,
    entry_type: Type,
    async_entry: bool,
) -> Result<Module, String> {
    crate::lower_source_profile_with_identities(source, typescript, entry_type, async_entry, false)
}

#[test]
fn fulfilled_promise_closure_results_keep_logical_signatures() {
    let make = || {
        snapshot_profile(
            "export async function run(a,b){let f=x=>Promise.resolve(x);return await f(a);}",
            false,
            Type::General,
            true,
        )
        .unwrap()
    };
    verify_module_with_entry_type(&make(), Type::General).unwrap();
    for change in 0..6 {
        let mut invalid = make();
        match change {
            0 => invalid.closure_signatures.higher[0].result = Type::General,
            1 => invalid.closure_signatures.higher[0].result = Type::Predicate,
            2 => invalid.closure_signatures.higher[0].parameters[0] = Type::FulfilledPromise,
            3 => invalid.functions[1].return_type = Type::General,
            4 => invalid.functions[0].return_type = Type::FulfilledPromise,
            _ => {
                let f = &mut invalid.functions[0];
                let result = f
                    .body
                    .operations
                    .iter()
                    .find_map(|op| match op.kind {
                        Op::ClosureCall { result, .. } => Some(result),
                        _ => None,
                    })
                    .unwrap();
                f.values[result.0] = Type::General;
            }
        }
        assert!(
            verify_module_with_entry_type(&invalid, Type::General).is_err(),
            "mutation {change}"
        );
    }
    let mut signatures = ClosureSignatures::default();
    let promise = signatures.intern(vec![Type::Number], Type::FulfilledPromise);
    let primitive = signatures.intern(vec![Type::Number], Type::General);
    assert_ne!(promise, primitive);
    assert!(signatures.is_callback_signature(promise));
    assert!(signatures.is_returned_function(promise));
}

#[test]
fn fulfilled_promise_inline_results_keep_return_and_async_contracts() {
    let make = || {
        snapshot_profile(
        "export async function run(a,b){const make=x=>{return Promise.resolve(x);};const p=make(a);return await p;}",
        false, Type::General, true,
    ).unwrap()
    };
    verify_module_with_entry_type(&make(), Type::General).unwrap();
    for change in 0..3 {
        let mut invalid = make();
        let function = &mut invalid.functions[0];
        let operation = function
            .body
            .operations
            .iter_mut()
            .find(|operation| matches!(operation.kind, Op::InlineCall { .. }))
            .unwrap();
        let Op::InlineCall { result, .. } = &operation.kind else {
            unreachable!()
        };
        let result = *result;
        match change {
            0 => function.values[result.0] = Type::Predicate,
            1 => function.values[result.0] = Type::General,
            _ => {
                let Op::InlineCall { result, body } = std::mem::replace(
                    &mut operation.kind,
                    Op::Boolean {
                        result,
                        value: false,
                    },
                ) else {
                    unreachable!()
                };
                operation.kind = Op::AwaitCall { result, body };
            }
        }
        let error = verify_module_with_entry_type(&invalid, Type::General).unwrap_err();
        assert!(
            error.contains(match change {
                0 => "inline calls require",
                1 => "invalid value ID or value type",
                _ => "awaited calls require General results",
            }),
            "{change}: {error}"
        );
    }
}

fn module() -> Module {
    snapshot_profile(
        "export async function run(a,b){const p=Promise.resolve(a);return await p;}",
        false,
        Type::General,
        true,
    )
    .unwrap()
}

#[test]
fn fulfilled_promise_wraps_and_projections_require_distinct_types() {
    verify_module_with_entry_type(&module(), Type::General).unwrap();
    for change in 0..3 {
        let mut module = module();
        let function = &mut module.functions[0];
        let index = function
            .body
            .operations
            .iter()
            .position(|op| matches!(op.kind, Op::FulfillPromise { .. }))
            .unwrap();
        let Op::FulfillPromise { result, input } = function.body.operations[index].kind else {
            unreachable!()
        };
        match change {
            0 => function.values[result.0] = Type::General,
            1 => {
                let numeric = function
                    .values
                    .iter()
                    .position(|ty| *ty == Type::Number)
                    .unwrap();
                function.body.operations[index].kind = Op::FulfillPromise {
                    result,
                    input: ValueId(numeric),
                };
            }
            _ => {
                let projection = function
                    .body
                    .operations
                    .iter_mut()
                    .find(|op| matches!(op.kind, Op::PromiseValue { .. }))
                    .unwrap();
                let Op::PromiseValue { result, .. } = projection.kind else {
                    unreachable!()
                };
                projection.kind = Op::PromiseValue { result, input };
            }
        }
        assert!(
            verify_module_with_entry_type(&module, Type::General).is_err(),
            "invalid Promise operation {change}"
        );
    }
}

#[test]
fn fulfilled_promises_cannot_be_public_entry_results() {
    let mut module = snapshot_profile(
        "export function run(a,b){const p=Promise.resolve(a);return a;}",
        false,
        Type::General,
        false,
    )
    .unwrap();
    let function = &mut module.functions[0];
    let promise = function
        .body
        .operations
        .iter()
        .find_map(|op| {
            if let Op::FulfillPromise { result, .. } = op.kind {
                Some(result)
            } else {
                None
            }
        })
        .unwrap();
    function.return_type = Type::FulfilledPromise;
    function.body.exit = Exit::Return(promise);
    assert!(
        verify_module_with_entry_type(&module, Type::FulfilledPromise)
            .unwrap_err()
            .contains("entry profile")
    );
}

#[test]
fn fulfilled_promises_cannot_be_public_parameters() {
    let mut module = snapshot_profile(
        "export function run(a,b){return b;}",
        false,
        Type::General,
        false,
    )
    .unwrap();
    module.functions[0].slots[0].ty = Type::FulfilledPromise;
    assert!(verify_module_with_entry_type(&module, Type::General).is_err());
    let mut signatures = ClosureSignatures::default();
    let signature = signatures.intern(vec![Type::FulfilledPromise], Type::General);
    assert!(signatures.is_callback_signature(signature));
    assert!(signatures.is_returned_function(signature));
}
