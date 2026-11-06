//! Intrinsic Array and String iteration on the shared source cursor.
use super::*;
use oxc_ast::ast::ForStatementLeft;

impl<'s, 'ast> Lowering<'s, 'ast> {
    pub(super) fn begin_for_of<'n>(
        &mut self,
        source: &'n ForOfStatement<'ast>,
        output: Region,
        depth: usize,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String> {
        if source.r#await {
            return Err(unsupported(source, "for-await-of is not yet admitted"));
        }
        let pattern = iteration::for_of_pattern(source)
            .ok_or_else(|| unsupported(source, "for-of requires one const or let binding"))?;
        let ForStatementLeft::VariableDeclaration(declaration) = &source.left else {
            unreachable!("checked lexical for-of binding");
        };
        self.charge_statement(declaration.as_ref())?;
        if let BindingPattern::BindingIdentifier(identifier) = pattern {
            self.charge_declaration_bindings(identifier.as_ref(), 1)?;
        }
        let ty = self.managed_source_type(&source.right, 0);
        let mode = if self.stored.string(source.right.node_id()) {
            evaluate::Mode::Managed
        } else if ty == Some(Type::ManagedArray)
            || (ty == Some(Type::ManagedValue)
                && self.stored.container(source.right.node_id()) == Some(Type::ManagedArray))
        {
            evaluate::Mode::Array
        } else {
            return Err(unsupported(
                &source.right,
                "for-of requires a source-proven Array or String",
            ));
        };
        let header = self.region(source);
        cursor.pending.push(Frame::ForOfSource {
            source,
            output,
            depth,
            outer: self.bindings.len(),
        });
        // Do not publish the head binding before RHS evaluation. References to
        // its shadowing symbol retain the existing controlled TDZ rejection.
        cursor.next = Some(Next::Expression {
            source: &source.right,
            output: header,
            depth: 0,
            mode,
        });
        Ok(())
    }

    pub(super) fn resume_for_of_source<'n>(
        &mut self,
        mut frame: Box<ForOfFrame<'n, 'ast>>,
        iterable: ValueId,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String> {
        let source = frame.source;
        let string = self.values[iterable.0] == Type::ManagedValue;
        let zero = self.value(Type::Number);
        Self::push(
            &mut frame.header,
            source,
            Op::Constant {
                result: zero,
                value: 0.0,
            },
        );
        let index = SlotId(self.slots.len());
        self.slots.push(Slot {
            ty: Type::Number,
            owner: frame.header.id,
            mutable: true,
            parameter: false,
            span: source.span(),
        });
        Self::push(
            &mut frame.header,
            source,
            Op::Initialize {
                slot: index,
                value: zero,
            },
        );

        let position = self.value(Type::Number);
        Self::push(
            &mut frame.condition,
            source,
            Op::Load {
                result: position,
                slot: index,
            },
        );
        let length = self.value(Type::Number);
        if string {
            // A saved String is immutable. Arrays must re-read their live length.
            Self::push(
                &mut frame.header,
                source,
                Op::ManagedStringLength {
                    result: length,
                    input: iterable,
                },
            );
        } else {
            Self::push(
                &mut frame.condition,
                source,
                Op::ManagedArrayLength {
                    result: length,
                    array: iterable,
                },
            );
        }
        let condition = self.value(Type::Predicate);
        Self::push(
            &mut frame.condition,
            source,
            Op::Compare {
                result: condition,
                operator: Compare::Less,
                left: position,
                right: length,
            },
        );
        frame.condition.exit = Exit::Yield(condition);

        let mut body = self.region(&source.body);
        // Condition and body are siblings. Reload the ancestor-owned slot and
        // advance before source body control flow, including continue or await.
        let position = self.value(Type::Number);
        Self::push(
            &mut body,
            source,
            Op::Load {
                result: position,
                slot: index,
            },
        );
        let selected = self.value(Type::ManagedValue);
        let width = self.value(Type::Number);
        if string {
            Self::push(
                &mut body,
                source,
                Op::ManagedStringIterValue {
                    result: selected,
                    input: iterable,
                    index: position,
                },
            );
            Self::push(
                &mut body,
                source,
                Op::ManagedStringLength {
                    result: width,
                    input: selected,
                },
            );
        } else {
            Self::push(
                &mut body,
                source,
                Op::ManagedArrayAt {
                    result: selected,
                    array: iterable,
                    index: position,
                },
            );
            Self::push(
                &mut body,
                source,
                Op::Constant {
                    result: width,
                    value: 1.0,
                },
            );
        }
        let next = self.value(Type::Number);
        Self::push(
            &mut body,
            source,
            Op::Binary {
                result: next,
                operator: Binary::Add,
                left: position,
                right: width,
            },
        );
        Self::push(
            &mut body,
            source,
            Op::Store {
                slot: index,
                value: next,
            },
        );
        let pattern = iteration::for_of_pattern(source).expect("admitted for-of binding");
        let ForStatementLeft::VariableDeclaration(declaration) = &source.left else {
            unreachable!("checked lexical for-of binding");
        };
        // A body-owned Initialize allocates a fresh captured cell each time,
        // for const as well as let. No classic for-header renewal is needed.
        let mutable = declaration.kind == VariableDeclarationKind::Let;
        if let BindingPattern::BindingIdentifier(identifier) = pattern {
            self.commit_binding_value(
                identifier.as_ref(),
                identifier.symbol_id.get().ok_or("missing for-of binding")?,
                &mut body,
                Type::ManagedValue,
                selected,
                mutable,
            )?;
            self.begin_for_of_body(frame, body, cursor);
        } else {
            let root_type = self.stored.container(pattern.node_id()).ok_or_else(|| {
                unsupported(
                    pattern,
                    "for-of binding patterns require source-proven container elements",
                )
            })?;
            let plan =
                self.prepare_binding_pattern(pattern, pattern.node_id(), root_type, mutable)?;
            self.start_binding_pattern(
                bindings::BindingResume::ForOf(frame),
                plan,
                body,
                selected,
                cursor,
            )?;
        }
        Ok(())
    }

    pub(in super::super) fn begin_for_of_body<'n>(
        &mut self,
        frame: Box<ForOfFrame<'n, 'ast>>,
        body: Region,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) {
        let source = frame.source;
        let depth = frame.depth;
        cursor.pending.push(Frame::ForOf(frame));
        self.in_loop_body = true;
        cursor.next = Some(Next::Statement {
            source: &source.body,
            output: body,
            depth: depth + 1,
        });
    }

    pub(super) fn finish_for_of<'n>(
        &mut self,
        frame: ForOfFrame<'n, 'ast>,
        body: Region,
        completion: Completion,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) {
        let ForOfFrame {
            source,
            mut output,
            mut header,
            condition,
            outer,
            old_loop,
            ..
        } = frame;
        self.in_loop_body = old_loop;
        self.bindings.truncate(outer);
        Self::push(
            &mut header,
            source,
            Op::While {
                condition,
                body,
                step: None,
            },
        );
        Self::push(&mut output, source, Op::Block(header));
        cursor.completed = Some(Outcome::Statement(output, completion.pretest_loop()));
    }
}
