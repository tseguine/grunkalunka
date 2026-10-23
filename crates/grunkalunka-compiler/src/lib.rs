//! JS/TS parsing and numeric AST-to-Wasm lowering.
//! The admitted numeric contract is documented in docs/compiler.md.
use std::fmt::Write;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    AssignmentTarget, BindingPattern, Declaration, Expression, FormalParameterKind, FunctionType,
    IdentifierReference, Statement, TSType, TSTypeAnnotation, VariableDeclarationKind,
};
use oxc_parser::Parser;
use oxc_semantic::{Semantic, SemanticBuilder, SymbolId};
use oxc_span::{GetSpan, SourceType};
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};

pub struct Artifact {
    pub wat: String,
    pub wasm: Vec<u8>,
}

fn unsupported(node: &impl GetSpan, reason: &str) -> String {
    let span = node.span();
    format!(
        "unsupported at bytes {}..{}: {reason}",
        span.start, span.end
    )
}

fn check_annotation(annotation: Option<&TSTypeAnnotation<'_>>) -> Result<(), String> {
    if let Some(annotation) = annotation
        && !matches!(annotation.type_annotation, TSType::TSNumberKeyword(_))
    {
        return Err(unsupported(
            annotation,
            "only an optional 'number' annotation is admitted",
        ));
    }
    Ok(())
}

// Active lexical bindings refer to monotonically allocated physical Wasm slots.
struct NumericBinding {
    symbol: SymbolId,
    slot: usize,
    mutable: bool,
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

fn lower_expression(
    expression: &Expression<'_>,
    semantic: &Semantic<'_>,
    bindings: &[NumericBinding],
    output: &mut String,
    depth: usize,
) -> Result<(), String> {
    if depth > 128 {
        return Err(unsupported(
            expression,
            "expression nesting exceeds this experiment's limit",
        ));
    }
    match expression {
        Expression::NumericLiteral(literal) => {
            // Decimal conversion of finite f64 values round-trips; overflow literals
            // are already +infinity in Oxc and use WAT's explicit spelling.
            let value = if literal.value.is_infinite() {
                "inf".to_string()
            } else {
                literal.value.to_string()
            };
            writeln!(output, "f64.const {value}").unwrap();
        }
        Expression::Identifier(identifier) => {
            let index = numeric_binding(identifier, semantic, bindings)?.slot;
            writeln!(output, "local.get {index}").unwrap();
        }
        Expression::AssignmentExpression(assignment) => {
            if assignment.operator != AssignmentOperator::Assign {
                return Err(unsupported(
                    expression,
                    "only simple '=' assignment is admitted",
                ));
            }
            let AssignmentTarget::AssignmentTargetIdentifier(identifier) = &assignment.left else {
                return Err(unsupported(
                    expression,
                    "assignment target must be an initialized identifier",
                ));
            };
            let binding = numeric_binding(identifier, semantic, bindings)?;
            if !binding.mutable {
                return Err(unsupported(
                    expression,
                    "assignment to a const binding is not admitted",
                ));
            }
            lower_expression(&assignment.right, semantic, bindings, output, depth + 1)?;
            writeln!(output, "local.tee {}", binding.slot).unwrap();
        }
        Expression::ParenthesizedExpression(parenthesized) => {
            lower_expression(
                &parenthesized.expression,
                semantic,
                bindings,
                output,
                depth + 1,
            )?;
        }
        Expression::ConditionalExpression(conditional) => {
            lower_predicate(&conditional.test, semantic, bindings, output, depth + 1)?;
            output.push_str("if (result f64)\n");
            lower_expression(
                &conditional.consequent,
                semantic,
                bindings,
                output,
                depth + 1,
            )?;
            output.push_str("else\n");
            lower_expression(
                &conditional.alternate,
                semantic,
                bindings,
                output,
                depth + 1,
            )?;
            output.push_str("end\n");
        }
        Expression::UnaryExpression(unary) => {
            if !matches!(
                unary.operator,
                UnaryOperator::UnaryPlus | UnaryOperator::UnaryNegation
            ) {
                return Err(unsupported(expression, "only unary + and - are admitted"));
            }
            lower_expression(&unary.argument, semantic, bindings, output, depth + 1)?;
            if unary.operator == UnaryOperator::UnaryNegation {
                output.push_str("f64.neg\n");
            }
        }
        Expression::BinaryExpression(binary) => {
            let instruction = match binary.operator {
                BinaryOperator::Addition => "f64.add",
                BinaryOperator::Subtraction => "f64.sub",
                BinaryOperator::Multiplication => "f64.mul",
                BinaryOperator::Division => "f64.div",
                _ => {
                    return Err(unsupported(
                        expression,
                        "only binary +, -, * and / are admitted",
                    ));
                }
            };
            // Preserve AST grouping and left/right order. No constant folding,
            // reassociation or fused multiply-add transformation occurs here.
            lower_expression(&binary.left, semantic, bindings, output, depth + 1)?;
            lower_expression(&binary.right, semantic, bindings, output, depth + 1)?;
            writeln!(output, "{instruction}").unwrap();
        }
        _ => {
            return Err(unsupported(
                expression,
                "expression is outside the numeric experiment",
            ));
        }
    }
    Ok(())
}

fn lower_predicate(
    expression: &Expression<'_>,
    semantic: &Semantic<'_>,
    bindings: &[NumericBinding],
    output: &mut String,
    depth: usize,
) -> Result<(), String> {
    if depth > 128 {
        return Err(unsupported(
            expression,
            "predicate nesting exceeds this experiment's limit",
        ));
    }
    if let Expression::ParenthesizedExpression(parenthesized) = expression {
        return lower_predicate(
            &parenthesized.expression,
            semantic,
            bindings,
            output,
            depth + 1,
        );
    }
    let Expression::BinaryExpression(binary) = expression else {
        return Err(unsupported(
            expression,
            "condition must be a numeric comparison; truthiness is not admitted yet",
        ));
    };
    let instruction = match binary.operator {
        BinaryOperator::LessThan => "f64.lt",
        BinaryOperator::LessEqualThan => "f64.le",
        BinaryOperator::GreaterThan => "f64.gt",
        BinaryOperator::GreaterEqualThan => "f64.ge",
        BinaryOperator::StrictEquality => "f64.eq",
        BinaryOperator::StrictInequality => "f64.ne",
        _ => {
            return Err(unsupported(
                expression,
                "condition must use <, <=, >, >=, === or !==",
            ));
        }
    };
    lower_expression(&binary.left, semantic, bindings, output, depth + 1)?;
    lower_expression(&binary.right, semantic, bindings, output, depth + 1)?;
    writeln!(output, "{instruction}").unwrap();
    Ok(())
}

struct StatementLowering<'s, 'a> {
    semantic: &'s Semantic<'a>,
    bindings: Vec<NumericBinding>,
    local_count: usize,
    statement_count: usize,
    output: String,
}

impl StatementLowering<'_, '_> {
    // True means every structurally possible path returns a numeric value.
    fn sequence(&mut self, statements: &[Statement<'_>], depth: usize) -> Result<bool, String> {
        let mut returns = false;
        for statement in statements {
            if returns {
                return Err(unsupported(
                    statement,
                    "statements after a guaranteed return are not admitted",
                ));
            }
            returns = self.statement(statement, depth)?;
        }
        Ok(returns)
    }

    fn statement(&mut self, statement: &Statement<'_>, depth: usize) -> Result<bool, String> {
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
            Statement::IfStatement(statement) => {
                lower_predicate(
                    &statement.test,
                    self.semantic,
                    &self.bindings,
                    &mut self.output,
                    0,
                )?;
                self.output.push_str("if\n");
                let visible = self.bindings.len();
                let consequent_returns = self.statement(&statement.consequent, depth + 1)?;
                self.bindings.truncate(visible);
                let alternate_returns = if let Some(alternate) = &statement.alternate {
                    self.output.push_str("else\n");
                    let returns = self.statement(alternate, depth + 1)?;
                    self.bindings.truncate(visible);
                    returns
                } else {
                    false
                };
                self.output.push_str("end\n");
                Ok(consequent_returns && alternate_returns)
            }
            Statement::BlockStatement(block) => {
                let visible = self.bindings.len();
                let result = self.sequence(&block.body, depth + 1);
                self.bindings.truncate(visible);
                result
            }
            Statement::ReturnStatement(statement) => {
                let expression = statement
                    .argument
                    .as_ref()
                    .ok_or("return must have a numeric expression")?;
                lower_expression(
                    expression,
                    self.semantic,
                    &self.bindings,
                    &mut self.output,
                    0,
                )?;
                self.output.push_str("return\n");
                Ok(true)
            }
            Statement::ExpressionStatement(statement) => {
                if !matches!(statement.expression, Expression::AssignmentExpression(_)) {
                    return Err(unsupported(
                        statement.as_ref(),
                        "only assignments are admitted as expression statements",
                    ));
                }
                lower_expression(
                    &statement.expression,
                    self.semantic,
                    &self.bindings,
                    &mut self.output,
                    0,
                )?;
                self.output.push_str("drop\n");
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
                // Publish a binding only after its initializer has been admitted/lowered.
                // Even dead-branch self/forward references are outside this narrow contract.
                lower_expression(
                    initializer,
                    self.semantic,
                    &self.bindings,
                    &mut self.output,
                    0,
                )?;
                let slot = 2 + self.local_count;
                self.local_count += 1;
                writeln!(self.output, "local.set {slot}").unwrap();
                self.bindings.push(NumericBinding {
                    symbol: identifier.symbol_id.get().ok_or("missing local binding")?,
                    slot,
                    mutable: declaration.kind == VariableDeclarationKind::Let,
                });

                Ok(false)
            }
            _ => Err(unsupported(
                statement,
                "statement is outside the numeric experiment",
            )),
        }
    }
}

pub fn compile(source: &str, typescript: bool) -> Result<Artifact, String> {
    if source.len() > 16_384 {
        return Err("source exceeds this experiment's 16 KiB limit".into());
    }
    let allocator = Allocator::default();
    let source_type = if typescript {
        SourceType::ts().with_module(true)
    } else {
        SourceType::mjs()
    };
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if parsed.fatal_error || !parsed.diagnostics.is_empty() {
        return Err(format!("parse diagnostics: {:?}", parsed.diagnostics));
    }
    let built = SemanticBuilder::new_compiler().build(&parsed.program);
    if !built.diagnostics.is_empty() {
        return Err(format!("semantic diagnostics: {:?}", built.diagnostics));
    }
    let program = &parsed.program;
    if program.hashbang.is_some() || !program.directives.is_empty() || program.body.len() != 1 {
        return Err(unsupported(
            program,
            "exactly one exported function and no directives/hashbang are admitted",
        ));
    }
    let Statement::ExportDeclaration(export) = &program.body[0] else {
        return Err(unsupported(program, "expected 'export function run(a, b)'"));
    };
    let Declaration::FunctionDeclaration(function) = &export.declaration else {
        return Err(unsupported(
            export.as_ref(),
            "only a function declaration is admitted",
        ));
    };
    if function.r#type != FunctionType::FunctionDeclaration
        || function
            .id
            .as_ref()
            .is_none_or(|id| id.name.as_str() != "run")
        || function.r#async
        || function.generator
        || function.declare
        || function.type_parameters.is_some()
        || function.this_param.is_some()
    {
        return Err(unsupported(
            function.as_ref(),
            "expected a plain, synchronous, non-generic function named run",
        ));
    }
    check_annotation(function.return_type.as_deref())?;
    if function.params.kind != FormalParameterKind::FormalParameter
        || function.params.items.len() != 2
        || function.params.rest.is_some()
    {
        return Err(unsupported(
            function.params.as_ref(),
            "exactly two ordinary numeric parameters are admitted",
        ));
    }
    let mut bindings = Vec::new();
    for parameter in &function.params.items {
        if parameter.optional
            || parameter.initializer.is_some()
            || !parameter.decorators.is_empty()
            || parameter.accessibility.is_some()
            || parameter.readonly
            || parameter.r#override
        {
            return Err(unsupported(
                parameter,
                "parameter modifiers, defaults and decorators are not admitted",
            ));
        }
        check_annotation(parameter.type_annotation.as_deref())?;
        let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern else {
            return Err(unsupported(
                parameter,
                "destructured parameters are not admitted",
            ));
        };
        bindings.push(NumericBinding {
            slot: bindings.len(),
            symbol: identifier
                .symbol_id
                .get()
                .ok_or("missing parameter binding")?,
            mutable: true,
        });
    }
    let body = function.body.as_ref().ok_or("function must have a body")?;
    if !body.directives.is_empty() {
        return Err(unsupported(
            body.as_ref(),
            "function directives are not admitted",
        ));
    }
    let mut lowering = StatementLowering {
        semantic: &built.semantic,
        bindings,
        local_count: 0,
        statement_count: 0,
        output: String::new(),
    };
    if !lowering.sequence(&body.statements, 0)? {
        return Err(unsupported(
            body.as_ref(),
            "every structural path must return a numeric value",
        ));
    }
    let mut wat = "(module (func (export \"run\") (param f64 f64) (result f64)\n".to_string();
    if lowering.local_count > 0 {
        writeln!(
            wat,
            "(local {})",
            vec!["f64"; lowering.local_count].join(" ")
        )
        .unwrap();
    }
    wat.push_str(&lowering.output);
    // Structured return analysis above rules out source fallthrough. This is only
    // a validator aid after nested void blocks/branches whose arms return.
    wat.push_str("unreachable\n))\n");
    let wasm = wat::parse_str(&wat).map_err(|error| format!("WAT encoding: {error}"))?;
    Ok(Artifact { wat, wasm })
}
