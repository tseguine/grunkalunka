use super::*;

#[test]
fn spread_operands_require_visible_object_and_boxed_value() {
    let source = "export function run(a,b){const source={x:a};const copy={...source};return copy.x===a?a:b;}";
    for fault in 0..6 {
        let mut module = crate::lower_source(source, false).unwrap();
        verify_module(&module).unwrap();
        if fault == 0 {
            continue;
        }
        let future = module.functions[0]
            .body
            .operations
            .iter()
            .find_map(|operation| match operation.kind {
                Op::ManagedGetOwn { result, .. } => Some(result),
                _ => None,
            })
            .unwrap();
        let spread = module.functions[0]
            .body
            .operations
            .iter_mut()
            .find_map(|operation| match &mut operation.kind {
                Op::ManagedObjectSpread { object, input } => Some((object, input)),
                _ => None,
            })
            .unwrap();
        match fault {
            1 => *spread.0 = *spread.1,
            2 => *spread.1 = *spread.0,
            3 => *spread.0 = ValueId(usize::MAX),
            4 => *spread.1 = ValueId(usize::MAX),
            _ => *spread.1 = future,
        }
        assert!(verify_module(&module).is_err());
    }
}
