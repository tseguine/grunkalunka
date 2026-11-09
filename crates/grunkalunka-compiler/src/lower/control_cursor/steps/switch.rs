//! Switch dispatch uses heap-owned continuations and existing structured IR.
use super::*;

pub(in crate::lower::control_cursor) struct SwitchFrame<'n, 'a> {
    source: &'n SwitchStatement<'a>,
    output: Region,
    pub(in crate::lower::control_cursor) depth: usize,
    visible: usize,
    discriminant: ValueId,
    old_loop: bool,
    index: usize,
    default: Option<usize>,
    arms: Vec<Arm>,
}

struct Arm {
    index: usize,
    test: Region,
    condition: ValueId,
    body: Region,
    completion: Completion,
}

impl<'s, 'ast> Lowering<'s, 'ast> {
    fn switch_operand(&self, expression: &Expression<'_>) -> bool {
        self.stored.represented(expression.node_id())
    }

    fn switch_index(
        &mut self,
        source: &SwitchStatement<'_>,
        index: usize,
        output: &mut Region,
    ) -> ValueId {
        let result = self.value(Type::Number);
        Self::push(
            output,
            source,
            Op::Constant {
                result,
                value: index as f64,
            },
        );
        result
    }

    pub(super) fn begin_switch<'n>(
        &mut self,
        source: &'n SwitchStatement<'ast>,
        output: Region,
        depth: usize,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String> {
        if !self.switch_operand(&source.discriminant) {
            return Err(unsupported(
                source,
                "switch discriminant requires proven primitives, Strings or references",
            ));
        }
        cursor
            .pending
            .push(Frame::SwitchDiscriminant { source, depth });
        cursor.next = Some(Next::Expression {
            source: &source.discriminant,
            output,
            depth: 0,
            mode: evaluate::Mode::Managed,
        });
        Ok(())
    }

    pub(super) fn resume_switch_discriminant<'n>(
        &mut self,
        source: &'n SwitchStatement<'ast>,
        depth: usize,
        output: Region,
        discriminant: ValueId,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String> {
        let old_loop = self.in_loop_body;
        self.in_loop_body = true;
        let state = Box::new(SwitchFrame {
            source,
            output,
            depth,
            visible: self.bindings.len(),
            discriminant,
            old_loop,
            index: 0,
            default: source.cases.iter().position(|case| case.test.is_none()),
            arms: Vec::new(),
        });
        self.next_switch_case(state, cursor)
    }

    fn next_switch_case<'n>(
        &mut self,
        mut state: Box<SwitchFrame<'n, 'ast>>,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String> {
        // A default in the middle is selected only after every case test misses.
        if Some(state.index) == state.default {
            state.index += 1;
        }
        if let Some(case) = state.source.cases.get(state.index) {
            let test = case.test.as_ref().expect("default skipped");
            if !self.switch_operand(test) {
                return Err(unsupported(
                    test,
                    "switch case requires proven primitives, Strings or references",
                ));
            }
            let output = self.region(case);
            cursor.pending.push(Frame::SwitchTest(state));
            cursor.next = Some(Next::Expression {
                source: test,
                output,
                depth: 0,
                mode: evaluate::Mode::Managed,
            });
        } else if let Some(default) = state.default {
            let case = &state.source.cases[default];
            let output = self.region(case);
            let depth = state.depth + 1;
            cursor.pending.push(Frame::SwitchDefault(state));
            cursor.next = Some(Next::Enter {
                source: ControlNode::Sequence(&case.consequent),
                output,
                depth,
                charge: source_control::SourceCharge::Charged,
            });
        } else {
            // An unmatched non-exhaustive switch resumes its enclosing sequence.
            // The discriminant and every missed test have already run once.
            let miss = self.region(state.source);
            self.finish_switch(*state, miss, Completion::NORMAL, cursor)?;
        }
        Ok(())
    }

    pub(super) fn resume_switch_test<'n>(
        &mut self,
        state: Box<SwitchFrame<'n, 'ast>>,
        mut test: Region,
        value: ValueId,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String> {
        let case = &state.source.cases[state.index];
        let condition = self.value(Type::Predicate);
        Self::push(
            &mut test,
            case,
            Op::ManagedEqual {
                result: condition,
                left: state.discriminant,
                right: value,
            },
        );
        let output = self.region(case);
        let depth = state.depth + 1;
        cursor.pending.push(Frame::SwitchArm {
            state,
            test,
            condition,
        });
        cursor.next = Some(Next::Enter {
            source: ControlNode::Sequence(&case.consequent),
            output,
            depth,
            charge: source_control::SourceCharge::Charged,
        });
        Ok(())
    }

    pub(super) fn resume_switch_arm<'n>(
        &mut self,
        mut state: Box<SwitchFrame<'n, 'ast>>,
        test: Region,
        condition: ValueId,
        body: Region,
        completion: Completion,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String> {
        self.bindings.truncate(state.visible);
        state.arms.push(Arm {
            index: state.index,
            test,
            condition,
            body,
            completion,
        });
        state.index += 1;
        self.next_switch_case(state, cursor)
    }

    pub(super) fn finish_switch<'n>(
        &mut self,
        mut state: SwitchFrame<'n, 'ast>,
        mut alternate: Region,
        mut completion: Completion,
        cursor: &mut Cursor<'s, 'n, 'ast>,
    ) -> Result<(), String> {
        self.bindings.truncate(state.visible);
        let count = state.source.cases.len();
        let selected = SlotId(self.slots.len());
        self.slots.push(Slot {
            ty: Type::Number,
            owner: state.output.id,
            mutable: true,
            parameter: false,
            span: state.source.span(),
        });
        let missing = self.switch_index(state.source, count, &mut state.output);
        Self::push(
            &mut state.output,
            state.source,
            Op::Initialize {
                slot: selected,
                value: missing,
            },
        );
        let mut bodies: Vec<Option<(Region, Completion)>> = (0..count).map(|_| None).collect();
        // Selection and execution are distinct. Case tests stop at the first
        // match; the selected index then enters bodies in original source order.
        let mut selection = self.region(state.source);
        if let Some(default) = state.default {
            bodies[default] = Some((alternate, completion));
            let index = self.switch_index(state.source, default, &mut selection);
            Self::push(
                &mut selection,
                state.source,
                Op::Store {
                    slot: selected,
                    value: index,
                },
            );
        }
        while let Some(mut arm) = state.arms.pop() {
            let mut chosen = self.region(state.source);
            let index = self.switch_index(state.source, arm.index, &mut chosen);
            Self::push(
                &mut chosen,
                state.source,
                Op::Store {
                    slot: selected,
                    value: index,
                },
            );
            bodies[arm.index] = Some((arm.body, arm.completion));
            Self::push(
                &mut arm.test,
                state.source,
                Op::If {
                    result: None,
                    condition: arm.condition,
                    consequent: chosen,
                    alternate: selection,
                },
            );
            selection = arm.test;
        }
        Self::push(&mut state.output, state.source, Op::Block(selection));
        // Each possible entry inherits the completion of its remaining suffix.
        let mut suffix = Completion::NORMAL;
        let mut entries: Option<Completion> = None;
        for (_, child) in bodies.iter().rev().flatten() {
            suffix = child.then(suffix);
            entries = Some(entries.map_or(suffix, |all| all.union(suffix)));
        }
        completion = entries.unwrap_or(Completion::NORMAL);
        if state.default.is_none() {
            completion = completion.union(Completion::NORMAL);
        }
        alternate = self.region(state.source);
        for (index, body) in bodies.into_iter().enumerate() {
            let (body, _) = body.expect("every switch body compiled once");
            if index + 1 == count && state.default.is_some() {
                // An exhaustive dispatch always reaches this final body unless
                // an earlier body completed abruptly. Preserve that structural
                // return proof in IR without relying on value correlations.
                Self::push(&mut alternate, state.source, Op::Block(body));
            } else {
                let position = self.value(Type::Number);
                Self::push(
                    &mut alternate,
                    state.source,
                    Op::Load {
                        result: position,
                        slot: selected,
                    },
                );
                let index = self.switch_index(state.source, index, &mut alternate);
                let condition = self.value(Type::Predicate);
                Self::push(
                    &mut alternate,
                    state.source,
                    Op::Compare {
                        result: condition,
                        operator: Compare::LessEqual,
                        left: position,
                        right: index,
                    },
                );
                let miss = self.region(state.source);
                Self::push(
                    &mut alternate,
                    state.source,
                    Op::If {
                        result: None,
                        condition,
                        consequent: body,
                        alternate: miss,
                    },
                );
            }
        }
        self.in_loop_body = state.old_loop;
        {
            // Even a break overridden by a finalizer needs a lexical IR target.
            // A switch-local break leaves this synthetic loop. For arms that
            // continue the enclosing loop, reserve count+1 as a completion
            // marker. Only continue may enter the condition: ordinary
            // fallthrough leaves via break, after any finalizers have run.
            let mut condition = self.region(state.source);
            let marker = if completion.continues {
                if completion.normal {
                    alternate.exit = Exit::Break;
                }
                let marker = self.switch_index(state.source, count + 1, &mut state.output);
                Self::push(
                    &mut condition,
                    state.source,
                    Op::Store {
                        slot: selected,
                        value: marker,
                    },
                );
                Some(marker)
            } else {
                None
            };
            let zero = self.value(Type::Number);
            Self::push(
                &mut condition,
                state.source,
                Op::Constant {
                    result: zero,
                    value: 0.0,
                },
            );
            let predicate = self.value(Type::Predicate);
            Self::push(
                &mut condition,
                state.source,
                Op::Truthy {
                    result: predicate,
                    input: zero,
                },
            );
            condition.exit = Exit::Yield(predicate);
            Self::push(
                &mut state.output,
                state.source,
                Op::DoWhile {
                    body: alternate,
                    condition,
                },
            );
            completion.normal |= completion.breaks;
            completion.breaks = false;
            if let Some(marker) = marker {
                let mut relay = self.region(state.source);
                relay.exit = Exit::Continue;
                if completion.normal {
                    let position = self.value(Type::Number);
                    Self::push(
                        &mut state.output,
                        state.source,
                        Op::Load {
                            result: position,
                            slot: selected,
                        },
                    );
                    let condition = self.value(Type::Predicate);
                    Self::push(
                        &mut state.output,
                        state.source,
                        Op::Compare {
                            result: condition,
                            operator: Compare::Equal,
                            left: position,
                            right: marker,
                        },
                    );
                    let fallthrough = self.region(state.source);
                    Self::push(
                        &mut state.output,
                        state.source,
                        Op::If {
                            result: None,
                            condition,
                            consequent: relay,
                            alternate: fallthrough,
                        },
                    );
                } else {
                    // Return and throw already left the function or protected region.
                    Self::push(&mut state.output, state.source, Op::Block(relay));
                }
            }
        }
        cursor.completed = Some(Outcome::Statement(state.output, completion));
        Ok(())
    }
}
