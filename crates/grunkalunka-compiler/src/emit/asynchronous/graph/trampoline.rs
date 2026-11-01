//! Thread administrative jumps in the validated async continuation graph.
//!
//! An empty Jump state with no resumed value has no observable effects, except
//! for the special start state, which initializes the first activation. Redirect
//! incoming edges to its eventual target without deleting states or changing
//! protocol state IDs, scheduling boundaries, or the 4,096-state admission cap.
use super::super::{End, Splitter, State};

fn target_of_empty_jump(states: &[State<'_>], index: usize, fresh: usize) -> Option<usize> {
    let state = &states[index];
    if index == fresh || !state.operations.is_empty() || state.resumed.is_some() {
        return None;
    }
    match &state.end {
        End::Jump(next) => Some(*next),
        _ => None,
    }
}

/// Resolve all targets in near-linear time, with canonical IDs for cycles.
/// A pure jump cycle is deliberately preserved: it is not an error or a
/// license to invent a completion for a source-level infinite loop.
fn canonical_targets(states: &[State<'_>], fresh: usize) -> Vec<usize> {
    let mut canonical = vec![None; states.len()];
    // Positions in an active DFS path are indexed by state ID rather than
    // inserting every step into a fresh balanced tree. A generation counter
    // avoids clearing this table for each disconnected path or cycle.
    let mut seen_generation = vec![0usize; states.len()];
    let mut path_position = vec![0usize; states.len()];
    let mut generation = 0usize;
    for start in 0..states.len() {
        if canonical[start].is_some() {
            continue;
        }
        generation += 1; // At most MAX_STATES generations.
        let mut path = Vec::new();
        let mut current = start;
        loop {
            if let Some(target) = canonical[current] {
                for index in path {
                    canonical[index] = Some(target);
                }
                break;
            }
            if seen_generation[current] == generation {
                let cycle_start = path_position[current];
                // The cycle's original edges remain intact; only upstream
                // administrative paths redirect to their first cycle node.
                for &index in &path[cycle_start..] {
                    canonical[index] = Some(index);
                }
                for &index in &path[..cycle_start] {
                    canonical[index] = Some(current);
                }
                break;
            }
            seen_generation[current] = generation;
            path_position[current] = path.len();
            if let Some(next) = target_of_empty_jump(states, current, fresh) {
                path.push(current);
                current = next;
            } else {
                canonical[current] = Some(current);
                for index in path {
                    canonical[index] = Some(current);
                }
                break;
            }
        }
    }
    canonical
        .into_iter()
        .map(|target| target.expect("all states were resolved"))
        .collect()
}

pub(in crate::emit::asynchronous) fn thread_empty_jumps(split: &mut Splitter<'_>, fresh: usize) {
    // The caller has already verified that all targets are in range.
    let canonical = canonical_targets(&split.states, fresh);
    let remap = |target: &mut usize| *target = canonical[*target];
    for state in &mut split.states {
        match &mut state.end {
            End::AdoptPromise { subscription } => remap(subscription),
            End::AdoptSubscription { reaction, .. }
            | End::AwaitStarted { reaction }
            | End::AwaitPromise { reaction, .. } => remap(reaction),
            End::StartedClosure { next, .. }
            | End::EnterClosure { next }
            | End::Jump(next)
            | End::Copy { next, .. } => remap(next),
            End::Reaction {
                fulfilled,
                rejected,
            }
            | End::ReturnedClosure {
                fulfilled,
                rejected,
                ..
            } => {
                remap(fulfilled);
                remap(rejected);
            }
            End::InvokeClosure { returned, .. } => remap(returned),
            End::Test { yes, no, .. } | End::SettledTest { yes, no, .. } => {
                remap(yes);
                remap(no);
            }
            End::Suspend { resume, .. } => remap(resume),
            End::Return(_) | End::Reject(_) | End::Trap => (),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Op, Operation, ValueId};
    use oxc_span::Span;

    fn state(end: End<'static>) -> State<'static> {
        State {
            function: 0,
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
    fn threads_empty_jump_chains_without_eliding_start_or_resumed_states() {
        let v = ValueId(0);
        let mut resumed = state(End::Jump(4));
        resumed.resumed = Some(v);
        let mut split = graph(vec![
            state(End::Jump(1)), // initial state, never bypassed as a target
            state(End::Jump(2)),
            state(End::Jump(3)),
            resumed, // a resumed value is a real side effect
            state(End::Return(v)),
            state(End::Jump(0)), // may reach the initial state again
        ]);
        super::super::verify(&split, &[None]).unwrap();
        thread_empty_jumps(&mut split, 0);
        assert!(matches!(split.states[0].end, End::Jump(3)));
        assert!(matches!(split.states[1].end, End::Jump(3)));
        assert!(matches!(split.states[2].end, End::Jump(3)));
        assert!(matches!(split.states[3].end, End::Jump(4)));
        assert!(matches!(split.states[5].end, End::Jump(0)));
        super::super::verify(&split, &[None]).unwrap();
    }

    #[test]
    fn does_not_skip_states_that_emit_source_operations() {
        let operations = [Operation {
            span: Span::new(0, 0),
            kind: Op::Constant {
                result: ValueId(0),
                value: 7.0,
            },
        }];
        let mut split = Splitter {
            states: vec![
                state(End::Jump(1)),
                State {
                    function: 0,
                    operations: &operations,
                    resumed: None,
                    end: End::Jump(2),
                },
                state(End::Return(ValueId(0))),
            ],
            ..Splitter::default()
        };
        thread_empty_jumps(&mut split, 0);
        assert!(matches!(split.states[0].end, End::Jump(1)));
    }

    #[test]
    fn preserves_empty_jump_cycles_instead_of_inventing_completions() {
        let mut split = graph(vec![
            state(End::Jump(1)),
            state(End::Jump(2)),
            state(End::Jump(1)),
            state(End::Return(ValueId(0))),
        ]);
        thread_empty_jumps(&mut split, 0);
        assert!(matches!(split.states[0].end, End::Jump(1)));
        assert!(matches!(split.states[1].end, End::Jump(2)));
        assert!(matches!(split.states[2].end, End::Jump(1)));
    }

    #[test]
    fn resolves_a_full_budget_chain_without_tree_allocations_or_recursion() {
        use super::super::super::MAX_STATES;
        let mut split = graph(
            (0..MAX_STATES)
                .map(|index| {
                    if index + 1 == MAX_STATES {
                        state(End::Return(ValueId(7)))
                    } else {
                        state(End::Jump(index + 1))
                    }
                })
                .collect(),
        );
        thread_empty_jumps(&mut split, 0);
        for state in split.states.iter().take(MAX_STATES - 1) {
            assert!(matches!(state.end, End::Jump(target) if target == MAX_STATES - 1));
        }
        assert!(matches!(
            split.states[MAX_STATES - 1].end,
            End::Return(ValueId(7))
        ));
    }

    #[test]
    fn rewrites_branch_and_reaction_edges_without_touching_resume_boundaries() {
        let v = ValueId(0);
        let mut split = graph(vec![
            state(End::Jump(1)),
            state(End::Return(v)),
            state(End::Jump(3)),
            state(End::Jump(4)),
            state(End::Test {
                condition: v,
                yes: 2,
                no: 1,
            }),
            state(End::Reaction {
                fulfilled: 2,
                rejected: 3,
            }),
            state(End::Suspend {
                input: v,
                rejected: false,
                resume: 2,
            }),
        ]);
        thread_empty_jumps(&mut split, 0);
        assert!(matches!(
            split.states[4].end,
            End::Test { yes: 4, no: 1, .. }
        ));
        assert!(matches!(
            split.states[5].end,
            End::Reaction {
                fulfilled: 4,
                rejected: 4,
            }
        ));
        assert!(matches!(
            split.states[6].end,
            End::Suspend { resume: 4, .. }
        ));
    }
}
