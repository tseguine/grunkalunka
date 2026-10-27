//! Positive source facts for represented fulfilled Promise inputs. This query
//! follows original symbols and constructors; it never executes a source call.
use super::*;
use oxc_ast::AstKind;
use oxc_syntax::node::NodeId;
use std::collections::{BTreeMap, BTreeSet};

fn callees(
    expression: &Expression<'_>,
    returns: &callbacks::ReturnOrigins<'_, '_, '_>,
) -> Option<BTreeSet<NodeId>> {
    let (sources, mut bodies) = returns.expression(expression, &BTreeMap::new(), 0, 0);
    for source in sources {
        bodies.extend(returns.targets.get(&source)?);
    }
    (!bodies.is_empty()).then_some(bodies)
}

pub(crate) fn parameters(semantic: &Semantic<'_>) -> BTreeSet<SymbolId> {
    let mut arrows = BTreeMap::new();
    let mut definitions = Vec::new();
    let mut calls = Vec::new();
    let mut has_constructor = false;
    for node in semantic.nodes().iter() {
        match node.kind() {
            AstKind::ArrowFunctionExpression(arrow) => {
                arrows.insert(arrow.node_id.get(), arrow);
            }
            AstKind::VariableDeclaration(declaration) => {
                for local in &declaration.declarations {
                    if let BindingPattern::BindingIdentifier(binding) = &local.id
                        && let (Some(symbol), Some(initializer)) =
                            (binding.symbol_id.get(), local.init.as_ref())
                    {
                        definitions.push((symbol, initializer));
                    }
                }
            }
            AstKind::AssignmentExpression(assignment)
                if matches!(
                    assignment.operator,
                    AssignmentOperator::Assign
                        | AssignmentOperator::LogicalAnd
                        | AssignmentOperator::LogicalOr
                        | AssignmentOperator::LogicalNullish
                ) =>
            {
                if let Some(identifier) = assignment
                    .left
                    .as_simple_assignment_target()
                    .and_then(|target| target_identifier(target, 0).ok().flatten())
                    && let Some(symbol) = identifier.reference_id.get().and_then(|reference| {
                        semantic.scoping().get_reference(reference).symbol_id()
                    })
                {
                    definitions.push((symbol, &assignment.right));
                }
            }
            AstKind::CallExpression(call) => {
                has_constructor |= awaited::intrinsic(call, semantic)
                    .ok()
                    .flatten()
                    .is_some_and(|intrinsic| !intrinsic.rejected);
                calls.push(call);
            }
            _ => (),
        }
    }
    // No represented Promise input can originate without a fulfilled constructor.
    // Facts below grow monotonically over finite original symbol/arrow sets;
    // source budgets remain the actual lowerer's responsibility.
    if !has_constructor {
        return BTreeSet::new();
    }
    let mut targets: BTreeMap<SymbolId, BTreeSet<NodeId>> = BTreeMap::new();
    loop {
        let returns = callbacks::ReturnOrigins {
            semantic,
            targets: &targets,
            arrows: &arrows,
        };
        let mut additions = Vec::new();
        for &(symbol, expression) in &definitions {
            if let Some(bodies) = callees(expression, &returns) {
                additions.extend(bodies.into_iter().map(|body| (symbol, body)));
            }
        }
        for call in &calls {
            let Some(bodies) = callees(&call.callee, &returns) else {
                continue;
            };
            for body in bodies {
                for (parameter, argument) in arrows[&body].params.items.iter().zip(&call.arguments)
                {
                    if let BindingPattern::BindingIdentifier(binding) = &parameter.pattern
                        && let (Some(symbol), Some(expression)) =
                            (binding.symbol_id.get(), argument.as_expression())
                        && let Some(bodies) = callees(expression, &returns)
                    {
                        additions.extend(bodies.into_iter().map(|body| (symbol, body)));
                    }
                }
            }
        }
        let mut changed = false;
        for (symbol, body) in additions {
            changed |= targets.entry(symbol).or_default().insert(body);
        }
        if !changed {
            break;
        }
    }
    let returns = callbacks::ReturnOrigins {
        semantic,
        targets: &targets,
        arrows: &arrows,
    };
    let mut promises = BTreeSet::new();
    loop {
        let mut additions = Vec::new();
        for call in &calls {
            let Some(bodies) = callees(&call.callee, &returns) else {
                continue;
            };
            for body in bodies {
                for (parameter, argument) in arrows[&body].params.items.iter().zip(&call.arguments)
                {
                    let BindingPattern::BindingIdentifier(binding) = &parameter.pattern else {
                        continue;
                    };
                    let (Some(symbol), Some(expression)) =
                        (binding.symbol_id.get(), argument.as_expression())
                    else {
                        continue;
                    };
                    if awaited::fulfilled_shape(
                        expression,
                        semantic,
                        0,
                        |identifier| {
                            identifier
                                .reference_id
                                .get()
                                .and_then(|reference| {
                                    semantic.scoping().get_reference(reference).symbol_id()
                                })
                                .filter(|symbol| promises.contains(symbol))
                                .map(|_| true)
                        },
                        |call, _| {
                            let mut results = Vec::new();
                            for body in callees(&call.callee, &returns)? {
                                let arrow = arrows[&body];
                                if arrow.r#async {
                                    return None;
                                }
                                let returned = storage::arrow_returns(arrow, semantic)?;
                                if returned.is_empty() {
                                    return None;
                                }
                                results.extend(returned);
                            }
                            Some(results)
                        },
                    ) {
                        additions.push(symbol);
                    }
                }
            }
        }
        let mut changed = false;
        for symbol in additions {
            changed |= promises.insert(symbol);
        }
        if !changed {
            return promises;
        }
    }
}
