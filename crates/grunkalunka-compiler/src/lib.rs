//! JS/TS parsing, verified numeric region lowering and structured Wasm emission.
//! The admitted numeric contract is documented in docs/compiler.md.
mod emit;
mod ir;
mod lower;

use lower::{Lowering, NumericBinding};

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    BindingPattern, Declaration, FormalParameterKind, FunctionType, Statement, TSType,
    TSTypeAnnotation,
};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType};

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
    let mut slots = Vec::new();
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
            slot: ir::SlotId(bindings.len()),
            symbol: identifier
                .symbol_id
                .get()
                .ok_or("missing parameter binding")?,
            mutable: true,
        });
        slots.push(ir::Slot {
            owner: ir::RegionId(0),
            mutable: true,
            parameter: true,
            span: parameter.span(),
        });
    }
    let body = function.body.as_ref().ok_or("function must have a body")?;
    if !body.directives.is_empty() {
        return Err(unsupported(
            body.as_ref(),
            "function directives are not admitted",
        ));
    }
    let mut lowering = Lowering {
        semantic: &built.semantic,
        bindings,
        local_count: 0,
        statement_count: 0,
        in_loop_body: false,
        slots,
        values: Vec::new(),
        next_region: 0,
    };
    let mut region = lowering.region(body.as_ref());
    if lowering.sequence(&body.statements, &mut region, 0)? != ir::Completion::RETURN {
        return Err(unsupported(
            body.as_ref(),
            "every structural path must return a numeric value",
        ));
    }
    let function = lowering.finish(region);
    let wat = emit::emit(&function)?;
    let wasm = wat::parse_str(&wat).map_err(|error| format!("WAT encoding: {error}"))?;
    Ok(Artifact { wat, wasm })
}
