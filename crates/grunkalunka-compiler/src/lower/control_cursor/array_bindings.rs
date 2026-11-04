//! Intrinsic array binding steps retain live length and persistent exhaustion
//! across lazy defaults. No source-sized recursion or runtime iterator escapes.
use super::*;
use oxc_ast::ast::BindingIdentifier;

struct IteratorState {
    active: ValueId,
    undefined: ValueId,
}

struct ArrayBindingState<'n, 'a> {
    binding: binding_patterns::ArrayBinding<'n, 'a>,
    output: Region,
    array: ValueId,
    iterator: Option<IteratorState>,
}

pub(super) struct DefaultBinding<'n, 'a> {
    state: ArrayBindingState<'n, 'a>,
    pending: bindings::LazyDefault,
    identifier: &'n BindingIdentifier<'a>,
}

impl<'s, 'ast> Lowering<'s, 'ast> {
    pub(super) fn resume_array_binding_source<'n>(
        &mut self,
        sequence: Box<DeclarationSequence<'n, 'ast>>,
        binding: binding_patterns::ArrayBinding<'n, 'ast>,
        mut output: Region,
        array: ValueId,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        // Identifier-only patterns have no source effects between steps and
        // retain their direct-read path. Any default needs iterator state for
        // the whole pattern, including non-defaulted leaves after exhaustion.
        let iterator = if binding
            .elements
            .as_slice()
            .iter()
            .any(|e| e.default.is_some())
        {
            let active = self.value(Type::Predicate);
            Self::push(
                &mut output,
                binding.source,
                Op::Boolean {
                    result: active,
                    value: true,
                },
            );
            let undefined = self.binding_undefined(binding.source, &mut output);
            Some(IteratorState { active, undefined })
        } else {
            None
        };
        self.advance_array_binding(
            sequence,
            ArrayBindingState {
                binding,
                output,
                array,
                iterator,
            },
            cursor,
        )
    }

    fn advance_array_binding<'n>(
        &mut self,
        mut sequence: Box<DeclarationSequence<'n, 'ast>>,
        mut state: ArrayBindingState<'n, 'ast>,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        while let Some(element) = state.binding.elements.next() {
            let selected = if let Some(iterator) = &mut state.iterator {
                self.select_array_binding_step(
                    element.identifier,
                    &mut state.output,
                    state.array,
                    iterator,
                    element.index,
                )
            } else {
                self.read_array_binding_element(
                    element.identifier,
                    &mut state.output,
                    state.array,
                    element.index,
                )
            };
            if let Some(source) = element.default {
                let undefined = state
                    .iterator
                    .as_ref()
                    .expect("default has iterator state")
                    .undefined;
                let pending = self.prepare_binding_default(
                    element.identifier,
                    &mut state.output,
                    selected,
                    undefined,
                );
                let child = self.region(source);
                cursor.pending.push(Frame::ArrayBindingDefault {
                    sequence,
                    active: Box::new(DefaultBinding {
                        state,
                        pending,
                        identifier: element.identifier,
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
            self.publish_pattern_binding(
                element.identifier,
                &mut state.output,
                selected,
                state.binding.mutable,
            )?;
        }
        sequence.index += 1;
        self.advance_declaration(sequence, state.output, cursor)
    }

    pub(super) fn resume_array_binding_default<'n>(
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
            mut state,
            pending,
            identifier,
        } = active;
        let result = self.finish_binding_default(
            identifier,
            &mut state.output,
            pending,
            consequent,
            default,
        );
        self.publish_pattern_binding(identifier, &mut state.output, result, state.binding.mutable)?;
        self.advance_array_binding(sequence, state, cursor)
    }

    fn read_array_binding_element(
        &mut self,
        node: &impl GetSpan,
        output: &mut Region,
        array: ValueId,
        index: u32,
    ) -> ValueId {
        let selected = self.value(Type::ManagedValue);
        Self::push(
            output,
            node,
            Op::ManagedArrayGetOwn {
                result: selected,
                array,
                key: index.to_string().encode_utf16().collect(),
            },
        );
        selected
    }

    fn select_array_binding_step(
        &mut self,
        node: &impl GetSpan,
        output: &mut Region,
        array: ValueId,
        iterator: &mut IteratorState,
        index: u32,
    ) -> ValueId {
        // No admitted hooks run in elisions. Checking the next bound index
        // also observes exhaustion anywhere in an intervening run of holes.
        // Length remains live until the first exhausted step, then stays done.
        let mut live = self.region(node);
        let length = self.value(Type::Number);
        Self::push(
            &mut live,
            node,
            Op::ManagedArrayLength {
                result: length,
                array,
            },
        );
        let position = self.value(Type::Number);
        Self::push(
            &mut live,
            node,
            Op::Constant {
                result: position,
                value: f64::from(index),
            },
        );
        let in_bounds = self.value(Type::Predicate);
        Self::push(
            &mut live,
            node,
            Op::Compare {
                result: in_bounds,
                operator: ir::Compare::Less,
                left: position,
                right: length,
            },
        );
        live.exit = Exit::Yield(in_bounds);
        let mut exhausted = self.region(node);
        exhausted.exit = Exit::Yield(iterator.active);
        let active = self.value(Type::Predicate);
        Self::push(
            output,
            node,
            Op::If {
                result: Some(active),
                condition: iterator.active,
                consequent: live,
                alternate: exhausted,
            },
        );
        iterator.active = active;

        let mut present = self.region(node);
        let selected = self.read_array_binding_element(node, &mut present, array, index);
        present.exit = Exit::Yield(selected);
        let mut absent = self.region(node);
        absent.exit = Exit::Yield(iterator.undefined);
        let result = self.value(Type::ManagedValue);
        Self::push(
            output,
            node,
            Op::If {
                result: Some(result),
                condition: active,
                consequent: present,
                alternate: absent,
            },
        );
        result
    }
}
