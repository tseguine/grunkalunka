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
    Number,
    Boolean,
    Null,
    Undefined,
    Other,
}
type Alternatives = BTreeSet<Origin>;
type Key = Vec<u16>;
enum Rule {
    Iterated(NodeId),
    Known(Origin),
    Copy(Vec<NodeId>),
    AwaitPrimitive(NodeId),
    Logical(LogicalOperator, NodeId, NodeId),
    Default(NodeId, NodeId),
    Symbol(SymbolId),
    Member(NodeId, Key),
    OptionalMember(NodeId, Key),
    OptionalUnsignedMember(NodeId),
    OptionalComputedMember {
        receiver: NodeId,
        key: NodeId,
        domain: record_names::Domain,
    },
    StringKeyMember {
        receiver: NodeId,
        key: NodeId,
        domain: record_names::Domain,
    },
    UnsignedMember(NodeId),
    PatternUnsignedMember(NodeId),
    Call(
        NodeId,
        Vec<NodeId>,
        Option<(NodeId, string_methods::Method)>,
        Option<(NodeId, bool)>,
    ),
    Addition(NodeId, NodeId),
    PropertyAddition(NodeId, NodeId),
}
enum Definition {
    Symbol(SymbolId, SymbolId),
    Value(SymbolId, NodeId),
    Iterated(SymbolId, NodeId),
    ForIn(SymbolId),
}
struct FunctionFlow {
    parameters: Vec<SymbolId>,
    defaults: Vec<Option<NodeId>>,
    returns: Vec<NodeId>,
    implicit: bool,
    undefined_return: bool,
}
#[derive(Default)]
pub(crate) struct Fields {
    pub(super) uint32_keys: Uint32Keys,
    pub(super) record_names: RecordNames,
    values: BTreeMap<NodeId, Alternatives>,
    number_updates: BTreeSet<NodeId>,
    var_symbols: BTreeSet<SymbolId>,
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
    record_names: &RecordNames,
    uint32_keys: &Uint32Keys,
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
                        .or_else(|| {
                            property
                                .key
                                .as_expression()
                                .filter(|key| uint32_keys.contains(key, 0))
                                .map(|_| Rule::PatternUnsignedMember(id))
                        })
                        .or_else(|| {
                            let key = property.key.as_expression()?;
                            record_names
                                .contains(key, 0)
                                .then(|| Rule::StringKeyMember {
                                    receiver: id,
                                    key: key.node_id(),
                                    domain: record_names.domain(key, 0),
                                })
                        })
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

fn assignment_definition(
    identifier: &IdentifierReference<'_>,
    selection: Rule,
    site: NodeId,
    default: Option<&Expression<'_>>,
    semantic: &Semantic<'_>,
    definitions: &mut Vec<Definition>,
    rules: &mut Vec<(NodeId, Rule)>,
) {
    if let Some(symbol) = symbol(identifier, semantic) {
        let id = identifier.node_id();
        if let Some(default) = default {
            rules.push((site, selection));
            rules.push((id, Rule::Default(site, default.node_id())));
        } else {
            rules.push((id, selection));
        }
        definitions.push(Definition::Value(symbol, id));
    }
}

pub(super) fn undefined_return_bodies(semantic: &Semantic<'_>) -> BTreeSet<NodeId> {
    let mut bodies: BTreeSet<_> = semantic
        .nodes()
        .iter()
        .filter_map(|node| {
            let AstKind::ReturnStatement(statement) = node.kind() else {
                return None;
            };
            if statement.argument.is_some() {
                return None;
            }
            semantic
                .nodes()
                .ancestor_kinds(node.id())
                .find_map(|kind| match kind {
                    AstKind::Function(function) => {
                        Some((!function.r#async).then_some(function.node_id.get()))
                    }
                    AstKind::ArrowFunctionExpression(arrow) => {
                        Some((!arrow.r#async).then_some(arrow.node_id.get()))
                    }
                    _ => None,
                })
                .flatten()
        })
        .collect();
    for node in semantic.nodes().iter() {
        if let AstKind::Function(function) = node.kind()
            && !function.r#async
            && (block_results::return_statements(semantic, node.id()).is_empty()
                || block_results::normal_completion(function))
        {
            bodies.insert(node.id());
        }
    }
    bodies
}

pub(super) fn has_omitted_arguments(semantic: &Semantic<'_>, entry: SymbolId) -> bool {
    let arities: BTreeMap<_, _> = semantic
        .nodes()
        .iter()
        .filter_map(|node| {
            let AstKind::Function(function) = node.kind() else {
                return None;
            };
            let name = function.id.as_ref()?.symbol_id.get()?;
            (name != entry && !function.r#async && function.body.is_some())
                .then_some((name, function.params.items.len()))
        })
        .collect();
    semantic.nodes().iter().any(|node| {
        let AstKind::CallExpression(call) = node.kind() else {
            return false;
        };
        let Expression::Identifier(identifier) = invocation::callee(call) else {
            return false;
        };
        symbol(identifier, semantic)
            .and_then(|name| arities.get(&name))
            .is_some_and(|count| invocation::arguments(call).len() < *count)
    }) || managed_calls::has_omitted_arrow_arguments(semantic)
}

impl Fields {
    pub(super) fn var_symbol(&self, symbol: SymbolId) -> bool {
        self.var_symbols.contains(&symbol)
    }
    /// Strict tag/identity observations accept any proven represented alternative.
    pub(super) fn represented(&self, node: NodeId) -> bool {
        self.values.get(&node).is_some_and(|values| {
            !values.is_empty()
                && values.iter().all(|origin| {
                    matches!(
                        origin,
                        Origin::Object(_)
                            | Origin::NullObject(_)
                            | Origin::Array(_)
                            | Origin::Arrow(_)
                            | Origin::Named(_)
                            | Origin::String
                            | Origin::Number
                            | Origin::Boolean
                            | Origin::Null
                            | Origin::Undefined
                    )
                })
        })
    }

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
    pub(super) fn optional_dictionary(&self, node: NodeId) -> bool {
        self.values.get(&node).is_some_and(|values| {
            !values.is_empty()
                && values.iter().all(|origin| {
                    matches!(
                        origin,
                        Origin::NullObject(_) | Origin::Null | Origin::Undefined
                    )
                })
        })
    }
    pub(super) fn string(&self, node: NodeId) -> bool {
        self.values
            .get(&node)
            .is_some_and(|values| values.len() == 1 && values.contains(&Origin::String))
    }
    pub(super) fn non_null_primitive(&self, node: NodeId) -> bool {
        self.values.get(&node).is_some_and(|values| {
            !values.is_empty()
                && values.iter().all(|v| {
                    matches!(
                        v,
                        Origin::Boolean | Origin::Number | Origin::String | Origin::Undefined
                    )
                })
        })
    }

    pub(super) fn scalar_string_operand(&self, node: NodeId) -> bool {
        self.values.get(&node).is_some_and(|values| {
            !values.is_empty()
                && values.iter().all(|origin| {
                    matches!(
                        origin,
                        Origin::String
                            | Origin::Boolean
                            | Origin::Null
                            | Origin::Undefined
                            | Origin::Number
                    )
                })
        })
    }

    pub(super) fn may_be_number(&self, node: NodeId) -> bool {
        self.values
            .get(&node)
            .is_some_and(|values| values.contains(&Origin::Number))
    }

    pub(super) fn number(&self, node: NodeId) -> bool {
        self.values
            .get(&node)
            .is_some_and(|values| values.len() == 1 && values.contains(&Origin::Number))
    }
    pub(super) fn non_number_element(&self, node: NodeId) -> bool {
        self.values.get(&node).is_some_and(|values| {
            !values.is_empty()
                && values
                    .iter()
                    .all(|v| !matches!(v, Origin::Number | Origin::Other))
        })
    }
    pub(super) fn primitive(&self, node: NodeId) -> bool {
        self.values.get(&node).is_some_and(|values| {
            !values.is_empty()
                && values.iter().all(|origin| {
                    matches!(
                        origin,
                        Origin::Number | Origin::Boolean | Origin::Null | Origin::Undefined
                    )
                })
        })
    }
    pub(super) fn object_primitive(&self, node: NodeId) -> bool {
        self.values.get(&node).is_some_and(|values| {
            !values.is_empty()
                && values
                    .iter()
                    .all(|origin| matches!(origin, Origin::Number | Origin::Boolean))
        })
    }
    pub(super) fn nullish(&self, node: NodeId) -> bool {
        self.values.get(&node).is_some_and(|values| {
            !values.is_empty()
                && values
                    .iter()
                    .all(|origin| matches!(origin, Origin::Null | Origin::Undefined))
        })
    }
    pub(super) fn boolean(&self, node: NodeId) -> bool {
        self.values
            .get(&node)
            .is_some_and(|values| values.len() == 1 && values.contains(&Origin::Boolean))
    }
    pub(super) fn number_update(&self, node: NodeId) -> bool {
        self.number_updates.contains(&node)
    }
    pub(super) fn reference(&self, node: NodeId) -> bool {
        self.values.get(&node).is_some_and(|values| {
            !values.is_empty()
                && values.iter().all(|origin| {
                    matches!(
                        origin,
                        Origin::Object(_)
                            | Origin::NullObject(_)
                            | Origin::Array(_)
                            | Origin::Arrow(_)
                            | Origin::Named(_)
                    )
                })
        })
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
    pub(super) fn optional_receiver(&self, node: NodeId) -> Option<Type> {
        let mut roots = self
            .values
            .get(&node)?
            .iter()
            .filter(|origin| !matches!(origin, Origin::Null | Origin::Undefined));
        let ty = match roots.next()? {
            Origin::Object(_) | Origin::NullObject(_) => Type::ManagedObject,
            Origin::Array(_) => Type::ManagedArray,
            Origin::String => Type::ManagedValue,
            _ => return None,
        };
        roots
            .all(|origin| {
                matches!(
                    (ty, origin),
                    (
                        Type::ManagedObject,
                        Origin::Object(_) | Origin::NullObject(_)
                    ) | (Type::ManagedArray, Origin::Array(_))
                        | (Type::ManagedValue, Origin::String)
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
        let uint32_keys = Uint32Keys::collect(semantic);
        let record_names = RecordNames::collect(semantic);
        if !has_omitted_arguments(semantic,entry) && !managed_calls::has_called_undefined_returns(semantic,entry) && !helper_defaults::present(semantic) && !var_bindings::present(semantic) && !semantic.nodes().iter().any(|node| {
            object_intrinsics::array_constructor_site(node.kind(),semantic) || matches!(
                node.kind(),
                AstKind::ObjectExpression(_) | AstKind::ArrayExpression(_) | AstKind::StringLiteral(_) | AstKind::TemplateLiteral(_) | AstKind::ForInStatement(_) | AstKind::ChainExpression(_)
            ) || matches!(node.kind(), AstKind::UnaryExpression(u) if u.operator == UnaryOperator::Typeof)
                || matches!(node.kind(), AstKind::CallExpression(call) if object_create::argument(call, semantic).is_some() || object_intrinsics::enumeration(call, semantic).is_some() || object_intrinsics::conversion_call(call, semantic, 0).is_some())
                || matches!(node.kind(), AstKind::ArrayPattern(array) if array.rest.is_some())
                || matches!(node.kind(), AstKind::LogicalExpression(_))
                || matches!(node.kind(), AstKind::SwitchStatement(_))
                // Function-only loose comparisons also need reference proofs,
                // even when no literal heap container occurs in the module.
                || matches!(node.kind(), AstKind::BinaryExpression(binary) if matches!(binary.operator, BinaryOperator::Equality | BinaryOperator::Inequality))
        }) {
            return Self { uint32_keys, record_names, ..Self::default() };
        }
        let mut values = BTreeMap::new();
        let mut bindings: BTreeMap<SymbolId, Alternatives> = BTreeMap::new();
        let var_symbols = var_bindings::symbols(semantic);
        let var_parameters = var_bindings::parameters(semantic);
        let var_aliases = var_bindings::aliases(semantic);
        let var_reads = var_bindings::reads(semantic);
        for symbol in &var_symbols {
            if !var_parameters.contains(symbol) && !var_aliases.contains_key(symbol) {
                bindings
                    .entry(*symbol)
                    .or_default()
                    .insert(Origin::Undefined);
            }
        }

        // Only lexical iteration *declarations* start initialized by their
        // iteration step. Assignment heads must retain any earlier undefined
        // origin of their preexisting binding, even before the first step.
        let lexical_iteration_symbols = semantic
            .nodes()
            .iter()
            .filter_map(|node| match node.kind() {
                AstKind::ForOfStatement(source) => iteration::for_of_binding(source)
                    .and_then(|identifier| identifier.symbol_id.get()),
                AstKind::ForInStatement(source) => {
                    iteration::lexical_iteration_pattern(&source.left).and_then(|pattern| {
                        match pattern {
                            BindingPattern::BindingIdentifier(identifier) => {
                                identifier.symbol_id.get()
                            }
                            _ => None,
                        }
                    })
                }
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let iterations = semantic
            .nodes()
            .iter()
            .filter_map(|node| {
                let AstKind::ForOfStatement(source) = node.kind() else {
                    return None;
                };
                iteration::for_of_binding(source)
                    .or_else(|| iteration::var_iteration_binding(&source.left))
                    .and_then(|binding| binding.symbol_id.get())
                    .or_else(|| {
                        iteration::assignment_iteration_identifier(&source.left)
                            .ok()
                            .flatten()
                            .and_then(|identifier| symbol(identifier, semantic))
                    })
                    .map(|symbol| (symbol, source.right.node_id()))
            })
            .collect::<Vec<_>>();
        let mut definitions = iterations
            .iter()
            .map(|(symbol, receiver)| Definition::Iterated(*symbol, *receiver))
            .collect::<Vec<_>>();
        definitions.extend(
            var_aliases
                .into_iter()
                .map(|(target, source)| Definition::Symbol(target, source)),
        );

        // Unlike for-of, for-in always binds a String key, independently of
        // the receiver's property value types. Later assignments still widen it.
        for node in semantic.nodes().iter() {
            let AstKind::ForInStatement(source) = node.kind() else {
                continue;
            };
            let symbol = iteration::lexical_iteration_pattern(&source.left)
                .and_then(|pattern| match pattern {
                    BindingPattern::BindingIdentifier(binding) => binding.symbol_id.get(),
                    _ => None,
                })
                .or_else(|| {
                    iteration::var_iteration_binding(&source.left)
                        .and_then(|binding| binding.symbol_id.get())
                })
                .or_else(|| {
                    iteration::assignment_iteration_identifier(&source.left)
                        .ok()
                        .flatten()
                        .and_then(|identifier| symbol(identifier, semantic))
                });
            if let Some(symbol) = symbol {
                definitions.push(Definition::ForIn(symbol));
            }
        }
        let mut rules = Vec::new();
        let mut pattern_rules = Vec::new();
        let patterned_iterations = semantic
            .nodes()
            .iter()
            .filter_map(|node| {
                let (pattern, rule) = match node.kind() {
                    AstKind::ForOfStatement(source) => (
                        iteration::for_of_pattern(source)?,
                        Rule::Iterated(source.right.node_id()),
                    ),
                    AstKind::ForInStatement(source) => (
                        iteration::declaration_iteration_pattern(&source.left)?,
                        Rule::Known(Origin::String),
                    ),
                    _ => return None,
                };
                (!matches!(pattern, BindingPattern::BindingIdentifier(_)))
                    .then_some((pattern, rule))
            })
            .collect::<Vec<_>>();
        let mut has_iterations = !definitions.is_empty() || !patterned_iterations.is_empty();
        for (pattern, rule) in patterned_iterations {
            pattern_definitions(
                &record_names,
                &uint32_keys,
                pattern,
                rule,
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
        let mut indexed_copy_inputs = Vec::new();
        let mut writes = Vec::new();
        let mut member_iteration_rules = Vec::new();
        let mut numeric_targets = Vec::new();
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
            let member_iteration = match node.kind() {
                AstKind::ForOfStatement(source) => {
                    Some((&source.left, Rule::Iterated(source.right.node_id())))
                }
                AstKind::ForInStatement(source) => {
                    Some((&source.left, Rule::Known(Origin::String)))
                }
                _ => None,
            };
            if let Some((left, rule)) = member_iteration
                && let Some((receiver, key)) = iteration::property_iteration_target(left)
            {
                has_iterations = true;
                let value = left.node_id();
                member_iteration_rules.push((value, rule));
                mutations::write(
                    receiver.node_id(),
                    key,
                    value,
                    &record_names,
                    &mut writes,
                    &mut mutations,
                );
            }
            if let AstKind::UnaryExpression(unary) = node.kind()
                && let Some(mutation) = mutations::delete(unary, &uint32_keys, &record_names)
            {
                mutations.push(mutation);
            }
            if let AstKind::CallExpression(call) = node.kind()
                && let Some(receiver) = string_methods::call(call)
                    .filter(|(_, method)| matches!(method, string_methods::Method::Slice))
                    .map(|(r, _)| r)
                    .or_else(|| {
                        array_mutations::operation(call)
                            .filter(|(op, _)| *op == array_mutations::Operation::ToReversed)
                            .map(|(_, r)| r)
                    })
            {
                // Only proven Array copies acquire this allocation origin.
                // Slices select a subset; reversed copies reorder all indexed values.
                elements.enumerated_values.entry(id).or_default();
                indexed_copy_inputs.push((id, receiver.node_id()));
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
                        dynamic_excluded: false,
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
                    if assignment.operator != AssignmentOperator::Assign =>
                {
                    assignment
                        .left
                        .as_simple_assignment_target()
                        .and_then(|target| target_identifier(target, 0).ok().flatten())
                }
                _ => None,
            };
            if let Some(target) = numeric_target
                && let Some(symbol) = symbol(target, semantic)
            {
                // Admission needs the alternatives in the cell before numeric
                // updates. The expression's own rule separately proves the new
                // result kind, including String alternatives for += and the
                // selected RHS alternatives of logical assignments.
                numeric_targets.push((target.node_id(), id, symbol));
            }
            let numeric_property = match node.kind() {
                AstKind::UpdateExpression(update) => Some(&update.argument),
                AstKind::AssignmentExpression(assignment)
                    if evaluate::property_update::compound(assignment.operator).is_some()
                        || matches!(
                            assignment.operator,
                            AssignmentOperator::LogicalAnd
                                | AssignmentOperator::LogicalOr
                                | AssignmentOperator::LogicalNullish
                        ) =>
                {
                    assignment.left.as_simple_assignment_target()
                }
                _ => None,
            };
            if let Some(target) = numeric_property {
                match target {
                    SimpleAssignmentTarget::StaticMemberExpression(member) => writes.push((
                        member.object.node_id(),
                        member.property.name.encode_utf16().collect(),
                        id,
                    )),
                    SimpleAssignmentTarget::ComputedMemberExpression(member) => {
                        mutations::write(
                            member.object.node_id(),
                            iteration::PropertyKey::Runtime(&member.expression),
                            id,
                            &record_names,
                            &mut writes,
                            &mut mutations,
                        );
                    }
                    _ => (),
                }
            }
            let rule = match node.kind() {
                kind if let Some(args) = object_intrinsics::array_site_args(kind, semantic) => {
                    let args = invocation::plain_arguments(args);
                    let mut own = BTreeMap::new();
                    if args.len() == 1
                        && args
                            .first()
                            .and_then(|arg| arg.as_expression())
                            .is_some_and(|v| uint32_keys.unsigned_value(v))
                    {
                        elements.sparse_array(id);
                    } else {
                        for (index, arg) in args.iter().enumerate() {
                            if let Some(value) = arg.as_expression() {
                                own.insert(
                                    index.to_string().encode_utf16().collect(),
                                    Some(value.node_id()),
                                );
                            } else {
                                open_fields.insert(id);
                            }
                        }
                    }
                    fields.insert(id, own);
                    values.insert(id, BTreeSet::from([Origin::Array(id)]));
                    None
                }
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
                AstKind::StringLiteral(_) | AstKind::TemplateLiteral(_) => {
                    values.insert(id, BTreeSet::from([Origin::String]));
                    None
                }
                AstKind::NumericLiteral(_) => {
                    values.insert(id, BTreeSet::from([Origin::Number]));
                    None
                }
                AstKind::BooleanLiteral(_) => {
                    values.insert(id, BTreeSet::from([Origin::Boolean]));
                    None
                }
                AstKind::UnaryExpression(unary)
                    if matches!(
                        unary.operator,
                        UnaryOperator::LogicalNot | UnaryOperator::Delete
                    ) =>
                {
                    values.insert(id, BTreeSet::from([Origin::Boolean]));
                    None
                }
                AstKind::BinaryExpression(binary)
                    if matches!(
                        binary.operator,
                        BinaryOperator::Equality
                            | BinaryOperator::Inequality
                            | BinaryOperator::StrictEquality
                            | BinaryOperator::StrictInequality
                            | BinaryOperator::LessThan
                            | BinaryOperator::LessEqualThan
                            | BinaryOperator::GreaterThan
                            | BinaryOperator::GreaterEqualThan
                            | BinaryOperator::In
                            | BinaryOperator::Instanceof
                    ) =>
                {
                    values.insert(id, BTreeSet::from([Origin::Boolean]));
                    None
                }
                AstKind::AwaitExpression(awaited) => {
                    Some(Rule::AwaitPrimitive(awaited.argument.node_id()))
                }
                AstKind::UpdateExpression(_) => {
                    values.insert(id, BTreeSet::from([Origin::Number]));
                    None
                }
                AstKind::AssignmentExpression(assignment)
                    if assignment.operator == AssignmentOperator::Addition
                        && assignment.left.as_simple_assignment_target().is_some() =>
                {
                    let target = assignment
                        .left
                        .as_simple_assignment_target()
                        .expect("checked simple assignment target");
                    Some(if target_identifier(target, 0).ok().flatten().is_some() {
                        Rule::Addition(target.node_id(), assignment.right.node_id())
                    } else {
                        Rule::PropertyAddition(target.node_id(), assignment.right.node_id())
                    })
                }
                AstKind::AssignmentExpression(assignment)
                    if evaluate::property_update::compound(assignment.operator).is_some() =>
                {
                    values.insert(id, BTreeSet::from([Origin::Number]));
                    None
                }
                AstKind::UnaryExpression(u)
                    if matches!(
                        u.operator,
                        UnaryOperator::UnaryPlus
                            | UnaryOperator::UnaryNegation
                            | UnaryOperator::BitwiseNot
                    ) =>
                {
                    values.insert(id, BTreeSet::from([Origin::Number]));
                    None
                }
                AstKind::NullLiteral(_) => {
                    values.insert(id, BTreeSet::from([Origin::Null]));
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
                    Some(Rule::Addition(b.left.node_id(), b.right.node_id()))
                }
                AstKind::BinaryExpression(b)
                    if matches!(
                        b.operator,
                        BinaryOperator::Subtraction
                            | BinaryOperator::Multiplication
                            | BinaryOperator::Division
                            | BinaryOperator::Remainder
                            | BinaryOperator::BitwiseAnd
                            | BinaryOperator::BitwiseOR
                            | BinaryOperator::BitwiseXOR
                            | BinaryOperator::ShiftLeft
                            | BinaryOperator::ShiftRight
                            | BinaryOperator::ShiftRightZeroFill
                    ) =>
                {
                    values.insert(id, BTreeSet::from([Origin::Number]));
                    None
                }
                AstKind::ObjectExpression(object) => {
                    let mut own = BTreeMap::new();
                    let mut dictionary = false;
                    let mut possible = BTreeMap::<Key, Vec<NodeId>>::new();
                    for property in &object.properties {
                        if let ObjectPropertyKind::ObjectProperty(property) = property {
                            if object_create::prototype_setter(property) {
                                dictionary = object_create::null_prototype_value(
                                    &property.value,
                                    semantic,
                                    0,
                                );
                                continue;
                            }
                            let value = property.value.node_id();
                            let mut exact =
                                static_keys::property(&property.key, property.computed, 0);
                            if exact.is_none()
                                && property.computed
                                && let Some(key) = property.key.as_expression()
                                && let record_names::Domain::Fields(mut names) =
                                    record_names.domain(key, 0)
                            {
                                if names.len() == 1 {
                                    exact = names.pop_first();
                                } else {
                                    for name in names {
                                        possible.entry(name).or_default().push(value);
                                    }
                                    continue;
                                }
                            }
                            if let Some(name) = exact {
                                // A definite later definition replaces every earlier candidate.
                                possible.remove(&name);
                                own.insert(name, Some(value));
                                continue;
                            }
                        }
                        // A spread or unbounded key may replace any earlier field.
                        open_fields.insert(id);
                        possible.clear();
                        for value in own.values_mut() {
                            *value = None;
                        }
                    }
                    for (name, candidates) in possible {
                        writes.extend(
                            candidates
                                .into_iter()
                                .map(|value| (id, name.clone(), value)),
                        );
                    }
                    fields.insert(id, own);
                    values.insert(
                        id,
                        BTreeSet::from([if dictionary {
                            Origin::NullObject(id)
                        } else {
                            Origin::Object(id)
                        }]),
                    );
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
                    if let Some(elements) = iteration::literal_elements(array) {
                        fields.insert(
                            id,
                            elements
                                .iter()
                                .enumerate()
                                .map(|(index, element)| {
                                    let value = match element {
                                        iteration::LiteralElement::Value(value, _) => {
                                            Some(value.node_id())
                                        }
                                        _ => None,
                                    };
                                    (index.to_string().encode_utf16().collect(), value)
                                })
                                .collect(),
                        );
                    } else {
                        // Unknown spread width cannot prove any fixed position.
                        open_fields.insert(id);
                    }
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
                            defaults: arrow
                                .params
                                .items
                                .iter()
                                .map(|p| {
                                    p.initializer.as_ref().map(|value| value.as_ref().node_id())
                                })
                                .collect(),
                            returns,
                            implicit: complete.is_none(),
                            undefined_return: !arrow.r#async
                                && block_results::return_statements(semantic, id)
                                    .iter()
                                    .any(|statement| statement.argument.is_none()),
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
                        let undefined_return = !function.r#async
                            && (returns.is_empty()
                                || block_results::normal_completion(function)
                                || block_results::return_statements(semantic, id)
                                    .iter()
                                    .any(|statement| statement.argument.is_none()));
                        let implicit = returns.is_empty() && !undefined_return;
                        functions.insert(
                            Origin::Named(name),
                            FunctionFlow {
                                parameters: parameters(&function.params),
                                defaults: function
                                    .params
                                    .items
                                    .iter()
                                    .map(|p| {
                                        p.initializer.as_ref().map(|value| value.as_ref().node_id())
                                    })
                                    .collect(),
                                returns,
                                implicit,
                                undefined_return,
                            },
                        );
                        if name == entry {
                            for parameter in &function.params.items {
                                if let BindingPattern::BindingIdentifier(binding) =
                                    &parameter.pattern
                                    && let Some(symbol) = binding.symbol_id.get()
                                {
                                    // Public entry inputs are Numbers in both source profiles.
                                    bindings.insert(symbol, BTreeSet::from([Origin::Number]));
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
                        } else if !var_symbols.contains(&symbol)
                            && !lexical_iteration_symbols.contains(&symbol)
                        {
                            bindings
                                .entry(symbol)
                                .or_default()
                                .insert(Origin::Undefined);
                        }
                    }
                    if let Some(initializer) = &declaration.init {
                        pattern_definitions(
                            &record_names,
                            &uint32_keys,
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
                        Some(
                            var_reads
                                .get(&id)
                                .map_or(Rule::Symbol(symbol), |reads| Rule::Copy(reads.clone())),
                        )
                    } else {
                        let origin = match identifier.name.as_str() {
                            "undefined" => Origin::Undefined,
                            "Infinity" | "NaN" => Origin::Number,
                            _ => Origin::Other,
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
                AstKind::LogicalExpression(c) => Some(Rule::Logical(
                    c.operator,
                    c.left.node_id(),
                    c.right.node_id(),
                )),
                AstKind::SequenceExpression(s) => Some(Rule::Copy(
                    s.expressions
                        .last()
                        .map(GetNodeId::node_id)
                        .into_iter()
                        .collect(),
                )),
                AstKind::CallExpression(call)
                    if let Some(kind) = object_intrinsics::conversion_call(call, semantic, 0) =>
                {
                    Some(Rule::Known(match kind {
                        object_intrinsics::ConversionCall::Number => Origin::Number,
                        object_intrinsics::ConversionCall::String => Origin::String,
                        object_intrinsics::ConversionCall::Boolean => Origin::Boolean,
                    }))
                }
                AstKind::CallExpression(call) => Some(Rule::Call(
                    invocation::callee(call).node_id(),
                    invocation::arguments(call)
                        .iter()
                        .filter_map(|argument| argument.as_expression().map(GetNodeId::node_id))
                        .collect(),
                    string_methods::call(call)
                        .filter(|(_, method)| {
                            method.returns_string() || matches!(method, string_methods::Method::At)
                        })
                        .map(|(receiver, method)| (receiver.node_id(), method)),
                    array_mutations::operation(call)
                        .filter(|(op, _)| {
                            matches!(
                                op,
                                array_mutations::Operation::Reverse
                                    | array_mutations::Operation::ToReversed
                            )
                        })
                        .map(|(op, r)| (r.node_id(), op == array_mutations::Operation::ToReversed)),
                )),
                AstKind::AssignmentExpression(a)
                    if matches!(
                        a.operator,
                        AssignmentOperator::LogicalAnd
                            | AssignmentOperator::LogicalOr
                            | AssignmentOperator::LogicalNullish
                    ) =>
                {
                    // Keep every possible write in the binding's storage facts.
                    // The assignment expression yields only the selected operand.
                    if let Some(name) = a
                        .left
                        .as_simple_assignment_target()
                        .and_then(|target| target_identifier(target, 0).ok().flatten())
                        && let Some(target) = symbol(name, semantic)
                    {
                        definitions.push(Definition::Value(target, a.right.node_id()));
                    }
                    if let Some(target) = a.left.as_simple_assignment_target() {
                        Some(Rule::Logical(
                            a.operator
                                .to_logical_operator()
                                .expect("logical assignment"),
                            target.node_id(),
                            a.right.node_id(),
                        ))
                    } else {
                        values.insert(id, BTreeSet::from([Origin::Other]));
                        None
                    }
                }
                AstKind::AssignmentExpression(a) if a.operator == AssignmentOperator::Assign => {
                    if let oxc_ast::ast::AssignmentTarget::ArrayAssignmentTarget(pattern) = &a.left
                    {
                        for (index, element) in pattern.elements.iter().enumerate() {
                            if let Some(element) = element {
                                let (target, default) =
                                    assignment_patterns::target_and_default(element);
                                if let Some(identifier) = target
                                    .as_simple_assignment_target()
                                    .and_then(|t| target_identifier(t, 0).ok().flatten())
                                {
                                    assignment_definition(
                                        identifier,
                                        Rule::Member(
                                            a.right.node_id(),
                                            index.to_string().encode_utf16().collect(),
                                        ),
                                        element.node_id(),
                                        default,
                                        semantic,
                                        &mut definitions,
                                        &mut pattern_rules,
                                    );
                                } else if let Some((receiver, key)) =
                                    assignment_patterns::property_target(target, 0)
                                {
                                    let id = target.node_id();
                                    let selection = Rule::Member(
                                        a.right.node_id(),
                                        index.to_string().encode_utf16().collect(),
                                    );
                                    if let Some(default) = default {
                                        pattern_rules.push((element.node_id(), selection));
                                        pattern_rules.push((
                                            id,
                                            Rule::Default(element.node_id(), default.node_id()),
                                        ));
                                    } else {
                                        pattern_rules.push((id, selection));
                                    }
                                    mutations::write(
                                        receiver.node_id(),
                                        key,
                                        id,
                                        &record_names,
                                        &mut writes,
                                        &mut mutations,
                                    );
                                }
                            }
                        }
                        if let Some(rest) = &pattern.rest {
                            elements.add_rest(
                                rest.node_id(),
                                a.right.node_id(),
                                pattern.elements.len() as u32,
                            );
                            if let Some(identifier) = rest
                                .target
                                .as_simple_assignment_target()
                                .and_then(|t| target_identifier(t, 0).ok().flatten())
                                && let Some(symbol) = symbol(identifier, semantic)
                            {
                                pattern_rules.push((
                                    identifier.node_id(),
                                    Rule::Known(Origin::Array(rest.node_id())),
                                ));
                                definitions.push(Definition::Value(symbol, identifier.node_id()));
                            } else if let Some((receiver, key)) =
                                assignment_patterns::property_target(&rest.target, 0)
                            {
                                let id = rest.target.node_id();
                                pattern_rules
                                    .push((id, Rule::Known(Origin::Array(rest.node_id()))));
                                mutations::write(
                                    receiver.node_id(),
                                    key,
                                    id,
                                    &record_names,
                                    &mut writes,
                                    &mut mutations,
                                );
                            }
                        }
                    }
                    if let oxc_ast::ast::AssignmentTarget::ObjectAssignmentTarget(pattern) = &a.left
                    {
                        let mut excluded = BTreeSet::new();
                        let mut dynamic_excluded = false;
                        for property in &pattern.properties {
                            use oxc_ast::ast::AssignmentTargetProperty as Property;
                            let (identifier, member, key, default) = match property {
                                Property::AssignmentTargetPropertyIdentifier(p) => (
                                    Some(&p.binding),
                                    None,
                                    Some(iteration::PropertyKey::Literal(
                                        p.binding.name.encode_utf16().collect(),
                                    )),
                                    p.init.as_ref(),
                                ),
                                Property::AssignmentTargetPropertyProperty(p) => {
                                    let (target, default) =
                                        assignment_patterns::target_and_default(&p.binding);
                                    (
                                        target
                                            .as_simple_assignment_target()
                                            .and_then(|t| target_identifier(t, 0).ok().flatten()),
                                        assignment_patterns::property_target(target, 0).map(
                                            |(receiver, key)| (receiver, key, target.node_id()),
                                        ),
                                        static_keys::property(&p.name, p.computed, 0)
                                            .map(iteration::PropertyKey::Literal)
                                            .or_else(|| {
                                                p.name
                                                    .as_expression()
                                                    .map(iteration::PropertyKey::Runtime)
                                            }),
                                        default,
                                    )
                                }
                            };
                            if let Some(key) = key {
                                let selection = match key {
                                    iteration::PropertyKey::Literal(key) => {
                                        excluded.insert(key.clone());
                                        Rule::Member(a.right.node_id(), key)
                                    }
                                    iteration::PropertyKey::Runtime(key)
                                        if uint32_keys.contains(key, 0) =>
                                    {
                                        dynamic_excluded = true;
                                        Rule::PatternUnsignedMember(a.right.node_id())
                                    }
                                    iteration::PropertyKey::Runtime(key) => {
                                        dynamic_excluded = true;
                                        Rule::StringKeyMember {
                                            receiver: a.right.node_id(),
                                            key: key.node_id(),
                                            domain: record_names.domain(key, 0),
                                        }
                                    }
                                };
                                if let Some(identifier) = identifier {
                                    assignment_definition(
                                        identifier,
                                        selection,
                                        property.node_id(),
                                        default,
                                        semantic,
                                        &mut definitions,
                                        &mut pattern_rules,
                                    );
                                } else if let Some((receiver, member_key, id)) = member {
                                    if let Some(default) = default {
                                        pattern_rules.push((property.node_id(), selection));
                                        pattern_rules.push((
                                            id,
                                            Rule::Default(property.node_id(), default.node_id()),
                                        ));
                                    } else {
                                        pattern_rules.push((id, selection));
                                    }
                                    mutations::write(
                                        receiver.node_id(),
                                        member_key,
                                        id,
                                        &record_names,
                                        &mut writes,
                                        &mut mutations,
                                    );
                                }
                            }
                        }
                        if let Some(rest) = &pattern.rest {
                            rest_sources.insert(
                                rest.node_id(),
                                member_origins::Rest {
                                    receiver: a.right.node_id(),
                                    excluded,
                                    dynamic_excluded,
                                },
                            );
                            if let Some(identifier) = rest
                                .target
                                .as_simple_assignment_target()
                                .and_then(|t| target_identifier(t, 0).ok().flatten())
                                && let Some(symbol) = symbol(identifier, semantic)
                            {
                                pattern_rules.push((
                                    identifier.node_id(),
                                    Rule::Known(Origin::Object(rest.node_id())),
                                ));
                                definitions.push(Definition::Value(symbol, identifier.node_id()));
                            } else if let Some((receiver, key)) =
                                assignment_patterns::property_target(&rest.target, 0)
                            {
                                let id = rest.target.node_id();
                                pattern_rules
                                    .push((id, Rule::Known(Origin::Object(rest.node_id()))));
                                mutations::write(
                                    receiver.node_id(),
                                    key,
                                    id,
                                    &record_names,
                                    &mut writes,
                                    &mut mutations,
                                );
                            }
                        }
                    }
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
                                mutations::write(
                                    m.object.node_id(),
                                    iteration::PropertyKey::Runtime(&m.expression),
                                    a.right.node_id(),
                                    &record_names,
                                    &mut writes,
                                    &mut mutations,
                                );
                            }
                            _ => (),
                        }
                    }
                    Some(Rule::Copy(vec![a.right.node_id()]))
                }
                AstKind::ChainExpression(chain) => Some(
                    if chain
                        .expression
                        .as_member_expression()
                        .is_some_and(|m| m.optional())
                    {
                        Rule::Copy(vec![chain.expression.node_id()])
                    } else {
                        Rule::Known(Origin::Other)
                    },
                ),
                AstKind::StaticMemberExpression(m) => {
                    let key = m.property.name.encode_utf16().collect();
                    Some(if m.optional {
                        Rule::OptionalMember(m.object.node_id(), key)
                    } else {
                        Rule::Member(m.object.node_id(), key)
                    })
                }
                AstKind::ComputedMemberExpression(m) => {
                    use evaluate::optional_member::{Key, computed_key};
                    let receiver = m.object.node_id();
                    Some(match computed_key(&m.expression, 0, &uint32_keys) {
                        Some(Key::Literal(key)) if m.optional => {
                            Rule::OptionalMember(receiver, key)
                        }
                        Some(Key::Literal(key)) => Rule::Member(receiver, key),
                        Some(Key::Uint32(_)) if m.optional => {
                            Rule::OptionalUnsignedMember(receiver)
                        }
                        Some(Key::Uint32(_)) => Rule::UnsignedMember(receiver),
                        Some(Key::Runtime(key)) => {
                            let domain = record_names.domain(key, 0);
                            let key = key.node_id();
                            if m.optional {
                                Rule::OptionalComputedMember {
                                    receiver,
                                    key,
                                    domain,
                                }
                            } else {
                                Rule::StringKeyMember {
                                    receiver,
                                    key,
                                    domain,
                                }
                            }
                        }
                        None => Rule::Known(Origin::Other),
                    })
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
        // Member heads are assignment references, never reads of their old
        // value. Their selected iteration origins feed the ordinary alias writes.
        let member_heads = member_iteration_rules
            .iter()
            .map(|(id, _)| *id)
            .collect::<BTreeSet<_>>();
        rules.retain(|(id, _)| !member_heads.contains(id));
        for (id, rule) in member_iteration_rules {
            values.remove(&id);
            rules.push((id, rule));
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
        let has_relative_selection = rules.iter().any(|(_, rule)| {
            matches!(
                rule,
                Rule::Call(_, _, Some((_, string_methods::Method::At)), _)
            )
        });
        let has_logical_selection = rules
            .iter()
            .any(|(_, rule)| matches!(rule, Rule::Logical(..)));
        let has_dictionary_selection = rules.iter().any(|(_, rule)| {
            matches!(
                rule,
                Rule::StringKeyMember { .. }
                    | Rule::UnsignedMember(..)
                    | Rule::PatternUnsignedMember(..)
                    | Rule::OptionalMember(..)
                    | Rule::OptionalUnsignedMember(..)
                    | Rule::OptionalComputedMember { .. }
            )
        });
        let track_elements = has_iterations
            || has_relative_selection
            || !elements.rest_sources.is_empty()
            || !elements.entry_lists.is_empty()
            || !elements.key_lists.is_empty()
            || !enumeration_inputs.is_empty()
            || !indexed_copy_inputs.is_empty();
        let mut finalized_unresolved = false;
        let mut numeric_writes_classified = false;
        let mut number_updates = BTreeSet::new();
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
                    Definition::Symbol(target, source) => {
                        (*target, bindings.get(source).cloned().unwrap_or_default())
                    }
                    Definition::Value(symbol, id) => (*symbol, get(id)),
                    Definition::Iterated(symbol, receiver) => (
                        *symbol,
                        elements.select(*receiver, &values, &fields, &index_deletes),
                    ),
                    Definition::ForIn(symbol) => (*symbol, BTreeSet::from([Origin::String])),
                })
                .collect::<Vec<_>>();
            // A fallback is also a possible input of an unused declaration.
            // Seed it conservatively; actuals and body writes still union their
            // origins, so it cannot hide a nonnumeric supplied alternative.
            for function in functions.values() {
                for (index, symbol) in function.parameters.iter().enumerate() {
                    if let Some(Some(default)) = function.defaults.get(index) {
                        symbol_additions.push((*symbol, get(default)));
                    }
                }
            }
            let mut additions = Vec::new();
            for (id, rule) in &rules {
                let found = match rule {
                    Rule::Iterated(receiver) => {
                        elements.select(*receiver, &values, &fields, &index_deletes)
                    }
                    Rule::Known(origin) => BTreeSet::from([*origin]),
                    Rule::Copy(inputs) => inputs.iter().flat_map(get).collect(),
                    Rule::AwaitPrimitive(input) => get(input)
                        .into_iter()
                        .map(|origin| {
                            // Known primitive kinds have no thenable assimilation.
                            // Do not retain reference identity or infer a value
                            // kind from unknown Promise/thenable fulfillments.
                            if matches!(
                                origin,
                                Origin::Number
                                    | Origin::String
                                    | Origin::Boolean
                                    | Origin::Null
                                    | Origin::Undefined
                            ) {
                                origin
                            } else {
                                Origin::Other
                            }
                        })
                        .collect(),
                    Rule::Logical(operator, left, right) => {
                        let mut found = Alternatives::new();
                        let mut selects_right = false;
                        for origin in get(left) {
                            let nullish = matches!(origin, Origin::Null | Origin::Undefined);
                            let truthy = matches!(
                                origin,
                                Origin::Object(_)
                                    | Origin::NullObject(_)
                                    | Origin::Array(_)
                                    | Origin::Arrow(_)
                                    | Origin::Named(_)
                            );
                            let (keep_left, use_right) = match operator {
                                LogicalOperator::Coalesce => {
                                    (!nullish, nullish || origin == Origin::Other)
                                }
                                LogicalOperator::Or => (!nullish, !truthy),
                                LogicalOperator::And => (!truthy, !nullish),
                            };
                            if keep_left {
                                found.insert(origin);
                            }
                            selects_right |= use_right;
                        }
                        if selects_right {
                            // Strings may be empty and Other may have either
                            // truthiness. An unresolved empty set proves neither.
                            found.extend(get(right));
                        }
                        found
                    }
                    Rule::Default(selected, fallback) => {
                        // Only positively known Undefined is replaced. Null
                        // and Other, including unknown receivers, remain.
                        // Keep the fallback union conservative even when the
                        // selected alternatives do not currently need it.
                        let mut found = get(selected);
                        found.remove(&Origin::Undefined);
                        found.extend(get(fallback));
                        found
                    }
                    Rule::Symbol(symbol) => bindings.get(symbol).cloned().unwrap_or_default(),
                    Rule::Addition(left, right) | Rule::PropertyAddition(left, right) => {
                        let left = get(left);
                        let right = get(right);
                        if left.is_empty() || right.is_empty() {
                            Alternatives::new()
                        } else if (left.iter().all(|v| *v == Origin::String)
                            || right.iter().all(|v| *v == Origin::String))
                            && left.iter().chain(&right).all(|v| {
                                matches!(
                                    v,
                                    Origin::String
                                        | Origin::Number
                                        | Origin::Boolean
                                        | Origin::Null
                                        | Origin::Undefined
                                )
                            })
                        {
                            BTreeSet::from([Origin::String])
                        } else if left
                            .iter()
                            .chain(&right)
                            .all(|origin| *origin == Origin::Number)
                            || (matches!(rule, Rule::PropertyAddition(_, _))
                                && !right.contains(&Origin::String)
                                && left.iter().all(|origin| {
                                    matches!(
                                        origin,
                                        Origin::Number
                                            | Origin::Boolean
                                            | Origin::Null
                                            | Origin::Undefined
                                    )
                                }))
                        {
                            // The numeric property path independently checks its RHS in
                            // Arithmetic mode. An abrupt RHS (e.g. a rejected await)
                            // never stores, and cannot invalidate the old field's kind.
                            BTreeSet::from([Origin::Number])
                        } else {
                            BTreeSet::from([Origin::Other])
                        }
                    }
                    Rule::Call(callee, arguments, string_receiver, array_receiver) => {
                        if let Some((receiver, fresh)) = array_receiver {
                            let origins = get(receiver);
                            if !origins.is_empty()
                                && origins
                                    .iter()
                                    .all(|origin| matches!(origin, Origin::Array(_)))
                            {
                                additions.push((
                                    *id,
                                    if *fresh {
                                        BTreeSet::from([Origin::Array(*id)])
                                    } else {
                                        origins
                                    },
                                ));
                                continue;
                            }
                        }
                        if let Some((receiver, method)) = string_receiver {
                            let origins = get(receiver);
                            if !origins.is_empty()
                                && origins
                                    .iter()
                                    .all(|origin| matches!(origin, Origin::Array(_)))
                            {
                                if matches!(method, string_methods::Method::Slice) {
                                    additions.push((*id, BTreeSet::from([Origin::Array(*id)])));
                                    continue;
                                }
                                if matches!(method, string_methods::Method::At) {
                                    let mut result = elements.select(
                                        *receiver,
                                        &values,
                                        &fields,
                                        &index_deletes,
                                    );
                                    // Keep all possible element origins and always
                                    // allow a miss, even for a literal index. Only a
                                    // binding default can replace this alternative.
                                    result.insert(Origin::Undefined);
                                    additions.push((*id, result));
                                    continue;
                                }
                            }
                            if !origins.is_empty()
                                && origins.iter().all(|origin| *origin == Origin::String)
                            {
                                let mut result = BTreeSet::from([Origin::String]);
                                if matches!(method, string_methods::Method::At) {
                                    // No index/length proof: every relative selection
                                    // may miss. A binding default can replace only
                                    // this Undefined alternative, never other kinds.
                                    result.insert(Origin::Undefined);
                                }
                                additions.push((*id, result));
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
                                    let mut incoming = arguments
                                        .get(index)
                                        .map(get)
                                        .unwrap_or_else(|| BTreeSet::from([Origin::Undefined]));
                                    if let Some(Some(default)) = function.defaults.get(index)
                                        && incoming.remove(&Origin::Undefined)
                                    {
                                        incoming.extend(get(default));
                                    }
                                    symbol_additions.push((*symbol, incoming));
                                }
                                for expression in &function.returns {
                                    found.extend(get(expression));
                                }
                                if function.undefined_return {
                                    found.insert(Origin::Undefined);
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
                    Rule::OptionalMember(receiver, key) => members.optional_select(*receiver, key),
                    Rule::OptionalComputedMember {
                        receiver,
                        key,
                        domain,
                    } => members.optional_computed(*receiver, *key, domain),
                    Rule::OptionalUnsignedMember(receiver) => {
                        members.optional_unsigned_value(*receiver)
                    }
                    Rule::UnsignedMember(receiver) => members.unsigned_value(*receiver),
                    Rule::PatternUnsignedMember(receiver) => {
                        members.pattern_unsigned_value(*receiver)
                    }
                    Rule::StringKeyMember {
                        receiver,
                        key,
                        domain,
                    } => members.string_key_value(*receiver, *key, domain),
                };
                additions.push((*id, found));
            }
            let snapshots = enumeration_inputs
                .iter()
                .map(|&(site, receiver)| (site, members.enumerable_values(receiver)))
                .chain(indexed_copy_inputs.iter().map(|&(site, receiver)| {
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
                    mutations::Mutation::KeyDelete { receiver, keys } => {
                        for key in keys {
                            for origin in get(receiver) {
                                if let Origin::Object(object)
                                | Origin::NullObject(object)
                                | Origin::Array(object) = origin
                                {
                                    let undefined = BTreeSet::from([Origin::Undefined]);
                                    write_additions
                                        .push(((object, key.clone()), undefined.clone()));
                                    if matches!(origin, Origin::Array(_))
                                        && mutations::array_index(key).is_some()
                                    {
                                        deleted.push(object);
                                        if track_elements {
                                            changed |=
                                                elements.write(object, key, undefined, &fields);
                                        }
                                    }
                                }
                            }
                        }
                    }
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
                    mutations::Mutation::AnyKeyDelete(receiver) => {
                        for origin in get(receiver) {
                            if let Origin::Object(object)
                            | Origin::NullObject(object)
                            | Origin::Array(object) = origin
                            {
                                let undefined = BTreeSet::from([Origin::Undefined]);
                                wildcard_additions.push((object, undefined.clone()));
                                if matches!(origin, Origin::Array(_)) {
                                    deleted.push(object);
                                    if track_elements {
                                        changed |= elements.wildcard(object, undefined);
                                    }
                                }
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
                    || has_relative_selection
                    || has_logical_selection
                    || has_dictionary_selection
                    || !elements.rest_sources.is_empty()
                    || !indexed_copy_inputs.is_empty())
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
            if !changed && !numeric_writes_classified {
                numeric_writes_classified = true;
                let symbols = numeric_targets
                    .iter()
                    .map(|(_, _, symbol)| *symbol)
                    .collect::<BTreeSet<_>>();
                for symbol in symbols {
                    let binding_is_number = bindings.get(&symbol).is_some_and(|origins| {
                        origins.len() == 1 && origins.contains(&Origin::Number)
                    });
                    let targets_are_number = numeric_targets
                        .iter()
                        .filter(|(_, _, target_symbol)| *target_symbol == symbol)
                        .all(|(_, result, _)| {
                            values.get(result).is_some_and(|origins| {
                                origins.len() == 1 && origins.contains(&Origin::Number)
                            })
                        });
                    // Same-kind String/Boolean writes preserve their lane.
                    // Numeric updates or an incompatible RHS still widen it.
                    let exact_string = |origins: &Alternatives| {
                        origins.len() == 1 && origins.contains(&Origin::String)
                    };
                    let nullable_string = |origins: &Alternatives| {
                        !origins.is_empty()
                            && origins.iter().all(|origin| {
                                matches!(origin, Origin::Null | Origin::Undefined | Origin::String)
                            })
                    };
                    let preserves_string = bindings.get(&symbol).is_some_and(|origins| nullable_string(origins) && origins.contains(&Origin::String)) && numeric_targets
                        .iter()
                        .filter(|(_, _, target_symbol)| *target_symbol == symbol)
                        .all(|(_, result, _)| {
                            // Logical assignment stores only a selected original operand.
                            // Arithmetic string updates still require an exact String input.
                            if matches!(semantic.nodes().kind(*result), AstKind::AssignmentExpression(a) if a.operator.is_logical()) {
                                values.get(result).is_some_and(nullable_string)
                            } else {
                                bindings.get(&symbol).is_some_and(exact_string) && values.get(result).is_some_and(exact_string)
                            }
                        });
                    let preserves_boolean = bindings.get(&symbol).is_some_and(|origins| {
                        origins.len() == 1 && origins.contains(&Origin::Boolean)
                    }) && numeric_targets
                        .iter()
                        .filter(|(_, _, target_symbol)| *target_symbol == symbol)
                        .all(|(_, result, _)| {
                            values.get(result).is_some_and(|origins| {
                                origins.len() == 1 && origins.contains(&Origin::Boolean)
                            })
                        });
                    // Logical writes can replace a function without applying
                    // numeric coercion. Keep every constructor alternative.
                    let preserves_functions = bindings.get(&symbol).is_some_and(|origins| {
                        !origins.is_empty()
                            && origins
                                .iter()
                                .all(|origin| matches!(origin, Origin::Arrow(_) | Origin::Named(_)))
                    }) && numeric_targets
                        .iter()
                        .filter(|(_, _, target)| *target == symbol)
                        .all(|(_, result, _)| {
                            values.get(result).is_some_and(|origins| {
                                !origins.is_empty()
                                    && origins.iter().all(|origin| {
                                        matches!(origin, Origin::Arrow(_) | Origin::Named(_))
                                    })
                            })
                        });
                    let primitive_var = var_symbols.contains(&symbol)
                        && bindings.get(&symbol).is_some_and(|origins| {
                            !origins.is_empty()
                                && origins.iter().all(|origin| {
                                    matches!(
                                        origin,
                                        Origin::Number
                                            | Origin::Boolean
                                            | Origin::Null
                                            | Origin::Undefined
                                    )
                                })
                        });
                    if (binding_is_number || primitive_var) && targets_are_number {
                        if primitive_var {
                            changed |=
                                extend(&mut bindings, symbol, BTreeSet::from([Origin::Number]));
                        }
                        number_updates.extend(
                            numeric_targets
                                .iter()
                                .filter(|(_, _, target_symbol)| *target_symbol == symbol)
                                .map(|(target, _, _)| *target),
                        );
                    } else if !preserves_string && !preserves_boolean && !preserves_functions {
                        changed |= extend(&mut bindings, symbol, BTreeSet::from([Origin::Other]));
                    }
                }
            }
            if !changed {
                break;
            }
        }
        Self {
            record_names,
            uint32_keys,
            values,
            number_updates,
            var_symbols,
        }
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
