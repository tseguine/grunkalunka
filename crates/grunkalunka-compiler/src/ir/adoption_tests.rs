use super::*;

fn fixture() -> Module {
    crate::lower_source_profile(
        "export async function run(a,b){let x=a;const f=async()=>{await a;return b;};return f();}",
        false,
        Type::General,
        true,
    )
    .unwrap()
}

#[test]
fn adoption_requires_a_pending_input_and_general_completion() {
    for input in [true, false] {
        let mut module = fixture();
        verify_module_with_entry_type(&module, Type::General).unwrap();
        let root = &mut module.functions[0];
        let number = ValueId(
            root.values
                .iter()
                .position(|ty| *ty == Type::Number)
                .unwrap(),
        );
        let Op::AdoptPromise {
            result,
            input: source,
        } = &mut root.body.operations.last_mut().unwrap().kind
        else {
            panic!("adoption fixture");
        };
        if input {
            *source = number;
        } else {
            root.values[result.0] = Type::Number;
        }
        assert!(verify_module_with_entry_type(&module, Type::General).is_err());
    }
}

#[test]
fn adoption_has_physical_activation_ownership_and_cannot_enter_a_snapshot_settlement() {
    for kind in [0, 1, 2] {
        let mut module = fixture();
        let root = &mut module.functions[0];
        let operation = root.body.operations.pop().unwrap();
        let Op::AdoptPromise { result, .. } = operation.kind else {
            panic!("adoption fixture");
        };
        let body = Region {
            id: RegionId(10000),
            span: operation.span,
            operations: vec![operation],
            exit: Exit::Return(result),
        };
        let value = ValueId(root.values.len());
        root.values.push(if kind == 2 {
            Type::SettledPromise
        } else {
            Type::General
        });
        root.body.operations.push(Operation {
            span: body.span,
            kind: match kind {
                0 => Op::InlineCall {
                    result: value,
                    body,
                },
                1 => Op::AwaitCall {
                    result: value,
                    body,
                },
                _ => Op::SettlePromise {
                    result: value,
                    body,
                },
            },
        });
        if kind != 2 {
            root.body.exit = Exit::Return(value);
        } else {
            let projected = ValueId(root.values.len());
            root.values.push(Type::General);
            root.body.operations.push(Operation {
                span: root.body.span,
                kind: Op::SettledValue {
                    result: projected,
                    input: value,
                },
            });
            root.body.exit = Exit::Return(projected);
        }
        let checked = verify_module_with_entry_type(&module, Type::General);
        if kind == 0 {
            checked.unwrap();
        } else {
            assert!(checked.err().unwrap().contains(if kind == 1 {
                "physical async activation"
            } else {
                "settlement bodies cannot suspend"
            }));
        }
    }
}
