//! Flat binding plans borrow the original source and select each own field or
//! array position in order. Origins prove receiver kinds; annotations never do.
use super::*;
use oxc_ast::ast::{ArrayPattern, BindingIdentifier, BindingProperty, ObjectPattern, PropertyKey};

/// Original-source leaves for representation and origin planning. This is not
/// admission: the declaration path still checks the entire pattern and RHS.
pub(super) fn object_leaves<'n, 'a>(
    pattern: &'n BindingPattern<'a>,
) -> impl Iterator<Item = (&'n BindingIdentifier<'a>, &'n BindingProperty<'a>)> {
    let object = match pattern {
        BindingPattern::ObjectPattern(object) => Some(object),
        _ => None,
    };
    object
        .into_iter()
        .flat_map(|object| object.properties.iter())
        .filter_map(|property| leaf(&property.value).map(|(identifier, _)| (identifier, property)))
}

pub(super) fn array_leaves<'n, 'a>(
    pattern: &'n BindingPattern<'a>,
) -> impl Iterator<Item = (usize, &'n BindingIdentifier<'a>, &'n BindingPattern<'a>)> {
    let array = match pattern {
        BindingPattern::ArrayPattern(array) => Some(array),
        _ => None,
    };
    array
        .into_iter()
        .flat_map(|array| array.elements.iter().enumerate())
        .filter_map(|(index, element)| {
            let pattern = element.as_ref()?;
            leaf(pattern).map(|(identifier, _)| (index, identifier, pattern))
        })
}

pub(super) fn leaves<'n, 'a>(
    pattern: &'n BindingPattern<'a>,
) -> impl Iterator<Item = &'n BindingIdentifier<'a>> {
    object_leaves(pattern)
        .map(|(identifier, _)| identifier)
        .chain(array_leaves(pattern).map(|(_, identifier, _)| identifier))
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

pub(super) struct PropertyBinding<'n, 'a> {
    pub(super) source: &'n BindingProperty<'a>,
    pub(super) identifier: &'n BindingIdentifier<'a>,
    pub(super) key: Vec<u16>,
    pub(super) default: Option<&'n Expression<'a>>,
}

pub(super) struct ObjectBinding<'n, 'a> {
    pub(super) source: &'n Expression<'a>,
    pub(super) fields: std::vec::IntoIter<PropertyBinding<'n, 'a>>,
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
        if pattern.rest.is_some() {
            return Err(unsupported(
                pattern,
                "object rest bindings are not yet admitted",
            ));
        }
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
        self.charge_declaration_bindings(pattern, pattern.properties.len())?;
        let dictionary = self.stored.dictionary(source.node_id());
        let mut fields = Vec::with_capacity(pattern.properties.len());
        for property in &pattern.properties {
            let Some((identifier, default)) = leaf(&property.value) else {
                return Err(unsupported(
                    property,
                    "object bindings currently require identifier leaves",
                ));
            };
            let key = key(property).ok_or_else(|| {
                unsupported(
                    property,
                    "object binding keys require an identifier or String literal",
                )
            })?;
            let key = evaluate::managed_member_key(property, key, false, dictionary)?;
            fields.push(PropertyBinding {
                source: property,
                identifier,
                key,
                default,
            });
        }
        Ok(ObjectBinding {
            source,
            fields: fields.into_iter(),
            mutable,
        })
    }
}
