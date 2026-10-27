//! Bounded Oxc frontend phase observation; not a Grunkalunka compiler benchmark.
use std::collections::BTreeSet;
use std::hint::black_box;
use std::time::Instant;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Controls {
    samples: usize,
    iterations: usize,
    warmup: usize,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            samples: 7,
            iterations: 100,
            warmup: 20,
        }
    }
}

impl Controls {
    fn validate(self) -> Result<(), String> {
        for (name, value, min, max) in [
            ("samples", self.samples, 2, 12),
            ("iterations", self.iterations, 1, 500),
            ("warmup", self.warmup, 0, 100),
        ] {
            if !(min..=max).contains(&value) {
                return Err(format!("{name} must be within {min}..={max}"));
            }
        }
        Ok(())
    }
}

fn parse_options(args: &[String]) -> Result<Controls, String> {
    if !args.len().is_multiple_of(2) {
        return Err("expected unique --option value pairs".into());
    }
    let mut result = Controls::default();
    let mut seen = BTreeSet::new();
    for pair in args.as_chunks::<2>().0 {
        let name = pair[0].as_str();
        if !seen.insert(name) {
            return Err(format!("duplicate option {name}"));
        }
        let destination = match name {
            "--samples" => &mut result.samples,
            "--iterations" => &mut result.iterations,
            "--warmup" => &mut result.warmup,
            _ => return Err(format!("unknown option {name}")),
        };
        if !pair[1].bytes().all(|byte| byte.is_ascii_digit()) || pair[1].is_empty() {
            return Err(format!("invalid integer for {name}"));
        }
        *destination = pair[1]
            .parse::<usize>()
            .map_err(|_| format!("integer too large for {name}"))?;
    }
    result.validate()?;
    Ok(result)
}

#[derive(Clone, Copy)]
struct Fixture {
    name: &'static str,
    source: &'static str,
    typescript: bool,
    expected_symbols: usize,
    expected_references: usize,
}

const FIXTURES: [Fixture; 4] = [
    Fixture {
        name: "bindings.js",
        source: include_str!("../fixtures/bindings.js"),
        typescript: false,
        expected_symbols: 5,
        expected_references: 4,
    },
    Fixture {
        name: "bindings.ts",
        source: include_str!("../fixtures/bindings.ts"),
        typescript: true,
        expected_symbols: 5,
        expected_references: 4,
    },
    Fixture {
        name: "coercion.js",
        source: include_str!("../fixtures/coercion.js"),
        typescript: false,
        expected_symbols: 4,
        expected_references: 5,
    },
    Fixture {
        name: "coercion.ts",
        source: include_str!("../fixtures/coercion.ts"),
        typescript: true,
        expected_symbols: 4,
        expected_references: 5,
    },
];

fn kind(fixture: Fixture) -> SourceType {
    if fixture.typescript {
        SourceType::ts().with_module(true)
    } else {
        SourceType::mjs()
    }
}

fn parse_batch(fixture: Fixture, iterations: usize) -> (usize, usize) {
    let mut statements = 0;
    let mut errors = 0;
    for _ in 0..iterations {
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, fixture.source, kind(fixture)).parse();
        statements += black_box(parsed.program.body.len());
        errors += parsed.diagnostics.len() + usize::from(parsed.fatal_error);
    }
    (statements, errors)
}

fn ensure_parse(fixture: Fixture, result: (usize, usize), iterations: usize) -> Result<(), String> {
    if result != (iterations, 0) {
        return Err(format!(
            "{} changed parser admission during measurement",
            fixture.name
        ));
    }
    Ok(())
}

fn ensure_semantic(
    fixture: Fixture,
    result: (usize, usize, usize),
    iterations: usize,
) -> Result<(), String> {
    if result
        != (
            iterations * fixture.expected_symbols,
            iterations * fixture.expected_references,
            0,
        )
    {
        return Err(format!(
            "{} changed semantic bindings during measurement",
            fixture.name
        ));
    }
    Ok(())
}

fn timed<R>(body: impl FnOnce() -> R) -> (R, f64) {
    let started = Instant::now();
    let result = body();
    (result, started.elapsed().as_secs_f64() * 1000.0)
}

fn statistics(samples: &[f64]) -> String {
    let mut sorted = samples.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let middle = sorted.len() / 2;
    let median = if sorted.len().is_multiple_of(2) {
        sorted[middle - 1] / 2.0 + sorted[middle] / 2.0
    } else {
        sorted[middle]
    };
    let raw = samples
        .iter()
        .map(|value| format!("{value:.9}"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"samplesMs":[{raw}],"minMs":{:.9},"medianMs":{median:.9},"maxMs":{:.9}}}"#,
        sorted[0],
        sorted.last().unwrap()
    )
}

fn measure_fixture(fixture: Fixture, controls: Controls) -> Result<String, String> {
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, fixture.source, kind(fixture)).parse();
    if parsed.fatal_error || !parsed.diagnostics.is_empty() || parsed.program.body.len() != 1 {
        return Err(format!("{} parser golden mismatch", fixture.name));
    }
    let witness = SemanticBuilder::new_compiler()
        .with_build_nodes(true)
        .build(&parsed.program);
    if !witness.diagnostics.is_empty() {
        return Err(format!("{} semantic golden diagnostics", fixture.name));
    }
    let symbol_count = witness.semantic.scoping().symbol_ids().count();
    let references = witness
        .semantic
        .scoping()
        .symbol_ids()
        .map(|symbol| witness.semantic.symbol_references(symbol).count())
        .sum::<usize>();
    if symbol_count != fixture.expected_symbols || references != fixture.expected_references {
        return Err(format!("{} semantic golden mismatch", fixture.name));
    }

    // The semantic phase builds repeatedly from the same previously parsed AST.
    // Parsing is not part of its timed interval.
    let semantic_batch = |iterations: usize| {
        let mut symbols = 0;
        let mut references = 0;
        let mut diagnostics = 0;
        for _ in 0..iterations {
            let built = SemanticBuilder::new_compiler()
                .with_build_nodes(true)
                .build(&parsed.program);
            let semantic = &built.semantic;
            symbols += black_box(semantic.scoping().symbol_ids().count());
            references += black_box(
                semantic
                    .scoping()
                    .symbol_ids()
                    .map(|symbol| semantic.symbol_references(symbol).count())
                    .sum::<usize>(),
            );
            diagnostics += built.diagnostics.len();
        }
        (symbols, references, diagnostics)
    };

    ensure_parse(
        fixture,
        parse_batch(fixture, controls.warmup),
        controls.warmup,
    )?;
    ensure_semantic(fixture, semantic_batch(controls.warmup), controls.warmup)?;

    let mut parsing = Vec::with_capacity(controls.samples);
    let mut semantics = Vec::with_capacity(controls.samples);
    for index in 0..controls.samples {
        if index.is_multiple_of(2) {
            let (counts, ms) = timed(|| parse_batch(fixture, controls.iterations));
            ensure_parse(fixture, counts, controls.iterations)?;
            parsing.push(ms);
            let (counts, ms) = timed(|| semantic_batch(controls.iterations));
            ensure_semantic(fixture, counts, controls.iterations)?;
            semantics.push(ms);
        } else {
            let (counts, ms) = timed(|| semantic_batch(controls.iterations));
            ensure_semantic(fixture, counts, controls.iterations)?;
            semantics.push(ms);
            let (counts, ms) = timed(|| parse_batch(fixture, controls.iterations));
            ensure_parse(fixture, counts, controls.iterations)?;
            parsing.push(ms);
        }
    }
    Ok(format!(
        r#"{{"name":"{}","language":"{}","sourceBytes":{},"symbols":{},"references":{},"parseWithAllocator":{},"semanticFromReusedAst":{}}}"#,
        fixture.name,
        if fixture.typescript { "ts" } else { "js" },
        fixture.source.len(),
        fixture.expected_symbols,
        fixture.expected_references,
        statistics(&parsing),
        statistics(&semantics)
    ))
}

fn measure(controls: Controls) -> Result<String, String> {
    controls.validate()?;
    let fixtures = FIXTURES
        .iter()
        .copied()
        .map(|fixture| measure_fixture(fixture, controls))
        .collect::<Result<Vec<_>, _>>()?
        .join(",");
    Ok(format!(
        r#"{{"schemaVersion":1,"workload":"oxc-standalone-frontend-phases","oxcVersion":"0.155.0","os":"{}","arch":"{}","config":{{"samples":{},"iterations":{},"warmup":{}}},"unit":"milliseconds per batch","order":"alternate parse/semantic","fixtures":[{fixtures}]}}"#,
        std::env::consts::OS,
        std::env::consts::ARCH,
        controls.samples,
        controls.iterations,
        controls.warmup
    ))
}

pub fn run_cost_mode(args: &[String]) -> Result<String, String> {
    measure(parse_options(args)?)
}

// The existing canonical spike invocation exercises this bounded correctness
// witness; it does not retain or compare any machine's timing values.
pub fn smoke() -> Result<(), String> {
    let controls = Controls {
        samples: 2,
        iterations: 2,
        warmup: 1,
    };
    let report = measure(controls)?;
    if !report.contains(r#""schemaVersion":1"#)
        || report.matches(r#""parseWithAllocator":"#).count() != FIXTURES.len()
        || report.matches(r#""semanticFromReusedAst":"#).count() != FIXTURES.len()
    {
        return Err("frontend cost receipt missing required phases".into());
    }
    for invalid in [
        vec!["--samples".to_owned()],
        vec!["--samples".to_owned(), "1".to_owned()],
        vec![
            "--samples".to_owned(),
            "2".to_owned(),
            "--samples".to_owned(),
            "3".to_owned(),
        ],
        vec!["--iterations".to_owned(), "501".to_owned()],
        vec!["--warmup".to_owned(), "-1".to_owned()],
        vec!["--unrecognized".to_owned(), "1".to_owned()],
    ] {
        if parse_options(&invalid).is_ok() {
            return Err(format!("accepted invalid cost controls: {invalid:?}"));
        }
    }
    Ok(())
}
