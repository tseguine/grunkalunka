//! Return/throw-only switch dispatch uses heap-owned continuations and ordinary If IR.
use super::*;

pub(in crate::lower::control_cursor) struct SwitchFrame<'n, 'a> {
    source: &'n SwitchStatement<'a>,
    output: Region,
    pub(in crate::lower::control_cursor) depth: usize,
    visible: usize,
    discriminant: ValueId,
    index: usize,
    default: Option<usize>,
    arms: Vec<Arm>,
}

struct Arm {
    test: Region,
    condition: ValueId,
    body: Region,
    completion: Completion,
}

impl<'s, 'ast> Lowering<'s, 'ast> {
    fn switch_operand(&self, expression: &Expression<'_>) -> bool {
        self.stored.represented(expression.node_id())
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
        let state = Box::new(SwitchFrame {
            source,
            output,
            depth,
            visible: self.bindings.len(),
            discriminant,
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
        let final_arm = state.index + 1 == state.source.cases.len();
        if completion.breaks || completion.continues || (!final_arm && completion.normal) {
            return Err(unsupported(
                &state.source.cases[state.index],
                "switch arms must return or throw on every path; fallthrough and switch breaks are not yet admitted",
            ));
        }
        self.bindings.truncate(state.visible);
        state.arms.push(Arm {
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
        if let Some(default) = state.default
            && (completion.breaks
                || completion.continues
                || (default + 1 != state.source.cases.len() && completion.normal))
        {
            return Err(unsupported(
                &state.source.cases[default],
                "switch arms must return or throw on every path; fallthrough and switch breaks are not yet admitted",
            ));
        }
        self.bindings.truncate(state.visible);
        // Fold the pending chain iteratively; no source-depth native recursion.
        while let Some(mut arm) = state.arms.pop() {
            completion = completion.union(arm.completion);
            Self::push(
                &mut arm.test,
                state.source,
                Op::If {
                    result: None,
                    condition: arm.condition,
                    consequent: arm.body,
                    alternate,
                },
            );
            alternate = arm.test;
        }
        Self::push(&mut state.output, state.source, Op::Block(alternate));
        cursor.completed = Some(Outcome::Statement(state.output, completion));
        Ok(())
    }
}
