//! Native integration driver for a private capability-backed Markdown component.
use std::fmt::Write as _;
use wasmtime::component::{Component, HasSelf, Linker, Resource};
use wasmtime::{Config, Engine, Store, Trap};

wasmtime::component::bindgen!({
    path: "tests/fixtures/document-outline",
    world: "inspector",
    with: { "grunkalunka:markdown/documents.document": BackingDocument },
    imports: { default: trappable },
});
use grunkalunka::markdown::documents::ReadError;

pub struct BackingDocument;

#[derive(Default)]
struct Host {
    allowed: bool,
    text: String,
    attempts: usize,
    reads: usize,
}

impl grunkalunka::markdown::documents::Host for Host {}
impl grunkalunka::markdown::documents::HostDocument for Host {
    fn read(&mut self, document: Resource<Document>) -> wasmtime::Result<Result<String, ReadError>> {
        self.attempts += 1;
        if !self.allowed {
            return Ok(Err(ReadError::Denied));
        }
        if document.rep() != 7 {
            return Ok(Err(ReadError::UnknownGrant));
        }
        if self.text.len() > 4096 {
            return Ok(Err(ReadError::TooLarge));
        }
        self.reads += 1;
        Ok(Ok(self.text.clone()))
    }

    fn drop(&mut self, _: Resource<Document>) -> wasmtime::Result<()> {
        wasmtime::bail!("the outline fixture never transfers owned documents")
    }
}

fn instantiate(engine: &Engine, component: &Component) -> (Store<Host>, Inspector) {
    let mut linker = Linker::new(engine);
    Inspector::add_to_linker::<_, HasSelf<_>>(&mut linker, |host| host).unwrap();
    let mut store = Store::new(engine, Host::default());
    store.set_fuel(50_000_000).unwrap();
    let inspector = Inspector::instantiate(&mut store, component, &linker).unwrap();
    (store, inspector)
}

fn inspect(
    store: &mut Store<Host>,
    inspector: &Inspector,
    rep: u32,
) -> Result<Vec<Heading>, InspectError> {
    store.set_fuel(50_000_000).unwrap();
    let before = inspector.call_cleanup_count(&mut *store).unwrap();
    let result = inspector
        .call_inspect(&mut *store, Resource::new_borrow(rep))
        .unwrap();
    assert_eq!(inspector.call_cleanup_count(&mut *store).unwrap(), before + 1);
    result
}

fn rows(headings: &[Heading]) -> String {
    let mut result = String::new();
    for heading in headings {
        write!(
            result,
            "{} {} {} {} {} {} ",
            heading.level,
            heading.start,
            heading.end,
            heading.section_end,
            heading.parent.map_or(-1, i64::from),
            heading.text.len()
        )
        .unwrap();
        for byte in heading.text.bytes() {
            write!(result, "{byte:02x}").unwrap();
        }
        result.push('\n');
    }
    result
}

fn hex(bytes: &[u8]) -> String {
    let mut result = String::new();
    for byte in bytes {
        write!(result, "{byte:02x}").unwrap();
    }
    result
}

fn main() -> wasmtime::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        wasmtime::bail!("expected outline_guest.wasm path");
    }
    let guest = std::fs::read(&args[0])?;
    if !guest.starts_with(b"\0asm\x01\0\0\0") {
        wasmtime::bail!("expected a core module");
    }
    let mut binary = String::new();
    for byte in &guest {
        write!(binary, "\\{byte:02x}")?;
    }
    let template = include_str!("../tests/fixtures/document-outline/inspector.wat.in");
    assert_eq!(template.matches("{{GUEST_MODULE}}").count(), 1);
    let component_bytes = wat::parse_str(template.replace(
        "{{GUEST_MODULE}}",
        &format!("(core module $guest binary \"{binary}\")"),
    ))?;
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let component = Component::from_binary(&engine, &component_bytes)?;
    let mut absent = Store::new(&engine, ());
    absent.set_fuel(50_000_000)?;
    let error = Inspector::instantiate(&mut absent, &component, &Linker::new(&engine))
        .err()
        .expect("document import must be explicitly linked");
    assert!(format!("{error:#}").contains("grunkalunka:markdown/documents"));

    let (mut store, inspector) = instantiate(&engine, &component);
    store.data_mut().text = "# Private".into();
    assert_eq!(
        inspect(&mut store, &inspector, 7).unwrap_err(),
        InspectError::Denied
    );
    assert_eq!((store.data().attempts, store.data().reads), (1, 0));
    store.data_mut().allowed = true;
    assert_eq!(
        inspect(&mut store, &inspector, 8).unwrap_err(),
        InspectError::UnknownGrant
    );
    assert_eq!((store.data().attempts, store.data().reads), (2, 0));

    let sources = [
        String::new(),
        "# Root\r\n### 子😀\n# Root\n".into(),
        "#\n\n##\n".into(),
        "> # Quote\r\n> text\r\n\r\n- ## Child\n".into(),
        "\u{feff}text\0\n# e\u{301} &amp; `code`\n".into(),
        "```md\n# literal\n```\n\nTitle\n=====\n".into(),
        "# Repeat\n".repeat(64),
        format!("# {}", "é".repeat(2047)),
    ];
    let mut held = Vec::new();
    for source in &sources {
        store.data_mut().text.clone_from(source);
        held.push(inspect(&mut store, &inspector, 7).unwrap());
    }
    assert_eq!(held[1].len(), 3);
    assert_eq!(held[1][1].parent, Some(0));
    assert_eq!(held[1][1].text, "子😀");
    assert_eq!((held[1][1].start, held[1][1].end), (8, 20));
    assert!(held[0].is_empty());
    let (mut sibling, other) = instantiate(&engine, &component);
    assert_eq!(
        inspect(&mut sibling, &other, 7).unwrap_err(),
        InspectError::Denied
    );
    assert_eq!(sibling.data().reads, 0);
    sibling.data_mut().allowed = true;
    sibling.data_mut().text = "# Separate".into();
    assert_eq!(inspect(&mut sibling, &other, 7).unwrap()[0].text, "Separate");
    for _ in 0..2 {
        for (source, expected) in sources.iter().zip(&held) {
            store.data_mut().text.clone_from(source);
            assert_eq!(
                rows(&inspect(&mut store, &inspector, 7).unwrap()),
                rows(expected)
            );
        }
    }
    store.data_mut().text = "x".repeat(4097);
    let reads = store.data().reads;
    assert_eq!(
        inspect(&mut store, &inspector, 7).unwrap_err(),
        InspectError::TooLarge
    );
    assert_eq!(store.data().reads, reads);
    store.data_mut().text = "# x\n".repeat(65);
    assert_eq!(
        inspect(&mut store, &inspector, 7).unwrap_err(),
        InspectError::OutlineTooLarge
    );
    let reads = store.data().reads;
    store.data_mut().allowed = false;
    assert_eq!(
        inspect(&mut store, &inspector, 7).unwrap_err(),
        InspectError::Denied
    );
    assert_eq!(store.data().reads, reads);
    store.data_mut().allowed = true;
    store.data_mut().text.clear();
    assert!(inspect(&mut store, &inspector, 7).unwrap().is_empty());

    store.set_fuel(1)?;
    let error = inspector
        .call_inspect(&mut store, Resource::new_borrow(7))
        .unwrap_err();
    assert_eq!(error.downcast_ref::<Trap>(), Some(&Trap::OutOfFuel));
    drop(store); // No post-trap cleanup or instance reuse is promised.
    let (mut fresh, second) = instantiate(&engine, &component);
    fresh.data_mut().allowed = true;
    fresh.data_mut().text = "# Independent\n".into();
    assert_eq!(
        inspect(&mut fresh, &second, 7).unwrap()[0].text,
        "Independent"
    );
    // Host-owned lists/strings remain intact after post-return, reuse and destruction.
    let mut output = String::new();
    for (source, result) in sources.iter().zip(&held) {
        writeln!(
            output,
            "{}\t{}",
            hex(source.as_bytes()),
            hex(rows(result).as_bytes())
        )?;
    }
    print!("{output}");
    Ok(())
}
