//! Ordered, typed structured regions. Values are snapshots; slots are source cells.
use oxc_span::Span;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct ValueId(pub usize);
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct SlotId(pub usize);
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct RegionId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Type {
    Number,
    Predicate,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Unary {
    Plus,
    Negate,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum Binary {
    Add,
    Subtract,
    Multiply,
    Divide,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum Compare {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,
}

#[derive(Debug)]
pub(crate) struct Slot {
    pub owner: RegionId,
    pub mutable: bool,
    pub parameter: bool,
    pub span: Span,
}

#[derive(Debug)]
pub(crate) struct Function {
    pub slots: Vec<Slot>,
    pub values: Vec<Type>,
    pub body: Region,
}

#[derive(Debug)]
pub(crate) struct Region {
    pub id: RegionId,
    pub span: Span,
    pub operations: Vec<Operation>,
    pub exit: Exit,
}

#[derive(Debug)]
pub(crate) struct Operation {
    pub span: Span,
    pub kind: Op,
}

#[derive(Debug)]
pub(crate) enum Op {
    Not {
        result: ValueId,
        input: ValueId,
    },
    Boolean {
        result: ValueId,
        value: bool,
    },
    Constant {
        result: ValueId,
        value: f64,
    },
    Load {
        result: ValueId,
        slot: SlotId,
    },
    Initialize {
        slot: SlotId,
        value: ValueId,
    },
    Store {
        slot: SlotId,
        value: ValueId,
    },
    Unary {
        result: ValueId,
        operator: Unary,
        input: ValueId,
    },
    Binary {
        result: ValueId,
        operator: Binary,
        left: ValueId,
        right: ValueId,
    },
    Compare {
        result: ValueId,
        operator: Compare,
        left: ValueId,
        right: ValueId,
    },
    Truthy {
        result: ValueId,
        input: ValueId,
    },
    Block(Region),
    If {
        result: Option<ValueId>,
        condition: ValueId,
        consequent: Region,
        alternate: Region,
    },
    While {
        condition: Region,
        body: Region,
        step: Option<Region>,
    },
    DoWhile {
        body: Region,
        condition: Region,
    },
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Exit {
    Fallthrough,
    Yield(ValueId),
    Return(ValueId),
    Break,
}

/// Possible source completions. Only normal paths reach a sequential suffix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Completion {
    pub normal: bool,
    pub returns: bool,
    pub breaks: bool,
}

impl Completion {
    pub const NORMAL: Self = Self {
        normal: true,
        returns: false,
        breaks: false,
    };
    pub const RETURN: Self = Self {
        normal: false,
        returns: true,
        breaks: false,
    };
    pub const BREAK: Self = Self {
        normal: false,
        returns: false,
        breaks: true,
    };

    pub fn union(self, other: Self) -> Self {
        Self {
            normal: self.normal || other.normal,
            returns: self.returns || other.returns,
            breaks: self.breaks || other.breaks,
        }
    }

    pub fn then(self, other: Self) -> Self {
        Self {
            normal: self.normal && other.normal,
            returns: self.returns || (self.normal && other.returns),
            breaks: self.breaks || (self.normal && other.breaks),
        }
    }

    pub fn pretest_loop(self) -> Self {
        Self {
            normal: true,
            returns: self.returns,
            breaks: false,
        }
    }

    pub fn do_loop(self) -> Self {
        Self {
            normal: self.normal || self.breaks,
            returns: self.returns,
            breaks: false,
        }
    }
}

fn invalid(span: Span, reason: &str) -> String {
    format!(
        "internal IR at bytes {}..{}: {reason}",
        span.start, span.end
    )
}

#[derive(Clone, Copy)]
enum Contract {
    Statement,
    Yield(Type),
    Function,
}

struct Verifier<'a> {
    function: &'a Function,
    defined: BTreeSet<ValueId>,
    regions: BTreeSet<RegionId>,
}

impl Verifier<'_> {
    fn value(
        &self,
        value: ValueId,
        ty: Type,
        visible: &BTreeSet<ValueId>,
        span: Span,
    ) -> Result<(), String> {
        if self.function.values.get(value.0) != Some(&ty) {
            return Err(invalid(span, "invalid value ID or value type"));
        }
        if !visible.contains(&value) {
            return Err(invalid(
                span,
                "value is not visible (undefined, forward or outside its region)",
            ));
        }
        Ok(())
    }

    fn define(
        &mut self,
        value: ValueId,
        ty: Type,
        visible: &mut BTreeSet<ValueId>,
        span: Span,
    ) -> Result<(), String> {
        if self.function.values.get(value.0) != Some(&ty) {
            return Err(invalid(span, "invalid result value ID or type"));
        }
        if !self.defined.insert(value) {
            return Err(invalid(span, "value defined twice"));
        }
        visible.insert(value);
        Ok(())
    }

    fn slot(&self, slot: SlotId, ancestors: &[RegionId], span: Span) -> Result<&Slot, String> {
        let slot = self
            .function
            .slots
            .get(slot.0)
            .ok_or_else(|| invalid(span, "invalid slot ID"))?;
        if !ancestors.contains(&slot.owner) {
            return Err(invalid(span, "slot owner is outside the lexical region"));
        }
        Ok(slot)
    }

    // Child regions inherit snapshots and initialized outer source cells. Their
    // new values and cells never become visible to siblings or their parent.
    fn region(
        &mut self,
        region: &Region,
        contract: Contract,
        outer_values: &BTreeSet<ValueId>,
        outer_cells: &BTreeSet<SlotId>,
        ancestors: &mut Vec<RegionId>,
        allow_break: bool,
    ) -> Result<Completion, String> {
        if !self.regions.insert(region.id) {
            return Err(invalid(region.span, "region ID is not unique"));
        }
        ancestors.push(region.id);
        let mut visible = outer_values.clone();
        let mut initialized = outer_cells.clone();
        let mut completion = Completion::NORMAL;
        for operation in &region.operations {
            let span = operation.span;
            if !completion.normal {
                return Err(invalid(
                    span,
                    if completion == Completion::RETURN {
                        "operation after guaranteed return"
                    } else {
                        "operation after guaranteed abrupt completion"
                    },
                ));
            }
            let mut next = Completion::NORMAL;
            match &operation.kind {
                Op::Not { result, input } => {
                    self.value(*input, Type::Predicate, &visible, span)?;
                    self.define(*result, Type::Predicate, &mut visible, span)?;
                }
                Op::Boolean { result, .. } => {
                    self.define(*result, Type::Predicate, &mut visible, span)?
                }
                Op::DoWhile { body, condition } => {
                    next = self
                        .region(
                            body,
                            Contract::Statement,
                            &visible,
                            &initialized,
                            ancestors,
                            true,
                        )?
                        .do_loop();
                    // The condition is a sibling and is checked even when the
                    // first body entry guarantees a function return.
                    self.region(
                        condition,
                        Contract::Yield(Type::Predicate),
                        &visible,
                        &initialized,
                        ancestors,
                        false,
                    )?;
                }
                Op::Constant { result, .. } => {
                    self.define(*result, Type::Number, &mut visible, span)?
                }
                Op::Load { result, slot } => {
                    self.slot(*slot, ancestors, span)?;
                    if !initialized.contains(slot) {
                        return Err(invalid(span, "slot is not initialized"));
                    }
                    self.define(*result, Type::Number, &mut visible, span)?;
                }
                Op::Initialize { slot, value } => {
                    self.value(*value, Type::Number, &visible, span)?;
                    let cell = self.slot(*slot, ancestors, span)?;
                    if cell.owner != region.id || cell.parameter {
                        return Err(invalid(
                            span,
                            "initialization must be in the slot owner's lexical entry",
                        ));
                    }
                    if !initialized.insert(*slot) {
                        return Err(invalid(span, "slot initialized twice per lexical entry"));
                    }
                }
                Op::Store { slot, value } => {
                    self.value(*value, Type::Number, &visible, span)?;
                    let cell = self.slot(*slot, ancestors, span)?;
                    if !cell.mutable {
                        return Err(invalid(span, "store to immutable source cell"));
                    }
                    if !initialized.contains(slot) {
                        return Err(invalid(span, "slot is not initialized"));
                    }
                }
                Op::Unary { result, input, .. } => {
                    self.value(*input, Type::Number, &visible, span)?;
                    self.define(*result, Type::Number, &mut visible, span)?;
                }
                Op::Binary {
                    result,
                    left,
                    right,
                    ..
                }
                | Op::Compare {
                    result,
                    left,
                    right,
                    ..
                } => {
                    self.value(*left, Type::Number, &visible, span)?;
                    self.value(*right, Type::Number, &visible, span)?;
                    let ty = if matches!(operation.kind, Op::Compare { .. }) {
                        Type::Predicate
                    } else {
                        Type::Number
                    };
                    self.define(*result, ty, &mut visible, span)?;
                }
                Op::Truthy { result, input } => {
                    self.value(*input, Type::Number, &visible, span)?;
                    self.define(*result, Type::Predicate, &mut visible, span)?;
                }
                Op::Block(child) => {
                    next = self.region(
                        child,
                        Contract::Statement,
                        &visible,
                        &initialized,
                        ancestors,
                        allow_break,
                    )?;
                }
                Op::If {
                    result,
                    condition,
                    consequent,
                    alternate,
                } => {
                    self.value(*condition, Type::Predicate, &visible, span)?;
                    let contract = if let Some(result) = result {
                        let ty = self
                            .function
                            .values
                            .get(result.0)
                            .copied()
                            .ok_or_else(|| invalid(span, "invalid if result value ID"))?;
                        Contract::Yield(ty)
                    } else {
                        Contract::Statement
                    };
                    let left = self.region(
                        consequent,
                        contract,
                        &visible,
                        &initialized,
                        ancestors,
                        allow_break && result.is_none(),
                    )?;
                    let right = self.region(
                        alternate,
                        contract,
                        &visible,
                        &initialized,
                        ancestors,
                        allow_break && result.is_none(),
                    )?;
                    if let Some(result) = result {
                        let Contract::Yield(ty) = contract else {
                            unreachable!()
                        };
                        self.define(*result, ty, &mut visible, span)?;
                    }
                    next = left.union(right);
                }
                Op::While {
                    condition,
                    body,
                    step,
                } => {
                    self.region(
                        condition,
                        Contract::Yield(Type::Predicate),
                        &visible,
                        &initialized,
                        ancestors,
                        false,
                    )?;
                    next = self
                        .region(
                            body,
                            Contract::Statement,
                            &visible,
                            &initialized,
                            ancestors,
                            true,
                        )?
                        .pretest_loop();
                    // Verify the sibling step even when every body path returns.
                    if let Some(step) = step {
                        let step_completion = self.region(
                            step,
                            Contract::Statement,
                            &visible,
                            &initialized,
                            ancestors,
                            false,
                        )?;
                        if step_completion != Completion::NORMAL {
                            return Err(invalid(step.span, "loop step must complete normally"));
                        }
                    }
                    // Pretest loops do not prove return coverage. This remains
                    // conservative for a source for loop with an omitted test.
                }
            }
            completion = completion.then(next);
        }
        for (index, slot) in self.function.slots.iter().enumerate() {
            if slot.owner == region.id && !initialized.contains(&SlotId(index)) {
                return Err(invalid(
                    slot.span,
                    "owned slot is not initialized on its lexical entry",
                ));
            }
        }
        match (contract, region.exit) {
            (Contract::Yield(ty), Exit::Yield(value)) if completion == Completion::NORMAL => {
                self.value(value, ty, &visible, region.span)?
            }
            (Contract::Statement | Contract::Function, Exit::Return(value))
                if completion.normal =>
            {
                self.value(value, Type::Number, &visible, region.span)?;
                completion = completion.then(Completion::RETURN);
            }
            (Contract::Statement, Exit::Break) if completion.normal && allow_break => {
                completion = completion.then(Completion::BREAK);
            }
            (Contract::Statement | Contract::Function, Exit::Fallthrough) => {}
            _ => return Err(invalid(region.span, "invalid region exit contract")),
        }
        if matches!(contract, Contract::Function) && completion != Completion::RETURN {
            return Err(invalid(
                region.span,
                "function must return a number on every structural path",
            ));
        }
        ancestors.pop();
        Ok(completion)
    }
}

pub(crate) fn verify(function: &Function) -> Result<(), String> {
    let mut initialized = BTreeSet::new();
    for (index, slot) in function.slots.iter().enumerate() {
        if slot.parameter != (index < 2)
            || (slot.parameter && (slot.owner != function.body.id || !slot.mutable))
        {
            return Err(invalid(
                slot.span,
                "exactly two mutable root parameter slots are required",
            ));
        }
        if slot.parameter {
            initialized.insert(SlotId(index));
        }
    }
    if initialized.len() != 2 {
        return Err(invalid(
            function.body.span,
            "two initialized parameters are required",
        ));
    }
    let mut verifier = Verifier {
        function,
        defined: BTreeSet::new(),
        regions: BTreeSet::new(),
    };
    verifier.region(
        &function.body,
        Contract::Function,
        &BTreeSet::new(),
        &initialized,
        &mut Vec::new(),
        false,
    )?;
    for slot in &function.slots {
        if !verifier.regions.contains(&slot.owner) {
            return Err(invalid(slot.span, "invalid slot owner region"));
        }
    }
    if verifier.defined.len() != function.values.len() {
        return Err(invalid(
            function.body.span,
            "value table contains undefined values",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn break_loop() -> Function {
        let mut body = region(vec![], Exit::Break);
        body.id = RegionId(1);
        let mut condition = region(
            vec![Op::Boolean {
                result: ValueId(1),
                value: false,
            }],
            Exit::Yield(ValueId(1)),
        );
        condition.id = RegionId(2);
        let mut f = function(
            vec![
                Op::Constant {
                    result: ValueId(0),
                    value: 7.0,
                },
                Op::While {
                    condition,
                    body,
                    step: None,
                },
            ],
            Exit::Return(ValueId(0)),
        );
        f.values.push(Type::Predicate);
        f
    }

    // The malformed inner header must not inherit the outer body's break target.
    fn inside_outer_loop(mut f: Function) -> Function {
        let mut body = region(vec![], f.body.exit);
        body.operations = std::mem::take(&mut f.body.operations);
        body.id = RegionId(3);
        let mut condition = region(
            vec![Op::Boolean {
                result: ValueId(2),
                value: false,
            }],
            Exit::Yield(ValueId(2)),
        );
        condition.id = RegionId(4);
        f.values.extend([Type::Predicate, Type::Number]);
        f.body = region(
            vec![
                Op::Constant {
                    result: ValueId(3),
                    value: 9.0,
                },
                Op::While {
                    condition,
                    body,
                    step: None,
                },
            ],
            Exit::Return(ValueId(3)),
        );
        f
    }

    #[test]
    fn break_is_consumed_only_by_a_loop_body() {
        verify(&break_loop()).unwrap();
        verify(&inside_outer_loop(break_loop())).unwrap();
        let mut f = function(vec![], Exit::Break);
        f.values.clear();
        assert!(verify(&f).unwrap_err().contains("exit contract"));
    }

    #[test]
    fn break_headers_reject_nested_block_and_if_exits() {
        for in_step in [false, true] {
            for in_if in [false, true] {
                let mut f = inside_outer_loop(break_loop());
                let mut bad = region(vec![], Exit::Break);
                bad.id = RegionId(6);
                let nested = if in_if {
                    let mut alternate = region(vec![], Exit::Fallthrough);
                    alternate.id = RegionId(7);
                    Op::If {
                        result: None,
                        condition: ValueId(if in_step { 4 } else { 1 }),
                        consequent: bad,
                        alternate,
                    }
                } else {
                    Op::Block(bad)
                };
                if in_step {
                    f.values.push(Type::Predicate);
                }
                let Op::While { body, .. } = &mut f.body.operations[1].kind else {
                    panic!()
                };
                let Op::While {
                    condition, step, ..
                } = &mut body.operations[1].kind
                else {
                    panic!()
                };
                if in_step {
                    let mut tail = region(
                        vec![
                            Op::Boolean {
                                result: ValueId(4),
                                value: true,
                            },
                            nested,
                        ],
                        Exit::Fallthrough,
                    );
                    tail.id = RegionId(5);
                    *step = Some(tail);
                } else {
                    condition.operations.push(Operation {
                        span: condition.span,
                        kind: nested,
                    });
                }
                assert!(verify(&f).unwrap_err().contains("exit contract"));
            }
        }
    }

    #[test]
    fn break_step_cannot_hide_a_return() {
        let mut f = break_loop();
        let mut tail = region(vec![], Exit::Return(ValueId(0)));
        tail.id = RegionId(5);
        let Op::While { step, .. } = &mut f.body.operations[1].kind else {
            panic!()
        };
        *step = Some(tail);
        assert!(
            verify(&inside_outer_loop(f))
                .unwrap_err()
                .contains("step must complete normally")
        );
    }

    #[test]
    fn break_and_return_alternatives_are_preserved() {
        let mixed = Completion::BREAK.union(Completion::NORMAL);
        assert_eq!(
            mixed.then(Completion::NORMAL).then(Completion::RETURN),
            Completion::BREAK.union(Completion::RETURN)
        );
        assert_eq!(
            mixed.then(Completion::RETURN).do_loop(),
            Completion::NORMAL.union(Completion::RETURN)
        );
        assert_eq!(
            Completion::BREAK.then(Completion::NORMAL),
            Completion::BREAK
        );
        assert_eq!(Completion::BREAK.pretest_loop(), Completion::NORMAL);
    }
    fn region(operations: Vec<Op>, exit: Exit) -> Region {
        Region {
            id: RegionId(0),
            span: Span::new(10, 20),
            operations: operations
                .into_iter()
                .map(|kind| Operation {
                    span: Span::new(12, 15),
                    kind,
                })
                .collect(),
            exit,
        }
    }
    fn function(operations: Vec<Op>, exit: Exit) -> Function {
        Function {
            slots: (0..2)
                .map(|_| Slot {
                    owner: RegionId(0),
                    mutable: true,
                    parameter: true,
                    span: Span::new(0, 1),
                })
                .collect(),
            values: vec![Type::Number],
            body: region(operations, exit),
        }
    }

    fn loop_with_step() -> Function {
        let mut condition = region(
            vec![
                Op::Load {
                    result: ValueId(1),
                    slot: SlotId(0),
                },
                Op::Truthy {
                    result: ValueId(2),
                    input: ValueId(1),
                },
            ],
            Exit::Yield(ValueId(2)),
        );
        condition.id = RegionId(1);
        let mut body = region(
            vec![Op::Constant {
                result: ValueId(3),
                value: 3.0,
            }],
            Exit::Return(ValueId(3)),
        );
        body.id = RegionId(2);
        let mut step = region(vec![], Exit::Fallthrough);
        step.id = RegionId(3);
        let mut f = function(
            vec![
                Op::Constant {
                    result: ValueId(0),
                    value: 0.0,
                },
                Op::While {
                    condition,
                    body,
                    step: Some(step),
                },
            ],
            Exit::Return(ValueId(0)),
        );
        f.values = vec![Type::Number, Type::Number, Type::Predicate, Type::Number];
        f
    }

    fn do_while_function(body_returns: bool) -> Function {
        let mut body = region(
            vec![Op::Constant {
                result: ValueId(1),
                value: 1.0,
            }],
            if body_returns {
                Exit::Return(ValueId(1))
            } else {
                Exit::Fallthrough
            },
        );
        body.id = RegionId(1);
        let mut condition = region(
            vec![
                Op::Constant {
                    result: ValueId(2),
                    value: 0.0,
                },
                Op::Truthy {
                    result: ValueId(3),
                    input: ValueId(2),
                },
            ],
            Exit::Yield(ValueId(3)),
        );
        condition.id = RegionId(2);
        let mut f = function(
            vec![
                Op::Constant {
                    result: ValueId(0),
                    value: 0.0,
                },
                Op::DoWhile { body, condition },
            ],
            if body_returns {
                Exit::Fallthrough
            } else {
                Exit::Return(ValueId(0))
            },
        );
        f.values = vec![Type::Number, Type::Number, Type::Number, Type::Predicate];
        f
    }

    #[test]
    fn do_while_return_coverage_follows_its_body() {
        verify(&do_while_function(true)).unwrap();
        verify(&do_while_function(false)).unwrap();
        let mut f = do_while_function(false);
        verify(&f).unwrap();
        f.body.exit = Exit::Fallthrough;
        assert!(verify(&f).unwrap_err().contains("every structural path"));
        let mut f = do_while_function(true);
        f.body.operations.push(Operation {
            span: Span::new(4, 8),
            kind: Op::Constant {
                result: ValueId(4),
                value: 0.0,
            },
        });
        f.values.push(Type::Number);
        assert!(verify(&f).unwrap_err().contains("after guaranteed return"));
    }

    #[test]
    fn do_while_checks_its_test_even_when_the_body_returns() {
        let mut f = do_while_function(true);
        let Op::DoWhile { condition, .. } = &mut f.body.operations[1].kind else {
            panic!()
        };
        condition.operations.push(Operation {
            span: Span::new(4, 8),
            kind: Op::Load {
                result: ValueId(4),
                slot: SlotId(99),
            },
        });
        f.values.push(Type::Number);
        assert!(verify(&f).unwrap_err().contains("slot"));
        let mut f = do_while_function(true);
        let Op::DoWhile { condition, .. } = &mut f.body.operations[1].kind else {
            panic!()
        };
        condition.exit = Exit::Yield(ValueId(2));
        assert!(verify(&f).unwrap_err().contains("value type"));
    }

    #[test]
    fn do_while_regions_cannot_share_new_values() {
        for into_condition in [false, true] {
            let mut f = do_while_function(false);
            verify(&f).unwrap();
            let Op::DoWhile { body, condition } = &mut f.body.operations[1].kind else {
                panic!()
            };
            let (target, input) = if into_condition {
                (condition, ValueId(1))
            } else {
                (body, ValueId(2))
            };
            target.operations.push(Operation {
                span: Span::new(4, 8),
                kind: Op::Unary {
                    result: ValueId(4),
                    operator: Unary::Plus,
                    input,
                },
            });
            f.values.push(Type::Number);
            assert!(verify(&f).unwrap_err().contains("not visible"));
        }
    }

    #[test]
    fn do_while_regions_cannot_share_new_source_cells() {
        for into_condition in [false, true] {
            let mut f = do_while_function(false);
            verify(&f).unwrap();
            let Op::DoWhile { body, condition } = &mut f.body.operations[1].kind else {
                panic!()
            };
            let (source, target, value) = if into_condition {
                (body, condition, ValueId(1))
            } else {
                (condition, body, ValueId(2))
            };
            f.slots.push(Slot {
                owner: source.id,
                mutable: true,
                parameter: false,
                span: Span::new(4, 8),
            });
            source.operations.push(Operation {
                span: Span::new(4, 8),
                kind: Op::Initialize {
                    slot: SlotId(2),
                    value,
                },
            });
            target.operations.push(Operation {
                span: Span::new(4, 8),
                kind: Op::Load {
                    result: ValueId(4),
                    slot: SlotId(2),
                },
            });
            f.values.push(Type::Number);
            assert!(verify(&f).unwrap_err().contains("owner"));
        }
    }

    #[test]
    fn loop_step_is_verified_even_when_the_body_returns() {
        let mut f = loop_with_step();
        verify(&f).unwrap();
        let Op::While {
            step: Some(step), ..
        } = &mut f.body.operations[1].kind
        else {
            panic!()
        };
        step.operations.push(Operation {
            span: Span::new(4, 8),
            kind: Op::Load {
                result: ValueId(4),
                slot: SlotId(99),
            },
        });
        f.values.push(Type::Number);
        assert!(verify(&f).unwrap_err().contains("slot"));
    }

    #[test]
    fn loop_body_values_and_cells_cannot_escape_into_the_step() {
        let mut f = loop_with_step();
        let Op::While {
            step: Some(step), ..
        } = &mut f.body.operations[1].kind
        else {
            panic!()
        };
        step.operations.push(Operation {
            span: Span::new(4, 8),
            kind: Op::Unary {
                result: ValueId(4),
                operator: Unary::Plus,
                input: ValueId(3),
            },
        });
        f.values.push(Type::Number);
        assert!(verify(&f).unwrap_err().contains("not visible"));
        let mut f = loop_with_step();
        let Op::While {
            body,
            step: Some(step),
            ..
        } = &mut f.body.operations[1].kind
        else {
            panic!()
        };
        body.operations.push(Operation {
            span: Span::new(4, 8),
            kind: Op::Initialize {
                slot: SlotId(2),
                value: ValueId(3),
            },
        });
        f.slots.push(Slot {
            owner: body.id,
            mutable: true,
            parameter: false,
            span: Span::new(4, 8),
        });
        step.operations.push(Operation {
            span: Span::new(4, 8),
            kind: Op::Load {
                result: ValueId(4),
                slot: SlotId(2),
            },
        });
        f.values.push(Type::Number);
        assert!(verify(&f).unwrap_err().contains("owner"));
    }

    #[test]
    fn loop_step_values_cannot_escape_into_the_condition_or_body() {
        for into_condition in [false, true] {
            let mut f = loop_with_step();
            let Op::While {
                condition,
                body,
                step: Some(step),
                ..
            } = &mut f.body.operations[1].kind
            else {
                panic!()
            };
            step.operations.push(Operation {
                span: Span::new(4, 8),
                kind: Op::Constant {
                    result: ValueId(4),
                    value: 1.0,
                },
            });
            f.values.push(Type::Number);
            let use_value = Operation {
                span: Span::new(4, 8),
                kind: Op::Unary {
                    result: ValueId(5),
                    operator: Unary::Plus,
                    input: ValueId(4),
                },
            };
            f.values.push(Type::Number);
            if into_condition {
                condition.operations.push(use_value);
            } else {
                body.operations.push(use_value);
            }
            assert!(verify(&f).unwrap_err().contains("not visible"));
        }
    }
    #[test]
    fn rejects_forward_value_use_and_function_fallthrough() {
        let forward = function(vec![], Exit::Return(ValueId(0)));
        assert!(verify(&forward).unwrap_err().contains("not visible"));
        let fallthrough = function(vec![], Exit::Fallthrough);
        assert!(verify(&fallthrough).unwrap_err().contains("return"));
    }
    #[test]
    fn rejects_uninitialized_source_load() {
        let mut f = function(
            vec![Op::Load {
                result: ValueId(0),
                slot: SlotId(2),
            }],
            Exit::Return(ValueId(0)),
        );
        f.slots.push(Slot {
            owner: RegionId(0),
            mutable: true,
            parameter: false,
            span: Span::new(1, 2),
        });
        assert!(verify(&f).unwrap_err().contains("not initialized"));
    }

    #[test]
    fn rejects_boolean_literals_as_numeric_definitions_returns_or_stores() {
        let mut f = function(
            vec![Op::Boolean {
                result: ValueId(0),
                value: true,
            }],
            Exit::Return(ValueId(0)),
        );
        assert!(verify(&f).unwrap_err().contains("type"));
        f.values[0] = Type::Predicate;
        assert!(verify(&f).unwrap_err().contains("type"));
        f.values.push(Type::Number);
        f.body.operations.push(Operation {
            span: Span::new(12, 15),
            kind: Op::Constant {
                result: ValueId(1),
                value: 1.0,
            },
        });
        f.body.operations.push(Operation {
            span: Span::new(12, 15),
            kind: Op::Store {
                slot: SlotId(0),
                value: ValueId(0),
            },
        });
        f.body.exit = Exit::Return(ValueId(1));
        assert!(verify(&f).unwrap_err().contains("type"));
    }

    #[test]
    fn logical_not_requires_a_visible_predicate_input_and_predicate_result() {
        let mut f = function(
            vec![
                Op::Boolean {
                    result: ValueId(0),
                    value: true,
                },
                Op::Not {
                    result: ValueId(1),
                    input: ValueId(0),
                },
                Op::Constant {
                    result: ValueId(2),
                    value: 1.0,
                },
            ],
            Exit::Return(ValueId(2)),
        );
        f.values = vec![Type::Predicate, Type::Predicate, Type::Number];
        verify(&f).unwrap();
        f.values[1] = Type::Number;
        assert!(verify(&f).unwrap_err().contains("type"));
        f.values[1] = Type::Predicate;
        f.body.operations[0].kind = Op::Constant {
            result: ValueId(0),
            value: 1.0,
        };
        f.values[0] = Type::Number;
        assert!(verify(&f).unwrap_err().contains("type"));
        f.body.operations[1].kind = Op::Not {
            result: ValueId(1),
            input: ValueId(1),
        };
        assert!(verify(&f).unwrap_err().contains("not visible"));
        f.body.operations[1].kind = Op::Not {
            result: ValueId(1),
            input: ValueId(99),
        };
        assert!(verify(&f).is_err());
        f.body.operations[1].kind = Op::Not {
            result: ValueId(99),
            input: ValueId(0),
        };
        f.values[0] = Type::Predicate;
        f.body.operations[0].kind = Op::Boolean {
            result: ValueId(0),
            value: false,
        };
        assert!(verify(&f).is_err());
    }

    #[test]
    fn accepts_numeric_return_and_initialized_parameters() {
        let f = function(
            vec![Op::Load {
                result: ValueId(0),
                slot: SlotId(0),
            }],
            Exit::Return(ValueId(0)),
        );
        verify(&f).unwrap();
    }

    #[test]
    fn rejects_wrong_types_invalid_ids_and_duplicate_definitions() {
        let mut f = function(
            vec![Op::Load {
                result: ValueId(0),
                slot: SlotId(0),
            }],
            Exit::Return(ValueId(0)),
        );
        f.values[0] = Type::Predicate;
        assert!(verify(&f).unwrap_err().contains("type"));
        f.values[0] = Type::Number;
        f.body.operations[0].kind = Op::Load {
            result: ValueId(0),
            slot: SlotId(99),
        };
        assert!(verify(&f).unwrap_err().contains("slot"));
        f.body.operations[0].kind = Op::Constant {
            result: ValueId(99),
            value: 0.0,
        };
        assert!(verify(&f).unwrap_err().contains("value"));
        f.body.operations[0].kind = Op::Constant {
            result: ValueId(0),
            value: 0.0,
        };
        f.body.operations.push(Operation {
            span: Span::new(12, 15),
            kind: Op::Load {
                result: ValueId(0),
                slot: SlotId(0),
            },
        });
        assert!(verify(&f).unwrap_err().contains("defined twice"));
    }

    #[test]
    fn rejects_branch_and_condition_values_escaping_their_regions() {
        for while_region in [false, true] {
            let mut child = region(
                vec![Op::Constant {
                    result: ValueId(0),
                    value: 1.0,
                }],
                Exit::Fallthrough,
            );
            child.id = RegionId(1);
            let mut condition = region(
                vec![Op::Truthy {
                    result: ValueId(1),
                    input: ValueId(0),
                }],
                Exit::Yield(ValueId(1)),
            );
            condition.id = RegionId(2);
            let nested = if while_region {
                condition.operations.insert(
                    0,
                    Operation {
                        span: Span::new(12, 15),
                        kind: Op::Constant {
                            result: ValueId(0),
                            value: 1.0,
                        },
                    },
                );
                child.operations.clear();
                Op::While {
                    condition,
                    body: child,
                    step: None,
                }
            } else {
                Op::Block(child)
            };
            let mut f = function(vec![nested], Exit::Return(ValueId(0)));
            f.values = if while_region {
                vec![Type::Number, Type::Predicate]
            } else {
                vec![Type::Number]
            };
            assert!(verify(&f).unwrap_err().contains("not visible"));
        }
    }

    #[test]
    fn rejects_const_reinitialization_store_and_foreign_slot_ownership() {
        let mut f = function(
            vec![
                Op::Constant {
                    result: ValueId(0),
                    value: 2.0,
                },
                Op::Initialize {
                    slot: SlotId(2),
                    value: ValueId(0),
                },
                Op::Initialize {
                    slot: SlotId(2),
                    value: ValueId(0),
                },
            ],
            Exit::Return(ValueId(0)),
        );
        f.slots.push(Slot {
            owner: RegionId(0),
            mutable: false,
            parameter: false,
            span: Span::new(1, 2),
        });
        assert!(verify(&f).unwrap_err().contains("initialized twice"));
        f.body.operations[2].kind = Op::Store {
            slot: SlotId(2),
            value: ValueId(0),
        };
        assert!(verify(&f).unwrap_err().contains("immutable"));
        f.slots[2].owner = RegionId(3);
        assert!(verify(&f).unwrap_err().contains("owner"));
    }

    #[test]
    fn rejects_invalid_yield_return_and_statement_exits() {
        let f = function(
            vec![Op::Constant {
                result: ValueId(0),
                value: 2.0,
            }],
            Exit::Yield(ValueId(0)),
        );
        assert!(verify(&f).unwrap_err().contains("exit"));
        let mut child = region(vec![], Exit::Yield(ValueId(0)));
        child.id = RegionId(1);
        let f = function(
            vec![
                Op::Constant {
                    result: ValueId(0),
                    value: 2.0,
                },
                Op::Block(child),
            ],
            Exit::Return(ValueId(0)),
        );
        assert!(verify(&f).unwrap_err().contains("exit"));
    }

    #[test]
    fn rejects_missing_initialization_even_for_unused_owned_slots() {
        let mut f = function(
            vec![Op::Constant {
                result: ValueId(0),
                value: 1.0,
            }],
            Exit::Return(ValueId(0)),
        );
        f.slots.push(Slot {
            owner: RegionId(0),
            mutable: false,
            parameter: false,
            span: Span::new(4, 7),
        });
        assert!(verify(&f).unwrap_err().contains("not initialized"));
    }

    fn value_if() -> Function {
        let mut left = region(
            vec![Op::Constant {
                result: ValueId(2),
                value: 2.0,
            }],
            Exit::Yield(ValueId(2)),
        );
        left.id = RegionId(1);
        let mut right = region(
            vec![Op::Constant {
                result: ValueId(3),
                value: 3.0,
            }],
            Exit::Yield(ValueId(3)),
        );
        right.id = RegionId(2);
        let mut f = function(
            vec![
                Op::Load {
                    result: ValueId(0),
                    slot: SlotId(0),
                },
                Op::Truthy {
                    result: ValueId(1),
                    input: ValueId(0),
                },
                Op::If {
                    result: Some(ValueId(4)),
                    condition: ValueId(1),
                    consequent: left,
                    alternate: right,
                },
            ],
            Exit::Return(ValueId(4)),
        );
        f.values = vec![
            Type::Number,
            Type::Predicate,
            Type::Number,
            Type::Number,
            Type::Number,
        ];
        f
    }

    fn predicate_if() -> Function {
        let mut f = value_if();
        let Op::If {
            consequent,
            alternate,
            ..
        } = &mut f.body.operations[2].kind
        else {
            panic!()
        };
        consequent.operations[0].kind = Op::Boolean {
            result: ValueId(2),
            value: true,
        };
        alternate.operations[0].kind = Op::Boolean {
            result: ValueId(3),
            value: false,
        };
        f.values[2..].fill(Type::Predicate);
        f.body.exit = Exit::Return(ValueId(0));
        f
    }

    #[test]
    fn if_can_join_predicates_without_making_them_numeric() {
        let mut f = predicate_if();
        verify(&f).unwrap();
        f.body.exit = Exit::Return(ValueId(4));
        assert!(verify(&f).unwrap_err().contains("type"));
        f.body.exit = Exit::Return(ValueId(0));
        f.body.operations.push(Operation {
            span: Span::new(4, 8),
            kind: Op::Store {
                slot: SlotId(0),
                value: ValueId(4),
            },
        });
        assert!(verify(&f).unwrap_err().contains("type"));
    }

    #[test]
    fn predicate_join_checks_result_arms_and_visibility() {
        let mut f = predicate_if();
        verify(&f).unwrap();
        let Op::If { result, .. } = &mut f.body.operations[2].kind else {
            panic!()
        };
        *result = Some(ValueId(99));
        assert!(verify(&f).is_err());
        let mut f = predicate_if();
        f.values[4] = Type::Number;
        assert!(verify(&f).unwrap_err().contains("type"));
        let mut f = predicate_if();
        let Op::If { alternate, .. } = &mut f.body.operations[2].kind else {
            panic!()
        };
        alternate.exit = Exit::Yield(ValueId(0));
        assert!(verify(&f).unwrap_err().contains("type"));
        let mut f = predicate_if();
        let Op::If { result, .. } = &mut f.body.operations[2].kind else {
            panic!()
        };
        *result = Some(ValueId(1));
        assert!(verify(&f).unwrap_err().contains("defined twice"));
        let mut f = predicate_if();
        let Op::If { consequent, .. } = &mut f.body.operations[2].kind else {
            panic!()
        };
        consequent.exit = Exit::Yield(ValueId(4));
        assert!(verify(&f).unwrap_err().contains("not visible"));
        let mut f = predicate_if();
        let Op::If { alternate, .. } = &mut f.body.operations[2].kind else {
            panic!()
        };
        alternate.exit = Exit::Yield(ValueId(2));
        assert!(verify(&f).unwrap_err().contains("not visible"));
    }

    #[test]
    fn only_explicit_if_result_escapes_and_both_arms_must_yield() {
        let mut f = value_if();
        verify(&f).unwrap();
        f.body.exit = Exit::Return(ValueId(2));
        assert!(verify(&f).unwrap_err().contains("not visible"));
        f.body.exit = Exit::Return(ValueId(4));
        let Op::If { alternate, .. } = &mut f.body.operations[2].kind else {
            panic!()
        };
        alternate.exit = Exit::Fallthrough;
        assert!(verify(&f).unwrap_err().contains("exit"));
    }

    #[test]
    fn branch_values_are_invisible_to_siblings_and_slots_to_parents() {
        let mut f = value_if();
        let Op::If { alternate, .. } = &mut f.body.operations[2].kind else {
            panic!()
        };
        alternate.exit = Exit::Yield(ValueId(2));
        assert!(verify(&f).unwrap_err().contains("not visible"));
        let mut f = value_if();
        f.slots.push(Slot {
            owner: RegionId(1),
            mutable: true,
            parameter: false,
            span: Span::new(4, 7),
        });
        let Op::If { consequent, .. } = &mut f.body.operations[2].kind else {
            panic!()
        };
        consequent.operations.push(Operation {
            span: Span::new(4, 7),
            kind: Op::Initialize {
                slot: SlotId(2),
                value: ValueId(2),
            },
        });
        f.values.push(Type::Number);
        f.body.operations.push(Operation {
            span: Span::new(4, 7),
            kind: Op::Load {
                result: ValueId(5),
                slot: SlotId(2),
            },
        });
        assert!(verify(&f).unwrap_err().contains("owner"));
    }
}
