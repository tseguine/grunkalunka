//! Select the public entry by its export name and resolve local aliases through Oxc.
use super::unsupported;
use oxc_ast::ast::{
    Declaration, ExportNamedDeclaration, Function, ImportOrExportKind, ModuleExportName, Program,
    Statement,
};
use oxc_semantic::{Semantic, SymbolId};

pub(super) struct ModuleFunctions<'n, 'a> {
    pub declarations: Vec<&'n Function<'a>>,
    pub exports_run: bool,
}

fn run_export_binding(
    export: &ExportNamedDeclaration<'_>,
    semantic: &Semantic<'_>,
) -> Option<SymbolId> {
    let [specifier] = export.specifiers.as_slice() else {
        return None;
    };
    if export.export_kind != ImportOrExportKind::Value
        || specifier.export_kind != ImportOrExportKind::Value
        || specifier.exported.name() != "run"
    {
        return None;
    }
    let ModuleExportName::IdentifierReference(local) = &specifier.local else {
        return None;
    };
    semantic
        .scoping()
        .get_reference(local.reference_id.get()?)
        .symbol_id()
}

pub(super) fn select<'n, 'a>(
    program: &Program<'a>,
    semantic: &Semantic<'a>,
    runtime: &[&'n Statement<'a>],
) -> Result<ModuleFunctions<'n, 'a>, String> {
    if runtime.is_empty() {
        return Err(unsupported(
            program,
            "exactly one exported function is admitted",
        ));
    }
    // Keep the original single-runtime-item diagnostics, including a bare run.
    if runtime.len() == 1 && !matches!(runtime[0], Statement::ExportDeclaration(_)) {
        return Err(unsupported(program, "expected 'export function run(a, b)'"));
    }
    let mut entry = None;
    let mut named = None;
    let mut helpers = Vec::new();
    for statement in runtime {
        match statement {
            Statement::ExportDeclaration(export) => {
                let Declaration::FunctionDeclaration(function) = &export.declaration else {
                    return Err(if runtime.len() == 1 {
                        unsupported(export.as_ref(), "only a function declaration is admitted")
                    } else {
                        unsupported(program, "exactly one exported function is admitted")
                    });
                };
                if entry.replace(function.as_ref()).is_some() {
                    return Err(unsupported(
                        program,
                        "exactly one exported function is admitted",
                    ));
                }
            }
            Statement::ExportNamedDeclaration(export) => {
                let symbol = run_export_binding(export, semantic).ok_or_else(|| {
                    unsupported(program, "exactly one exported function is admitted")
                })?;
                if named.replace(symbol).is_some() {
                    return Err(unsupported(
                        program,
                        "exactly one exported function is admitted",
                    ));
                }
            }
            Statement::FunctionDeclaration(function)
                if !function.declare && function.body.is_some() =>
            {
                helpers.push(function.as_ref());
            }
            _ => {
                return Err(unsupported(
                    program,
                    "exactly one exported function is admitted",
                ));
            }
        }
    }
    if let Some(symbol) = named {
        if entry.is_some() {
            return Err(unsupported(
                program,
                "exactly one exported function is admitted",
            ));
        }
        let index = helpers
            .iter()
            .position(|function| {
                function.id.as_ref().and_then(|id| id.symbol_id.get()) == Some(symbol)
            })
            .ok_or_else(|| {
                unsupported(
                    program,
                    "the run export must resolve to a module function declaration",
                )
            })?;
        entry = Some(helpers.remove(index));
    }
    let entry =
        entry.ok_or_else(|| unsupported(program, "exactly one exported function is admitted"))?;
    let exports_run = named.is_some() || entry.id.as_ref().is_some_and(|id| id.name == "run");
    Ok(ModuleFunctions {
        declarations: std::iter::once(entry).chain(helpers).collect(),
        exports_run,
    })
}
