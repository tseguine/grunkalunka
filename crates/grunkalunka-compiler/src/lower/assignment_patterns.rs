//! Source-local assignment targets that need boxed lexical storage.
use super::*;
use oxc_ast::{
    AstKind,
    ast::{AssignmentTarget, AssignmentTargetMaybeDefault, AssignmentTargetProperty},
};
use std::collections::BTreeSet;

pub(super) fn target_and_default<'n, 'a>(
    element: &'n AssignmentTargetMaybeDefault<'a>,
) -> (&'n AssignmentTarget<'a>, Option<&'n Expression<'a>>) {
    match element {
        AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(default) => {
            (&default.binding, Some(&default.init))
        }
        _ => (
            element
                .as_assignment_target()
                .expect("assignment target variant"),
            None,
        ),
    }
}

pub(super) fn identifiers<'n, 'a>(
    root: &'n AssignmentTarget<'a>,
) -> Vec<&'n IdentifierReference<'a>> {
    enum Task<'n, 'a> {
        Target(&'n AssignmentTarget<'a>),
        Element(&'n AssignmentTargetMaybeDefault<'a>),
        Identifier(&'n IdentifierReference<'a>),
    }
    let mut pending = vec![Task::Target(root)];
    let mut found = Vec::new();
    while let Some(task) = pending.pop() {
        match task {
            Task::Identifier(identifier) => found.push(identifier),
            Task::Element(element) => {
                if let AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(default) = element
                {
                    pending.push(Task::Target(&default.binding));
                } else if let Some(target) = element.as_assignment_target() {
                    pending.push(Task::Target(target));
                }
            }
            Task::Target(target) => match target {
                AssignmentTarget::ArrayAssignmentTarget(pattern) => {
                    if let Some(rest) = &pattern.rest {
                        pending.push(Task::Target(&rest.target));
                    }
                    pending.extend(pattern.elements.iter().rev().flatten().map(Task::Element));
                }
                AssignmentTarget::ObjectAssignmentTarget(pattern) => {
                    if let Some(rest) = &pattern.rest {
                        pending.push(Task::Target(&rest.target));
                    }
                    for property in pattern.properties.iter().rev() {
                        match property {
                            AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(p) => {
                                pending.push(Task::Identifier(&p.binding))
                            }
                            AssignmentTargetProperty::AssignmentTargetPropertyProperty(p) => {
                                pending.push(Task::Element(&p.binding))
                            }
                        }
                    }
                }
                _ => {
                    if let Some(identifier) = target
                        .as_simple_assignment_target()
                        .and_then(|t| target_identifier(t, 0).ok().flatten())
                    {
                        found.push(identifier);
                    }
                }
            },
        }
    }
    found
}

pub(super) fn local_symbols(semantic: &Semantic<'_>) -> BTreeSet<SymbolId> {
    let mut locals = BTreeSet::new();
    for node in semantic.nodes().iter() {
        if let AstKind::VariableDeclarator(local) = node.kind() {
            if let BindingPattern::BindingIdentifier(identifier) = &local.id {
                locals.extend(identifier.symbol_id.get());
            }
            locals.extend(binding_patterns::leaves(&local.id).filter_map(|i| i.symbol_id.get()));
        }
    }
    let mut assigned = BTreeSet::new();
    for node in semantic.nodes().iter() {
        if let AstKind::AssignmentExpression(a) = node.kind()
            && a.left.as_assignment_target_pattern().is_some()
        {
            for identifier in identifiers(&a.left) {
                if let Some(symbol) = identifier
                    .reference_id
                    .get()
                    .and_then(|r| semantic.scoping().get_reference(r).symbol_id())
                    && locals.contains(&symbol)
                {
                    assigned.insert(symbol);
                }
            }
        }
    }
    assigned
}

pub(super) fn member_target<'n, 'a>(
    target: &'n AssignmentTarget<'a>,
    depth: usize,
) -> Option<(&'n Expression<'a>, Vec<u16>)> {
    match target.as_simple_assignment_target()? {
        SimpleAssignmentTarget::StaticMemberExpression(m) if !m.optional => {
            Some((&m.object, m.property.name.encode_utf16().collect()))
        }
        SimpleAssignmentTarget::ComputedMemberExpression(m) if !m.optional => {
            let (key, _) = closures::unwrapped_at(&m.expression, depth)?;
            Some((&m.object, static_keys::literal(key)?))
        }
        _ => None,
    }
}
