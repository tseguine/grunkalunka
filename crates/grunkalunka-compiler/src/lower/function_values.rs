//! A separate explicit cursor for function-value constructors and assignments. Numeric
//! conditions/arguments finish in their cursor before construction continues.
use super::*;
use std::collections::BTreeMap;

enum Work<'e, 'a> {
    Eval(&'e Expression<'a>, usize),
    Consequent {
        node: &'e Expression<'a>,
        condition: ValueId,
        alternate: &'e Expression<'a>,
        depth: usize,
    },
    Alternate {
        node: &'e Expression<'a>,
        condition: ValueId,
        consequent: Region,
    },
    Store {
        node: &'e Expression<'a>,
        slot: SlotId,
    },
}

struct Regions<'r> {
    root: &'r mut Region,
    children: Vec<Region>,
}
impl Regions<'_> {
    fn current(&mut self) -> &mut Region {
        self.children.last_mut().unwrap_or(self.root)
    }
    fn finish(&mut self, value: ValueId) -> Region {
        let mut region = self.children.pop().expect("function child region");
        region.exit = Exit::Yield(value);
        region
    }
}

impl Lowering<'_, '_> {
    pub(super) fn arrow_signature(
        &self,
        arrow: &oxc_ast::ast::ArrowFunctionExpression<'_>,
    ) -> Option<usize> {
        self.arrow_signature_at(arrow, 0)
    }

    fn arrow_signature_at(
        &self,
        arrow: &oxc_ast::ast::ArrowFunctionExpression<'_>,
        calls: usize,
    ) -> Option<usize> {
        if calls >= 8 {
            return None;
        }
        let parameters = callbacks::parameter_types(arrow, self.callback_parameters);
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
        let result = arrow
            .get_expression()
            .and_then(|body| self.function_arity_in(body, 0, &bindings, calls + 1))
            .map_or(Type::Number, Type::Closure);
        Some(
            self.closure_signatures
                .borrow_mut()
                .intern(parameters, result),
        )
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

    fn function_assignment_target(
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
    pub(super) fn function_arity_in(
        &self,
        expression: &Expression<'_>,
        depth: usize,
        bindings: &BTreeMap<SymbolId, Option<usize>>,
        calls: usize,
    ) -> Option<usize> {
        let mut pending = vec![(expression, depth)];
        let mut arity = None;
        while let Some((node, depth)) = pending.pop() {
            if depth > 128 {
                return None;
            }
            if let Some(inner) = erased_inner(node) {
                pending.push((inner, depth + 1));
                continue;
            }
            let next = match node {
                Expression::ParenthesizedExpression(wrapper) => {
                    pending.push((&wrapper.expression, depth + 1));
                    continue;
                }
                Expression::ConditionalExpression(conditional) => {
                    pending.push((&conditional.alternate, depth + 1));
                    pending.push((&conditional.consequent, depth + 1));
                    continue;
                }
                Expression::SequenceExpression(sequence) => {
                    pending.push((sequence.expressions.last()?, depth + 1));
                    continue;
                }
                Expression::AssignmentExpression(assignment)
                    if assignment.operator == AssignmentOperator::Assign =>
                {
                    let identifier =
                        target_identifier(assignment.left.as_simple_assignment_target()?, depth)
                            .ok()??;
                    let symbol = self
                        .semantic
                        .scoping()
                        .get_reference(identifier.reference_id.get()?)
                        .symbol_id()?;
                    let target_arity = match bindings.get(&symbol) {
                        Some(kind) => (*kind)?,
                        None => self.function_assignment_target(assignment, depth)?.1,
                    };
                    if arity.is_some_and(|expected| expected != target_arity) {
                        return None;
                    }
                    arity = Some(target_arity);
                    pending.push((&assignment.right, depth + 1));
                    continue;
                }
                Expression::CallExpression(call) => {
                    if self.closure_target(&call.callee, depth + 1).is_none() {
                        if calls >= 8 {
                            return None;
                        }
                        let callee =
                            self.function_arity_in(&call.callee, depth + 1, bindings, calls + 1)?;
                        let Type::Closure(result) =
                            self.closure_signatures.borrow().result(callee)?
                        else {
                            return None;
                        };
                        if arity.is_some_and(|expected| expected != result) {
                            return None;
                        }
                        arity = Some(result);
                        continue;
                    }
                    let factory = self.closure_target(&call.callee, depth + 1)?;
                    let arrow = self.arrow(factory.arrow);
                    let body = arrow.get_expression();
                    // Preserve the old known-arrow factory recognition, including
                    // its source-depth behavior and specialization path.
                    if factory.factory
                        && let Some(body) = body
                        && let Some(returned) = self.closure_target(body, 0)
                    {
                        self.arrow_signature_at(self.arrow(returned.arrow), calls)?
                    } else {
                        if calls >= 8 || arrow.params.items.len() != call.arguments.len() {
                            return None;
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
                        for (parameter, argument) in arrow.params.items.iter().zip(&call.arguments)
                        {
                            let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern
                            else {
                                return None;
                            };
                            let argument = argument.as_expression()?;
                            parameters.insert(
                                identifier.symbol_id.get()?,
                                self.function_arity_in(argument, depth + 1, bindings, calls + 1),
                            );
                        }
                        if let Some(body) = body {
                            self.function_arity_in(body, 0, &parameters, calls + 1)?
                        } else {
                            self.block_function_arity_in(arrow, 0, &parameters, calls + 1)?
                        }
                    }
                }
                _ => {
                    if let Expression::Identifier(identifier) = node
                        && let Some(symbol) = identifier.reference_id.get().and_then(|reference| {
                            self.semantic.scoping().get_reference(reference).symbol_id()
                        })
                        && let Some(kind) = bindings.get(&symbol)
                    {
                        let next = (*kind)?;
                        if arity.is_some_and(|expected| expected != next) {
                            return None;
                        }
                        arity = Some(next);
                        continue;
                    }
                    if let Some((_, arity)) = self.runtime_closure_callee(node, depth) {
                        arity
                    } else {
                        self.arrow_signature_at(
                            self.arrow(self.closure_target(node, depth)?.arrow),
                            calls,
                        )?
                    }
                }
            };
            if arity.is_some_and(|expected| expected != next) {
                return None;
            }
            arity = Some(next);
        }
        arity
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
        let result = self.function_value_inner(expression, region, arity, depth);
        self.active_function_values -= 1;
        result
    }

    fn function_value_inner(
        &mut self,
        expression: &Expression<'_>,
        region: &mut Region,
        arity: usize,
        depth: usize,
    ) -> Result<ValueId, String> {
        let mut regions = Regions {
            root: region,
            children: Vec::new(),
        };
        let mut work = vec![Work::Eval(expression, depth)];
        let mut values = Vec::new();
        while let Some(task) = work.pop() {
            match task {
                Work::Eval(node, depth) => {
                    let context =
                        (self.inline_depth + self.statement_depth).max(self.function_result_depth);
                    let nesting = context + regions.children.len();
                    if nesting > 128 {
                        return Err(unsupported(
                            node,
                            "function-value expansion exceeds 128 aggregate nested regions",
                        ));
                    }
                    if depth > 128 {
                        return Err(unsupported(
                            node,
                            "expression nesting exceeds this experiment's limit",
                        ));
                    }
                    if let Some(inner) = erased_inner(node) {
                        work.push(Work::Eval(inner, depth + 1));
                        continue;
                    }
                    match node {
                        Expression::ParenthesizedExpression(wrapper) => {
                            work.push(Work::Eval(&wrapper.expression, depth + 1))
                        }
                        Expression::ConditionalExpression(conditional) => {
                            let outer = self.function_result_depth;
                            self.function_result_depth = nesting;
                            let condition =
                                self.predicate(&conditional.test, regions.current(), depth + 1);
                            self.function_result_depth = outer;
                            let condition = condition?;
                            work.push(Work::Consequent {
                                node,
                                condition,
                                alternate: &conditional.alternate,
                                depth: depth + 1,
                            });
                            regions.children.push(self.region(&conditional.consequent));
                            work.push(Work::Eval(&conditional.consequent, depth + 1));
                        }
                        Expression::SequenceExpression(sequence) => {
                            let (last, preceding) = sequence
                                .expressions
                                .split_last()
                                .ok_or("empty function sequence")?;
                            for item in preceding {
                                let outer = self.function_result_depth;
                                self.function_result_depth = nesting;
                                let discarded = self.discarded(item, regions.current(), depth + 1);
                                self.function_result_depth = outer;
                                discarded?;
                            }
                            work.push(Work::Eval(last, depth + 1));
                        }
                        Expression::AssignmentExpression(assignment) => {
                            let (binding, actual) = self.function_assignment_target(assignment, depth)
                                .ok_or_else(|| unsupported(node, "function assignment requires an initialized function binding"))?;
                            if !binding.mutable {
                                return Err(unsupported(
                                    node,
                                    "assignment to a const binding is not admitted",
                                ));
                            }
                            if assignment.operator != AssignmentOperator::Assign || actual != arity
                            {
                                return Err(unsupported(
                                    node,
                                    "function assignment requires plain = with unchanged arity",
                                ));
                            }
                            work.push(Work::Store {
                                node,
                                slot: binding.slot,
                            });
                            work.push(Work::Eval(&assignment.right, depth + 1));
                        }
                        Expression::CallExpression(call) => {
                            let outer = self.function_result_depth;
                            self.function_result_depth = nesting;
                            let result = (|| {
                                if self.closure_target(&call.callee, depth + 1).is_none() {
                                    self.runtime_function_result(
                                        call,
                                        regions.current(),
                                        arity,
                                        depth,
                                    )
                                } else if let Some(closure) =
                                    self.factory_result(node, regions.current(), depth)?
                                {
                                    self.materialize_closure(
                                        closure,
                                        regions.current(),
                                        node,
                                        arity,
                                    )
                                } else {
                                    self.local_function_result(
                                        call,
                                        regions.current(),
                                        arity,
                                        depth,
                                    )
                                }
                            })();
                            self.function_result_depth = outer;
                            values.push(result?);
                        }
                        _ => {
                            if let Some((slot, actual)) = self.runtime_closure_callee(node, depth) {
                                if actual != arity {
                                    return Err(unsupported(node, "function value arity mismatch"));
                                }
                                let result = self.value(Type::Closure(arity));
                                Self::push(regions.current(), node, Op::Load { result, slot });
                                values.push(result);
                            } else {
                                let closure = match self.closure_target(node, depth) {
                                    Some(closure) => closure,
                                    None => self
                                        .factory_result(node, regions.current(), depth)?
                                        .ok_or_else(|| {
                                            unsupported(
                                                node,
                                                "expected a known function constructor",
                                            )
                                        })?,
                                };
                                values.push(self.materialize_closure(
                                    closure,
                                    regions.current(),
                                    node,
                                    arity,
                                )?);
                            }
                        }
                    }
                }
                Work::Consequent {
                    node,
                    condition,
                    alternate,
                    depth,
                } => {
                    let consequent = regions.finish(values.pop().expect("function consequent"));
                    work.push(Work::Alternate {
                        node,
                        condition,
                        consequent,
                    });
                    regions.children.push(self.region(alternate));
                    work.push(Work::Eval(alternate, depth));
                }
                Work::Alternate {
                    node,
                    condition,
                    consequent,
                } => {
                    let alternate = regions.finish(values.pop().expect("function alternate"));
                    let result = self.value(Type::Closure(arity));
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
                    values.push(result);
                }
                Work::Store { node, slot } => {
                    // Store the RHS snapshot, then retain it as the assignment's
                    // result; a subsequent write cannot change an existing alias.
                    Self::push(
                        regions.current(),
                        node,
                        Op::Store {
                            slot,
                            value: *values.last().expect("function assignment RHS"),
                        },
                    );
                }
            }
        }
        assert!(regions.children.is_empty());
        assert_eq!(values.len(), 1);
        Ok(values.pop().unwrap())
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
                .function_arity(&call.callee, depth + 1)
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
                || call.type_arguments.is_some()
                || call.arguments.len() != parameter_count
            {
                return Err(unsupported(
                    call,
                    "runtime factory requires a plain call with exact arguments",
                ));
            }
            if result_type != Type::Closure(expected) {
                return Err(unsupported(
                    call,
                    "runtime factory result signature mismatch",
                ));
            }
            let callee = self.function_value(&call.callee, region, signature, depth + 1)?;
            let mut arguments = Vec::new();
            for (index, argument) in call.arguments.iter().enumerate() {
                let expression = argument
                    .as_expression()
                    .ok_or_else(|| unsupported(call, "spread call arguments are not admitted"))?;
                let parameter = self
                    .closure_signatures
                    .borrow()
                    .parameter(signature, index)
                    .expect("checked signature");
                let value = if let Type::Closure(callback) = parameter {
                    self.callback_argument(expression, region, callback, depth + 1)?
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
                .closure_target(&call.callee, depth + 1)
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
            for argument in &call.arguments {
                let expression = argument
                    .as_expression()
                    .ok_or_else(|| unsupported(call, "spread call arguments are not admitted"))?;
                let value = if let Some(kind) = self.function_arity(expression, depth + 1) {
                    self.callback_argument(expression, region, kind, depth + 1)?
                } else {
                    self.expression(expression, region, depth + 1)?
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
        if !matches!(return_type, Type::Number | Type::Closure(0..=1000)) {
            return Err(unsupported(
                node,
                "runtime factories return only numeric-leaf functions",
            ));
        }
        if matches!(return_type, Type::Closure(_)) && arrow.get_expression().is_none() {
            return Err(unsupported(
                node,
                "runtime factories currently require concise bodies",
            ));
        }
        self.charge_closure_expansion(node)?;
        let mut child = self.clone();
        child.bindings.clear();
        child.slots.clear();
        child.values.clear();
        child.next_region = 0;
        child.return_type = return_type;
        child.allow_await = false;
        child.source_charging = false;
        child.active_local_calls = 0;
        child.active_block_calls = 0;
        child.closure_expansions = 0;
        child.inline_depth = 0;
        child.statement_depth = 0;
        child.function_result_depth = 0;
        child.in_loop_body = false;
        child.lifting_depth += 1;
        for parameter in &arrow.params.items {
            let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern else {
                unreachable!("checked closure parameter")
            };
            let slot = SlotId(child.slots.len());
            child.slots.push(Slot {
                ty: self
                    .callback_parameters
                    .get(&arrow.node_id.get())
                    .and_then(|parameters| {
                        parameters.get(&identifier.symbol_id.get().expect("parameter symbol"))
                    })
                    .map_or(Type::Number, |arity| Type::Closure(*arity)),
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
        Self::push(&mut body, arrow, Op::ClosureEntry { captures: imported });
        child.initialize_general_parameters(&mut body);
        child.bindings.extend(capture_bindings);
        let returned = if let Some(expression) = arrow.get_expression() {
            if let Type::Closure(signature) = return_type {
                child.function_value(expression, &mut body, signature, 0)?
            } else {
                child.expression(expression, &mut body, 0)?
            }
        } else {
            let region = child.arrow_block(arrow, 0)?;
            let result = child.value(Type::Number);
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
