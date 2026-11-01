//! Verify the splitter's private continuation graph before rendering WAT.
//! The source IR verifier checks language semantics; this catches malformed
//! backend edges as compiler diagnostics instead of unbound label emission or
//! an index/pattern panic while rendering dispatcher states.
use super::{End, MAX_STATES, Splitter};

mod trampoline;
pub(super) use trampoline::thread_empty_jumps;

pub(super) fn verify(split: &Splitter<'_>, entries: &[Option<usize>]) -> Result<(), String> {
    let size = split.states.len();
    if size == 0 || size > MAX_STATES {
        return Err("async continuation graph has an invalid state count".into());
    }
    if entries.is_empty() {
        return Err("async continuation graph has no function entry".into());
    }
    for (index, state) in split.states.iter().enumerate() {
        if state.function >= entries.len() {
            return Err(format!(
                "async continuation state {index} has an invalid function ID"
            ));
        }
        let edges = match &state.end {
            End::AdoptPromise { subscription } => [Some(*subscription), None],
            End::AdoptSubscription { reaction, .. }
            | End::AwaitStarted { reaction }
            | End::AwaitPromise { reaction, .. } => [Some(*reaction), None],
            End::StartedClosure { next, .. }
            | End::EnterClosure { next }
            | End::Jump(next)
            | End::Copy { next, .. } => [Some(*next), None],
            End::Reaction {
                fulfilled,
                rejected,
            }
            | End::ReturnedClosure {
                fulfilled,
                rejected,
                ..
            } => [Some(*fulfilled), Some(*rejected)],
            End::InvokeClosure { returned, .. } => [Some(*returned), None],
            End::Test { yes, no, .. } | End::SettledTest { yes, no, .. } => [Some(*yes), Some(*no)],
            End::Suspend { resume, .. } => [Some(*resume), None],
            End::Return(_) | End::Reject(_) | End::Trap => [None, None],
        };
        for destination in edges.into_iter().flatten() {
            if destination >= size {
                return Err(format!(
                    "async continuation state {index} references invalid state {destination}"
                ));
            }
            // Direct graph edges run within the current activation. Entering
            // another async closure happens through InvokeClosure and its
            // separate module entry; returning to the caller happens through
            // the scheduler, not a cross-function graph edge. Crossing here
            // would read/write locals from the wrong physical coroutine frame.
            if split.states[destination].function != state.function {
                return Err(format!(
                    "async continuation state {index} crosses function boundary to state {destination}"
                ));
            }
        }
        if let End::AdoptPromise { subscription } = &state.end
            && !matches!(
                split.states[*subscription].end,
                End::AdoptSubscription { .. }
            )
        {
            return Err(format!(
                "async continuation state {index} lacks an adoption subscription"
            ));
        }
    }
    for (function, entry) in entries.iter().enumerate() {
        let Some(index) = *entry else {
            continue;
        };
        if function == 0
            || index >= size
            || split.states[index].function != function
            || !matches!(split.states[index].end, End::EnterClosure { .. })
        {
            return Err(format!(
                "async continuation graph has an invalid entry for function {function}"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::State;
    use super::*;
    use crate::ir::ValueId;

    fn state(function: usize, end: End<'static>) -> State<'static> {
        State {
            function,
            operations: &[],
            resumed: None,
            end,
        }
    }

    fn graph(states: Vec<State<'static>>) -> Splitter<'static> {
        Splitter {
            states,
            ..Splitter::default()
        }
    }

    #[test]
    fn checks_all_control_edge_variants_without_traversing_source_regions() {
        let v = ValueId(0);
        let invalid = vec![
            End::AdoptPromise { subscription: 8 },
            End::AdoptSubscription {
                input: v,
                reaction: 8,
            },
            End::AwaitStarted { reaction: 8 },
            End::StartedClosure { result: v, next: 8 },
            End::AwaitPromise {
                input: v,
                reaction: 8,
            },
            End::Reaction {
                fulfilled: 0,
                rejected: 8,
            },
            End::InvokeClosure {
                callee: v,
                arguments: &[],
                returned: 8,
            },
            End::EnterClosure { next: 8 },
            End::ReturnedClosure {
                result: v,
                fulfilled: 8,
                rejected: 0,
            },
            End::Jump(8),
            End::Copy {
                input: v,
                result: v,
                next: 8,
            },
            End::Test {
                condition: v,
                yes: 0,
                no: 8,
            },
            End::SettledTest {
                input: v,
                yes: 8,
                no: 0,
            },
            End::Suspend {
                input: v,
                rejected: false,
                resume: 8,
            },
        ];
        for end in invalid {
            let split = graph(vec![state(0, end)]);
            let error = verify(&split, &[None]).expect_err("out-of-range target");
            assert!(error.contains("references invalid state 8"), "{error}");
        }
    }

    #[test]
    fn rejects_cross_function_edges_even_when_state_ids_are_in_bounds() {
        let value = ValueId(0);
        for (start, end) in [
            (
                End::Jump(1),
                End::Return(value),
            ),
            (
                End::Reaction {
                    fulfilled: 1,
                    rejected: 0,
                },
                End::Reject(value),
            ),
            (
                End::Suspend {
                    input: value,
                    rejected: false,
                    resume: 1,
                },
                End::Return(value),
            ),
        ] {
            let split = graph(vec![state(0, start), state(1, end)]);
            let error = verify(&split, &[None, None])
                .expect_err("continuations may not enter another function's frame");
            assert!(
                error.contains("crosses function boundary to state 1"),
                "{error}"
            );
        }
        let split = graph(vec![state(0, End::Jump(1)), state(0, End::Return(value))]);
        verify(&split, &[None, None]).expect("same-function continuation remains valid");
    }

    #[test]
    fn checks_adoption_kind_and_private_function_entries() {
        let v = ValueId(0);
        let split = graph(vec![
            state(0, End::Jump(1)),
            state(0, End::Return(v)),
            state(
                0,
                End::AdoptSubscription {
                    input: v,
                    reaction: 1,
                },
            ),
            state(0, End::AdoptPromise { subscription: 2 }),
            state(1, End::EnterClosure { next: 5 }),
            state(1, End::Return(v)),
        ]);
        verify(&split, &[None, Some(4)]).expect("well-formed private graph");
        assert!(
            verify(&split, &[None, Some(1)])
                .unwrap_err()
                .contains("invalid entry for function 1")
        );
        assert!(
            verify(&split, &[Some(0), Some(4)])
                .unwrap_err()
                .contains("invalid entry for function 0")
        );

        let invalid = graph(vec![
            state(0, End::AdoptPromise { subscription: 1 }),
            state(0, End::Return(v)),
        ]);
        assert!(
            verify(&invalid, &[None])
                .unwrap_err()
                .contains("lacks an adoption subscription")
        );

        let invalid_function = graph(vec![state(2, End::Trap)]);
        assert!(
            verify(&invalid_function, &[None])
                .unwrap_err()
                .contains("invalid function ID")
        );
    }
}
