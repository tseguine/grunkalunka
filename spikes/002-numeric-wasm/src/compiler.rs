//! Narrow, disposable AST-to-WAT lowering. This is not the platform language contract.
use std::fmt::Write;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    BindingPattern, Declaration, Expression, FormalParameterKind, FunctionType, Statement, TSType,
    TSTypeAnnotation, VariableDeclarationKind,
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
                        "reference is not an initialized numeric parameter or local",
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
        Expression::ConditionalExpression(conditional) => {
            lower_predicate(&conditional.test, semantic, parameters, output, depth + 1)?;
            output.push_str("if (result f64)\n");
            lower_expression(
                &conditional.consequent,
                semantic,
                parameters,
                output,
                depth + 1,
            )?;
            output.push_str("else\n");
            lower_expression(
                &conditional.alternate,
                semantic,
                parameters,
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

fn lower_predicate(
    expression: &Expression<'_>,
    semantic: &Semantic<'_>,
    bindings: &[SymbolId],
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
    if !body.directives.is_empty() || body.statements.is_empty() || body.statements.len() > 129 {
        return Err(unsupported(
            body.as_ref(),
            "body must contain at most 128 const locals followed by one return, and no directives",
        ));
    }
    let mut instructions = String::new();
    let parameter_count = parameters.len();
    for statement in &body.statements[..body.statements.len() - 1] {
        let Statement::VariableDeclaration(declaration) = statement else {
            return Err(unsupported(
                statement,
                "only numeric const declarations may precede the final return",
            ));
        };
        if declaration.kind != VariableDeclarationKind::Const
            || declaration.declare
            || declaration.declarations.len() != 1
        {
            return Err(unsupported(
                declaration.as_ref(),
                "one initialized const identifier per declaration is admitted",
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
            .ok_or("const local must have an initializer")?;
        // Publish a binding only after its initializer has been admitted/lowered.
        // Even dead-branch self/forward references are outside this narrow contract.
        lower_expression(
            initializer,
            &built.semantic,
            &parameters,
            &mut instructions,
            0,
        )?;
        writeln!(instructions, "local.set {}", parameters.len()).unwrap();
        parameters.push(identifier.symbol_id.get().ok_or("missing local binding")?);
    }
    let Statement::ReturnStatement(return_statement) = body.statements.last().unwrap() else {
        return Err(unsupported(
            body.as_ref(),
            "body must contain a return expression",
        ));
    };
    let expression = return_statement
        .argument
        .as_ref()
        .ok_or("return must have a numeric expression")?;
    lower_expression(
        expression,
        &built.semantic,
        &parameters,
        &mut instructions,
        0,
    )?;
    let mut wat = "(module (func (export \"run\") (param f64 f64) (result f64)\n".to_string();
    if parameters.len() > parameter_count {
        writeln!(
            wat,
            "(local {})",
            vec!["f64"; parameters.len() - parameter_count].join(" ")
        )
        .unwrap();
    }
    wat.push_str(&instructions);
    wat.push_str("))\n");
    let wasm = wat::parse_str(&wat).map_err(|error| format!("WAT encoding: {error}"))?;
    Ok(Artifact { wat, wasm })
}
