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
    pub targets: &'s BTreeMap<SymbolId, BTreeSet<NodeId>>,
    pub arrows: &'s BTreeMap<NodeId, &'n oxc_ast::ast::ArrowFunctionExpression<'a>>,
}

impl ReturnOrigins<'_, '_, '_> {
    // Follow source-known concise and block results. Actual argument origins are
    // computed before shadowing formals; no call effects or argument bindings
    // escape this read-only query. The same eight-call bound limits recursion.
    pub(super) fn expression(
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
            Expression::LogicalExpression(logical) => {
                let mut result = self.expression(&logical.left, bindings, depth + 1, calls);
                let other = self.expression(&logical.right, bindings, depth + 1, calls);
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
                    if arrow.params.items.len() > call.arguments.len() {
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
) -> Result<Parameters, String> {
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
    // Initial seeds use Number-leaf arities; refinement fills higher parameter
    // kinds. Block result joins independently compare complete signature IDs.
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
    let declared_targets = targets.clone();
    // Calls through an aliased callback can reveal another known target's
    // parameter origins. Saturate this finite source relation before shapes.
    loop {
        let mut additions = Vec::new();
        for call in &calls {
            let (sources, mut bodies) = origins(&call.callee, semantic);
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
                for (parameter, argument) in arrows[&body].params.items.iter().zip(&call.arguments)
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
            targets: &targets,
            arrows: &arrows,
        };
        let mut peers = targets.values().cloned().collect::<Vec<_>>();
        for call in &calls {
            let (sources, mut bodies) = returns.expression(&call.callee, &BTreeMap::new(), 0, 0);
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
                &kinds.inputs,
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
                                let (sources, mut bodies) =
                                    returns.expression(&call.callee, &BTreeMap::new(), 0, 0);
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
                                let (sources, mut bodies) =
                                    returns.expression(&call.callee, &BTreeMap::new(), 0, 0);
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
            parameter_types(arrow, parameters, &kinds.inputs, promise_parameters),
            if (promise_parameters.identities || !arrow.r#async)
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
            );
            demands.clear();
            for call in calls {
                let (sources, mut bodies) =
                    returns.expression(&call.callee, &BTreeMap::new(), 0, 0);
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
                    for (parameter, argument) in
                        arrows[&body].params.items.iter().zip(&call.arguments)
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
        );
        for call in calls {
            let (sources, _) = origins(&call.callee, semantic);
            let expected = function_kind(&call.callee, &working, false, &general);
            let mut arguments = Vec::new();
            let declared =
                expected.and_then(|signature| signatures.borrow().parameter_count(signature));
            for (index, argument) in call
                .arguments
                .iter()
                .take(declared.unwrap_or(call.arguments.len()))
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
                        None if expected == Some(Type::Promise) => Type::Promise,
                        None if expected == Some(Type::FulfilledPromise) => Type::FulfilledPromise,
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
pub(super) fn parameter_types(
    arrow: &oxc_ast::ast::ArrowFunctionExpression<'_>,
    parameters: &Parameters,
    general: &BTreeSet<SymbolId>,
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
                        } else if general.contains(&symbol) {
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
