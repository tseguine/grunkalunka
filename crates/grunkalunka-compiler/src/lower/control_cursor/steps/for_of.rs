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
        self.begin_iteration(IterationSource::Of(source), output, depth, cursor)
    }

    pub(super) fn begin_for_in<'n>(
        &mut self,
        source: &'n ForInStatement<'ast>,
        output: Region,
        depth: usize,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String> {
        self.begin_iteration(IterationSource::In(source), output, depth, cursor)
    }

    fn begin_iteration<'n>(
        &mut self,
        source: IterationSource<'n, 'ast>,
        output: Region,
        depth: usize,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String> {
        if let IterationSource::Of(for_of) = source
            && for_of.r#await
        {
            return Err(unsupported(&source, "for-await-of is not yet admitted"));
        }
        if let Some(pattern) = iteration::declaration_iteration_pattern(source.left()) {
            let ForStatementLeft::VariableDeclaration(declaration) = source.left() else {
                unreachable!("checked lexical iteration binding");
            };
            self.charge_statement(declaration.as_ref())?;
            if let BindingPattern::BindingIdentifier(identifier) = pattern {
                self.charge_declaration_bindings(identifier.as_ref(), 1)?;
            }
        } else if let Some(identifier) = iteration::var_iteration_binding(source.left()) {
            let symbol = identifier
                .symbol_id
                .get()
                .ok_or("missing var iteration symbol")?;
            let binding = self
                .bindings
                .iter()
                .find(|b| b.symbol == symbol)
                .ok_or_else(|| {
                    unsupported(identifier, "var iteration requires its hoisted cell")
                })?;
            if !binding.mutable || self.slots[binding.slot.0].ty != Type::ManagedValue {
                return Err(unsupported(
                    identifier,
                    "var iteration requires mutable managed-value storage",
                ));
            }
            self.charge_statement(source.left())?;
            self.charge_declaration_bindings(identifier, 1)?;
        } else if let Some((object, key)) = iteration::property_iteration_target(source.left()) {
            self.check_iteration_member(source, object, key)?;
        } else if let Some(identifier) = iteration::assignment_iteration_identifier(source.left())?
        {
            let binding = numeric_binding(identifier, self.semantic, &self.bindings)?;
            let compatible = match self.slots[binding.slot.0].ty {
                Type::ManagedValue => true,
                Type::Number => self.stored.number(identifier.node_id()),
                Type::General => self.stored.primitive(identifier.node_id()),
                _ => false,
            };
            if !binding.mutable || !compatible {
                return Err(unsupported(
                    identifier,
                    "iteration assignment requires an initialized mutable managed-value binding",
                ));
            }
        } else {
            return Err(unsupported(
                &source,
                "iteration requires one const/let binding or existing mutable identifier",
            ));
        }
        let mode = if source.is_in() {
            match self.managed_source_type(source.right(), 0) {
                Some(Type::ManagedObject) => evaluate::Mode::Object,
                Some(Type::ManagedArray) => evaluate::Mode::Array,
                Some(Type::ManagedValue)
                    if self.stored.string(source.right().node_id())
                        || self.stored.container(source.right().node_id()).is_some() =>
                {
                    evaluate::Mode::Managed
                }
                _ if self
                    .stored
                    .optional_receiver(source.right().node_id())
                    .is_some() =>
                {
                    evaluate::Mode::Managed
                }
                _ if self.numeric_result_shape(source.right(), 0) => evaluate::Mode::Number,
                _ if self.stored.primitive(source.right().node_id()) => {
                    // The four scalar kinds have no enumerable own fields in
                    // the represented pristine realm. Numeric conversion is
                    // effect-free for this source-proven set; it evaluates the
                    // original RHS once and reuses empty scalar enumeration.
                    evaluate::Mode::Numeric
                }
                _ => {
                    if let Some(signature) = self.function_arity(source.right(), 0) {
                        evaluate::Mode::Function(signature)
                    } else {
                        return Err(unsupported(
                            source.right(),
                            "for-in requires a source-proven object, array, String or Number/Boolean/nullish primitive, or represented function",
                        ));
                    }
                }
            }
        } else {
            self.intrinsic_iterable_mode(source.right(), 0)
                .ok_or_else(|| {
                    unsupported(
                        source.right(),
                        "for-of requires a source-proven Array or String",
                    )
                })?
        };
        let header = self.region(&source);
        cursor.pending.push(Frame::ForOfSource {
            source,
            output,
            depth,
            outer: self.bindings.len(),
        });
        // RHS evaluation precedes publication of the shadowing head binding.
        cursor.next = Some(Next::Expression {
            source: source.right(),
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
        let mut iterable = iterable;
        if source.is_in() {
            // The source domain has immutable intrinsic prototypes and own data
            // entries, so enumeration begins with the original own-key order.
            let keys = self.value(Type::ManagedArray);
            let nullable = self.values[iterable.0] == Type::ManagedValue
                && !self.stored.string(source.right().node_id())
                && self.stored.container(source.right().node_id()).is_none();
            if nullable {
                let condition = self.value(Type::Predicate);
                Self::push(
                    &mut frame.header,
                    &source,
                    Op::ManagedIsNullish {
                        result: condition,
                        input: iterable,
                    },
                );
                let mut skipped = self.region(&source);
                let empty = self.value(Type::ManagedArray);
                Self::push(
                    &mut skipped,
                    &source,
                    Op::ManagedArrayNew {
                        result: empty,
                        length: 0,
                    },
                );
                skipped.exit = Exit::Yield(empty);
                let mut present = self.region(&source);
                let nonnull_keys = self.value(Type::ManagedArray);
                Self::push(
                    &mut present,
                    &source,
                    Op::ManagedEnumerateOwn {
                        kind: ir::OwnEnumeration::Keys,
                        result: nonnull_keys,
                        input: iterable,
                    },
                );
                present.exit = Exit::Yield(nonnull_keys);
                Self::push(
                    &mut frame.header,
                    &source,
                    Op::If {
                        result: Some(keys),
                        condition,
                        consequent: skipped,
                        alternate: present,
                    },
                );
            } else {
                Self::push(
                    &mut frame.header,
                    &source,
                    Op::ManagedEnumerateOwn {
                        kind: ir::OwnEnumeration::Keys,
                        result: keys,
                        input: iterable,
                    },
                );
            }
            // Represented closures have no mutable own-property table. Their
            // enumeration is empty, so no per-key object presence check applies.
            if !matches!(self.values[iterable.0], Type::Closure(_)) {
                frame.enumerated = Some(iterable);
            }
            iterable = keys;
        }
        let string = !source.is_in() && self.values[iterable.0] == Type::ManagedValue;
        let zero = self.value(Type::Number);
        Self::push(
            &mut frame.header,
            &source,
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
            &source,
            Op::Initialize {
                slot: index,
                value: zero,
            },
        );

        let position = self.value(Type::Number);
        Self::push(
            &mut frame.condition,
            &source,
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
                &source,
                Op::ManagedStringLength {
                    result: length,
                    input: iterable,
                },
            );
        } else {
            Self::push(
                &mut frame.condition,
                &source,
                Op::ManagedArrayLength {
                    result: length,
                    array: iterable,
                },
            );
        }
        let condition = self.value(Type::Predicate);
        Self::push(
            &mut frame.condition,
            &source,
            Op::Compare {
                result: condition,
                operator: Compare::Less,
                left: position,
                right: length,
            },
        );
        frame.condition.exit = Exit::Yield(condition);

        let mut body = self.region(source.body());
        // Condition and body are siblings. Reload the ancestor-owned slot and
        // advance before source body control flow, including continue or await.
        let position = self.value(Type::Number);
        Self::push(
            &mut body,
            &source,
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
                &source,
                Op::ManagedStringIterValue {
                    result: selected,
                    input: iterable,
                    index: position,
                },
            );
            Self::push(
                &mut body,
                &source,
                Op::ManagedStringLength {
                    result: width,
                    input: selected,
                },
            );
        } else {
            Self::push(
                &mut body,
                &source,
                Op::ManagedArrayAt {
                    result: selected,
                    array: iterable,
                    index: position,
                },
            );
            Self::push(
                &mut body,
                &source,
                Op::Constant {
                    result: width,
                    value: 1.0,
                },
            );
        }
        let next = self.value(Type::Number);
        Self::push(
            &mut body,
            &source,
            Op::Binary {
                result: next,
                operator: Binary::Add,
                left: position,
                right: width,
            },
        );
        Self::push(
            &mut body,
            &source,
            Op::Store {
                slot: index,
                value: next,
            },
        );
        if let Some(original) = frame.enumerated {
            let original = if self.values[original.0] == Type::ManagedValue {
                match self.stored.optional_receiver(source.right().node_id()) {
                    Some(ty @ (Type::ManagedObject | Type::ManagedArray)) => {
                        // A yielded key proves the saved receiver was non-nullish.
                        let projected = self.value(ty);
                        Self::push(
                            &mut body,
                            &source,
                            if ty == Type::ManagedArray {
                                Op::ManagedToArray {
                                    result: projected,
                                    input: original,
                                }
                            } else {
                                Op::ManagedToObject {
                                    result: projected,
                                    input: original,
                                }
                            },
                        );
                        projected
                    }
                    _ => original,
                }
            } else {
                original
            };
            // Unlike a plain Object.keys loop, for-in skips a key removed
            // before its visit. New keys do not join the initial enumeration.
            let present = self.value(Type::Predicate);
            Self::push(
                &mut body,
                &source,
                Op::ManagedHasOwn {
                    result: present,
                    input: original,
                    key: ir::ManagedKey::String(selected),
                },
            );
            frame.presence = Some(present);
            // Keep only the unconditionally executed cursor advance and own-key
            // check in the prefix. Assigning the head (or creating its captured
            // lexical cell) is conditional on the key still existing.
            frame.prefix = Some(body);
            body = self.region(source.body());
        }
        if let Some(pattern) = iteration::declaration_iteration_pattern(source.left()) {
            let ForStatementLeft::VariableDeclaration(declaration) = source.left() else {
                unreachable!("checked lexical iteration binding");
            };
            // Lexical heads create fresh cells; var patterns write hoisted cells.
            let mutable = declaration.kind != VariableDeclarationKind::Const;
            if let BindingPattern::BindingIdentifier(identifier) = pattern {
                self.commit_binding_value(
                    identifier.as_ref(),
                    identifier
                        .symbol_id
                        .get()
                        .ok_or("missing iteration binding")?,
                    &mut body,
                    Type::ManagedValue,
                    selected,
                    mutable,
                )?;
                self.begin_for_of_body(frame, body, cursor);
            } else {
                let root_type = if self.stored.string(pattern.node_id()) {
                    Some(Type::ManagedValue)
                } else if self.stored.object_primitive(pattern.node_id()) {
                    Some(Type::General)
                } else {
                    self.stored.container(pattern.node_id())
                }
                .ok_or_else(|| {
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
        } else if let Some(identifier) = iteration::var_iteration_binding(source.left()) {
            let symbol = identifier
                .symbol_id
                .get()
                .ok_or("missing var iteration symbol")?;
            let slot = self
                .bindings
                .iter()
                .find(|b| b.symbol == symbol)
                .ok_or("missing hoisted var iteration cell")?
                .slot;
            Self::push(
                &mut body,
                &source,
                Op::Store {
                    slot,
                    value: selected,
                },
            );
            self.begin_for_of_body(frame, body, cursor);
        } else if let Some((object, _)) = iteration::property_iteration_target(source.left()) {
            let ty = self
                .receiver_type(object, 1)
                .expect("validated iteration target");
            cursor
                .pending
                .push(Frame::ForOfMemberReceiver { frame, selected });
            cursor.next = Some(Next::Expression {
                source: object,
                output: body,
                depth: 1,
                mode: if ty == Type::ManagedArray {
                    evaluate::Mode::Array
                } else {
                    evaluate::Mode::Object
                },
            });
        } else {
            let identifier = iteration::assignment_iteration_identifier(source.left())?
                .expect("validated existing iteration binding");
            let binding = numeric_binding(identifier, self.semantic, &self.bindings)?;
            let slot = binding.slot;
            // Unlike a declaration, the existing cell is assigned on every step.
            // Captures of that cell observe subsequent iterations.
            let selected = match self.slots[slot.0].ty {
                Type::ManagedValue => selected,
                ty @ (Type::Number | Type::General) => {
                    // Admission proved the selection's kind, so these existing
                    // projections preserve its tag/bits without user coercion.
                    let primitive = self.value(Type::General);
                    Self::push(
                        &mut body,
                        &source,
                        Op::ManagedToPrimitive {
                            result: primitive,
                            input: selected,
                        },
                    );
                    if ty == Type::Number {
                        let number = self.value(Type::Number);
                        Self::push(
                            &mut body,
                            &source,
                            Op::NumberFromGeneral {
                                result: number,
                                input: primitive,
                            },
                        );
                        number
                    } else {
                        primitive
                    }
                }
                _ => unreachable!("validated iteration assignment storage"),
            };
            Self::push(
                &mut body,
                &source,
                Op::Store {
                    slot,
                    value: selected,
                },
            );
            self.begin_for_of_body(frame, body, cursor);
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
            source: source.body(),
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
            presence,
            prefix,
            ..
        } = frame;
        self.in_loop_body = old_loop;
        self.bindings.truncate(outer);
        let (body, completion) = match (presence, prefix) {
            (Some(present), Some(mut prefix)) => {
                let alternate = self.region(&source);
                Self::push(
                    &mut prefix,
                    &source,
                    Op::If {
                        result: None,
                        condition: present,
                        consequent: body,
                        alternate,
                    },
                );
                (prefix, Completion::NORMAL.union(completion))
            }
            (None, None) => (body, completion),
            _ => unreachable!("for-in key presence belongs to its prefix"),
        };
        Self::push(
            &mut header,
            &source,
            Op::While {
                condition,
                body,
                step: None,
            },
        );
        Self::push(&mut output, &source, Op::Block(header));
        cursor.completed = Some(Outcome::Statement(output, completion.pretest_loop()));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn scalar_iteration_assignments_require_matching_storage_proofs() {
        for (body, admitted) in [
            ("let x=0;for(x of [a,b]){}return x;", true),
            ("for(a of [b]){}return a;", true),
            ("let x=0;for(x of []){}return x;", true),
            ("let x=null;for(x of [a,false,undefined]){}return +x;", true),
            ("let x=0;for(x of ['x']){}return x;", false),
            ("let x=undefined;for(x of [a,,b]){}return +x;", false),
            ("let x=0;for(x of [a,null]){}return x;", false),
            ("let x=0;for(x of [a])x=false;return +x;", true),
            ("let x=null;for(x of [{}]){}return +x;", false),
            ("let x=0;for(x in {a:1}){}return x;", false),
            ("const x=0;for(x of [a]){}return x;", false),
            ("for(x of [a]){}return a;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let r = crate::compile(&source, typed);
                assert_eq!(r.is_ok(), admitted, "{source}: {:?}", r.err());
            }
        }
        assert!(
            crate::compile(
                "export function run(a,b){let x:number=0;for(x of ['x'] as number[]){}return x;}",
                true
            )
            .is_err()
        );
    }

    #[test]
    fn scalar_iteration_heads_keep_statement_depth_limits_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                for (depth, admitted) in [(64, true), (65, false)] {
                    let mut body = String::from("break;");
                    for _ in 0..depth {
                        body = format!("for(x of [a]){body}");
                    }
                    let source = format!("export function run(a,b){{let x=0;{body}return x;}}");
                    for typed in [false, true] {
                        let r = crate::compile(&source, typed);
                        assert_eq!(r.is_ok(), admitted, "{depth}: {:?}", r.err());
                    }
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn function_for_in_requires_proven_callable_values_and_validates_bodies() {
        assert!(crate::compile("function f(x){return x+1;}export function run(a,b){let n=0;for(const k in f)n++;return n;}",false).is_ok());
        for (body, ok) in [
            (
                "const f=x=>x+a;let n=0;for(const k in f)n++;return n;",
                true,
            ),
            ("let n=0;for(const k in (x=>x+a))n++;return n;", true),
            (
                "const f=x=>x+a;const g=x=>x+b;let n=0;for(const k in (a?f:g))n++;return n;",
                true,
            ),
            ("let n=0;for(const k in (x=>1n))n++;return n;", false),
            (
                "const f=x=>x+a;let n=0;for(const k in (a?f:null))n++;return n;",
                false,
            ),
            (
                "const f=x=>x+a;let n=0;for(const k of f)n++;return n;",
                false,
            ),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let r = crate::compile(&source, typed);
                assert_eq!(r.is_ok(), ok, "{source}: {:?}", r.err());
            }
        }
    }

    #[test]
    fn for_in_scalar_sources_keep_effects_and_reject_opaque_values() {
        for (body, admitted) in [
            ("let n=0;for(const k in null)n++;return n;", true),
            ("let n=0;for(const k in undefined)n++;return n;", true),
            ("let n=0;for(const k in false)n++;return n;", true),
            ("let n=0;for(const k in (a?true:null))n++;return n;", true),
            ("let n=0;for(const k in (n++,undefined))n++;return n;", true),
            ("let n=0;for(const {} in null)n++;return n;", true),
            ("let n=0;for(const k of null)n++;return n;", false),
            ("let n=0;for(const k in 1n)n++;return n;", false),
            ("let n=0;for(const k in (a?{}:null))n++;return n;", true),
        ] {
            let s = format!("export function run(a,b){{{body}}}");
            for ts in [false, true] {
                let r = crate::compile(&s, ts);
                assert_eq!(r.is_ok(), admitted, "{s}: {:?}", r.err());
            }
        }
        let js = "export function run(a,b){let n=0;for(const k in (a?false:null))n++;return n;}";
        let ts = "export function run(a:string,b:boolean):never{let n=0;for(const k in ((a?false:null) as object))n++;return n;}";
        assert_eq!(
            crate::compile(js, false).unwrap().wasm,
            crate::compile(ts, true).unwrap().wasm
        );
    }
}

#[cfg(test)]
mod nullable_tests {
    #[test]
    fn nullable_for_in_requires_proven_homogeneous_receivers() {
        for (body, ok) in [
            (
                "const o=a?{x:b}:null;let n=0;for(const k in o)n++;return n;",
                true,
            ),
            (
                "const o=a?[b]:undefined;let n=0;for(const k in o)n++;return n;",
                true,
            ),
            (
                "const o=a?'ab':null;let n=0;for(const k in o)n++;return n;",
                true,
            ),
            ("const o=a?{}:[];for(const k in o){}return a;", false),
            ("const o=a?{}:b;for(const k in o){}return a;", false),
            ("const o=a?{x:b}:null;for(const k of o){}return a;", false),
            ("for(const k in (a?{x:1}:1n)){}return a;", false),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typed in [false, true] {
                let r = crate::compile(&source, typed);
                assert_eq!(r.is_ok(), ok, "{source}: {:?}", r.err());
            }
        }
        assert!(
            crate::compile(
                "export function run(a,b){for(const k in ((a?{}:b) as object)){}return b;}",
                true
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod nullable_depth_tests {
    #[test]
    fn nullable_for_in_keeps_statement_depth_limits_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                for (depth, ok) in [(64, true), (65, false)] {
                    let mut body = String::from("break;");
                    for i in 0..depth {
                        body = format!("for(const k{i} in (a?{{x:b}}:null)){body}");
                    }
                    let source = format!("export function run(a,b){{{body}return a;}}");
                    for typed in [false, true] {
                        let r = crate::compile(&source, typed);
                        assert_eq!(r.is_ok(), ok, "{depth}: {:?}", r.err());
                    }
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
