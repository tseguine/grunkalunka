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
pub(crate) struct FunctionId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Type {
    Number,
    Predicate,
    General,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Unary {
    Abs,
    Sqrt,
    Floor,
    Ceil,
    Trunc,
    Plus,
    Negate,
    BitNot,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum Binary {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
    ShiftRightUnsigned,
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

#[derive(Clone, Debug)]
pub(crate) struct Slot {
    pub ty: Type,
    pub owner: RegionId,
    pub mutable: bool,
    pub parameter: bool,
    pub span: Span,
}

#[derive(Debug)]
pub(crate) struct Function {
    pub parameter_count: usize,
    pub slots: Vec<Slot>,
    pub values: Vec<Type>,
    pub body: Region,
}

#[derive(Debug)]
pub(crate) struct Module {
    // Entry first; all signatures are available before any body is verified.
    pub functions: Vec<Function>,
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

#[derive(Clone, Copy, Debug)]
pub(crate) enum Nullish {
    Undefined,
    Null,
}

#[derive(Debug)]
pub(crate) enum Op {
    InlineCall {
        result: ValueId,
        body: Region,
    },
    Await {
        result: ValueId,
        input: ValueId,
        rejected: bool,
    },
    PrimitiveStrictEqual {
        result: ValueId,
        left: ValueId,
        right: ValueId,
        negated: bool,
    },
    PrimitiveLooseEqual {
        result: ValueId,
        left: ValueId,
        right: ValueId,
        negated: bool,
    },
    Nullish {
        result: ValueId,
        value: Nullish,
    },
    Pack {
        result: ValueId,
        input: ValueId,
    },
    Call {
        result: ValueId,
        function: FunctionId,
        arguments: Vec<ValueId>,
    },
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
    NumberFromPredicate {
        result: ValueId,
        input: ValueId,
    },
    Truthy {
        result: ValueId,
        input: ValueId,
    },
    GeneralTruthy {
        result: ValueId,
        input: ValueId,
    },
    NumberFromGeneral {
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
    Continue,
}

/// Possible source completions. Only normal paths reach a sequential suffix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Completion {
    pub normal: bool,
    pub returns: bool,
    pub breaks: bool,
    pub continues: bool,
}

impl Completion {
    pub const NORMAL: Self = Self {
        normal: true,
        returns: false,
        breaks: false,
        continues: false,
    };
    pub const RETURN: Self = Self {
        normal: false,
        returns: true,
        breaks: false,
        continues: false,
    };
    pub const BREAK: Self = Self {
        normal: false,
        returns: false,
        breaks: true,
        continues: false,
    };
    pub const CONTINUE: Self = Self {
        normal: false,
        returns: false,
        breaks: false,
        continues: true,
    };

    pub fn union(self, other: Self) -> Self {
        Self {
            normal: self.normal || other.normal,
            returns: self.returns || other.returns,
            breaks: self.breaks || other.breaks,
            continues: self.continues || other.continues,
        }
    }

    pub fn then(self, other: Self) -> Self {
        Self {
            normal: self.normal && other.normal,
            returns: self.returns || (self.normal && other.returns),
            breaks: self.breaks || (self.normal && other.breaks),
            continues: self.continues || (self.normal && other.continues),
        }
    }

    pub fn pretest_loop(self) -> Self {
        Self {
            normal: true,
            returns: self.returns,
            breaks: false,
            continues: false,
        }
    }

    pub fn do_loop(self) -> Self {
        Self {
            normal: self.normal || self.breaks || self.continues,
            returns: self.returns,
            breaks: false,
            continues: false,
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
    signatures: &'a [usize],
    return_type: Type,
    entry_type: Type,
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
        allow_loop_control: bool,
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
                Op::InlineCall { result, body } => {
                    let outer_return = self.return_type;
                    self.return_type = Type::Number;
                    let checked = self.region(
                        body,
                        Contract::Function,
                        &visible,
                        &initialized,
                        ancestors,
                        false,
                    );
                    self.return_type = outer_return;
                    checked?;
                    self.define(*result, Type::Number, &mut visible, span)?;
                }
                Op::Await { result, input, .. } => {
                    self.value(*input, Type::Number, &visible, span)?;
                    self.define(*result, Type::Number, &mut visible, span)?;
                }
                Op::Nullish { result, .. } => {
                    self.define(*result, Type::General, &mut visible, span)?;
                }
                Op::Call {
                    result,
                    function,
                    arguments,
                } => {
                    let count = self
                        .signatures
                        .get(function.0)
                        .ok_or_else(|| invalid(span, "invalid callee function ID"))?;
                    if arguments.len() != *count {
                        return Err(invalid(
                            span,
                            "call argument count does not match signature",
                        ));
                    }
                    for argument in arguments {
                        self.value(*argument, Type::Number, &visible, span)?;
                    }
                    let ty = if function.0 == 0 {
                        self.entry_type
                    } else {
                        Type::Number
                    };
                    self.define(*result, ty, &mut visible, span)?;
                }
                Op::Pack { result, input } => {
                    let ty = *self
                        .function
                        .values
                        .get(input.0)
                        .ok_or_else(|| invalid(span, "invalid pack input"))?;
                    if ty == Type::General {
                        return Err(invalid(span, "pack requires a scalar input"));
                    }
                    self.value(*input, ty, &visible, span)?;
                    self.define(*result, Type::General, &mut visible, span)?;
                }
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
                    let ty = self.slot(*slot, ancestors, span)?.ty;
                    if !initialized.contains(slot) {
                        return Err(invalid(span, "slot is not initialized"));
                    }
                    self.define(*result, ty, &mut visible, span)?;
                }
                Op::Initialize { slot, value } => {
                    let cell = self.slot(*slot, ancestors, span)?;
                    self.value(*value, cell.ty, &visible, span)?;
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
                    let cell = self.slot(*slot, ancestors, span)?;
                    self.value(*value, cell.ty, &visible, span)?;
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
                Op::PrimitiveStrictEqual {
                    result,
                    left,
                    right,
                    ..
                }
                | Op::PrimitiveLooseEqual {
                    result,
                    left,
                    right,
                    ..
                } => {
                    self.value(*left, Type::General, &visible, span)?;
                    self.value(*right, Type::General, &visible, span)?;
                    self.define(*result, Type::Predicate, &mut visible, span)?;
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
                Op::NumberFromPredicate { result, input } => {
                    self.value(*input, Type::Predicate, &visible, span)?;
                    self.define(*result, Type::Number, &mut visible, span)?;
                }
                Op::Truthy { result, input } => {
                    self.value(*input, Type::Number, &visible, span)?;
                    self.define(*result, Type::Predicate, &mut visible, span)?;
                }
                Op::GeneralTruthy { result, input } => {
                    self.value(*input, Type::General, &visible, span)?;
                    self.define(*result, Type::Predicate, &mut visible, span)?;
                }
                Op::NumberFromGeneral { result, input } => {
                    self.value(*input, Type::General, &visible, span)?;
                    self.define(*result, Type::Number, &mut visible, span)?;
                }
                Op::Block(child) => {
                    next = self.region(
                        child,
                        Contract::Statement,
                        &visible,
                        &initialized,
                        ancestors,
                        allow_loop_control,
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
                        allow_loop_control && result.is_none(),
                    )?;
                    let right = self.region(
                        alternate,
                        contract,
                        &visible,
                        &initialized,
                        ancestors,
                        allow_loop_control && result.is_none(),
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
                self.value(value, self.return_type, &visible, region.span)?;
                completion = completion.then(Completion::RETURN);
            }
            (Contract::Statement, Exit::Break) if completion.normal && allow_loop_control => {
                completion = completion.then(Completion::BREAK);
            }
            (Contract::Statement, Exit::Continue) if completion.normal && allow_loop_control => {
                completion = completion.then(Completion::CONTINUE);
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

pub(crate) fn verify_module(module: &Module) -> Result<(), String> {
    verify_module_with_entry_type(module, Type::Number)
}

pub(crate) fn verify_module_with_entry_type(
    module: &Module,
    entry_type: Type,
) -> Result<(), String> {
    let entry = module
        .functions
        .first()
        .ok_or("internal IR: missing entry function")?;
    if entry.parameter_count != 2 {
        return Err(invalid(
            entry.body.span,
            "entry must have two numeric parameters",
        ));
    }
    let signatures: Vec<_> = module
        .functions
        .iter()
        .map(|function| function.parameter_count)
        .collect();
    for (index, function) in module.functions.iter().enumerate() {
        verify_function(
            function,
            &signatures,
            if index == 0 { entry_type } else { Type::Number },
            entry_type,
        )?;
    }
    Ok(())
}

#[cfg(test)]
fn verify(function: &Function) -> Result<(), String> {
    verify_function(
        function,
        &[function.parameter_count],
        Type::Number,
        Type::Number,
    )
}

fn verify_function(
    function: &Function,
    signatures: &[usize],
    return_type: Type,
    entry_type: Type,
) -> Result<(), String> {
    if function.parameter_count > 1000 {
        return Err(invalid(
            function.body.span,
            "at most 1000 function parameters are admitted",
        ));
    }
    let mut initialized = BTreeSet::new();
    for (index, slot) in function.slots.iter().enumerate() {
        if slot.ty == Type::General && slot.parameter {
            return Err(invalid(
                slot.span,
                "General source slots cannot be parameters",
            ));
        }
        if slot.parameter && slot.ty != Type::Number {
            return Err(invalid(slot.span, "parameter slots must be Number"));
        }
        if slot.parameter != (index < function.parameter_count)
            || (slot.parameter && (slot.owner != function.body.id || !slot.mutable))
        {
            return Err(invalid(
                slot.span,
                "mutable root parameter slots must match the signature",
            ));
        }
        if slot.parameter {
            initialized.insert(SlotId(index));
        }
    }
    if initialized.len() != function.parameter_count {
        return Err(invalid(
            function.body.span,
            "initialized parameters must match the signature",
        ));
    }
    let mut verifier = Verifier {
        function,
        signatures,
        return_type,
        entry_type,
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

    fn calling_module() -> Module {
        let mut caller = function(
            vec![
                Op::Constant {
                    result: ValueId(0),
                    value: 5.0,
                },
                Op::Constant {
                    result: ValueId(1),
                    value: 3.0,
                },
                Op::Call {
                    result: ValueId(2),
                    function: FunctionId(1),
                    arguments: vec![ValueId(0), ValueId(1)],
                },
            ],
            Exit::Return(ValueId(2)),
        );
        caller.values = vec![Type::Number; 3];
        let callee = function(
            vec![Op::Load {
                result: ValueId(0),
                slot: SlotId(0),
            }],
            Exit::Return(ValueId(0)),
        );
        Module {
            functions: vec![caller, callee],
        }
    }

    #[test]
    fn calls_check_callee_arity_and_arguments_before_defining_the_result() {
        verify_module(&calling_module()).unwrap();
        for (callee, arguments, expected) in [
            (FunctionId(2), vec![ValueId(0), ValueId(1)], "callee"),
            (FunctionId(1), vec![ValueId(0)], "argument count"),
            (FunctionId(1), vec![ValueId(2), ValueId(1)], "not visible"),
        ] {
            let mut module = calling_module();
            module.functions[0].body.operations[2].kind = Op::Call {
                result: ValueId(2),
                function: callee,
                arguments,
            };
            assert!(verify_module(&module).unwrap_err().contains(expected));
        }
    }

    #[test]
    fn calls_require_numeric_arguments_and_result() {
        for predicate_argument in [true, false] {
            let mut module = calling_module();
            if predicate_argument {
                module.functions[0].values[1] = Type::Predicate;
                module.functions[0].body.operations[1].kind = Op::Boolean {
                    result: ValueId(1),
                    value: true,
                };
            } else {
                module.functions[0].values[2] = Type::Predicate;
            }
            assert!(verify_module(&module).unwrap_err().contains("type"));
        }
    }

    #[test]
    fn calls_cannot_read_a_child_region_or_another_functions_values() {
        let mut module = calling_module();
        let mut child = region(
            vec![Op::Constant {
                result: ValueId(0),
                value: 5.0,
            }],
            Exit::Fallthrough,
        );
        child.id = RegionId(1);
        module.functions[0].body.operations[0].kind = Op::Block(child);
        assert!(verify_module(&module).unwrap_err().contains("not visible"));
        let mut module = calling_module();
        module.functions[0].body.operations.drain(0..2);
        assert!(verify_module(&module).unwrap_err().contains("not visible"));
    }

    #[test]
    fn module_signatures_and_each_function_have_independent_parameter_and_loop_state() {
        let mut module = calling_module();
        module.functions[1].parameter_count = 1;
        let Op::Call { arguments, .. } = &mut module.functions[0].body.operations[2].kind else {
            unreachable!()
        };
        arguments.truncate(1);
        assert!(verify_module(&module).unwrap_err().contains("signature"));
        let mut module = calling_module();
        module.functions[1].body.exit = Exit::Continue;
        assert!(
            verify_module(&module)
                .unwrap_err()
                .contains("exit contract")
        );
        let mut module = calling_module();
        module.functions[0].parameter_count = 1;
        assert!(verify_module(&module).unwrap_err().contains("entry"));
        assert!(
            verify_module(&Module {
                functions: Vec::new()
            })
            .is_err()
        );
    }

    #[test]
    fn binary_operations_require_numeric_operands_and_result() {
        for operator in [
            Binary::Remainder,
            Binary::BitAnd,
            Binary::BitOr,
            Binary::BitXor,
            Binary::ShiftLeft,
            Binary::ShiftRight,
            Binary::ShiftRightUnsigned,
        ] {
            for types in [
                [Type::Number, Type::Number, Type::Number],
                [Type::Predicate, Type::Number, Type::Number],
                [Type::Number, Type::Predicate, Type::Number],
                [Type::Number, Type::Number, Type::Predicate],
            ] {
                let mut operations: Vec<_> = types[..2]
                    .iter()
                    .enumerate()
                    .map(|(id, ty)| {
                        if *ty == Type::Number {
                            Op::Constant {
                                result: ValueId(id),
                                value: 5.0,
                            }
                        } else {
                            Op::Boolean {
                                result: ValueId(id),
                                value: true,
                            }
                        }
                    })
                    .collect();
                operations.push(Op::Binary {
                    result: ValueId(2),
                    operator,
                    left: ValueId(0),
                    right: ValueId(1),
                });
                let mut f = function(operations, Exit::Return(ValueId(2)));
                f.values = types.to_vec();
                if types.iter().all(|ty| *ty == Type::Number) {
                    verify(&f).unwrap();
                } else {
                    assert!(verify(&f).unwrap_err().contains("type"));
                }
            }
        }
    }

    #[test]
    fn number_from_predicate_enforces_types_visibility_and_single_definition() {
        for (input_type, result_type) in [
            (Type::Predicate, Type::Number),
            (Type::Number, Type::Number),
            (Type::Predicate, Type::Predicate),
        ] {
            let input = if input_type == Type::Predicate {
                Op::Boolean {
                    result: ValueId(0),
                    value: true,
                }
            } else {
                Op::Constant {
                    result: ValueId(0),
                    value: 1.0,
                }
            };
            let mut f = function(
                vec![
                    input,
                    Op::NumberFromPredicate {
                        input: ValueId(0),
                        result: ValueId(1),
                    },
                    // Return another Number so a wrong bridge result type cannot
                    // fail only at the function's independent return contract.
                    Op::Constant {
                        result: ValueId(2),
                        value: 7.0,
                    },
                ],
                Exit::Return(ValueId(2)),
            );
            f.values = vec![input_type, result_type, Type::Number];
            assert_eq!(
                verify(&f).is_ok(),
                input_type == Type::Predicate && result_type == Type::Number
            );
        }
        for input in [ValueId(0), ValueId(9)] {
            let mut f = function(
                vec![Op::NumberFromPredicate {
                    input,
                    result: ValueId(1),
                }],
                Exit::Return(ValueId(1)),
            );
            f.values = vec![Type::Predicate, Type::Number];
            assert!(verify(&f).is_err());
        }
        let mut f = function(
            vec![
                Op::Boolean {
                    result: ValueId(0),
                    value: true,
                },
                Op::NumberFromPredicate {
                    input: ValueId(0),
                    result: ValueId(1),
                },
                Op::NumberFromPredicate {
                    input: ValueId(0),
                    result: ValueId(1),
                },
            ],
            Exit::Return(ValueId(1)),
        );
        f.values = vec![Type::Predicate, Type::Number];
        assert!(verify(&f).unwrap_err().contains("defined twice"));
        let mut child = region(
            vec![Op::Boolean {
                result: ValueId(0),
                value: true,
            }],
            Exit::Fallthrough,
        );
        child.id = RegionId(1);
        let mut f = function(
            vec![
                Op::Block(child),
                Op::NumberFromPredicate {
                    input: ValueId(0),
                    result: ValueId(1),
                },
            ],
            Exit::Return(ValueId(1)),
        );
        f.values = vec![Type::Predicate, Type::Number];
        assert!(verify(&f).unwrap_err().contains("not visible"));
    }

    #[test]
    fn bit_not_requires_numeric_input_and_result() {
        for (input_type, result_type) in [
            (Type::Number, Type::Number),
            (Type::Predicate, Type::Number),
            (Type::Number, Type::Predicate),
        ] {
            let input = if input_type == Type::Number {
                Op::Load {
                    result: ValueId(0),
                    slot: SlotId(0),
                }
            } else {
                Op::Boolean {
                    result: ValueId(0),
                    value: true,
                }
            };
            let mut f = function(
                vec![
                    input,
                    Op::Unary {
                        result: ValueId(1),
                        operator: Unary::BitNot,
                        input: ValueId(0),
                    },
                ],
                Exit::Return(ValueId(1)),
            );
            f.values = vec![input_type, result_type];
            if input_type == Type::Number && result_type == Type::Number {
                verify(&f).unwrap();
            } else {
                assert!(verify(&f).unwrap_err().contains("type"));
            }
        }
    }

    #[test]
    fn continue_alternatives_reach_the_nearest_loop_only() {
        assert_eq!(
            Completion::CONTINUE.then(Completion::RETURN),
            Completion::CONTINUE
        );
        let mixed = Completion::CONTINUE
            .union(Completion::NORMAL)
            .then(Completion::RETURN);
        assert_eq!(mixed, Completion::CONTINUE.union(Completion::RETURN));
        assert_eq!(
            mixed.do_loop(),
            Completion::NORMAL.union(Completion::RETURN)
        );
        assert_eq!(Completion::CONTINUE.pretest_loop(), Completion::NORMAL);
        let mut f = break_loop();
        let Op::While { body, .. } = &mut f.body.operations[1].kind else {
            panic!()
        };
        body.exit = Exit::Continue;
        verify(&f).unwrap();
        verify(&inside_outer_loop(f)).unwrap();
        let mut outside = function(vec![], Exit::Continue);
        outside.values.clear();
        assert!(verify(&outside).unwrap_err().contains("exit contract"));
    }

    #[test]
    fn continue_cannot_hide_in_a_value_yielding_if() {
        for ty in [Type::Number, Type::Predicate] {
            let mut consequent = region(vec![], Exit::Continue);
            consequent.id = RegionId(3);
            let mut alternate = region(
                vec![],
                Exit::Yield(ValueId(if ty == Type::Number { 0 } else { 1 })),
            );
            alternate.id = RegionId(4);
            let mut body = region(
                vec![Op::If {
                    result: Some(ValueId(2)),
                    condition: ValueId(1),
                    consequent,
                    alternate,
                }],
                Exit::Fallthrough,
            );
            body.id = RegionId(2);
            let mut condition = region(vec![], Exit::Yield(ValueId(1)));
            condition.id = RegionId(1);
            let mut f = function(
                vec![
                    Op::Constant {
                        result: ValueId(0),
                        value: 7.0,
                    },
                    Op::Boolean {
                        result: ValueId(1),
                        value: false,
                    },
                    Op::While {
                        condition,
                        body,
                        step: None,
                    },
                ],
                Exit::Return(ValueId(0)),
            );
            f.values = vec![Type::Number, Type::Predicate, ty];
            assert!(verify(&f).unwrap_err().contains("exit contract"));
        }
    }

    #[test]
    fn continue_rejects_an_ir_operation_after_an_abrupt_block() {
        let mut f = break_loop();
        let mut child = region(vec![], Exit::Continue);
        child.id = RegionId(5);
        let Op::While { body, .. } = &mut f.body.operations[1].kind else {
            panic!()
        };
        body.exit = Exit::Fallthrough;
        body.operations = region(
            vec![
                Op::Block(child),
                Op::Load {
                    result: ValueId(2),
                    slot: SlotId(0),
                },
            ],
            Exit::Fallthrough,
        )
        .operations;
        f.values.push(Type::Number);
        assert!(
            verify(&f)
                .unwrap_err()
                .contains("operation after guaranteed abrupt")
        );
    }

    #[test]
    fn continue_cannot_hide_in_a_do_test_even_after_return() {
        for in_if in [false, true] {
            let mut f = break_loop();
            let Op::While {
                mut condition,
                mut body,
                ..
            } = f.body.operations.pop().unwrap().kind
            else {
                panic!()
            };
            body.exit = Exit::Return(ValueId(0));
            let mut bad = region(vec![], Exit::Continue);
            bad.id = RegionId(3);
            let nested = if in_if {
                let mut alternate = region(vec![], Exit::Fallthrough);
                alternate.id = RegionId(4);
                Op::If {
                    result: None,
                    condition: ValueId(1),
                    consequent: bad,
                    alternate,
                }
            } else {
                Op::Block(bad)
            };
            condition.operations.push(Operation {
                span: condition.span,
                kind: nested,
            });
            f.body.operations.push(Operation {
                span: f.body.span,
                kind: Op::DoWhile { body, condition },
            });
            f.body.exit = Exit::Fallthrough;
            assert!(verify(&f).unwrap_err().contains("exit contract"));
        }
    }

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
    fn loop_headers_reject_nested_block_and_if_exits() {
        for exit in [Exit::Break, Exit::Continue] {
            for in_step in [false, true] {
                for in_if in [false, true] {
                    let mut f = inside_outer_loop(break_loop());
                    let mut bad = region(vec![], exit);
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
            parameter_count: 2,
            slots: (0..2)
                .map(|_| Slot {
                    ty: Type::Number,
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
                ty: Type::Number,
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
            ty: Type::Number,
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
            ty: Type::Number,
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
            ty: Type::Number,
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
            ty: Type::Number,
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
            ty: Type::Number,
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

    #[test]
    fn predicate_slots_verify_init_load_store() {
        let mut f = function(
            vec![
                Op::Boolean {
                    result: ValueId(0),
                    value: true,
                },
                Op::Initialize {
                    slot: SlotId(2),
                    value: ValueId(0),
                },
                Op::Load {
                    result: ValueId(1),
                    slot: SlotId(2),
                },
                Op::Boolean {
                    result: ValueId(2),
                    value: false,
                },
                Op::Store {
                    slot: SlotId(2),
                    value: ValueId(2),
                },
                Op::NumberFromPredicate {
                    result: ValueId(3),
                    input: ValueId(1),
                },
            ],
            Exit::Return(ValueId(3)),
        );
        f.values = vec![
            Type::Predicate,
            Type::Predicate,
            Type::Predicate,
            Type::Number,
        ];
        f.slots.push(Slot {
            ty: Type::Predicate,
            owner: RegionId(0),
            mutable: true,
            parameter: false,
            span: Span::new(0, 1),
        });
        verify(&f).unwrap();
    }

    #[test]
    fn scalar_slots_reject_each_type_mismatch_and_preserve_cell_guards() {
        for ty in [Type::Number, Type::Predicate] {
            for operation in ["initialize", "load", "store"] {
                let correct = ValueId(usize::from(ty == Type::Predicate));
                let wrong = ValueId(1 - correct.0);
                let mut f = function(
                    vec![
                        Op::Constant {
                            result: ValueId(0),
                            value: 1.0,
                        },
                        Op::Boolean {
                            result: ValueId(1),
                            value: true,
                        },
                        Op::Initialize {
                            slot: SlotId(2),
                            value: correct,
                        },
                    ],
                    Exit::Return(ValueId(0)),
                );
                f.slots.push(Slot {
                    ty,
                    owner: RegionId(0),
                    mutable: true,
                    parameter: false,
                    span: Span::new(0, 1),
                });
                f.values = vec![Type::Number, Type::Predicate];
                if operation == "load" {
                    f.values.push(if ty == Type::Number {
                        Type::Predicate
                    } else {
                        Type::Number
                    });
                }
                match operation {
                    "initialize" => {
                        f.body.operations[2].kind = Op::Initialize {
                            slot: SlotId(2),
                            value: wrong,
                        }
                    }
                    "load" => f.body.operations.push(Operation {
                        span: f.body.span,
                        kind: Op::Load {
                            result: ValueId(2),
                            slot: SlotId(2),
                        },
                    }),
                    _ => f.body.operations.push(Operation {
                        span: f.body.span,
                        kind: Op::Store {
                            slot: SlotId(2),
                            value: wrong,
                        },
                    }),
                }
                assert!(
                    verify(&f).unwrap_err().contains("type"),
                    "{ty:?} {operation}"
                );
            }
        }
        for (case, expected) in [
            ("load-before-init", "initialized"),
            ("store-before-init", "initialized"),
            ("double-init", "twice"),
            ("const-store", "immutable"),
            ("invalid-slot", "slot"),
            ("invalid-value", "value"),
            ("invisible-slot", "slot owner is outside the lexical region"),
            ("wrong-entry-init", "lexical entry"),
            ("invalid-owner", "owner"),
            ("invisible-value", "not visible"),
            ("duplicate-value", "defined twice"),
        ] {
            let mut f = function(
                vec![
                    Op::Boolean {
                        result: ValueId(0),
                        value: true,
                    },
                    Op::NumberFromPredicate {
                        result: ValueId(1),
                        input: ValueId(0),
                    },
                    Op::Initialize {
                        slot: SlotId(2),
                        value: ValueId(0),
                    },
                    Op::Load {
                        result: ValueId(2),
                        slot: SlotId(2),
                    },
                ],
                Exit::Return(ValueId(1)),
            );
            f.values = vec![Type::Predicate, Type::Number, Type::Predicate];
            f.slots.push(Slot {
                ty: Type::Predicate,
                owner: RegionId(0),
                mutable: true,
                parameter: false,
                span: Span::new(0, 1),
            });
            let initialize = || Op::Initialize {
                slot: SlotId(2),
                value: ValueId(0),
            };
            let store = |value| Op::Store {
                slot: SlotId(2),
                value,
            };
            let extra = match case {
                "load-before-init" => {
                    f.body.operations.remove(2);
                    None
                }
                "store-before-init" => {
                    f.body.operations[2].kind = store(ValueId(0));
                    None
                }
                "double-init" => Some(initialize()),
                "const-store" => {
                    f.slots[2].mutable = false;
                    Some(store(ValueId(0)))
                }
                "invalid-slot" => Some(Op::Load {
                    result: ValueId(2),
                    slot: SlotId(99),
                }),
                "invalid-value" => Some(store(ValueId(99))),
                "invisible-slot" => {
                    let mut child = region(vec![initialize()], Exit::Fallthrough);
                    child.id = RegionId(1);
                    f.slots[2].owner = child.id;
                    f.body.operations[2].kind = Op::Block(child);
                    None
                }
                "wrong-entry-init" => {
                    let mut child = region(vec![initialize()], Exit::Fallthrough);
                    child.id = RegionId(1);
                    f.body.operations[2].kind = Op::Block(child);
                    None
                }
                "invalid-owner" => {
                    f.slots[2].owner = RegionId(99);
                    f.body.operations.truncate(2);
                    f.values.pop();
                    None
                }
                "invisible-value" => {
                    let mut child = region(
                        vec![Op::Boolean {
                            result: ValueId(3),
                            value: false,
                        }],
                        Exit::Fallthrough,
                    );
                    child.id = RegionId(1);
                    f.values.push(Type::Predicate);
                    f.body.operations.insert(
                        2,
                        Operation {
                            span: f.body.span,
                            kind: Op::Block(child),
                        },
                    );
                    f.body.operations[3].kind = Op::Initialize {
                        slot: SlotId(2),
                        value: ValueId(3),
                    };
                    None
                }
                _ => Some(Op::Load {
                    result: ValueId(0),
                    slot: SlotId(2),
                }),
            };
            if let Some(kind) = extra {
                f.body.operations.push(Operation {
                    span: f.body.span,
                    kind,
                });
            }
            assert!(verify(&f).unwrap_err().contains(expected), "{case}");
        }
    }

    #[test]
    fn entry_and_helper_parameters_stay_number_even_if_unused() {
        for function_index in 0..2 {
            for slot_index in 0..2 {
                let constant = || {
                    function(
                        vec![Op::Constant {
                            result: ValueId(0),
                            value: 1.0,
                        }],
                        Exit::Return(ValueId(0)),
                    )
                };
                let mut module = Module {
                    functions: vec![constant(), constant()],
                };
                verify_module(&module).unwrap();
                module.functions[function_index].slots[slot_index].ty = Type::Predicate;
                assert!(
                    verify_module(&module)
                        .unwrap_err()
                        .contains("parameter slots must be Number")
                );
            }
        }
    }

    #[test]
    fn mixed_scalar_slot_emission_preserves_snapshots_branches_and_loop_entry() {
        let load = |result, slot| Op::Load {
            result: ValueId(result),
            slot: SlotId(slot),
        };
        let store = |slot, value| Op::Store {
            slot: SlotId(slot),
            value: ValueId(value),
        };
        let binary = |result, operator, left, right| Op::Binary {
            result: ValueId(result),
            operator,
            left: ValueId(left),
            right: ValueId(right),
        };
        let mut condition = region(
            vec![
                load(1, 0),
                Op::Truthy {
                    result: ValueId(2),
                    input: ValueId(1),
                },
            ],
            Exit::Yield(ValueId(2)),
        );
        condition.id = RegionId(1);
        let mut consequent = region(vec![store(2, 8)], Exit::Fallthrough);
        consequent.id = RegionId(3);
        let mut alternate = region(vec![store(2, 3)], Exit::Fallthrough);
        alternate.id = RegionId(4);
        let mut body = region(
            vec![
                Op::Boolean {
                    result: ValueId(3),
                    value: true,
                },
                Op::Initialize {
                    slot: SlotId(2),
                    value: ValueId(3),
                },
                load(4, 2),
                Op::Constant {
                    result: ValueId(5),
                    value: 1.0,
                },
                load(6, 0),
                Op::Compare {
                    result: ValueId(7),
                    operator: Compare::Greater,
                    left: ValueId(6),
                    right: ValueId(5),
                },
                Op::Boolean {
                    result: ValueId(8),
                    value: false,
                },
                Op::If {
                    condition: ValueId(7),
                    consequent,
                    alternate,
                    result: None,
                },
                load(9, 2),
                Op::NumberFromPredicate {
                    result: ValueId(10),
                    input: ValueId(4),
                },
                Op::NumberFromPredicate {
                    result: ValueId(11),
                    input: ValueId(9),
                },
                binary(12, Binary::Add, 10, 11),
                load(13, 3),
                binary(14, Binary::Add, 13, 12),
                store(3, 14),
                binary(15, Binary::Subtract, 6, 5),
                store(0, 15),
            ],
            Exit::Fallthrough,
        );
        body.id = RegionId(2);
        let mut f = function(
            vec![
                Op::Constant {
                    result: ValueId(0),
                    value: 0.0,
                },
                Op::Initialize {
                    slot: SlotId(3),
                    value: ValueId(0),
                },
                Op::While {
                    condition,
                    body,
                    step: None,
                },
                load(16, 3),
            ],
            Exit::Return(ValueId(16)),
        );
        f.values = (0..17)
            .map(|i| {
                if [2, 3, 4, 7, 8, 9].contains(&i) {
                    Type::Predicate
                } else {
                    Type::Number
                }
            })
            .collect();
        for (ty, owner) in [(Type::Predicate, RegionId(2)), (Type::Number, RegionId(0))] {
            f.slots.push(Slot {
                ty,
                owner,
                mutable: true,
                parameter: false,
                span: Span::new(0, 1),
            });
        }
        let wat = crate::emit::emit(&Module { functions: vec![f] }).unwrap();
        assert_eq!(
            wat.lines()
                .filter(|line| line.starts_with("(local"))
                .take(2)
                .collect::<Vec<_>>(),
            ["(local i32)", "(local f64)"]
        );
        let wasm = wat::parse_str(&wat).unwrap();
        if let Some(directory) = std::env::var_os("GRUNKALUNKA_IR_TEST_ARTIFACTS") {
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(
                std::path::Path::new(&directory).join("scalar-slots.wat"),
                wat,
            )
            .unwrap();
            std::fs::write(
                std::path::Path::new(&directory).join("scalar-slots.wasm"),
                wasm,
            )
            .unwrap();
        }
    }
}

#[cfg(test)]
mod primitive_value_tests {
    use super::*;
    fn lower(source: &str) -> Module {
        crate::lower_source_with_entry_type(source, false, Type::General).unwrap()
    }
    #[test]
    fn pack_requires_visible_scalar_input_and_general_result() {
        for (input, output, reverse) in [
            (ValueId(0), Type::General, false),
            (ValueId(99), Type::General, false),
            (ValueId(1), Type::General, false),
            (ValueId(0), Type::Number, false),
            (ValueId(0), Type::General, true),
        ] {
            let mut module =
                crate::lower_source("export function run(a,b) {return a;}", false).unwrap();
            let f = &mut module.functions[0];
            f.values.push(output);
            f.body.operations.push(Operation {
                span: f.body.span,
                kind: Op::Pack {
                    result: ValueId(1),
                    input,
                },
            });
            f.body.exit = Exit::Return(ValueId(1));
            if reverse {
                f.body.operations.reverse();
            }
            assert_eq!(
                verify_module_with_entry_type(&module, Type::General).is_ok(),
                input == ValueId(0) && output == Type::General && !reverse
            );
        }
    }
    #[test]
    fn primitive_returns_and_cells_obey_their_profile() {
        let mut module = lower("export function run(a,b) {return null;}");
        verify_module_with_entry_type(&module, Type::General).unwrap();
        assert!(verify_module(&module).is_err());
        module.functions[0].values[0] = Type::Number;
        assert!(verify_module_with_entry_type(&module, Type::General).is_err());
        let mut module = lower("export function run(a,b) {return a;}");
        module.functions[0].slots[0].ty = Type::General;
        assert!(
            verify_module_with_entry_type(&module, Type::General)
                .unwrap_err()
                .contains("General source slots")
        );
    }
    #[test]
    fn general_if_joins_and_calls_check_result_types() {
        let mut module = lower("export function run(a,b) {return a ? run(0,b) : null;}");
        verify_module_with_entry_type(&module, Type::General).unwrap();
        let f = &mut module.functions[0];
        let Op::If {
            result, consequent, ..
        } = &f.body.operations.last().unwrap().kind
        else {
            panic!("if");
        };
        let call = consequent
            .operations
            .iter()
            .find_map(|op| match op.kind {
                Op::Call { result, .. } => Some(result),
                _ => None,
            })
            .unwrap();
        let joined = result.unwrap();
        f.values[call.0] = Type::Number;
        assert!(verify_module_with_entry_type(&module, Type::General).is_err());
        module.functions[0].values[call.0] = Type::General;
        module.functions[0].values[joined.0] = Type::Number;
        assert!(verify_module_with_entry_type(&module, Type::General).is_err());
        let mut module =
            lower("function h(a,b){return a+b;} export function run(a,b){return h(a,b);}");
        let f = &mut module.functions[0];
        let call = f
            .body
            .operations
            .iter()
            .find_map(|op| match op.kind {
                Op::Call { result, .. } => Some(result),
                _ => None,
            })
            .unwrap();
        f.values[call.0] = Type::General;
        assert!(verify_module_with_entry_type(&module, Type::General).is_err());
    }
}

#[cfg(test)]
mod primitive_strict_equality_tests {
    use super::*;
    fn module() -> Module {
        crate::lower_source_with_entry_type(
            "export function run(a,b) {return null===undefined;}",
            false,
            Type::General,
        )
        .unwrap()
    }
    #[test]
    fn strict_equality_requires_general_operands_and_predicate_result() {
        let original = module();
        verify_module_with_entry_type(&original, Type::General).unwrap();
        let Op::PrimitiveStrictEqual {
            result,
            left,
            right,
            ..
        } = original.functions[0].body.operations[2].kind
        else {
            panic!("comparison")
        };
        for (value, ty) in [
            (left, Type::Number),
            (right, Type::Predicate),
            (result, Type::General),
        ] {
            let mut mutated = module();
            mutated.functions[0].values[value.0] = ty;
            assert!(verify_module_with_entry_type(&mutated, Type::General).is_err());
        }
    }
    #[test]
    fn strict_equality_requires_defined_visible_inputs() {
        for invalid in [ValueId(99), ValueId(3)] {
            let mut mutated = module();
            let Op::PrimitiveStrictEqual { left, .. } =
                &mut mutated.functions[0].body.operations[2].kind
            else {
                panic!("comparison")
            };
            *left = invalid;
            assert!(verify_module_with_entry_type(&mutated, Type::General).is_err());
        }
        let mut reordered = module();
        reordered.functions[0].body.operations.swap(1, 2);
        assert!(verify_module_with_entry_type(&reordered, Type::General).is_err());
    }
}

#[cfg(test)]
mod general_const_tests {
    use super::*;
    fn module() -> Module {
        crate::lower_source_with_entry_type(
            "export function run(a,b) {const value=null; const alias=value; return alias;}",
            false,
            Type::General,
        )
        .unwrap()
    }
    #[test]
    fn general_source_cells_require_initialized_local_ownership() {
        verify_module_with_entry_type(&module(), Type::General).unwrap();
        for kind in 0..5 {
            let mut m = module();
            let f = &mut m.functions[0];
            match kind {
                0 => f.body.operations.push(Operation {
                    span: f.body.span,
                    kind: Op::Store {
                        slot: SlotId(2),
                        value: ValueId(0),
                    },
                }),
                1 => f.slots[2].parameter = true,
                2 => f.slots[2].owner = RegionId(99),
                3 => f.slots[2].ty = Type::Number,
                _ => {
                    f.body.operations.remove(1);
                }
            }
            assert!(verify_module_with_entry_type(&m, Type::General).is_err());
        }
    }
}

#[cfg(test)]
mod general_truthiness_tests {
    use super::*;
    fn module() -> Module {
        crate::lower_source(
            "export function run(a,b) {if(null) return a; return b;}",
            false,
        )
        .unwrap()
    }
    #[test]
    fn general_truthiness_requires_visible_general_input_and_predicate_result() {
        verify_module(&module()).unwrap();
        for change in 0..4 {
            let mut m = module();
            let f = &mut m.functions[0];
            let Op::GeneralTruthy { input, result } = &mut f.body.operations[1].kind else {
                panic!("truthiness")
            };
            match change {
                0 => *input = ValueId(99),
                1 => f.values[input.0] = Type::Number,
                2 => f.values[result.0] = Type::General,
                _ => {
                    f.body.operations.swap(0, 1);
                }
            }
            assert!(verify_module(&m).is_err());
        }
    }
}

#[cfg(test)]
mod mutable_general_tests {
    use super::*;
    fn module() -> Module {
        crate::lower_source_with_entry_type(
            "export function run(a,b) {let value=null; value=a; return value;}",
            false,
            Type::General,
        )
        .unwrap()
    }
    #[test]
    fn mutable_general_store_retains_initialization_type_and_owner_checks() {
        verify_module_with_entry_type(&module(), Type::General).unwrap();
        for change in 0..5 {
            let mut m = module();
            let f = &mut m.functions[0];
            match change {
                0 => f.slots[2].mutable = false,
                1 => f.slots[2].owner = RegionId(99),
                2 => {
                    f.body.operations.remove(1);
                }
                3 => {
                    let Op::Store { value, .. } = &mut f.body.operations[4].kind else {
                        panic!("store")
                    };
                    *value = ValueId(1);
                }
                _ => f.slots[2].parameter = true,
            }
            assert!(verify_module_with_entry_type(&m, Type::General).is_err());
        }
    }
}

#[cfg(test)]
mod general_number_tests {
    use super::*;
    #[test]
    fn number_from_general_requires_visible_general_input_and_number_result() {
        let module =
            crate::lower_source("export function run(a,b) {return +null;}", false).unwrap();
        verify_module(&module).unwrap();
        for change in 0..4 {
            let mut m =
                crate::lower_source("export function run(a,b) {return +null;}", false).unwrap();
            let f = &mut m.functions[0];
            let Op::NumberFromGeneral { input, result } = &mut f.body.operations[1].kind else {
                panic!("conversion")
            };
            match change {
                0 => *input = ValueId(99),
                1 => f.values[input.0] = Type::Number,
                2 => f.values[result.0] = Type::General,
                _ => f.body.operations.swap(0, 1),
            }
            assert!(verify_module(&m).is_err());
        }
    }
}

#[cfg(test)]
mod general_parameter_cell_tests {
    use super::*;
    fn module() -> Module {
        crate::lower_source_with_entry_type(
            "export function run(a,b) {a=null; return a;}",
            false,
            Type::General,
        )
        .unwrap()
    }
    #[test]
    fn parameter_shadow_keeps_number_signature_and_initialized_owned_general_cell() {
        verify_module_with_entry_type(&module(), Type::General).unwrap();
        for change in 0..4 {
            let mut m = module();
            let f = &mut m.functions[0];
            assert_eq!(f.slots[0].ty, Type::Number);
            assert!(f.slots[0].parameter);
            assert_eq!(f.slots[2].ty, Type::General);
            assert!(!f.slots[2].parameter);
            match change {
                0 => {
                    f.body.operations.remove(2);
                }
                1 => f.slots[2].parameter = true,
                2 => f.slots[2].owner = RegionId(99),
                _ => f.slots[0].ty = Type::General,
            }
            assert!(verify_module_with_entry_type(&m, Type::General).is_err());
        }
    }
}

#[cfg(test)]
mod primitive_loose_tests {
    use super::*;
    fn module() -> Module {
        crate::lower_source(
            "export function run(a,b) {return null==undefined?1:0;}",
            false,
        )
        .unwrap()
    }
    #[test]
    fn primitive_loose_equality_requires_visible_general_operands_and_predicate_result() {
        verify_module(&module()).unwrap();
        for change in 0..5 {
            let mut m = module();
            let f = &mut m.functions[0];
            let Op::PrimitiveLooseEqual {
                left,
                right,
                result,
                ..
            } = &mut f.body.operations[2].kind
            else {
                panic!("loose comparison")
            };
            match change {
                0 => *left = ValueId(99),
                1 => *right = *result,
                2 => f.values[left.0] = Type::Number,
                3 => f.values[result.0] = Type::General,
                _ => f.body.operations.swap(1, 2),
            };
            assert!(verify_module(&m).is_err());
        }
    }
}

#[cfg(test)]
mod inline_call_tests {
    use super::*;
    #[test]
    fn inline_returns_are_numeric_complete_and_lexically_scoped() {
        let source = "export function run(a,b){const f=()=>{return a;};return f();}";
        for change in 0..5 {
            let mut module = crate::lower_source(source, false).unwrap();
            verify_module(&module).unwrap();
            let function = &mut module.functions[0];
            let Op::InlineCall { result, body } = &mut function.body.operations[0].kind else {
                panic!("inline call")
            };
            let result = *result;
            let Exit::Return(child) = body.exit else {
                panic!("body return")
            };
            match change {
                0 => body.exit = Exit::Fallthrough,
                1 => body.exit = Exit::Return(result),
                2 => body.exit = Exit::Break,
                3 => function.values[result.0] = Type::General,
                _ => function.body.exit = Exit::Return(child),
            }
            assert!(verify_module(&module).is_err(), "mutation {change}");
        }
    }
}
