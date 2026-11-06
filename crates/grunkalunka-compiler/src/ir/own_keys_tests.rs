use super::*;

#[test]
fn own_keys_checks_visible_inputs_and_array_results() {
    for fault in 0..6 {
        let mut module = crate::lower_source(
            "export function run(a,b){const k=Object.keys({x:a});return k.length;}",
            false,
        )
        .unwrap();
        verify_module(&module).unwrap();
        let root = &mut module.functions[0];
        let Op::ManagedOwnKeys { result, input } = &mut root
            .body
            .operations
            .iter_mut()
            .find(|op| matches!(op.kind, Op::ManagedOwnKeys { .. }))
            .unwrap()
            .kind
        else {
            unreachable!()
        };
        match fault {
            0 => *input = ValueId(usize::MAX),
            1 => *input = *result,
            2 => root.values[input.0] = Type::General,
            3 => root.values[result.0] = Type::ManagedObject,
            4 => *result = *input,
            _ => *result = ValueId(usize::MAX),
        }
        assert!(verify_module(&module).is_err(), "fault {fault}");
    }
}

#[test]
fn fallible_own_keys_checks_all_completion_outputs_and_cannot_use_sync_emission() {
    for fault in 0..10 {
        let mut module = crate::lower_source_profile(
            "export async function run(a,b){const k=Object.keys(a?{}:null);return k.length;}",
            false,
            Type::General,
            true,
        )
        .unwrap();
        verify_module_with_entry_type(&module, Type::General).unwrap();
        if fault == 0 {
            assert!(
                crate::emit::emit_with_entry_type(&module, Type::General)
                    .unwrap_err()
                    .contains("fallible own-keys")
            );
            continue;
        }
        let root = &mut module.functions[0];
        let Op::TryManagedOwnKeys {
            result,
            input,
            failure,
        } = &mut root
            .body
            .operations
            .iter_mut()
            .find(|op| matches!(op.kind, Op::TryManagedOwnKeys { .. }))
            .unwrap()
            .kind
        else {
            unreachable!()
        };
        match fault {
            1 => root.values[input.0] = Type::ManagedObject,
            2 => *input = *result,
            3 => root.values[result.0] = Type::ManagedValue,
            4 => root.values[failure.threw.0] = Type::Number,
            5 => root.values[failure.reason.0] = Type::AsyncValue,
            6 => failure.reason = failure.threw,
            7 => *result = *input,
            8 => failure.threw = ValueId(usize::MAX),
            _ => failure.reason = ValueId(usize::MAX),
        }
        assert!(
            verify_module_with_entry_type(&module, Type::General).is_err(),
            "fault {fault}"
        );
    }
}

#[test]
fn has_own_checks_receiver_key_visibility_and_boolean_result() {
    let source = "export function run(a,b){const d={x:undefined};const name='x';return Object.hasOwn(d,name)?1:0;}";
    for fault in 0..4 {
        let mut module = crate::lower_source(source, false).unwrap();
        verify_module(&module).unwrap();
        let function = &mut module.functions[0];
        let (input, key, result) = function
            .body
            .operations
            .iter_mut()
            .find_map(|op| match &mut op.kind {
                Op::ManagedHasOwn { input, key, result } => Some((input, key, result)),
                _ => None,
            })
            .expect("source own-property operation");
        let ManagedKey::String(key) = key else {
            panic!("source dynamic key")
        };
        match fault {
            0 => *input = *key,
            1 => *key = *input,
            2 => *key = *result,
            _ => function.values[result.0] = Type::Number,
        }
        assert!(verify_module(&module).is_err());
    }
}

#[test]
fn has_own_admits_string_keys_on_known_arrays_and_objects_only() {
    for (body, admitted) in [
        (
            "const d=Object.create(null);return Object.hasOwn(d,'toString')?1:0;",
            true,
        ),
        ("return Object['hasOwn'].call(null,[,],'0')?1:0;", true),
        (
            "const {row}={row:[a]};return Object.hasOwn(row,'length')?1:0;",
            true,
        ),
        ("return Object.hasOwn({},a)?1:0;", false),
        ("return Object.hasOwn(a?{}:null,'x')?1:0;", false),
        ("return Object.hasOwn(...[{},'x'])?1:0;", false),
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let result = crate::compile(&source, ts);
            if admitted {
                assert!(result.is_ok(), "{source}: {:?}", result.err());
            } else {
                let err = result.err().expect("unsupported input");
                assert!(
                    err.contains("unsupported") && !err.contains("internal IR"),
                    "{source}: {err}"
                );
            }
        }
    }
}

#[test]
fn has_own_can_emit_a_key_longer_than_the_fixed_wasm_constructor_limit() {
    let key = "a".repeat(10_001);
    let source = format!("export function run(a,b){{return Object.hasOwn({{}},'{key}')?1:0;}}");
    for ts in [false, true] {
        let artifact = crate::compile(&source, ts).unwrap();
        assert!(artifact.wat.contains("local.set $gc_literal"));
    }
}
