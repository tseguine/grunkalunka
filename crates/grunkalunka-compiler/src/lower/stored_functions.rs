//! Finite original-source heap alternatives; never effects or runtime targets.
use super::*;
use oxc_ast::{
    AstKind,
    ast::{ObjectPropertyKind, PropertyKey},
};
use oxc_syntax::{GetNodeId, node::NodeId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Origin {
    Object(NodeId),
    Array(NodeId),
    Arrow(NodeId),
    Named(SymbolId),
    Other,
}
type Alternatives = BTreeSet<Origin>;
type Key = Vec<u16>;
enum Rule {
    Copy(Vec<NodeId>),
    Symbol(SymbolId),
    Member(NodeId, Key),
    Call(NodeId, Vec<NodeId>),
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
fn key(node: &Expression<'_>) -> Option<Key> {
    match closures::unwrapped(node, 0)? {
        Expression::StringLiteral(s) => Some(s.value.encode_utf16().collect()),
        Expression::NumericLiteral(n)
            if n.value >= 0.0 && n.value < f64::from(u32::MAX) && n.value.fract() == 0.0 =>
        {
            Some((n.value as u32).to_string().encode_utf16().collect())
        }
        _ => None,
    }
}
fn extend<K: Ord>(map: &mut BTreeMap<K, Alternatives>, key: K, values: Alternatives) -> bool {
    let entry = map.entry(key).or_default();
    let before = entry.len();
    entry.extend(values);
    before != entry.len()
}
impl Fields {
    pub(super) fn container(&self, node: NodeId) -> Option<Type> {
        let values = self.values.get(&node)?;
        let ty = match values.first()? {
            Origin::Object(_) => Type::ManagedObject,
            Origin::Array(_) => Type::ManagedArray,
            _ => return None,
        };
        values
            .iter()
            .all(|origin| {
                matches!(
                    (ty, origin),
                    (Type::ManagedObject, Origin::Object(_))
                        | (Type::ManagedArray, Origin::Array(_))
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
                AstKind::ObjectExpression(_) | AstKind::ArrayExpression(_)
            )
        }) {
            return Self::default();
        }
        let mut values = BTreeMap::new();
        let mut bindings = BTreeMap::new();
        let mut definitions = Vec::new();
        let mut rules = Vec::new();
        let mut fields: BTreeMap<NodeId, BTreeMap<Key, Option<NodeId>>> = BTreeMap::new();
        let mut writes = Vec::new();
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
            let rule = match node.kind() {
                AstKind::ObjectExpression(object) => {
                    let mut own = BTreeMap::new();
                    for property in &object.properties {
                        if let ObjectPropertyKind::ObjectProperty(property) = property {
                            let name = match &property.key {
                                PropertyKey::StaticIdentifier(k) if !property.computed => {
                                    Some(k.name.encode_utf16().collect())
                                }
                                PropertyKey::StringLiteral(k) => {
                                    Some(k.value.encode_utf16().collect())
                                }
                                _ => None,
                            };
                            if let Some(name) = name {
                                own.insert(name, Some(property.value.node_id()));
                            }
                        }
                    }
                    fields.insert(id, own);
                    values.insert(id, BTreeSet::from([Origin::Object(id)]));
                    None
                }
                AstKind::ArrayExpression(array) => {
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
                            definitions.push((symbol, initializer.node_id()));
                        } else {
                            bindings.entry(symbol).or_default().insert(Origin::Other);
                        }
                    }
                    None
                }
                AstKind::IdentifierReference(identifier) => {
                    if let Some(symbol) = symbol(identifier, semantic) {
                        Some(Rule::Symbol(symbol))
                    } else {
                        values.insert(id, BTreeSet::from([Origin::Other]));
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
                        definitions.push((target, a.right.node_id()));
                        Some(Rule::Symbol(target))
                    } else {
                        values.insert(id, BTreeSet::from([Origin::Other]));
                        None
                    }
                }
                AstKind::AssignmentExpression(a) if a.operator == AssignmentOperator::Assign => {
                    match a.left.as_simple_assignment_target() {
                        Some(SimpleAssignmentTarget::AssignmentTargetIdentifier(name)) => {
                            if let Some(symbol) = symbol(name, semantic) {
                                definitions.push((symbol, a.right.node_id()));
                            }
                        }
                        Some(SimpleAssignmentTarget::StaticMemberExpression(m)) => writes.push((
                            m.object.node_id(),
                            m.property.name.encode_utf16().collect(),
                            a.right.node_id(),
                        )),
                        Some(SimpleAssignmentTarget::ComputedMemberExpression(m)) => {
                            if let Some(key) = key(&m.expression) {
                                writes.push((m.object.node_id(), key, a.right.node_id()));
                            }
                        }
                        _ => (),
                    }
                    Some(Rule::Copy(vec![a.right.node_id()]))
                }
                AstKind::StaticMemberExpression(m) if !m.optional => Some(Rule::Member(
                    m.object.node_id(),
                    m.property.name.encode_utf16().collect(),
                )),
                AstKind::ComputedMemberExpression(m) if !m.optional => {
                    key(&m.expression).map(|key| Rule::Member(m.object.node_id(), key))
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
        let mut field_writes: BTreeMap<(NodeId, Key), Alternatives> = BTreeMap::new();
        loop {
            let get = |id: &NodeId| values.get(id).cloned().unwrap_or_default();
            let mut symbol_additions = definitions
                .iter()
                .map(|(symbol, id)| (*symbol, get(id)))
                .collect::<Vec<_>>();
            let mut additions = Vec::new();
            for (id, rule) in &rules {
                let found = match rule {
                    Rule::Copy(inputs) => inputs.iter().flat_map(get).collect(),
                    Rule::Symbol(symbol) => bindings.get(symbol).cloned().unwrap_or_default(),
                    Rule::Call(callee, arguments) => {
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
                    Rule::Member(receiver, key) => {
                        let mut found = Alternatives::new();
                        for origin in get(receiver) {
                            if let Origin::Object(object) | Origin::Array(object) = origin {
                                match fields.get(&object).and_then(|own| own.get(key)) {
                                    Some(Some(value)) => found.extend(get(value)),
                                    _ => {
                                        found.insert(Origin::Other);
                                    }
                                }
                                if let Some(written) = field_writes.get(&(object, key.clone())) {
                                    found.extend(written);
                                }
                            } else {
                                found.insert(Origin::Other);
                            }
                        }
                        found
                    }
                };
                additions.push((*id, found));
            }
            let mut write_additions = Vec::new();
            for (receiver, key, value) in &writes {
                for origin in get(receiver) {
                    if let Origin::Object(object) | Origin::Array(object) = origin {
                        write_additions.push(((object, key.clone()), get(value)));
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
            let mut changed = false;
            for (symbol, found) in symbol_additions {
                changed |= extend(&mut bindings, symbol, found);
            }
            for (id, found) in additions {
                changed |= extend(&mut values, id, found);
            }
            for (key, found) in write_additions {
                changed |= extend(&mut field_writes, key, found);
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
