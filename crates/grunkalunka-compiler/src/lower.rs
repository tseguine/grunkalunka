//! Source admission and AST-to-region lowering. Source limits count source nodes,
//! never the generated regions, values or Wasm temporary locals.
use oxc_ast::ast::{
    BindingPattern, CallExpression, Expression, ForStatementInit, IdentifierReference,
    SimpleAssignmentTarget, Statement, VariableDeclaration, VariableDeclarationKind,
};
use oxc_semantic::{Semantic, SymbolId};
use oxc_span::GetSpan;
use oxc_syntax::operator::{
    AssignmentOperator, BinaryOperator, LogicalOperator, UnaryOperator, UpdateOperator,
};

use crate::ir::{
    self, Binary, Compare, Completion, Exit, Op, Operation, Region, RegionId, Slot, SlotId, Type,
    Unary, ValueId,
};
use crate::{is_type_declaration, unsupported};

mod evaluate;

pub(super) struct NumericBinding {
    pub symbol: SymbolId,
    pub slot: SlotId,
    pub mutable: bool,
}

pub(super) struct NumericFunction {
    pub symbol: SymbolId,
    pub id: ir::FunctionId,
    pub parameter_count: usize,
    pub return_type: Type,
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
        || call.type_arguments.is_some()
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
fn erased_inner<'e, 'a>(expression: &'e Expression<'a>) -> Option<&'e Expression<'a>> {
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
    if depth > 128 {
        return false;
    }
    if let Some(inner) = erased_inner(expression) {
        return discarded_boolean_shape(inner, depth + 1);
    }
    match expression {
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
        ),
        Expression::ParenthesizedExpression(wrapper) => {
            discarded_boolean_shape(&wrapper.expression, depth + 1)
        }
        Expression::SequenceExpression(sequence) => sequence
            .expressions
            .last()
            .is_some_and(|last| discarded_boolean_shape(last, depth + 1)),
        Expression::ConditionalExpression(conditional) => {
            discarded_boolean_shape(&conditional.consequent, depth + 1)
                || discarded_boolean_shape(&conditional.alternate, depth + 1)
        }
        Expression::LogicalExpression(logical) if logical.operator != LogicalOperator::Coalesce => {
            discarded_boolean_shape(&logical.left, depth + 1)
                || discarded_boolean_shape(&logical.right, depth + 1)
        }
        _ => false,
    }
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

pub(super) struct Lowering<'s, 'a> {
    pub return_type: Type,
    pub semantic: &'s Semantic<'a>,
    pub functions: &'s [NumericFunction],
    pub bindings: Vec<NumericBinding>,
    pub slots: Vec<Slot>,
    pub values: Vec<Type>,
    pub next_region: usize,
    pub local_count: usize,
    pub statement_count: usize,
    pub in_loop_body: bool,
}

impl Lowering<'_, '_> {
    // Number and Boolean results can be discarded without exposing a value kind.
    // Keep numeric lowering byte-identical; never retry a partially emitted tree.
    fn discarded(
        &mut self,
        expression: &Expression<'_>,
        region: &mut Region,
        depth: usize,
    ) -> Result<(), String> {
        self.evaluate(expression, region, depth, evaluate::Mode::Discard)?;
        Ok(())
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
            } else {
                return boolean_shape(expression, depth);
            }
            depth += 1;
        }
        false
    }

    fn expression(
        &mut self,
        expression: &Expression<'_>,
        region: &mut Region,
        depth: usize,
    ) -> Result<ValueId, String> {
        Ok(self
            .evaluate(expression, region, depth, evaluate::Mode::Number)?
            .expect("value mode"))
    }

    fn predicate(
        &mut self,
        expression: &Expression<'_>,
        region: &mut Region,
        depth: usize,
    ) -> Result<ValueId, String> {
        Ok(self
            .evaluate(expression, region, depth, evaluate::Mode::Predicate)?
            .expect("value mode"))
    }

    // Preserve every possible completion; only normal paths reach the suffix.
    pub fn sequence(
        &mut self,
        statements: &[Statement<'_>],
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String> {
        let mut completion = Completion::NORMAL;
        for statement in statements {
            if !completion.normal && !is_type_declaration(statement) {
                return Err(unsupported(
                    statement,
                    if completion == Completion::RETURN {
                        "statements after a guaranteed return are not admitted"
                    } else {
                        "statements after a guaranteed abrupt completion are not admitted"
                    },
                ));
            }
            completion = completion.then(self.statement(statement, region, depth)?);
        }
        Ok(completion)
    }

    pub(super) fn charge_statement(&mut self, statement: &impl GetSpan) -> Result<(), String> {
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

    fn declaration(
        &mut self,
        declaration: &VariableDeclaration<'_>,
        region: &mut Region,
    ) -> Result<(), String> {
        if !matches!(
            declaration.kind,
            VariableDeclarationKind::Const | VariableDeclarationKind::Let
        ) || declaration.declare
            || declaration.declarations.is_empty()
        {
            return Err(unsupported(
                declaration,
                "initialized const or let identifiers are required",
            ));
        }
        for local in &declaration.declarations {
            if self.local_count >= 128 {
                return Err(unsupported(local, "at most 128 source locals are admitted"));
            }
            if local.definite {
                return Err(unsupported(
                    local,
                    "definite-assignment assertions are not admitted",
                ));
            }
            let BindingPattern::BindingIdentifier(identifier) = &local.id else {
                return Err(unsupported(local, "local destructuring is not admitted"));
            };
            let initializer = local
                .init
                .as_ref()
                .ok_or("numeric local must have an initializer")?;
            let mutable = declaration.kind == VariableDeclarationKind::Let;
            let ty = if !mutable && self.const_boolean_initializer(initializer, 0) {
                Type::Predicate
            } else {
                Type::Number
            };
            let value = match ty {
                Type::General => unreachable!("scalar declaration typing"),
                Type::Predicate => self.predicate(initializer, region, 0)?,
                Type::Number => self.expression(initializer, region, 0)?,
            };
            let slot = SlotId(self.slots.len());
            self.slots.push(Slot {
                ty,
                owner: region.id,
                mutable,
                parameter: false,
                span: local.span(),
            });
            self.local_count += 1;
            Self::push(region, local, Op::Initialize { slot, value });
            self.bindings.push(NumericBinding {
                symbol: identifier.symbol_id.get().ok_or("missing local binding")?,
                slot,
                mutable,
            });
        }
        Ok(())
    }

    fn loop_body(
        &mut self,
        statement: &Statement<'_>,
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String> {
        let outer = self.in_loop_body;
        self.in_loop_body = true;
        let result = self.statement(statement, region, depth);
        self.in_loop_body = outer;
        result
    }

    fn statement(
        &mut self,
        statement: &Statement<'_>,
        region: &mut Region,
        depth: usize,
    ) -> Result<Completion, String> {
        if depth > 64 {
            return Err(unsupported(statement, "statement nesting exceeds 64"));
        }
        self.charge_statement(statement)?;
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
            Statement::DoWhileStatement(statement) => {
                let mut body = self.region(&statement.body);
                let visible = self.bindings.len();
                let returns = self.loop_body(&statement.body, &mut body, depth + 1)?;
                self.bindings.truncate(visible);
                // Admit the whole condition even when a body return skips it.
                let mut condition = self.region(&statement.test);
                let predicate = self.predicate(&statement.test, &mut condition, 0)?;
                condition.exit = Exit::Yield(predicate);
                Self::push(region, statement.as_ref(), Op::DoWhile { body, condition });
                // The body executes at least once; this loop consumes its loop control.
                Ok(returns.do_loop())
            }
            Statement::ForStatement(statement) => {
                // This lexical block and the loop's regions are generated, so
                // they consume neither source statements nor nesting depth.
                let mut header = self.region(statement.as_ref());
                let outer = self.bindings.len();
                if let Some(init) = &statement.init {
                    match init {
                        ForStatementInit::VariableDeclaration(declaration) => {
                            self.charge_statement(declaration.as_ref())?;
                            self.declaration(declaration, &mut header)?;
                        }
                        _ => {
                            self.discarded(init.to_expression(), &mut header, 0)?;
                        }
                    }
                }
                let header_bindings = self.bindings.len();
                let mut condition = self.region(statement.as_ref());
                let predicate = if let Some(test) = &statement.test {
                    condition.span = test.span();
                    self.predicate(test, &mut condition, 0)?
                } else {
                    let one = self.value(Type::Number);
                    Self::push(
                        &mut condition,
                        statement.as_ref(),
                        Op::Constant {
                            result: one,
                            value: 1.0,
                        },
                    );
                    let predicate = self.value(Type::Predicate);
                    Self::push(
                        &mut condition,
                        statement.as_ref(),
                        Op::Truthy {
                            result: predicate,
                            input: one,
                        },
                    );
                    predicate
                };
                condition.exit = Exit::Yield(predicate);
                let mut body = self.region(&statement.body);
                let completion = self.loop_body(&statement.body, &mut body, depth + 1)?;
                self.bindings.truncate(header_bindings);
                // Always admit the update, including after a returning body.
                let step = if let Some(update) = &statement.update {
                    let mut step = self.region(update);
                    self.discarded(update, &mut step, 0)?;
                    Some(step)
                } else {
                    None
                };
                Self::push(
                    &mut header,
                    statement.as_ref(),
                    Op::While {
                        condition,
                        body,
                        step,
                    },
                );
                self.bindings.truncate(outer);
                Self::push(region, statement.as_ref(), Op::Block(header));
                // Preserve the existing conservative structural return policy,
                // even when the source omits its test.
                Ok(completion.pretest_loop())
            }
            Statement::WhileStatement(statement) => {
                let mut condition = self.region(&statement.test);
                let value = self.predicate(&statement.test, &mut condition, 0)?;
                condition.exit = Exit::Yield(value);
                let mut body = self.region(&statement.body);
                let visible = self.bindings.len();
                let completion = self.loop_body(&statement.body, &mut body, depth + 1)?;
                self.bindings.truncate(visible);
                Self::push(
                    region,
                    statement.as_ref(),
                    Op::While {
                        condition,
                        body,
                        step: None,
                    },
                );
                Ok(completion.pretest_loop())
            }
            Statement::IfStatement(statement) => {
                let condition = self.predicate(&statement.test, region, 0)?;
                let mut consequent = self.region(&statement.consequent);
                let visible = self.bindings.len();
                let consequent_returns =
                    self.statement(&statement.consequent, &mut consequent, depth + 1)?;
                self.bindings.truncate(visible);
                let mut alternate = self.region(statement.as_ref());
                let alternate_returns = if let Some(source) = &statement.alternate {
                    alternate.span = source.span();
                    let returns = self.statement(source, &mut alternate, depth + 1)?;
                    self.bindings.truncate(visible);
                    returns
                } else {
                    Completion::NORMAL
                };
                Self::push(
                    region,
                    statement.as_ref(),
                    Op::If {
                        result: None,
                        condition,
                        consequent,
                        alternate,
                    },
                );
                Ok(consequent_returns.union(alternate_returns))
            }
            Statement::BlockStatement(block) => {
                let mut child = self.region(block.as_ref());
                let visible = self.bindings.len();
                let returns = self.sequence(&block.body, &mut child, depth + 1)?;
                self.bindings.truncate(visible);
                Self::push(region, block.as_ref(), Op::Block(child));
                Ok(returns)
            }
            Statement::ReturnStatement(statement) => {
                let expression =
                    statement
                        .argument
                        .as_ref()
                        .ok_or(if self.return_type == Type::General {
                            "return must have a primitive expression"
                        } else {
                            "return must have a numeric expression"
                        })?;
                let value = if self.return_type == Type::General {
                    self.evaluate(expression, region, 0, evaluate::Mode::General)?
                        .expect("General expression result")
                } else {
                    self.expression(expression, region, 0)?
                };
                region.exit = Exit::Return(value);
                Ok(Completion::RETURN)
            }
            Statement::ExpressionStatement(statement) => {
                self.discarded(&statement.expression, region, 0)?;
                Ok(Completion::NORMAL)
            }
            Statement::VariableDeclaration(declaration) => {
                self.declaration(declaration, region)?;
                Ok(Completion::NORMAL)
            }
            _ => Err(unsupported(
                statement,
                "statement is outside the numeric experiment",
            )),
        }
    }

    pub fn finish(self, body: Region, parameter_count: usize) -> ir::Function {
        ir::Function {
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
