//! Read-only callback demands follow original symbols through aliases and known
//! local calls. This establishes checking types, never a runtime call target.
use super::*;
use oxc_ast::AstKind;
use oxc_syntax::node::NodeId;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) type Parameters = BTreeMap<NodeId, BTreeMap<SymbolId, usize>>;

fn symbol(identifier: &IdentifierReference<'_>, semantic: &Semantic<'_>) -> Option<SymbolId> {
    semantic
        .scoping()
        .get_reference(identifier.reference_id.get()?)
        .symbol_id()
}

// Origins of a function-valued expression; scalar effects/tests do not supply
// its value. Actual lowering still checks every branch, target and source limit.
fn origins(
    expression: &Expression<'_>,
    semantic: &Semantic<'_>,
) -> (BTreeSet<SymbolId>, BTreeSet<NodeId>) {
    let mut symbols = BTreeSet::new();
    let mut arrows = BTreeSet::new();
    let mut pending = vec![(expression, 0)];
    while let Some((node, depth)) = pending.pop() {
        if depth > 128 {
            continue;
        }
        if let Some(inner) = erased_inner(node) {
            pending.push((inner, depth + 1));
            continue;
        }
        match node {
            Expression::ParenthesizedExpression(wrapper) => {
                pending.push((&wrapper.expression, depth + 1))
            }
            Expression::ConditionalExpression(conditional) => {
                pending.push((&conditional.consequent, depth + 1));
                pending.push((&conditional.alternate, depth + 1));
            }
            Expression::SequenceExpression(sequence) => {
                if let Some(last) = sequence.expressions.last() {
                    pending.push((last, depth + 1));
                }
            }
            Expression::AssignmentExpression(assignment)
                if assignment.operator == AssignmentOperator::Assign =>
            {
                pending.push((&assignment.right, depth + 1))
            }
            Expression::Identifier(identifier) => {
                if let Some(id) = symbol(identifier, semantic) {
                    symbols.insert(id);
                }
            }
            Expression::ArrowFunctionExpression(arrow) => {
                arrows.insert(arrow.node_id.get());
            }
            _ => (),
        }
    }
    (symbols, arrows)
}

pub(crate) fn infer_callback_parameters(semantic: &Semantic<'_>) -> Result<Parameters, String> {
    let mut aliases: BTreeMap<SymbolId, BTreeSet<SymbolId>> = BTreeMap::new();
    let mut targets: BTreeMap<SymbolId, BTreeSet<NodeId>> = BTreeMap::new();
    let mut demands: BTreeMap<SymbolId, BTreeSet<usize>> = BTreeMap::new();
    let mut arrows = BTreeMap::new();
    let mut calls = Vec::new();
    let mut definition = |id, expression| {
        let (sources, bodies) = origins(expression, semantic);
        aliases.entry(id).or_default().extend(sources);
        targets.entry(id).or_default().extend(bodies);
    };
    for node in semantic.nodes().iter() {
        match node.kind() {
            AstKind::ArrowFunctionExpression(arrow) => {
                arrows.insert(arrow.node_id.get(), arrow);
            }
            AstKind::VariableDeclaration(declaration) => {
                for local in &declaration.declarations {
                    if let BindingPattern::BindingIdentifier(identifier) = &local.id
                        && let Some(id) = identifier.symbol_id.get()
                        && let Some(initializer) = &local.init
                    {
                        definition(id, initializer);
                    }
                }
            }
            AstKind::AssignmentExpression(assignment)
                if assignment.operator == AssignmentOperator::Assign =>
            {
                if let Some(identifier) = assignment
                    .left
                    .as_simple_assignment_target()
                    .and_then(|target| target_identifier(target, 0).ok().flatten())
                    && let Some(id) = symbol(identifier, semantic)
                {
                    definition(id, &assignment.right);
                }
            }
            AstKind::CallExpression(call) => {
                if let Some(Expression::Identifier(identifier)) =
                    closures::unwrapped(&call.callee, 0)
                    && let Some(id) = symbol(identifier, semantic)
                {
                    demands.entry(id).or_default().insert(call.arguments.len());
                }
                calls.push(call);
            }
            _ => (),
        }
    }
    // Resolve only source-known arrow targets, including aliases, so forwarding
    // can transfer a formal callback demand to the caller's argument symbol.
    loop {
        let mut additions = Vec::new();
        for (alias, sources) in &aliases {
            for source in sources {
                if let Some(bodies) = targets.get(source) {
                    for body in bodies {
                        additions.push((*alias, *body));
                    }
                }
            }
        }
        let mut changed = false;
        for (alias, body) in additions {
            changed |= targets.entry(alias).or_default().insert(body);
        }
        if !changed {
            break;
        }
    }
    for call in calls {
        let (sources, mut bodies) = origins(&call.callee, semantic);
        for source in sources {
            if let Some(known) = targets.get(&source) {
                bodies.extend(known);
            }
        }
        for body in bodies {
            let arrow = arrows[&body];
            for (parameter, argument) in arrow.params.items.iter().zip(&call.arguments) {
                if let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern
                    && let Some(id) = identifier.symbol_id.get()
                    && let Some(expression) = argument.as_expression()
                {
                    aliases
                        .entry(id)
                        .or_default()
                        .extend(origins(expression, semantic).0);
                }
            }
        }
    }
    loop {
        let mut additions = Vec::new();
        for (alias, sources) in &aliases {
            if let Some(arities) = demands.get(alias) {
                for source in sources {
                    for arity in arities {
                        additions.push((*source, *arity));
                    }
                }
            }
        }
        let mut changed = false;
        for (source, arity) in additions {
            changed |= demands.entry(source).or_default().insert(arity);
        }
        if !changed {
            break;
        }
    }
    let mut result = Parameters::new();
    for (id, arrow) in arrows {
        let mut parameters = BTreeMap::new();
        for parameter in &arrow.params.items {
            if let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern
                && let Some(symbol) = identifier.symbol_id.get()
                && let Some(arities) = demands.get(&symbol)
            {
                if arities.len() != 1 || *arities.first().unwrap() > 1000 {
                    return Err(unsupported(
                        parameter,
                        "callback parameters require one fixed arity of at most 1000 Number arguments",
                    ));
                }
                parameters.insert(symbol, *arities.first().unwrap());
            }
        }
        result.insert(id, parameters);
    }
    Ok(result)
}
