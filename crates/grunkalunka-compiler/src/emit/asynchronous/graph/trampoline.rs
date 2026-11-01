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
    // A state can occur only once within one walk; store its index in the
    // current path and the walk's starting ID in dense, reusable arrays.
    // This avoids an ordered-map lookup and allocation for each chain.
    let mut visited_by = vec![usize::MAX; states.len()];
    let mut positions = vec![0; states.len()];
    let mut path = Vec::new();
    for start in 0..states.len() {
        if canonical[start].is_some() {
            continue;
        }
        path.clear();
        let mut current = start;
        loop {
            if let Some(target) = canonical[current] {
                for &index in &path {
                    canonical[index] = Some(target);
                }
                break;
            }
            if visited_by[current] == start {
                let cycle_start = positions[current];
                // Retain each cyclic state's original target and redirect a
                // predecessor only to its first entry into that cycle.
                for &index in &path[cycle_start..] {
                    canonical[index] = Some(index);
                }
                for &index in &path[..cycle_start] {
                    canonical[index] = Some(current);
                }
                break;
            }
            visited_by[current] = start;
            positions[current] = path.len();
            if let Some(next) = target_of_empty_jump(states, current, fresh) {
                path.push(current);
                current = next;
            } else {
                canonical[current] = Some(current);
                for &index in &path {
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

    // Independent reference semantics: follow the original hops until a
    // nonempty state or the first repeated state in a pure jump cycle.
    fn canonical_by_direct_walk(states: &[State<'_>], fresh: usize, start: usize) -> usize {
        let mut seen = vec![false; states.len()];
        let mut current = start;
        loop {
            if seen[current] {
                return current;
            }
            seen[current] = true;
            match target_of_empty_jump(states, current, fresh) {
                Some(next) => current = next,
                None => return current,
            }
        }
    }

    #[test]
    fn indexed_cycle_detection_matches_direct_walk_for_all_small_jump_graphs() {
        // Every state independently chooses one of N jump targets or Return.
        // This covers cycles, self-loops, paths entering cycles and terminals.
        for size in 1..=5 {
            let choices = size + 1;
            let count = (0..size).fold(1_usize, |value, _| value * choices);
            for encoding in 0..count {
                let mut code = encoding;
                let states = (0..size)
                    .map(|_| {
                        let target = code % choices;
                        code /= choices;
                        if target == size {
                            state(End::Return(ValueId(0)))
                        } else {
                            state(End::Jump(target))
                        }
                    })
                    .collect::<Vec<_>>();
                let computed = canonical_targets(&states, 0);
                for (start, &target) in computed.iter().enumerate() {
                    assert_eq!(
                        target,
                        canonical_by_direct_walk(&states, 0, start),
                        "size={size} encoding={encoding} state={start}"
                    );
                }
            }
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
