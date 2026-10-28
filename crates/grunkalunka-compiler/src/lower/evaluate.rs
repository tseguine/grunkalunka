//! Explicit control for Number/Predicate/General/Function/Promise/Discard evaluation.
//! Requests borrow original AST nodes. Only this cursor owns unfinished regions;
//! continuations own completed children until their parent operation is emitted.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum Mode {
    Number,
    Predicate,
    Discard,
    Arithmetic,
    General,
    Function(usize),
    Promise,
    Pending,
    Settled,
    Awaited(usize),
}

#[derive(Clone, Copy)]
enum Value {
    Number(ValueId),
    Predicate(ValueId),
    General(ValueId),
    Function(ValueId),
    Promise(ValueId),
    Pending(ValueId),
    Settled(ValueId),
}
impl Value {
    fn id(self) -> ValueId {
        match self {
            Self::Number(id)
            | Self::Predicate(id)
            | Self::General(id)
            | Self::Function(id)
            | Self::Pending(id)
            | Self::Promise(id)
            | Self::Settled(id) => id,
        }
    }
    fn general(self) -> ValueId {
        let Self::General(id) = self else {
            unreachable!("General continuation")
        };
        id
    }
    fn promise(self) -> ValueId {
        let Self::Promise(id) = self else {
            unreachable!("Promise continuation")
        };
        id
    }
    fn pending(self) -> ValueId {
        let Self::Pending(id) = self else {
            unreachable!("pending Promise continuation")
        };
        id
    }
    fn number(self) -> ValueId {
        let Self::Number(id) = self else {
            unreachable!("Number continuation")
        };
        id
    }
    fn predicate(self) -> ValueId {
        let Self::Predicate(id) = self else {
            unreachable!("Predicate continuation")
        };
        id
    }
}

struct Regions<'r> {
    root: &'r mut Region,
    children: Vec<Region>,
}

fn call_result<'e, 'a>(
    work: &mut Vec<Work<'e, 'a>>,
    node: &'e Expression<'a>,
    mode: Mode,
    ty: Type,
    number_proven: bool,
) -> Result<(), String> {
    match (ty, mode) {
        (Type::Number, Mode::General) => work.push(Work::Pack(node)),
        (Type::Number, Mode::Predicate) => work.push(Work::Truthy(node)),
        (Type::General, Mode::Predicate) => work.push(Work::GeneralTruthy(node)),
        (Type::General, Mode::Number) if number_proven => {
            work.push(Work::NumberFromGeneral(node));
        }
        (Type::General, Mode::Number) => {
            return Err(unsupported(
                node,
                "primitive closure result requires a value or coercion position",
            ));
        }
        (Type::Number, Mode::Number) | (Type::General, Mode::General) => (),
        (Type::Promise, Mode::Pending) => (),
        (Type::Promise, _) => {
            return Err(unsupported(
                node,
                "pending Promise values require await, storage or object truthiness",
            ));
        }
        (_, Mode::Pending) => {
            return Err(unsupported(
                node,
                "pending Promise position requires a represented pending identity",
            ));
        }
        (Type::FulfilledPromise, Mode::Promise) => (),
        (Type::FulfilledPromise, Mode::Settled) => work.push(Work::PromotePromise(node)),
        (Type::SettledPromise, Mode::Settled) => (),
        (Type::SettledPromise, _) => {
            return Err(unsupported(
                node,
                "rejected Promise call results require an immediate await or synchronous forwarding position",
            ));
        }
        (_, Mode::Settled) => {
            return Err(unsupported(
                node,
                "settled Promise positions require a settled call result",
            ));
        }
        (Type::FulfilledPromise, _) => {
            return Err(unsupported(
                node,
                "fulfilled Promise call results require a Promise value or await position",
            ));
        }
        (_, Mode::Promise) => {
            return Err(unsupported(
                node,
                "Promise positions require fulfilled Promise call results",
            ));
        }
        _ => unreachable!("scalar call result after mode routing"),
    }
    Ok(())
}
impl Regions<'_> {
    fn current(&mut self) -> &mut Region {
        self.children.last_mut().unwrap_or(self.root)
    }
    fn finish(&mut self, value: ValueId) -> Region {
        let mut child = self.children.pop().expect("owned child region");
        child.exit = Exit::Yield(value);
        child
    }
}

#[derive(Clone, Copy)]
enum ClosureEffect {
    Sync,
    Await,
    Start,
}

enum Work<'e, 'a> {
    Eval(&'e Expression<'a>, usize, Mode),
    AwaitOperand {
        node: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
        operand: awaited::Operand<'e, 'a>,
        site: Option<usize>,
    },
    Forget,
    Await(&'e Expression<'a>, bool, Type, Option<usize>),
    AwaitPending(&'e Expression<'a>, Option<usize>),
    PendingTruthy(&'e Expression<'a>),
    AwaitSettled(&'e Expression<'a>, Option<usize>),
    FulfillPromise(&'e Expression<'a>),
    PromotePromise(&'e Expression<'a>),
    PromiseValue(&'e Expression<'a>),
    PromiseTruthy(&'e Expression<'a>),
    FinishAwaitCall(&'e Expression<'a>, Type, Option<usize>),
    VoidResult(&'e Expression<'a>),
    Pack(&'e Expression<'a>),
    Convert(&'e Expression<'a>),
    Truthy(&'e Expression<'a>),
    GeneralTruthy(&'e Expression<'a>),
    NumberFromGeneral(&'e Expression<'a>),
    Not(&'e Expression<'a>),
    Unary(&'e Expression<'a>, Unary),
    BinaryRight {
        node: &'e Expression<'a>,
        right: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
        operator: Binary,
    },
    Binary {
        node: &'e Expression<'a>,
        left: ValueId,
        operator: Binary,
    },
    PrimitiveCompareRight {
        node: &'e Expression<'a>,
        right: &'e Expression<'a>,
        depth: usize,
        loose: bool,
        negated: bool,
    },
    PrimitiveCompare {
        node: &'e Expression<'a>,
        left: ValueId,
        loose: bool,
        negated: bool,
    },
    CompareRight {
        node: &'e Expression<'a>,
        right: &'e Expression<'a>,
        depth: usize,
        booleans: bool,
        operator: Compare,
    },
    Compare {
        node: &'e Expression<'a>,
        left: ValueId,
        operator: Compare,
    },
    LogicalAssign {
        node: &'e Expression<'a>,
        assignment: &'e oxc_ast::ast::AssignmentExpression<'a>,
        slot: SlotId,
        old: ValueId,
        condition: ValueId,
    },
    StoreGeneral {
        node: &'e Expression<'a>,
        slot: SlotId,
    },
    StoreTyped {
        node: &'e Expression<'a>,
        slot: SlotId,
    },
    Assign {
        node: &'e Expression<'a>,
        slot: SlotId,
        previous: Option<ValueId>,
        operator: Option<Binary>,
    },
    Call {
        node: &'e Expression<'a>,
        call: &'e CallExpression<'a>,
        function: ir::FunctionId,
        index: usize,
        arguments: Vec<ValueId>,
        depth: usize,
    },
    MathCall {
        node: &'e Expression<'a>,
        call: &'e CallExpression<'a>,
        operator: Unary,
        index: usize,
        input: Option<ValueId>,
        depth: usize,
    },
    LocalCall {
        node: &'e Expression<'a>,
        call: &'e CallExpression<'a>,
        closure: closures::LocalClosure,
        index: usize,
        arguments: Vec<ValueId>,
        depth: usize,
        site: Option<usize>,
    },
    ClosureCall {
        node: &'e Expression<'a>,
        call: &'e CallExpression<'a>,
        callee: ValueId,
        effect: ClosureEffect,
        index: usize,
        arguments: Vec<ValueId>,
        depth: usize,
        site: Option<usize>,
    },
    BeginAwaitedClosureCall {
        node: &'e Expression<'a>,
        call: &'e CallExpression<'a>,
        signature: usize,
        depth: usize,
        site: Option<usize>,
    },
    RestoreBindings(Vec<NumericBinding>, bool),
    Sequence {
        remaining: &'e [Expression<'a>],
        last: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
    },
    Condition {
        node: &'e Expression<'a>,
        consequent: &'e Expression<'a>,
        alternate: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
    },
    Consequent {
        node: &'e Expression<'a>,
        condition: ValueId,
        alternate: &'e Expression<'a>,
        depth: usize,
        mode: Mode,
    },
    Alternate {
        node: &'e Expression<'a>,
        condition: ValueId,
        consequent: Region,
        mode: Mode,
    },
    LogicalLeft {
        node: &'e Expression<'a>,
        left: &'e Expression<'a>,
        right: &'e Expression<'a>,
        operator: LogicalOperator,
        depth: usize,
        mode: Mode,
    },
    LogicalRight {
        node: &'e Expression<'a>,
        left_node: &'e Expression<'a>,
        left: Value,
        condition: ValueId,
        operator: LogicalOperator,
        mode: Mode,
    },
}

fn predicate_number<'e, 'a>(work: &mut Vec<Work<'e, 'a>>, node: &'e Expression<'a>, depth: usize) {
    work.push(Work::Convert(node));
    work.push(Work::Eval(node, depth, Mode::Predicate));
}

// Selection has one traversal and one set of region-owning continuations for
// every admitted result kind. Constructor/call semantics remain typed leaves.
fn schedule_selection<'e, 'a>(
    work: &mut Vec<Work<'e, 'a>>,
    node: &'e Expression<'a>,
    depth: usize,
    mode: Mode,
) -> Result<bool, String> {
    match node {
        Expression::SequenceExpression(sequence) => {
            let (last, preceding) = sequence.expressions.split_last().ok_or_else(|| {
                unsupported(sequence.as_ref(), "comma expression must have an operand")
            })?;
            work.push(Work::Sequence {
                remaining: preceding,
                last,
                depth: depth + 1,
                mode,
            });
        }
        Expression::ConditionalExpression(conditional) => {
            work.push(Work::Condition {
                node,
                consequent: &conditional.consequent,
                alternate: &conditional.alternate,
                depth: depth + 1,
                mode,
            });
            work.push(Work::Eval(&conditional.test, depth + 1, Mode::Predicate));
        }
        Expression::LogicalExpression(logical) => {
            work.push(Work::LogicalLeft {
                node,
                left: &logical.left,
                right: &logical.right,
                operator: logical.operator,
                depth: depth + 1,
                mode,
            });
            work.push(Work::Eval(&logical.left, depth + 1, mode));
        }
        _ => return Ok(false),
    }
    Ok(true)
}

impl Lowering<'_, '_> {
    // Logical assignment validates and captures the target before entering its RHS.
    // All RHS work stays in the selected owned child; no old assignment route changes.
    fn schedule_logical_assignment<'e, 'a>(
        &mut self,
        node: &'e Expression<'a>,
        assignment: &'e oxc_ast::ast::AssignmentExpression<'a>,
        depth: usize,
        mode: Mode,
        regions: &mut Regions<'_>,
        work: &mut Vec<Work<'e, 'a>>,
    ) -> Result<(), String> {
        let Some(identifier) = assignment
            .left
            .as_simple_assignment_target()
            .map(|target| target_identifier(target, depth))
            .transpose()?
            .flatten()
        else {
            return Err(unsupported(
                node,
                "assignment target must be an initialized identifier",
            ));
        };
        let binding = numeric_binding(identifier, self.semantic, &self.bindings)?;
        if !binding.mutable {
            return Err(unsupported(
                node,
                "assignment to a const binding is not admitted",
            ));
        }
        let slot = binding.slot;
        let ty = self.slots[slot.0].ty;
        if let Type::Closure(signature) = ty
            && !matches!(mode, Mode::Function(expected) if expected == signature)
        {
            return Err(unsupported(
                node,
                "function logical assignment requires an unchanged function signature",
            ));
        }
        let general = ty == Type::General;
        if ty == Type::FulfilledPromise && !matches!(mode, Mode::Promise) {
            return Err(unsupported(
                node,
                "Promise logical assignments require a Promise value position",
            ));
        }
        if general && !matches!(mode, Mode::General) {
            return Err(unsupported(
                node,
                "General logical assignments require a primitive value position",
            ));
        }
        let old = self.value(ty);
        Self::push(regions.current(), node, Op::Load { result: old, slot });
        let condition = self.value(Type::Predicate);
        let test = if assignment.operator == AssignmentOperator::LogicalNullish {
            if general {
                Op::GeneralIsNullish {
                    result: condition,
                    input: old,
                }
            } else {
                Op::Boolean {
                    result: condition,
                    value: false,
                }
            }
        } else if matches!(
            ty,
            Type::Closure(_) | Type::FulfilledPromise | Type::Promise
        ) {
            Op::Boolean {
                result: condition,
                value: true,
            }
        } else if general {
            Op::GeneralTruthy {
                result: condition,
                input: old,
            }
        } else {
            Op::Truthy {
                result: condition,
                input: old,
            }
        };
        Self::push(regions.current(), node, test);
        regions.children.push(self.region(&assignment.right));
        work.push(Work::LogicalAssign {
            node,
            assignment,
            slot,
            old,
            condition,
        });
        work.push(Work::Eval(
            &assignment.right,
            depth + 1,
            match ty {
                Type::Closure(signature) => Mode::Function(signature),
                Type::General => Mode::General,
                Type::FulfilledPromise => Mode::Promise,
                Type::Promise => Mode::Pending,
                _ => Mode::Number,
            },
        ));
        Ok(())
    }

    pub(super) fn local_scalar_argument_mode(
        &self,
        expression: &Expression<'_>,
        depth: usize,
    ) -> Mode {
        if self.fulfilled_promise_shape(expression, depth) {
            Mode::Promise
        } else if self.numeric_result_shape(expression, depth) {
            Mode::Number
        } else {
            Mode::General
        }
    }

    pub(super) fn evaluate<'e, 'a>(
        &mut self,
        expression: &'e Expression<'a>,
        region: &mut Region,
        depth: usize,
        mode: Mode,
    ) -> Result<Option<ValueId>, String> {
        #[cfg(test)]
        let _entry = crate::evaluator_tests::entry();
        let mut regions = Regions {
            root: region,
            children: Vec::new(),
        };
        let mut work = vec![Work::Eval(expression, depth, mode)];
        let mut values: Vec<Value> = Vec::new();
        while !work.is_empty() {
            #[cfg(test)]
            crate::evaluator_tests::pending(work.len());
            match work.pop().expect("pending work") {
                Work::AwaitOperand {
                    node,
                    depth,
                    mode,
                    operand,
                    site,
                } => {
                    if let Some(argument) = operand.argument
                        && self.composed_await_selection(argument, operand.depth)
                        // Identity evaluation keeps the selected Promise while
                        // surplus intrinsic arguments finish before its await.
                        && (operand.extras.is_empty()
                            || !self.pending_promise_shape(argument, operand.depth))
                    {
                        if operand.rejected || !operand.extras.is_empty() {
                            return Err(unsupported(
                                node,
                                "selected pending await operands require direct fulfillment without surplus intrinsic arguments",
                            ));
                        }
                        call_result(
                            &mut work,
                            node,
                            mode,
                            Type::General,
                            self.numeric_result_shape(node, depth),
                        )?;
                        work.push(Work::Eval(
                            argument,
                            operand.depth,
                            Mode::Awaited(site.unwrap_or(node.span().start as usize)),
                        ));
                        continue;
                    }
                    let Some(argument) = operand.argument else {
                        if !matches!(mode, Mode::General) {
                            return Err(unsupported(
                                node,
                                "undefined await completions require a primitive value or coercion position",
                            ));
                        }
                        let input = self.undefined_value(node, regions.current());
                        values.push(Value::General(input));
                        work.push(Work::Await(node, operand.rejected, Type::General, site));
                        continue;
                    };
                    let argument_depth = operand.depth;
                    let rejected = operand.rejected;
                    let known_async = closures::unwrapped_at(argument, argument_depth)
                        .and_then(|(node, depth)| match node {
                            Expression::CallExpression(call) => {
                                self.closure_target(&call.callee, depth + 1)
                            }
                            _ => None,
                        })
                        .is_some_and(|closure| {
                            let arrow = self.arrow(closure.arrow);
                            arrow.r#async
                                && (operand.extras.is_empty()
                                    || !awaited::eager_async_arrow(arrow, self.semantic))
                        });
                    if !known_async
                        && operand.extras.is_empty()
                        && let Some(private) = awaited::private_call(argument, argument_depth)
                        && let Some(signature) =
                            self.function_arity(&private.call.callee, private.depth + 1)
                        && self.closure_signatures.borrow().is_async(signature)
                    {
                        if rejected || !operand.extras.is_empty() {
                            return Err(unsupported(
                                node,
                                "pending async-call operands require a direct await without Promise-valued rejection or surplus intrinsic arguments",
                            ));
                        }
                        call_result(
                            &mut work,
                            node,
                            mode,
                            Type::General,
                            self.numeric_result_shape(node, depth),
                        )?;
                        work.push(Work::BeginAwaitedClosureCall {
                            node: argument,
                            call: private.call,
                            signature,
                            depth: private.depth + 1,
                            site,
                        });
                        for (prefix, depth) in private.prefixes.into_iter().rev() {
                            work.push(Work::Eval(prefix, depth, Mode::Discard));
                        }
                        continue;
                    }
                    if (!known_async || !operand.extras.is_empty())
                        && self.pending_promise_shape(argument, argument_depth)
                    {
                        if rejected {
                            return Err(unsupported(
                                node,
                                "pending Promise intrinsic adoption or object rejection is not admitted",
                            ));
                        }
                        call_result(
                            &mut work,
                            node,
                            mode,
                            Type::General,
                            self.numeric_result_shape(node, depth),
                        )?;
                        work.push(Work::AwaitPending(node, site));
                        for extra in operand.extras.iter().rev() {
                            work.push(Work::Eval(
                                extra.as_expression().expect("validated intrinsic argument"),
                                argument_depth,
                                Mode::Discard,
                            ));
                        }
                        work.push(Work::Eval(argument, argument_depth, Mode::Pending));
                        continue;
                    }
                    if !known_async && self.fulfilled_promise_shape(argument, argument_depth) {
                        if rejected {
                            return Err(unsupported(
                                node,
                                "Promise-valued rejection reasons are not admitted",
                            ));
                        }
                        if matches!(mode, Mode::Number) && self.numeric_result_shape(node, depth) {
                            work.push(Work::NumberFromGeneral(node));
                        } else if !matches!(mode, Mode::General) {
                            return Err(unsupported(
                                node,
                                "stored Promise completions require a primitive value or coercion position",
                            ));
                        }
                        work.push(Work::Await(node, false, Type::General, site));
                        for extra in operand.extras.iter().rev() {
                            work.push(Work::Eval(
                                extra.as_expression().expect("validated ordinary argument"),
                                argument_depth,
                                Mode::Discard,
                            ));
                        }
                        work.push(Work::PromiseValue(node));
                        work.push(Work::Eval(argument, argument_depth, Mode::Promise));
                        continue;
                    }
                    if !known_async
                        && self.settled_promise_shape_in(
                            argument,
                            argument_depth,
                            &std::collections::BTreeMap::new(),
                            0,
                        )
                    {
                        if rejected || !operand.extras.is_empty() {
                            return Err(unsupported(
                                node,
                                "settled async-call operands require a direct await without Promise-valued rejection or surplus intrinsic arguments",
                            ));
                        }
                        call_result(&mut work, node, mode, Type::General, false)?;
                        work.push(Work::AwaitSettled(node, site));
                        work.push(Work::Eval(argument, argument_depth, Mode::Settled));
                        continue;
                    }
                    let ty = if matches!(mode, Mode::General) {
                        Type::General
                    } else {
                        Type::Number
                    };
                    work.push(Work::Await(node, rejected, ty, site));
                    if let Some((Expression::CallExpression(call), call_depth)) =
                        closures::unwrapped_at(argument, argument_depth)
                        && let Some(mut closure) = self.closure_target(&call.callee, call_depth + 1)
                        && self.arrow(closure.arrow).r#async
                    {
                        if !operand.extras.is_empty() {
                            return Err(unsupported(
                                node,
                                "async-call Promise payloads with surplus arguments require stored pending Promises",
                            ));
                        }
                        if rejected {
                            return Err(unsupported(
                                node,
                                "a rejected async-call result would require a Promise-valued reason",
                            ));
                        }
                        if !closure.checked {
                            self.check_local_closure(&mut closure)?;
                            self.capture_cells(&closure, regions.current());
                        }
                        self.check_local_call(call, &closure)?;
                        // AwaitCall produces a General completion pair even
                        // when the normal body result is known Number.
                        work.pop().expect("scheduled caller await");
                        if closure.result == Type::Number && matches!(mode, Mode::Number) {
                            work.push(Work::NumberFromGeneral(node));
                        } else {
                            call_result(&mut work, node, mode, Type::General, false)?;
                        }
                        work.push(Work::LocalCall {
                            node: argument,
                            call,
                            closure,
                            index: 0,
                            arguments: Vec::new(),
                            depth: call_depth + 1,
                            site,
                        });
                        continue;
                    }
                    for extra in operand.extras.iter().rev() {
                        work.push(Work::Eval(
                            extra.as_expression().expect("validated ordinary argument"),
                            argument_depth,
                            Mode::Discard,
                        ));
                    }
                    work.push(Work::Eval(argument, argument_depth, mode));
                    continue;
                }
                Work::VoidResult(node) => {
                    let result = self.value(Type::General);
                    Self::push(
                        regions.current(),
                        node,
                        Op::Nullish {
                            result,
                            value: ir::Nullish::Undefined,
                        },
                    );
                    values.push(Value::General(result));
                }
                Work::FinishAwaitCall(node, ty, site) => {
                    let mut value = values.pop().expect("async body result").id();
                    let mut body = regions.children.pop().expect("owned async body");
                    if ty == Type::Number {
                        let result = self.value(Type::General);
                        Self::push(
                            &mut body,
                            node,
                            Op::Pack {
                                result,
                                input: value,
                            },
                        );
                        value = result;
                    }
                    body.exit = Exit::Return(value);
                    let result = self.value(Type::General);
                    Self::push_await(
                        regions.current(),
                        node,
                        site,
                        Op::AwaitCall { result, body },
                    );
                    values.push(Value::General(result));
                }
                Work::PromotePromise(node) => {
                    let input = values.pop().expect("fulfilled promotion").promise();
                    let result = self.value(Type::SettledPromise);
                    Self::push(
                        regions.current(),
                        node,
                        Op::PromotePromise { result, input },
                    );
                    values.push(Value::Settled(result));
                }
                Work::FulfillPromise(node) => {
                    let input = values.pop().expect("fulfilled payload").general();
                    let result = self.value(Type::FulfilledPromise);
                    Self::push(
                        regions.current(),
                        node,
                        Op::FulfillPromise { result, input },
                    );
                    values.push(Value::Promise(result));
                }
                Work::PromiseValue(node) => {
                    let input = values.pop().expect("awaited Promise").promise();
                    let result = self.value(Type::General);
                    Self::push(regions.current(), node, Op::PromiseValue { result, input });
                    values.push(Value::General(result));
                }
                Work::AwaitPending(node, site) => {
                    let input = values.pop().expect("awaited pending identity").pending();
                    let result = self.value(Type::General);
                    Self::push_await(
                        regions.current(),
                        node,
                        site,
                        Op::AwaitPromise { result, input },
                    );
                    values.push(Value::General(result));
                }
                Work::PendingTruthy(node) => {
                    values.pop().expect("pending truthiness").pending();
                    let result = self.value(Type::Predicate);
                    Self::push(
                        regions.current(),
                        node,
                        Op::Boolean {
                            result,
                            value: true,
                        },
                    );
                    values.push(Value::Predicate(result));
                }
                Work::PromiseTruthy(node) => {
                    values.pop().expect("Promise truthiness operand").promise();
                    let result = self.value(Type::Predicate);
                    Self::push(
                        regions.current(),
                        node,
                        Op::Boolean {
                            result,
                            value: true,
                        },
                    );
                    values.push(Value::Predicate(result));
                }
                Work::AwaitSettled(node, site) => {
                    let Value::Settled(input) = values.pop().expect("settled await input") else {
                        unreachable!()
                    };
                    let result = self.value(Type::General);
                    Self::push_await(
                        regions.current(),
                        node,
                        site,
                        Op::AwaitSettled { result, input },
                    );
                    values.push(Value::General(result));
                }
                Work::Await(node, rejected, ty, site) => {
                    let input = values.pop().expect("await operand").id();
                    let result = self.value(ty);
                    Self::push_await(
                        regions.current(),
                        node,
                        site,
                        Op::Await {
                            result,
                            input,
                            rejected,
                        },
                    );
                    values.push(if ty == Type::General {
                        Value::General(result)
                    } else {
                        Value::Number(result)
                    });
                }
                Work::RestoreBindings(bindings, source_charging) => {
                    self.bindings = bindings;
                    self.source_charging = source_charging;
                    self.active_local_calls -= 1;
                }
                Work::LocalCall {
                    node,
                    call,
                    closure,
                    index,
                    mut arguments,
                    depth,
                    site,
                } => {
                    // Only declared inputs need values. Surplus expressions use
                    // Discard, which consumes its own temporary scalar result.
                    let parameter_count = self.arrow(closure.arrow).params.items.len();
                    if index <= parameter_count && arguments.len() < index {
                        arguments.push(values.pop().expect("local call argument").id());
                    }
                    if index < call.arguments.len() {
                        let argument = call.arguments[index].as_expression().ok_or_else(|| {
                            unsupported(node, "spread call arguments are not admitted")
                        })?;
                        if index >= parameter_count {
                            work.push(Work::LocalCall {
                                node,
                                call,
                                closure,
                                index: index + 1,
                                arguments,
                                depth,
                                site,
                            });
                            work.push(Work::Eval(argument, depth, Mode::Discard));
                            continue;
                        }
                        let scalar_mode = self.local_scalar_argument_mode(argument, depth);
                        let function = self.function_arity(argument, depth);
                        if let Some(arity) = function {
                            let outer = self.function_result_depth;
                            self.function_result_depth = (self.inline_depth + self.statement_depth)
                                .max(outer)
                                + regions.children.len();
                            let value =
                                self.callback_argument(argument, regions.current(), arity, depth);
                            self.function_result_depth = outer;
                            arguments.push(value?);
                        }
                        work.push(Work::LocalCall {
                            node,
                            call,
                            closure,
                            index: index + 1,
                            arguments,
                            depth,
                            site,
                        });
                        if function.is_none() {
                            work.push(Work::Eval(argument, depth, scalar_mode));
                        }
                    } else {
                        self.charge_closure_expansion(node)?;
                        self.active_local_calls += 1;
                        let arrow = self.arrow(closure.arrow);
                        let previous = std::mem::replace(&mut self.bindings, closure.captures);
                        self.bind_arrow_parameters(arrow, arguments, regions.current())?;
                        let charging = std::mem::replace(&mut self.source_charging, false);
                        work.push(Work::RestoreBindings(previous, charging));
                        if arrow.r#async {
                            regions.children.push(self.region(arrow));
                            work.push(Work::FinishAwaitCall(node, closure.result, site));
                        }
                        if let Some(expression) = arrow.get_expression() {
                            work.push(Work::Eval(
                                expression,
                                0,
                                if closure.result == Type::SettledPromise {
                                    Mode::Settled
                                } else if closure.result == Type::FulfilledPromise {
                                    Mode::Promise
                                } else if closure.result == Type::General {
                                    Mode::General
                                } else {
                                    Mode::Number
                                },
                            ));
                        } else {
                            let depth = self.inline_depth
                                + self.statement_depth
                                + regions.children.len()
                                + 1;
                            let body = self.arrow_block_result(arrow, depth, closure.result)?;
                            let result = self.value(closure.result);
                            Self::push(regions.current(), node, Op::InlineCall { result, body });
                            values.push(if closure.result == Type::SettledPromise {
                                Value::Settled(result)
                            } else if closure.result == Type::FulfilledPromise {
                                Value::Promise(result)
                            } else if closure.result == Type::General {
                                Value::General(result)
                            } else {
                                Value::Number(result)
                            });
                        }
                    }
                }
                Work::BeginAwaitedClosureCall {
                    node,
                    call,
                    signature,
                    depth,
                    site,
                } => {
                    let region_depth = regions.children.len();
                    let callee = self.prepare_closure_call(
                        node,
                        call,
                        signature,
                        regions.current(),
                        depth,
                        region_depth,
                    )?;
                    work.push(Work::ClosureCall {
                        node,
                        call,
                        callee,
                        effect: ClosureEffect::Await,
                        index: 0,
                        arguments: Vec::new(),
                        depth,
                        site,
                    });
                }
                Work::ClosureCall {
                    node,
                    call,
                    callee,
                    effect,
                    index,
                    mut arguments,
                    depth,
                    site,
                } => {
                    let Type::Closure(signature) = self.values[callee.0] else {
                        unreachable!()
                    };
                    let parameter_count = self
                        .closure_signatures
                        .borrow()
                        .parameter_count(signature)
                        .expect("checked signature");
                    if index <= parameter_count && arguments.len() < index {
                        arguments.push(values.pop().expect("runtime closure argument").id());
                    }
                    if index < call.arguments.len() {
                        let argument = call.arguments[index].as_expression().ok_or_else(|| {
                            unsupported(node, "spread call arguments are not admitted")
                        })?;
                        if index >= parameter_count {
                            work.push(Work::ClosureCall {
                                node,
                                call,
                                callee,
                                effect,
                                index: index + 1,
                                arguments,
                                depth,
                                site,
                            });
                            work.push(Work::Eval(argument, depth, Mode::Discard));
                            continue;
                        }
                        let parameter = self
                            .closure_signatures
                            .borrow()
                            .parameter(signature, index)
                            .expect("checked signature");
                        let callback = if let Type::Closure(signature) = parameter {
                            let outer = self.function_result_depth;
                            self.function_result_depth = (self.inline_depth + self.statement_depth)
                                .max(outer)
                                + regions.children.len();
                            let value = self.callback_argument(
                                argument,
                                regions.current(),
                                signature,
                                depth,
                            );
                            self.function_result_depth = outer;
                            arguments.push(value?);
                            true
                        } else {
                            false
                        };
                        work.push(Work::ClosureCall {
                            node,
                            call,
                            callee,
                            effect,
                            index: index + 1,
                            arguments,
                            depth,
                            site,
                        });
                        if !callback {
                            let mode = if parameter == Type::FulfilledPromise {
                                Mode::Promise
                            } else if parameter == Type::General {
                                Mode::General
                            } else {
                                Mode::Number
                            };
                            work.push(Work::Eval(argument, depth, mode));
                        }
                    } else {
                        self.charge_closure_expansion(node)?;
                        let Type::Closure(signature) = self.values[callee.0] else {
                            unreachable!()
                        };
                        let ty = self
                            .closure_signatures
                            .borrow()
                            .result(signature)
                            .expect("checked signature");
                        let ty = if matches!(effect, ClosureEffect::Start) {
                            Type::Promise
                        } else {
                            ty
                        };
                        let result = self.value(ty);
                        Self::push_await(
                            regions.current(),
                            node,
                            site,
                            if matches!(effect, ClosureEffect::Await) {
                                Op::AwaitClosureCall {
                                    result,
                                    callee,
                                    arguments,
                                }
                            } else if matches!(effect, ClosureEffect::Start) {
                                Op::StartAsyncClosure {
                                    result,
                                    callee,
                                    arguments,
                                }
                            } else {
                                Op::ClosureCall {
                                    result,
                                    callee,
                                    arguments,
                                }
                            },
                        );
                        values.push(if ty == Type::Promise {
                            Value::Pending(result)
                        } else if ty == Type::SettledPromise {
                            Value::Settled(result)
                        } else if ty == Type::FulfilledPromise {
                            Value::Promise(result)
                        } else if ty == Type::General {
                            Value::General(result)
                        } else {
                            Value::Number(result)
                        });
                    }
                }
                Work::Forget => {
                    values.pop().expect("discarded result");
                }
                Work::Convert(node) => {
                    let input = values.pop().expect("conversion input").predicate();
                    let result = self.value(Type::Number);
                    Self::push(
                        regions.current(),
                        node,
                        Op::NumberFromPredicate { result, input },
                    );
                    values.push(Value::Number(result));
                }
                Work::Pack(node) => {
                    let input = values.pop().expect("pack input").id();
                    let result = self.value(Type::General);
                    Self::push(regions.current(), node, Op::Pack { result, input });
                    values.push(Value::General(result));
                }
                Work::NumberFromGeneral(node) => {
                    let input = values.pop().expect("General numeric input").general();
                    let result = self.value(Type::Number);
                    Self::push(
                        regions.current(),
                        node,
                        Op::NumberFromGeneral { result, input },
                    );
                    values.push(Value::Number(result));
                }
                Work::GeneralTruthy(node) => {
                    let input = values.pop().expect("General truthiness input").general();
                    let result = self.value(Type::Predicate);
                    Self::push(regions.current(), node, Op::GeneralTruthy { result, input });
                    values.push(Value::Predicate(result));
                }
                Work::Truthy(node) => {
                    let input = values.pop().expect("truthiness input").number();
                    let result = self.value(Type::Predicate);
                    Self::push(regions.current(), node, Op::Truthy { result, input });
                    values.push(Value::Predicate(result));
                }
                Work::Not(node) => {
                    let input = values.pop().expect("not input").predicate();
                    let result = self.value(Type::Predicate);
                    Self::push(regions.current(), node, Op::Not { result, input });
                    values.push(Value::Predicate(result));
                }
                Work::Unary(node, operator) => {
                    let input = values.pop().expect("unary input").number();
                    let result = self.value(Type::Number);
                    Self::push(
                        regions.current(),
                        node,
                        Op::Unary {
                            result,
                            operator,
                            input,
                        },
                    );
                    values.push(Value::Number(result));
                }
                Work::BinaryRight {
                    node,
                    right,
                    depth,
                    mode,
                    operator,
                } => {
                    let left = values.pop().expect("binary left").number();
                    work.push(Work::Binary {
                        node,
                        left,
                        operator,
                    });
                    work.push(Work::Eval(right, depth, mode));
                }
                Work::Binary {
                    node,
                    left,
                    operator,
                } => {
                    let right = values.pop().expect("binary right").number();
                    let result = self.value(Type::Number);
                    Self::push(
                        regions.current(),
                        node,
                        Op::Binary {
                            result,
                            operator,
                            left,
                            right,
                        },
                    );
                    values.push(Value::Number(result));
                }
                Work::PrimitiveCompareRight {
                    node,
                    right,
                    depth,
                    loose,
                    negated,
                } => {
                    let left = values.pop().expect("primitive comparison left").general();
                    work.push(Work::PrimitiveCompare {
                        node,
                        left,
                        loose,
                        negated,
                    });
                    work.push(Work::Eval(right, depth, Mode::General));
                }
                Work::PrimitiveCompare {
                    node,
                    left,
                    loose,
                    negated,
                } => {
                    let right = values.pop().expect("primitive comparison right").general();
                    let result = self.value(Type::Predicate);
                    let operation = if loose {
                        Op::PrimitiveLooseEqual {
                            result,
                            left,
                            right,
                            negated,
                        }
                    } else {
                        Op::PrimitiveStrictEqual {
                            result,
                            left,
                            right,
                            negated,
                        }
                    };
                    Self::push(regions.current(), node, operation);
                    values.push(Value::Predicate(result));
                }
                Work::CompareRight {
                    node,
                    right,
                    depth,
                    booleans,
                    operator,
                } => {
                    let left = values.pop().expect("comparison left").number();
                    work.push(Work::Compare {
                        node,
                        left,
                        operator,
                    });
                    if booleans {
                        predicate_number(&mut work, right, depth);
                    } else {
                        work.push(Work::Eval(
                            right,
                            depth,
                            if matches!(
                                operator,
                                Compare::Less
                                    | Compare::LessEqual
                                    | Compare::Greater
                                    | Compare::GreaterEqual
                            ) {
                                Mode::Arithmetic
                            } else {
                                Mode::Number
                            },
                        ));
                    }
                }
                Work::Compare {
                    node,
                    left,
                    operator,
                } => {
                    let right = values.pop().expect("comparison right").number();
                    let result = self.value(Type::Predicate);
                    Self::push(
                        regions.current(),
                        node,
                        Op::Compare {
                            result,
                            operator,
                            left,
                            right,
                        },
                    );
                    values.push(Value::Predicate(result));
                }
                Work::LogicalAssign {
                    node,
                    assignment,
                    slot,
                    old,
                    condition,
                } => {
                    let ty = self.slots[slot.0].ty;
                    let general = ty == Type::General;
                    let value = values.pop().expect("logical assignment RHS");
                    let value = if matches!(ty, Type::Closure(_)) {
                        let Value::Function(value) = value else {
                            unreachable!("function logical assignment RHS")
                        };
                        value
                    } else if ty == Type::Promise {
                        value.pending()
                    } else if ty == Type::FulfilledPromise {
                        value.promise()
                    } else if general {
                        value.general()
                    } else {
                        value.number()
                    };
                    let mut selected = regions.finish(value);
                    Self::push(&mut selected, node, Op::Store { slot, value });
                    let mut skipped = self.region(&assignment.left);
                    skipped.exit = Exit::Yield(old);
                    let (consequent, alternate) =
                        if assignment.operator != AssignmentOperator::LogicalOr {
                            (selected, skipped)
                        } else {
                            (skipped, selected)
                        };
                    let result = self.value(ty);
                    Self::push(
                        regions.current(),
                        node,
                        Op::If {
                            result: Some(result),
                            condition,
                            consequent,
                            alternate,
                        },
                    );
                    values.push(if matches!(ty, Type::Closure(_)) {
                        Value::Function(result)
                    } else if ty == Type::Promise {
                        Value::Pending(result)
                    } else if ty == Type::FulfilledPromise {
                        Value::Promise(result)
                    } else if general {
                        Value::General(result)
                    } else {
                        Value::Number(result)
                    });
                }
                Work::StoreTyped { node, slot } => {
                    Self::push(
                        regions.current(),
                        node,
                        Op::Store {
                            slot,
                            value: values.last().expect("typed assignment RHS").id(),
                        },
                    );
                }
                Work::StoreGeneral { node, slot } => {
                    let value = values.pop().expect("General assignment RHS");
                    let stored = match value {
                        Value::General(id) => id,
                        Value::Number(input) => {
                            let result = self.value(Type::General);
                            Self::push(regions.current(), node, Op::Pack { result, input });
                            result
                        }
                        Value::Predicate(_)
                        | Value::Function(_)
                        | Value::Pending(_)
                        | Value::Promise(_)
                        | Value::Settled(_) => {
                            unreachable!("typed General assignment RHS")
                        }
                    };
                    Self::push(
                        regions.current(),
                        node,
                        Op::Store {
                            slot,
                            value: stored,
                        },
                    );
                    values.push(value);
                }
                Work::Assign {
                    node,
                    slot,
                    previous,
                    operator,
                } => {
                    let mut value = values.pop().expect("assignment right").number();
                    if let (Some(left), Some(operator)) = (previous, operator) {
                        let left = if self.slots[slot.0].ty == Type::General {
                            let result = self.value(Type::Number);
                            Self::push(
                                regions.current(),
                                node,
                                Op::NumberFromGeneral {
                                    result,
                                    input: left,
                                },
                            );
                            result
                        } else {
                            left
                        };
                        let result = self.value(Type::Number);
                        Self::push(
                            regions.current(),
                            node,
                            Op::Binary {
                                result,
                                operator,
                                left,
                                right: value,
                            },
                        );
                        value = result;
                    }
                    let stored = if self.slots[slot.0].ty == Type::General {
                        let result = self.value(Type::General);
                        Self::push(
                            regions.current(),
                            node,
                            Op::Pack {
                                result,
                                input: value,
                            },
                        );
                        result
                    } else {
                        value
                    };
                    Self::push(
                        regions.current(),
                        node,
                        Op::Store {
                            slot,
                            value: stored,
                        },
                    );
                    values.push(Value::Number(value));
                }
                Work::Call {
                    node,
                    call,
                    function,
                    index,
                    mut arguments,
                    depth,
                } => {
                    let parameter_count = self.functions[function.0].parameter_count;
                    if index != 0 && index <= parameter_count {
                        arguments.push(values.pop().expect("call argument").number());
                    }
                    if index == call.arguments.len() {
                        let ty = self.functions[function.0].return_type;
                        let result = self.value(ty);
                        Self::push(
                            regions.current(),
                            node,
                            Op::Call {
                                result,
                                function,
                                arguments,
                            },
                        );
                        values.push(if ty == Type::General {
                            Value::General(result)
                        } else {
                            Value::Number(result)
                        });
                    } else {
                        // Validate a later spread only after earlier arguments finish.
                        let Some(argument) = call.arguments[index].as_expression() else {
                            return Err(unsupported(
                                node,
                                "spread call arguments are not admitted",
                            ));
                        };
                        work.push(Work::Call {
                            node,
                            call,
                            function,
                            index: index + 1,
                            arguments,
                            depth,
                        });
                        work.push(Work::Eval(
                            argument,
                            depth,
                            if index < parameter_count {
                                Mode::Number
                            } else {
                                Mode::Discard
                            },
                        ));
                    }
                }
                Work::MathCall {
                    node,
                    call,
                    operator,
                    index,
                    mut input,
                    depth,
                } => {
                    if index == 1 {
                        input = Some(values.pop().expect("Math argument").number());
                    }
                    if index == call.arguments.len() {
                        let result = self.value(Type::Number);
                        Self::push(
                            regions.current(),
                            node,
                            Op::Unary {
                                result,
                                operator,
                                input: input.expect("checked Math argument"),
                            },
                        );
                        values.push(Value::Number(result));
                    } else {
                        let argument = call.arguments[index].as_expression().ok_or_else(|| {
                            unsupported(node, "spread call arguments are not admitted")
                        })?;
                        work.push(Work::MathCall {
                            node,
                            call,
                            operator,
                            index: index + 1,
                            input,
                            depth,
                        });
                        work.push(Work::Eval(
                            argument,
                            depth,
                            if index == 0 {
                                Mode::Arithmetic
                            } else {
                                Mode::Discard
                            },
                        ));
                    }
                }
                Work::Sequence {
                    remaining,
                    last,
                    depth,
                    mode,
                } => {
                    if let Some((first, rest)) = remaining.split_first() {
                        work.push(Work::Sequence {
                            remaining: rest,
                            last,
                            depth,
                            mode,
                        });
                        work.push(Work::Eval(first, depth, Mode::Discard));
                    } else {
                        work.push(Work::Eval(last, depth, mode));
                    }
                }
                Work::Condition {
                    node,
                    consequent,
                    alternate,
                    depth,
                    mode,
                } => {
                    let condition = values.pop().expect("conditional test").predicate();
                    regions.children.push(self.region(consequent));
                    work.push(Work::Consequent {
                        node,
                        condition,
                        alternate,
                        depth,
                        mode,
                    });
                    work.push(Work::Eval(consequent, depth, mode));
                }
                Work::Consequent {
                    node,
                    condition,
                    alternate,
                    depth,
                    mode,
                } => {
                    let value = values.pop().expect("consequent result").id();
                    let consequent = regions.finish(value);
                    // Preserve allocation order: complete consequent before alternate.
                    regions.children.push(self.region(alternate));
                    work.push(Work::Alternate {
                        node,
                        condition,
                        consequent,
                        mode,
                    });
                    work.push(Work::Eval(alternate, depth, mode));
                }
                Work::Alternate {
                    node,
                    condition,
                    consequent,
                    mode,
                } => {
                    let value = values.pop().expect("alternate result").id();
                    let alternate = regions.finish(value);
                    let ty = match mode {
                        Mode::General | Mode::Awaited(_) => Type::General,
                        Mode::Predicate => Type::Predicate,
                        Mode::Function(signature) => Type::Closure(signature),
                        Mode::Pending => Type::Promise,
                        Mode::Promise => Type::FulfilledPromise,
                        Mode::Settled => Type::SettledPromise,
                        _ => Type::Number,
                    };
                    let result = self.value(ty);
                    Self::push(
                        regions.current(),
                        node,
                        Op::If {
                            result: Some(result),
                            condition,
                            consequent,
                            alternate,
                        },
                    );
                    values.push(match ty {
                        Type::General => Value::General(result),
                        Type::Predicate => Value::Predicate(result),
                        Type::Number => Value::Number(result),
                        Type::Closure(_) => Value::Function(result),
                        Type::FulfilledPromise => Value::Promise(result),
                        Type::SettledPromise => Value::Settled(result),
                        Type::Promise => Value::Pending(result),
                    });
                }
                Work::LogicalLeft {
                    node,
                    left: left_node,
                    right,
                    operator,
                    depth,
                    mode,
                } => {
                    let left = values.pop().expect("logical left");
                    let condition = match left {
                        Value::General(input) if operator == LogicalOperator::Coalesce => {
                            let nullish = self.value(Type::Predicate);
                            Self::push(
                                regions.current(),
                                node,
                                Op::GeneralIsNullish {
                                    result: nullish,
                                    input,
                                },
                            );
                            let result = self.value(Type::Predicate);
                            Self::push(
                                regions.current(),
                                node,
                                Op::Not {
                                    result,
                                    input: nullish,
                                },
                            );
                            result
                        }
                        Value::Number(_) | Value::Predicate(_)
                            if operator == LogicalOperator::Coalesce =>
                        {
                            let result = self.value(Type::Predicate);
                            Self::push(
                                regions.current(),
                                node,
                                Op::Boolean {
                                    result,
                                    value: true,
                                },
                            );
                            result
                        }
                        Value::General(input) => {
                            let result = self.value(Type::Predicate);
                            Self::push(
                                regions.current(),
                                node,
                                Op::GeneralTruthy { result, input },
                            );
                            result
                        }
                        Value::Predicate(id) => id,
                        Value::Number(input) => {
                            let result = self.value(Type::Predicate);
                            Self::push(regions.current(), node, Op::Truthy { result, input });
                            result
                        }
                        Value::Settled(_) => {
                            return Err(unsupported(
                                node,
                                "settled Promise logical selection requires rejection tracking",
                            ));
                        }
                        Value::Function(_) | Value::Promise(_) | Value::Pending(_) => {
                            // Functions and Promises are truthy and non-nullish,
                            // so || and ?? keep this evaluated left snapshot.
                            let result = self.value(Type::Predicate);
                            Self::push(
                                regions.current(),
                                node,
                                Op::Boolean {
                                    result,
                                    value: true,
                                },
                            );
                            result
                        }
                    };
                    regions.children.push(self.region(right));
                    work.push(Work::LogicalRight {
                        node,
                        left_node,
                        left,
                        condition,
                        operator,
                        mode,
                    });
                    work.push(Work::Eval(right, depth, mode));
                }
                Work::LogicalRight {
                    node,
                    left_node,
                    left,
                    condition,
                    operator,
                    mode,
                } => {
                    let right = values.pop().expect("logical right").id();
                    let selected = regions.finish(right);
                    let mut skipped = self.region(left_node);
                    skipped.exit = Exit::Yield(left.id());
                    let (consequent, alternate) = if operator == LogicalOperator::And {
                        (selected, skipped)
                    } else {
                        (skipped, selected)
                    };
                    let ty = match mode {
                        Mode::General | Mode::Awaited(_) => Type::General,
                        Mode::Predicate => Type::Predicate,
                        Mode::Function(signature) => Type::Closure(signature),
                        Mode::Pending => Type::Promise,
                        Mode::Promise => Type::FulfilledPromise,
                        Mode::Settled => Type::SettledPromise,
                        _ => Type::Number,
                    };
                    let result = self.value(ty);
                    Self::push(
                        regions.current(),
                        node,
                        Op::If {
                            result: Some(result),
                            condition,
                            consequent,
                            alternate,
                        },
                    );
                    values.push(match ty {
                        Type::General => Value::General(result),
                        Type::Predicate => Value::Predicate(result),
                        Type::Number => Value::Number(result),
                        Type::Closure(_) => Value::Function(result),
                        Type::FulfilledPromise => Value::Promise(result),
                        Type::SettledPromise => Value::Settled(result),
                        Type::Promise => Value::Pending(result),
                    });
                }
                Work::Eval(node, depth, Mode::Discard) => {
                    if self.pending_promise_shape(node, depth) {
                        work.push(Work::Forget);
                        work.push(Work::Eval(node, depth, Mode::Pending));
                        continue;
                    }

                    if self.fulfilled_promise_shape(node, depth) {
                        work.push(Work::Forget);
                        work.push(Work::Eval(node, depth, Mode::Promise));
                        continue;
                    }
                    if let Some(arity) = self.function_arity(node, depth) {
                        let outer = self.function_result_depth;
                        self.function_result_depth = (self.inline_depth + self.statement_depth)
                            .max(outer)
                            + regions.children.len();
                        let result = self.function_value(node, regions.current(), arity, depth);
                        self.function_result_depth = outer;
                        result?;
                        continue;
                    }
                    // Routing owns classification before the selected mode's guard.
                    let mode = if self.general_binding_shape(node, depth)
                        || self.general_assignment_target(node, depth).is_some()
                    {
                        Mode::General
                    } else if discarded_boolean_shape(node, depth)
                        || self.operand_boolean_shape(node, depth)
                    {
                        Mode::Predicate
                    } else if self.general_logical_shape(node, depth) {
                        Mode::General
                    } else if self.numeric_result_shape(node, depth) {
                        Mode::Number
                    } else {
                        Mode::General
                    };
                    work.push(Work::Forget);
                    work.push(Work::Eval(node, depth, mode));
                }
                Work::Eval(node, depth, Mode::Arithmetic) => {
                    if self.operand_boolean_shape(node, depth) {
                        predicate_number(&mut work, node, depth);
                    } else if self.numeric_result_shape(node, depth) {
                        work.push(Work::Eval(node, depth, Mode::Number));
                    } else {
                        work.push(Work::NumberFromGeneral(node));
                        work.push(Work::Eval(node, depth, Mode::General));
                    }
                }
                Work::Eval(node, depth, mode) => {
                    if (self.active_block_calls != 0
                        || self.function_result_depth != 0
                        || self.active_function_values != 0)
                        && (self.inline_depth + self.statement_depth)
                            .max(self.function_result_depth)
                            + regions.children.len()
                            > 128
                    {
                        return Err(unsupported(
                            node,
                            if matches!(mode, Mode::Function(_)) {
                                "function-value expansion exceeds 128 aggregate nested regions"
                            } else if self.function_result_depth != 0
                                || self.active_function_values != 0
                            {
                                "local function-value expansion exceeds 128 nested regions"
                            } else {
                                "block-arrow expansion exceeds 128 nested regions"
                            },
                        ));
                    }
                    if self.active_local_calls != 0 && regions.children.len() > 128 {
                        return Err(unsupported(
                            node,
                            "local arrow expansion exceeds 128 nested expression regions",
                        ));
                    }
                    if depth > 128 {
                        return Err(unsupported(
                            node,
                            if matches!(mode, Mode::Predicate) {
                                "predicate nesting exceeds this experiment's limit"
                            } else {
                                "expression nesting exceeds this experiment's limit"
                            },
                        ));
                    }
                    if let Some(inner) = erased_inner(node) {
                        work.push(Work::Eval(inner, depth + 1, mode));
                        continue;
                    }
                    if let Expression::ParenthesizedExpression(wrapper) = node {
                        work.push(Work::Eval(&wrapper.expression, depth + 1, mode));
                        continue;
                    }
                    if matches!(mode, Mode::Predicate) && self.pending_promise_shape(node, depth) {
                        work.push(Work::PendingTruthy(node));
                        work.push(Work::Eval(node, depth, Mode::Pending));
                        continue;
                    }
                    if matches!(mode, Mode::Pending) {
                        if schedule_selection(&mut work, node, depth, mode)? {
                            continue;
                        }
                        if let Expression::Identifier(id) = node {
                            let binding = numeric_binding(id, self.semantic, &self.bindings)?;
                            if self.slots[binding.slot.0].ty != Type::Promise {
                                return Err(unsupported(node, "pending Promise binding required"));
                            }
                            let slot = binding.slot;
                            let result = self.value(Type::Promise);
                            Self::push(regions.current(), node, Op::Load { result, slot });
                            values.push(Value::Pending(result));
                            continue;
                        }
                        if let Expression::AssignmentExpression(assignment) = node {
                            if matches!(
                                assignment.operator,
                                AssignmentOperator::LogicalAnd
                                    | AssignmentOperator::LogicalOr
                                    | AssignmentOperator::LogicalNullish
                            ) {
                                self.schedule_logical_assignment(
                                    node,
                                    assignment,
                                    depth,
                                    mode,
                                    &mut regions,
                                    &mut work,
                                )?;
                                continue;
                            }
                            let id = assignment
                                .left
                                .as_simple_assignment_target()
                                .and_then(|t| target_identifier(t, depth).ok().flatten())
                                .ok_or_else(|| {
                                    unsupported(
                                        node,
                                        "pending Promise assignment requires an identifier",
                                    )
                                })?;
                            let binding = numeric_binding(id, self.semantic, &self.bindings)?;
                            if !binding.mutable
                                || assignment.operator != AssignmentOperator::Assign
                                || self.slots[binding.slot.0].ty != Type::Promise
                            {
                                return Err(unsupported(
                                    node,
                                    "pending Promise assignment requires mutable unchanged identity type",
                                ));
                            }
                            work.push(Work::StoreTyped {
                                node,
                                slot: binding.slot,
                            });
                            work.push(Work::Eval(&assignment.right, depth + 1, Mode::Pending));
                            continue;
                        }
                        if let Expression::CallExpression(call) = node {
                            if let Some(intrinsic) = awaited::intrinsic(call, self.semantic)
                                .map_err(|message| unsupported(node, message))?
                                && !intrinsic.rejected
                                && let Some(argument) = intrinsic.argument
                                && self.pending_promise_shape(argument, depth + 1)
                            {
                                for extra in intrinsic.extras.iter().rev() {
                                    work.push(Work::Eval(
                                        extra
                                            .as_expression()
                                            .expect("validated intrinsic argument"),
                                        depth + 1,
                                        Mode::Discard,
                                    ));
                                }
                                work.push(Work::Eval(argument, depth + 1, Mode::Pending));
                                continue;
                            }
                            let signature = self
                                .function_arity(&call.callee, depth + 1)
                                .ok_or_else(|| {
                                    unsupported(
                                        node,
                                        "pending call requires a represented function",
                                    )
                                })?;
                            let asynchronous = self.closure_signatures.borrow().is_async(signature);
                            if asynchronous && !self.allow_await {
                                return Err(unsupported(
                                    node,
                                    "starting pending async calls requires a coroutine function",
                                ));
                            }
                            if !asynchronous
                                && self.closure_signatures.borrow().result(signature)
                                    != Some(Type::Promise)
                            {
                                return Err(unsupported(
                                    node,
                                    "call does not return a pending Promise identity",
                                ));
                            }
                            let region_depth = regions.children.len();
                            let callee = self.prepare_closure_call(
                                node,
                                call,
                                signature,
                                regions.current(),
                                depth + 1,
                                region_depth,
                            )?;
                            work.push(Work::ClosureCall {
                                node,
                                call,
                                callee,
                                effect: if asynchronous {
                                    ClosureEffect::Start
                                } else {
                                    ClosureEffect::Sync
                                },
                                index: 0,
                                arguments: Vec::new(),
                                depth: depth + 1,
                                site: None,
                            });
                            continue;
                        }
                        return Err(unsupported(
                            node,
                            "expression requires a pending Promise identity",
                        ));
                    }
                    if let Mode::Awaited(site) = mode {
                        if matches!(
                            node,
                            Expression::ConditionalExpression(_)
                                | Expression::SequenceExpression(_)
                        ) && schedule_selection(&mut work, node, depth, mode)?
                        {
                            continue;
                        }
                        let operand = if let Expression::CallExpression(call) = node
                            && let Some(intrinsic) = awaited::intrinsic(call, self.semantic)
                                .map_err(|message| unsupported(node, message))?
                        {
                            awaited::Operand {
                                argument: intrinsic.argument,
                                depth: depth + 1,
                                rejected: intrinsic.rejected,
                                extras: intrinsic.extras,
                            }
                        } else {
                            awaited::Operand {
                                argument: Some(node),
                                depth,
                                rejected: false,
                                extras: &[],
                            }
                        };
                        work.push(Work::AwaitOperand {
                            node,
                            depth,
                            mode: Mode::General,
                            operand,
                            site: Some(site),
                        });
                        continue;
                    }
                    if matches!(mode, Mode::Settled) {
                        if matches!(
                            node,
                            Expression::LogicalExpression(_) | Expression::AssignmentExpression(_)
                        ) {
                            return Err(unsupported(
                                node,
                                "settled Promise selection or storage requires rejection tracking",
                            ));
                        }
                        if schedule_selection(&mut work, node, depth, mode)? {
                            continue;
                        }
                        if !self.settled_promise_shape_in(
                            node,
                            depth,
                            &std::collections::BTreeMap::new(),
                            0,
                        ) {
                            if self.fulfilled_promise_shape(node, depth) {
                                work.push(Work::PromotePromise(node));
                                work.push(Work::Eval(node, depth, Mode::Promise));
                                continue;
                            }
                            return Err(unsupported(
                                node,
                                "settled Promise positions require a synchronous forwarding call",
                            ));
                        }
                    }
                    if matches!(mode, Mode::Promise) {
                        if schedule_selection(&mut work, node, depth, mode)? {
                            continue;
                        }
                        if let Some(slot) = self.fulfilled_promise_slot(node, depth) {
                            let result = self.value(Type::FulfilledPromise);
                            Self::push(regions.current(), node, Op::Load { result, slot });
                            values.push(Value::Promise(result));
                            continue;
                        }
                        if let Expression::AssignmentExpression(assignment) = node {
                            if matches!(
                                assignment.operator,
                                AssignmentOperator::LogicalAnd
                                    | AssignmentOperator::LogicalOr
                                    | AssignmentOperator::LogicalNullish
                            ) {
                                self.schedule_logical_assignment(
                                    node,
                                    assignment,
                                    depth,
                                    mode,
                                    &mut regions,
                                    &mut work,
                                )?;
                                continue;
                            }
                            let identifier = assignment
                                .left
                                .as_simple_assignment_target()
                                .and_then(|target| target_identifier(target, depth).ok().flatten())
                                .ok_or_else(|| {
                                    unsupported(
                                        node,
                                        "Promise assignment requires an initialized identifier",
                                    )
                                })?;
                            let binding =
                                numeric_binding(identifier, self.semantic, &self.bindings)?;
                            if !binding.mutable {
                                return Err(unsupported(
                                    node,
                                    "assignment to a const binding is not admitted",
                                ));
                            }
                            if assignment.operator != AssignmentOperator::Assign
                                || self.slots[binding.slot.0].ty != Type::FulfilledPromise
                            {
                                return Err(unsupported(
                                    node,
                                    "Promise assignment requires plain = and an unchanged Promise type",
                                ));
                            }
                            work.push(Work::StoreTyped {
                                node,
                                slot: binding.slot,
                            });
                            work.push(Work::Eval(&assignment.right, depth + 1, Mode::Promise));
                            continue;
                        }
                        if let Expression::CallExpression(call) = node
                            && let Some(intrinsic) = awaited::intrinsic(call, self.semantic)
                                .map_err(|message| unsupported(node, message))?
                            && !intrinsic.rejected
                        {
                            for extra in intrinsic.extras.iter().rev() {
                                work.push(Work::Eval(
                                    extra.as_expression().expect("validated ordinary argument"),
                                    depth + 1,
                                    Mode::Discard,
                                ));
                            }
                            if let Some(argument) = intrinsic.argument {
                                if self.fulfilled_promise_shape(argument, depth + 1) {
                                    work.push(Work::Eval(argument, depth + 1, Mode::Promise));
                                } else {
                                    work.push(Work::FulfillPromise(node));
                                    work.push(Work::Eval(argument, depth + 1, Mode::General));
                                }
                            } else {
                                let input = self.undefined_value(node, regions.current());
                                values.push(Value::General(input));
                                work.push(Work::FulfillPromise(node));
                            }
                            continue;
                        }
                        if !matches!(node, Expression::CallExpression(call) if self.closure_target(&call.callee, depth + 1).is_some() || self.function_arity(&call.callee, depth + 1).is_some_and(|signature| self.closure_signatures.borrow().result(signature) == Some(Type::FulfilledPromise)))
                        {
                            return Err(unsupported(
                                node,
                                "expression requires a represented fulfilled Promise",
                            ));
                        }
                    }
                    if matches!(mode, Mode::Predicate) && self.fulfilled_promise_shape(node, depth)
                    {
                        work.push(Work::PromiseTruthy(node));
                        work.push(Work::Eval(node, depth, Mode::Promise));
                        continue;
                    }
                    if let Mode::Function(signature) = mode {
                        if schedule_selection(&mut work, node, depth, mode)? {
                            continue;
                        }
                        if let Expression::AssignmentExpression(assignment) = node {
                            if matches!(
                                assignment.operator,
                                AssignmentOperator::LogicalAnd
                                    | AssignmentOperator::LogicalOr
                                    | AssignmentOperator::LogicalNullish
                            ) {
                                self.schedule_logical_assignment(
                                    node,
                                    assignment,
                                    depth,
                                    mode,
                                    &mut regions,
                                    &mut work,
                                )?;
                                continue;
                            }
                            let (binding, actual) = self.function_assignment_target(assignment, depth)
                                .ok_or_else(|| unsupported(node, "function assignment requires an initialized function binding"))?;
                            if !binding.mutable {
                                return Err(unsupported(
                                    node,
                                    "assignment to a const binding is not admitted",
                                ));
                            }
                            if assignment.operator != AssignmentOperator::Assign
                                || actual != signature
                            {
                                return Err(unsupported(
                                    node,
                                    "function assignment requires plain = with unchanged arity",
                                ));
                            }
                            work.push(Work::StoreTyped {
                                node,
                                slot: binding.slot,
                            });
                            work.push(Work::Eval(&assignment.right, depth + 1, mode));
                            continue;
                        }
                        let outer = self.function_result_depth;
                        if matches!(node, Expression::CallExpression(_)) {
                            self.function_result_depth = (self.inline_depth + self.statement_depth)
                                .max(outer)
                                + regions.children.len();
                        }
                        let result =
                            self.function_value_leaf(node, regions.current(), signature, depth);
                        self.function_result_depth = outer;
                        values.push(Value::Function(result?));
                        continue;
                    }
                    if matches!(mode, Mode::Predicate)
                        && let Some(signature) = self.function_arity(node, depth)
                    {
                        // Evaluating a function can have constructor/argument/
                        // assignment effects. Its ToBoolean result is always true;
                        // evaluating its body is neither needed nor permitted here.
                        let outer = self.function_result_depth;
                        self.function_result_depth = (self.inline_depth + self.statement_depth)
                            .max(outer)
                            + regions.children.len();
                        let value = self.function_value(node, regions.current(), signature, depth);
                        self.function_result_depth = outer;
                        value?;
                        let result = self.value(Type::Predicate);
                        Self::push(
                            regions.current(),
                            node,
                            Op::Boolean {
                                result,
                                value: true,
                            },
                        );
                        values.push(Value::Predicate(result));
                        continue;
                    }
                    if let Expression::UnaryExpression(unary) = node
                        && unary.operator == UnaryOperator::Void
                    {
                        if matches!(mode, Mode::Predicate) {
                            work.push(Work::GeneralTruthy(node));
                            work.push(Work::Eval(node, depth, Mode::General));
                            continue;
                        }
                        if !matches!(mode, Mode::General) {
                            return Err(unsupported(
                                node,
                                "void result requires a primitive value or coercion position",
                            ));
                        }
                        work.push(Work::VoidResult(node));
                        work.push(Work::Eval(&unary.argument, depth + 1, Mode::Discard));
                        continue;
                    }
                    if let Expression::AwaitExpression(awaited) = node {
                        if !self.allow_await {
                            return Err(unsupported(node, "await requires the async profile"));
                        }
                        if matches!(mode, Mode::Predicate) {
                            work.push(Work::GeneralTruthy(node));
                            work.push(Work::Eval(node, depth, Mode::General));
                            continue;
                        }
                        let mode = if matches!(mode, Mode::General)
                            && self.numeric_result_shape(node, depth)
                        {
                            work.push(Work::Pack(node));
                            Mode::Number
                        } else {
                            mode
                        };
                        let operand = awaited::operand(awaited, depth, self.semantic)?;
                        work.push(Work::AwaitOperand {
                            node,
                            depth,
                            mode,
                            operand,
                            site: None,
                        });
                        continue;
                    }
                    if let Expression::CallExpression(call) = node
                        && self
                            .closure_target(&call.callee, depth + 1)
                            .is_none_or(|closure| {
                                awaited::immediate_async_arrow(
                                    self.arrow(closure.arrow),
                                    self.semantic,
                                )
                            })
                        && let Some(arity) = self.function_arity(&call.callee, depth + 1)
                    {
                        if self.closure_signatures.borrow().is_async(arity) {
                            return Err(unsupported(
                                node,
                                "suspending async closure calls require a direct await",
                            ));
                        }
                        let result_type = self
                            .closure_signatures
                            .borrow()
                            .result(arity)
                            .expect("checked signature");
                        if !matches!(
                            result_type,
                            Type::Number
                                | Type::General
                                | Type::FulfilledPromise
                                | Type::SettledPromise
                        ) {
                            return Err(unsupported(
                                node,
                                "function-valued runtime calls require a function position",
                            ));
                        }
                        let region_depth = regions.children.len();
                        let callee = self.prepare_closure_call(
                            node,
                            call,
                            arity,
                            regions.current(),
                            depth + 1,
                            region_depth,
                        )?;
                        call_result(
                            &mut work,
                            node,
                            mode,
                            result_type,
                            self.numeric_result_shape(node, depth),
                        )?;
                        work.push(Work::ClosureCall {
                            node,
                            call,
                            callee,
                            effect: ClosureEffect::Sync,
                            index: 0,
                            arguments: Vec::new(),
                            depth: depth + 1,
                            site: None,
                        });
                        continue;
                    }
                    if let Expression::CallExpression(call) = node
                        && let Some(mut closure) = self.closure_target(&call.callee, depth + 1)
                    {
                        let arrow = self.arrow(closure.arrow);
                        self.check_arrow(arrow)?;
                        if !closure.checked {
                            self.check_local_closure(&mut closure)?;
                            self.capture_cells(&closure, regions.current());
                        }
                        self.check_local_call(call, &closure)?;
                        if closure.factory {
                            return Err(unsupported(
                                node,
                                "function-valued local calls require an immutable closure binding",
                            ));
                        }
                        call_result(
                            &mut work,
                            node,
                            mode,
                            closure.result,
                            self.numeric_result_shape(node, depth),
                        )?;
                        work.push(Work::LocalCall {
                            node,
                            call,
                            closure,
                            index: 0,
                            arguments: Vec::new(),
                            depth: depth + 1,
                            site: None,
                        });
                        continue;
                    }
                    if self.fulfilled_promise_slot(node, depth).is_some() {
                        return Err(unsupported(
                            node,
                            "fulfilled Promise values require an await or Promise-alias initializer",
                        ));
                    }
                    if matches!(mode, Mode::General) {
                        if let Expression::Identifier(identifier) = node
                            && let Ok(binding) =
                                numeric_binding(identifier, self.semantic, &self.bindings)
                            && self.slots[binding.slot.0].ty == Type::General
                        {
                            let slot = binding.slot;
                            let result = self.value(Type::General);
                            Self::push(regions.current(), node, Op::Load { result, slot });
                            values.push(Value::General(result));
                            continue;
                        }
                        let nullish = match node {
                            Expression::NullLiteral(_) => Some(ir::Nullish::Null),
                            Expression::Identifier(identifier)
                                if identifier.name == "undefined"
                                    && identifier.reference_id.get().is_some_and(|reference| {
                                        self.semantic
                                            .scoping()
                                            .get_reference(reference)
                                            .symbol_id()
                                            .is_none()
                                    }) =>
                            {
                                Some(ir::Nullish::Undefined)
                            }
                            _ => None,
                        };
                        // Resolve the original callee symbol, never its spelling or a TS annotation.
                        let general_call = match node {
                            Expression::CallExpression(call) => match &call.callee {
                                Expression::Identifier(identifier) => {
                                    let symbol =
                                        identifier.reference_id.get().and_then(|reference| {
                                            self.semantic
                                                .scoping()
                                                .get_reference(reference)
                                                .symbol_id()
                                        });
                                    self.functions.iter().any(|function| {
                                        Some(function.symbol) == symbol
                                            && function.return_type == Type::General
                                    })
                                }
                                _ => false,
                            },
                            _ => false,
                        };
                        if let Some(value) = nullish {
                            let result = self.value(Type::General);
                            Self::push(regions.current(), node, Op::Nullish { result, value });
                            values.push(Value::General(result));
                            continue;
                        } else if !general_call
                            && self.general_assignment_target(node, depth).is_none()
                            && !self.general_logical_shape(node, depth)
                            && !matches!(
                                node,
                                Expression::ConditionalExpression(_)
                                    | Expression::SequenceExpression(_)
                            )
                        {
                            let scalar = if self.operand_boolean_shape(node, depth) {
                                Mode::Predicate
                            } else {
                                Mode::Number
                            };
                            work.push(Work::Pack(node));
                            work.push(Work::Eval(node, depth, scalar));
                            continue;
                        }
                    }
                    if matches!(mode, Mode::Predicate) {
                        if self.general_predicate_leaf(node, depth)
                            || matches!(node, Expression::LogicalExpression(logical)
                                if logical.operator == LogicalOperator::Coalesce)
                        {
                            work.push(Work::GeneralTruthy(node));
                            work.push(Work::Eval(node, depth, Mode::General));
                            continue;
                        }
                        if let Expression::BooleanLiteral(literal) = node {
                            let result = self.value(Type::Predicate);
                            Self::push(
                                regions.current(),
                                node,
                                Op::Boolean {
                                    result,
                                    value: literal.value,
                                },
                            );
                            values.push(Value::Predicate(result));
                            continue;
                        }
                        if let Expression::Identifier(identifier) = node
                            && let Ok(binding) =
                                numeric_binding(identifier, self.semantic, &self.bindings)
                            && self.slots[binding.slot.0].ty == Type::Predicate
                        {
                            let slot = binding.slot;
                            let result = self.value(Type::Predicate);
                            Self::push(regions.current(), node, Op::Load { result, slot });
                            values.push(Value::Predicate(result));
                            continue;
                        }
                        if let Expression::UnaryExpression(unary) = node
                            && unary.operator == UnaryOperator::LogicalNot
                        {
                            work.push(Work::Not(node));
                            work.push(Work::Eval(&unary.argument, depth + 1, Mode::Predicate));
                            continue;
                        }
                        if let Expression::BinaryExpression(binary) = node {
                            let operator = match binary.operator {
                                BinaryOperator::LessThan => Some(Compare::Less),
                                BinaryOperator::LessEqualThan => Some(Compare::LessEqual),
                                BinaryOperator::GreaterThan => Some(Compare::Greater),
                                BinaryOperator::GreaterEqualThan => Some(Compare::GreaterEqual),
                                BinaryOperator::Equality | BinaryOperator::StrictEquality => {
                                    Some(Compare::Equal)
                                }
                                BinaryOperator::Inequality | BinaryOperator::StrictInequality => {
                                    Some(Compare::NotEqual)
                                }
                                _ => None,
                            };
                            if let Some(operator) = operator {
                                let booleans = matches!(
                                    binary.operator,
                                    BinaryOperator::StrictEquality
                                        | BinaryOperator::StrictInequality
                                ) && self
                                    .operand_boolean_shape(&binary.left, depth + 1)
                                    && self.operand_boolean_shape(&binary.right, depth + 1);
                                let strict = matches!(
                                    binary.operator,
                                    BinaryOperator::StrictEquality
                                        | BinaryOperator::StrictInequality
                                );
                                let equality = strict
                                    || matches!(
                                        binary.operator,
                                        BinaryOperator::Equality | BinaryOperator::Inequality
                                    );
                                if equality
                                    && !(strict && booleans)
                                    && !(self.numeric_result_shape(&binary.left, depth + 1)
                                        && self.numeric_result_shape(&binary.right, depth + 1))
                                {
                                    work.push(Work::PrimitiveCompareRight {
                                        node,
                                        right: &binary.right,
                                        depth: depth + 1,
                                        loose: !strict,
                                        negated: matches!(
                                            binary.operator,
                                            BinaryOperator::Inequality
                                                | BinaryOperator::StrictInequality
                                        ),
                                    });
                                    work.push(Work::Eval(&binary.left, depth + 1, Mode::General));
                                    continue;
                                }
                                work.push(Work::CompareRight {
                                    node,
                                    right: &binary.right,
                                    depth: depth + 1,
                                    booleans,
                                    operator,
                                });
                                if booleans {
                                    predicate_number(&mut work, &binary.left, depth + 1);
                                } else {
                                    work.push(Work::Eval(
                                        &binary.left,
                                        depth + 1,
                                        if equality {
                                            Mode::Number
                                        } else {
                                            Mode::Arithmetic
                                        },
                                    ));
                                }
                                continue;
                            }
                        }
                        if !matches!(
                            node,
                            Expression::ConditionalExpression(_)
                                | Expression::LogicalExpression(_)
                                | Expression::SequenceExpression(_)
                        ) {
                            work.push(Work::Truthy(node));
                            // Predicate fallback does not consume another source node.
                            work.push(Work::Eval(node, depth, Mode::Number));
                            continue;
                        }
                    }
                    if schedule_selection(&mut work, node, depth, mode)? {
                        continue;
                    }
                    match node {
                        Expression::NumericLiteral(literal) => {
                            let result = self.value(Type::Number);
                            Self::push(
                                regions.current(),
                                node,
                                Op::Constant {
                                    result,
                                    value: literal.value,
                                },
                            );
                            values.push(Value::Number(result));
                        }
                        Expression::Identifier(identifier) => {
                            let reference = identifier
                                .reference_id
                                .get()
                                .ok_or("missing semantic reference")?;
                            let constant = match (
                                self.semantic.scoping().get_reference(reference).symbol_id(),
                                identifier.name.as_str(),
                            ) {
                                (None, "Infinity") => Some(f64::INFINITY),
                                (None, "NaN") => Some(f64::NAN),
                                _ => None,
                            };
                            let result = self.value(Type::Number);
                            let kind = if let Some(value) = constant {
                                Op::Constant { result, value }
                            } else {
                                let slot =
                                    numeric_binding(identifier, self.semantic, &self.bindings)?
                                        .slot;
                                if self.slots[slot.0].ty != Type::Number {
                                    return Err(unsupported(
                                        node,
                                        if self.slots[slot.0].ty == Type::General {
                                            "General bindings require a primitive value position"
                                        } else {
                                            "Boolean bindings are not admitted in numeric value positions"
                                        },
                                    ));
                                }
                                Op::Load { result, slot }
                            };
                            Self::push(regions.current(), node, kind);
                            values.push(Value::Number(result));
                        }
                        Expression::CallExpression(call) => {
                            if let Some((operator, _)) = math_unary(call, self.semantic) {
                                if call.arguments.is_empty() {
                                    let result = self.value(Type::Number);
                                    Self::push(
                                        regions.current(),
                                        node,
                                        Op::Constant {
                                            result,
                                            value: f64::NAN,
                                        },
                                    );
                                    values.push(Value::Number(result));
                                    continue;
                                }
                                if call.arguments.len() > 1000 {
                                    return Err(unsupported(
                                        node,
                                        "at most 1000 ordinary Math arguments are admitted",
                                    ));
                                }
                                work.push(Work::MathCall {
                                    node,
                                    call,
                                    operator,
                                    index: 0,
                                    input: None,
                                    depth: depth + 1,
                                });
                            } else {
                                let Expression::Identifier(identifier) = &call.callee else {
                                    return Err(unsupported(
                                        node,
                                        "expression is outside the numeric experiment",
                                    ));
                                };
                                let symbol = identifier.reference_id.get().and_then(|reference| {
                                    self.semantic.scoping().get_reference(reference).symbol_id()
                                });
                                let function = self
                                    .functions
                                    .iter()
                                    .find(|function| Some(function.symbol) == symbol)
                                    .ok_or_else(|| {
                                        unsupported(
                                            node,
                                            "expression is outside the numeric experiment",
                                        )
                                    })?;
                                if call.optional || call.type_arguments.is_some() {
                                    return Err(unsupported(
                                        node,
                                        "optional and generic calls are not admitted",
                                    ));
                                }
                                if call.arguments.len() > 1000 {
                                    return Err(unsupported(
                                        node,
                                        "at most 1000 ordinary direct call arguments are admitted",
                                    ));
                                }
                                if call.arguments.len() < function.parameter_count {
                                    return Err(unsupported(
                                        node,
                                        "call is missing declared arguments",
                                    ));
                                }
                                if function.return_type == Type::General
                                    && !matches!(mode, Mode::General)
                                {
                                    return Err(unsupported(
                                        node,
                                        "value-returning calls require a General result position",
                                    ));
                                }
                                work.push(Work::Call {
                                    node,
                                    call,
                                    function: function.id,
                                    index: 0,
                                    arguments: Vec::new(),
                                    depth: depth + 1,
                                });
                            }
                        }
                        Expression::AssignmentExpression(assignment) => {
                            if matches!(
                                assignment.operator,
                                AssignmentOperator::LogicalAnd
                                    | AssignmentOperator::LogicalOr
                                    | AssignmentOperator::LogicalNullish
                            ) {
                                self.schedule_logical_assignment(
                                    node,
                                    assignment,
                                    depth,
                                    mode,
                                    &mut regions,
                                    &mut work,
                                )?;
                                continue;
                            }
                            let operator = match assignment.operator {
                                AssignmentOperator::Assign => None,
                                AssignmentOperator::Addition => Some(Binary::Add),
                                AssignmentOperator::Subtraction => Some(Binary::Subtract),
                                AssignmentOperator::Multiplication => Some(Binary::Multiply),
                                AssignmentOperator::Division => Some(Binary::Divide),
                                AssignmentOperator::Remainder => Some(Binary::Remainder),
                                AssignmentOperator::BitwiseAnd => Some(Binary::BitAnd),
                                AssignmentOperator::BitwiseOR => Some(Binary::BitOr),
                                AssignmentOperator::BitwiseXOR => Some(Binary::BitXor),
                                AssignmentOperator::ShiftLeft => Some(Binary::ShiftLeft),
                                AssignmentOperator::ShiftRight => Some(Binary::ShiftRight),
                                AssignmentOperator::ShiftRightZeroFill => {
                                    Some(Binary::ShiftRightUnsigned)
                                }
                                _ => {
                                    return Err(unsupported(
                                        node,
                                        "only =, +=, -=, *=, /=, %=, &=, |=, ^=, <<=, >>=, >>>=, &&=, ||= and ??= assignments are admitted",
                                    ));
                                }
                            };
                            let Some(identifier) = assignment
                                .left
                                .as_simple_assignment_target()
                                .map(|target| target_identifier(target, depth))
                                .transpose()?
                                .flatten()
                            else {
                                return Err(unsupported(
                                    node,
                                    "assignment target must be an initialized identifier",
                                ));
                            };
                            let binding =
                                numeric_binding(identifier, self.semantic, &self.bindings)?;
                            if !binding.mutable {
                                return Err(unsupported(
                                    node,
                                    "assignment to a const binding is not admitted",
                                ));
                            }
                            let slot = binding.slot;
                            if matches!(self.slots[slot.0].ty, Type::Closure(_)) {
                                return Err(unsupported(
                                    node,
                                    "function assignments require a function-valued or discarded position",
                                ));
                            }
                            if matches!(
                                self.slots[slot.0].ty,
                                Type::FulfilledPromise | Type::Promise
                            ) {
                                return Err(unsupported(
                                    node,
                                    "Promise assignments require a Promise-valued or discarded position",
                                ));
                            }
                            let general = self.slots[slot.0].ty == Type::General;
                            if general && operator.is_none() {
                                work.push(Work::StoreGeneral { node, slot });
                                work.push(Work::Eval(
                                    &assignment.right,
                                    depth + 1,
                                    if matches!(mode, Mode::Number) {
                                        Mode::Number
                                    } else {
                                        Mode::General
                                    },
                                ));
                                continue;
                            }
                            let previous = if operator.is_some() {
                                let result =
                                    self.value(if general { Type::General } else { Type::Number });
                                Self::push(regions.current(), node, Op::Load { result, slot });
                                Some(result)
                            } else {
                                None
                            };
                            work.push(Work::Assign {
                                node,
                                slot,
                                previous,
                                operator,
                            });
                            work.push(Work::Eval(
                                &assignment.right,
                                depth + 1,
                                if operator.is_some() {
                                    Mode::Arithmetic
                                } else {
                                    Mode::Number
                                },
                            ));
                        }
                        Expression::UpdateExpression(update) => {
                            let Some(identifier) = target_identifier(&update.argument, depth)?
                            else {
                                return Err(unsupported(
                                    node,
                                    "update target must be an initialized identifier",
                                ));
                            };
                            let binding =
                                numeric_binding(identifier, self.semantic, &self.bindings)?;
                            if !binding.mutable {
                                return Err(unsupported(
                                    node,
                                    "update to a const binding is not admitted",
                                ));
                            }
                            let slot = binding.slot;
                            if matches!(self.slots[slot.0].ty, Type::Closure(_)) {
                                return Err(unsupported(
                                    node,
                                    "function bindings do not support numeric updates",
                                ));
                            }
                            if matches!(
                                self.slots[slot.0].ty,
                                Type::FulfilledPromise | Type::Promise
                            ) {
                                return Err(unsupported(
                                    node,
                                    "Promise bindings do not support numeric updates",
                                ));
                            }
                            let general = self.slots[slot.0].ty == Type::General;
                            let loaded =
                                self.value(if general { Type::General } else { Type::Number });
                            Self::push(
                                regions.current(),
                                node,
                                Op::Load {
                                    result: loaded,
                                    slot,
                                },
                            );
                            // ToNumeric is ToNumber for the four represented primitive kinds.
                            let previous = if general {
                                let result = self.value(Type::Number);
                                Self::push(
                                    regions.current(),
                                    node,
                                    Op::NumberFromGeneral {
                                        result,
                                        input: loaded,
                                    },
                                );
                                result
                            } else {
                                loaded
                            };
                            let one = self.value(Type::Number);
                            Self::push(
                                regions.current(),
                                node,
                                Op::Constant {
                                    result: one,
                                    value: 1.0,
                                },
                            );
                            let next = self.value(Type::Number);
                            let operator = match update.operator {
                                UpdateOperator::Increment => Binary::Add,
                                UpdateOperator::Decrement => Binary::Subtract,
                            };
                            Self::push(
                                regions.current(),
                                node,
                                Op::Binary {
                                    result: next,
                                    operator,
                                    left: previous,
                                    right: one,
                                },
                            );
                            let stored = if general {
                                let result = self.value(Type::General);
                                Self::push(
                                    regions.current(),
                                    node,
                                    Op::Pack {
                                        result,
                                        input: next,
                                    },
                                );
                                result
                            } else {
                                next
                            };
                            Self::push(
                                regions.current(),
                                node,
                                Op::Store {
                                    slot,
                                    value: stored,
                                },
                            );
                            values.push(Value::Number(if update.prefix { next } else { previous }));
                        }
                        Expression::UnaryExpression(unary) => {
                            let operator = match unary.operator {
                                UnaryOperator::UnaryPlus => Unary::Plus,
                                UnaryOperator::UnaryNegation => Unary::Negate,
                                UnaryOperator::BitwiseNot => Unary::BitNot,
                                _ => {
                                    return Err(unsupported(
                                        node,
                                        "only unary +, - and ~ are admitted",
                                    ));
                                }
                            };
                            work.push(Work::Unary(node, operator));
                            work.push(Work::Eval(&unary.argument, depth + 1, Mode::Arithmetic));
                        }
                        Expression::BinaryExpression(binary) => {
                            let operator = match binary.operator {
                                BinaryOperator::Addition => Binary::Add,
                                BinaryOperator::Subtraction => Binary::Subtract,
                                BinaryOperator::Multiplication => Binary::Multiply,
                                BinaryOperator::Division => Binary::Divide,
                                BinaryOperator::Remainder => Binary::Remainder,
                                BinaryOperator::BitwiseAnd => Binary::BitAnd,
                                BinaryOperator::BitwiseOR => Binary::BitOr,
                                BinaryOperator::BitwiseXOR => Binary::BitXor,
                                BinaryOperator::ShiftLeft => Binary::ShiftLeft,
                                BinaryOperator::ShiftRight => Binary::ShiftRight,
                                BinaryOperator::ShiftRightZeroFill => Binary::ShiftRightUnsigned,
                                _ => {
                                    return Err(unsupported(
                                        node,
                                        "only binary +, -, *, /, %, &, |, ^, <<, >> and >>> are admitted",
                                    ));
                                }
                            };
                            let mode = Mode::Arithmetic;
                            work.push(Work::BinaryRight {
                                node,
                                right: &binary.right,
                                depth: depth + 1,
                                mode,
                                operator,
                            });
                            work.push(Work::Eval(&binary.left, depth + 1, mode));
                        }
                        _ => {
                            return Err(unsupported(
                                node,
                                "expression is outside the numeric experiment",
                            ));
                        }
                    }
                }
            }
        }
        assert!(regions.children.is_empty(), "all child regions completed");
        let result = if matches!(mode, Mode::Discard) {
            None
        } else {
            Some(values.pop().expect("evaluation result").id())
        };
        assert!(values.is_empty(), "all intermediate values consumed");
        Ok(result)
    }

    // Both call effects share source validation and callee-before-arguments evaluation.
    fn prepare_closure_call(
        &mut self,
        node: &Expression<'_>,
        call: &CallExpression<'_>,
        signature: usize,
        region: &mut Region,
        depth: usize,
        region_depth: usize,
    ) -> Result<ValueId, String> {
        if call.optional || call.type_arguments.is_some() {
            return Err(unsupported(
                node,
                "optional and generic runtime closure calls are not admitted",
            ));
        }
        if call.arguments.len() > 1000 {
            return Err(unsupported(
                node,
                "at most 1000 ordinary runtime closure arguments are admitted",
            ));
        }
        if call.arguments.len()
            < self
                .closure_signatures
                .borrow()
                .parameter_count(signature)
                .expect("checked closure signature")
        {
            return Err(unsupported(
                node,
                "runtime closure call is missing declared arguments",
            ));
        }
        let callee = if let Some((slot, _)) = self.runtime_closure_callee(&call.callee, depth) {
            let value = self.value(Type::Closure(signature));
            Self::push(
                region,
                node,
                Op::Load {
                    result: value,
                    slot,
                },
            );
            value
        } else {
            let outer = self.function_result_depth;
            self.function_result_depth =
                (self.inline_depth + self.statement_depth).max(outer) + region_depth;
            let value = self.function_value(&call.callee, region, signature, depth);
            self.function_result_depth = outer;
            value?
        };
        Ok(callee)
    }

    fn push_await(region: &mut Region, node: &impl GetSpan, site: Option<usize>, kind: Op) {
        if let Some(site) = site {
            Self::push(region, node, Op::AwaitSite { site });
        }
        Self::push(region, node, kind);
    }
}
