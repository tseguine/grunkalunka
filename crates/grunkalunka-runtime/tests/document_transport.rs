//! Private canonical string/result transport, before a document-processing guest.
use wasmtime::component::{Component, HasSelf, Linker, Resource};
use wasmtime::{Config, Engine, Store};

wasmtime::component::bindgen!({
    path: "tests/fixtures/document-transport",
    world: "reader",
    with: { "grunkalunka:documents/documents.document": BackingDocument },
    imports: { default: trappable },
});

pub struct BackingDocument;
const LIMIT: usize = 4096;

#[derive(Default)]
struct Host {
    allowed: bool,
    text: String,
    attempts: usize,
    reads: usize,
    fault: bool,
}

impl grunkalunka::documents::documents::Host for Host {}

impl grunkalunka::documents::documents::HostDocument for Host {
    fn read(
        &mut self,
        document: Resource<Document>,
    ) -> wasmtime::Result<Result<String, ReadError>> {
        self.attempts += 1;
        if !self.allowed {
            return Ok(Err(ReadError::Denied));
        }
        if document.rep() != 7 {
            return Ok(Err(ReadError::UnknownGrant));
        }
        if self.fault {
            wasmtime::bail!("injected backing failure");
        }
        if self.text.len() > LIMIT {
            return Ok(Err(ReadError::TooLarge));
        }
        self.reads += 1;
        Ok(Ok(self.text.clone()))
    }

    fn drop(&mut self, _: Resource<Document>) -> wasmtime::Result<()> {
        wasmtime::bail!("this fixture never transfers owned documents")
    }
}

fn setup() -> (Store<Host>, Reader) {
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config).unwrap();
    let bytes = wat::parse_str(include_str!("fixtures/document-transport/reader.wat")).unwrap();
    let component = Component::from_binary(&engine, &bytes).unwrap();
    let mut linker = Linker::new(&engine);
    Reader::add_to_linker::<_, HasSelf<_>>(&mut linker, |host| host).unwrap();
    let mut store = Store::new(&engine, Host::default());
    store.set_fuel(100_000).unwrap();
    let reader = Reader::instantiate(&mut store, &component, &linker).unwrap();
    (store, reader)
}

#[test]
fn policy_errors_are_values_and_do_not_poison_the_same_instance() {
    let (mut store, reader) = setup();
    store.data_mut().text = "# Private\r\n猫😀".into();
    assert_eq!(
        reader.call_read(&mut store, Resource::new_borrow(7)).unwrap(),
        Err(ReadError::Denied)
    );
    assert_eq!((store.data().attempts, store.data().reads), (1, 0));
    assert_eq!(reader.call_cleanup_count(&mut store).unwrap(), 1);
    store.data_mut().allowed = true;
    assert_eq!(
        reader.call_read(&mut store, Resource::new_borrow(8)).unwrap(),
        Err(ReadError::UnknownGrant)
    );
    assert_eq!((store.data().attempts, store.data().reads), (2, 0));
    let first = reader
        .call_read(&mut store, Resource::new_borrow(7))
        .unwrap()
        .unwrap();
    assert_eq!(first, "# Private\r\n猫😀");
    assert_eq!(reader.call_cleanup_count(&mut store).unwrap(), 3);
    store.data_mut().text = "replacement".into();
    assert_eq!(
        reader.call_read(&mut store, Resource::new_borrow(7)).unwrap(),
        Ok("replacement".into())
    );
    assert_eq!(
        first, "# Private\r\n猫😀",
        "owned output survives cleanup and later calls"
    );
    assert_eq!(reader.call_cleanup_count(&mut store).unwrap(), 4);
}

#[test]
fn exact_utf8_content_and_byte_limits_survive_canonical_transport() {
    let (mut store, reader) = setup();
    store.data_mut().allowed = true;
    let values = [
        String::new(),
        "\u{feff}é\0\r\n😀e\u{301}\r\u{2028}".into(),
        "é".repeat(LIMIT / 2),
    ];
    for (index, expected) in values.iter().enumerate() {
        store.data_mut().text.clone_from(expected);
        assert_eq!(
            reader.call_read(&mut store, Resource::new_borrow(7)).unwrap(),
            Ok(expected.clone())
        );
        assert_eq!(
            reader.call_cleanup_count(&mut store).unwrap(),
            index as u32 + 1
        );
    }
    store.data_mut().text = format!("{}x", values[2]);
    assert_eq!(
        reader.call_read(&mut store, Resource::new_borrow(7)).unwrap(),
        Err(ReadError::TooLarge)
    );
    assert_eq!((store.data().attempts, store.data().reads), (4, 3));
    assert_eq!(reader.call_cleanup_count(&mut store).unwrap(), 4);
    store.data_mut().text = "after limit".into();
    assert_eq!(
        reader.call_read(&mut store, Resource::new_borrow(7)).unwrap(),
        Ok("after limit".into())
    );
    assert_eq!(reader.call_cleanup_count(&mut store).unwrap(), 5);
}

#[test]
fn host_faults_remain_traps_not_fabricated_document_results() {
    let (mut store, reader) = setup();
    store.data_mut().allowed = true;
    store.data_mut().fault = true;
    let error = reader
        .call_read(&mut store, Resource::new_borrow(7))
        .unwrap_err();
    assert!(format!("{error:#}").contains("injected backing failure"));
    assert_eq!((store.data().attempts, store.data().reads), (1, 0));
    // Discard the trapping instance; ordinary result errors above are different.
}
