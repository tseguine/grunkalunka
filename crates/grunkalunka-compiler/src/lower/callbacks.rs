//! Read-only callback demands follow original symbols through aliases and known
//! local calls. This establishes checking types, never a runtime call target.
use super::*;
use oxc_ast::AstKind;
use oxc_syntax::node::NodeId;
use std::collections::{BTreeMap, BTreeSet};

// Entries are full scalar-result signature IDs; numeric leaves use their arity.
pub(crate) type Parameters = BTreeMap<NodeId, BTreeMap<SymbolId, usize>>;
type Origins = (BTreeSet<SymbolId>, BTreeSet<NodeId>);

pub(super) struct ReturnOrigins<'s, 'n, 'a> {
    pub semantic: &'s Semantic<'a>,
    pub stored: &'s stored_functions::Fields,
    pub targets: &'s BTreeMap<SymbolId, BTreeSet<NodeId>>,
    pub arrows: &'s BTreeMap<NodeId, &'n oxc_ast::ast::ArrowFunctionExpression<'a>>,
}

impl<'s, 'n, 'a> ReturnOrigins<'s, 'n, 'a> {
    // Follow source-known concise and block results. Actual argument origins are
    // computed before shadowing formals; no call effects or argument bindings
    // escape this read-only query. The eight-call admission bound is unchanged.
    pub(super) fn expression<'e>(
        &'e self,
        expression: &'e Expression<'a>,
        bindings: &BTreeMap<SymbolId, Origins>,
        depth: usize,
        calls: usize,
    ) -> Origins {
        // A source call can lead through a callee, actual arguments and one or
        // more source-known returned arrows. Keep all of those continuations on
        // the heap: limiting call *depth* to eight never bounded native frames.
        enum Work<'e, 'a> {
            Expression {
                node: &'e Expression<'a>,
                bindings: std::rc::Rc<BTreeMap<SymbolId, Origins>>,
                depth: usize,
                calls: usize,
            },
            Union(usize),
            Callee {
                call: &'e CallExpression<'a>,
                bindings: std::rc::Rc<BTreeMap<SymbolId, Origins>>,
                depth: usize,
                calls: usize,
            },
            Body {
                body: NodeId,
                call: &'e CallExpression<'a>,
                bindings: std::rc::Rc<BTreeMap<SymbolId, Origins>>,
                depth: usize,
                calls: usize,
            },
            BindArguments {
                symbols: Vec<SymbolId>,
                returns: Vec<&'e Expression<'a>>,
                bindings: std::rc::Rc<BTreeMap<SymbolId, Origins>>,
                depth: usize,
                calls: usize,
            },
        }

        let mut work = vec![Work::Expression {
            node: expression,
            bindings: std::rc::Rc::new(bindings.clone()),
            depth,
            calls,
        }];
        let mut results = Vec::new();
        while let Some(task) = work.pop() {
            match task {
                Work::Union(count) => {
                    let mut joined = Origins::default();
                    for _ in 0..count {
                        let found: Origins = results.pop().expect("scheduled return origins");
                        joined.0.extend(found.0);
                        joined.1.extend(found.1);
                    }
                    results.push(joined);
                }
                Work::Expression {
                    node,
                    bindings,
                    depth,
                    calls,
                } => {
                    if depth > 128 {
                        results.push(Origins::default());
                        continue;
                    }
                    if let Some(inner) = erased_inner(node) {
                        work.push(Work::Expression {
                            node: inner,
                            bindings,
                            depth: depth + 1,
                            calls,
                        });
                        continue;
                    }
                    match node {
                        Expression::StaticMemberExpression(_)
                        | Expression::ComputedMemberExpression(_) => {
                            results.push(self.stored.origins(node.node_id()));
                        }
                        Expression::ParenthesizedExpression(wrapper) => {
                            work.push(Work::Expression {
                                node: &wrapper.expression,
                                bindings,
                                depth: depth + 1,
                                calls,
                            });
                        }
                        Expression::Identifier(identifier) => {
                            results.push(
                                symbol(identifier, self.semantic)
                                    .and_then(|id| bindings.get(&id))
                                    .cloned()
                                    .unwrap_or_else(|| origins(node, self.semantic)),
                            );
                        }
                        Expression::SequenceExpression(sequence) => {
                            if let Some(last) = sequence.expressions.last() {
                                work.push(Work::Expression {
                                    node: last,
                                    bindings,
                                    depth: depth + 1,
                                    calls,
                                });
                            } else {
                                results.push(Origins::default());
                            }
                        }
                        Expression::AssignmentExpression(assignment)
                            if matches!(
                                assignment.operator,
                                AssignmentOperator::LogicalAnd
                                    | AssignmentOperator::LogicalOr
                                    | AssignmentOperator::LogicalNullish
                            ) =>
                        {
                            results.push(self.stored.origins(node.node_id()));
                        }
                        Expression::AssignmentExpression(assignment)
                            if assignment.operator == AssignmentOperator::Assign =>
                        {
                            work.push(Work::Expression {
                                node: &assignment.right,
                                bindings,
                                depth: depth + 1,
                                calls,
                            });
                        }
                        Expression::ConditionalExpression(conditional) => {
                            work.push(Work::Union(2));
                            work.push(Work::Expression {
                                node: &conditional.alternate,
                                bindings: bindings.clone(),
                                depth: depth + 1,
                                calls,
                            });
                            work.push(Work::Expression {
                                node: &conditional.consequent,
                                bindings,
                                depth: depth + 1,
                                calls,
                            });
                        }
                        Expression::LogicalExpression(logical) => {
                            work.push(Work::Union(2));
                            work.push(Work::Expression {
                                node: &logical.right,
                                bindings: bindings.clone(),
                                depth: depth + 1,
                                calls,
                            });
                            work.push(Work::Expression {
                                node: &logical.left,
                                bindings,
                                depth: depth + 1,
                                calls,
                            });
                        }
                        Expression::CallExpression(call) if calls < 8 => {
                            work.push(Work::Callee {
                                call,
                                bindings: bindings.clone(),
                                depth,
                                calls,
                            });
                            work.push(Work::Expression {
                                node: invocation::callee(call),
                                bindings,
                                depth: depth + 1,
                                calls: calls + 1,
                            });
                        }
                        Expression::CallExpression(_) => {
                            results.push(Origins::default());
                        }
                        _ => results.push(origins(node, self.semantic)),
                    }
                }
                Work::Callee {
                    call,
                    bindings,
                    depth,
                    calls,
                } => {
                    let (sources, mut bodies): Origins =
                        results.pop().expect("scheduled callee origins");
                    for source in sources {
                        if let Some(known) = self.targets.get(&source) {
                            bodies.extend(known);
                        }
                    }
                    work.push(Work::Union(bodies.len()));
                    for body in bodies.into_iter().rev() {
                        work.push(Work::Body {
                            body,
                            call,
                            bindings: bindings.clone(),
                            depth,
                            calls,
                        });
                    }
                }
                Work::Body {
                    body,
                    call,
                    bindings,
                    depth,
                    calls,
                } => {
                    let arrow = self.arrows[&body];
                    if arrow.params.items.len() > invocation::arguments(call).len() {
                        results.push(Origins::default());
                        continue;
                    }
                    let returns = if let Some(expression) = arrow.get_expression() {
                        vec![expression]
                    } else {
                        block_results::return_statements(self.semantic, body)
                            .into_iter()
                            .filter_map(|statement| statement.argument.as_ref())
                            .collect()
                    };
                    let mut arguments = Vec::new();
                    for (parameter, argument) in
                        arrow.params.items.iter().zip(invocation::arguments(call))
                    {
                        if let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern
                            && let Some(symbol) = identifier.symbol_id.get()
                            && let Some(argument) = argument.as_expression()
                        {
                            arguments.push((symbol, argument));
                        }
                    }
                    work.push(Work::BindArguments {
                        symbols: arguments.iter().map(|(symbol, _)| *symbol).collect(),
                        returns,
                        bindings: bindings.clone(),
                        depth,
                        calls,
                    });
                    for (_, argument) in arguments.into_iter().rev() {
                        work.push(Work::Expression {
                            node: argument,
                            bindings: bindings.clone(),
                            depth: depth + 1,
                            calls: calls + 1,
                        });
                    }
                }
                Work::BindArguments {
                    symbols,
                    returns,
                    bindings,
                    depth,
                    calls,
                } => {
                    let mut parameters = (*bindings).clone();
                    // Actuals were evaluated under the caller bindings, before
                    // any formal is shadowed, exactly as in the source query.
                    for symbol in symbols.into_iter().rev() {
                        parameters
                            .insert(symbol, results.pop().expect("scheduled argument origins"));
                    }
                    let parameters = std::rc::Rc::new(parameters);
                    work.push(Work::Union(returns.len()));
                    for expression in returns.into_iter().rev() {
                        work.push(Work::Expression {
                            node: expression,
                            bindings: parameters.clone(),
                            depth: depth + 1,
                            calls: calls + 1,
                        });
                    }
                }
            }
        }
        results.pop().expect("root return origins")
    }
}

fn symbol(identifier: &IdentifierReference<'_>, semantic: &Semantic<'_>) -> Option<SymbolId> {
    semantic
        .scoping()
        .get_reference(identifier.reference_id.get()?)
        .symbol_id()
}

pub(super) fn named_async_targets(
    returns: &ReturnOrigins<'_, '_, '_>,
    functions: &[NumericFunction],
    definitions: &[(SymbolId, &Expression<'_>)],
    calls: &[&CallExpression<'_>],
) -> BTreeMap<SymbolId, BTreeSet<SymbolId>> {
    named_targets(
        returns,
        functions
            .iter()
            .filter(|function| function.async_signature.is_some())
            .map(|function| function.symbol),
        definitions,
        calls,
    )
}

pub(super) fn named_targets(
    returns: &ReturnOrigins<'_, '_, '_>,
    seeds: impl Iterator<Item = SymbolId>,
    definitions: &[(SymbolId, &Expression<'_>)],
    calls: &[&CallExpression<'_>],
) -> BTreeMap<SymbolId, BTreeSet<SymbolId>> {
    let mut named: BTreeMap<_, _> = seeds
        .map(|symbol| (symbol, BTreeSet::from([symbol])))
        .collect();
    while !named.is_empty() {
        let origins = |expression| {
            returns
                .expression(expression, &BTreeMap::new(), 0, 0)
                .0
                .iter()
                .filter_map(|source| named.get(source))
                .flatten()
                .copied()
                .collect::<BTreeSet<_>>()
        };
        let mut additions = definitions
            .iter()
            .map(|(symbol, expression)| (*symbol, origins(expression)))
            .collect::<Vec<_>>();
        for call in calls {
            let (sources, mut bodies) =
                returns.expression(invocation::callee(call), &BTreeMap::new(), 0, 0);
            for source in sources {
                if let Some(known) = returns.targets.get(&source) {
                    bodies.extend(known);
                }
            }
            for body in bodies {
                for (parameter, argument) in returns.arrows[&body]
                    .params
                    .items
                    .iter()
                    .zip(invocation::arguments(call))
                {
                    if let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern
                        && let Some(symbol) = identifier.symbol_id.get()
                        && let Some(actual) = argument.as_expression()
                    {
                        additions.push((symbol, origins(actual)));
                    }
                }
            }
        }
        let mut changed = false;
        for (symbol, targets) in additions {
            if targets.is_empty() {
                continue;
            }
            let known = named.entry(symbol).or_default();
            let previous = known.len();
            known.extend(targets);
            changed |= known.len() != previous;
        }
        if !changed {
            break;
        }
    }
    named
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
            Expression::LogicalExpression(logical) => {
                pending.push((&logical.left, depth + 1));
                pending.push((&logical.right, depth + 1));
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

pub(crate) fn infer_callback_parameters(
    semantic: &Semantic<'_>,
    functions: &[NumericFunction],
    promise_parameters: &super::promises::Inputs,
    signatures: &std::cell::RefCell<ir::ClosureSignatures>,
    stored: &stored_functions::Fields,
) -> Result<Parameters, String> {
    let mut aliases: BTreeMap<SymbolId, BTreeSet<SymbolId>> = BTreeMap::new();
    let mut targets: BTreeMap<SymbolId, BTreeSet<NodeId>> = BTreeMap::new();
    let mut demands: BTreeMap<SymbolId, BTreeSet<usize>> = BTreeMap::new();
    let mut arrows = BTreeMap::new();
    let mut calls = Vec::new();
    let mut length_receivers = BTreeSet::new();
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
                    closures::unwrapped(invocation::callee(call), invocation::depth(call, 0))
                    && let Some(id) = symbol(identifier, semantic)
                {
                    demands
                        .entry(id)
                        .or_default()
                        .insert(invocation::arguments(call).len());
                }
                calls.push(call);
            }
            AstKind::StaticMemberExpression(member)
                if !member.optional && member.property.name == "length" =>
            {
                length_receivers.extend(origins(&member.object, semantic).0);
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
            stored,
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
    // Initial seeds use Number-leaf arities; refinement fills higher parameter
    // kinds. Block result joins independently compare complete signature IDs.
    let returns = ReturnOrigins {
        semantic,
        stored,
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
    let declared_targets = targets.clone();
    // Calls through an aliased callback can reveal another known target's
    // parameter origins. Saturate this finite source relation before shapes.
    loop {
        let mut additions = Vec::new();
        for call in &calls {
            let (sources, mut bodies) = origins(invocation::callee(call), semantic);
            let mut pending = sources.into_iter().collect::<Vec<_>>();
            let mut seen = BTreeSet::new();
            while let Some(source) = pending.pop() {
                if !seen.insert(source) {
                    continue;
                }
                if let Some(known) = targets.get(&source) {
                    bodies.extend(known);
                }
                if let Some(sources) = aliases.get(&source) {
                    pending.extend(sources);
                }
            }
            for body in bodies {
                for (parameter, argument) in arrows[&body]
                    .params
                    .items
                    .iter()
                    .zip(invocation::arguments(call))
                {
                    if let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern
                        && let Some(id) = identifier.symbol_id.get()
                        && let Some(expression) = argument.as_expression()
                    {
                        additions.push((id, origins(expression, semantic)));
                    }
                }
            }
        }
        let mut changed = false;
        for (id, (sources, bodies)) in additions {
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
    // A length-only callback use requires a function, but does not invent its
    // signature. Seed that demand only from actual source-known constructors.
    for receiver in length_receivers {
        let mut pending = vec![receiver];
        let mut seen = BTreeSet::new();
        while let Some(source) = pending.pop() {
            if !seen.insert(source) {
                continue;
            }
            if let Some(bodies) = targets.get(&source) {
                demands
                    .entry(receiver)
                    .or_default()
                    .extend(bodies.iter().map(|body| arrows[body].params.items.len()));
            }
            if let Some(function) = functions.iter().find(|function| function.symbol == source) {
                demands
                    .entry(receiver)
                    .or_default()
                    .insert(function.parameter_count);
            }
            if let Some(sources) = aliases.get(&source) {
                pending.extend(sources);
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
        // Runtime alternatives need one input shape even when a peer ignores
        // a callback. Transfer existing demands by formal position; actual
        // argument constructors alone must not monomorphize identity arrows.
        let returns = ReturnOrigins {
            semantic,
            stored,
            targets: &targets,
            arrows: &arrows,
        };
        let mut peers = targets.values().cloned().collect::<Vec<_>>();
        for call in &calls {
            let (sources, mut bodies) =
                returns.expression(invocation::callee(call), &BTreeMap::new(), 0, 0);
            for source in sources {
                if let Some(known) = targets.get(&source) {
                    bodies.extend(known);
                }
            }
            peers.push(bodies);
        }
        for bodies in peers {
            let mut positions: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
            for body in &bodies {
                for (index, parameter) in arrows[body].params.items.iter().enumerate() {
                    if let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern
                        && let Some(symbol) = identifier.symbol_id.get()
                        && let Some(arities) = demands.get(&symbol)
                    {
                        positions.entry(index).or_default().extend(arities);
                    }
                }
            }
            for body in bodies {
                for (index, parameter) in arrows[&body].params.items.iter().enumerate() {
                    if let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern
                        && let Some(symbol) = identifier.symbol_id.get()
                        && let Some(arities) = positions.get(&index)
                    {
                        additions.extend(arities.iter().map(|arity| (symbol, *arity)));
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
                // Argument-derived targets are separate from declared writes:
                // scalar identity functions remain polymorphic at specialization.
                if let Some(bodies) = declared_targets.get(&symbol) {
                    arities.extend(bodies.iter().map(|body| arrows[body].params.items.len()));
                }
                if arities.is_empty() {
                    continue;
                }
                // Call counts are upper bounds on required inputs. Start at
                // the smallest demand; concrete targets refine the full type.
                if *arities.first().unwrap() > 1000 {
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
    refine_shapes(
        ReturnOrigins {
            semantic,
            stored,
            targets: &targets,
            arrows: &arrows,
        },
        functions,
        promise_parameters,
        &aliases,
        &calls,
        result,
        signatures,
    )
}

fn refine_shapes<'a>(
    returns: ReturnOrigins<'_, '_, 'a>,
    functions: &[NumericFunction],
    promise_parameters: &super::promises::Inputs,
    aliases: &BTreeMap<SymbolId, BTreeSet<SymbolId>>,
    calls: &[&oxc_ast::ast::CallExpression<'a>],
    mut parameters: Parameters,
    signatures: &std::cell::RefCell<ir::ClosureSignatures>,
) -> Result<Parameters, String> {
    let semantic = returns.semantic;
    let targets = returns.targets;
    let arrows = returns.arrows;
    let owners: BTreeMap<_, _> = arrows
        .iter()
        .flat_map(|(body, arrow)| {
            arrow.params.items.iter().filter_map(move |parameter| {
                let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern else {
                    return None;
                };
                identifier.symbol_id.get().map(|symbol| (symbol, *body))
            })
        })
        .collect();
    let constructor_kind = |body: NodeId,
                            parameters: &Parameters,
                            kinds: &storage::BindingKinds| {
        let arrow = arrows[&body];
        if arrow.r#async
            && (kinds.async_results.contains(&body)
                || promise_parameters.asynchronous.contains(&body))
        {
            return signatures.borrow_mut().intern_async(parameter_types(
                arrow,
                parameters,
                ValueInputs {
                    general: &kinds.inputs,
                    managed: &kinds.managed.inputs,
                },
                promise_parameters,
            ));
        }
        let synchronous_promise = !arrow.r#async
            && storage::arrow_returns(arrow, semantic).is_some_and(|results| {
                !results.is_empty()
                    && results.into_iter().all(|expression| {
                        awaited::fulfilled_shape(
                            expression,
                            semantic,
                            0,
                            |identifier| {
                                symbol(identifier, semantic)
                                    .filter(|symbol| promise_parameters.is_fulfilled(symbol))
                                    .map(|_| true)
                            },
                            |call, _| {
                                let (sources, mut bodies) = returns.expression(
                                    invocation::callee(call),
                                    &BTreeMap::new(),
                                    0,
                                    0,
                                );
                                let mut proven = !bodies.is_empty();
                                let mut results = Vec::new();
                                for source in sources {
                                    if let Some(signature) =
                                        parameters.values().find_map(|inputs| inputs.get(&source))
                                    {
                                        if signatures.borrow().result(*signature)
                                            != Some(Type::FulfilledPromise)
                                        {
                                            return None;
                                        }
                                        proven = true;
                                    } else {
                                        let known = targets.get(&source)?;
                                        bodies.extend(known);
                                        proven |= !known.is_empty();
                                    }
                                }
                                if !proven {
                                    return None;
                                }
                                for body in bodies {
                                    let arrow = arrows[&body];
                                    if arrow.r#async {
                                        if awaited::eager_async_arrow(arrow, semantic) {
                                            continue;
                                        }
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
                        )
                    })
            });
        let synchronous_settled = !arrow.r#async
            && storage::arrow_returns(arrow, semantic).is_some_and(|results| {
                !results.is_empty()
                    && results.into_iter().all(|expression| {
                        awaited::settled_shape(
                            expression,
                            0,
                            |call, _| {
                                if awaited::intrinsic(call, semantic)
                                    .ok()
                                    .flatten()
                                    .is_some_and(|intrinsic| !intrinsic.rejected)
                                {
                                    return Some(Type::FulfilledPromise);
                                }
                                let (sources, mut bodies) = returns.expression(
                                    invocation::callee(call),
                                    &BTreeMap::new(),
                                    0,
                                    0,
                                );
                                let mut result_kinds = Vec::new();
                                for source in sources {
                                    if let Some(signature) =
                                        parameters.values().find_map(|inputs| inputs.get(&source))
                                    {
                                        result_kinds.push(signatures.borrow().result(*signature)?);
                                    } else {
                                        bodies.extend(targets.get(&source)?);
                                    }
                                }
                                for body in bodies {
                                    let arrow = arrows[&body];
                                    if !awaited::immediate_async_arrow(arrow, semantic) {
                                        return None;
                                    }
                                    result_kinds.push(
                                        if kinds.settled_results.contains(&body)
                                            || awaited::owns_throw(arrow, semantic)
                                        {
                                            Type::SettledPromise
                                        } else {
                                            Type::FulfilledPromise
                                        },
                                    );
                                }
                                if result_kinds.is_empty()
                                    || result_kinds.iter().any(|kind| {
                                        !matches!(
                                            kind,
                                            Type::FulfilledPromise | Type::SettledPromise
                                        )
                                    })
                                {
                                    return None;
                                }
                                Some(if result_kinds.contains(&Type::SettledPromise) {
                                    Type::SettledPromise
                                } else {
                                    Type::FulfilledPromise
                                })
                            },
                            |_, _| false,
                        )
                    })
            });
        let promise = synchronous_promise || awaited::immediate_async_arrow(arrow, semantic);
        let number = !kinds.general_results.contains(&body)
            && storage::arrow_number_result_shape(
                arrow,
                semantic,
                functions,
                storage::NumberCalls::Proven(&kinds.number_calls),
                |symbol| !kinds.storage.contains(&symbol),
                |symbol| {
                    !kinds.storage.contains(&symbol)
                        && !parameters
                            .values()
                            .any(|inputs| inputs.contains_key(&symbol))
                },
            );
        signatures.borrow_mut().intern(
            parameter_types(
                arrow,
                parameters,
                ValueInputs {
                    general: &kinds.inputs,
                    managed: &kinds.managed.inputs,
                },
                promise_parameters,
            ),
            if !arrow.r#async && promise_parameters.mixed_results.contains(&body) {
                Type::AsyncValue
            } else if (promise_parameters.identities || !arrow.r#async)
                && promise_parameters.results.contains(&body)
            {
                Type::Promise
            } else if synchronous_settled {
                Type::SettledPromise
            } else if promise {
                if kinds.settled_results.contains(&body)
                    || (arrow.r#async && awaited::owns_throw(arrow, semantic))
                {
                    Type::SettledPromise
                } else {
                    Type::FulfilledPromise
                }
            } else if !arrow.r#async
                && let Some(ty) = kinds.managed.results.get(&body)
            {
                *ty
            } else if number {
                Type::Number
            } else {
                Type::General
            },
        )
    };
    for _ in 0..8 {
        let function_kind = |expression: &Expression<'a>,
                             parameters: &Parameters,
                             constructors_only: bool,
                             general: &storage::BindingKinds| {
            let (sources, bodies) = returns.expression(expression, &BTreeMap::new(), 0, 0);
            let mut pending = sources.into_iter().collect::<Vec<_>>();
            let mut seen = BTreeSet::new();
            let mut kinds = BTreeSet::new();
            let constructor = |body| constructor_kind(body, parameters, general);
            kinds.extend(bodies.into_iter().map(constructor));
            while let Some(source) = pending.pop() {
                if !seen.insert(source) {
                    continue;
                }
                if let Some(signature) = functions
                    .iter()
                    .find(|function| function.symbol == source)
                    .map(|function| function.closure_signature(signatures))
                {
                    kinds.insert(signature);
                }
                if !constructors_only
                    && let Some(kind) = parameters
                        .values()
                        .find_map(|parameters| parameters.get(&source))
                {
                    kinds.insert(*kind);
                }
                if let Some(bodies) = targets.get(&source) {
                    kinds.extend(bodies.iter().copied().map(constructor));
                }
                if let Some(sources) = aliases.get(&source) {
                    pending.extend(sources);
                }
            }
            (kinds.len() == 1).then(|| *kinds.first().unwrap())
        };
        let mut demands: BTreeMap<SymbolId, BTreeSet<usize>> = BTreeMap::new();
        let mut working = parameters.clone();
        // Actual constructors refine already-demanded callbacks, not arbitrary
        // identity formals. Their own result must still pass real body checking.
        let mut constructors_settled = false;
        // Stabilize constructor inputs before comparing dependent call shapes.
        for _ in 0..8 {
            let general = storage::general_mutable_bindings(
                semantic,
                functions,
                &working,
                &signatures.borrow(),
                promise_parameters,
                returns.stored,
            );
            demands.clear();
            for call in calls {
                let (sources, mut bodies) =
                    returns.expression(invocation::callee(call), &BTreeMap::new(), 0, 0);
                let mut pending = sources.into_iter().collect::<Vec<_>>();
                let mut seen = BTreeSet::new();
                while let Some(source) = pending.pop() {
                    if !seen.insert(source) {
                        continue;
                    }
                    if let Some(known) = targets.get(&source) {
                        bodies.extend(known);
                    }
                    if let Some(sources) = aliases.get(&source) {
                        pending.extend(sources);
                    }
                }
                for body in bodies {
                    for (parameter, argument) in arrows[&body]
                        .params
                        .items
                        .iter()
                        .zip(invocation::arguments(call))
                    {
                        let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern
                        else {
                            continue;
                        };
                        let Some(symbol) = identifier.symbol_id.get() else {
                            continue;
                        };
                        let Some(previous) = working
                            .get(&body)
                            .and_then(|parameters| parameters.get(&symbol))
                        else {
                            continue;
                        };
                        if let Some(signature) = argument.as_expression().and_then(|expression| {
                            function_kind(expression, &working, true, &general)
                        }) && signatures
                            .borrow()
                            .parameter_count(signature)
                            .zip(signatures.borrow().parameter_count(*previous))
                            .is_some_and(|(declared, supplied)| declared <= supplied)
                        {
                            demands.entry(symbol).or_default().insert(signature);
                        }
                    }
                }
            }
            let previous = working.clone();
            for (symbol, kinds) in &demands {
                if kinds.len() != 1 {
                    return Err(
                        "callback arguments require one fixed complete function signature".into(),
                    );
                }
                working
                    .get_mut(&owners[symbol])
                    .unwrap()
                    .insert(*symbol, *kinds.first().unwrap());
            }
            if working == previous {
                constructors_settled = true;
                break;
            }
        }
        if !constructors_settled {
            return Err("callback constructor signatures exceed eight rounds".into());
        }
        // Constructor seeds have served their purpose; compare final call and
        // target shapes after any demanded-formal promotion in this phase.
        demands.clear();
        let mut target_refined = false;
        let general = storage::general_mutable_bindings(
            semantic,
            functions,
            &working,
            &signatures.borrow(),
            promise_parameters,
            returns.stored,
        );
        for call in calls {
            let (sources, _) = origins(invocation::callee(call), semantic);
            let expected = function_kind(invocation::callee(call), &working, false, &general);
            let mut arguments = Vec::new();
            let declared =
                expected.and_then(|signature| signatures.borrow().parameter_count(signature));
            for (index, argument) in invocation::arguments(call)
                .iter()
                .take(declared.unwrap_or(invocation::arguments(call).len()))
                .enumerate()
            {
                let Some(expression) = argument.as_expression() else {
                    arguments.push(Type::Number);
                    continue;
                };
                let actual = function_kind(expression, &working, false, &general);
                let expected =
                    expected.and_then(|signature| signatures.borrow().parameter(signature, index));
                if let Some(Type::Closure(signature)) = expected {
                    // This is a demand on the original formal even if an alias
                    // already reveals its constructor. Unused identity formals
                    // receive no such demand and remain polymorphic.
                    let (symbols, _) = returns.expression(expression, &BTreeMap::new(), 0, 0);
                    let mut pending = symbols.into_iter().collect::<Vec<_>>();
                    let mut seen = BTreeSet::new();
                    while let Some(symbol) = pending.pop() {
                        if !seen.insert(symbol) {
                            continue;
                        }
                        if let Some(body) = owners.get(&symbol) {
                            working.get_mut(body).unwrap().insert(symbol, signature);
                            demands.entry(symbol).or_default().insert(signature);
                        }
                        if let Some(sources) = aliases.get(&symbol) {
                            pending.extend(sources);
                        }
                    }
                    arguments.push(Type::Closure(actual.unwrap_or(signature)));
                } else {
                    arguments.push(match actual {
                        Some(signature) => Type::Closure(signature),
                        None if expected == Some(Type::AsyncValue) => Type::AsyncValue,
                        None if expected == Some(Type::Promise) => Type::Promise,
                        None if expected == Some(Type::FulfilledPromise) => Type::FulfilledPromise,
                        None if expected.is_some_and(Type::is_managed) => expected.unwrap(),
                        None if expected == Some(Type::General) => Type::General,
                        None => Type::Number,
                    });
                }
            }
            // A runtime callback's signature constrains even an unused formal
            // of its concrete target. Ordinary known-call identity arrows do
            // not acquire this monomorphic demand from their callers.
            if sources
                .iter()
                .any(|symbol| working.values().any(|inputs| inputs.contains_key(symbol)))
            {
                let mut pending = sources.iter().copied().collect::<Vec<_>>();
                let mut seen = BTreeSet::new();
                while let Some(source) = pending.pop() {
                    if !seen.insert(source) {
                        continue;
                    }
                    if let Some(bodies) = targets.get(&source) {
                        for body in bodies {
                            for (parameter, argument) in
                                arrows[body].params.items.iter().zip(&arguments)
                            {
                                if let (
                                    BindingPattern::BindingIdentifier(identifier),
                                    Type::Closure(signature),
                                ) = (&parameter.pattern, argument)
                                    && let Some(symbol) = identifier.symbol_id.get()
                                {
                                    target_refined |=
                                        working.get_mut(body).unwrap().insert(symbol, *signature)
                                            != Some(*signature);
                                }
                            }
                        }
                    }
                    if let Some(sources) = aliases.get(&source) {
                        pending.extend(sources);
                    }
                }
            }
            let result = expected
                .and_then(|signature| signatures.borrow().result(signature))
                .unwrap_or(Type::Number);
            let asynchronous = expected.is_some_and(|id| signatures.borrow().is_async(id));
            let signature = if asynchronous {
                signatures.borrow_mut().intern_async(arguments)
            } else {
                signatures.borrow_mut().intern(arguments, result)
            };
            for source in sources {
                demands.entry(source).or_default().insert(signature);
            }
        }
        if target_refined {
            parameters = working;
            continue;
        }
        for parameter in working.values() {
            for symbol in parameter.keys() {
                if let Some(bodies) = targets.get(symbol) {
                    for body in bodies {
                        let signature = constructor_kind(*body, &working, &general);
                        demands.entry(*symbol).or_default().insert(signature);
                    }
                }
            }
        }
        loop {
            let mut additions = Vec::new();
            for (alias, sources) in aliases {
                if let Some(kinds) = demands.get(alias) {
                    for source in sources {
                        for kind in kinds {
                            additions.push((*source, *kind));
                        }
                    }
                }
            }
            let mut changed = false;
            for (source, kind) in additions {
                changed |= demands.entry(source).or_default().insert(kind);
            }
            if !changed {
                break;
            }
        }
        let mut next = working.clone();
        for (body, parameter) in &working {
            for symbol in parameter.keys() {
                if let Some(kinds) = demands.get(symbol) {
                    if kinds.len() != 1 {
                        return Err(unsupported(
                            arrows[body],
                            "callback parameters require one fixed complete function signature",
                        ));
                    }
                    next.get_mut(body)
                        .unwrap()
                        .insert(*symbol, *kinds.first().unwrap());
                }
            }
        }
        if next == parameters {
            return Ok(parameters);
        }
        parameters = next;
    }
    Err("callback signature propagation exceeds eight rounds".into())
}

// Callback demands describe finite Number/General-result signatures. An arrow's own shape
// follows parameter order, while source syntax and TypeScript names prove no kind.
pub(super) struct ValueInputs<'s> {
    pub general: &'s BTreeSet<SymbolId>,
    pub managed: &'s BTreeMap<SymbolId, Type>,
}
pub(super) fn parameter_types(
    arrow: &oxc_ast::ast::ArrowFunctionExpression<'_>,
    parameters: &Parameters,
    inputs: ValueInputs<'_>,
    promises: &super::promises::Inputs,
) -> Vec<Type> {
    arrow
        .params
        .items
        .iter()
        .map(|parameter| {
            let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern else {
                return Type::Number;
            };
            let Some(symbol) = identifier.symbol_id.get() else {
                return Type::Number;
            };
            parameters
                .get(&arrow.node_id.get())
                .and_then(|parameters| parameters.get(&symbol))
                .map_or_else(
                    || {
                        if let Some(ty) = promises.parameter(&symbol) {
                            ty
                        } else if !arrow.r#async
                            && let Some(ty) = inputs.managed.get(&symbol)
                        {
                            *ty
                        } else if inputs.general.contains(&symbol) {
                            Type::General
                        } else {
                            Type::Number
                        }
                    },
                    |signature| Type::Closure(*signature),
                )
        })
        .collect()
}

#[cfg(test)]
mod return_origin_stack_tests {
    use super::*;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_semantic::SemanticBuilder;
    use oxc_span::SourceType;

    #[test]
    fn nested_conditional_origins_have_bounded_native_stack_and_stable_union() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                let mut expression = "(() => 1)".to_string();
                const DEPTH: usize = 96;
                for _ in 0..DEPTH {
                    expression = format!("a ? {expression} : (() => 2)");
                }
                let source = format!("{expression};");
                let allocator = Allocator::default();
                let parsed = Parser::new(&allocator, &source, SourceType::mjs()).parse();
                assert!(!parsed.fatal_error && parsed.diagnostics.is_empty());
                let built = SemanticBuilder::new_compiler()
                    .with_build_nodes(true)
                    .build(&parsed.program);
                assert!(built.diagnostics.is_empty());
                let oxc_ast::ast::Statement::ExpressionStatement(statement) =
                    &parsed.program.body[0]
                else {
                    panic!("expected expression statement");
                };
                let targets = BTreeMap::new();
                let arrows = BTreeMap::new();
                let returns = ReturnOrigins {
                    semantic: &built.semantic,
                    stored: &stored_functions::Fields::default(),
                    targets: &targets,
                    arrows: &arrows,
                };
                let (symbols, bodies) =
                    returns.expression(&statement.expression, &BTreeMap::new(), 0, 0);
                assert!(symbols.is_empty());
                assert_eq!(bodies.len(), DEPTH + 1);
            })
            .expect("small-stack callback-origin thread")
            .join()
            .expect("iterative return-origin query completed");
    }
}
