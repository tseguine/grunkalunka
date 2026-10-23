//! Narrow, disposable AST-to-WAT lowering. This is not the platform language contract.
use std::fmt::Write;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    BindingPattern, Declaration, Expression, FormalParameterKind, FunctionType, Statement, TSType,
    TSTypeAnnotation,
};
use oxc_parser::Parser;
use oxc_semantic::{Semantic, SemanticBuilder, SymbolId};
use oxc_span::{GetSpan, SourceType};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};

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

fn lower_expression(
    expression: &Expression<'_>,
    semantic: &Semantic<'_>,
    parameters: &[SymbolId],
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
            let reference = identifier
                .reference_id
                .get()
                .ok_or("missing semantic reference")?;
            let symbol = semantic.scoping().get_reference(reference).symbol_id();
            let index = parameters
                .iter()
                .position(|parameter| Some(*parameter) == symbol)
                .ok_or_else(|| {
                    unsupported(
                        expression,
                        "only references to the two parameters are admitted",
                    )
                })?;
            writeln!(output, "local.get {index}").unwrap();
        }
        Expression::ParenthesizedExpression(parenthesized) => {
            lower_expression(
                &parenthesized.expression,
                semantic,
                parameters,
                output,
                depth + 1,
            )?;
        }
        Expression::UnaryExpression(unary) => {
            if !matches!(
                unary.operator,
                UnaryOperator::UnaryPlus | UnaryOperator::UnaryNegation
            ) {
                return Err(unsupported(expression, "only unary + and - are admitted"));
            }
            lower_expression(&unary.argument, semantic, parameters, output, depth + 1)?;
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
            lower_expression(&binary.left, semantic, parameters, output, depth + 1)?;
            lower_expression(&binary.right, semantic, parameters, output, depth + 1)?;
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
    let mut parameters = Vec::new();
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
        parameters.push(
            identifier
                .symbol_id
                .get()
                .ok_or("missing parameter binding")?,
        );
    }
    let body = function.body.as_ref().ok_or("function must have a body")?;
    if !body.directives.is_empty() || body.statements.len() != 1 {
        return Err(unsupported(
            body.as_ref(),
            "body must contain exactly one return expression and no directives",
        ));
    }
    let Statement::ReturnStatement(return_statement) = &body.statements[0] else {
        return Err(unsupported(
            body.as_ref(),
            "body must contain a return expression",
        ));
    };
    let expression = return_statement
        .argument
        .as_ref()
        .ok_or("return must have a numeric expression")?;
    let mut wat = "(module (func (export \"run\") (param f64 f64) (result f64)\n".to_string();
    lower_expression(expression, &built.semantic, &parameters, &mut wat, 0)?;
    wat.push_str("))\n");
    let wasm = wat::parse_str(&wat).map_err(|error| format!("WAT encoding: {error}"))?;
    Ok(Artifact { wat, wasm })
}
