//! Binding plans borrow the original source and select each own field or array
//! position in order. Origins prove receiver kinds; annotations never do.
use super::*;
use oxc_ast::ast::{ArrayPattern, BindingIdentifier, BindingProperty, ObjectPattern, PropertyKey};
use oxc_syntax::node::NodeId;

/// Original-source pattern leaves for representation planning. Plain identifier
/// declarations have their own planners. This walk does not admit a pattern.
pub(super) fn leaves<'n, 'a>(
    pattern: &'n BindingPattern<'a>,
) -> impl Iterator<Item = &'n BindingIdentifier<'a>> {
    let mut pending = if matches!(pattern, BindingPattern::BindingIdentifier(_)) {
        Vec::new()
    } else {
        vec![pattern]
    };
    std::iter::from_fn(move || {
        while let Some(pattern) = pending.pop() {
            match pattern {
                BindingPattern::BindingIdentifier(identifier) => return Some(identifier.as_ref()),
                BindingPattern::AssignmentPattern(assignment) => pending.push(&assignment.left),
                BindingPattern::ObjectPattern(object) => {
                    pending.extend(
                        object
                            .properties
                            .iter()
                            .rev()
                            .map(|property| &property.value),
                    );
                }
                BindingPattern::ArrayPattern(array) => {
                    pending.extend(array.elements.iter().rev().flatten());
                }
            }
        }
        None
    })
}

fn leaf<'n, 'a>(
    pattern: &'n BindingPattern<'a>,
) -> Option<(&'n BindingIdentifier<'a>, Option<&'n Expression<'a>>)> {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => Some((identifier, None)),
        BindingPattern::AssignmentPattern(assignment) => match &assignment.left {
            BindingPattern::BindingIdentifier(identifier) => {
                Some((identifier, Some(&assignment.right)))
            }
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn key(property: &BindingProperty<'_>) -> Option<Vec<u16>> {
    match &property.key {
        PropertyKey::StaticIdentifier(identifier) if !property.computed => {
            Some(identifier.name.encode_utf16().collect())
        }
        PropertyKey::StringLiteral(literal) => Some(literal.value.encode_utf16().collect()),
        _ => None,
    }
}

pub(super) enum BindingStep<'n, 'a> {
    Object(&'n ObjectPattern<'a>),
    Array {
        source: &'n ArrayPattern<'a>,
        live: bool,
    },
    Field {
        source: &'n BindingProperty<'a>,
        key: Vec<u16>,
    },
    Element {
        source: &'n BindingPattern<'a>,
        index: u32,
    },
    Bind {
        identifier: &'n BindingIdentifier<'a>,
        default: Option<&'n Expression<'a>>,
    },
    Leave,
}

pub(super) struct BindingPlan<'n, 'a> {
    pub(super) steps: std::vec::IntoIter<BindingStep<'n, 'a>>,
    pub(super) mutable: bool,
}

pub(super) struct PatternBinding<'n, 'a> {
    pub(super) source: &'n Expression<'a>,
    pub(super) plan: BindingPlan<'n, 'a>,
}

impl<'s, 'ast> Lowering<'s, 'ast> {
    pub(super) fn prepare_array_binding<'n, 'a>(
        &mut self,
        local: &'n VariableDeclarator<'a>,
        mutable: bool,
    ) -> Result<PatternBinding<'n, 'a>, String> {
        let source = local
            .init
            .as_ref()
            .ok_or_else(|| unsupported(local, "array binding patterns require an initializer"))?;
        let ty = self.managed_source_type(source, 0);
        if ty != Some(Type::ManagedArray)
            && !(ty == Some(Type::ManagedValue)
                && self.stored.container(source.node_id()) == Some(Type::ManagedArray))
        {
            return Err(unsupported(
                source,
                "array bindings require a source-proven managed array",
            ));
        }
        let plan =
            self.prepare_binding_pattern(&local.id, source.node_id(), Type::ManagedArray, mutable)?;
        Ok(PatternBinding { source, plan })
    }

    pub(super) fn prepare_object_binding<'n, 'a>(
        &mut self,
        local: &'n VariableDeclarator<'a>,
        mutable: bool,
    ) -> Result<PatternBinding<'n, 'a>, String> {
        let source = local
            .init
            .as_ref()
            .ok_or_else(|| unsupported(local, "object binding patterns require an initializer"))?;
        let ty = self.managed_source_type(source, 0);
        if ty != Some(Type::ManagedObject)
            && !(ty == Some(Type::ManagedValue)
                && self.stored.container(source.node_id()) == Some(Type::ManagedObject))
        {
            return Err(unsupported(
                source,
                "object bindings require a source-proven managed object",
            ));
        }
        let plan = self.prepare_binding_pattern(
            &local.id,
            source.node_id(),
            Type::ManagedObject,
            mutable,
        )?;
        Ok(PatternBinding { source, plan })
    }

    /// Prepare a pattern independently of evaluating its original source. The
    /// caller proves the root kind; every nested selection has its own source
    /// proof. The resulting plan can consume an already evaluated value.
    pub(super) fn prepare_binding_pattern<'n, 'a>(
        &mut self,
        pattern: &'n BindingPattern<'a>,
        origin: NodeId,
        root_type: Type,
        mutable: bool,
    ) -> Result<BindingPlan<'n, 'a>, String> {
        enum Work<'n, 'a> {
            Pattern(&'n BindingPattern<'a>, NodeId, usize),
            Field(&'n BindingProperty<'a>, bool, usize),
            Element(&'n BindingPattern<'a>, u32, usize),
            Leave,
        }
        let mut pending = vec![Work::Pattern(pattern, origin, 0)];
        let mut steps = Vec::new();
        let mut containers = Vec::new();
        let mut count = 0;
        while let Some(work) = pending.pop() {
            match work {
                Work::Pattern(pattern, origin, parent_depth) => {
                    if let Some((identifier, default)) = leaf(pattern) {
                        count += 1;
                        if default.is_some() {
                            // A descendant's default can mutate any ancestor
                            // array before its next step. Each ancestor keeps
                            // its own independent sticky exhaustion state.
                            for &index in &containers {
                                if let BindingStep::Array { live, .. } = &mut steps[index] {
                                    *live = true;
                                }
                            }
                        }
                        steps.push(BindingStep::Bind {
                            identifier,
                            default,
                        });
                        continue;
                    }
                    let depth = parent_depth + 1;
                    if depth > 128 {
                        return Err(unsupported(pattern, "binding pattern nesting exceeds 128"));
                    }
                    let proven = if parent_depth == 0 {
                        Some(root_type)
                    } else {
                        self.stored.container(origin)
                    };
                    containers.push(steps.len());
                    pending.push(Work::Leave);
                    match pattern {
                        BindingPattern::ObjectPattern(object) => {
                            if object.rest.is_some() {
                                return Err(unsupported(
                                    object.as_ref(),
                                    "object rest bindings are not yet admitted",
                                ));
                            }
                            if proven != Some(Type::ManagedObject) {
                                return Err(unsupported(
                                    object.as_ref(),
                                    "nested object bindings require a source-proven managed object",
                                ));
                            }
                            steps.push(BindingStep::Object(object));
                            let dictionary = self.stored.dictionary(origin);
                            pending.extend(
                                object
                                    .properties
                                    .iter()
                                    .rev()
                                    .map(|property| Work::Field(property, dictionary, depth)),
                            );
                        }
                        BindingPattern::ArrayPattern(array) => {
                            if array.rest.is_some() {
                                return Err(unsupported(
                                    array.as_ref(),
                                    "array rest bindings are not yet admitted",
                                ));
                            }
                            if proven != Some(Type::ManagedArray) {
                                return Err(unsupported(
                                    array.as_ref(),
                                    "nested array bindings require a source-proven managed array",
                                ));
                            }
                            u32::try_from(array.elements.len()).map_err(|_| {
                                unsupported(array.as_ref(), "array binding length exceeds u32")
                            })?;
                            steps.push(BindingStep::Array {
                                source: array,
                                live: false,
                            });
                            pending.extend(array.elements.iter().enumerate().rev().filter_map(
                                |(index, element)| {
                                    element
                                        .as_ref()
                                        .map(|element| Work::Element(element, index as u32, depth))
                                },
                            ));
                        }
                        _ => {
                            return Err(unsupported(
                                pattern,
                                "binding defaults currently require identifier leaves",
                            ));
                        }
                    }
                }
                Work::Field(property, dictionary, depth) => {
                    let key = key(property).ok_or_else(|| {
                        unsupported(
                            property,
                            "object binding keys require an identifier or String literal",
                        )
                    })?;
                    let key = evaluate::managed_member_key(property, key, false, dictionary)?;
                    steps.push(BindingStep::Field {
                        source: property,
                        key,
                    });
                    pending.push(Work::Pattern(
                        &property.value,
                        property.value.node_id(),
                        depth,
                    ));
                }
                Work::Element(source, index, depth) => {
                    steps.push(BindingStep::Element { source, index });
                    pending.push(Work::Pattern(source, source.node_id(), depth));
                }
                Work::Leave => {
                    containers.pop().expect("balanced binding plan");
                    steps.push(BindingStep::Leave);
                }
            }
        }
        self.charge_declaration_bindings(pattern, count)?;
        Ok(BindingPlan {
            steps: steps.into_iter(),
            mutable,
        })
    }
}
