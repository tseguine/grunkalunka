//! Fields and array elements are selected only after the preceding binding
//! completes. Every continuation retains the originally evaluated RHS.
use super::*;
use oxc_ast::ast::BindingIdentifier;

pub(super) struct DefaultBinding<'n, 'a> {
    binding: binding_patterns::ObjectBinding<'n, 'a>,
    output: Region,
    object: ValueId,
    selected: ValueId,
    condition: ValueId,
    identifier: &'n BindingIdentifier<'a>,
}

impl<'s, 'ast> Lowering<'s, 'ast> {
    pub(super) fn resume_array_binding_source<'n>(
        &mut self,
        mut sequence: Box<DeclarationSequence<'n, 'ast>>,
        binding: binding_patterns::ArrayBinding<'n, 'ast>,
        mut output: Region,
        array: ValueId,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        // Only the intrinsic iterator of a source-proven managed array is
        // admitted. Without defaults there are no source effects between steps;
        // direct own reads also produce Undefined for holes and exhausted steps.
        // Elisions need no value read or generated operation in this slice.
        for element in binding.elements {
            let selected = self.value(Type::ManagedValue);
            Self::push(
                &mut output,
                element.identifier,
                Op::ManagedArrayGetOwn {
                    result: selected,
                    array,
                    key: element.index.to_string().encode_utf16().collect(),
                },
            );
            self.publish_pattern_binding(
                element.identifier,
                &mut output,
                selected,
                binding.mutable,
            )?;
        }
        sequence.index += 1;
        self.advance_declaration(sequence, output, cursor)
    }

    pub(super) fn resume_object_binding_source<'n>(
        &mut self,
        mut sequence: Box<DeclarationSequence<'n, 'ast>>,
        mut binding: binding_patterns::ObjectBinding<'n, 'ast>,
        mut output: Region,
        object: ValueId,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        while let Some(field) = binding.fields.next() {
            let selected = self.value(Type::ManagedValue);
            Self::push(
                &mut output,
                field.source,
                Op::ManagedGetOwn {
                    result: selected,
                    object,
                    key: field.key,
                },
            );
            if let Some(source) = field.default {
                // Null, false, zero, NaN and empty Strings retain their value.
                // This intrinsic is independent of a source binding named undefined.
                let primitive = self.value(Type::General);
                Self::push(
                    &mut output,
                    field.source,
                    Op::Nullish {
                        result: primitive,
                        value: ir::Nullish::Undefined,
                    },
                );
                let undefined = self.value(Type::ManagedValue);
                Self::push(
                    &mut output,
                    field.source,
                    Op::ManagedPrimitive {
                        result: undefined,
                        input: primitive,
                    },
                );
                let condition = self.value(Type::Predicate);
                Self::push(
                    &mut output,
                    field.source,
                    Op::ManagedEqual {
                        result: condition,
                        left: selected,
                        right: undefined,
                    },
                );
                let child = self.region(source);
                cursor.pending.push(Frame::ObjectBindingDefault {
                    sequence,
                    active: Box::new(DefaultBinding {
                        binding,
                        output,
                        object,
                        selected,
                        condition,
                        identifier: field.identifier,
                    }),
                });
                cursor.next = Some(Next::Expression {
                    source,
                    output: child,
                    depth: 0,
                    mode: evaluate::Mode::Managed,
                });
                return Ok(());
            }
            self.publish_pattern_binding(field.identifier, &mut output, selected, binding.mutable)?;
        }
        sequence.index += 1;
        self.advance_declaration(sequence, output, cursor)
    }

    pub(super) fn resume_object_binding_default<'n>(
        &mut self,
        sequence: Box<DeclarationSequence<'n, 'ast>>,
        active: DefaultBinding<'n, 'ast>,
        mut consequent: Region,
        default: ValueId,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        let DefaultBinding {
            binding,
            mut output,
            object,
            selected,
            condition,
            identifier,
        } = active;
        consequent.exit = Exit::Yield(default);
        let mut alternate = self.region(identifier);
        alternate.exit = Exit::Yield(selected);
        let result = self.value(Type::ManagedValue);
        Self::push(
            &mut output,
            identifier,
            Op::If {
                result: Some(result),
                condition,
                consequent,
                alternate,
            },
        );
        self.publish_pattern_binding(identifier, &mut output, result, binding.mutable)?;
        self.resume_object_binding_source(sequence, binding, output, object, cursor)
    }

    fn publish_pattern_binding(
        &mut self,
        identifier: &BindingIdentifier<'_>,
        output: &mut Region,
        value: ValueId,
        mutable: bool,
    ) -> Result<(), String> {
        self.commit_binding_value(
            identifier,
            identifier
                .symbol_id
                .get()
                .ok_or("missing pattern binding")?,
            output,
            Type::ManagedValue,
            value,
            mutable,
        )
    }
}
