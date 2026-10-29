//! Source facts for fulfilled snapshots, Promise identities and private mixed values.
//! This query follows original symbols and constructors; it never executes a call.
use super::*;
use oxc_ast::AstKind;
use oxc_syntax::node::NodeId;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(crate) struct Inputs {
    pub(super) identities: bool,
    named_async: BTreeSet<SymbolId>,
    fulfilled: BTreeSet<SymbolId>,
    pending: BTreeSet<SymbolId>,
    pub(super) values: BTreeSet<SymbolId>,
    // May contain a Promise; separate from the homogeneous value/signature fact.
    choices: BTreeSet<SymbolId>,
    mixed: BTreeSet<SymbolId>,
    pub(super) results: BTreeSet<NodeId>,
    pub(super) mixed_results: BTreeSet<NodeId>,
    pub(super) mixed_calls: BTreeSet<NodeId>,
    pub(crate) asynchronous: BTreeSet<NodeId>,
    pub(super) adopting: BTreeSet<NodeId>,
}

impl Inputs {
    fn named_async_call(
        &self,
        callee: &Expression<'_>,
        returns: &callbacks::ReturnOrigins<'_, '_, '_>,
    ) -> bool {
        let (sources, mut bodies) = returns.expression(callee, &BTreeMap::new(), 0, 0);
        let present = !sources.is_empty() || !bodies.is_empty();
        for source in sources {
            if self.named_async.contains(&source) {
                continue;
            }
            let Some(known) = returns.targets.get(&source) else {
                return false;
            };
            bodies.extend(known);
        }
        present
            && bodies
                .iter()
                .all(|body| self.results.contains(body) && !self.mixed_results.contains(body))
    }

    pub(super) fn is_mixed(&self, symbol: &SymbolId) -> bool {
        self.mixed.contains(symbol)
    }
    pub(super) fn is_fulfilled(&self, symbol: &SymbolId) -> bool {
        self.fulfilled.contains(symbol)
            && !self.pending.contains(symbol)
            && !self.mixed.contains(symbol)
    }

    pub(crate) fn parameter(&self, symbol: &SymbolId) -> Option<Type> {
        if self.mixed.contains(symbol) {
            Some(Type::AsyncValue)
        } else if self.pending.contains(symbol) {
            Some(Type::Promise)
        } else if self.fulfilled.contains(symbol) {
            Some(Type::FulfilledPromise)
        } else {
            None
        }
    }
}

// Homogeneous Promise proof stays separate from conservative may-Promise facts.
// Every actual call and body is still checked against its physical signature.
fn pending_shape(
    expression: &Expression<'_>,
    semantic: &Semantic<'_>,
    returns: &callbacks::ReturnOrigins<'_, '_, '_>,
    inputs: &Inputs,
) -> bool {
    let mut work = vec![(expression, 0)];
    while let Some((expression, depth)) = work.pop() {
        if depth > 128 {
            return false;
        }
        if let Some(inner) = erased_inner(expression) {
            work.push((inner, depth + 1));
            continue;
        }
        match expression {
            Expression::ParenthesizedExpression(p) => work.push((&p.expression, depth + 1)),
            Expression::Identifier(id) => {
                if !id
                    .reference_id
                    .get()
                    .and_then(|r| semantic.scoping().get_reference(r).symbol_id())
                    .is_some_and(|s| inputs.values.contains(&s) && !inputs.mixed.contains(&s))
                {
                    return false;
                }
            }
            Expression::CallExpression(call) => {
                if inputs.identities && inputs.named_async_call(&call.callee, returns) {
                    continue;
                }
                if let Some(intrinsic) = awaited::intrinsic(call, semantic).ok().flatten()
                    && (inputs.identities || !intrinsic.rejected)
                {
                    if inputs.identities {
                        continue;
                    }
                    let Some(argument) = intrinsic.argument else {
                        return false;
                    };
                    work.push((argument, depth + 1));
                } else if !callees(&call.callee, returns).is_some_and(|bodies| {
                    bodies.iter().all(|body| {
                        inputs.results.contains(body) && !inputs.mixed_results.contains(body)
                    })
                }) {
                    return false;
                }
            }
            Expression::ConditionalExpression(c) => {
                work.push((&c.consequent, depth + 1));
                work.push((&c.alternate, depth + 1));
            }
            Expression::LogicalExpression(l) => {
                work.push((&l.left, depth + 1));
                work.push((&l.right, depth + 1));
            }
            Expression::SequenceExpression(s) => {
                let Some(last) = s.expressions.last() else {
                    return false;
                };
                work.push((last, depth + 1));
            }
            Expression::AssignmentExpression(a)
                if matches!(
                    a.operator,
                    AssignmentOperator::Assign
                        | AssignmentOperator::LogicalAnd
                        | AssignmentOperator::LogicalOr
                        | AssignmentOperator::LogicalNullish
                ) =>
            {
                work.push((&a.right, depth + 1))
            }
            _ => return false,
        }
    }
    true
}

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

// Named declarations are planned separately from arrow bodies. Keep both sides
// of a selected callable in the input graph without indexing a declaration as
// an arrow. Unknown source symbols still supply no target proof.
fn input_callees(
    expression: &Expression<'_>,
    returns: &callbacks::ReturnOrigins<'_, '_, '_>,
    named: Option<&BTreeSet<SymbolId>>,
) -> Option<BTreeSet<NodeId>> {
    let (sources, mut bodies) = returns.expression(expression, &BTreeMap::new(), 0, 0);
    for source in sources {
        if let Some(known) = returns.targets.get(&source) {
            bodies.extend(known);
        } else if !named.is_some_and(|named| named.contains(&source)) {
            return None;
        }
    }
    (!bodies.is_empty()).then_some(bodies)
}

// Function effects need any selected Promise-return leaf, while ordinary value
// positions still require the homogeneous kind proved by pending_shape.
fn return_adopts(
    expression: &Expression<'_>,
    semantic: &Semantic<'_>,
    returns: &callbacks::ReturnOrigins<'_, '_, '_>,
    inputs: &Inputs,
) -> bool {
    let mut work = vec![(expression, 0)];
    while let Some((node, depth)) = work.pop() {
        let Some((node, depth)) = closures::unwrapped_at(node, depth) else {
            return false;
        };
        match node {
            Expression::CallExpression(call)
                if callees(&call.callee, returns)
                    .is_some_and(|bodies| !bodies.is_disjoint(&inputs.mixed_results)) =>
            {
                return true;
            }
            Expression::AssignmentExpression(assignment) => {
                if assignment
                    .left
                    .as_simple_assignment_target()
                    .and_then(|target| target_identifier(target, depth).ok().flatten())
                    .and_then(|id| id.reference_id.get())
                    .and_then(|r| semantic.scoping().get_reference(r).symbol_id())
                    .is_some_and(|s| inputs.choices.contains(&s))
                {
                    return true;
                }
                work.push((&assignment.right, depth + 1));
            }
            Expression::Identifier(id)
                if id
                    .reference_id
                    .get()
                    .and_then(|r| semantic.scoping().get_reference(r).symbol_id())
                    .is_some_and(|s| inputs.choices.contains(&s)) =>
            {
                return true;
            }
            Expression::ConditionalExpression(c) => {
                work.push((&c.consequent, depth + 1));
                work.push((&c.alternate, depth + 1));
            }
            Expression::LogicalExpression(l) => {
                // Conservatively retain the activation route for a logical
                // Promise operand. Actual selection happens before adoption.
                work.push((&l.left, depth + 1));
                work.push((&l.right, depth + 1));
            }
            Expression::SequenceExpression(s) => {
                if let Some(last) = s.expressions.last() {
                    work.push((last, depth + 1));
                }
            }
            _ if pending_shape(node, semantic, returns, inputs) => return true,
            _ => (),
        }
    }
    false
}

pub(crate) fn parameters(
    semantic: &Semantic<'_>,
    identities: bool,
    functions: &[NumericFunction],
) -> Inputs {
    let mut named_async: BTreeSet<_> = functions
        .iter()
        .filter(|f| f.async_signature.is_some())
        .map(|f| f.symbol)
        .collect();
    let mut arrows = BTreeMap::new();
    let mut named_parameters = BTreeMap::new();
    let mut definitions = Vec::new();
    let mut uninitialized = BTreeSet::new();
    let mut calls = Vec::new();
    let mut has_constructor = !named_async.is_empty();
    for node in semantic.nodes().iter() {
        match node.kind() {
            AstKind::ArrowFunctionExpression(arrow) => {
                arrows.insert(arrow.node_id.get(), arrow);
                has_constructor |= arrow.r#async;
            }
            AstKind::Function(function) => {
                if let Some(symbol) = function.id.as_ref().and_then(|id| id.symbol_id.get())
                    && named_async.contains(&symbol)
                {
                    named_parameters.insert(symbol, &function.params);
                }
            }
            AstKind::VariableDeclaration(declaration) => {
                for local in &declaration.declarations {
                    if declaration.kind == VariableDeclarationKind::Let
                        && local.init.is_none()
                        && let BindingPattern::BindingIdentifier(binding) = &local.id
                        && let Some(symbol) = binding.symbol_id.get()
                    {
                        uninitialized.insert(symbol);
                    }
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
                    .is_some_and(|intrinsic| identities || !intrinsic.rejected);
                calls.push(call);
            }
            _ => (),
        }
    }
    // Inputs originate from explicit constructors or source async calls.
    // Facts below grow monotonically over finite original symbol/arrow sets;
    // source budgets remain the actual lowerer's responsibility.
    if !has_constructor {
        return Inputs::default();
    }
    let mut targets: BTreeMap<SymbolId, BTreeSet<NodeId>> = BTreeMap::new();
    let named_targets = loop {
        let returns = callbacks::ReturnOrigins {
            semantic,
            targets: &targets,
            arrows: &arrows,
        };
        let named_targets =
            callbacks::named_async_targets(&returns, functions, &definitions, &calls);
        named_async.extend(named_targets.keys().copied());
        let mut additions = Vec::new();
        for &(symbol, expression) in &definitions {
            if let Some(bodies) =
                input_callees(expression, &returns, identities.then_some(&named_async))
            {
                additions.extend(bodies.into_iter().map(|body| (symbol, body)));
            }
        }
        for call in &calls {
            let Some(bodies) =
                input_callees(&call.callee, &returns, identities.then_some(&named_async))
            else {
                continue;
            };
            for body in bodies {
                for (parameter, argument) in arrows[&body].params.items.iter().zip(&call.arguments)
                {
                    if let BindingPattern::BindingIdentifier(binding) = &parameter.pattern
                        && let (Some(symbol), Some(expression)) =
                            (binding.symbol_id.get(), argument.as_expression())
                        && let Some(bodies) =
                            input_callees(expression, &returns, identities.then_some(&named_async))
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
            break named_targets;
        }
    };
    let returns = callbacks::ReturnOrigins {
        semantic,
        targets: &targets,
        arrows: &arrows,
    };
    let mut named_incoming = Vec::new();
    if identities {
        // Actual inputs join the same original-source provenance graph as local
        // initializers/writes. Resolving targets never evaluates an argument.
        for call in &calls {
            let (sources, _) = returns.expression(&call.callee, &BTreeMap::new(), 0, 0);
            for target in sources
                .iter()
                .filter_map(|source| named_targets.get(source))
                .flatten()
            {
                if let Some(parameters) = named_parameters.get(target) {
                    for (parameter, argument) in parameters.items.iter().zip(&call.arguments) {
                        if let BindingPattern::BindingIdentifier(binding) = &parameter.pattern
                            && let (Some(symbol), Some(expression)) =
                                (binding.symbol_id.get(), argument.as_expression())
                        {
                            definitions.push((symbol, expression));
                            named_incoming.push((symbol, expression));
                        }
                    }
                }
            }
            let Some(bodies) = input_callees(&call.callee, &returns, Some(&named_async)) else {
                continue;
            };
            for body in bodies {
                for (parameter, argument) in arrows[&body].params.items.iter().zip(&call.arguments)
                {
                    if let BindingPattern::BindingIdentifier(binding) = &parameter.pattern
                        && let (Some(symbol), Some(expression)) =
                            (binding.symbol_id.get(), argument.as_expression())
                    {
                        definitions.push((symbol, expression));
                    }
                }
            }
        }
    }
    let mut promises = BTreeSet::new();
    loop {
        let mut additions = Vec::new();
        for call in &calls {
            let Some(bodies) =
                input_callees(&call.callee, &returns, identities.then_some(&named_async))
            else {
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
            break;
        }
    }
    let mut inputs = Inputs {
        identities,
        named_async,
        fulfilled: promises,
        ..Inputs::default()
    };
    inputs
        .results
        .extend(arrows.iter().filter_map(|(id, arrow)| {
            (arrow.r#async && (identities || !awaited::immediate_async_arrow(arrow, semantic)))
                .then_some(*id)
        }));
    // Match the existing async effect interface shared by selected constructors.
    loop {
        let old = inputs.results.len();
        for bodies in targets.values() {
            if !bodies.is_disjoint(&inputs.results) {
                inputs
                    .results
                    .extend(bodies.iter().filter(|id| arrows[id].r#async));
            }
        }
        if old == inputs.results.len() {
            break;
        }
    }
    inputs
        .asynchronous
        .extend(arrows.iter().filter_map(|(id, arrow)| {
            (arrow.r#async && !awaited::immediate_async_arrow(arrow, semantic)).then_some(*id)
        }));
    // Cleanup keeps returns raw, then resolves them outside every finalizer.
    // This conditional adoption boundary requires the child's own activation.
    inputs
        .adopting
        .extend(arrows.iter().filter_map(|(id, arrow)| {
            (arrow.r#async && awaited::owns_finally(arrow, semantic)).then_some(*id)
        }));
    loop {
        let old = (
            inputs.values.len(),
            inputs.choices.len(),
            inputs.pending.len(),
            inputs.results.len(),
            inputs.asynchronous.len(),
        );
        for (symbol, expression) in &definitions {
            if pending_shape(expression, semantic, &returns, &inputs) {
                inputs.values.insert(*symbol);
            }
            if inputs.identities && return_adopts(expression, semantic, &returns, &inputs) {
                inputs.choices.insert(*symbol);
            }
        }
        for (symbol, expression) in &named_incoming {
            if pending_shape(expression, semantic, &returns, &inputs) {
                inputs.pending.insert(*symbol);
                inputs.values.insert(*symbol);
            }
        }
        for call in &calls {
            let Some(bodies) = input_callees(
                &call.callee,
                &returns,
                inputs.identities.then_some(&inputs.named_async),
            ) else {
                continue;
            };
            for body in bodies {
                for (parameter, argument) in arrows[&body].params.items.iter().zip(&call.arguments)
                {
                    if let BindingPattern::BindingIdentifier(binding) = &parameter.pattern
                        && let (Some(symbol), Some(expression)) =
                            (binding.symbol_id.get(), argument.as_expression())
                        && pending_shape(expression, semantic, &returns, &inputs)
                    {
                        inputs.pending.insert(symbol);
                        inputs.values.insert(symbol);
                    }
                }
            }
        }
        for (id, arrow) in &arrows {
            let results = if arrow.r#async {
                arrow
                    .get_expression()
                    .map(|body| vec![body])
                    .unwrap_or_else(|| {
                        super::block_results::return_statements(semantic, *id)
                            .into_iter()
                            .filter_map(|s| s.argument.as_ref())
                            .collect()
                    })
            } else {
                storage::arrow_returns(arrow, semantic).unwrap_or_default()
            };
            if !results.is_empty() {
                let facts: Vec<_> = results
                    .into_iter()
                    .map(|e| {
                        if arrow.r#async {
                            return_adopts(e, semantic, &returns, &inputs)
                        } else {
                            pending_shape(e, semantic, &returns, &inputs)
                        }
                    })
                    .collect();
                if arrow.r#async && facts.iter().any(|p| *p) {
                    inputs.results.insert(*id);
                    inputs.asynchronous.insert(*id);
                    inputs.adopting.insert(*id);
                } else if !arrow.r#async && facts.iter().all(|p| *p) {
                    inputs.results.insert(*id);
                }
            }
        }
        for bodies in targets.values() {
            if !bodies.is_disjoint(&inputs.asynchronous) {
                let added: Vec<_> = bodies
                    .iter()
                    .filter(|id| arrows[id].r#async)
                    .copied()
                    .collect();
                inputs.results.extend(&added);
                inputs.asynchronous.extend(added);
            }
        }
        if old
            == (
                inputs.values.len(),
                inputs.choices.len(),
                inputs.pending.len(),
                inputs.results.len(),
                inputs.asynchronous.len(),
            )
        {
            // Classify unions only after positive constructor/alias facts converge.
            // Then propagate mixed provenance through aliases without treating a
            // may-Promise fact as a homogeneous Promise signature.
            if identities {
                loop {
                    let old = (
                        inputs.mixed.len(),
                        inputs.choices.len(),
                        inputs.mixed_results.len(),
                        inputs.asynchronous.len(),
                    );
                    for (symbol, expression) in &definitions {
                        if return_adopts(expression, semantic, &returns, &inputs) {
                            inputs.choices.insert(*symbol);
                        }
                    }
                    let mixed: Vec<_> = definitions
                        .iter()
                        .filter_map(|(symbol, expression)| {
                            (inputs.choices.contains(symbol)
                                && (uninitialized.contains(symbol)
                                    || !pending_shape(expression, semantic, &returns, &inputs)))
                            .then_some(*symbol)
                        })
                        .collect();
                    inputs.mixed.extend(mixed);
                    for (id, arrow) in &arrows {
                        let expressions = arrow
                            .get_expression()
                            .map(|body| vec![body])
                            .unwrap_or_else(|| {
                                super::block_results::return_statements(semantic, *id)
                                    .into_iter()
                                    .filter_map(|s| s.argument.as_ref())
                                    .collect()
                            });
                        if !expressions
                            .iter()
                            .any(|e| return_adopts(e, semantic, &returns, &inputs))
                        {
                            continue;
                        }
                        if arrow.r#async {
                            inputs.results.insert(*id);
                            inputs.asynchronous.insert(*id);
                            inputs.adopting.insert(*id);
                        } else if storage::arrow_returns(arrow, semantic).is_none()
                            || expressions
                                .iter()
                                .any(|e| !pending_shape(e, semantic, &returns, &inputs))
                        {
                            inputs.mixed_results.insert(*id);
                        }
                    }
                    for bodies in targets.values() {
                        if !bodies.is_disjoint(&inputs.mixed_results)
                            || (!bodies.is_disjoint(&inputs.results)
                                && bodies.iter().any(|id| !inputs.results.contains(id)))
                        {
                            inputs
                                .mixed_results
                                .extend(bodies.iter().filter(|id| !arrows[id].r#async).copied());
                        }
                        if !bodies.is_disjoint(&inputs.asynchronous) {
                            let added: Vec<_> = bodies
                                .iter()
                                .filter(|id| arrows[id].r#async)
                                .copied()
                                .collect();
                            inputs.results.extend(&added);
                            inputs.asynchronous.extend(added);
                        }
                    }
                    if old
                        == (
                            inputs.mixed.len(),
                            inputs.choices.len(),
                            inputs.mixed_results.len(),
                            inputs.asynchronous.len(),
                        )
                    {
                        break;
                    }
                }
                inputs.mixed_calls.extend(calls.iter().filter_map(|call| {
                    callees(&call.callee, &returns)
                        .is_some_and(|bodies| !bodies.is_disjoint(&inputs.mixed_results))
                        .then_some(call.node_id.get())
                }));
            }
            return inputs;
        }
    }
}
