//! Effect-free function signature queries and constructor/call leaves for the
//! shared typed evaluator. Queries never execute source arguments or conditions.
use super::*;
use oxc_ast::ast::ArrowFunctionExpression;
use std::collections::BTreeMap;

#[cfg(test)]
mod optional_tests;

/// Function-valued result calls have one shared eight-active-call budget.
/// Admission is a move-only token so a heap continuation can own the
/// charge until its argument, body and result phases finish or unwind.
#[derive(Clone, Copy)]
enum FunctionResultKind {
    Local,
    Runtime,
}

struct FunctionResultActivation {
    _kind: FunctionResultKind,
}

/// Runtime factory admission spans callee construction, ordered arguments and
/// the final Function-valued call. Source validation completes before effects.
pub(super) struct PreparedRuntimeFactory<'n, 'a> {
    admission: FunctionResultActivation,
    call: &'n CallExpression<'a>,
    depth: usize,
    signature: usize,
    parameter_count: usize,
    result_type: Type,
}

impl<'n, 'a> PreparedRuntimeFactory<'n, 'a> {
    pub(super) fn call(&self) -> (&'n CallExpression<'a>, usize) {
        (self.call, self.depth)
    }

    pub(super) fn signature(&self) -> usize {
        self.signature
    }
}

/// An admitted local factory keeps both its eight-active-function-result
/// charge and the caller's lexical state while its source body is suspended.
pub(super) struct LocalFunctionResult {
    admission: FunctionResultActivation,
    bindings: closures::FactoryBindings,
}

/// Arguments run in the caller's environment while this token keeps the
/// function-result admission and the selected source closure alive.
pub(super) struct PreparedFunctionResult {
    admission: FunctionResultActivation,
    closure: closures::LocalClosure,
}

/// Local result admission precedes definition checking and remains owned until
/// checked call validation produces argument preparation or rejects the call.
pub(super) struct LocalResultPreparation {
    admission: FunctionResultActivation,
}

impl PreparedFunctionResult {
    pub(super) fn arrow(&self) -> oxc_syntax::node::NodeId {
        self.closure.arrow
    }
}

/// These factories share ordered caller arguments. Local and runtime results
/// own an active-result admission; runtime arguments also retain their callee
/// snapshot, constructed before the first argument starts.
pub(super) enum PreparedFactory<'n, 'a> {
    LocalResult(PreparedFunctionResult),
    ClosureMetadata(closures::PreparedConciseFactory<'n, 'a>),
    Runtime {
        factory: PreparedRuntimeFactory<'n, 'a>,
        callee: ValueId,
    },
}

/// Body entry retains the distinct context of each local function-result path.
/// A concise expression owns an ordinary function-value admission and runs in
/// the caller's Region; a block owns its existing arrow activation and Region.
pub(super) enum FunctionResultBody<'n, 'a> {
    Block {
        activation: closures::BlockArrowActivation,
        request: closures::ArrowBodyRequest<'n, 'a>,
    },
    Expression(&'n Expression<'a>),
}

/// A source-owned leaf selects checking, argument or materialization work, or
/// completes a direct module/slot value. Selection is consumed once, including
/// None results from concise-factory preparation.
pub(super) enum FunctionLeafResult<'n, 'a> {
    Factory(PreparedFactory<'n, 'a>),
    LocalFactory(closures::LocalClosure),
    CheckConciseFactory {
        candidate: closures::ConciseFactoryCandidate<'n, 'a>,
        closure: closures::LocalClosure,
        fallback: closures::LocalClosure,
    },
    RuntimeFactory(PreparedRuntimeFactory<'n, 'a>),
    Materialize(closures::LocalClosure),
    Value(ValueId),
}

/// A real lifted body retains its checked closure and result policy until the
/// parent publishes its nested bodies followed by the enclosing function.
pub(super) struct LiftedClosure {
    closure: closures::LocalClosure,
    signature: usize,
    return_type: Type,
    asynchronous: bool,
}

/// Real materialization starts from an owned child with fresh function IR and
/// remapped captured cells. It is distinct from disposable definition checking.
pub(super) struct PreparedLiftedClosure<'s, 'a> {
    pub(super) lowering: Box<Lowering<'s, 'a>>,
    pub(super) body: Region,
    pub(super) completion: LiftedClosure,
}

#[derive(Clone, Copy)]
enum LiftedBodyKind {
    Direct {
        mode: evaluate::Mode,
        result_type: Type,
    },
    Fulfill {
        materialize: bool,
    },
    Settle {
        materialize: bool,
    },
}

/// Body completion owns the entry prefix when a block or settlement runs in a
/// separate Region. The returned value remains in the active lifted child.
pub(super) struct LiftedBodyCompletion {
    arrow: oxc_syntax::node::NodeId,
    kind: LiftedBodyKind,
    prefix: Option<Region>,
}

pub(super) enum LiftedBodyRequest<'a> {
    Expression {
        source: &'a Expression<'a>,
        output: Region,
        mode: evaluate::Mode,
    },
    Block {
        arrow: &'a ArrowFunctionExpression<'a>,
        result_type: Type,
    },
}

enum NonCallFunctionValue {
    Value(ValueId),
    Materialize(closures::LocalClosure),
}

impl<'s, 'a> Lowering<'s, 'a> {
    pub(super) fn named_module_function(
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
            .find(|f| f.symbol == symbol)?
            .module_value
    }

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
        // A => B => C creates a mutual native recursion seam
        // through function_arity_in, even though both helpers have admission
        // limits. Keep the direct concise-arrow chain on the heap.
        if !arrow.r#async
            && calls + 1 < 8
            && matches!(
                arrow.get_expression(),
                Some(Expression::ArrowFunctionExpression(_))
            )
        {
            return self.arrow_signature_concise_chain(arrow, calls, outer);
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

    #[inline(never)]
    fn arrow_signature_concise_chain<'n, 'ast>(
        &self,
        arrow: &'n oxc_ast::ast::ArrowFunctionExpression<'ast>,
        calls: usize,
        outer: &BTreeMap<SymbolId, Option<usize>>,
    ) -> Option<usize> {
        struct Frame<'n, 'ast> {
            arrow: &'n oxc_ast::ast::ArrowFunctionExpression<'ast>,
            parameters: Vec<Type>,
            bindings: BTreeMap<SymbolId, Option<usize>>,
            calls: usize,
        }

        let mut pending: Vec<Frame<'n, 'ast>> = Vec::new();
        let mut current = arrow;
        let mut current_calls = calls;
        let mut outer = outer.clone();
        let mut resolved = loop {
            // Let the normal query handle a leaf, an async arrow, or the
            // existing eight-call boundary. It never receives a direct chain
            // at a lower calls value here, so this method cannot reenter itself.
            if current.r#async || current_calls + 1 >= 8 {
                break self.arrow_signature_at(current, current_calls, &outer);
            }
            let Some(Expression::ArrowFunctionExpression(child)) = current.get_expression() else {
                break self.arrow_signature_at(current, current_calls, &outer);
            };

            let parameters = callbacks::parameter_types(
                current,
                self.callback_parameters,
                callbacks::ValueInputs {
                    general: self.general_parameters,
                    managed: &self.managed_kinds.inputs,
                },
                self.promise_parameters,
            );
            // Match arrow_signature_at's lexical binding precedence. The
            // current arrow's formals shadow the caller's captured signatures.
            let next_bindings = (|| {
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
                for (symbol, signature) in &outer {
                    bindings.entry(*symbol).or_insert(*signature);
                }
                for (parameter, ty) in current.params.items.iter().zip(&parameters) {
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
                Some(bindings)
            })();
            let Some(bindings) = next_bindings else {
                break None;
            };
            pending.push(Frame {
                arrow: current,
                parameters,
                bindings: bindings.clone(),
                calls: current_calls,
            });
            outer = bindings;
            current = child;
            current_calls += 1;
        };

        // Recursive signature inference interns the innermost arrow first.
        // Reconstruct parents in exactly that order, including the scalar
        // fallback when a child does not prove a complete function signature.
        while let Some(frame) = pending.pop() {
            let mut result = if let Some(signature) = resolved {
                Type::Closure(signature)
            } else {
                self.arrow_scalar_result_at(frame.arrow, &frame.bindings, frame.calls)
            };
            if result == Type::FulfilledPromise
                && self.settled_results.contains(&frame.arrow.node_id.get())
            {
                result = Type::SettledPromise;
            }
            resolved = Some(
                self.closure_signatures
                    .borrow_mut()
                    .intern(frame.parameters, result),
            );
        }
        resolved
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

    /// An owned source continuation holds callback admission until its
    /// Function-mode expression finishes.
    pub(super) fn enter_callback_argument(
        &mut self,
        expression: &Expression<'_>,
    ) -> Result<(), String> {
        // Preserve the callback-specific diagnostic before function-value
        // admission, including when an argument suspends into a source body.
        if self.active_function_values >= 8 {
            return Err(unsupported(
                expression,
                "callback arguments require fewer than 8 active function-value cursors",
            ));
        }
        self.enter_function_value(expression)
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
                        Expression::StaticMemberExpression(_)
                        | Expression::ComputedMemberExpression(_) => {
                            let signature =
                                self.stored.functions(node.node_id()).and_then(|origins| {
                                    let mut expected = None;
                                    for origin in origins {
                                        let next = match origin {
                                            stored_functions::Origin::Arrow(body) => self
                                                .arrow_signature_at(
                                                    self.arrow(*body),
                                                    calls,
                                                    &bindings,
                                                ),
                                            stored_functions::Origin::Named(symbol) => self
                                                .functions
                                                .iter()
                                                .find(|f| f.symbol == *symbol)
                                                .and_then(|f| {
                                                    f.module_value.map(|(_, signature)| signature)
                                                }),
                                            _ => None,
                                        }?;
                                        if expected.is_some_and(|old| old != next) {
                                            return None;
                                        }
                                        expected = Some(next);
                                    }
                                    expected
                                });
                            results.push(signature);
                        }
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
                            } else if let Some((_, arity)) = self
                                .named_module_function(node, depth)
                                .or_else(|| self.named_async_function(node, depth))
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

    /// Source continuations own function-value admission through expression
    /// completion and restore it on every diagnostic path.
    pub(super) fn enter_function_value(
        &mut self,
        expression: &Expression<'_>,
    ) -> Result<(), String> {
        if self.active_function_values >= 16 {
            return Err(unsupported(
                expression,
                "at most 16 active function-value cursors are admitted",
            ));
        }
        self.active_function_values += 1;
        Ok(())
    }

    pub(super) fn leave_function_value(&mut self) {
        self.active_function_values -= 1;
    }

    pub(super) fn source_function_value_leaf<'n>(
        &mut self,
        node: &'n Expression<'a>,
        region: &mut Region,
        arity: usize,
        depth: usize,
    ) -> Result<FunctionLeafResult<'n, 'a>, String> {
        let Expression::CallExpression(call) = node else {
            return self
                .prepare_non_call_function_value(node, region, arity, depth)
                .map(|prepared| match prepared {
                    NonCallFunctionValue::Value(value) => FunctionLeafResult::Value(value),
                    NonCallFunctionValue::Materialize(closure) => {
                        FunctionLeafResult::Materialize(closure)
                    }
                });
        };
        let Some(closure) =
            self.closure_target(invocation::callee(call), invocation::depth(call, depth + 1))
        else {
            return self
                .prepare_runtime_factory(call, arity, depth)
                .map(FunctionLeafResult::RuntimeFactory);
        };
        if let Some((candidate, factory)) = self.select_concise_factory(node, depth) {
            if !factory.checked {
                return Ok(FunctionLeafResult::CheckConciseFactory {
                    candidate,
                    closure: factory,
                    fallback: closure,
                });
            }
            if let Some(factory) = self.finish_concise_factory_preparation(candidate, factory)? {
                return Ok(FunctionLeafResult::Factory(
                    PreparedFactory::ClosureMetadata(factory),
                ));
            }
        }
        // Metadata preparation may publish definition costs/capture operations,
        // but it leaves the caller's lexical targets unchanged. Retain the known
        // callee and preserve metadata precedence without a compatibility retry.
        Ok(FunctionLeafResult::LocalFactory(closure))
    }

    /// Select non-call constructors without entering their checking or real
    /// body. Module and runtime slots keep their original immediate operations.
    fn prepare_non_call_function_value(
        &mut self,
        node: &Expression<'_>,
        region: &mut Region,
        arity: usize,
        depth: usize,
    ) -> Result<NonCallFunctionValue, String> {
        debug_assert!(
            !matches!(
                node,
                Expression::CallExpression(_) | Expression::ParenthesizedExpression(_)
            ) && erased_inner(node).is_none(),
            "source function leaves are unwrapped before non-call selection"
        );
        if let Some((function, actual)) = self.named_module_function(node, depth) {
            if actual != arity {
                return Err(unsupported(
                    node,
                    "module function requires its complete signature",
                ));
            }
            let result = self.value(Type::Closure(arity));
            Self::push(region, node, Op::ModuleClosure { result, function });
            Ok(NonCallFunctionValue::Value(result))
        } else if let Some((slot, actual)) = self.runtime_closure_callee(node, depth) {
            if actual != arity {
                return Err(unsupported(node, "function value arity mismatch"));
            }
            let result = self.value(Type::Closure(arity));
            Self::push(region, node, Op::Load { result, slot });
            Ok(NonCallFunctionValue::Value(result))
        } else {
            // Work::Eval consumes erased/parenthesized wrappers before the
            // Function leaf, and direct calls selected a factory above. No
            // effectful factory fallback belongs to this non-call branch.
            let closure = self
                .closure_target(node, depth)
                .ok_or_else(|| unsupported(node, "expected a known function constructor"))?;
            Ok(NonCallFunctionValue::Materialize(closure))
        }
    }

    /// Keep resource charging separate from the native function-result frame.
    /// Both kinds share the same counter, but preserve their original first
    /// diagnostic when the limit is exceeded.
    fn begin_function_result(
        &mut self,
        call: &CallExpression<'_>,
        kind: FunctionResultKind,
    ) -> Result<FunctionResultActivation, String> {
        if self.active_function_results >= 8 {
            return Err(unsupported(
                call,
                match kind {
                    FunctionResultKind::Local => {
                        "at most 8 active local function-result calls are admitted"
                    }
                    FunctionResultKind::Runtime => {
                        "at most 8 active runtime function-result calls are admitted"
                    }
                },
            ));
        }
        self.active_function_results += 1;
        Ok(FunctionResultActivation { _kind: kind })
    }

    fn finish_function_result(&mut self, _activation: FunctionResultActivation) {
        self.active_function_results -= 1;
    }

    /// Read-only selection for a direct block-bodied local factory call.
    /// Other callees use the general function-value request; this preserves the
    /// existing direct-block specialization and its admission order.
    pub(super) fn block_factory_target(
        &self,
        node: &Expression<'_>,
        depth: usize,
    ) -> Option<closures::LocalClosure> {
        let Expression::CallExpression(call) = node else {
            return None;
        };
        if call.optional || invocation::is_method(call) {
            return None;
        }
        self.closure_target(invocation::callee(call), invocation::depth(call, depth + 1))
            .filter(|closure| self.arrow(closure.arrow).get_expression().is_none())
    }

    pub(super) fn begin_local_result_preparation(
        &mut self,
        call: &CallExpression<'_>,
    ) -> Result<LocalResultPreparation, String> {
        self.begin_function_result(call, FunctionResultKind::Local)
            .map(|admission| LocalResultPreparation { admission })
    }

    /// Checking/capture publication has completed before call validation. This
    /// consumes the preparation token on both validation failure and success.
    pub(super) fn finish_local_result_preparation(
        &mut self,
        preparation: LocalResultPreparation,
        call: &CallExpression<'_>,
        closure: closures::LocalClosure,
    ) -> Result<PreparedFunctionResult, String> {
        let checked = self
            .check_local_call(call, &closure)
            .and_then(|()| self.charge_closure_expansion(call));
        if let Err(error) = checked {
            self.abort_local_result_preparation(preparation);
            return Err(error);
        }
        Ok(PreparedFunctionResult {
            admission: preparation.admission,
            closure,
        })
    }

    pub(super) fn abort_local_result_preparation(&mut self, preparation: LocalResultPreparation) {
        self.finish_function_result(preparation.admission);
    }

    pub(super) fn abort_function_result_arguments(&mut self, factory: PreparedFunctionResult) {
        self.finish_function_result(factory.admission);
    }

    /// All caller arguments have completed. Install fresh parameters in the
    /// captured environment, then yield the body to the existing source cursor.
    /// Consuming the preparation token also makes this helper own every error
    /// before the caller can publish a pending body frame.
    pub(super) fn begin_function_result_body<'n>(
        &mut self,
        factory: PreparedFunctionResult,
        arguments: Vec<ValueId>,
        region: &mut Region,
        arity: usize,
    ) -> Result<(LocalFunctionResult, FunctionResultBody<'n, 'a>), String>
    where
        'a: 'n,
    {
        let PreparedFunctionResult { admission, closure } = factory;
        let arrow = self.arrow(closure.arrow);
        let bindings = self.enter_factory_bindings(closure.captures);
        let started = (|| {
            self.bind_arrow_parameters(arrow, arguments, region)?;
            if let Some(body) = arrow.get_expression() {
                self.enter_function_value(body)?;
                Ok(FunctionResultBody::Expression(body))
            } else {
                let parent_depth =
                    (self.inline_depth + self.statement_depth).max(self.function_result_depth) + 1;
                let (activation, request) =
                    self.begin_arrow_block_activation(arrow, parent_depth, Type::Closure(arity))?;
                Ok(FunctionResultBody::Block {
                    activation,
                    request,
                })
            }
        })();
        match started {
            Ok(body) => Ok((
                LocalFunctionResult {
                    admission,
                    bindings,
                },
                body,
            )),
            Err(error) => {
                self.restore_factory_bindings(bindings);
                self.finish_function_result(admission);
                Err(error)
            }
        }
    }

    /// After body completion (or on source diagnostic unwind), restore
    /// the caller's lexical state and then release the function-result charge.
    pub(super) fn finish_local_function_result(&mut self, factory: LocalFunctionResult) {
        self.restore_factory_bindings(factory.bindings);
        self.finish_function_result(factory.admission);
    }

    /// Acquire the runtime result charge before signature and call validation.
    /// A failed preparation restores only that charge; no callee or arguments
    /// have run and no output Region is retained by the returned token.
    fn prepare_runtime_factory<'n, 'ast>(
        &mut self,
        call: &'n CallExpression<'ast>,
        expected: usize,
        depth: usize,
    ) -> Result<PreparedRuntimeFactory<'n, 'ast>, String> {
        let admission = self.begin_function_result(call, FunctionResultKind::Runtime)?;
        let validated = (|| {
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
            Ok((signature, parameter_count, result_type))
        })();
        match validated {
            Ok((signature, parameter_count, result_type)) => Ok(PreparedRuntimeFactory {
                admission,
                call,
                depth,
                signature,
                parameter_count,
                result_type,
            }),
            Err(error) => {
                self.finish_function_result(admission);
                Err(error)
            }
        }
    }

    pub(super) fn abort_runtime_factory(&mut self, factory: PreparedRuntimeFactory<'_, '_>) {
        self.finish_function_result(factory.admission);
    }

    /// Local factory arguments infer their source kind. Runtime factories use
    /// the complete formal signature, queried only for the next argument after
    /// all preceding effects, including a discarded .call receiver.
    pub(super) fn classify_prepared_factory_argument(
        &self,
        factory: &PreparedFactory<'_, '_>,
        call: &CallExpression<'_>,
        index: usize,
        expression: &Expression<'_>,
        depth: usize,
    ) -> closures::FactoryArgumentKind {
        let arrow = match factory {
            PreparedFactory::LocalResult(factory) => factory.arrow(),
            PreparedFactory::ClosureMetadata(factory) => factory.arrow(),
            PreparedFactory::Runtime { factory, .. } => {
                return self.classify_runtime_factory_argument(factory, index);
            }
        };
        self.classify_factory_argument(call, arrow, index, expression, depth)
    }

    fn classify_runtime_factory_argument(
        &self,
        factory: &PreparedRuntimeFactory<'_, '_>,
        index: usize,
    ) -> closures::FactoryArgumentKind {
        let offset = invocation::offset(factory.call);
        if index < offset || index - offset >= factory.parameter_count {
            return closures::FactoryArgumentKind::Discard;
        }
        let parameter = self
            .closure_signatures
            .borrow()
            .parameter(factory.signature, index - offset)
            .expect("checked signature");
        if let Type::Closure(signature) = parameter {
            return closures::FactoryArgumentKind::Callback(signature);
        }
        let mode = if parameter.is_managed() {
            match parameter {
                Type::ManagedObject => evaluate::Mode::Object,
                Type::ManagedArray => evaluate::Mode::Array,
                _ => evaluate::Mode::Managed,
            }
        } else if parameter == Type::AsyncValue {
            evaluate::Mode::AsyncValue
        } else if parameter == Type::Promise {
            evaluate::Mode::Pending
        } else if parameter == Type::FulfilledPromise {
            evaluate::Mode::Promise
        } else if parameter == Type::General {
            evaluate::Mode::General
        } else {
            evaluate::Mode::Number
        };
        closures::FactoryArgumentKind::Scalar(mode)
    }

    /// Emission is charged only after all callee and argument effects finish.
    /// This consumes the admission token and restores it on evaluation errors,
    /// expansion-limit errors and successful Function-valued completion alike.
    pub(super) fn finish_runtime_factory(
        &mut self,
        factory: PreparedRuntimeFactory<'_, '_>,
        region: &mut Region,
        evaluated: Result<(ValueId, Vec<ValueId>), String>,
    ) -> Result<ValueId, String> {
        let result = evaluated.and_then(|(callee, arguments)| {
            self.charge_closure_expansion(factory.call)?;
            let result = self.value(factory.result_type);
            Self::push(
                region,
                factory.call,
                Op::ClosureCall {
                    result,
                    callee,
                    arguments,
                },
            );
            Ok(result)
        });
        self.finish_function_result(factory.admission);
        result
    }

    pub(super) fn admit_lifted_closure(&self, node: &Expression<'_>) -> Result<(), String> {
        // Check before scratch validation can enter another source body.
        if self.lifting_depth + self.active_block_calls >= 8 {
            return Err(unsupported(
                node,
                "at most 8 active runtime-closure lowering frames are admitted",
            ));
        }
        Ok(())
    }

    /// Called only after the initial lifting admission and definition check.
    /// Validation and expansion charging precede cloning the real body state.
    pub(super) fn prepare_lifted_closure(
        &mut self,
        closure: closures::LocalClosure,
        node: &Expression<'_>,
        arity: usize,
    ) -> Result<PreparedLiftedClosure<'s, 'a>, String> {
        let arrow = self.arrow(closure.arrow);
        if self.arrow_signature(arrow) != Some(arity) {
            return Err(unsupported(node, "function value signature mismatch"));
        }
        if self.lifted_base + self.lifted.len() >= self.lifted_limit() {
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
        if (return_type.is_managed() || self.managed_kinds.identity_bodies.contains(&closure.arrow))
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
        let mut child = Box::new(self.clone());
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
        Ok(PreparedLiftedClosure {
            lowering: child,
            body,
            completion: LiftedClosure {
                closure,
                signature: arity,
                return_type,
                asynchronous,
            },
        })
    }

    /// Choose the original body mode and Region ownership before scheduling.
    /// Only eager settlement changes await allowance; it restores that flag
    /// after the body has produced its original completion Region.
    pub(super) fn prepare_lifted_body(
        &mut self,
        completion: &LiftedClosure,
        body: Region,
    ) -> (LiftedBodyCompletion, LiftedBodyRequest<'a>) {
        let arrow = self.arrow(completion.closure.arrow);
        let kind = if completion.asynchronous {
            LiftedBodyKind::Direct {
                mode: evaluate::Mode::AsyncReturn,
                result_type: Type::General,
            }
        } else if arrow.r#async
            && (completion.return_type == Type::SettledPromise
                || (completion.return_type == Type::Promise
                    && (awaited::owns_throw(arrow, self.semantic)
                        || self.settled_results.contains(&arrow.node_id.get()))))
        {
            self.allow_await = true;
            LiftedBodyKind::Settle {
                materialize: completion.return_type == Type::Promise,
            }
        } else if arrow.r#async {
            LiftedBodyKind::Fulfill {
                materialize: completion.return_type == Type::Promise,
            }
        } else {
            let mode = match completion.return_type {
                Type::Closure(signature) => evaluate::Mode::Function(signature),
                Type::AsyncValue => evaluate::Mode::AsyncValue,
                Type::Promise => evaluate::Mode::Pending,
                Type::SettledPromise => evaluate::Mode::Settled,
                Type::FulfilledPromise => evaluate::Mode::Promise,
                ty if ty.is_managed() => evaluate::Mode::managed(ty),
                Type::General => evaluate::Mode::General,
                _ => evaluate::Mode::Number,
            };
            LiftedBodyKind::Direct {
                mode,
                result_type: completion.return_type,
            }
        };
        let (mode, result_type) = match kind {
            LiftedBodyKind::Direct { mode, result_type } => (mode, result_type),
            LiftedBodyKind::Fulfill { .. } | LiftedBodyKind::Settle { .. } => {
                (evaluate::Mode::General, Type::General)
            }
        };
        let (prefix, request) = if let Some(source) = arrow.get_expression() {
            let (prefix, output) = if matches!(kind, LiftedBodyKind::Settle { .. }) {
                (Some(body), self.region(arrow))
            } else {
                (None, body)
            };
            (
                prefix,
                LiftedBodyRequest::Expression {
                    source,
                    output,
                    mode,
                },
            )
        } else {
            (Some(body), LiftedBodyRequest::Block { arrow, result_type })
        };
        (
            LiftedBodyCompletion {
                arrow: completion.closure.arrow,
                kind,
                prefix,
            },
            request,
        )
    }

    /// Finish a concise value or a completed block in the still-active real
    /// child. InlineCall, settlement and fulfillment keep their allocation order.
    pub(super) fn finish_lifted_body(
        &mut self,
        completion: LiftedBodyCompletion,
        mut finished: Region,
        value: Option<ValueId>,
    ) -> Region {
        let arrow = self.arrow(completion.arrow);
        let (mut body, mut result) = if let Some(mut prefix) = completion.prefix {
            let result = if matches!(completion.kind, LiftedBodyKind::Settle { .. }) {
                if let Some(value) = value {
                    finished.exit = Exit::Return(value);
                }
                self.allow_await = false;
                let result = self.value(Type::SettledPromise);
                Self::push(
                    &mut prefix,
                    arrow,
                    Op::SettlePromise {
                        result,
                        body: finished,
                    },
                );
                result
            } else {
                debug_assert!(value.is_none(), "separate non-settlement Region is a block");
                let ty = match completion.kind {
                    LiftedBodyKind::Direct { result_type, .. } => result_type,
                    LiftedBodyKind::Fulfill { .. } => Type::General,
                    LiftedBodyKind::Settle { .. } => unreachable!("handled settlement"),
                };
                let result = self.value(ty);
                Self::push(
                    &mut prefix,
                    arrow,
                    Op::InlineCall {
                        result,
                        body: finished,
                    },
                );
                result
            };
            (prefix, result)
        } else {
            (finished, value.expect("concise lifted body value"))
        };
        if let LiftedBodyKind::Fulfill { .. } = completion.kind {
            let input = result;
            result = self.value(Type::FulfilledPromise);
            Self::push(&mut body, arrow, Op::FulfillPromise { result, input });
        }
        if matches!(
            completion.kind,
            LiftedBodyKind::Fulfill { materialize: true }
                | LiftedBodyKind::Settle { materialize: true }
        ) {
            let input = result;
            result = self.value(Type::Promise);
            Self::push(&mut body, arrow, Op::MaterializePromise { result, input });
        }
        body.exit = Exit::Return(result);
        body
    }

    /// A successful body publishes nested functions before the final capacity
    /// check. A late capacity error therefore keeps that nested pool, while a
    /// body error never reaches this completion and publishes no child IR.
    pub(super) fn finish_lifted_closure(
        &mut self,
        completion: LiftedClosure,
        mut child: Box<Lowering<'s, 'a>>,
        body: Region,
        region: &mut Region,
        node: &Expression<'_>,
    ) -> Result<ValueId, String> {
        let arrow = self.arrow(completion.closure.arrow);
        self.lifted = std::mem::take(&mut child.lifted);
        // Nested construction may have consumed the remaining module capacity.
        if self.lifted_base + self.lifted.len() >= self.lifted_limit() {
            return Err(unsupported(
                node,
                "at most 256 lifted closure bodies are admitted",
            ));
        }
        let function = ir::FunctionId(self.lifted_base + self.lifted.len());
        self.lifted.push(std::rc::Rc::new(
            child.finish(body, arrow.params.items.len()),
        ));
        let result = self.value(Type::Closure(completion.signature));
        Self::push(
            region,
            node,
            Op::MakeClosure {
                result,
                function,
                captures: completion
                    .closure
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

#[cfg(test)]
mod concise_signature_stack_tests {
    #[test]
    fn owned_function_result_admission_keeps_the_eight_call_boundary() {
        for (depth, admitted) in [(0, true), (3, true), (7, true), (8, false)] {
            let mut returned = String::from("()=>a");
            for _ in 0..depth {
                returned = format!("id({returned})");
            }
            for typescript in [false, true] {
                let source = format!(
                    "export function run(a,b){{const id=fn=>fn;const f={returned};return f();}}"
                );
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile(&source, typescript))
                    .expect("2 MiB returned-function admission thread")
                    .join()
                    .expect("returned-function cursor must not overflow native stack");
                assert_eq!(
                    result.is_ok(),
                    admitted,
                    "{depth} nested returned-function calls: {:?}",
                    result.err()
                );
            }
        }
    }

    #[test]
    fn nested_concise_arrow_signatures_do_not_reenter_native_stack() {
        for depth in [1, 2, 3, 5, 8] {
            let mut expression = String::from("a");
            for _ in 0..depth {
                expression = format!("()=>{expression}");
            }
            let calls = "()".repeat(depth);
            let source =
                format!("export function run(a,b){{const f={expression};return f{calls};}}");
            for typescript in [false, true] {
                let source = source.clone();
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile(&source, typescript))
                    .expect("small-stack signature query thread")
                    .join()
                    .expect("signature inference must not overflow native stack");
                if depth <= 3 {
                    assert!(
                        result.is_ok(),
                        "{depth} nested arrow results: {:?}",
                        result.err()
                    );
                }
            }
        }
    }
}
