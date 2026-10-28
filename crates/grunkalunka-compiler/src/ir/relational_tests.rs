use super::*;

#[test]
fn general_relational_requires_visible_general_operands_and_predicate_result() {
    let source = "export function run(a,b){return null<=undefined?1:0;}";
    verify_module(&crate::lower_source(source, false).unwrap()).unwrap();
    for change in 0..6 {
        let mut module = crate::lower_source(source, false).unwrap();
        let f = &mut module.functions[0];
        let Op::RelationalCompare {
            left,
            right,
            result,
            ..
        } = &mut f.body.operations[2].kind
        else {
            panic!("General relational operation");
        };
        match change {
            0 => *left = ValueId(99),
            1 => *right = *result,
            2 => f.values[left.0] = Type::Number,
            3 => f.values[right.0] = Type::Predicate,
            4 => f.values[result.0] = Type::General,
            _ => f.body.operations.swap(1, 2),
        }
        assert!(verify_module(&module).is_err());
    }
}

#[test]
fn relational_carriers_keep_visibility_result_and_other_operation_type_guards() {
    let source =
        "export async function run(a,b){const p=Promise.resolve(a);const x=a?p:b;return x<=p;}";
    let original = crate::lower_source_profile(source, false, Type::General, true).unwrap();
    verify_module_with_entry_type(&original, Type::General).unwrap();
    for change in 0..7 {
        let mut module = crate::lower_source_profile(source, false, Type::General, true).unwrap();
        let f = &mut module.functions[0];
        let position = f
            .body
            .operations
            .iter()
            .position(|o| matches!(o.kind, Op::RelationalCompare { .. }))
            .unwrap();
        let Op::RelationalCompare {
            left,
            right,
            result,
            ..
        } = f.body.operations[position].kind
        else {
            unreachable!()
        };
        assert_eq!(f.values[left.0], Type::AsyncValue);
        assert_eq!(f.values[right.0], Type::AsyncValue);
        match change {
            0 => {
                if let Op::RelationalCompare { left, .. } = &mut f.body.operations[position].kind {
                    *left = ValueId(usize::MAX);
                }
            }
            1 => {
                if let Op::RelationalCompare { right, .. } = &mut f.body.operations[position].kind {
                    *right = result;
                }
            }
            2 => f.values[result.0] = Type::Number,
            3 => f.body.operations.swap(0, position),
            4 => {
                f.body.operations[position].kind = Op::PrimitiveStrictEqual {
                    result,
                    left,
                    right,
                    negated: false,
                }
            }
            5 => {
                f.body.operations[position].kind = Op::PrimitiveLooseEqual {
                    result,
                    left,
                    right,
                    negated: false,
                }
            }
            _ => {
                if let Op::RelationalCompare { left, .. } = &mut f.body.operations[position].kind {
                    *left = ValueId(0);
                }
            }
        }
        assert!(
            verify_module_with_entry_type(&module, Type::General).is_err(),
            "malformed relational carrier case {change}"
        );
    }
}
