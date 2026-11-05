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

#[test]
fn rest_requires_a_visible_object_and_defines_a_fresh_object_value() {
    let source = "export function run(a,b){const {x,...rest}={x:a,y:b};return rest.y===b?1:0;}";
    for fault in 0..3 {
        let mut module = crate::lower_source(source, false).unwrap();
        verify_module(&module).unwrap();
        let function = &mut module.functions[0];
        let (result, object) = function
            .body
            .operations
            .iter_mut()
            .find_map(|op| match &mut op.kind {
                Op::ManagedObjectRest { result, object, .. } => Some((result, object)),
                _ => None,
            })
            .unwrap();
        match fault {
            0 => *object = *result, // forward reference
            1 => function.values[result.0] = Type::ManagedArray,
            _ => *result = *object, // duplicate definition
        }
        assert!(verify_module(&module).is_err());
    }
}

#[test]
fn computed_rest_exclusions_require_visible_managed_values() {
    let source = "export function run(a,b){const d=Object.create(null);const k='x';const {[k]:v,...rest}=d;return rest.x===undefined?1:0;}";
    for fault in 0..2 {
        let mut module = crate::lower_source(source, false).unwrap();
        verify_module(&module).unwrap();
        let function = &mut module.functions[0];
        let rest = function
            .body
            .operations
            .iter_mut()
            .find_map(|op| match &mut op.kind {
                Op::ManagedObjectRest {
                    result,
                    object,
                    excluded,
                } => Some((result, object, excluded)),
                _ => None,
            })
            .unwrap();
        let [ManagedKey::String(key)] = rest.2.as_mut_slice() else {
            panic!("missing computed exclusion")
        };
        *key = if fault == 0 { *rest.1 } else { *rest.0 };
        assert!(verify_module(&module).is_err());
    }
}
