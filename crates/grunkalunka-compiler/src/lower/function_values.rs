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
                    let (_, target_arity) = self.function_assignment_target(assignment, depth)?;
                    if arity.is_some_and(|expected| expected != target_arity) {
                        return None;
                    }
                    arity = Some(target_arity);
                    pending.push((&assignment.right, depth + 1));
                    continue;
                }
                Expression::CallExpression(call) => {
                    let factory = self.closure_target(&call.callee, depth + 1)?;
                    if factory.checked && !factory.factory {
                        return None;
                    }
                    let returned =
                        self.closure_target(self.arrow(factory.arrow).get_expression()?, 0)?;
                    self.arrow(returned.arrow).params.items.len()
                }
                _ => {
                    if let Some((_, arity)) = self.runtime_closure_callee(node, depth) {
                        arity
                    } else {
                        self.arrow(self.closure_target(node, depth)?.arrow)
                            .params
                            .items
                            .len()
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
                            let condition =
                                self.predicate(&conditional.test, regions.current(), depth + 1)?;
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
                                self.discarded(item, regions.current(), depth + 1)?;
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
        if closure.factory {
            return Err(unsupported(
                node,
                "runtime function values currently require Number results",
            ));
        }
        let arrow = self.arrow(closure.arrow);
        if arrow.params.items.len() != arity {
            return Err(unsupported(node, "function value arity mismatch"));
        }
        if self.lifted_base + self.lifted.len() >= self.functions.len() + 256 {
            return Err(unsupported(
                node,
                "at most 256 lifted closure bodies are admitted",
            ));
        }
        self.charge_closure_expansion(node)?;
        let mut child = self.clone();
        child.bindings.clear();
        child.slots.clear();
        child.values.clear();
        child.next_region = 0;
        child.return_type = Type::Number;
        child.allow_await = false;
        child.source_charging = false;
        child.active_local_calls = 0;
        child.active_block_calls = 0;
        child.closure_expansions = 0;
        child.inline_depth = 0;
        child.statement_depth = 0;
        child.in_loop_body = false;
        child.lifting_depth += 1;
        for parameter in &arrow.params.items {
            let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern else {
                unreachable!("checked closure parameter")
            };
            let slot = SlotId(child.slots.len());
            child.slots.push(Slot {
                ty: Type::Number,
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
            child.expression(expression, &mut body, 0)?
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
        self.lifted
            .push(std::rc::Rc::new(child.finish(body, arity)));
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
