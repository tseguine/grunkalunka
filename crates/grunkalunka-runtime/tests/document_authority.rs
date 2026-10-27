//! Document grants and canonical guest indices are different authority boundaries.
use wasmtime::component::{Component, HasSelf, Linker, Resource};
use wasmtime::{Config, Engine, Store};

wasmtime::component::bindgen!({
    path: "tests/fixtures/document-transport",
    world: "reader",
    with: { "grunkalunka:documents/documents.document": BackingDocument },
    imports: { default: trappable },
});

#[path = "fixtures/document-authority/grants.rs"]
mod grants;
use grants::{Documents, Grant};
pub struct BackingDocument;
const READER: &str = include_str!("fixtures/document-transport/reader.wat");

impl grunkalunka::documents::documents::Host for Documents {}
impl grunkalunka::documents::documents::HostDocument for Documents {
    fn read(
        &mut self,
        document: Resource<Document>,
    ) -> wasmtime::Result<Result<String, ReadError>> {
        Ok(self.read(document.rep()))
    }

    fn drop(&mut self, _: Resource<Document>) -> wasmtime::Result<()> {
        wasmtime::bail!("this fixture never transfers owned documents")
    }
}

fn setup(wat: &str, documents: Documents) -> (Store<Documents>, Reader) {
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config).unwrap();
    let component = Component::from_binary(&engine, &wat::parse_str(wat).unwrap()).unwrap();
    let mut linker = Linker::new(&engine);
    Reader::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state).unwrap();
    let mut store = Store::new(&engine, documents);
    store.set_fuel(100_000).unwrap();
    let reader = Reader::instantiate(&mut store, &component, &linker).unwrap();
    (store, reader)
}

fn raw_read(
    store: &mut Store<Documents>,
    reader: &Reader,
    rep: u32,
) -> wasmtime::Result<Result<String, ReadError>> {
    store.set_fuel(100_000)?;
    let before = reader.call_cleanup_count(&mut *store)?;
    let result = reader.call_read(&mut *store, Resource::new_borrow(rep))?;
    assert_eq!(reader.call_cleanup_count(&mut *store)?, before + 1);
    Ok(result)
}

fn read(
    store: &mut Store<Documents>,
    reader: &Reader,
    grant: &Grant,
) -> wasmtime::Result<Result<String, ReadError>> {
    let Some(rep) = store.data().representation(grant) else {
        wasmtime::bail!("foreign document grant");
    };
    raw_read(store, reader, rep)
}

#[test]
fn backing_indices_are_not_grants_and_policy_denial_is_recoverable() {
    let text = "# Public\r\n猫😀\n[private](../private.md)";
    let mut documents = Documents::new(vec![text.into(), "# Private sentinel".into()]);
    let public = documents.issue(0).unwrap();
    let (mut store, reader) = setup(READER, documents);
    assert_eq!(
        read(&mut store, &reader, &public).unwrap(),
        Err(ReadError::Denied)
    );
    assert_eq!(store.data().attempts, 1);
    assert!(store.data().reads.is_empty());
    store.data_mut().allowed = true;
    // Deliberately bypass the host wrapper: a valid canonical borrow is not authority.
    assert_eq!(
        raw_read(&mut store, &reader, 1).unwrap(),
        Err(ReadError::UnknownGrant)
    );
    assert!(store.data().reads.is_empty());
    let held = read(&mut store, &reader, &public).unwrap().unwrap();
    assert_eq!(held, text);
    assert_eq!(store.data().reads, vec![0]);
    store.data_mut().allowed = false;
    assert_eq!(
        read(&mut store, &reader, &public).unwrap(),
        Err(ReadError::Denied)
    );
    store.data_mut().allowed = true;
    assert_eq!(read(&mut store, &reader, &public).unwrap().unwrap(), text);
    assert_eq!(store.data().reads, vec![0, 0]);
    assert_eq!(store.data().attempts, 5);
    drop(store);
    assert_eq!(held, text);
}

#[test]
fn revocation_does_not_retarget_aliases_or_invalidate_another_document() {
    let mut documents = Documents::new(vec!["A".into(), "B".into()]);
    documents.allowed = true;
    let a = documents.issue(0).unwrap();
    let alias = a.clone();
    let b = documents.issue(1).unwrap();
    let (mut store, reader) = setup(READER, documents);
    assert_eq!(read(&mut store, &reader, &a).unwrap().unwrap(), "A");
    assert_eq!(read(&mut store, &reader, &b).unwrap().unwrap(), "B");
    assert!(store.data_mut().revoke(&a));
    let replacement = store.data_mut().issue(0).unwrap();
    assert_ne!(
        store.data().representation(&a),
        store.data().representation(&replacement)
    );
    assert_eq!(
        read(&mut store, &reader, &replacement).unwrap().unwrap(),
        "A"
    );
    for old in [&a, &alias] {
        assert_eq!(
            read(&mut store, &reader, old).unwrap(),
            Err(ReadError::UnknownGrant)
        );
        assert_eq!(store.data().reads, vec![0, 1, 0]);
    }
    assert_eq!(read(&mut store, &reader, &b).unwrap().unwrap(), "B");
    assert_eq!(store.data().reads, vec![0, 1, 0, 1]);
    assert_eq!(store.data().attempts, 6);
}

#[test]
fn foreign_tokens_cannot_select_or_revoke_a_colliding_local_grant() {
    let mut first = Documents::new(vec!["first".into()]);
    let mut second = Documents::new(vec!["second".into()]);
    first.allowed = true;
    second.allowed = true;
    let a = first.issue(0).unwrap();
    let b = second.issue(0).unwrap();
    assert_eq!(first.representation(&a), second.representation(&b));
    let (mut first, first_reader) = setup(READER, first);
    let (mut second, second_reader) = setup(READER, second);
    assert_eq!(
        read(&mut first, &first_reader, &a).unwrap().unwrap(),
        "first"
    );
    assert_eq!(
        read(&mut second, &second_reader, &b).unwrap().unwrap(),
        "second"
    );
    let before = second_reader.call_cleanup_count(&mut second).unwrap();
    let error = read(&mut second, &second_reader, &a).unwrap_err();
    assert_eq!(error.to_string(), "foreign document grant");
    assert!(!second.data_mut().revoke(&a));
    assert_eq!(second.data().attempts, 1);
    assert_eq!(second.data().reads, vec![0]);
    assert_eq!(
        second_reader.call_cleanup_count(&mut second).unwrap(),
        before
    );
    assert_eq!(
        read(&mut second, &second_reader, &b).unwrap().unwrap(),
        "second"
    );
}

#[test]
fn a_guest_cannot_turn_another_host_grant_representation_into_a_borrow() {
    let make_documents = || {
        let mut documents = Documents::new(vec!["A".into(), "unpassed B".into()]);
        documents.allowed = true;
        let a = documents.issue(0).unwrap();
        let b = documents.issue(1).unwrap();
        (documents, a, b)
    };
    let (documents, a, b) = make_documents();
    let (mut control, reader) = setup(READER, documents);
    assert_eq!(read(&mut control, &reader, &a).unwrap().unwrap(), "A");
    assert_eq!(
        read(&mut control, &reader, &b).unwrap().unwrap(),
        "unpassed B"
    );
    assert_eq!(control.data().reads, vec![0, 1]);

    let (documents, a, b) = make_documents();
    let rep = documents.representation(&b).unwrap();
    let original = "local.get $document\n      i32.const 0\n      call $read";
    assert_eq!(READER.matches(original).count(), 1);
    let forged = READER.replacen(
        original,
        &format!("i32.const {rep}\n      i32.const 0\n      call $read"),
        1,
    );
    let (mut store, reader) = setup(&forged, documents);
    let error = read(&mut store, &reader, &a).unwrap_err();
    assert!(
        format!("{error:#}").contains("unknown handle index"),
        "{error:#}"
    );
    assert_eq!(store.data().attempts, 0);
    assert!(store.data().reads.is_empty());
    // No further invocation or cleanup/reuse guarantee after this expected trap.
}
