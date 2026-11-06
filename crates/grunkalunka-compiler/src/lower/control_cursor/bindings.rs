//! Fields and array elements are selected only after the preceding binding
//! completes. Every continuation retains the originally evaluated RHS.
use super::*;
use oxc_ast::ast::{AssignmentPattern, BindingIdentifier, BindingProperty};

pub(super) struct DefaultBinding<'n, 'a> {
    state: BindingState<'n, 'a>,
    output: Region,
    pending: LazyDefault,
    source: &'n AssignmentPattern<'a>,
}

pub(super) enum BindingResume<'n, 'a> {
    Declaration(Box<DeclarationSequence<'n, 'a>>),
    ForOf(Box<ForOfFrame<'n, 'a>>),
}

impl BindingResume<'_, '_> {
    pub(super) fn depth(&self) -> usize {
        match self {
            Self::Declaration(sequence) => sequence.depth,
            Self::ForOf(frame) => frame.depth,
        }
    }
}

enum Container {
    Object {
        value: ValueId,
        excluded: Option<Vec<ir::ManagedKey>>,
    },
    Array {
        value: ValueId,
        iterator: Option<array_bindings::IteratorState>,
    },
}

pub(super) struct BindingState<'n, 'a> {
    plan: binding_patterns::BindingPlan<'n, 'a>,
    containers: Vec<Container>,
    selected: Option<ValueId>,
}

pub(super) struct LazyDefault {
    selected: ValueId,
    condition: ValueId,
}

impl<'s, 'ast> Lowering<'s, 'ast> {
    pub(super) fn resume_pattern_binding_source<'n>(
        &mut self,
        sequence: Box<DeclarationSequence<'n, 'ast>>,
        binding: binding_patterns::PatternBinding<'n, 'ast>,
        output: Region,
        value: ValueId,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        self.start_binding_pattern(
            BindingResume::Declaration(sequence),
            binding.plan,
            output,
            value,
            cursor,
        )
    }

    pub(super) fn start_binding_pattern<'n>(
        &mut self,
        resume: BindingResume<'n, 'ast>,
        plan: binding_patterns::BindingPlan<'n, 'ast>,
        output: Region,
        value: ValueId,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        self.advance_binding_pattern(
            resume,
            BindingState {
                plan,
                containers: Vec::new(),
                selected: Some(value),
            },
            output,
            cursor,
        )
    }

    fn advance_binding_pattern<'n>(
        &mut self,
        resume: BindingResume<'n, 'ast>,
        mut state: BindingState<'n, 'ast>,
        mut output: Region,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        use binding_patterns::BindingStep;
        while let Some(step) = state.plan.steps.next() {
            match step {
                BindingStep::Object { source, receiver } => {
                    let selected = state.selected.take().expect("selected object binding");
                    let value = if self.values[selected.0] == receiver {
                        selected
                    } else {
                        let result = self.value(receiver);
                        let op = match receiver {
                            Type::ManagedArray => Op::ManagedToArray {
                                result,
                                input: selected,
                            },
                            Type::ManagedObject => Op::ManagedToObject {
                                result,
                                input: selected,
                            },
                            _ => unreachable!("planned property binding requires a container"),
                        };
                        Self::push(&mut output, source, op);
                        result
                    };
                    state.containers.push(Container::Object {
                        value,
                        excluded: source.rest.as_ref().map(|_| Vec::new()),
                    });
                }
                BindingStep::Array { source, live } => {
                    let selected = state.selected.take().expect("selected array binding");
                    let array = if self.values[selected.0] == Type::ManagedArray {
                        selected
                    } else {
                        let result = self.value(Type::ManagedArray);
                        Self::push(
                            &mut output,
                            source,
                            Op::ManagedToArray {
                                result,
                                input: selected,
                            },
                        );
                        result
                    };
                    let iterator =
                        live.then(|| self.begin_array_binding_iterator(source, &mut output));
                    state.containers.push(Container::Array {
                        value: array,
                        iterator,
                    });
                }
                BindingStep::Field { source, key } => {
                    let Some(Container::Object {
                        value: object,
                        excluded,
                    }) = state.containers.last_mut()
                    else {
                        unreachable!("object field has an object receiver");
                    };
                    if let Some(excluded) = excluded {
                        excluded.push(ir::ManagedKey::Literal(key.clone()));
                    }
                    let selected = self.value(Type::ManagedValue);
                    let op = if self.values[object.0] == Type::ManagedArray {
                        Op::ManagedArrayGetOwn {
                            result: selected,
                            array: *object,
                            key,
                        }
                    } else {
                        Op::ManagedGetOwn {
                            result: selected,
                            object: *object,
                            key,
                        }
                    };
                    Self::push(&mut output, source, op);
                    state.selected = Some(selected);
                }
                BindingStep::ComputedField { source, key, depth } => {
                    cursor.pending.push(Frame::PatternBindingKey {
                        resume,
                        state: Box::new(state),
                        source,
                    });
                    cursor.next = Some(Next::Expression {
                        source: key,
                        output,
                        depth,
                        mode: evaluate::Mode::Managed,
                    });
                    return Ok(());
                }
                BindingStep::Element { source, index } => {
                    let Some(Container::Array { value, iterator }) = state.containers.last_mut()
                    else {
                        unreachable!("array element has an array receiver");
                    };
                    let selected = if let Some(iterator) = iterator {
                        self.select_array_binding_step(source, &mut output, *value, iterator, index)
                    } else {
                        self.read_array_binding_element(source, &mut output, *value, index)
                    };
                    state.selected = Some(selected);
                }
                BindingStep::ObjectRest(source) => {
                    let Some(Container::Object {
                        value: object,
                        excluded,
                    }) = state.containers.last_mut()
                    else {
                        unreachable!("object rest has an object receiver");
                    };
                    let rest = self.value(Type::ManagedObject);
                    Self::push(
                        &mut output,
                        source,
                        Op::ManagedObjectRest {
                            result: rest,
                            object: *object,
                            excluded: excluded.take().expect("planned object rest"),
                        },
                    );
                    let selected = self.value(Type::ManagedValue);
                    Self::push(
                        &mut output,
                        source,
                        Op::ManagedFromObject {
                            result: selected,
                            input: rest,
                        },
                    );
                    state.selected = Some(selected);
                }
                BindingStep::ArrayRest { source, start } => {
                    let Some(Container::Array { value, iterator }) = state.containers.last() else {
                        unreachable!("array rest has an array receiver");
                    };
                    let array = self.collect_array_binding_rest(
                        source,
                        &mut output,
                        *value,
                        iterator.as_ref(),
                        start,
                    );
                    let selected = self.value(Type::ManagedValue);
                    Self::push(
                        &mut output,
                        source,
                        Op::ManagedFromArray {
                            result: selected,
                            input: array,
                        },
                    );
                    state.selected = Some(selected);
                }
                BindingStep::Default(source) => {
                    let selected = state.selected.take().expect("selected binding value");
                    let undefined = match state.containers.last() {
                        Some(Container::Array {
                            iterator: Some(iterator),
                            ..
                        }) => iterator.undefined,
                        _ => self.binding_undefined(&source.left, &mut output),
                    };
                    let pending = self.prepare_binding_default(
                        &source.left,
                        &mut output,
                        selected,
                        undefined,
                    );
                    let child = self.region(&source.right);
                    cursor.pending.push(Frame::PatternBindingDefault {
                        resume,
                        active: Box::new(DefaultBinding {
                            state,
                            output,
                            pending,
                            source,
                        }),
                    });
                    cursor.next = Some(Next::Expression {
                        source: &source.right,
                        output: child,
                        depth: 0,
                        mode: evaluate::Mode::Managed,
                    });
                    return Ok(());
                }
                BindingStep::Bind { identifier } => {
                    let selected = state.selected.take().expect("selected binding value");
                    self.publish_pattern_binding(
                        identifier,
                        &mut output,
                        selected,
                        state.plan.mutable,
                    )?;
                }
                BindingStep::Leave => {
                    state.containers.pop().expect("balanced binding plan");
                }
            }
        }
        match resume {
            BindingResume::Declaration(mut sequence) => {
                sequence.index += 1;
                self.advance_declaration(sequence, output, cursor)
            }
            BindingResume::ForOf(frame) => {
                self.begin_for_of_body(frame, output, cursor);
                Ok(())
            }
        }
    }

    pub(super) fn resume_pattern_binding_key<'n>(
        &mut self,
        resume: BindingResume<'n, 'ast>,
        mut state: BindingState<'n, 'ast>,
        source: &'n BindingProperty<'ast>,
        mut output: Region,
        key: ValueId,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String>
    where
        'ast: 'n,
    {
        let Some(Container::Object { value, excluded }) = state.containers.last_mut() else {
            unreachable!("computed binding field has an object receiver");
        };
        if let Some(excluded) = excluded {
            excluded.push(ir::ManagedKey::String(key));
        }
        let selected = self.value(Type::ManagedValue);
        Self::push(
            &mut output,
            source,
            Op::ManagedGetOwnDynamic {
                result: selected,
                object: *value,
                key,
            },
        );
        state.selected = Some(selected);
        self.advance_binding_pattern(resume, state, output, cursor)
    }

    pub(super) fn resume_pattern_binding_default<'n>(
        &mut self,
        resume: BindingResume<'n, 'ast>,
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
            mut output,
            pending,
            source,
        } = active;
        let result =
            self.finish_binding_default(&source.left, &mut output, pending, consequent, default);
        state.selected = Some(result);
        self.advance_binding_pattern(resume, state, output, cursor)
    }

    pub(in crate::lower) fn binding_undefined(
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
