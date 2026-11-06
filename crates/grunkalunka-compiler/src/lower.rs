//! Source admission and AST-to-region lowering. Source limits count source nodes,
//! never the generated regions, values or Wasm temporary locals.
use oxc_ast::ast::{
    BindingPattern, CallExpression, Expression, ForStatementInit, IdentifierReference,
    SimpleAssignmentTarget, Statement, VariableDeclaration, VariableDeclarationKind,
    VariableDeclarator,
};
use oxc_semantic::{Semantic, SymbolId};
use oxc_span::GetSpan;
use oxc_syntax::operator::{
    AssignmentOperator, BinaryOperator, LogicalOperator, UnaryOperator, UpdateOperator,
};

use crate::ir::{
    self, Binary, Compare, Completion, Exit, Op, Operation, Region, RegionId, Relational, Slot,
    SlotId, Type, Unary, ValueId,
};
use crate::{is_type_declaration, unsupported};

mod array_at;
mod array_mutations;
mod assignment_patterns;
mod async_values;
mod awaited;
mod binding_patterns;
mod block_results;
mod callbacks;
mod closures;
mod control_cursor;
mod declarations;
mod evaluate;
#[cfg(test)]
mod finalizers;
#[cfg(test)]
mod for_statements;
mod function_values;
#[cfg(test)]
mod if_statements;
mod invocation;
mod iteration;
#[cfg(test)]
mod loop_statements;
mod managed_calls;
mod string_methods;
pub(super) use managed_calls::infer as infer_managed_calls;
mod module_functions;
mod object_create;
mod object_intrinsics;
pub(super) use module_functions::plan as plan_module_functions;
mod pending;
mod promises;
#[cfg(test)]
mod source_block_tests;
mod source_control;
mod source_errors;
mod static_keys;
mod storage;
pub(super) mod stored_functions;
pub(super) use callbacks::infer_callback_parameters;
pub(super) use promises::parameters as infer_promise_parameters;
pub(super) use storage::general_mutable_bindings;

#[derive(Clone)]
pub(super) struct NumericBinding {
    pub symbol: SymbolId,
    pub slot: SlotId,
    pub mutable: bool,
}

pub(super) struct NumericFunction {
    pub symbol: SymbolId,
    pub id: ir::FunctionId,
    pub parameter_count: usize,
    // Empty retains the ordinary all-Number signature without a parallel vector.
    pub parameter_types: Vec<Type>,
    pub return_type: Type,
    pub async_signature: Option<usize>,
    pub module_value: Option<(ir::FunctionId, usize)>,
}

impl NumericFunction {
    pub(super) fn closure_signature(
        &self,
        signatures: &std::cell::RefCell<ir::ClosureSignatures>,
    ) -> usize {
        self.async_signature.unwrap_or_else(|| {
            signatures.borrow_mut().intern(
                (0..self.parameter_count)
                    .map(|i| self.parameter_type(i))
                    .collect(),
                self.return_type,
            )
        })
    }

    pub(super) fn parameter_type(&self, index: usize) -> Type {
        self.parameter_types
            .get(index)
            .copied()
            .unwrap_or(Type::Number)
    }
}

fn numeric_binding<'b>(
    identifier: &IdentifierReference<'_>,
    semantic: &Semantic<'_>,
    bindings: &'b [NumericBinding],
) -> Result<&'b NumericBinding, String> {
    let reference = identifier
        .reference_id
        .get()
        .ok_or("missing semantic reference")?;
    let symbol = semantic.scoping().get_reference(reference).symbol_id();
    bindings
        .iter()
        .find(|binding| Some(binding.symbol) == symbol)
        .ok_or_else(|| {
            unsupported(
                identifier,
                "reference is not an initialized numeric parameter or local",
            )
        })
}

// The numeric module supplies a pristine intrinsic, never an external realm object.
fn math_unary(call: &CallExpression<'_>, semantic: &Semantic<'_>) -> Option<(Unary, &'static str)> {
    let Expression::StaticMemberExpression(member) = &call.callee else {
        return None;
    };
    let Expression::Identifier(object) = &member.object else {
        return None;
    };
    let reference = object.reference_id.get()?;
    if call.optional
        || member.optional
        || object.name != "Math"
        || semantic
            .scoping()
            .get_reference(reference)
            .symbol_id()
            .is_some()
    {
        return None;
    }
    match member.property.name.as_str() {
        "abs" => Some((Unary::Abs, "abs")),
        "sqrt" => Some((Unary::Sqrt, "sqrt")),
        "floor" => Some((Unary::Floor, "floor")),
        "ceil" => Some((Unary::Ceil, "ceil")),
        "trunc" => Some((Unary::Trunc, "trunc")),
        _ => None,
    }
}

// One wrapper at a time keeps each erased source node in the nesting budget.
// These type-only forms do not convert or validate their runtime operand.
pub(super) fn erased_inner<'e, 'a>(expression: &'e Expression<'a>) -> Option<&'e Expression<'a>> {
    match expression {
        Expression::TSAsExpression(wrapper) => Some(&wrapper.expression),
        Expression::TSTypeAssertion(wrapper) => Some(&wrapper.expression),
        Expression::TSNonNullExpression(wrapper) => Some(&wrapper.expression),
        Expression::TSSatisfiesExpression(wrapper) => Some(&wrapper.expression),
        _ => None,
    }
}

// Only known Boolean-producing shapes select the bridge. This scan emits nothing
// and is bounded by the source nesting limit. Type assertions never prove a type.
fn boolean_shape(mut expression: &Expression<'_>, mut depth: usize) -> bool {
    while depth <= 128 {
        if let Some(inner) = erased_inner(expression) {
            expression = inner;
        } else if let Expression::ParenthesizedExpression(wrapper) = expression {
            expression = &wrapper.expression;
        } else {
            return match expression {
                Expression::BooleanLiteral(_) => true,
                Expression::UnaryExpression(unary) => unary.operator == UnaryOperator::LogicalNot,
                Expression::BinaryExpression(binary) => matches!(
                    binary.operator,
                    BinaryOperator::LessThan
                        | BinaryOperator::LessEqualThan
                        | BinaryOperator::GreaterThan
                        | BinaryOperator::GreaterEqualThan
                        | BinaryOperator::Equality
                        | BinaryOperator::StrictEquality
                        | BinaryOperator::Inequality
                        | BinaryOperator::StrictInequality
                        | BinaryOperator::In
                ),
                _ => false,
            };
        }
        depth += 1;
    }
    // Original lowering still diagnoses over-depth input; no flattened retry.
    false
}

// Inspect only result-producing syntax, before emitting anything. Types asserted by
// TypeScript are not runtime kinds. Ordinary lowering owns the depth diagnostic.
fn discarded_boolean_shape(expression: &Expression<'_>, depth: usize) -> bool {
    // An existential read-only shape query. Preserve the original left-first
    // short circuit and charge depth on every wrapper/branch.
    let mut pending = vec![(expression, depth)];
    while let Some((expression, depth)) = pending.pop() {
        if depth > 128 {
            continue;
        }
        if let Some(inner) = erased_inner(expression) {
            pending.push((inner, depth + 1));
            continue;
        }
        match expression {
            Expression::BooleanLiteral(_) => return true,
            Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
                return true;
            }
            Expression::BinaryExpression(binary)
                if matches!(
                    binary.operator,
                    BinaryOperator::LessThan
                        | BinaryOperator::LessEqualThan
                        | BinaryOperator::GreaterThan
                        | BinaryOperator::GreaterEqualThan
                        | BinaryOperator::Equality
                        | BinaryOperator::StrictEquality
                        | BinaryOperator::Inequality
                        | BinaryOperator::StrictInequality
                        | BinaryOperator::In
                ) =>
            {
                return true;
            }
            Expression::ParenthesizedExpression(wrapper) => {
                pending.push((&wrapper.expression, depth + 1));
            }
            Expression::SequenceExpression(sequence) => {
                if let Some(last) = sequence.expressions.last() {
                    pending.push((last, depth + 1));
                }
            }
            Expression::ConditionalExpression(conditional) => {
                pending.push((&conditional.alternate, depth + 1));
                pending.push((&conditional.consequent, depth + 1));
            }
            Expression::LogicalExpression(logical)
                if logical.operator != LogicalOperator::Coalesce =>
            {
                pending.push((&logical.right, depth + 1));
                pending.push((&logical.left, depth + 1));
            }
            _ => {}
        }
    }
    false
}

// Resolve only the original reference; never evaluate a write target as a value.
// Oxc discards outer target parentheses, but retained inner nodes each cost depth.
fn target_identifier<'e, 'a>(
    target: &'e SimpleAssignmentTarget<'a>,
    depth: usize,
) -> Result<Option<&'e IdentifierReference<'a>>, String> {
    if let SimpleAssignmentTarget::AssignmentTargetIdentifier(identifier) = target {
        return Ok(Some(identifier));
    }
    let Some(mut expression) = target.get_expression() else {
        return Ok(None);
    };
    let mut depth = depth + 1;
    loop {
        if depth > 128 {
            return Err(unsupported(
                expression,
                "expression nesting exceeds this experiment's limit",
            ));
        }
        match expression {
            Expression::Identifier(identifier) => return Ok(Some(identifier)),
            Expression::ParenthesizedExpression(wrapper) => expression = &wrapper.expression,
            _ => match erased_inner(expression) {
                Some(inner) => expression = inner,
                None => return Ok(None),
            },
        }
        depth += 1;
    }
}

#[derive(Clone)]
pub(super) struct Lowering<'s, 'a> {
    pub return_type: Type,
    pub semantic: &'s Semantic<'a>,
    pub stored: &'s stored_functions::Fields,
    pub functions: &'s [NumericFunction],
    /// Only present in the explicitly selected A1 scalar-import compiler mode.
    /// Resolved by Oxc semantic symbol identity, never identifier spelling.
    pub ui_import: Option<SymbolId>,
    pub general_bindings: &'s std::collections::BTreeSet<SymbolId>,
    pub general_parameters: &'s std::collections::BTreeSet<SymbolId>,
    pub managed_kinds: &'s managed_calls::ClosureKinds,
    pub number_calls: &'s std::collections::BTreeSet<oxc_syntax::node::NodeId>,
    pub general_results: &'s std::collections::BTreeSet<oxc_syntax::node::NodeId>,
    pub settled_results: &'s std::collections::BTreeSet<oxc_syntax::node::NodeId>,
    pub async_results: &'s std::collections::BTreeSet<oxc_syntax::node::NodeId>,
    pub callback_parameters: &'s callbacks::Parameters,
    pub promise_parameters: &'s promises::Inputs,
    pub bindings: Vec<NumericBinding>,
    pub slots: Vec<Slot>,
    pub values: Vec<Type>,
    pub next_region: usize,
    pub local_count: usize,
    pub statement_count: usize,
    pub in_loop_body: bool,
    pub closures: Vec<closures::LocalClosure>,
    pub closure_expansions: usize,
    pub active_local_calls: usize,
    pub active_block_calls: usize,
    pub source_charging: bool,
    pub inline_depth: usize,
    pub statement_depth: usize,
    pub allow_await: bool,
    pub allow_async_closures: bool,
    pub lifted: Vec<std::rc::Rc<ir::Function>>,
    pub lifted_base: usize,
    pub lifting_depth: usize,
    pub active_function_values: usize,
    pub active_function_results: usize,
    // Generated parent regions retained across scalar/function cursor crossings.
    // AST wrapper depth has its own budget and must not be charged here again.
    pub function_result_depth: usize,
    pub closure_signatures: &'s std::cell::RefCell<ir::ClosureSignatures>,
}

impl<'s, 'ast> Lowering<'s, 'ast> {
    fn lifted_limit(&self) -> usize {
        self.functions.len()
            + self
                .functions
                .iter()
                .filter(|f| f.module_value.is_some())
                .count()
            + 256
    }

    pub fn region(&mut self, node: &impl GetSpan) -> Region {
        let id = RegionId(self.next_region);
        self.next_region += 1;
        Region {
            id,
            span: node.span(),
            operations: Vec::new(),
            exit: Exit::Fallthrough,
        }
    }

    // Numeric physical parameters retain Number slots; callback parameters keep pairs.
    // Only source bindings whose
    // later writes require General storage receive a separately initialized cell.
    pub(super) fn initialize_general_parameters(&mut self, region: &mut Region) {
        for index in 0..self.bindings.len() {
            if self.slots[self.bindings[index].slot.0].ty != Type::Number
                || !self.general_bindings.contains(&self.bindings[index].symbol)
            {
                continue;
            }
            let input_slot = self.bindings[index].slot;
            let span = self.slots[input_slot.0].span;
            let number = self.value(Type::Number);
            Self::push(
                region,
                &span,
                Op::Load {
                    result: number,
                    slot: input_slot,
                },
            );
            let value = self.value(Type::General);
            Self::push(
                region,
                &span,
                Op::Pack {
                    result: value,
                    input: number,
                },
            );
            let slot = SlotId(self.slots.len());
            self.slots.push(Slot {
                ty: Type::General,
                owner: region.id,
                mutable: true,
                parameter: false,
                span,
            });
            Self::push(region, &span, Op::Initialize { slot, value });
            self.bindings[index].slot = slot;
        }
    }

    pub fn return_undefined(&mut self, node: &impl GetSpan, region: &mut Region) {
        let value = self.value(Type::General);
        Self::push(
            region,
            node,
            Op::Nullish {
                result: value,
                value: ir::Nullish::Undefined,
            },
        );
        let value = if self.return_type == Type::ManagedValue {
            let result = self.value(Type::ManagedValue);
            Self::push(
                region,
                node,
                Op::ManagedPrimitive {
                    result,
                    input: value,
                },
            );
            result
        } else if self.return_type == Type::AsyncValue {
            let result = self.value(Type::AsyncValue);
            Self::push(
                region,
                node,
                Op::ToAsyncValue {
                    result,
                    input: value,
                },
            );
            result
        } else {
            value
        };
        region.exit = Exit::Return(value);
    }

    fn value(&mut self, ty: Type) -> ValueId {
        let id = ValueId(self.values.len());
        self.values.push(ty);
        id
    }

    fn push(region: &mut Region, node: &impl GetSpan, kind: Op) {
        region.operations.push(Operation {
            span: node.span(),
            kind,
        });
    }

    // Selected numeric bridges may read only published immutable Predicate bindings.
    // Keep original wrapper depth and never infer a runtime kind from type syntax.
    fn operand_boolean_shape(&self, mut expression: &Expression<'_>, mut depth: usize) -> bool {
        while depth <= 128 {
            if let Some(inner) = erased_inner(expression) {
                expression = inner;
            } else if let Expression::ParenthesizedExpression(wrapper) = expression {
                expression = &wrapper.expression;
            } else if let Expression::Identifier(identifier) = expression {
                return numeric_binding(identifier, self.semantic, &self.bindings).is_ok_and(
                    |binding| !binding.mutable && self.slots[binding.slot.0].ty == Type::Predicate,
                );
            } else if let Expression::CallExpression(call) = expression {
                return object_intrinsics::is_array_test(call, self.semantic)
                    || string_methods::call(call).is_some_and(|(receiver, method)| {
                        matches!(method, string_methods::Method::Includes)
                            && self.stored.container(receiver.node_id()) == Some(Type::ManagedArray)
                    });
            } else {
                return boolean_shape(expression, depth);
            }
            depth += 1;
        }
        false
    }

    // A positive classification keeps existing scalar comparison paths unchanged.
    // This scan emits nothing; the evaluator still validates every source operand.
    fn numeric_result_shape(&self, expression: &Expression<'_>, depth: usize) -> bool {
        if depth <= 128 && self.stored.number(expression.node_id()) {
            return true;
        }
        if self.managed_length_receiver(expression, depth).is_some() {
            return true;
        }
        if let Expression::CallExpression(call) = expression
            && self
                .closure_target(invocation::callee(call), invocation::depth(call, depth + 1))
                .is_some_and(|closure| {
                    if closure.checked {
                        !closure.factory && closure.result == Type::Number
                    } else {
                        self.arrow_signature(self.arrow(closure.arrow))
                            .is_some_and(|id| {
                                self.closure_signatures.borrow().result(id) == Some(Type::Number)
                            })
                    }
                })
        {
            return true;
        }
        let closures: std::collections::BTreeSet<_> = self
            .closures
            .iter()
            .filter(|closure| !closure.factory && closure.result == Type::Number)
            .filter_map(|closure| closure.symbol)
            .chain(
                self.bindings
                    .iter()
                    .filter(|binding| match self.slots[binding.slot.0].ty {
                        Type::Closure(id) => {
                            self.closure_signatures.borrow().result(id) == Some(Type::Number)
                        }
                        _ => false,
                    })
                    .map(|binding| binding.symbol),
            )
            .collect();
        storage::number_result_shape(
            expression,
            depth,
            self.semantic,
            self.functions,
            storage::NumberCalls::Proven(self.number_calls),
            |symbol| closures.contains(&symbol),
            |symbol| {
                self.bindings
                    .iter()
                    .find(|binding| binding.symbol == symbol)
                    .is_some_and(|binding| self.slots[binding.slot.0].ty == Type::Number)
            },
        )
    }

    // Source control and lexical sequencing share one explicit heap cursor.
    // Statement families no longer reenter an independent sequence dispatcher.
    pub fn sequence<'n>(
        &mut self,
        statements: &'n [Statement<'ast>],
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String>
    where
        'ast: 'n,
    {
        self.control_statement_iterative(
            control_cursor::ControlNode::Sequence(statements),
            region,
            depth,
        )
    }

    pub(super) fn charge_statement(&mut self, statement: &impl GetSpan) -> Result<(), String> {
        if !self.source_charging {
            return Ok(());
        }
        if self.statement_count >= 256 {
            return Err(unsupported(
                statement,
                "at most 256 statements including blocks and returns are admitted",
            ));
        }
        self.statement_count += 1;
        Ok(())
    }

    // Inspect retained nodes without emitting or trusting type annotations. Only
    // already-published immutable Predicate bindings can supply an alias.
    fn const_boolean_initializer(&self, mut expression: &Expression<'_>, mut depth: usize) -> bool {
        while depth <= 128 {
            if let Some(inner) = erased_inner(expression) {
                expression = inner;
            } else if let Expression::ParenthesizedExpression(wrapper) = expression {
                expression = &wrapper.expression;
            } else if let Expression::Identifier(identifier) = expression {
                return numeric_binding(identifier, self.semantic, &self.bindings).is_ok_and(
                    |binding| !binding.mutable && self.slots[binding.slot.0].ty == Type::Predicate,
                );
            } else {
                return boolean_shape(expression, depth);
            }
            depth += 1;
        }
        false
    }

    // Only already-published General cells qualify; wrappers retain depth charges.
    fn general_binding_shape(&self, mut expression: &Expression<'_>, mut depth: usize) -> bool {
        while depth <= 128 {
            if let Some(inner) = erased_inner(expression) {
                expression = inner;
            } else if let Expression::ParenthesizedExpression(wrapper) = expression {
                expression = &wrapper.expression;
            } else if let Expression::Identifier(identifier) = expression {
                return numeric_binding(identifier, self.semantic, &self.bindings)
                    .is_ok_and(|binding| self.slots[binding.slot.0].ty == Type::General);
            } else {
                return false;
            }
            depth += 1;
        }
        false
    }

    // Wrappers are already stripped by the evaluator with their original depth charges.
    // This positive classification emits nothing and does not replace source validation.
    fn general_predicate_leaf(&self, expression: &Expression<'_>, depth: usize) -> bool {
        match expression {
            Expression::NullLiteral(_) => true,
            Expression::Identifier(identifier) => {
                if let Ok(binding) = numeric_binding(identifier, self.semantic, &self.bindings) {
                    self.slots[binding.slot.0].ty == Type::General
                } else {
                    identifier.name == "undefined"
                        && identifier.reference_id.get().is_some_and(|reference| {
                            self.semantic
                                .scoping()
                                .get_reference(reference)
                                .symbol_id()
                                .is_none()
                        })
                }
            }
            Expression::CallExpression(call) => {
                let Expression::Identifier(identifier) = invocation::callee(call) else {
                    return false;
                };
                let symbol = identifier.reference_id.get().and_then(|reference| {
                    self.semantic.scoping().get_reference(reference).symbol_id()
                });
                self.functions.iter().any(|function| {
                    Some(function.symbol) == symbol && function.return_type == Type::General
                })
            }
            Expression::AssignmentExpression(_) => {
                self.general_assignment_target(expression, depth).is_some()
            }
            _ => false,
        }
    }

    // Keep existing scalar Number logical results specialized; wrappers still charge depth.
    fn general_logical_shape(&self, mut expression: &Expression<'_>, mut depth: usize) -> bool {
        while depth <= 128 {
            if let Some(inner) = erased_inner(expression) {
                expression = inner;
            } else if let Expression::ParenthesizedExpression(wrapper) = expression {
                expression = &wrapper.expression;
            } else {
                return matches!(expression, Expression::LogicalExpression(_))
                    && !self.numeric_result_shape(expression, depth);
            }
            depth += 1;
        }
        false
    }

    fn general_assignment_target(
        &self,
        mut expression: &Expression<'_>,
        mut depth: usize,
    ) -> Option<SlotId> {
        while depth <= 128 {
            if let Some(inner) = erased_inner(expression) {
                expression = inner;
            } else if let Expression::ParenthesizedExpression(wrapper) = expression {
                expression = &wrapper.expression;
            } else if let Expression::AssignmentExpression(assignment) = expression {
                if !matches!(
                    assignment.operator,
                    AssignmentOperator::Assign
                        | AssignmentOperator::LogicalAnd
                        | AssignmentOperator::LogicalOr
                        | AssignmentOperator::LogicalNullish
                ) {
                    return None;
                }
                let identifier = assignment
                    .left
                    .as_simple_assignment_target()
                    .and_then(|target| target_identifier(target, depth).ok().flatten())?;
                let slot = numeric_binding(identifier, self.semantic, &self.bindings)
                    .ok()?
                    .slot;
                return (self.slots[slot.0].ty == Type::General).then_some(slot);
            } else {
                return None;
            }
            depth += 1;
        }
        None
    }

    /// Publish a completed scalar/managed/Promise initializer into one local
    /// slot. Scalar and managed source continuations share the original
    /// allocation, IR and binding order.
    pub(super) fn commit_local_value(
        &mut self,
        local: &VariableDeclarator<'_>,
        region: &mut Region,
        ty: Type,
        value: ValueId,
        mutable: bool,
    ) -> Result<(), String> {
        let BindingPattern::BindingIdentifier(identifier) = &local.id else {
            unreachable!("declaration local was checked before evaluation");
        };
        self.commit_binding_value(
            local,
            identifier.symbol_id.get().ok_or("missing local binding")?,
            region,
            ty,
            value,
            mutable,
        )
    }

    /// Publish one initialized lexical binding. Pattern leaves use the same
    /// slots and capture cells as ordinary identifier declarations.
    pub(super) fn commit_binding_value(
        &mut self,
        node: &impl GetSpan,
        symbol: SymbolId,
        region: &mut Region,
        ty: Type,
        value: ValueId,
        mutable: bool,
    ) -> Result<(), String> {
        let slot = SlotId(self.slots.len());
        self.slots.push(Slot {
            ty,
            owner: region.id,
            mutable,
            parameter: false,
            span: node.span(),
        });
        Self::push(region, node, Op::Initialize { slot, value });
        self.bindings.push(NumericBinding {
            symbol,
            slot,
            mutable,
        });
        Ok(())
    }

    /// The scheduler owns every expression and source-control transition.
    /// This boundary admits only intrinsic, erased, or rejected statements.
    fn statement_leaf<'n>(
        &mut self,
        statement: &'n Statement<'ast>,
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String> {
        let outer = std::mem::replace(&mut self.statement_depth, depth);
        let result = self.statement_leaf_inner(statement, region, depth);
        self.statement_depth = outer;
        result
    }

    fn statement_leaf_inner(
        &mut self,
        statement: &Statement<'ast>,
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String> {
        self.admit_source_statement(statement, depth)?;
        if is_type_declaration(statement) {
            return Ok(Completion::NORMAL);
        }
        match statement {
            Statement::ContinueStatement(statement) => {
                if statement.label.is_some() || !self.in_loop_body {
                    return Err(unsupported(
                        statement.as_ref(),
                        "only unlabelled continue inside a loop is admitted",
                    ));
                }
                region.exit = Exit::Continue;
                Ok(Completion::CONTINUE)
            }
            Statement::BreakStatement(statement) => {
                if statement.label.is_some() || !self.in_loop_body {
                    return Err(unsupported(
                        statement.as_ref(),
                        "only unlabelled break inside a loop is admitted",
                    ));
                }
                region.exit = Exit::Break;
                Ok(Completion::BREAK)
            }
            Statement::EmptyStatement(_) => Ok(Completion::NORMAL),
            Statement::ReturnStatement(statement) if statement.argument.is_none() => {
                if matches!(
                    self.return_type,
                    Type::General | Type::AsyncValue | Type::ManagedValue
                ) && (self.active_block_calls != 0 || self.allow_await)
                {
                    self.return_undefined(statement.as_ref(), region);
                    Ok(Completion::RETURN)
                } else {
                    Err(if self.return_type == Type::General {
                        "return must have a primitive expression"
                    } else {
                        "return must have a numeric expression"
                    }
                    .into())
                }
            }
            Statement::ThrowStatement(statement) if !self.allow_await => Err(unsupported(
                statement.as_ref(),
                "statement is outside the numeric experiment",
            )),
            Statement::IfStatement(_)
            | Statement::WhileStatement(_)
            | Statement::DoWhileStatement(_)
            | Statement::ForStatement(_)
            | Statement::BlockStatement(_)
            | Statement::TryStatement(_)
            | Statement::ReturnStatement(_)
            | Statement::ThrowStatement(_)
            | Statement::ExpressionStatement(_)
            | Statement::VariableDeclaration(_) => {
                unreachable!("source continuations own control and expression statements")
            }
            _ => Err(unsupported(
                statement,
                "statement is outside the numeric experiment",
            )),
        }
    }

    pub fn finish(self, body: Region, parameter_count: usize) -> ir::Function {
        ir::Function {
            return_type: self.return_type,
            parameter_count,
            slots: self.slots,
            values: self.values,
            body,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn shift_unsigned_is_admitted() {
        crate::compile("export function run(a,b) { return -1 >>> 0; }", false).unwrap();
    }

    #[test]
    fn shift_signed_is_admitted() {
        crate::compile("export function run(a,b) { return -1 >> 0; }", false).unwrap();
    }

    #[test]
    fn shift_left_is_admitted() {
        crate::compile("export function run(a,b) { return 1 << 31; }", false).unwrap();
    }

    #[test]
    fn bitwise_binary_and_is_admitted() {
        crate::compile("export function run(a,b) { return 5 & 3; }", false).unwrap();
    }

    #[test]
    fn bitwise_binary_or_is_admitted() {
        crate::compile("export function run(a,b) { return 5 | 3; }", false).unwrap();
    }

    #[test]
    fn bitwise_binary_xor_is_admitted() {
        crate::compile("export function run(a,b) { return 5 ^ 3; }", false).unwrap();
    }

    #[test]
    fn bit_not_zero_is_admitted() {
        crate::compile("export function run(a,b) { return ~0; }", false).unwrap();
    }

    #[test]
    fn bit_not_negative_one_is_admitted() {
        crate::compile("export function run(a,b) { return ~(-1); }", false).unwrap();
    }

    #[test]
    fn continue_reaches_each_loops_next_iteration_machinery() {
        for body in [
            "let i=0; while(i++<3) { continue; } return i;",
            "let g=0,s=0; for(;g++<3;s++) { continue; } return 10*g+s;",
            "let i=0,t=0; do { i++; if(i<3) continue; } while((t++,0)); return i*10+t;",
        ] {
            crate::compile(&format!("export function run(a,b) {{ {body} }}"), false)
                .unwrap_or_else(|error| panic!("{body}: {error}"));
        }
    }

    #[test]
    fn continue_alternative_survives_a_return_suffix() {
        let error = crate::compile(
            "export function run(a,b) { do { if(a) continue; return b; } while(0); }",
            false,
        )
        .err()
        .expect("a continue path still needs a numeric return after the loop");
        assert!(
            error.contains("every structural path must return"),
            "{error}"
        );
    }

    #[test]
    fn break_exits_each_numeric_loop() {
        for body in [
            "while(a) { break; } return b;",
            "for(;;a=b) { break; } return a;",
            "do { break; } while(a=b); return a;",
        ] {
            crate::compile(&format!("export function run(a,b) {{ {body} }}"), false)
                .unwrap_or_else(|error| panic!("{body}: {error}"));
        }
    }

    #[test]
    fn break_alternatives_survive_a_return_suffix() {
        let error = crate::compile(
            "export function run(a,b) { do { if(a) break; return b; } while(0); }",
            false,
        )
        .err()
        .expect("break path still needs a numeric return");
        assert!(
            error.contains("every structural path must return"),
            "{error}"
        );
    }

    #[test]
    fn reads_unresolved_infinity() {
        let intrinsic =
            crate::compile("export function run(a,b) { return Infinity; }", false).unwrap();
        let literal = crate::compile("export function run(a,b) { return 1e999; }", false).unwrap();
        assert_eq!(intrinsic.wasm, literal.wasm);
    }

    #[test]
    fn reads_unresolved_nan() {
        crate::compile("export function run(a,b) { return NaN; }", false).unwrap();
    }

    #[test]
    fn intrinsic_names_remain_mutable_parameters() {
        for typescript in [false, true] {
            let shadowed = crate::compile(
                "export function run(Infinity,NaN) { Infinity+=NaN; NaN++; return Infinity+NaN; }",
                typescript,
            )
            .unwrap();
            let renamed = crate::compile(
                "export function run(a,b) { a+=b; b++; return a+b; }",
                typescript,
            )
            .unwrap();
            assert_eq!(shadowed.wasm, renamed.wasm);
        }
    }

    #[test]
    fn intrinsic_names_keep_self_and_forward_tdz() {
        for name in ["Infinity", "NaN"] {
            for body in [
                format!("let {name}={name}; return {name};"),
                format!("const x={name}; let {name}=a; return x;"),
                format!("const x=0?{name}:a; let {name}=b; return x;"),
                format!("let x={name},{name}=a; return x;"),
                format!("let x=0?{name}:a,{name}=b; return x;"),
                format!("for(let x={name},{name}=0;0;) {{}} return a;"),
            ] {
                let error =
                    crate::compile(&format!("export function run(a,b) {{ {body} }}"), false)
                        .err()
                        .expect("a resolved TDZ reference must not become a global constant");
                assert!(
                    error.contains("reference is not an initialized numeric parameter or local")
                );
            }
        }
    }

    #[test]
    fn empty_statement_falls_through() {
        let empty = crate::compile("export function run(a,b) { ; return a; }", false).unwrap();
        let plain = crate::compile("export function run(a,b) { return a; }", false).unwrap();
        assert_eq!(empty.wasm, plain.wasm);
    }
    #[test]
    fn empty_loop_body_is_admitted() {
        crate::compile(
            "export function run(a,b) { let i=0; while(i++<3); return a; }",
            false,
        )
        .unwrap();
    }

    #[test]
    fn discarded_numeric_statement() {
        crate::compile("export function run(a,b) { a+(a=b); return a; }", false).unwrap();
    }
    #[test]
    fn discarded_comma_statement() {
        crate::compile(
            "export function run(a,b) { (a=b,b=a+1); return a+b; }",
            false,
        )
        .unwrap();
    }
    #[test]
    fn discarded_wrapped_statement() {
        crate::compile(
            "export function run(a,b) { (a=b) as string; return a; }",
            true,
        )
        .unwrap();
    }

    fn same_numeric_equality(loose: &str, strict: &str) {
        let compile = |expression| {
            crate::compile(
                &format!("export function run(a,b) {{ return {expression}?-0:0; }}"),
                false,
            )
            .unwrap()
            .wasm
        };
        assert_eq!(compile(loose), compile(strict));
    }

    #[test]
    fn numeric_loose_equality_uses_number_equality() {
        same_numeric_equality("a==b", "a===b");
    }
    #[test]
    fn numeric_loose_inequality_uses_number_inequality() {
        same_numeric_equality("a!=b", "a!==b");
    }
    #[test]
    fn numeric_loose_equality_preserves_operand_snapshots() {
        same_numeric_equality("a==(a=b)", "a===(a=b)");
    }

    fn admits_do(body: &str) {
        crate::compile(&format!("export function run(a,b) {{ {body} }}"), false).unwrap();
    }

    #[test]
    fn do_runs_before_zero_test() {
        admits_do("do { a+=b; } while(0); return a;");
    }
    #[test]
    fn do_three_iterations() {
        admits_do("let i=0; do { a+=b; i++; } while(i<3); return a;");
    }
    #[test]
    fn do_return_skips_test_without_fallback() {
        admits_do("do { return a; } while(a=b);");
    }

    fn admits_for(body: &str) {
        crate::compile(&format!("export function run(a,b) {{ {body} }}"), false).unwrap();
    }

    #[test]
    fn for_zero_iterations() {
        admits_for("for (;0;) { a=b; } return a;");
    }
    #[test]
    fn for_three_iterations() {
        admits_for("let x=a; for(let i=0;i<3;i++) { x+=b; } return x;");
    }
    #[test]
    fn for_step_follows_body() {
        admits_for("let x=0; for(let i=0;i<3;i++) { x=x*10+i; } return x;");
    }
    #[test]
    fn for_return_skips_step() {
        admits_for("for(;;a=b) { return a; } return b;");
    }

    fn same_runtime_expression(expression: &str) {
        let js = crate::compile("export function run(a,b) { return a; }", false).unwrap();
        let ts = crate::compile(
            &format!("export function run(a:number,b:number):number {{ return {expression}; }}"),
            true,
        )
        .unwrap();
        assert_eq!(js.wasm, ts.wasm);
    }

    #[test]
    fn erases_as_expression() {
        same_runtime_expression("a as number");
    }
    #[test]
    fn erases_angle_assertion() {
        same_runtime_expression("<number>a");
    }
    #[test]
    fn erases_non_null_expression() {
        same_runtime_expression("a!");
    }
    #[test]
    fn erases_satisfies_expression() {
        same_runtime_expression("a satisfies number");
    }

    fn same_runtime_predicate(predicate: &str) {
        let js =
            crate::compile("export function run(a,b) { return a < b ? -0 : b; }", false).unwrap();
        let ts = crate::compile(
            &format!(
                "export function run(a:number,b:number):number {{ return {predicate} ? -0 : b; }}"
            ),
            true,
        )
        .unwrap();
        assert_eq!(js.wasm, ts.wasm);
    }

    #[test]
    fn erases_as_predicate() {
        same_runtime_predicate("(a < b) as boolean");
    }
    #[test]
    fn erases_angle_predicate() {
        same_runtime_predicate("<boolean>(a < b)");
    }
    #[test]
    fn erases_non_null_predicate() {
        same_runtime_predicate("(a < b)!");
    }
    #[test]
    fn erases_satisfies_predicate() {
        same_runtime_predicate("(a < b) satisfies boolean");
    }
}
