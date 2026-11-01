//! Finite original-source constructor and formal-input relations. These facts
//! select representation lanes, never evaluate effects or select runtime callees.
use super::*;
use oxc_ast::ast::ArrowFunctionExpression;

// Known module targets must agree. These positive facts do not prove a complete
// callable signature: lowering still validates every source alternative.
pub(super) fn module_result(
    callee: &Expression<'_>,
    origins: &callbacks::ReturnOrigins<'_, '_, '_>,
    named: &BTreeMap<SymbolId, BTreeSet<SymbolId>>,
    functions: &[NumericFunction],
) -> Option<Type> {
    let (sources, bodies) = origins.expression(callee, &BTreeMap::new(), 0, 0);
    if sources.is_empty() || !bodies.is_empty() {
        return None;
    }
    let mut result = None;
    for source in sources {
        for target in named.get(&source)? {
            let ty = functions.iter().find(|f| f.symbol == *target)?.return_type;
            if result.is_some_and(|old| old != ty) {
                return None;
            }
            result = Some(ty);
        }
    }
    result.filter(|ty| ty.is_managed())
}

pub(super) struct Inputs<'n, 'a> {
    pub arrows: BTreeMap<NodeId, &'n ArrowFunctionExpression<'a>>,
    pub targets: BTreeMap<SymbolId, BTreeSet<NodeId>>,
    pub actuals: Vec<(SymbolId, &'n Expression<'a>)>,
}

pub(super) fn collect<'n, 'a>(
    semantic: &'n Semantic<'a>,
    definitions: &[(SymbolId, Option<&'n Expression<'a>>)],
    calls: &[&'n CallExpression<'a>],
) -> Inputs<'n, 'a> {
    let arrows: BTreeMap<_, _> = semantic
        .nodes()
        .iter()
        .filter_map(|node| match node.kind() {
            AstKind::ArrowFunctionExpression(arrow) => Some((arrow.node_id.get(), arrow)),
            _ => None,
        })
        .collect();
    let mut targets: BTreeMap<SymbolId, BTreeSet<NodeId>> = BTreeMap::new();
    let mut actuals: Vec<(SymbolId, &Expression<'a>)> = Vec::new();
    let mut seen = BTreeSet::new();
    while !arrows.is_empty() {
        let origins = callbacks::ReturnOrigins {
            semantic,
            targets: &targets,
            arrows: &arrows,
        };
        let bodies = |expression| {
            let (sources, mut bodies) = origins.expression(expression, &BTreeMap::new(), 0, 0);
            for source in sources {
                if let Some(known) = targets.get(&source) {
                    bodies.extend(known);
                }
            }
            bodies
        };
        let mut additions = Vec::new();
        for (symbol, expression) in definitions
            .iter()
            .filter_map(|(s, e)| e.map(|e| (*s, e)))
            .chain(actuals.iter().copied())
        {
            for body in bodies(expression) {
                additions.push((symbol, body));
            }
        }
        let mut inputs = Vec::new();
        for call in calls {
            for body in bodies(invocation::callee(call)) {
                for (parameter, argument) in arrows[&body]
                    .params
                    .items
                    .iter()
                    .zip(invocation::arguments(call))
                {
                    if let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern
                        && let Some(symbol) = identifier.symbol_id.get()
                        && let Some(actual) = argument.as_expression()
                    {
                        let span = actual.span();
                        if seen.insert((symbol, span.start, span.end)) {
                            inputs.push((symbol, actual));
                        }
                    }
                }
            }
        }
        let mut changed = !inputs.is_empty();
        actuals.extend(inputs);
        for (symbol, body) in additions {
            changed |= targets.entry(symbol).or_default().insert(body);
        }
        if !changed {
            break;
        }
    }
    Inputs {
        arrows,
        targets,
        actuals,
    }
}
