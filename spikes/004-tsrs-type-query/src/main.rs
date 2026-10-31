use std::{
    collections::HashMap,
    error::Error,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::SystemTime,
};

use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Statement};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use tsrs_ast::Kind;
use tsrs_compiler::{Context, ProgramOptions};
use tsrs_core::{CompilerOptions, P, Tristate, arena::Region, tspath};
use tsrs_vfs::{Entries, FS, FileInfo};

const INPUT_DIRECTORY: &str = "/p";
const LIB_DIRECTORY: &str = "/lib";

#[derive(Default)]
struct MemoryFs {
    files: HashMap<String, String>,
    unexpected: Mutex<Option<String>>,
}

impl MemoryFs {
    fn note(&self, operation: &str, path: &str) {
        let mut unexpected = self.unexpected.lock().unwrap();
        if unexpected.is_none() {
            *unexpected = Some(format!("unexpected {operation} for {path:?}"));
        }
    }
}

impl FS for MemoryFs {
    fn use_case_sensitive_file_names(&self) -> bool {
        true
    }

    fn file_exists(&self, path: &str) -> bool {
        let exists = self.files.contains_key(path);
        if !exists {
            self.note("file existence check", path);
        }
        exists
    }

    fn read_file(&self, path: &str) -> Option<String> {
        let content = self.files.get(path).cloned();
        if content.is_none() {
            self.note("file read", path);
        }
        content
    }

    fn write_file(&self, path: &str, _data: &str) -> Result<(), String> {
        Err(format!("unexpected write for {path:?}"))
    }

    fn append_file(&self, path: &str, _data: &str) -> Result<(), String> {
        Err(format!("unexpected append for {path:?}"))
    }

    fn remove(&self, path: &str) -> Result<(), String> {
        Err(format!("unexpected remove for {path:?}"))
    }

    fn chtimes(
        &self,
        path: &str,
        _accessed: SystemTime,
        _modified: SystemTime,
    ) -> Result<(), String> {
        Err(format!("unexpected chtimes for {path:?}"))
    }

    fn directory_exists(&self, path: &str) -> bool {
        self.note("directory existence check", path);
        false
    }

    fn get_accessible_entries(&self, path: &str) -> Entries {
        self.note("directory listing", path);
        Entries::default()
    }

    fn stat(&self, path: &str) -> Option<FileInfo> {
        self.note("stat", path);
        None
    }

    fn realpath(&self, path: &str) -> String {
        self.note("realpath request", path);
        path.to_string()
    }
}

enum ExpressionSelector {
    OxcSpan((usize, usize)),
    TsrsSource(&'static str),
}

fn oxc_binary_expression_span(
    source: &str,
    source_type: SourceType,
) -> Result<(usize, usize), Box<dyn Error>> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if parsed.fatal_error || !parsed.diagnostics.is_empty() {
        return Err(format!("Oxc parse diagnostics: {:?}", parsed.diagnostics).into());
    }
    let built = SemanticBuilder::new_compiler().build(&parsed.program);
    if !built.diagnostics.is_empty() {
        return Err(format!("Oxc semantic diagnostics: {:?}", built.diagnostics).into());
    }

    for statement in &parsed.program.body {
        let Statement::VariableDeclaration(declaration) = statement else {
            continue;
        };
        for binding in &declaration.declarations {
            let Some(Expression::BinaryExpression(expression)) = &binding.init else {
                continue;
            };
            if &source[expression.span.start as usize..expression.span.end as usize] == "seed + 2"
                || &source[expression.span.start as usize..expression.span.end as usize] == "40 + 2"
            {
                return Ok((expression.span.start as usize, expression.span.end as usize));
            }
        }
    }
    Err("Oxc did not find the expected binary expression".into())
}

fn oxc_parse_diagnostic_count(source: &str, source_type: SourceType) -> usize {
    let allocator = Allocator::default();
    Parser::new(&allocator, source, source_type)
        .parse()
        .diagnostics
        .len()
}

fn find_tsrs_expression(
    node: tsrs_core::P<tsrs_ast::Node>,
    source: &str,
    expression_text: &str,
    matches: &mut Vec<tsrs_core::P<tsrs_ast::Node>>,
) {
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        if node.kind() == Kind::BinaryExpression {
            let start = node
                .pos()
                .expect_source("parsed node has a source position") as usize;
            let end = node.end().expect_source("parsed node has a source end") as usize;
            if source
                .get(start..end)
                .is_some_and(|text| text.trim() == expression_text)
            {
                matches.push(node);
            }
        }
        // tsrs exposes immediate children via a callback. Collect and reverse
        // them so our LIFO worklist retains recursive preorder.
        let mut children = Vec::new();
        node.for_each_child(&mut |child| {
            children.push(child);
            false
        });
        pending.extend(children.into_iter().rev());
    }
}

fn check_with_tsrs(
    source: &str,
    extension: &str,
    selector: Option<ExpressionSelector>,
) -> Result<(usize, usize, Option<String>, Option<(u32, u32)>), Box<dyn Error>> {
    let region = Region::new(1 << 20);
    let result = {
        let _scope = region.enter();
        check_with_tsrs_in_region(source, extension, selector)
    };
    drop(region);
    result
}

fn check_with_tsrs_in_region(
    source: &str,
    extension: &str,
    selector: Option<ExpressionSelector>,
) -> Result<(usize, usize, Option<String>, Option<(u32, u32)>), Box<dyn Error>> {
    let file_name = format!("{INPUT_DIRECTORY}/main.{extension}");
    let files = HashMap::from([(file_name.clone(), source.to_string())]);
    let fs = Arc::new(MemoryFs {
        files,
        unexpected: Mutex::new(None),
    });
    let host = tsrs_compiler::new_compiler_host(
        INPUT_DIRECTORY,
        Arc::<MemoryFs>::clone(&fs),
        LIB_DIRECTORY,
        None,
        None,
        None,
    );

    let mut options = CompilerOptions::default();
    options.no_lib = Tristate::True;
    options.no_resolve = Tristate::True;
    options.allow_js = Tristate::True;
    options.check_js = Tristate::True;
    let config = tsrs_tsoptions::new_parsed_command_line(
        P::new(options),
        vec![file_name.clone()],
        Vec::new(),
        tspath::ComparePathsOptions::default(),
    );
    let mut program_options = ProgramOptions::new(P::new(config), host);
    program_options.skip_module_resolution = true;
    program_options.single_threaded = Tristate::True;
    let program = tsrs_compiler::new_program(program_options);

    let source_file = program
        .get_source_file(&file_name)
        .ok_or_else(|| format!("tsrs did not load {file_name}"))?;
    let context = Context::background();
    let syntactic_diagnostics = program.get_syntactic_diagnostics(&context, Some(source_file));
    let semantic_diagnostics = program.get_semantic_diagnostics(&context, Some(source_file));
    let query = if let Some(selector) = selector {
        let (target, (start_offset, end_offset)) = match selector {
            ExpressionSelector::OxcSpan((start, end)) => {
                let start_offset = start as u32;
                let end_offset = end as u32;
                let mut node = tsrs_ast::get_node_at_position(source_file, start_offset, false);
                let target = loop {
                    let node_end = node.end().expect_source("parsed node has a source end");
                    if node.kind() == Kind::BinaryExpression
                        && node_end == end_offset
                        && node
                            .pos()
                            .expect_source("parsed node has a source position")
                            <= start_offset
                    {
                        break node;
                    }
                    node = node
                        .parent()
                        .ok_or("could not match Oxc expression span to a tsrs AST node")?;
                };
                (target, (start_offset, end_offset))
            }
            ExpressionSelector::TsrsSource(expression_text) => {
                let mut matches = Vec::new();
                find_tsrs_expression(source_file.as_node(), source, expression_text, &mut matches);
                let [target] = matches.as_slice() else {
                    return Err(format!(
                        "tsrs-only source selection found {} matching binary expressions for {expression_text:?}",
                        matches.len()
                    )
                    .into());
                };
                let end_offset = target.end().expect_source("parsed node has a source end");
                let start_offset = end_offset - expression_text.len() as u32;
                if source.get(start_offset as usize..end_offset as usize) != Some(expression_text) {
                    return Err(format!(
                        "tsrs-only expression text did not match its source span: {expression_text:?}"
                    )
                    .into());
                }
                (*target, (start_offset, end_offset))
            }
        };
        let mut checker = program.get_type_checker_for_file(&context, source_file);
        let ty = checker.get_type_at_location(target);
        let type_name = checker.type_to_string_exported(ty);
        Some((type_name, (start_offset, end_offset)))
    } else {
        None
    };
    let unexpected = fs.unexpected.lock().unwrap().clone();
    if let Some(unexpected) = unexpected {
        return Err(unexpected.into());
    }
    Ok((
        syntactic_diagnostics.len(),
        semantic_diagnostics.len(),
        query.as_ref().map(|(type_name, _)| type_name.clone()),
        query.map(|(_, range)| range),
    ))
}

fn read_fixture(root: &Path, name: &str) -> Result<String, Box<dyn Error>> {
    Ok(fs::read_to_string(root.join("fixtures").join(name))?)
}

fn run() -> Result<(), Box<dyn Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let valid_js = read_fixture(&root, "valid.js")?;
    let valid_ts = read_fixture(&root, "valid.ts")?;
    let invalid_js = read_fixture(&root, "invalid.js")?;
    let invalid_ts = read_fixture(&root, "invalid.ts")?;

    let js_span = oxc_binary_expression_span(&valid_js, SourceType::mjs())?;
    let ts_span = oxc_binary_expression_span(&valid_ts, SourceType::ts().with_module(true))?;
    let invalid_ts_span =
        oxc_binary_expression_span(&invalid_ts, SourceType::ts().with_module(true))?;
    let invalid_js_oxc_syntax = oxc_parse_diagnostic_count(&invalid_js, SourceType::mjs());
    let invalid_js_expression_start = invalid_js
        .find("40 + 2")
        .ok_or("invalid.js fixture is missing its expected expression")?
        as u32;

    assert_eq!(
        &valid_js[js_span.0..js_span.1],
        "seed + 2",
        "Oxc JS source span must select the intended expression"
    );
    assert_eq!(
        &valid_ts[ts_span.0..ts_span.1],
        "seed + 2",
        "Oxc TS source span must select the intended expression"
    );

    let (valid_js_syntax, valid_js_semantic, valid_js_type, valid_js_range) =
        check_with_tsrs(&valid_js, "js", Some(ExpressionSelector::OxcSpan(js_span)))?;
    let (valid_ts_syntax, valid_ts_semantic, valid_ts_type, valid_ts_range) =
        check_with_tsrs(&valid_ts, "ts", Some(ExpressionSelector::OxcSpan(ts_span)))?;
    let (invalid_ts_syntax, invalid_ts_semantic, invalid_ts_type, invalid_ts_range) =
        check_with_tsrs(
            &invalid_ts,
            "ts",
            Some(ExpressionSelector::OxcSpan(invalid_ts_span)),
        )?;
    let (_, _, invalid_ts_tsrs_type, invalid_ts_tsrs_range) = check_with_tsrs(
        &invalid_ts,
        "ts",
        Some(ExpressionSelector::TsrsSource("40 + 2")),
    )?;
    let (invalid_js_syntax, invalid_js_semantic, invalid_js_type, invalid_js_range) =
        check_with_tsrs(
            &invalid_js,
            "js",
            Some(ExpressionSelector::TsrsSource("40 + 2")),
        )?;
    let (valid_js_tsrs_syntax, valid_js_tsrs_semantic, valid_js_tsrs_type, valid_js_tsrs_range) =
        check_with_tsrs(
            &valid_js,
            "js",
            Some(ExpressionSelector::TsrsSource("seed + 2")),
        )?;
    let (valid_ts_tsrs_syntax, valid_ts_tsrs_semantic, valid_ts_tsrs_type, valid_ts_tsrs_range) =
        check_with_tsrs(
            &valid_ts,
            "ts",
            Some(ExpressionSelector::TsrsSource("seed + 2")),
        )?;

    assert_eq!(valid_js_type.as_deref(), Some("number"));
    assert_eq!(valid_ts_type.as_deref(), Some("number"));
    assert_eq!(invalid_ts_type.as_deref(), Some("number"));
    assert_eq!(valid_js_tsrs_type, valid_js_type);
    assert_eq!(valid_ts_tsrs_type, valid_ts_type);
    assert_eq!(valid_js_tsrs_range, valid_js_range);
    assert_eq!(valid_ts_tsrs_range, valid_ts_range);
    assert_eq!(invalid_ts_tsrs_type, invalid_ts_type);
    assert_eq!(invalid_ts_tsrs_range, invalid_ts_range);
    assert_eq!(invalid_js_type.as_deref(), Some("number"));
    assert_eq!(
        invalid_js_range,
        Some((invalid_js_expression_start, invalid_js_expression_start + 6))
    );
    assert_eq!(valid_js_tsrs_syntax, valid_js_syntax);
    assert_eq!(valid_js_tsrs_semantic, valid_js_semantic);
    assert_eq!(valid_ts_tsrs_syntax, valid_ts_syntax);
    assert_eq!(valid_ts_tsrs_semantic, valid_ts_semantic);
    assert!(
        invalid_js_syntax > 0,
        "JS fixture should reject TS annotation syntax"
    );
    assert!(
        invalid_js_oxc_syntax > 0,
        "Oxc JS parser should reject TS annotation syntax"
    );
    assert!(
        invalid_ts_semantic > valid_ts_semantic,
        "tsrs should report an additional type error for the invalid TS fixture"
    );

    println!(
        "valid.js: tsrs syntax diagnostics={valid_js_syntax}, semantic diagnostics={valid_js_semantic}, Oxc+tsrs type={:?} span={:?}, tsrs-only type={:?} span={:?}",
        valid_js_type, js_span, valid_js_tsrs_type, valid_js_tsrs_range
    );
    println!(
        "valid.ts: tsrs syntax diagnostics={valid_ts_syntax}, semantic diagnostics={valid_ts_semantic}, Oxc+tsrs type={:?} span={:?}, tsrs-only type={:?} span={:?}",
        valid_ts_type, ts_span, valid_ts_tsrs_type, valid_ts_tsrs_range
    );
    println!(
        "invalid.ts: tsrs syntax diagnostics={invalid_ts_syntax}, semantic diagnostics={invalid_ts_semantic}, Oxc+tsrs type={:?} span={:?}, tsrs-only type={:?} span={:?}",
        invalid_ts_type, invalid_ts_span, invalid_ts_tsrs_type, invalid_ts_tsrs_range
    );
    println!(
        "invalid.js: Oxc syntax diagnostics={invalid_js_oxc_syntax}, tsrs syntax diagnostics={invalid_js_syntax}, semantic diagnostics={invalid_js_semantic}, tsrs-only expression type={:?}, byte span={:?}; TS annotation rejected",
        invalid_js_type, invalid_js_range
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    run()
}
