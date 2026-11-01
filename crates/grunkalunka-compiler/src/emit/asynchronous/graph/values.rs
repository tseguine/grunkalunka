//! Validate value-table references in the split async continuation graph.
//! Source IR verification proves the input operations, but backend-generated
//! resume/return/copy states must not index a different or absent value.
use super::super::{End, Splitter};
use crate::ir::{self, ValueId};

pub(super) fn verify_state_values(
    split: &Splitter<'_>,
    module: &ir::Module,
) -> Result<(), String> {
    for (state_id, state) in split.states.iter().enumerate() {
        let Some(function) = module.functions.get(state.function) else {
            return Err(format!(
                "async continuation state {state_id} references invalid function {}",
                state.function
            ));
        };
        let check = |value: ValueId| {
            if value.0 >= function.values.len() {
                Err(format!(
                    "async continuation state {state_id} references invalid value {} in function {}",
                    value.0, state.function
                ))
            } else {
                Ok(())
            }
        };
        if let Some(resumed) = state.resumed {
            check(resumed)?;
        }
        match &state.end {
            End::AdoptSubscription { input, .. }
            | End::AwaitPromise { input, .. }
            | End::SettledTest { input, .. }
            | End::Suspend { input, .. } => check(*input)?,
            End::StartedClosure { result, .. }
            | End::ReturnedClosure { result, .. } => check(*result)?,
            End::InvokeClosure {
                callee, arguments, ..
            } => {
                check(*callee)?;
                for argument in *arguments {
                    check(*argument)?;
                }
            }
            End::Copy { input, result, .. } => {
                check(*input)?;
                check(*result)?;
            }
            End::Test { condition, .. } => check(*condition)?,
            End::Return(value) | End::Reject(value) => check(*value)?,
            End::AdoptPromise { .. }
            | End::AwaitStarted { .. }
            | End::Reaction { .. }
            | End::EnterClosure { .. }
            | End::Jump(_)
            | End::Trap => (),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{ClosureSignatures, Exit, Function, Region, RegionId, Type};
    use oxc_span::Span;

    fn module(values: usize) -> ir::Module {
        ir::Module {
            closure_signatures: ClosureSignatures::default(),
            functions: vec![Function {
                return_type: Type::General,
                parameter_count: 0,
                slots: Vec::new(),
                values: vec![Type::General; values],
                body: Region {
                    id: RegionId(0),
                    span: Span::new(0, 0),
                    operations: Vec::new(),
                    exit: Exit::Fallthrough,
                },
            }],
        }
    }

    fn state(end: End<'_>) -> super::super::State<'_> {
        super::super::State {
            function: 0,
            operations: &[],
            resumed: None,
            end,
        }
    }

    #[test]
    fn every_value_bearing_continuation_checks_its_own_function_table() {
        let good = ValueId(0);
        let bad = ValueId(9);
        let invalid_arguments = [bad];
        let cases = vec![
            End::Return(bad),
            End::Reject(bad),
            End::StartedClosure {
                result: bad,
                next: 0,
            },
            End::AwaitPromise {
                input: bad,
                reaction: 0,
            },
            End::AdoptSubscription {
                input: bad,
                reaction: 0,
            },
            End::ReturnedClosure {
                result: bad,
                fulfilled: 0,
                rejected: 0,
            },
            End::Copy {
                input: bad,
                result: good,
                next: 0,
            },
            End::Copy {
                input: good,
                result: bad,
                next: 0,
            },
            End::Test {
                condition: bad,
                yes: 0,
                no: 0,
            },
            End::SettledTest {
                input: bad,
                yes: 0,
                no: 0,
            },
            End::Suspend {
                input: bad,
                rejected: false,
                resume: 0,
            },
            End::InvokeClosure {
                callee: good,
                arguments: &invalid_arguments,
                returned: 0,
            },
            End::InvokeClosure {
                callee: bad,
                arguments: &[],
                returned: 0,
            },
        ];
        let module = module(1);
        for end in cases {
            let split = Splitter {
                states: vec![state(end)],
                ..Splitter::default()
            };
            let error = verify_state_values(&split, &module)
                .expect_err("missing continuation operands must be rejected");
            assert!(error.contains("invalid value 9"), "{error}");
        }
    }

    #[test]
    fn resumed_values_and_function_ownership_are_checked() {
        let module = module(1);
        let mut resumed = state(End::Jump(0));
        resumed.resumed = Some(ValueId(99));
        let split = Splitter {
            states: vec![resumed],
            ..Splitter::default()
        };
        assert!(verify_state_values(&split, &module)
            .unwrap_err()
            .contains("invalid value 99"));

        let mut wrong_function = state(End::Return(ValueId(0)));
        wrong_function.function = 4;
        let split = Splitter {
            states: vec![wrong_function],
            ..Splitter::default()
        };
        assert!(verify_state_values(&split, &module)
            .unwrap_err()
            .contains("invalid function 4"));

        let split = Splitter {
            states: vec![state(End::Copy {
                input: ValueId(0),
                result: ValueId(0),
                next: 0,
            })],
            ..Splitter::default()
        };
        verify_state_values(&split, &module)
            .expect("existing in-range continuation value IDs remain valid");
    }
}
