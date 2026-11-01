//! Effect-free function signature queries and constructor/call leaves for the
//! shared typed evaluator. Queries never execute source arguments or conditions.
use super::*;
use std::collections::BTreeMap;

impl<'s, 'a> Lowering<'s, 'a> {
    pub(super) fn named_async_function(
        &self,
        expression: &Expression<'_>,
        depth: usize,
    ) -> Option<(ir::FunctionId, usize)> {
        let Expression::Identifier(identifier) = closures::unwrapped(expression, depth)? else {
            return None;
        };
        let symbol = self
            .semantic
            .scoping()
            .get_reference(identifier.reference_id.get()?)
            .symbol_id()?;
        self.functions
            .iter()
            .find(|function| function.symbol == symbol)
            .and_then(|function| {
                function
                    .async_signature
                    .map(|signature| (function.id, signature))
            })
    }

    pub(super) fn named_async_value(
        &mut self,
        node: &Expression<'_>,
        depth: usize,
        signature: usize,
        region: &mut Region,
    ) -> Result<Option<ValueId>, String> {
        let Some((function, actual)) = self.named_async_function(node, depth) else {
            return Ok(None);
        };
        if actual != signature {
            return Err(unsupported(
                node,
                "named async function requires its complete signature",
            ));
        }
        let result = self.value(Type::Closure(signature));
        Self::push(
            region,
            node,
            Op::MakeClosure {
                result,
                function,
                captures: Vec::new(),
            },
        );
        Ok(Some(result))
    }

    pub(super) fn arrow_signature(
        &self,
        arrow: &oxc_ast::ast::ArrowFunctionExpression<'_>,
    ) -> Option<usize> {
        self.arrow_signature_at(arrow, 0, &BTreeMap::new())
    }

    fn arrow_signature_at(
        &self,
        arrow: &oxc_ast::ast::ArrowFunctionExpression<'_>,
        calls: usize,
        outer: &BTreeMap<SymbolId, Option<usize>>,
    ) -> Option<usize> {
        if calls >= 8 {
            return None;
        }
        let parameters = callbacks::parameter_types(
            arrow,
            self.callback_parameters,
            callbacks::ValueInputs {
                general: self.general_parameters,
                managed: &self.managed_kinds.inputs,
            },
            self.promise_parameters,
        );
        if arrow.r#async && self.async_results.contains(&arrow.node_id.get()) {
            return Some(
                self.closure_signatures
                    .borrow_mut()
                    .intern_async(parameters),
            );
        }
        let mut bindings: BTreeMap<_, _> = self
            .bindings
            .iter()
            .map(|binding| {
                (
                    binding.symbol,
                    match self.slots[binding.slot.0].ty {
                        Type::Closure(id) => Some(id),
                        _ => None,
                    },
                )
            })
            .collect();
        for (symbol, signature) in outer {
            bindings.entry(*symbol).or_insert(*signature);
        }
        for (parameter, ty) in arrow.params.items.iter().zip(&parameters) {
            if let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern {
                bindings.insert(
                    identifier.symbol_id.get()?,
                    match ty {
                        Type::Closure(id) => Some(*id),
                        _ => None,
                    },
                );
            }
        }
        let mut result = if arrow.r#async {
            if self.promise_parameters.identities {
                Type::Promise
            } else if awaited::owns_throw(arrow, self.semantic)
                || self.settled_results.contains(&arrow.node_id.get())
            {
                Type::SettledPromise
            } else {
                Type::FulfilledPromise
            }
        } else {
            arrow
                .get_expression()
                .and_then(|body| self.function_arity_in(body, 0, &bindings, calls + 1))
                .or_else(|| self.block_function_arity_in(arrow, 0, &bindings, calls + 1))
                .map_or_else(
                    || self.arrow_scalar_result_at(arrow, &bindings, calls),
                    Type::Closure,
                )
        };
        if result == Type::FulfilledPromise && self.settled_results.contains(&arrow.node_id.get()) {
            result = Type::SettledPromise;
        }
        Some(
            self.closure_signatures
                .borrow_mut()
                .intern(parameters, result),
        )
    }

    // Scalar results use the same positive Number proof as storage planning; otherwise
    // the body checker must validate the represented primitive domain.
    pub(super) fn arrow_scalar_result(
        &self,
        arrow: &oxc_ast::ast::ArrowFunctionExpression<'_>,
        bindings: &BTreeMap<SymbolId, Option<usize>>,
    ) -> Type {
        self.arrow_scalar_result_at(arrow, bindings, 0)
    }

    fn arrow_scalar_result_at(
        &self,
        arrow: &oxc_ast::ast::ArrowFunctionExpression<'_>,
        bindings: &BTreeMap<SymbolId, Option<usize>>,
        calls: usize,
    ) -> Type {
        if !arrow.r#async
            && let Some(ty) = self.managed_kinds.results.get(&arrow.node_id.get())
        {
            return *ty;
        }
        // Original-source provenance also sees aliases declared inside a block
        // before this definition checker has lowered their local bindings.
        if !arrow.r#async
            && self
                .promise_parameters
                .mixed_results
                .contains(&arrow.node_id.get())
        {
            return Type::AsyncValue;
        }
        if !arrow.r#async
            && self
                .promise_parameters
                .results
                .contains(&arrow.node_id.get())
        {
            return Type::Promise;
        }
        if !arrow.r#async
            && storage::arrow_returns(arrow, self.semantic).is_some_and(|results| {
                !results.is_empty()
                    && results.into_iter().all(|expression| {
                        self.pending_promise_shape_in(expression, 0, bindings, calls)
                    })
            })
        {
            return Type::Promise;
        }
        if !arrow.r#async
            && storage::arrow_returns(arrow, self.semantic).is_some_and(|results| {
                !results.is_empty()
                    && results.into_iter().all(|expression| {
                        self.settled_promise_shape_in(expression, 0, bindings, calls)
                    })
            })
        {
            return Type::SettledPromise;
        }
        if !arrow.r#async
            && storage::arrow_returns(arrow, self.semantic).is_some_and(|results| {
                !results.is_empty()
                    && results.into_iter().all(|expression| {
                        self.fulfilled_promise_shape_in(expression, 0, bindings, calls)
                    })
            })
        {
            return Type::FulfilledPromise;
        }
        if self.general_results.contains(&arrow.node_id.get()) {
            return Type::General;
        }
        let mut numeric_calls: std::collections::BTreeSet<_> = self
            .closures
            .iter()
            .filter(|closure| !closure.factory && closure.result == Type::Number)
            .filter_map(|closure| closure.symbol)
            .collect();
        for (symbol, signature) in self
            .callback_parameters
            .values()
            .flat_map(|parameters| parameters.iter())
        {
            if self.closure_signatures.borrow().result(*signature) == Some(Type::Number) {
                numeric_calls.insert(*symbol);
            }
        }
        for (symbol, signature) in bindings {
            if signature
                .is_some_and(|id| self.closure_signatures.borrow().result(id) == Some(Type::Number))
            {
                numeric_calls.insert(*symbol);
            }
        }
        let number = storage::arrow_number_result_shape(
            arrow,
            self.semantic,
            self.functions,
            storage::NumberCalls::Proven(self.number_calls),
            |symbol| {
                numeric_calls.contains(&symbol)
                    || (!self.general_bindings.contains(&symbol)
                        && !self.bindings.iter().any(|binding| binding.symbol == symbol))
            },
            |symbol| {
                if let Some(binding) = self
                    .bindings
                    .iter()
                    .find(|binding| binding.symbol == symbol)
                {
                    return self.slots[binding.slot.0].ty == Type::Number;
                }
                // A not-yet-bound parameter or captured local can still have a
                // static Number proof, including all later writes. Function
                // parameters remain function pairs, never scalar Numbers.
                !self.general_bindings.contains(&symbol)
                    && !bindings.get(&symbol).is_some_and(Option::is_some)
                    && !self
                        .callback_parameters
                        .values()
                        .any(|parameters| parameters.contains_key(&symbol))
            },
        );
        if number { Type::Number } else { Type::General }
    }

    pub(super) fn callback_argument(
        &mut self,
        expression: &Expression<'_>,
        region: &mut Region,
        arity: usize,
        depth: usize,
    ) -> Result<ValueId, String> {
        // An argument constructor can reenter full body checking, which has a
        // larger native frame than comma-prefix evaluation's cursor crossing.
        if self.active_function_values >= 8 {
            return Err(unsupported(
                expression,
                "callback arguments require fewer than 8 active function-value cursors",
            ));
        }
        self.function_value(expression, region, arity, depth)
    }

    pub(super) fn function_assignment_target(
        &self,
        assignment: &oxc_ast::ast::AssignmentExpression<'_>,
        depth: usize,
    ) -> Option<(NumericBinding, usize)> {
        let identifier =
            target_identifier(assignment.left.as_simple_assignment_target()?, depth).ok()??;
        let binding = numeric_binding(identifier, self.semantic, &self.bindings).ok()?;
        let Type::Closure(arity) = self.slots[binding.slot.0].ty else {
            return None;
        };
        Some((binding.clone(), arity))
    }

    pub(super) fn runtime_closure_callee(
        &self,
        node: &Expression<'_>,
        depth: usize,
    ) -> Option<(SlotId, usize)> {
        let Expression::Identifier(identifier) = closures::unwrapped(node, depth)? else {
            return None;
        };
        let slot = numeric_binding(identifier, self.semantic, &self.bindings)
            .ok()?
            .slot;
        let Type::Closure(arity) = self.slots[slot.0].ty else {
            return None;
        };
        Some((slot, arity))
    }

    pub(super) fn function_arity(
        &self,
        expression: &Expression<'_>,
        depth: usize,
    ) -> Option<usize> {
        self.function_arity_in(expression, depth, &BTreeMap::new(), 0)
    }

    // This is a type query, not specialization: source arguments, conditions and
    // comma effects are not executed. None in a bound formal explicitly means
    // scalar, so another call's callback binding cannot leak into this scope.
    pub(super) fn function_arity_in<'e>(
        &'e self,
        expression: &'e Expression<'a>,
        depth: usize,
        bindings: &BTreeMap<SymbolId, Option<usize>>,
        calls: usize,
    ) -> Option<usize> {
        // A signature query can traverse source wrappers, callees, actuals
        // and returned factory bodies. Neither the 128 AST-depth limit nor
        // the eight-call limit bounds the size of the native Rust frames.
        type Bindings = std::rc::Rc<BTreeMap<SymbolId, Option<usize>>>;
        enum Work<'e, 'a> {
            Expression {
                node: &'e Expression<'a>,
                bindings: Bindings,
                depth: usize,
                calls: usize,
            },
            Same(usize),
            Require(usize),
            CalleeResult,
            FactoryArguments {
                arrow: &'e oxc_ast::ast::ArrowFunctionExpression<'a>,
                parameters: BTreeMap<SymbolId, Option<usize>>,
                symbols: Vec<SymbolId>,
                calls: usize,
            },
        }

        let mut work = vec![Work::Expression {
            node: expression,
            bindings: std::rc::Rc::new(bindings.clone()),
            depth,
            calls,
        }];
        let mut results: Vec<Option<usize>> = Vec::new();
        while let Some(task) = work.pop() {
            match task {
                Work::Same(count) => {
                    let mut matched = None;
                    let mut valid = true;
                    for _ in 0..count {
                        match results.pop().expect("scheduled signature query") {
                            Some(next) if matched.is_none_or(|expected| expected == next) => {
                                matched = Some(next);
                            }
                            _ => valid = false,
                        }
                    }
                    results.push(if valid { matched } else { None });
                }
                Work::Require(expected) => {
                    let actual = results.pop().expect("scheduled assignment signature");
                    results.push((actual == Some(expected)).then_some(expected));
                }
                Work::CalleeResult => {
                    let signature = results.pop().expect("scheduled callee signature");
                    let result = signature
                        .and_then(|callee| self.closure_signatures.borrow().result(callee))
                        .and_then(|ty| match ty {
                            Type::Closure(result) => Some(result),
                            _ => None,
                        });
                    results.push(result);
                }
                Work::FactoryArguments {
                    arrow,
                    mut parameters,
                    symbols,
                    calls,
                } => {
                    for symbol in symbols.into_iter().rev() {
                        parameters.insert(
                            symbol,
                            results.pop().expect("scheduled factory argument signature"),
                        );
                    }
                    if let Some(body) = arrow.get_expression() {
                        work.push(Work::Expression {
                            node: body,
                            bindings: std::rc::Rc::new(parameters),
                            depth: 0,
                            calls: calls + 1,
                        });
                    } else {
                        results.push(self.block_function_arity_in(
                            arrow,
                            0,
                            &parameters,
                            calls + 1,
                        ));
                    }
                }
                Work::Expression {
                    node,
                    bindings,
                    depth,
                    calls,
                } => {
                    if depth > 128 {
                        results.push(None);
                        continue;
                    }
                    if let Some(inner) = erased_inner(node) {
                        work.push(Work::Expression {
                            node: inner,
                            bindings,
                            depth: depth + 1,
                            calls,
                        });
                        continue;
                    }
                    match node {
                        Expression::ParenthesizedExpression(wrapper) => {
                            work.push(Work::Expression {
                                node: &wrapper.expression,
                                bindings,
                                depth: depth + 1,
                                calls,
                            });
                        }
                        Expression::ConditionalExpression(conditional) => {
                            work.push(Work::Same(2));
                            work.push(Work::Expression {
                                node: &conditional.alternate,
                                bindings: bindings.clone(),
                                depth: depth + 1,
                                calls,
                            });
                            work.push(Work::Expression {
                                node: &conditional.consequent,
                                bindings,
                                depth: depth + 1,
                                calls,
                            });
                        }
                        Expression::LogicalExpression(logical) => {
                            work.push(Work::Same(2));
                            work.push(Work::Expression {
                                node: &logical.right,
                                bindings: bindings.clone(),
                                depth: depth + 1,
                                calls,
                            });
                            work.push(Work::Expression {
                                node: &logical.left,
                                bindings,
                                depth: depth + 1,
                                calls,
                            });
                        }
                        Expression::SequenceExpression(sequence) => {
                            if let Some(last) = sequence.expressions.last() {
                                work.push(Work::Expression {
                                    node: last,
                                    bindings,
                                    depth: depth + 1,
                                    calls,
                                });
                            } else {
                                results.push(None);
                            }
                        }
                        Expression::AssignmentExpression(assignment)
                            if matches!(
                                assignment.operator,
                                AssignmentOperator::Assign
                                    | AssignmentOperator::LogicalAnd
                                    | AssignmentOperator::LogicalOr
                                    | AssignmentOperator::LogicalNullish
                            ) =>
                        {
                            // A failed nested query supplies None to its
                            // caller; it must not abort the root worklist.
                            let expected: Option<usize> = (|| {
                                let target = target_identifier(
                                    assignment.left.as_simple_assignment_target()?,
                                    depth,
                                )
                                .ok()
                                .flatten()?;
                                let symbol = self
                                    .semantic
                                    .scoping()
                                    .get_reference(target.reference_id.get()?)
                                    .symbol_id()?;
                                let expected = match bindings.get(&symbol) {
                                    Some(kind) => (*kind)?,
                                    None => self.function_assignment_target(assignment, depth)?.1,
                                };
                                Some(expected)
                            })();
                            if let Some(expected) = expected {
                                work.push(Work::Require(expected));
                                work.push(Work::Expression {
                                    node: &assignment.right,
                                    bindings,
                                    depth: depth + 1,
                                    calls,
                                });
                            } else {
                                results.push(None);
                            }
                        }
                        Expression::CallExpression(call) => {
                            let callee = invocation::callee(call);
                            let callee_depth = invocation::depth(call, depth + 1);
                            if let Some(factory) = self.closure_target(callee, callee_depth) {
                                let arrow = self.arrow(factory.arrow);
                                let body = arrow.get_expression();
                                if factory.factory
                                    && let Some(body) = body
                                    && let Some(returned) = self.closure_target(body, 0)
                                {
                                    results.push(self.arrow_signature_at(
                                        self.arrow(returned.arrow),
                                        calls,
                                        &bindings,
                                    ));
                                    continue;
                                }
                                if calls >= 8
                                    || arrow.params.items.len() > invocation::arguments(call).len()
                                {
                                    results.push(None);
                                    continue;
                                }
                                let mut parameters = BTreeMap::new();
                                for capture in &factory.captures {
                                    parameters.insert(
                                        capture.symbol,
                                        match self.slots[capture.slot.0].ty {
                                            Type::Closure(arity) => Some(arity),
                                            _ => None,
                                        },
                                    );
                                }
                                let inputs = (|| {
                                    let mut arguments = Vec::new();
                                    let mut symbols = Vec::new();
                                    for (parameter, argument) in
                                        arrow.params.items.iter().zip(invocation::arguments(call))
                                    {
                                        let BindingPattern::BindingIdentifier(identifier) =
                                            &parameter.pattern
                                        else {
                                            return None;
                                        };
                                        arguments.push(argument.as_expression()?);
                                        symbols.push(identifier.symbol_id.get()?);
                                    }
                                    Some((arguments, symbols))
                                })();
                                if let Some((arguments, symbols)) = inputs {
                                    work.push(Work::FactoryArguments {
                                        arrow,
                                        parameters,
                                        symbols,
                                        calls,
                                    });
                                    for argument in arguments.into_iter().rev() {
                                        work.push(Work::Expression {
                                            node: argument,
                                            bindings: bindings.clone(),
                                            depth: depth + 1,
                                            calls: calls + 1,
                                        });
                                    }
                                } else {
                                    results.push(None);
                                }
                            } else if calls >= 8 {
                                results.push(None);
                            } else {
                                work.push(Work::CalleeResult);
                                work.push(Work::Expression {
                                    node: callee,
                                    bindings,
                                    depth: callee_depth,
                                    calls: calls + 1,
                                });
                            }
                        }
                        _ => {
                            if let Expression::Identifier(identifier) = node
                                && let Some(symbol) =
                                    identifier.reference_id.get().and_then(|reference| {
                                        self.semantic.scoping().get_reference(reference).symbol_id()
                                    })
                                && let Some(kind) = bindings.get(&symbol)
                            {
                                results.push(*kind);
                            } else if let Some((_, arity)) = self.named_async_function(node, depth)
                            {
                                results.push(Some(arity));
                            } else if let Some((_, arity)) =
                                self.runtime_closure_callee(node, depth)
                            {
                                results.push(Some(arity));
                            } else {
                                results.push(self.closure_target(node, depth).and_then(
                                    |closure| {
                                        self.arrow_signature_at(
                                            self.arrow(closure.arrow),
                                            calls,
                                            &bindings,
                                        )
                                    },
                                ));
                            }
                        }
                    }
                }
            }
        }
        results.pop().expect("root signature query")
    }

    pub(super) fn function_value(
        &mut self,
        expression: &Expression<'_>,
        region: &mut Region,
        arity: usize,
        depth: usize,
    ) -> Result<ValueId, String> {
        // Constructors can run scalar comma prefixes, which can construct another
        // function. Bound that cursor crossing independently of AST/lift limits.
        if self.active_function_values >= 16 {
            return Err(unsupported(
                expression,
                "at most 16 active function-value cursors are admitted",
            ));
        }
        self.active_function_values += 1;
        let result = self
            .evaluate(expression, region, depth, evaluate::Mode::Function(arity))
            .map(|value| value.expect("function evaluation result"));
        self.active_function_values -= 1;
        result
    }

    pub(super) fn function_value_leaf(
        &mut self,
        node: &Expression<'_>,
        region: &mut Region,
        arity: usize,
        depth: usize,
    ) -> Result<ValueId, String> {
        if let Expression::CallExpression(call) = node {
            if self
                .closure_target(invocation::callee(call), invocation::depth(call, depth + 1))
                .is_none()
            {
                self.runtime_function_result(call, region, arity, depth)
            } else if let Some(closure) = self.factory_result(node, region, depth)? {
                self.materialize_closure(closure, region, node, arity)
            } else {
                self.local_function_result(call, region, arity, depth)
            }
        } else if let Some((slot, actual)) = self.runtime_closure_callee(node, depth) {
            if actual != arity {
                return Err(unsupported(node, "function value arity mismatch"));
            }
            let result = self.value(Type::Closure(arity));
            Self::push(region, node, Op::Load { result, slot });
            Ok(result)
        } else {
            let closure = match self.closure_target(node, depth) {
                Some(closure) => closure,
                None => self
                    .factory_result(node, region, depth)?
                    .ok_or_else(|| unsupported(node, "expected a known function constructor"))?,
            };
            self.materialize_closure(closure, region, node, arity)
        }
    }

    fn runtime_function_result(
        &mut self,
        call: &CallExpression<'_>,
        region: &mut Region,
        expected: usize,
        depth: usize,
    ) -> Result<ValueId, String> {
        if self.active_function_results >= 8 {
            return Err(unsupported(
                call,
                "at most 8 active runtime function-result calls are admitted",
            ));
        }
        self.active_function_results += 1;
        let result = (|| {
            let signature = self
                .function_arity(invocation::callee(call), invocation::depth(call, depth + 1))
                .ok_or_else(|| unsupported(call, "runtime factory requires a function callee"))?;
            let (parameter_count, result_type) = {
                let signatures = self.closure_signatures.borrow();
                (
                    signatures
                        .parameter_count(signature)
                        .expect("checked signature"),
                    signatures.result(signature).expect("checked signature"),
                )
            };
            if call.optional
                || invocation::arguments(call).len() < parameter_count
                || call.arguments.len() > 1000
            {
                return Err(unsupported(
                    call,
                    "runtime factory requires a plain call, all declared inputs and at most 1000 arguments",
                ));
            }
            if result_type != Type::Closure(expected) {
                return Err(unsupported(
                    call,
                    "runtime factory result signature mismatch",
                ));
            }
            let callee = self.function_value(
                invocation::callee(call),
                region,
                signature,
                invocation::depth(call, depth + 1),
            )?;
            let mut arguments = Vec::new();
            for (index, argument) in call.arguments.iter().enumerate() {
                let offset = invocation::offset(call);
                let expression = argument
                    .as_expression()
                    .ok_or_else(|| unsupported(call, "spread call arguments are not admitted"))?;
                if index < offset {
                    self.evaluate(expression, region, depth + 1, evaluate::Mode::Discard)?;
                    continue;
                }
                let index = index - offset;
                if index >= parameter_count {
                    self.evaluate(expression, region, depth + 1, evaluate::Mode::Discard)?;
                    continue;
                }
                let parameter = self
                    .closure_signatures
                    .borrow()
                    .parameter(signature, index)
                    .expect("checked signature");
                let value = if let Type::Closure(callback) = parameter {
                    self.callback_argument(expression, region, callback, depth + 1)?
                } else if parameter.is_managed() {
                    let mode = match parameter {
                        Type::ManagedObject => evaluate::Mode::Object,
                        Type::ManagedArray => evaluate::Mode::Array,
                        _ => evaluate::Mode::Managed,
                    };
                    self.evaluate(expression, region, depth + 1, mode)?
                        .expect("managed runtime factory argument")
                } else if parameter == Type::AsyncValue {
                    self.evaluate(expression, region, depth + 1, evaluate::Mode::AsyncValue)?
                        .expect("mixed runtime factory argument")
                } else if parameter == Type::Promise {
                    self.evaluate(expression, region, depth + 1, evaluate::Mode::Pending)?
                        .expect("pending Promise runtime factory argument")
                } else if parameter == Type::FulfilledPromise {
                    self.evaluate(expression, region, depth + 1, evaluate::Mode::Promise)?
                        .expect("Promise runtime factory argument")
                } else if parameter == Type::General {
                    self.evaluate(expression, region, depth + 1, evaluate::Mode::General)?
                        .expect("General runtime factory argument")
                } else {
                    self.expression(expression, region, depth + 1)?
                };
                arguments.push(value);
            }
            self.charge_closure_expansion(call)?;
            let result = self.value(result_type);
            Self::push(
                region,
                call,
                Op::ClosureCall {
                    result,
                    callee,
                    arguments,
                },
            );
            Ok(result)
        })();
        self.active_function_results -= 1;
        result
    }

    fn local_function_result(
        &mut self,
        call: &CallExpression<'_>,
        region: &mut Region,
        arity: usize,
        depth: usize,
    ) -> Result<ValueId, String> {
        if self.active_function_results >= 8 {
            return Err(unsupported(
                call,
                "at most 8 active local function-result calls are admitted",
            ));
        }
        self.active_function_results += 1;
        let result = (|| {
            let mut closure = self
                .closure_target(invocation::callee(call), invocation::depth(call, depth + 1))
                .ok_or_else(|| {
                    unsupported(call, "function results require a known local callee")
                })?;
            if !closure.checked {
                self.check_local_closure(&mut closure)?;
                self.capture_cells(&closure, region);
            }
            self.check_local_call(call, &closure)?;
            self.charge_closure_expansion(call)?;
            let arrow = self.arrow(closure.arrow);
            let mut arguments = Vec::new();
            for (index, argument) in call.arguments.iter().enumerate() {
                let offset = invocation::offset(call);
                let expression = argument
                    .as_expression()
                    .ok_or_else(|| unsupported(call, "spread call arguments are not admitted"))?;
                if index < offset {
                    self.evaluate(expression, region, depth + 1, evaluate::Mode::Discard)?;
                    continue;
                }
                let index = index - offset;
                if index >= arrow.params.items.len() {
                    self.evaluate(expression, region, depth + 1, evaluate::Mode::Discard)?;
                    continue;
                }
                let value = if let Some(kind) = self.function_arity(expression, depth + 1) {
                    self.callback_argument(expression, region, kind, depth + 1)?
                } else {
                    let mode =
                        self.local_arrow_argument_mode(closure.arrow, index, expression, depth + 1);
                    self.evaluate(expression, region, depth + 1, mode)?
                        .expect("factory scalar argument")
                };
                arguments.push(value);
            }
            // Only after all caller arguments finish do fresh formal bindings
            // replace the caller's. Returned values are snapshots of these slots.
            let previous = std::mem::replace(&mut self.bindings, closure.captures);
            let charging = std::mem::replace(&mut self.source_charging, false);
            let result = (|| {
                self.bind_arrow_parameters(arrow, arguments, region)?;
                if let Some(body) = arrow.get_expression() {
                    self.function_value(body, region, arity, 0)
                } else {
                    let parent_depth = (self.inline_depth + self.statement_depth)
                        .max(self.function_result_depth)
                        + 1;
                    let body =
                        self.arrow_block_result(arrow, parent_depth, Type::Closure(arity))?;
                    let result = self.value(Type::Closure(arity));
                    Self::push(region, call, Op::InlineCall { result, body });
                    Ok(result)
                }
            })();
            self.bindings = previous;
            self.source_charging = charging;
            result
        })();
        self.active_function_results -= 1;
        result
    }

    fn materialize_closure(
        &mut self,
        mut closure: closures::LocalClosure,
        region: &mut Region,
        node: &Expression<'_>,
        arity: usize,
    ) -> Result<ValueId, String> {
        // Check before scratch validation can enter another source body.
        if self.lifting_depth + self.active_block_calls >= 8 {
            return Err(unsupported(
                node,
                "at most 8 active runtime-closure lowering frames are admitted",
            ));
        }
        if !closure.checked {
            self.check_local_closure(&mut closure)?;
        }
        let arrow = self.arrow(closure.arrow);
        if self.arrow_signature(arrow) != Some(arity) {
            return Err(unsupported(node, "function value signature mismatch"));
        }
        if self.lifted_base + self.lifted.len() >= self.functions.len() + 256 {
            return Err(unsupported(
                node,
                "at most 256 lifted closure bodies are admitted",
            ));
        }
        let return_type = self
            .closure_signatures
            .borrow()
            .result(arity)
            .expect("checked signature");
        let asynchronous = self.closure_signatures.borrow().is_async(arity);
        if return_type.is_managed()
            && closure
                .captures
                .iter()
                .any(|binding| captured_for_let_header(binding.symbol, self.semantic))
        {
            return Err(unsupported(
                node,
                "managed closure results over for-loop lexical headers require per-iteration environments (#334)",
            ));
        }
        if !match return_type {
            Type::ManagedObject
            | Type::ManagedArray
            | Type::ManagedValue
            | Type::Number
            | Type::General
            | Type::AsyncValue
            | Type::FulfilledPromise
            | Type::SettledPromise
            | Type::Promise => true,
            Type::Closure(signature) => self
                .closure_signatures
                .borrow()
                .is_returned_function(signature),
            _ => false,
        } {
            return Err(unsupported(
                node,
                "runtime factories require Number or finite scalar-result callback parameters in returned functions",
            ));
        }
        self.charge_closure_expansion(node)?;
        let mut child = self.clone();
        child.bindings.clear();
        child.slots.clear();
        child.values.clear();
        child.next_region = 0;
        child.return_type = return_type;
        child.allow_await = asynchronous;
        child.source_charging = false;
        child.active_local_calls = 0;
        child.active_block_calls = 0;
        child.closure_expansions = 0;
        child.inline_depth = 0;
        child.statement_depth = 0;
        child.function_result_depth = 0;
        child.in_loop_body = false;
        child.lifting_depth += 1;
        let parameter_types = callbacks::parameter_types(
            arrow,
            self.callback_parameters,
            callbacks::ValueInputs {
                general: self.general_parameters,
                managed: &self.managed_kinds.inputs,
            },
            self.promise_parameters,
        );
        for (parameter, ty) in arrow.params.items.iter().zip(parameter_types) {
            let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern else {
                unreachable!("checked closure parameter")
            };
            let slot = SlotId(child.slots.len());
            child.slots.push(Slot {
                ty,
                owner: RegionId(0),
                mutable: true,
                parameter: true,
                span: parameter.span(),
            });
            child.bindings.push(NumericBinding {
                symbol: identifier
                    .symbol_id
                    .get()
                    .ok_or("missing lifted parameter")?,
                slot,
                mutable: true,
            });
        }
        let mut mapping = BTreeMap::new();
        let mut imported = Vec::new();
        let mut capture_bindings = Vec::new();
        for capture in &closure.captures {
            let slot = SlotId(child.slots.len());
            mapping.insert(capture.slot, slot);
            imported.push(slot);
            child.slots.push(Slot {
                ty: self.slots[capture.slot.0].ty,
                owner: RegionId(0),
                mutable: capture.mutable,
                parameter: false,
                span: self.slots[capture.slot.0].span,
            });
            capture_bindings.push(NumericBinding {
                symbol: capture.symbol,
                slot,
                mutable: capture.mutable,
            });
        }
        // Static callees retain code identity, with every external capture remapped
        // to this function's imported cells. Unused inaccessible metadata is omitted.
        child.closures = self
            .closures
            .iter()
            .filter_map(|original| {
                let mut remapped = original.clone();
                for binding in &mut remapped.captures {
                    binding.slot = *mapping.get(&binding.slot)?;
                }
                Some(remapped)
            })
            .collect();
        let mut body = child.region(arrow);
        Self::push(
            &mut body,
            arrow,
            if asynchronous {
                Op::AsyncClosureEntry { captures: imported }
            } else {
                Op::ClosureEntry { captures: imported }
            },
        );
        child.initialize_general_parameters(&mut body);
        child.bindings.extend(capture_bindings);
        let returned = if asynchronous {
            if let Some(expression) = arrow.get_expression() {
                child.async_result(expression, &mut body, 0)?
            } else {
                let region = child.arrow_block_result(arrow, 0, Type::General)?;
                let result = child.value(Type::General);
                Self::push(
                    &mut body,
                    arrow,
                    Op::InlineCall {
                        result,
                        body: region,
                    },
                );
                result
            }
        } else if arrow.r#async
            && (return_type == Type::SettledPromise
                || (return_type == Type::Promise
                    && (awaited::owns_throw(arrow, self.semantic)
                        || self.settled_results.contains(&arrow.node_id.get()))))
        {
            // Close the primitive completion synchronously. Materialization below
            // adds no job; the caller's await schedules its reaction.
            child.allow_await = true;
            let region = if let Some(expression) = arrow.get_expression() {
                let mut region = child.region(arrow);
                let value = child
                    .evaluate(expression, &mut region, 0, evaluate::Mode::General)?
                    .expect("primitive settlement");
                region.exit = Exit::Return(value);
                region
            } else {
                child.arrow_block_result(arrow, 0, Type::General)?
            };
            child.allow_await = false;
            let result = child.value(Type::SettledPromise);
            Self::push(
                &mut body,
                arrow,
                Op::SettlePromise {
                    result,
                    body: region,
                },
            );
            if return_type == Type::Promise {
                let input = result;
                let result = child.value(Type::Promise);
                Self::push(&mut body, arrow, Op::MaterializePromise { result, input });
                result
            } else {
                result
            }
        } else if arrow.r#async {
            let input = if let Some(expression) = arrow.get_expression() {
                child
                    .evaluate(expression, &mut body, 0, evaluate::Mode::General)?
                    .expect("eager async primitive completion")
            } else {
                let region = child.arrow_block_result(arrow, 0, Type::General)?;
                let result = child.value(Type::General);
                Self::push(
                    &mut body,
                    arrow,
                    Op::InlineCall {
                        result,
                        body: region,
                    },
                );
                result
            };
            let result = child.value(Type::FulfilledPromise);
            Self::push(&mut body, arrow, Op::FulfillPromise { result, input });
            if return_type == Type::Promise {
                let input = result;
                let result = child.value(Type::Promise);
                Self::push(&mut body, arrow, Op::MaterializePromise { result, input });
                result
            } else {
                result
            }
        } else if let Some(expression) = arrow.get_expression() {
            if let Type::Closure(signature) = return_type {
                child.function_value(expression, &mut body, signature, 0)?
            } else if return_type == Type::AsyncValue {
                child
                    .evaluate(expression, &mut body, 0, evaluate::Mode::AsyncValue)?
                    .expect("raw private mixed result")
            } else if return_type == Type::Promise {
                child
                    .evaluate(expression, &mut body, 0, evaluate::Mode::Pending)?
                    .expect("forwarded pending Promise identity")
            } else if return_type == Type::SettledPromise {
                child
                    .evaluate(expression, &mut body, 0, evaluate::Mode::Settled)?
                    .expect("forwarded settled result")
            } else if return_type == Type::FulfilledPromise {
                child
                    .evaluate(expression, &mut body, 0, evaluate::Mode::Promise)?
                    .expect("fulfilled Promise closure result")
            } else if return_type.is_managed() {
                child
                    .evaluate(
                        expression,
                        &mut body,
                        0,
                        evaluate::Mode::managed(return_type),
                    )?
                    .expect("managed private result")
            } else if return_type == Type::General {
                child
                    .evaluate(expression, &mut body, 0, evaluate::Mode::General)?
                    .expect("General result")
            } else {
                child.expression(expression, &mut body, 0)?
            }
        } else {
            let region = child.arrow_block_result(arrow, 0, return_type)?;
            let result = child.value(return_type);
            Self::push(
                &mut body,
                arrow,
                Op::InlineCall {
                    result,
                    body: region,
                },
            );
            result
        };
        body.exit = Exit::Return(returned);
        self.lifted = std::mem::take(&mut child.lifted);
        // Nested construction may have consumed the remaining module capacity.
        if self.lifted_base + self.lifted.len() >= self.functions.len() + 256 {
            return Err(unsupported(
                node,
                "at most 256 lifted closure bodies are admitted",
            ));
        }
        let function = ir::FunctionId(self.lifted_base + self.lifted.len());
        self.lifted.push(std::rc::Rc::new(
            child.finish(body, arrow.params.items.len()),
        ));
        let result = self.value(Type::Closure(arity));
        Self::push(
            region,
            node,
            Op::MakeClosure {
                result,
                function,
                captures: closure
                    .captures
                    .iter()
                    .map(|binding| binding.slot)
                    .collect(),
            },
        );
        Ok(result)
    }
}

// The shared header-environment gap is tracked in #334. New managed-return
// function values must not silently inherit the older numeric-path miscompile.
fn captured_for_let_header(symbol: SymbolId, semantic: &Semantic<'_>) -> bool {
    semantic.scoping().symbol_declarations(symbol).any(|node| {
        if !matches!(
            semantic.nodes().kind(node),
            oxc_ast::AstKind::VariableDeclarator(_)
        ) {
            return false;
        }
        let mut ancestors = semantic.nodes().ancestor_kinds(node);
        let Some(oxc_ast::AstKind::VariableDeclaration(declaration)) = ancestors.next() else {
            return false;
        };
        if declaration.kind != oxc_ast::ast::VariableDeclarationKind::Let {
            return false;
        }
        let Some(oxc_ast::AstKind::ForStatement(statement)) = ancestors.next() else {
            return false;
        };
        statement
            .init
            .as_ref()
            .is_some_and(|init| init.span() == declaration.span)
    })
}
