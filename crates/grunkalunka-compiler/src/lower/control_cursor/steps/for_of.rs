//! Intrinsic managed-array iteration on the shared source cursor.
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
        let identifier = iteration::for_of_binding(source).ok_or_else(|| {
            unsupported(
                source,
                "for-of requires one const or let identifier binding",
            )
        })?;
        let ForStatementLeft::VariableDeclaration(declaration) = &source.left else {
            unreachable!("checked lexical for-of binding");
        };
        self.charge_statement(declaration.as_ref())?;
        self.charge_declaration_bindings(identifier, 1)?;
        let ty = self.managed_source_type(&source.right, 0);
        if ty != Some(Type::ManagedArray)
            && !(ty == Some(Type::ManagedValue)
                && self.stored.container(source.right.node_id()) == Some(Type::ManagedArray))
        {
            return Err(unsupported(
                &source.right,
                "for-of requires a source-proven managed array",
            ));
        }
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
            mode: evaluate::Mode::Array,
        });
        Ok(())
    }

    pub(super) fn resume_for_of_source<'n>(
        &mut self,
        mut frame: Box<ForOfFrame<'n, 'ast>>,
        array: ValueId,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String> {
        let source = frame.source;
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
        Self::push(
            &mut frame.condition,
            source,
            Op::ManagedArrayLength {
                result: length,
                array,
            },
        );
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
        let one = self.value(Type::Number);
        Self::push(
            &mut body,
            source,
            Op::Constant {
                result: one,
                value: 1.0,
            },
        );
        let next = self.value(Type::Number);
        Self::push(
            &mut body,
            source,
            Op::Binary {
                result: next,
                operator: Binary::Add,
                left: position,
                right: one,
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
        let selected = self.value(Type::ManagedValue);
        Self::push(
            &mut body,
            source,
            Op::ManagedArrayAt {
                result: selected,
                array,
                index: position,
            },
        );
        let identifier = iteration::for_of_binding(source).expect("admitted for-of binding");
        let ForStatementLeft::VariableDeclaration(declaration) = &source.left else {
            unreachable!("checked lexical for-of binding");
        };
        // A body-owned Initialize allocates a fresh captured cell each time,
        // for const as well as let. No classic for-header renewal is needed.
        self.commit_binding_value(
            identifier,
            identifier.symbol_id.get().ok_or("missing for-of binding")?,
            &mut body,
            Type::ManagedValue,
            selected,
            declaration.kind == VariableDeclarationKind::Let,
        )?;
        let depth = frame.depth;
        cursor.pending.push(Frame::ForOf(frame));
        self.in_loop_body = true;
        cursor.next = Some(Next::Statement {
            source: &source.body,
            output: body,
            depth: depth + 1,
        });
        Ok(())
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
