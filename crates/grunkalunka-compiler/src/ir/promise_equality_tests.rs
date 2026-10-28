use super::*;

#[test]
fn promise_identity_equality_requires_visible_identity_values() {
    let source = "export async function run(a,b){const f=async()=>{await a;return b;};const p=f();const primitive=null;const number=0;const snapshot=Promise.resolve(a);return p===p;}";
    let make = || crate::lower_source_profile(source, false, Type::General, true).unwrap();
    verify_module_with_entry_type(&make(), Type::General).unwrap();
    for change in 0..7 {
        let mut module = make();
        let f = &mut module.functions[0];
        let index = f
            .body
            .operations
            .iter()
            .position(|o| matches!(o.kind, Op::PromiseIdentityEqual { .. }))
            .unwrap();
        let general = f
            .body
            .operations
            .iter()
            .find_map(|o| {
                if let Op::Nullish { result, .. } = o.kind {
                    Some(result)
                } else {
                    None
                }
            })
            .unwrap();
        let number = f
            .body
            .operations
            .iter()
            .find_map(|o| {
                if let Op::Constant { result, .. } = o.kind {
                    Some(result)
                } else {
                    None
                }
            })
            .unwrap();
        let snapshot = f
            .body
            .operations
            .iter()
            .find_map(|o| {
                if let Op::FulfillPromise { result, .. } = o.kind {
                    Some(result)
                } else {
                    None
                }
            })
            .unwrap();
        let Op::PromiseIdentityEqual {
            left,
            right,
            result,
            ..
        } = &mut f.body.operations[index].kind
        else {
            unreachable!()
        };
        match change {
            0 => *left = ValueId(999),
            1 => *right = *result,
            2 => *left = general,
            3 => *right = number,
            4 => *left = snapshot,
            5 => f.values[result.0] = Type::General,
            _ => f.body.operations.swap(0, index),
        }
        assert!(verify_module_with_entry_type(&module, Type::General).is_err());
    }
}
