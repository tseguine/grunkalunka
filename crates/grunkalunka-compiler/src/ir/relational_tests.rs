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
