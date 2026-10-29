//! A1-only host bridge: run the *existing* Markdown guest and private WIT inspector
//! with identical in-memory bytes under granted/denied profiles. No filesystem
//! capability is exposed to the guest; only the trusted CLI reads its input.
#[path = "../tests/fixtures/document-outline/embedding.rs"]
mod embedding;

use std::fmt::Write as _;
use std::io::Read;
use wasmtime::component::{Component, HasSelf, Linker, Resource};
use wasmtime::{Config, Engine, Store};

wasmtime::component::bindgen!({
    path: "tests/fixtures/document-outline",
    world: "inspector",
    with: { "grunkalunka:markdown/documents.document": BackingDocument },
    imports: { default: trappable },
});
use grunkalunka::markdown::documents::ReadError;

pub struct BackingDocument;
const DOCUMENT_REP: u32 = 7;
const MAX_INPUT_BYTES: usize = 4096;

struct Host {
    allowed: bool,
    text: String,
    attempts: u32,
    reads: u32,
}

impl grunkalunka::markdown::documents::Host for Host {}
impl grunkalunka::markdown::documents::HostDocument for Host {
    fn read(
        &mut self,
        document: Resource<Document>,
    ) -> wasmtime::Result<Result<String, ReadError>> {
        self.attempts += 1;
        if !self.allowed {
            return Ok(Err(ReadError::Denied));
        }
        if document.rep() != DOCUMENT_REP {
            return Ok(Err(ReadError::UnknownGrant));
        }
        if self.text.len() > MAX_INPUT_BYTES {
            return Ok(Err(ReadError::TooLarge));
        }
        self.reads += 1;
        Ok(Ok(self.text.clone()))
    }

    fn drop(&mut self, _: Resource<Document>) -> wasmtime::Result<()> {
        wasmtime::bail!("the A1 host only borrows documents")
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut result, "{byte:02x}").unwrap();
    }
    result
}

fn inspect_profile(
    engine: &Engine,
    component: &Component,
    text: &str,
    allowed: bool,
) -> wasmtime::Result<String> {
    let mut linker = Linker::new(engine);
    Inspector::add_to_linker::<_, HasSelf<_>>(&mut linker, |host| host)?;
    let mut store = Store::new(
        engine,
        Host {
            allowed,
            text: text.into(),
            attempts: 0,
            reads: 0,
        },
    );
    store.set_fuel(50_000_000)?;
    let inspector = Inspector::instantiate(&mut store, component, &linker)?;
    let result = inspector.call_inspect(&mut store, Resource::new_borrow(DOCUMENT_REP))?;
    let cleanups = inspector.call_cleanup_count(&mut store)?;
    let host = store.data();
    let expected_reads = u32::from(allowed);
    if host.attempts != 1 || host.reads != expected_reads || cleanups != 1 {
        wasmtime::bail!("unexpected document capability attempts, reads or cleanup");
    }

    let profile = if allowed { "granted" } else { "denied" };
    let mut output = format!(
        r#"{{"profile":"{profile}","attempts":{},"backingReads":{},"cleanups":{cleanups},"headings":["#,
        host.attempts, host.reads
    );
    match (allowed, result) {
        (false, Err(InspectError::Denied)) => {}
        (false, _) => wasmtime::bail!("denied profile failed to return the typed denial"),
        (true, Err(error)) => wasmtime::bail!("granted inspection failed: {error:?}"),
        (true, Ok(headings)) => {
            for (index, heading) in headings.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                let parent = heading.parent.map_or("null".into(), |p| p.to_string());
                write!(
                    &mut output,
                    r#"{{"level":{},"start":{},"end":{},"sectionEnd":{},"parent":{},"textHex":"{}"}}"#,
                    heading.level,
                    heading.start,
                    heading.end,
                    heading.section_end,
                    parent,
                    hex(heading.text.as_bytes())
                )?;
            }
        }
    }
    output.push(']');
    if allowed {
        // The trusted renderer needs the unchanged source for byte-range checks.
        // Never print source bytes in the denied profile.
        write!(&mut output, r#","sourceHex":"{}""#, hex(text.as_bytes()))?;
    }
    output.push('}');
    Ok(output)
}

fn run() -> wasmtime::Result<()> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if !matches!(args.len(), 1 | 3) {
        wasmtime::bail!("usage: a1_note_preview GUEST.wasm [--profile granted|denied]");
    }
    let selected = if args.len() == 3 {
        if args[1].to_str() != Some("--profile") {
            wasmtime::bail!("expected --profile granted|denied");
        }
        match args[2].to_str() {
            Some("granted") => Some(true),
            Some("denied") => Some(false),
            _ => wasmtime::bail!("unknown document profile"),
        }
    } else {
        None
    };

    // In a single-profile denied run, do not even read stdin. This property
    // is stronger than just recording zero protected host backing reads.
    // The original dual-profile test-only invocation remains supported.
    let mut input = Vec::new();
    if selected != Some(false) {
        std::io::stdin()
            .take((MAX_INPUT_BYTES + 1) as u64)
            .read_to_end(&mut input)?;
        if input.len() > MAX_INPUT_BYTES {
            wasmtime::bail!("note input exceeds Markdown guest quota");
        }
    }
    let text = String::from_utf8(input)?;

    let guest = std::fs::read(&args[0])?;
    let template = include_str!("../tests/fixtures/document-outline/inspector.wat.in");
    let component_bytes = embedding::embed_guest(template, &guest)?;
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let component = Component::from_binary(&engine, &component_bytes)?;

    // The original integration test compares the *same* component and note
    // under two separate Stores. A selected application invocation creates
    // only its authorized profile, never an extra granted Store for denial.
    let mut profiles = Vec::new();
    if selected != Some(true) {
        profiles.push(inspect_profile(&engine, &component, &text, false)?);
    }
    if selected != Some(false) {
        profiles.push(inspect_profile(&engine, &component, &text, true)?);
    }
    println!(
        r#"{{"schemaVersion":1,"profiles":[{}]}}"#,
        profiles.join(",")
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("A1 Markdown/WIT host: {error:#}");
        std::process::exit(1);
    }
}
