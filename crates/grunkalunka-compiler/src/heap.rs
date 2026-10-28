//! Provisional String/JSON expression heap profile, grown by counterexamples.
use crate::{Artifact, lower};
use oxc_allocator::Allocator;
use oxc_ast::ast::{BindingPattern, Declaration, Expression, Statement};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

pub(super) fn compile(source: &str, typescript: bool) -> Result<Artifact, String> {
    if source.len() > 16_384 {
        return Err("source exceeds the heap profile's 16 KiB limit".into());
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
    let built = SemanticBuilder::new_compiler()
        .with_build_nodes(true)
        .build(&parsed.program);
    if !built.diagnostics.is_empty() {
        return Err(format!("semantic diagnostics: {:?}", built.diagnostics));
    }
    let [Statement::ExportDeclaration(export)] = parsed.program.body.as_slice() else {
        return Err("heap spike expects one exported function".into());
    };
    let Declaration::FunctionDeclaration(function) = &export.declaration else {
        return Err("heap spike expects a function".into());
    };
    if function.r#async
        || function.generator
        || function.declare
        || function.this_param.is_some()
        || function.id.as_ref().is_none_or(|id| id.name != "run")
    {
        return Err("heap profile expects a plain synchronous function named run".into());
    }
    if function.params.items.len() != 2 || function.params.rest.is_some() {
        return Err("heap profile expects two ordinary numeric parameters".into());
    }
    for parameter in &function.params.items {
        if !matches!(parameter.pattern, BindingPattern::BindingIdentifier(_))
            || parameter.optional
            || parameter.initializer.is_some()
            || !parameter.decorators.is_empty()
            || parameter.accessibility.is_some()
            || parameter.readonly
            || parameter.r#override
        {
            return Err(
                "heap profile expects ordinary parameters without defaults or modifiers".into(),
            );
        }
    }
    let body = function.body.as_ref().ok_or("heap spike expects a body")?;
    let [Statement::ReturnStatement(returned)] = body.statements.as_slice() else {
        return Err("heap profile expects one direct expression return".into());
    };
    let mut expression = returned
        .argument
        .as_ref()
        .ok_or("heap spike expects a return value")?;
    // Each admitted operation has one operand. Evaluation is a linear chain,
    // so no other temporary needs pinning across the next guest allocation.
    let mut operations = Vec::new();
    let mut depth = 0;
    loop {
        depth += 1;
        if depth > 128 {
            return Err("heap expression exceeds the 128-node nesting limit".into());
        }
        if let Some(inner) = lower::erased_inner(expression) {
            expression = inner;
        } else if let oxc_ast::ast::Expression::ParenthesizedExpression(wrapper) = expression {
            expression = &wrapper.expression;
        } else if let Expression::CallExpression(call) = expression {
            let Expression::StaticMemberExpression(member) = &call.callee else {
                return Err("heap profile expects JSON.parse or JSON.stringify".into());
            };
            let Expression::Identifier(object) = &member.object else {
                return Err("heap profile expects the unshadowed JSON intrinsic".into());
            };
            let unresolved = object.reference_id.get().is_some_and(|reference| {
                built
                    .semantic
                    .scoping()
                    .get_reference(reference)
                    .symbol_id()
                    .is_none()
            });
            if object.name != "JSON"
                || !unresolved
                || call.optional
                || member.optional
                || call.arguments.len() != 1
            {
                return Err(
                    "heap JSON calls require one argument and the unshadowed intrinsic".into(),
                );
            }
            let operation = match member.property.name.as_str() {
                "parse" => "parse",
                "stringify" => "stringify",
                _ => return Err("heap profile expects JSON.parse or JSON.stringify".into()),
            };
            operations.push(operation);
            expression = call.arguments[0]
                .as_expression()
                .ok_or("heap JSON calls do not admit spread arguments")?;
        } else {
            break;
        }
    }
    let oxc_ast::ast::Expression::StringLiteral(literal) = expression else {
        return Err("heap expression must begin with a String literal".into());
    };
    let mut is_string = true;
    let mut imports = String::new();
    let mut instructions = String::new();
    for operation in operations.iter().rev() {
        if *operation == "parse" && !is_string {
            return Err("heap JSON.parse currently requires a statically known String".into());
        }
        is_string = *operation == "stringify";
        instructions.push_str(&format!(
            "\n        local.get $frame\n        local.get $root\n        call ${operation}\n        local.tee $root\n        i32.eqz\n        if unreachable end"
        ));
    }
    if !is_string {
        return Err(
            "heap host currently requires a String result; serialize parsed JSON before returning"
                .into(),
        );
    }
    for operation in ["parse", "stringify"] {
        if operations.contains(&operation) {
            imports.push_str(&format!(
                "\n      (import \"runtime\" \"gk_heap_{operation}\" (func ${operation} (param i32 i32) (result i32)))"
            ));
        }
    }
    let units: Vec<u16> = literal.value.encode_utf16().collect();
    let units_len = units.len();
    let byte_len = units_len * 2;
    let data: String = units
        .iter()
        .flat_map(|unit| unit.to_le_bytes())
        .map(|byte| format!("\\{byte:02x}"))
        .collect();
    let wat = format!(
        r#"(module
      (import "runtime" "memory" (memory 1))
      (import "runtime" "gk_heap_enter" (func $enter (result i32)))
      (import "runtime" "gk_json_input_resize" (func $input (param i32) (result i32)))
      (import "runtime" "gk_heap_string" (func $string (param i32) (result i32)))
      (import "runtime" "gk_heap_leave" (func $leave (param i32 i32) (result i32))){imports}
      (func (export "run") (param f64 f64) (result i32 f64)
        (local $frame i32) (local $root i32) (local $pointer i32)
        call $enter
        local.tee $frame
        i32.eqz
        if unreachable end
        i32.const {units_len}
        call $input
        local.tee $pointer
        i32.eqz
        if unreachable end
        local.get $pointer
        i32.const 0
        i32.const {byte_len}
        memory.init $literal
        local.get $frame
        call $string
        local.tee $root
        i32.eqz
        if unreachable end{instructions}
        local.get $frame
        local.get $root
        call $leave
        if unreachable end
        i32.const 4
        local.get $root
        f64.convert_i32_u)
      (data $literal "{data}"))"#
    );
    let wasm = wat::parse_str(&wat).map_err(|error| format!("WAT encoding: {error}"))?;
    Ok(Artifact { wat, wasm })
}
