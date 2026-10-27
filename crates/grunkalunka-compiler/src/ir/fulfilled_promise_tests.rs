use super::*;

fn module() -> Module {
    crate::lower_source_profile(
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
    let mut module = crate::lower_source_profile(
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
fn fulfilled_promises_cannot_be_source_or_callback_parameters() {
    let mut module = crate::lower_source_profile(
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
    assert!(!signatures.is_callback_signature(signature));
    assert!(!signatures.is_returned_function(signature));
}
