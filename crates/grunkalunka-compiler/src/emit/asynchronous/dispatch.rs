//! Precompute source-closure targets for the async emitter's indirect calls.
//! Every target is kept in original function-index order so the emitted WAT
//! and the runtime's dynamic function dispatch order remain unchanged.
use crate::ir::{self, Type};
use std::collections::BTreeMap;

pub(super) fn targets(module: &ir::Module) -> BTreeMap<usize, Vec<usize>> {
    let mut by_signature = BTreeMap::<usize, Vec<usize>>::new();
    for (id, function) in module.functions.iter().enumerate().skip(1) {
        if !ir::is_async_closure(function) {
            continue;
        }
        let parameters = function
            .slots
            .iter()
            .take(function.parameter_count)
            .map(|slot| slot.ty)
            .collect::<Vec<Type>>();
        if let Some(signature) = module.closure_signatures.id_async(&parameters) {
            by_signature.entry(signature).or_default().push(id);
        }
    }
    by_signature
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{ClosureSignatures, Exit, Function, Op, Operation, Region, RegionId, Slot};
    use oxc_span::Span;

    fn function(params: &[Type], asynchronous: bool) -> Function {
        let span = Span::new(0, 0);
        Function {
            return_type: Type::General,
            parameter_count: params.len(),
            slots: params
                .iter()
                .map(|ty| Slot {
                    ty: *ty,
                    owner: RegionId(0),
                    mutable: true,
                    parameter: true,
                    span,
                })
                .collect(),
            values: Vec::new(),
            body: Region {
                id: RegionId(0),
                span,
                operations: if asynchronous {
                    vec![Operation {
                        span,
                        kind: Op::AsyncClosureEntry {
                            captures: Vec::new(),
                        },
                    }]
                } else {
                    Vec::new()
                },
                exit: Exit::Fallthrough,
            },
        }
    }

    #[test]
    fn index_retains_function_order_and_skips_sync_helpers() {
        let mut signatures = ClosureSignatures::default();
        let number = signatures.intern_async(vec![Type::Number]);
        let general = signatures.intern_async(vec![Type::General]);
        let module = ir::Module {
            closure_signatures: signatures,
            functions: vec![
                function(&[], false),
                function(&[Type::Number], true),
                function(&[Type::Number], false),
                function(&[Type::General], true),
                function(&[Type::Number], true),
                function(&[Type::Number], false),
            ],
        };
        let result = targets(&module);
        assert_eq!(result.get(&number), Some(&vec![1, 4]));
        assert_eq!(result.get(&general), Some(&vec![3]));
        assert_eq!(result.len(), 2);

        // Characterize the original per-invocation scan as an independent
        // oracle, including the negative/uninterned signature case.
        for signature in [number, general, usize::MAX] {
            let previous = module
                .functions
                .iter()
                .enumerate()
                .skip(1)
                .filter(|(_, function)| ir::is_async_closure(function))
                .filter(|(_, function)| {
                    let params = function
                        .slots
                        .iter()
                        .take(function.parameter_count)
                        .map(|slot| slot.ty)
                        .collect::<Vec<_>>();
                    module.closure_signatures.id_async(&params) == Some(signature)
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            assert_eq!(
                result.get(&signature).cloned().unwrap_or_default(),
                previous
            );
        }
    }
}
