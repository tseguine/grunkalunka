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

pub(super) enum ObjectBindingStep<'n, 'a> {
    Enter(&'n ObjectPattern<'a>),
    Field {
        source: &'n BindingProperty<'a>,
        key: Vec<u16>,
    },
    Bind {
        identifier: &'n BindingIdentifier<'a>,
        default: Option<&'n Expression<'a>>,
    },
    Leave,
}

pub(super) struct ObjectBinding<'n, 'a> {
    pub(super) source: &'n Expression<'a>,
    pub(super) steps: std::vec::IntoIter<ObjectBindingStep<'n, 'a>>,
    pub(super) mutable: bool,
}

pub(super) struct ElementBinding<'n, 'a> {
    pub(super) identifier: &'n BindingIdentifier<'a>,
    pub(super) index: u32,
    pub(super) default: Option<&'n Expression<'a>>,
}

pub(super) struct ArrayBinding<'n, 'a> {
    pub(super) source: &'n Expression<'a>,
    pub(super) elements: std::vec::IntoIter<ElementBinding<'n, 'a>>,
    pub(super) mutable: bool,
}

impl<'s, 'ast> Lowering<'s, 'ast> {
    pub(super) fn prepare_array_binding<'n, 'a>(
        &mut self,
        local: &'n VariableDeclarator<'a>,
        pattern: &'n ArrayPattern<'a>,
        mutable: bool,
    ) -> Result<ArrayBinding<'n, 'a>, String> {
        if pattern.rest.is_some() {
            return Err(unsupported(
                pattern,
                "array rest bindings are not yet admitted",
            ));
        }
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
        u32::try_from(pattern.elements.len())
            .map_err(|_| unsupported(pattern, "array binding length exceeds u32"))?;
        let count = pattern.elements.iter().flatten().count();
        self.charge_declaration_bindings(pattern, count)?;
        let mut elements = Vec::with_capacity(count);
        for (index, element) in pattern.elements.iter().enumerate() {
            let Some(element) = element else { continue };
            let Some((identifier, default)) = leaf(element) else {
                return Err(unsupported(
                    element,
                    "array bindings currently require identifier leaves",
                ));
            };
            elements.push(ElementBinding {
                identifier,
                index: index as u32,
                default,
            });
        }
        Ok(ArrayBinding {
            source,
            elements: elements.into_iter(),
            mutable,
        })
    }

    pub(super) fn prepare_object_binding<'n, 'a>(
        &mut self,
        local: &'n VariableDeclarator<'a>,
        pattern: &'n ObjectPattern<'a>,
        mutable: bool,
    ) -> Result<ObjectBinding<'n, 'a>, String> {
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
        enum Work<'n, 'a> {
            Object(&'n ObjectPattern<'a>, NodeId, usize),
            Field(&'n BindingProperty<'a>, bool, usize),
            Leave,
        }
        let mut pending = vec![Work::Object(pattern, source.node_id(), 1)];
        let mut steps = Vec::new();
        let mut count = 0;
        while let Some(work) = pending.pop() {
            match work {
                Work::Object(object, origin, depth) => {
                    if depth > 128 {
                        return Err(unsupported(object, "binding pattern nesting exceeds 128"));
                    }
                    if object.rest.is_some() {
                        return Err(unsupported(
                            object,
                            "object rest bindings are not yet admitted",
                        ));
                    }
                    if depth > 1 && self.stored.container(origin) != Some(Type::ManagedObject) {
                        return Err(unsupported(
                            object,
                            "nested object bindings require a source-proven managed object",
                        ));
                    }
                    steps.push(ObjectBindingStep::Enter(object));
                    pending.push(Work::Leave);
                    let dictionary = self.stored.dictionary(origin);
                    pending.extend(
                        object
                            .properties
                            .iter()
                            .rev()
                            .map(|property| Work::Field(property, dictionary, depth)),
                    );
                }
                Work::Field(property, dictionary, depth) => {
                    let key = key(property).ok_or_else(|| {
                        unsupported(
                            property,
                            "object binding keys require an identifier or String literal",
                        )
                    })?;
                    let key = evaluate::managed_member_key(property, key, false, dictionary)?;
                    steps.push(ObjectBindingStep::Field {
                        source: property,
                        key,
                    });
                    if let Some((identifier, default)) = leaf(&property.value) {
                        count += 1;
                        steps.push(ObjectBindingStep::Bind {
                            identifier,
                            default,
                        });
                    } else if let BindingPattern::ObjectPattern(object) = &property.value {
                        pending.push(Work::Object(object, object.node_id(), depth + 1));
                    } else {
                        return Err(unsupported(
                            property,
                            "object bindings currently require identifier leaves or nested object patterns",
                        ));
                    }
                }
                Work::Leave => steps.push(ObjectBindingStep::Leave),
            }
        }
        self.charge_declaration_bindings(pattern, count)?;
        Ok(ObjectBinding {
            source,
            steps: steps.into_iter(),
            mutable,
        })
    }
}
