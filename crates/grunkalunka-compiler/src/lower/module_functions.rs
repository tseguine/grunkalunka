//! Adapt source module declarations to the private typed closure calling ABI.
//! Planning reads original references; it never evaluates a source expression.
use super::*;
use oxc_ast::{AstKind, ast::Function};

fn needs_value(symbol: SymbolId, semantic: &Semantic<'_>) -> bool {
    semantic
        .scoping()
        .get_resolved_references(symbol)
        .any(|reference| {
            if !reference.is_value() {
                return false;
            }
            let mut ancestors = semantic.nodes().ancestor_kinds(reference.node_id());
            loop {
                match ancestors.next() {
                    Some(
                        AstKind::ParenthesizedExpression(_)
                        | AstKind::TSAsExpression(_)
                        | AstKind::TSTypeAssertion(_)
                        | AstKind::TSNonNullExpression(_)
                        | AstKind::TSSatisfiesExpression(_),
                    ) => (),
                    Some(AstKind::StaticMemberExpression(member))
                        if member.property.name == "call" => {}
                    Some(AstKind::CallExpression(call)) => {
                        return !matches!(invocation::callee(call),
                        Expression::Identifier(id) if id.reference_id.get().is_some_and(|id|
                            semantic.scoping().get_reference(id).node_id() == reference.node_id()));
                    }
                    // The existing typeof path answers a hoisted declaration
                    // directly, without reading or constructing its identity.
                    Some(AstKind::UnaryExpression(unary))
                        if unary.operator == UnaryOperator::Typeof =>
                    {
                        return false;
                    }
                    _ => return true,
                }
            }
        })
}

pub(crate) fn plan(
    semantic: &Semantic<'_>,
    declarations: &[&Function<'_>],
    functions: &mut [NumericFunction],
    signatures: &std::cell::RefCell<ir::ClosureSignatures>,
) -> Vec<ir::Function> {
    let base = functions.len();
    let mut adapters = Vec::new();
    for (source, function) in declarations.iter().zip(functions) {
        if source.r#async || !needs_value(function.symbol, semantic) {
            continue;
        }
        let signature = function.closure_signature(signatures);
        function.module_value = Some((ir::FunctionId(base + adapters.len()), signature));
        let span = source.span();
        let parameters: Vec<_> = (0..function.parameter_count)
            .map(|i| function.parameter_type(i))
            .collect();
        let slots = parameters
            .iter()
            .map(|ty| Slot {
                ty: *ty,
                owner: RegionId(0),
                mutable: true,
                parameter: true,
                span,
            })
            .collect();
        let mut operations = vec![Operation {
            span,
            kind: Op::ClosureEntry { captures: vec![] },
        }];
        let mut arguments = Vec::new();
        for index in 0..parameters.len() {
            let result = ValueId(index);
            operations.push(Operation {
                span,
                kind: Op::Load {
                    result,
                    slot: SlotId(index),
                },
            });
            arguments.push(result);
        }
        let result = ValueId(parameters.len());
        operations.push(Operation {
            span,
            kind: Op::Call {
                result,
                function: function.id,
                arguments,
            },
        });
        let mut values = parameters;
        values.push(function.return_type);
        adapters.push(ir::Function {
            return_type: function.return_type,
            parameter_count: function.parameter_count,
            slots,
            values,
            body: Region {
                id: RegionId(0),
                span,
                operations,
                exit: Exit::Return(result),
            },
        });
    }
    adapters
}
