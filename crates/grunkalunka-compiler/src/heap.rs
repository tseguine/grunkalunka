//! Provisional literal-only heap profile, grown by counterexamples.
use crate::{Artifact, lower};
use oxc_allocator::Allocator;
use oxc_ast::ast::{Declaration, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;

pub(super) fn compile(source: &str, typescript: bool) -> Result<Artifact, String> {
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
    let [Statement::ExportDeclaration(export)] = parsed.program.body.as_slice() else {
        return Err("heap spike expects one exported function".into());
    };
    let Declaration::FunctionDeclaration(function) = &export.declaration else {
        return Err("heap spike expects a function".into());
    };
    if function.r#async || function.generator {
        return Err("heap spike expects a synchronous non-generator function".into());
    }
    let body = function.body.as_ref().ok_or("heap spike expects a body")?;
    let [Statement::ReturnStatement(returned)] = body.statements.as_slice() else {
        return Err("heap spike expects a direct literal return".into());
    };
    let mut expression = returned
        .argument
        .as_ref()
        .ok_or("heap spike expects a return value")?;
    loop {
        if let Some(inner) = lower::erased_inner(expression) {
            expression = inner;
        } else if let oxc_ast::ast::Expression::ParenthesizedExpression(wrapper) = expression {
            expression = &wrapper.expression;
        } else {
            break;
        }
    }
    let oxc_ast::ast::Expression::StringLiteral(literal) = expression else {
        return Err("heap spike expects a String literal".into());
    };
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
      (import "runtime" "gk_heap_leave" (func $leave (param i32 i32) (result i32)))
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
        if unreachable end
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
