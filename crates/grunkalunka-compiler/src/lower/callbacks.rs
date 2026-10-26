//! Read-only callback demands follow original symbols through aliases and known
//! local calls. This establishes checking types, never a runtime call target.
use super::*;
use oxc_ast::AstKind;
use oxc_syntax::node::NodeId;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) type Parameters = BTreeMap<NodeId, BTreeMap<SymbolId, usize>>;
type Origins = (BTreeSet<SymbolId>, BTreeSet<NodeId>);

struct ReturnOrigins<'s, 'n, 'a> {
    semantic: &'s Semantic<'a>,
    targets: &'s BTreeMap<SymbolId, BTreeSet<NodeId>>,
    arrows: &'s BTreeMap<NodeId, &'n oxc_ast::ast::ArrowFunctionExpression<'a>>,
}

impl ReturnOrigins<'_, '_, '_> {
    // Follow source-known concise and block results. Actual argument origins are
    // computed before shadowing formals; no call effects or argument bindings
    // escape this read-only query. The same eight-call bound limits recursion.
    fn expression(
        &self,
        expression: &Expression<'_>,
        bindings: &BTreeMap<SymbolId, Origins>,
        depth: usize,
        calls: usize,
    ) -> Origins {
        if depth > 128 {
            return Origins::default();
        }
        if let Some(inner) = erased_inner(expression) {
            return self.expression(inner, bindings, depth + 1, calls);
        }
        match expression {
            Expression::ParenthesizedExpression(wrapper) => {
                self.expression(&wrapper.expression, bindings, depth + 1, calls)
            }
            Expression::Identifier(identifier) => symbol(identifier, self.semantic)
                .and_then(|id| bindings.get(&id))
                .cloned()
                .unwrap_or_else(|| origins(expression, self.semantic)),
            Expression::SequenceExpression(sequence) => sequence
                .expressions
                .last()
                .map(|last| self.expression(last, bindings, depth + 1, calls))
                .unwrap_or_default(),
            Expression::AssignmentExpression(assignment)
                if assignment.operator == AssignmentOperator::Assign =>
            {
                self.expression(&assignment.right, bindings, depth + 1, calls)
            }
            Expression::ConditionalExpression(conditional) => {
                let mut result =
                    self.expression(&conditional.consequent, bindings, depth + 1, calls);
                let other = self.expression(&conditional.alternate, bindings, depth + 1, calls);
                result.0.extend(other.0);
                result.1.extend(other.1);
                result
            }
            Expression::CallExpression(call) => {
                if calls >= 8 {
                    return Origins::default();
                }
                let (sources, mut bodies) =
                    self.expression(&call.callee, bindings, depth + 1, calls + 1);
                for source in sources {
                    if let Some(known) = self.targets.get(&source) {
                        bodies.extend(known);
                    }
                }
                let mut result = Origins::default();
                for body in bodies {
                    let arrow = self.arrows[&body];
                    if arrow.params.items.len() != call.arguments.len() {
                        continue;
                    }
                    let expressions = if let Some(expression) = arrow.get_expression() {
                        vec![expression]
                    } else {
                        block_results::return_statements(self.semantic, body)
                            .into_iter()
                            .filter_map(|statement| statement.argument.as_ref())
                            .collect()
                    };
                    let mut parameters = bindings.clone();
                    for (parameter, argument) in arrow.params.items.iter().zip(&call.arguments) {
                        if let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern
                            && let Some(id) = identifier.symbol_id.get()
                            && let Some(argument) = argument.as_expression()
                        {
                            parameters.insert(
                                id,
                                self.expression(argument, bindings, depth + 1, calls + 1),
                            );
                        }
                    }
                    for expression in expressions {
                        let returned =
                            self.expression(expression, &parameters, depth + 1, calls + 1);
                        result.0.extend(returned.0);
                        result.1.extend(returned.1);
                    }
                }
                result
            }
            _ => origins(expression, self.semantic),
        }
    }
}

fn symbol(identifier: &IdentifierReference<'_>, semantic: &Semantic<'_>) -> Option<SymbolId> {
    semantic
        .scoping()
        .get_reference(identifier.reference_id.get()?)
        .symbol_id()
}

// Origins of a function-valued expression; scalar effects/tests do not supply
// its value. Actual lowering still checks every branch, target and source limit.
fn origins(expression: &Expression<'_>, semantic: &Semantic<'_>) -> Origins {
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
    let mut definitions = Vec::new();
    let mut definition = |id, expression| {
        definitions.push((id, expression));
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
        let returns = ReturnOrigins {
            semantic,
            targets: &targets,
            arrows: &arrows,
        };
        let mut origins = Vec::new();
        for &(id, expression) in &definitions {
            origins.push((id, returns.expression(expression, &BTreeMap::new(), 0, 0)));
        }
        let mut changed = false;
        for (alias, body) in additions {
            changed |= targets.entry(alias).or_default().insert(body);
        }
        for (id, (sources, bodies)) in origins {
            for source in sources {
                changed |= aliases.entry(id).or_default().insert(source);
            }
            for body in bodies {
                changed |= targets.entry(id).or_default().insert(body);
            }
        }
        if !changed {
            break;
        }
    }
    // A known constructor on one return path fixes the kind of callbacks
    // returned on the other paths. This also gives disposable body checking a
    // function input without making constructor-free identity arrows monomorphic.
    // These callback demands are Number-leaf arities, not higher-order signature IDs;
    // block result joins independently compare complete signature IDs.
    let returns = ReturnOrigins {
        semantic,
        targets: &targets,
        arrows: &arrows,
    };
    for (id, arrow) in &arrows {
        if arrow.get_expression().is_some() {
            continue;
        }
        let mut returned = Origins::default();
        for statement in block_results::return_statements(semantic, *id) {
            if let Some(expression) = &statement.argument {
                let origins = returns.expression(expression, &BTreeMap::new(), 0, 0);
                returned.0.extend(origins.0);
                returned.1.extend(origins.1);
            }
        }
        for symbol in &returned.0 {
            if let Some(bodies) = targets.get(symbol) {
                returned.1.extend(bodies);
            }
        }
        let arities = returned
            .1
            .iter()
            .map(|body| arrows[body].params.items.len())
            .collect::<BTreeSet<_>>();
        if arities.len() == 1 {
            let arity = *arities.first().unwrap();
            for symbol in returned.0 {
                demands.entry(symbol).or_default().insert(arity);
            }
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
    for (id, arrow) in &arrows {
        let mut parameters = BTreeMap::new();
        for parameter in &arrow.params.items {
            if let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern
                && let Some(symbol) = identifier.symbol_id.get()
            {
                let mut arities = demands.get(&symbol).cloned().unwrap_or_default();
                // A function constructor written to a formal also fixes its
                // represented kind, even when the body returns it without calling.
                // Known call arguments do not populate `targets` for formals:
                // scalar identity functions remain polymorphic at specialization.
                if let Some(bodies) = targets.get(&symbol) {
                    arities.extend(bodies.iter().map(|body| arrows[body].params.items.len()));
                }
                if arities.is_empty() {
                    continue;
                }
                if arities.len() != 1 || *arities.first().unwrap() > 1000 {
                    return Err(unsupported(
                        parameter,
                        "callback parameters require one fixed arity of at most 1000 Number arguments",
                    ));
                }
                parameters.insert(symbol, *arities.first().unwrap());
            }
        }
        result.insert(*id, parameters);
    }
    Ok(result)
}

// Callback demands describe leaf signatures. A higher-order arrow's own shape
// follows parameter order, while source syntax and TypeScript names prove no kind.
pub(super) fn parameter_types(
    arrow: &oxc_ast::ast::ArrowFunctionExpression<'_>,
    parameters: &Parameters,
) -> Vec<Type> {
    arrow
        .params
        .items
        .iter()
        .map(|parameter| {
            let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern else {
                return Type::Number;
            };
            identifier
                .symbol_id
                .get()
                .and_then(|symbol| parameters.get(&arrow.node_id.get())?.get(&symbol))
                .map_or(Type::Number, |arity| Type::Closure(*arity))
        })
        .collect()
}

pub(crate) fn closure_signatures(
    semantic: &Semantic<'_>,
    parameters: &Parameters,
) -> ir::ClosureSignatures {
    let mut signatures = ir::ClosureSignatures::default();
    for node in semantic.nodes().iter() {
        if let AstKind::ArrowFunctionExpression(arrow) = node.kind() {
            signatures.intern(parameter_types(arrow, parameters));
        }
    }
    signatures
}
