use super::*;

fn seed() -> Module {
    crate::lower_source_profile(
        "export async function run(a,b){try{return a;}finally{await b;}}",
        false,
        Type::General,
        true,
    )
    .unwrap()
}

#[test]
fn finally_completion_matrix_preserves_only_on_normal_cleanup() {
    let kinds = [
        Completion::NORMAL,
        Completion::RETURN,
        Completion::THROW,
        Completion::BREAK,
        Completion::CONTINUE,
    ];
    for pending in kinds {
        for cleanup in kinds {
            assert_eq!(
                pending.finalized_by(cleanup),
                if cleanup == Completion::NORMAL {
                    pending
                } else {
                    cleanup
                }
            );
        }
        let cleanup = Completion::NORMAL.union(Completion::THROW);
        assert_eq!(
            pending.finalized_by(cleanup),
            pending.union(Completion::THROW)
        );
    }
}

#[test]
fn finally_capture_types_and_lexical_visibility_are_checked() {
    verify_module_with_entry_type(&seed(), Type::General).unwrap();
    for change in 0..5 {
        let mut module = seed();
        let f = &mut module.functions[0];
        let Op::TryFinally {
            body,
            finalizer,
            returned,
            reason,
            ..
        } = &mut f.body.operations.last_mut().unwrap().kind
        else {
            panic!("finally");
        };
        match change {
            0 => f.values[returned.0] = Type::General,
            1 => f.values[reason.0] = Type::Number,
            2 => finalizer.exit = Exit::Return(*returned),
            3 => {
                let Exit::Return(input) = body.exit else {
                    panic!("pending return");
                };
                finalizer.exit = Exit::Return(input);
            }
            _ => *reason = *returned,
        }
        assert!(
            verify_module_with_entry_type(&module, Type::General).is_err(),
            "{change}"
        );
    }
}

#[test]
fn finally_continuations_must_complete_and_regions_must_be_distinct() {
    for change in 0..4 {
        let mut module = seed();
        let f = &mut module.functions[0];
        let Op::TryFinally {
            body,
            finalizer,
            continuation,
            reason,
            ..
        } = &mut f.body.operations.last_mut().unwrap().kind
        else {
            panic!("finally");
        };
        match change {
            0 => continuation.exit = Exit::Fallthrough,
            1 => continuation.exit = Exit::Break,
            2 => finalizer.id = body.id,
            _ => continuation.exit = Exit::Return(*reason),
        }
        assert!(
            verify_module_with_entry_type(&module, Type::General).is_err(),
            "{change}"
        );
    }
}
