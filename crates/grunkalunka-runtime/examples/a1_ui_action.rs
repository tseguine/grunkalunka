//! Private A1 action host: source-compiled component calls mutate staged Rust
//! view state. Only a completed action publishes that state. No document, path,
//! filesystem or network capability is linked into the guest.
//!
//! This example accepts locally generated components, not arbitrary untrusted
//! modules. The private action profile admits only expanded = 0 or 1, at most
//! 64 authorized headings, and a finite source result (which is not view state).

use grunkalunka_compiler::compile_ui_import;
use std::io::{Read, Write};
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Config, Engine, Store, StoreLimits, StoreLimitsBuilder};

wasmtime::component::bindgen!({
    path: "tests/fixtures/a1-ui",
    world: "outline",
    imports: { default: trappable },
});

const MAX_SOURCE_BYTES: usize = 64 * 1024;
const MAX_COMPONENT_BYTES: usize = 4 * 1024 * 1024;
const MAX_CALLS: u32 = 256;
const FUEL: u64 = 100_000;
const USAGE: &str =
    "usage: a1_ui_action --compile js|ts | --run|--native granted|denied COUNT EXPANDED";

#[derive(Clone, Copy, Debug)]
enum ActionError {
    Denied,
    InvalidCount,
    InvalidExpanded,
    CallLimit,
}

impl ActionError {
    fn label(self) -> &'static str {
        match self {
            Self::Denied => "denied",
            Self::InvalidCount => "invalid-count",
            Self::InvalidExpanded => "invalid-expanded",
            Self::CallLimit => "call-limit",
        }
    }
}

impl std::fmt::Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "A1 UI policy: {}", self.label())
    }
}

impl std::error::Error for ActionError {}

struct Host {
    allowed: bool,
    permitted_count: u32,
    attempts: u32,
    transitions: u32,
    visible: u32,
    limits: StoreLimits,
}

impl Host {
    fn new(allowed: bool, permitted_count: u32) -> Self {
        Self {
            allowed,
            permitted_count,
            attempts: 0,
            transitions: 0,
            visible: 0,
            // Installed before instantiation, including any guest start code.
            limits: StoreLimitsBuilder::new()
                .memory_size(4 * 1024 * 1024)
                .build(),
        }
    }

    // Both the WIT provider and the native Rust control use this action.
    // State is staged in this invocation's Host until receipt() commits it.
    fn select(&mut self, expanded: f64, count: f64) -> wasmtime::Result<f64> {
        self.attempts += 1;
        if self.attempts > MAX_CALLS {
            wasmtime::bail!(ActionError::CallLimit);
        }
        if !self.allowed {
            wasmtime::bail!(ActionError::Denied);
        }
        if !count.is_finite() || count != f64::from(self.permitted_count) {
            wasmtime::bail!(ActionError::InvalidCount);
        }
        if expanded != 0.0 && expanded != 1.0 {
            wasmtime::bail!(ActionError::InvalidExpanded);
        }
        self.visible = if expanded == 1.0 {
            self.permitted_count
        } else {
            0
        };
        self.transitions += 1;
        Ok(f64::from(self.visible))
    }
}

impl grunkalunka::a1::ui::Host for Host {
    fn select_visible(&mut self, expanded: f64, count: f64) -> wasmtime::Result<f64> {
        self.select(expanded, count)
    }
}

fn read_input(limit: usize) -> wasmtime::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .lock()
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        wasmtime::bail!("A1 input exceeds {limit} bytes");
    }
    Ok(bytes)
}

fn compile(typescript: bool) -> wasmtime::Result<()> {
    let source = String::from_utf8(read_input(MAX_SOURCE_BYTES)?)?;
    let artifact = compile_ui_import(&source, typescript).map_err(wasmtime::Error::msg)?;
    let fields = artifact
        .wat
        .strip_prefix("(module")
        .ok_or_else(|| wasmtime::Error::msg("compiler output is not a core module"))?;
    let core = format!("(core module{fields}");
    let template = include_str!("../tests/fixtures/a1-ui/outline.wat.in");
    if template.matches("{{COMPILED_MODULE}}").count() != 1 {
        wasmtime::bail!("invalid A1 component template");
    }
    let bytes = wat::parse_str(template.replace("{{COMPILED_MODULE}}", &core))?;
    if bytes.len() > MAX_COMPONENT_BYTES {
        wasmtime::bail!("compiled A1 component exceeds artifact quota");
    }
    std::io::stdout().lock().write_all(&bytes)?;
    Ok(())
}

fn receipt(host: &Host, outcome: wasmtime::Result<f64>) -> wasmtime::Result<String> {
    let (status, result) = match outcome {
        Ok(value) if value.is_finite() => ("ok", Some(value)),
        Ok(_) => ("invalid-result", None),
        Err(error) => match error.downcast_ref::<ActionError>() {
            Some(reason) => (reason.label(), None),
            None => return Err(error),
        },
    };
    // Failure discards every staged mutation, including successful earlier
    // calls in this action. Attempts remain evidence; no view state commits.
    let (transitions, visible) = if result.is_some() {
        (host.transitions, host.visible)
    } else {
        (0, 0)
    };
    let result = result.map_or_else(|| "null".into(), |value| value.to_string());
    let profile = if host.allowed { "granted" } else { "denied" };
    Ok(format!(
        r#"{{"schemaVersion":1,"profile":"{profile}","permittedCount":{},"attempts":{},"transitions":{transitions},"visible":{visible},"result":{result},"status":"{status}"}}"#,
        host.permitted_count, host.attempts
    ))
}

fn execute(allowed: bool, count: u32, expanded: f64) -> wasmtime::Result<String> {
    let bytes = read_input(MAX_COMPONENT_BYTES)?;
    let mut config = Config::new();
    config
        .consume_fuel(true)
        .collector(wasmtime::Collector::Copying);
    let engine = Engine::new(&config)?;
    let component = Component::from_binary(&engine, &bytes)?;
    let mut linker = Linker::new(&engine);
    Outline::add_to_linker::<_, HasSelf<_>>(&mut linker, |host| host)?;
    let mut store = Store::new(&engine, Host::new(allowed, count));
    store.limiter(|host| &mut host.limits);
    store.set_fuel(FUEL)?;
    let instance = Outline::instantiate(&mut store, &component, &linker)?;
    if store.data().attempts != 0 {
        wasmtime::bail!("A1 service called during component instantiation");
    }
    // Pinned Wasmtime 50 generated calls complete canonical post-return.
    // No manual deprecated post_return call is needed (docs/capabilities.md).
    let result = instance.call_run(&mut store, expanded, f64::from(count));
    receipt(store.data(), result)
}

fn run() -> wasmtime::Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [mode, language] if mode == "--compile" => match language.as_str() {
            "js" => compile(false),
            "ts" => compile(true),
            _ => wasmtime::bail!(USAGE),
        },
        [mode, profile, count, expanded] if matches!(mode.as_str(), "--run" | "--native") => {
            let allowed = match profile.as_str() {
                "granted" => true,
                "denied" => false,
                _ => wasmtime::bail!(USAGE),
            };
            // Canonical bounded numbers make the private CLI protocol unambiguous.
            let count: u32 = count
                .parse()
                .ok()
                .filter(|value: &u32| *value <= 64 && value.to_string() == count.as_str())
                .ok_or_else(|| wasmtime::Error::msg("COUNT must be an integer from 0 to 64"))?;
            let expanded = match expanded.as_str() {
                "0" => 0.0,
                "1" => 1.0,
                _ => wasmtime::bail!("EXPANDED must be 0 or 1"),
            };
            let output = if mode == "--native" {
                // Native control never reads stdin and shares the exact service.
                let mut host = Host::new(allowed, count);
                let result = host.select(expanded, f64::from(count));
                receipt(&host, result)?
            } else {
                execute(allowed, count, expanded)?
            };
            println!("{output}");
            Ok(())
        }
        _ => wasmtime::bail!(USAGE),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("A1 UI action: {error:#}");
        std::process::exit(1);
    }
}
