//! Finite original-source heap alternatives; never effects or runtime targets.
use super::*;
use oxc_ast::{AstKind, ast::ObjectPropertyKind};
use oxc_syntax::{GetNodeId, node::NodeId};
use std::collections::{BTreeMap, BTreeSet};
mod elements;
#[cfg(test)]
mod iteration_tests;
mod member_origins;
mod mutations;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Origin {
    Object(NodeId),
    NullObject(NodeId),
    Array(NodeId),
    Arrow(NodeId),
    Named(SymbolId),
    String,
    Undefined,
    Other,
}
type Alternatives = BTreeSet<Origin>;
type Key = Vec<u16>;
enum Rule {
    Iterated(NodeId),
    Known(Origin),
    Copy(Vec<NodeId>),
    Default(NodeId, NodeId),
    Symbol(SymbolId),
    Member(NodeId, Key),
    Call(NodeId, Vec<NodeId>, Option<(NodeId, bool)>, Option<NodeId>),
    StringConcat(NodeId, NodeId),
}
enum Definition {
    Value(SymbolId, NodeId),
    Iterated(SymbolId, NodeId),
}
struct FunctionFlow {
    parameters: Vec<SymbolId>,
    returns: Vec<NodeId>,
    implicit: bool,
}
#[derive(Default)]
pub(crate) struct Fields {
    values: BTreeMap<NodeId, Alternatives>,
}
fn symbol(id: &IdentifierReference<'_>, semantic: &Semantic<'_>) -> Option<SymbolId> {
    id.reference_id
        .get()
        .and_then(|reference| semantic.scoping().get_reference(reference).symbol_id())
}
fn extend<K: Ord>(map: &mut BTreeMap<K, Alternatives>, key: K, values: Alternatives) -> bool {
    let entry = map.entry(key).or_default();
    let before = entry.len();
    entry.extend(values);
    before != entry.len()
}

/// Pattern nodes retain the same source alternatives as each ordered member
/// projection. Keeping intermediate nodes makes nested receiver proofs available
/// without treating a binding's syntax or annotation as a runtime kind.
fn pattern_definitions(
    pattern: &BindingPattern<'_>,
    root: Rule,
    definitions: &mut Vec<Definition>,
    rules: &mut Vec<(NodeId, Rule)>,
) {
    if matches!(pattern, BindingPattern::BindingIdentifier(_)) {
        return;
    }
    let mut pending = vec![(pattern, root)];
    while let Some((pattern, rule)) = pending.pop() {
        let id = pattern.node_id();
        rules.push((id, rule));
        match pattern {
            BindingPattern::BindingIdentifier(identifier) => {
                if let Some(symbol) = identifier.symbol_id.get() {
                    definitions.push(Definition::Value(symbol, id));
                }
            }
            BindingPattern::AssignmentPattern(assignment) => {
                pending.push((
                    &assignment.left,
                    Rule::Default(id, assignment.right.node_id()),
                ));
            }
            BindingPattern::ObjectPattern(object) => {
                if let Some(rest) = &object.rest {
                    // The fresh allocation keeps its own writes separate from the source.
                    pending.push((&rest.argument, Rule::Known(Origin::Object(rest.node_id()))));
                }
                for property in object.properties.iter().rev() {
                    let rule = static_keys::property(&property.key, property.computed, 0)
                        .map(|key| Rule::Member(id, key))
                        .unwrap_or(Rule::Known(Origin::Other));
                    pending.push((&property.value, rule));
                }
            }
            BindingPattern::ArrayPattern(array) => {
                if let Some(rest) = &array.rest {
                    // Rest constructs a fresh array allocation. Its copied
                    // reads and iteration track the source tail separately from writes to the copy.
                    pending.push((&rest.argument, Rule::Known(Origin::Array(rest.node_id()))));
                }
                for (index, element) in array.elements.iter().enumerate().rev() {
                    if let Some(element) = element {
                        pending.push((
                            element,
                            Rule::Member(id, index.to_string().encode_utf16().collect()),
                        ));
                    }
                }
            }
        }
    }
}

impl Fields {
    pub(super) fn ordinary_object(&self, node: NodeId) -> bool {
        self.values.get(&node).is_some_and(|values| {
            !values.is_empty()
                && values
                    .iter()
                    .all(|origin| matches!(origin, Origin::Object(_)))
        })
    }

    pub(super) fn dictionary(&self, node: NodeId) -> bool {
        self.values.get(&node).is_some_and(|values| {
            !values.is_empty()
                && values
                    .iter()
                    .all(|origin| matches!(origin, Origin::NullObject(_)))
        })
    }
    pub(super) fn string(&self, node: NodeId) -> bool {
        self.values
            .get(&node)
            .is_some_and(|values| values.len() == 1 && values.contains(&Origin::String))
    }
    pub(super) fn container(&self, node: NodeId) -> Option<Type> {
        let values = self.values.get(&node)?;
        let ty = match values.first()? {
            Origin::Object(_) | Origin::NullObject(_) => Type::ManagedObject,
            Origin::Array(_) => Type::ManagedArray,
            _ => return None,
        };
        values
            .iter()
            .all(|origin| {
                matches!(
                    (ty, origin),
                    (
                        Type::ManagedObject,
                        Origin::Object(_) | Origin::NullObject(_)
                    ) | (Type::ManagedArray, Origin::Array(_))
                )
            })
            .then_some(ty)
    }
    pub(super) fn functions(&self, node: NodeId) -> Option<&Alternatives> {
        let values = self.values.get(&node)?;
        (!values.is_empty()
            && values
                .iter()
                .all(|origin| matches!(origin, Origin::Arrow(_) | Origin::Named(_))))
        .then_some(values)
    }
    pub(super) fn origins(&self, node: NodeId) -> (BTreeSet<SymbolId>, BTreeSet<NodeId>) {
        let mut named = BTreeSet::new();
        let mut arrows = BTreeSet::new();
        for origin in self.values.get(&node).into_iter().flatten() {
            match origin {
                Origin::Named(symbol) => {
                    named.insert(*symbol);
                }
                Origin::Arrow(body) => {
                    arrows.insert(*body);
                }
                _ => (),
            }
        }
        (named, arrows)
    }
    pub(crate) fn collect(semantic: &Semantic<'_>, entry: SymbolId) -> Self {
        if !semantic.nodes().iter().any(|node| {
            matches!(
                node.kind(),
                AstKind::ObjectExpression(_) | AstKind::ArrayExpression(_) | AstKind::StringLiteral(_)
            ) || matches!(node.kind(), AstKind::UnaryExpression(u) if u.operator == UnaryOperator::Typeof)
                || matches!(node.kind(), AstKind::CallExpression(call) if object_create::argument(call, semantic).is_some() || object_intrinsics::enumeration(call, semantic).is_some())
                || matches!(node.kind(), AstKind::ArrayPattern(array) if array.rest.is_some())
        }) {
            return Self::default();
        }
        let mut values = BTreeMap::new();
        let mut bindings: BTreeMap<SymbolId, Alternatives> = BTreeMap::new();
        let iterations = semantic
            .nodes()
            .iter()
            .filter_map(|node| {
                let AstKind::ForOfStatement(source) = node.kind() else {
                    return None;
                };
                iteration::for_of_binding(source)
                    .and_then(|binding| binding.symbol_id.get())
                    .map(|symbol| (symbol, source.right.node_id()))
            })
            .collect::<BTreeMap<_, _>>();
        let mut definitions = iterations
            .iter()
            .map(|(symbol, receiver)| Definition::Iterated(*symbol, *receiver))
            .collect::<Vec<_>>();
        let mut rules = Vec::new();
        let mut pattern_rules = Vec::new();
        let patterned_iterations = semantic
            .nodes()
            .iter()
            .filter_map(|node| {
                let AstKind::ForOfStatement(source) = node.kind() else {
                    return None;
                };
                let pattern = iteration::for_of_pattern(source)?;
                (!matches!(pattern, BindingPattern::BindingIdentifier(_)))
                    .then_some((pattern, source.right.node_id()))
            })
            .collect::<Vec<_>>();
        let has_iterations = !iterations.is_empty() || !patterned_iterations.is_empty();
        for (pattern, receiver) in patterned_iterations {
            pattern_definitions(
                pattern,
                Rule::Iterated(receiver),
                &mut definitions,
                &mut pattern_rules,
            );
        }
        let mut fields: BTreeMap<NodeId, BTreeMap<Key, Option<NodeId>>> = BTreeMap::new();
        // A spread or untracked property key can introduce an unlisted field.
        // Explicit properties after it may still establish their own origins.
        let mut open_fields = BTreeSet::new();
        let mut rest_sources = BTreeMap::new();
        let mut elements = elements::Elements::default();
        let mut enumeration_inputs = Vec::new();
        let mut slice_inputs = Vec::new();
        let mut writes = Vec::new();
        let mut mutations = Vec::new();
        let mut functions = BTreeMap::new();
        let parameters = |parameters: &oxc_ast::ast::FormalParameters<'_>| {
            parameters
                .items
                .iter()
                .filter_map(|parameter| match &parameter.pattern {
                    BindingPattern::BindingIdentifier(binding) => binding.symbol_id.get(),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        for node in semantic.nodes().iter() {
            let id = node.id();
            if let AstKind::CallExpression(call) = node.kind()
                && let Some((receiver, string_methods::Method::Slice)) = string_methods::call(call)
            {
                // Only proven Array.slice calls acquire this allocation origin.
                // Dynamic endpoints may select any subset of indexed values.
                elements.enumerated_values.entry(id).or_default();
                slice_inputs.push((id, receiver.node_id()));
            }
            if let AstKind::ArrayPattern(pattern) = node.kind()
                && let Some(rest) = &pattern.rest
                && let Ok(start) = u32::try_from(pattern.elements.len())
            {
                elements.add_rest(rest.node_id(), id, start);
            }
            if let AstKind::ObjectPattern(pattern) = node.kind()
                && let Some(rest) = &pattern.rest
                && let Some(excluded) = pattern
                    .properties
                    .iter()
                    .map(|property| static_keys::property(&property.key, property.computed, 0))
                    .collect::<Option<BTreeSet<_>>>()
            {
                rest_sources.insert(
                    rest.node_id(),
                    member_origins::Rest {
                        receiver: id,
                        excluded,
                    },
                );
            }
            if let AstKind::CallExpression(call) = node.kind()
                && let Some(mutation) = mutations::call(call)
            {
                mutations.push(mutation);
            }
            let numeric_target = match node.kind() {
                AstKind::UpdateExpression(update) => {
                    target_identifier(&update.argument, 0).ok().flatten()
                }
                AstKind::AssignmentExpression(assignment)
                    if !matches!(
                        assignment.operator,
                        AssignmentOperator::Assign
                            | AssignmentOperator::LogicalAnd
                            | AssignmentOperator::LogicalOr
                            | AssignmentOperator::LogicalNullish
                    ) =>
                {
                    assignment
                        .left
                        .as_simple_assignment_target()
                        .and_then(|target| target_identifier(target, 0).ok().flatten())
                }
                _ => None,
            };
            if let Some(target) = numeric_target.and_then(|name| symbol(name, semantic)) {
                // A numeric update or compound write can replace an initially
                // Undefined General cell. Keep its non-Undefined alternative,
                // including through captured and erased TypeScript targets.
                bindings.entry(target).or_default().insert(Origin::Other);
            }
            let rule = match node.kind() {
                AstKind::CallExpression(call)
                    if let Some(kind) = object_intrinsics::enumeration(call, semantic) =>
                {
                    let value_site = match kind {
                        ir::OwnEnumeration::Keys => {
                            elements.key_lists.insert(id);
                            None
                        }
                        ir::OwnEnumeration::Entries => {
                            // The callee is a distinct allocation site for rows returned by this call.
                            let pair = call.callee.node_id();
                            elements.entry_lists.insert(id, pair);
                            elements.pair_sites.insert(pair);
                            Some(pair)
                        }
                        ir::OwnEnumeration::Values => Some(id),
                    };
                    if let Some(site) = value_site
                        && let Some(receiver) = invocation::arguments(call)
                            .first()
                            .and_then(|arg| arg.as_expression())
                    {
                        elements.enumerated_values.entry(site).or_default();
                        enumeration_inputs.push((site, receiver.node_id()));
                    }
                    values.insert(id, BTreeSet::from([Origin::Array(id)]));
                    None
                }
                AstKind::CallExpression(call)
                    if object_create::argument(call, semantic).is_some() =>
                {
                    fields.insert(id, BTreeMap::new());
                    values.insert(id, BTreeSet::from([Origin::NullObject(id)]));
                    None
                }
                AstKind::StringLiteral(_) => {
                    values.insert(id, BTreeSet::from([Origin::String]));
                    None
                }
                AstKind::UnaryExpression(u) if u.operator == UnaryOperator::Typeof => {
                    values.insert(id, BTreeSet::from([Origin::String]));
                    None
                }
                AstKind::UnaryExpression(u) if u.operator == UnaryOperator::Void => {
                    values.insert(id, BTreeSet::from([Origin::Undefined]));
                    None
                }
                AstKind::BinaryExpression(b) if b.operator == BinaryOperator::Addition => {
                    Some(Rule::StringConcat(b.left.node_id(), b.right.node_id()))
                }
                AstKind::ObjectExpression(object) => {
                    let mut own = BTreeMap::new();
                    for property in &object.properties {
                        if matches!(property, ObjectPropertyKind::SpreadProperty(_)) {
                            // A runtime spread may overwrite any earlier field.
                            // A later explicit property can establish it again.
                            open_fields.insert(id);
                            for value in own.values_mut() {
                                *value = None;
                            }
                        }
                        if let ObjectPropertyKind::ObjectProperty(property) = property {
                            let name = static_keys::property(&property.key, property.computed, 0);
                            if let Some(name) = name {
                                own.insert(name, Some(property.value.node_id()));
                            } else {
                                open_fields.insert(id);
                                for value in own.values_mut() {
                                    *value = None;
                                }
                            }
                        }
                    }
                    fields.insert(id, own);
                    values.insert(id, BTreeSet::from([Origin::Object(id)]));
                    None
                }
                AstKind::ArrayExpression(array)
                    if let Some(source) = iteration::single_spread(array) =>
                {
                    // A single spread copies the entire iterator into a fresh array.
                    elements.add_rest(id, source.node_id(), 0);
                    values.insert(id, BTreeSet::from([Origin::Array(id)]));
                    None
                }
                AstKind::ArrayExpression(array) => {
                    if array.elements.iter().any(|element| {
                        matches!(
                            element,
                            oxc_ast::ast::ArrayExpressionElement::SpreadElement(_)
                        )
                    }) {
                        // Mixed literals with spread are still opaque. Do not let their
                        // unknown width masquerade as fixed holes or positions.
                        open_fields.insert(id);
                    }
                    fields.insert(
                        id,
                        array
                            .elements
                            .iter()
                            .enumerate()
                            .map(|(index, value)| {
                                (
                                    index.to_string().encode_utf16().collect(),
                                    value.as_expression().map(GetNodeId::node_id),
                                )
                            })
                            .collect(),
                    );
                    values.insert(id, BTreeSet::from([Origin::Array(id)]));
                    None
                }
                AstKind::ArrowFunctionExpression(arrow) => {
                    let complete = storage::arrow_returns(arrow, semantic);
                    let returns = complete
                        .as_ref()
                        .map(|values| values.iter().map(|value| value.node_id()).collect())
                        .unwrap_or_else(|| {
                            block_results::return_statements(semantic, id)
                                .iter()
                                .filter_map(|statement| {
                                    statement.argument.as_ref().map(GetNodeId::node_id)
                                })
                                .collect()
                        });
                    functions.insert(
                        Origin::Arrow(id),
                        FunctionFlow {
                            parameters: parameters(&arrow.params),
                            returns,
                            implicit: complete.is_none(),
                        },
                    );
                    values.insert(id, BTreeSet::from([Origin::Arrow(id)]));
                    None
                }
                AstKind::Function(function) => {
                    // Function expressions remain unsupported; declaration values
                    // enter through their resolved binding, not this AST node.
                    values.insert(id, BTreeSet::from([Origin::Other]));
                    if let Some(name) = function.id.as_ref().and_then(|name| name.symbol_id.get()) {
                        bindings.insert(name, BTreeSet::from([Origin::Named(name)]));
                        let returns = block_results::return_statements(semantic, id)
                            .iter()
                            .filter_map(|statement| {
                                statement.argument.as_ref().map(GetNodeId::node_id)
                            })
                            .collect::<Vec<_>>();
                        let implicit = returns.is_empty();
                        functions.insert(
                            Origin::Named(name),
                            FunctionFlow {
                                parameters: parameters(&function.params),
                                returns,
                                implicit,
                            },
                        );
                        if name == entry {
                            for parameter in &function.params.items {
                                if let BindingPattern::BindingIdentifier(binding) =
                                    &parameter.pattern
                                    && let Some(symbol) = binding.symbol_id.get()
                                {
                                    bindings.insert(symbol, BTreeSet::from([Origin::Other]));
                                }
                            }
                        }
                    }
                    None
                }
                AstKind::VariableDeclarator(declaration) => {
                    if let BindingPattern::BindingIdentifier(name) = &declaration.id
                        && let Some(symbol) = name.symbol_id.get()
                    {
                        if let Some(initializer) = &declaration.init {
                            definitions.push(Definition::Value(symbol, initializer.node_id()));
                        } else if !iterations.contains_key(&symbol) {
                            bindings
                                .entry(symbol)
                                .or_default()
                                .insert(Origin::Undefined);
                        }
                    }
                    if let Some(initializer) = &declaration.init {
                        pattern_definitions(
                            &declaration.id,
                            Rule::Copy(vec![initializer.node_id()]),
                            &mut definitions,
                            &mut pattern_rules,
                        );
                    }
                    None
                }
                AstKind::IdentifierReference(identifier) => {
                    if let Some(symbol) = symbol(identifier, semantic) {
                        Some(Rule::Symbol(symbol))
                    } else {
                        let origin = if identifier.name == "undefined" {
                            Origin::Undefined
                        } else {
                            Origin::Other
                        };
                        values.insert(id, BTreeSet::from([origin]));
                        None
                    }
                }
                AstKind::ParenthesizedExpression(w) => {
                    Some(Rule::Copy(vec![w.expression.node_id()]))
                }
                AstKind::TSAsExpression(w) => Some(Rule::Copy(vec![w.expression.node_id()])),
                AstKind::TSTypeAssertion(w) => Some(Rule::Copy(vec![w.expression.node_id()])),
                AstKind::TSNonNullExpression(w) => Some(Rule::Copy(vec![w.expression.node_id()])),
                AstKind::TSSatisfiesExpression(w) => Some(Rule::Copy(vec![w.expression.node_id()])),
                AstKind::TSInstantiationExpression(w) => {
                    Some(Rule::Copy(vec![w.expression.node_id()]))
                }
                AstKind::ConditionalExpression(c) => Some(Rule::Copy(vec![
                    c.consequent.node_id(),
                    c.alternate.node_id(),
                ])),
                AstKind::LogicalExpression(c) => {
                    Some(Rule::Copy(vec![c.left.node_id(), c.right.node_id()]))
                }
                AstKind::SequenceExpression(s) => Some(Rule::Copy(
                    s.expressions
                        .last()
                        .map(GetNodeId::node_id)
                        .into_iter()
                        .collect(),
                )),
                AstKind::CallExpression(call) => Some(Rule::Call(
                    invocation::callee(call).node_id(),
                    invocation::arguments(call)
                        .iter()
                        .filter_map(|argument| argument.as_expression().map(GetNodeId::node_id))
                        .collect(),
                    string_methods::call(call)
                        .filter(|(_, method)| method.result() == Type::ManagedValue)
                        .map(|(receiver, method)| {
                            (
                                receiver.node_id(),
                                matches!(method, string_methods::Method::Slice),
                            )
                        }),
                    array_mutations::operation(call)
                        .filter(|(op, _)| *op == array_mutations::Operation::Reverse)
                        .map(|(_, r)| r.node_id()),
                )),
                AstKind::AssignmentExpression(a)
                    if matches!(
                        a.operator,
                        AssignmentOperator::LogicalAnd
                            | AssignmentOperator::LogicalOr
                            | AssignmentOperator::LogicalNullish
                    ) =>
                {
                    // A function binding may select another factory while its
                    // generic result ABI stays unchanged. Retain every target;
                    // provenance never selects the branch executed at runtime.
                    if let Some(name) = a
                        .left
                        .as_simple_assignment_target()
                        .and_then(|target| target_identifier(target, 0).ok().flatten())
                        && let Some(target) = symbol(name, semantic)
                    {
                        definitions.push(Definition::Value(target, a.right.node_id()));
                        Some(Rule::Symbol(target))
                    } else {
                        values.insert(id, BTreeSet::from([Origin::Other]));
                        None
                    }
                }
                AstKind::AssignmentExpression(a) if a.operator == AssignmentOperator::Assign => {
                    if let Some(name) = a
                        .left
                        .as_simple_assignment_target()
                        .and_then(|target| target_identifier(target, 0).ok().flatten())
                    {
                        if let Some(symbol) = symbol(name, semantic) {
                            definitions.push(Definition::Value(symbol, a.right.node_id()));
                        }
                    } else {
                        match a.left.as_simple_assignment_target() {
                            Some(SimpleAssignmentTarget::StaticMemberExpression(m)) => {
                                writes.push((
                                    m.object.node_id(),
                                    m.property.name.encode_utf16().collect(),
                                    a.right.node_id(),
                                ))
                            }
                            Some(SimpleAssignmentTarget::ComputedMemberExpression(m)) => {
                                if let Some(key) = closures::unwrapped(&m.expression, 0)
                                    .and_then(static_keys::literal)
                                {
                                    writes.push((m.object.node_id(), key, a.right.node_id()));
                                } else {
                                    mutations.push(mutations::Mutation::AnyKeyWrite {
                                        receiver: m.object.node_id(),
                                        value: a.right.node_id(),
                                    });
                                }
                            }
                            _ => (),
                        }
                    }
                    Some(Rule::Copy(vec![a.right.node_id()]))
                }
                AstKind::StaticMemberExpression(m) if !m.optional => Some(Rule::Member(
                    m.object.node_id(),
                    m.property.name.encode_utf16().collect(),
                )),
                AstKind::ComputedMemberExpression(m) if !m.optional => {
                    if let Some(key) =
                        closures::unwrapped(&m.expression, 0).and_then(static_keys::literal)
                    {
                        Some(Rule::Member(m.object.node_id(), key))
                    } else {
                        // A dynamic read is an unknown boxed value, not an
                        // unresolved fact that a conditional can silently omit.
                        values.insert(id, BTreeSet::from([Origin::Other]));
                        None
                    }
                }
                _ => {
                    values.insert(id, BTreeSet::from([Origin::Other]));
                    None
                }
            };
            if let Some(rule) = rule {
                rules.push((id, rule));
            }
        }
        let has_patterns = !pattern_rules.is_empty();
        for (id, rule) in pattern_rules {
            // The ordinary AST pass does not interpret binding nodes as values.
            // Their original-source rules replace its generic Other seed.
            values.remove(&id);
            rules.push((id, rule));
        }
        let mut field_writes: BTreeMap<(NodeId, Key), Alternatives> = BTreeMap::new();
        let mut wildcard_writes: BTreeMap<NodeId, Alternatives> = BTreeMap::new();
        let mut index_deletes = BTreeSet::new();
        let track_elements = has_iterations
            || !elements.rest_sources.is_empty()
            || !elements.entry_lists.is_empty()
            || !elements.key_lists.is_empty()
            || !enumeration_inputs.is_empty()
            || !slice_inputs.is_empty();
        let mut finalized_unresolved = false;
        loop {
            let get = |id: &NodeId| values.get(id).cloned().unwrap_or_default();
            let members = member_origins::Members {
                semantic,
                values: &values,
                fields: &fields,
                open_fields: &open_fields,
                rest_sources: &rest_sources,
                elements: &elements,
                field_writes: &field_writes,
                wildcard_writes: &wildcard_writes,
                index_deletes: &index_deletes,
            };
            let mut symbol_additions = definitions
                .iter()
                .map(|definition| match definition {
                    Definition::Value(symbol, id) => (*symbol, get(id)),
                    Definition::Iterated(symbol, receiver) => (
                        *symbol,
                        elements.select(*receiver, &values, &fields, &index_deletes),
                    ),
                })
                .collect::<Vec<_>>();
            let mut additions = Vec::new();
            for (id, rule) in &rules {
                let found = match rule {
                    Rule::Iterated(receiver) => {
                        elements.select(*receiver, &values, &fields, &index_deletes)
                    }
                    Rule::Known(origin) => BTreeSet::from([*origin]),
                    Rule::Copy(inputs) => inputs.iter().flat_map(get).collect(),
                    Rule::Default(selected, fallback) => {
                        // Only positively known Undefined is replaced. Other
                        // still includes null, primitives and unknown receivers.
                        // Keep the fallback union conservative even when the
                        // selected alternatives do not currently need it.
                        let mut found = get(selected);
                        found.remove(&Origin::Undefined);
                        found.extend(get(fallback));
                        found
                    }
                    Rule::Symbol(symbol) => bindings.get(symbol).cloned().unwrap_or_default(),
                    Rule::StringConcat(left, right) => {
                        let left = get(left);
                        let right = get(right);
                        if left.is_empty() || right.is_empty() {
                            Alternatives::new()
                        } else if left
                            .iter()
                            .chain(&right)
                            .all(|origin| *origin == Origin::String)
                        {
                            BTreeSet::from([Origin::String])
                        } else {
                            BTreeSet::from([Origin::Other])
                        }
                    }
                    Rule::Call(callee, arguments, string_receiver, array_receiver) => {
                        if let Some(receiver) = array_receiver {
                            let origins = get(receiver);
                            if !origins.is_empty()
                                && origins
                                    .iter()
                                    .all(|origin| matches!(origin, Origin::Array(_)))
                            {
                                additions.push((*id, origins));
                                continue;
                            }
                        }
                        if let Some((receiver, slice)) = string_receiver {
                            let origins = get(receiver);
                            if *slice
                                && !origins.is_empty()
                                && origins
                                    .iter()
                                    .all(|origin| matches!(origin, Origin::Array(_)))
                            {
                                additions.push((*id, BTreeSet::from([Origin::Array(*id)])));
                                continue;
                            }
                            if !origins.is_empty()
                                && origins.iter().all(|origin| *origin == Origin::String)
                            {
                                additions.push((*id, BTreeSet::from([Origin::String])));
                                continue;
                            }
                        }
                        // Preserve ordinary methods with the same spelling. If
                        // a provisional String later gains Other, this path adds
                        // that alternative rather than laundering a String proof.
                        let mut found = Alternatives::new();
                        for target in get(callee) {
                            if let Some(function) = functions.get(&target) {
                                for (index, symbol) in function.parameters.iter().enumerate() {
                                    symbol_additions.push((
                                        *symbol,
                                        arguments
                                            .get(index)
                                            .map(get)
                                            .unwrap_or_else(|| BTreeSet::from([Origin::Other])),
                                    ));
                                }
                                for expression in &function.returns {
                                    found.extend(get(expression));
                                }
                                if function.implicit {
                                    found.insert(Origin::Other);
                                }
                            } else {
                                found.insert(Origin::Other);
                            }
                        }
                        found
                    }
                    Rule::Member(receiver, key) => members.select(*receiver, key),
                };
                additions.push((*id, found));
            }
            let snapshots = enumeration_inputs
                .iter()
                .map(|&(site, receiver)| (site, members.enumerable_values(receiver)))
                .chain(slice_inputs.iter().map(|&(site, receiver)| {
                    (
                        site,
                        elements.select(receiver, &values, &fields, &index_deletes),
                    )
                }))
                .collect::<Vec<_>>();
            let mut changed = false;
            for (site, origins) in snapshots {
                changed |= extend(&mut elements.enumerated_values, site, origins);
            }
            let mut write_additions = Vec::new();
            for (receiver, key, value) in &writes {
                for origin in get(receiver) {
                    if let Origin::Object(object)
                    | Origin::NullObject(object)
                    | Origin::Array(object) = origin
                    {
                        write_additions.push(((object, key.clone()), get(value)));
                        if track_elements && matches!(origin, Origin::Array(_)) {
                            changed |= elements.write(object, key, get(value), &fields);
                        }
                        if matches!(origin, Origin::Array(_))
                            && key.as_slice() == [108, 101, 110, 103, 116, 104]
                        {
                            let length = literal_length(semantic, *value);
                            for index in fields.get(&object).into_iter().flat_map(|own| own.keys())
                            {
                                let numeric = String::from_utf16_lossy(index).parse::<u32>().ok();
                                if numeric.is_some_and(|index| {
                                    length.is_none_or(|length| index >= length)
                                }) {
                                    write_additions.push((
                                        (object, index.clone()),
                                        BTreeSet::from([Origin::Other]),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            let mut deleted = Vec::new();
            let mut wildcard_additions = Vec::new();
            for mutation in &mutations {
                match mutation {
                    mutations::Mutation::Reorder(receiver) => {
                        for origin in get(receiver) {
                            if let Origin::Array(array) = origin {
                                changed |= elements.reordered.insert(array);
                            }
                        }
                    }
                    mutations::Mutation::IndexDelete(receiver) => {
                        for origin in get(receiver) {
                            if let Origin::Array(array) = origin {
                                deleted.push(array);
                            }
                        }
                    }
                    mutations::Mutation::AnyKeyWrite { receiver, value } => {
                        // A computed write may replace any key of every source
                        // allocation reached through aliases, captures or calls.
                        for origin in get(receiver) {
                            if let Origin::Object(object)
                            | Origin::NullObject(object)
                            | Origin::Array(object) = origin
                            {
                                wildcard_additions.push((object, get(value)));
                                if track_elements && matches!(origin, Origin::Array(_)) {
                                    changed |= elements.wildcard(object, get(value));
                                }
                            }
                        }
                    }
                }
            }
            for array in deleted {
                changed |= index_deletes.insert(array);
            }
            for (object, found) in wildcard_additions {
                changed |= extend(&mut wildcard_writes, object, found);
            }
            for (symbol, found) in symbol_additions {
                changed |= extend(&mut bindings, symbol, found);
            }
            for (id, found) in additions {
                changed |= extend(&mut values, id, found);
            }
            for (key, found) in write_additions {
                changed |= extend(&mut field_writes, key, found);
            }
            if !changed
                && (has_patterns
                    || has_iterations
                    || !elements.rest_sources.is_empty()
                    || !slice_inputs.is_empty())
                && !finalized_unresolved
            {
                finalized_unresolved = true;
                // Bottom can be an unresolved cyclic value, not evidence that
                // no element exists. Finalize only after ordinary convergence
                // so delayed aliases and helper results retain their precision.
                for (id, _) in &rules {
                    if values.get(id).is_none_or(Alternatives::is_empty) {
                        changed |= extend(&mut values, *id, BTreeSet::from([Origin::Other]));
                    }
                }
            }
            if !changed {
                break;
            }
        }
        Self { values }
    }
}

fn literal_length(semantic: &Semantic<'_>, mut node: NodeId) -> Option<u32> {
    for _ in 0..=128 {
        node = match semantic.nodes().kind(node) {
            AstKind::NumericLiteral(number)
                if number.value >= 0.0
                    && number.value <= f64::from(u32::MAX)
                    && number.value.fract() == 0.0 =>
            {
                return Some(number.value as u32);
            }
            AstKind::ParenthesizedExpression(w) => w.expression.node_id(),
            AstKind::TSAsExpression(w) => w.expression.node_id(),
            AstKind::TSTypeAssertion(w) => w.expression.node_id(),
            AstKind::TSNonNullExpression(w) => w.expression.node_id(),
            AstKind::TSSatisfiesExpression(w) => w.expression.node_id(),
            AstKind::TSInstantiationExpression(w) => w.expression.node_id(),
            _ => return None,
        };
    }
    None
}
