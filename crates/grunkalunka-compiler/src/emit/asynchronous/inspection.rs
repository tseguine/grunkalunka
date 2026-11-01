//! One ordered IR walk per source function for the async backend's feature
//! decisions. A presence check never executes an operation, but the source
//! entry call restriction must retain its per-function diagnostic scope.
use crate::ir::{self, Op};

#[derive(Default)]
pub(super) struct Needs {
    pub queued: bool,
    pub errors: bool,
    pub starters: bool,
    pub calls_to_entry: Vec<bool>,
}

pub(super) fn inspect(module: &ir::Module) -> Needs {
    let mut needs = Needs {
        calls_to_entry: vec![false; module.functions.len()],
        ..Needs::default()
    };
    for (index, function) in module.functions.iter().enumerate() {
        let synchronous_helper = index != 0 && !ir::is_async_closure(function);
        ir::visit_operations(&function.body, &mut |op| match op {
            Op::StartAsyncClosure { .. } => {
                needs.queued = true;
                needs.starters |= synchronous_helper;
            }
            Op::AwaitPromise { .. } | Op::MaterializePromise { .. } => needs.queued = true,
            Op::AdoptPromise { .. } => {
                needs.queued = true;
                needs.errors = true;
            }
            Op::Call { function, .. } if function.0 == 0 => {
                needs.calls_to_entry[index] = true;
            }
            _ => (),
        });
    }
    needs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        ClosureSignatures, Exit, Function, FunctionId, Operation, Region, RegionId, Type, ValueId,
    };
    use oxc_span::Span;

    fn region(operations: Vec<Op>) -> Region {
        let span = Span::new(0, 0);
        Region {
            id: RegionId(0),
            span,
            operations: operations
                .into_iter()
                .map(|kind| Operation { span, kind })
                .collect(),
            exit: Exit::Fallthrough,
        }
    }

    fn function(body: Region, asynchronous: bool) -> Function {
        let mut body = body;
        if asynchronous {
            body.operations.insert(
                0,
                Operation {
                    span: Span::new(0, 0),
                    kind: Op::AsyncClosureEntry {
                        captures: Vec::new(),
                    },
                },
            );
        }
        Function {
            return_type: Type::General,
            parameter_count: 0,
            slots: Vec::new(),
            values: Vec::new(),
            body,
        }
    }

    #[test]
    fn matches_the_prior_independent_scans_including_nested_helpers() {
        let value = ValueId(0);
        let module = ir::Module {
            closure_signatures: ClosureSignatures::default(),
            functions: vec![
                function(
                    region(vec![
                        Op::AwaitPromise {
                            result: value,
                            input: value,
                        },
                        Op::Call {
                            function: FunctionId(0),
                            result: value,
                            arguments: Vec::new(),
                        },
                    ]),
                    false,
                ),
                function(
                    region(vec![Op::Block(region(vec![Op::StartAsyncClosure {
                        result: value,
                        callee: value,
                        arguments: Vec::new(),
                    }]))]),
                    false,
                ),
                function(
                    region(vec![Op::StartAsyncClosure {
                        result: value,
                        callee: value,
                        arguments: Vec::new(),
                    }]),
                    true,
                ),
                function(
                    region(vec![Op::AdoptPromise {
                        result: value,
                        input: value,
                    }]),
                    false,
                ),
            ],
        };
        let actual = inspect(&module);
        assert!(actual.queued);
        assert!(actual.errors);
        assert!(actual.starters);
        assert_eq!(actual.calls_to_entry, vec![true, false, false, false]);

        // Independent characterization of the old detection passes. Keep this
        // as a regression oracle rather than deriving assertions from inspect.
        let old_queued = module.functions.iter().any(|function| {
            ir::any_operation(&function.body, &mut |op| {
                matches!(
                    op,
                    Op::StartAsyncClosure { .. }
                        | Op::AwaitPromise { .. }
                        | Op::AdoptPromise { .. }
                        | Op::MaterializePromise { .. }
                )
            })
        });
        let old_errors = module.functions.iter().any(|function| {
            ir::any_operation(&function.body, &mut |op| {
                matches!(op, Op::AdoptPromise { .. })
            })
        });
        let old_starters = module.functions.iter().skip(1).any(|function| {
            !ir::is_async_closure(function)
                && ir::any_operation(&function.body, &mut |op| {
                    matches!(op, Op::StartAsyncClosure { .. })
                })
        });
        let old_entry_calls = module
            .functions
            .iter()
            .map(|function| {
                ir::any_operation(
                    &function.body,
                    &mut |op| matches!(op, Op::Call { function, .. } if function.0 == 0),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(actual.queued, old_queued);
        assert_eq!(actual.errors, old_errors);
        assert_eq!(actual.starters, old_starters);
        assert_eq!(actual.calls_to_entry, old_entry_calls);
    }

    #[test]
    fn empty_module_or_unrelated_operations_have_no_feature_flags() {
        let empty = ir::Module {
            closure_signatures: ClosureSignatures::default(),
            functions: Vec::new(),
        };
        let absent = inspect(&empty);
        assert!(!absent.queued && !absent.errors && !absent.starters);
        assert!(absent.calls_to_entry.is_empty());

        let unrelated = ir::Module {
            closure_signatures: ClosureSignatures::default(),
            functions: vec![function(
                region(vec![Op::Constant {
                    result: ValueId(0),
                    value: 2.0,
                }]),
                false,
            )],
        };
        let absent = inspect(&unrelated);
        assert!(!absent.queued && !absent.errors && !absent.starters);
        assert_eq!(absent.calls_to_entry, vec![false]);
    }
}
