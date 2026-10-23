//! Source admission and AST-to-region lowering. Source limits count source nodes,
//! never the generated regions, values or Wasm temporary locals.
use oxc_ast::ast::{
    AssignmentTarget, BindingPattern, Expression, IdentifierReference, Statement,
    VariableDeclarationKind,
};
use oxc_semantic::{Semantic, SymbolId};
use oxc_span::GetSpan;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};

use crate::ir::{
    self, Binary, Compare, Exit, Op, Operation, Region, RegionId, Slot, SlotId, Type, Unary,
    ValueId,
};
use crate::{check_annotation, unsupported};

pub(super) struct NumericBinding {
    pub symbol: SymbolId,
    pub slot: SlotId,
    pub mutable: bool,
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

pub(super) struct Lowering<'s, 'a> {
    pub semantic: &'s Semantic<'a>,
    pub bindings: Vec<NumericBinding>,
    pub slots: Vec<Slot>,
    pub values: Vec<Type>,
    pub next_region: usize,
    pub local_count: usize,
    pub statement_count: usize,
}

impl Lowering<'_, '_> {
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

    fn expression(
        &mut self,
        expression: &Expression<'_>,
        region: &mut Region,
        depth: usize,
    ) -> Result<ValueId, String> {
        if depth > 128 {
            return Err(unsupported(
                expression,
                "expression nesting exceeds this experiment's limit",
            ));
        }
        let kind = match expression {
            Expression::NumericLiteral(literal) => {
                let result = self.value(Type::Number);
                Op::Constant {
                    result,
                    value: literal.value,
                }
            }
            Expression::Identifier(identifier) => {
                let slot = numeric_binding(identifier, self.semantic, &self.bindings)?.slot;
                let result = self.value(Type::Number);
                Op::Load { result, slot }
            }
            Expression::AssignmentExpression(assignment) => {
                if assignment.operator != AssignmentOperator::Assign {
                    return Err(unsupported(
                        expression,
                        "only simple '=' assignment is admitted",
                    ));
                }
                let AssignmentTarget::AssignmentTargetIdentifier(identifier) = &assignment.left
                else {
                    return Err(unsupported(
                        expression,
                        "assignment target must be an initialized identifier",
                    ));
                };
                let binding = numeric_binding(identifier, self.semantic, &self.bindings)?;
                if !binding.mutable {
                    return Err(unsupported(
                        expression,
                        "assignment to a const binding is not admitted",
                    ));
                }
                let slot = binding.slot;
                let value = self.expression(&assignment.right, region, depth + 1)?;
                Self::push(region, expression, Op::Store { slot, value });
                // An assignment yields the RHS snapshot, never a delayed cell read.
                return Ok(value);
            }
            Expression::ParenthesizedExpression(parenthesized) => {
                return self.expression(&parenthesized.expression, region, depth + 1);
            }
            Expression::ConditionalExpression(conditional) => {
                let condition = self.predicate(&conditional.test, region, depth + 1)?;
                let mut consequent = self.region(&conditional.consequent);
                let value = self.expression(&conditional.consequent, &mut consequent, depth + 1)?;
                consequent.exit = Exit::Yield(value);
                let mut alternate = self.region(&conditional.alternate);
                let value = self.expression(&conditional.alternate, &mut alternate, depth + 1)?;
                alternate.exit = Exit::Yield(value);
                let result = self.value(Type::Number);
                Self::push(
                    region,
                    expression,
                    Op::If {
                        result: Some(result),
                        condition,
                        consequent,
                        alternate,
                    },
                );
                return Ok(result);
            }
            Expression::UnaryExpression(unary) => {
                let operator = match unary.operator {
                    UnaryOperator::UnaryPlus => Unary::Plus,
                    UnaryOperator::UnaryNegation => Unary::Negate,
                    _ => return Err(unsupported(expression, "only unary + and - are admitted")),
                };
                let input = self.expression(&unary.argument, region, depth + 1)?;
                let result = self.value(Type::Number);
                Op::Unary {
                    result,
                    operator,
                    input,
                }
            }
            Expression::BinaryExpression(binary) => {
                let operator = match binary.operator {
                    BinaryOperator::Addition => Binary::Add,
                    BinaryOperator::Subtraction => Binary::Subtract,
                    BinaryOperator::Multiplication => Binary::Multiply,
                    BinaryOperator::Division => Binary::Divide,
                    _ => {
                        return Err(unsupported(
                            expression,
                            "only binary +, -, * and / are admitted",
                        ));
                    }
                };
                let left = self.expression(&binary.left, region, depth + 1)?;
                let right = self.expression(&binary.right, region, depth + 1)?;
                let result = self.value(Type::Number);
                Op::Binary {
                    result,
                    operator,
                    left,
                    right,
                }
            }
            _ => {
                return Err(unsupported(
                    expression,
                    "expression is outside the numeric experiment",
                ));
            }
        };
        let result = match &kind {
            Op::Constant { result, .. }
            | Op::Load { result, .. }
            | Op::Unary { result, .. }
            | Op::Binary { result, .. } => *result,
            _ => unreachable!("numeric expression operation"),
        };
        Self::push(region, expression, kind);
        Ok(result)
    }

    fn predicate(
        &mut self,
        expression: &Expression<'_>,
        region: &mut Region,
        depth: usize,
    ) -> Result<ValueId, String> {
        if depth > 128 {
            return Err(unsupported(
                expression,
                "predicate nesting exceeds this experiment's limit",
            ));
        }
        if let Expression::ParenthesizedExpression(parenthesized) = expression {
            return self.predicate(&parenthesized.expression, region, depth + 1);
        }
        if let Expression::BinaryExpression(binary) = expression {
            let operator = match binary.operator {
                BinaryOperator::LessThan => Some(Compare::Less),
                BinaryOperator::LessEqualThan => Some(Compare::LessEqual),
                BinaryOperator::GreaterThan => Some(Compare::Greater),
                BinaryOperator::GreaterEqualThan => Some(Compare::GreaterEqual),
                BinaryOperator::StrictEquality => Some(Compare::Equal),
                BinaryOperator::StrictInequality => Some(Compare::NotEqual),
                _ => None,
            };
            if let Some(operator) = operator {
                let left = self.expression(&binary.left, region, depth + 1)?;
                let right = self.expression(&binary.right, region, depth + 1)?;
                let result = self.value(Type::Predicate);
                Self::push(
                    region,
                    expression,
                    Op::Compare {
                        result,
                        operator,
                        left,
                        right,
                    },
                );
                return Ok(result);
            }
        }
        let input = self.expression(expression, region, depth)?;
        let result = self.value(Type::Predicate);
        Self::push(region, expression, Op::Truthy { result, input });
        Ok(result)
    }

    // True means every structurally possible path returns a numeric value.
    pub fn sequence(
        &mut self,
        statements: &[Statement<'_>],
        region: &mut Region,
        depth: usize,
    ) -> Result<bool, String> {
        let mut returns = false;
        for statement in statements {
            if returns {
                return Err(unsupported(
                    statement,
                    "statements after a guaranteed return are not admitted",
                ));
            }
            returns = self.statement(statement, region, depth)?;
        }
        Ok(returns)
    }

    fn statement(
        &mut self,
        statement: &Statement<'_>,
        region: &mut Region,
        depth: usize,
    ) -> Result<bool, String> {
        if depth > 64 {
            return Err(unsupported(statement, "statement nesting exceeds 64"));
        }
        if self.statement_count >= 256 {
            return Err(unsupported(
                statement,
                "at most 256 statements including blocks and returns are admitted",
            ));
        }
        self.statement_count += 1;
        match statement {
            Statement::WhileStatement(statement) => {
                let mut condition = self.region(&statement.test);
                let value = self.predicate(&statement.test, &mut condition, 0)?;
                condition.exit = Exit::Yield(value);
                let mut body = self.region(&statement.body);
                let visible = self.bindings.len();
                self.statement(&statement.body, &mut body, depth + 1)?;
                self.bindings.truncate(visible);
                Self::push(region, statement.as_ref(), Op::While { condition, body });
                Ok(false)
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
                    false
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
                Ok(consequent_returns && alternate_returns)
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
                let expression = statement
                    .argument
                    .as_ref()
                    .ok_or("return must have a numeric expression")?;
                region.exit = Exit::Return(self.expression(expression, region, 0)?);
                Ok(true)
            }
            Statement::ExpressionStatement(statement) => {
                if !matches!(statement.expression, Expression::AssignmentExpression(_)) {
                    return Err(unsupported(
                        statement.as_ref(),
                        "only assignments are admitted as expression statements",
                    ));
                }
                self.expression(&statement.expression, region, 0)?;
                Ok(false)
            }
            Statement::VariableDeclaration(declaration) => {
                if self.local_count >= 128 {
                    return Err(unsupported(
                        statement,
                        "at most 128 numeric locals are admitted",
                    ));
                }
                if !matches!(
                    declaration.kind,
                    VariableDeclarationKind::Const | VariableDeclarationKind::Let
                ) || declaration.declare
                    || declaration.declarations.len() != 1
                {
                    return Err(unsupported(
                        declaration.as_ref(),
                        "one initialized const or let identifier per declaration is admitted",
                    ));
                }
                let local = &declaration.declarations[0];
                if local.definite {
                    return Err(unsupported(
                        local,
                        "definite-assignment assertions are not admitted",
                    ));
                }
                check_annotation(local.type_annotation.as_deref())?;
                let BindingPattern::BindingIdentifier(identifier) = &local.id else {
                    return Err(unsupported(local, "local destructuring is not admitted"));
                };
                let initializer = local
                    .init
                    .as_ref()
                    .ok_or("numeric local must have an initializer")?;
                let value = self.expression(initializer, region, 0)?;
                let slot = SlotId(self.slots.len());
                let mutable = declaration.kind == VariableDeclarationKind::Let;
                self.slots.push(Slot {
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
                Ok(false)
            }
            _ => Err(unsupported(
                statement,
                "statement is outside the numeric experiment",
            )),
        }
    }

    pub fn finish(self, body: Region) -> ir::Function {
        ir::Function {
            slots: self.slots,
            values: self.values,
            body,
        }
    }
}
