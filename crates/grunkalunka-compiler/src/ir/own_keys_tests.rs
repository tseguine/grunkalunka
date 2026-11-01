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
