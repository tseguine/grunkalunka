//! Fields and array elements are selected only after the preceding binding
//! completes. Every continuation retains the originally evaluated RHS.
use super::*;
use oxc_ast::ast::BindingIdentifier;

pub(super) struct DefaultBinding<'n, 'a> {
    binding: binding_patterns::ObjectBinding<'n, 'a>,
    output: Region,
    object: ValueId,
    pending: LazyDefault,
    identifier: &'n BindingIdentifier<'a>,
}

pub(super) struct LazyDefault {
    selected: ValueId,
    condition: ValueId,
}

impl<'s, 'ast> Lowering<'s, 'ast> {
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
                let undefined = self.binding_undefined(field.source, &mut output);
                let pending =
                    self.prepare_binding_default(field.source, &mut output, selected, undefined);
                let child = self.region(source);
                cursor.pending.push(Frame::ObjectBindingDefault {
                    sequence,
                    active: Box::new(DefaultBinding {
                        binding,
                        output,
                        object,
                        pending,
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
        consequent: Region,
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
            pending,
            identifier,
        } = active;
        let result =
            self.finish_binding_default(identifier, &mut output, pending, consequent, default);
        self.publish_pattern_binding(identifier, &mut output, result, binding.mutable)?;
        self.resume_object_binding_source(sequence, binding, output, object, cursor)
    }

    pub(super) fn binding_undefined(
        &mut self,
        node: &impl GetSpan,
        output: &mut Region,
    ) -> ValueId {
        let primitive = self.value(Type::General);
        Self::push(
            output,
            node,
            Op::Nullish {
                result: primitive,
                value: ir::Nullish::Undefined,
            },
        );
        let undefined = self.value(Type::ManagedValue);
        Self::push(
            output,
            node,
            Op::ManagedPrimitive {
                result: undefined,
                input: primitive,
            },
        );
        undefined
    }

    pub(super) fn prepare_binding_default(
        &mut self,
        node: &impl GetSpan,
        output: &mut Region,
        selected: ValueId,
        undefined: ValueId,
    ) -> LazyDefault {
        // Intrinsic Undefined is independent of source shadowing. All other
        // values, including null and false, retain their original selection.
        let condition = self.value(Type::Predicate);
        Self::push(
            output,
            node,
            Op::ManagedEqual {
                result: condition,
                left: selected,
                right: undefined,
            },
        );
        LazyDefault {
            selected,
            condition,
        }
    }

    pub(super) fn finish_binding_default(
        &mut self,
        node: &impl GetSpan,
        output: &mut Region,
        pending: LazyDefault,
        mut consequent: Region,
        default: ValueId,
    ) -> ValueId {
        consequent.exit = Exit::Yield(default);
        let mut alternate = self.region(node);
        alternate.exit = Exit::Yield(pending.selected);
        let result = self.value(Type::ManagedValue);
        Self::push(
            output,
            node,
            Op::If {
                result: Some(result),
                condition: pending.condition,
                consequent,
                alternate,
            },
        );
        result
    }

    pub(super) fn publish_pattern_binding(
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
