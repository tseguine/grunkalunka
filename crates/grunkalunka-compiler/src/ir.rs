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
    },
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Exit {
    Fallthrough,
    Yield(ValueId),
    Return(ValueId),
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
    ) -> Result<bool, String> {
        if !self.regions.insert(region.id) {
            return Err(invalid(region.span, "region ID is not unique"));
        }
        ancestors.push(region.id);
        let mut visible = outer_values.clone();
        let mut initialized = outer_cells.clone();
        let mut returns = false;
        for operation in &region.operations {
            let span = operation.span;
            if returns {
                return Err(invalid(span, "operation after guaranteed return"));
            }
            match &operation.kind {
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
                    returns = self.region(
                        child,
                        Contract::Statement,
                        &visible,
                        &initialized,
                        ancestors,
                    )?;
                }
                Op::If {
                    result,
                    condition,
                    consequent,
                    alternate,
                } => {
                    self.value(*condition, Type::Predicate, &visible, span)?;
                    let contract = if result.is_some() {
                        Contract::Yield(Type::Number)
                    } else {
                        Contract::Statement
                    };
                    let left =
                        self.region(consequent, contract, &visible, &initialized, ancestors)?;
                    let right =
                        self.region(alternate, contract, &visible, &initialized, ancestors)?;
                    if let Some(result) = result {
                        self.define(*result, Type::Number, &mut visible, span)?;
                    }
                    returns = left && right;
                }
                Op::While { condition, body } => {
                    self.region(
                        condition,
                        Contract::Yield(Type::Predicate),
                        &visible,
                        &initialized,
                        ancestors,
                    )?;
                    self.region(body, Contract::Statement, &visible, &initialized, ancestors)?;
                    // A source while may execute zero times, even if its body returns.
                }
            }
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
            (Contract::Yield(ty), Exit::Yield(value)) if !returns => {
                self.value(value, ty, &visible, region.span)?
            }
            (Contract::Statement | Contract::Function, Exit::Return(value)) if !returns => {
                self.value(value, Type::Number, &visible, region.span)?;
                returns = true;
            }
            (Contract::Statement | Contract::Function, Exit::Fallthrough) => {}
            _ => return Err(invalid(region.span, "invalid region exit contract")),
        }
        if matches!(contract, Contract::Function) && !returns {
            return Err(invalid(
                region.span,
                "function must return a number on every structural path",
            ));
        }
        ancestors.pop();
        Ok(returns)
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
