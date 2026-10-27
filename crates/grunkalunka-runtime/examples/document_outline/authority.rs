//! Apply the private multi-document grant fixture to the actual Markdown component.
#[path = "../../tests/fixtures/document-authority/grants.rs"]
mod grants;

use super::{Document, Heading, InspectError, Inspector, ReadError, embedding, rows};
use grants::{Documents, Grant};
use wasmtime::component::{Component, HasSelf, Linker, Resource};
use wasmtime::{Engine, Store};

impl super::grunkalunka::markdown::documents::Host for Documents {}
impl super::grunkalunka::markdown::documents::HostDocument for Documents {
    fn read(
        &mut self,
        document: Resource<Document>,
    ) -> wasmtime::Result<Result<String, ReadError>> {
        Ok(self.read(document.rep()))
    }

    fn drop(&mut self, _: Resource<Document>) -> wasmtime::Result<()> {
        wasmtime::bail!("the outline consumer only borrows documents")
    }
}

fn setup(
    engine: &Engine,
    component: &Component,
    documents: Documents,
) -> wasmtime::Result<(Store<Documents>, Inspector)> {
    let mut linker = Linker::new(engine);
    Inspector::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state)?;
    let mut store = Store::new(engine, documents);
    store.set_fuel(50_000_000)?;
    let inspector = Inspector::instantiate(&mut store, component, &linker)?;
    Ok((store, inspector))
}

fn raw_inspect(
    store: &mut Store<Documents>,
    inspector: &Inspector,
    rep: u32,
) -> wasmtime::Result<Result<Vec<Heading>, InspectError>> {
    store.set_fuel(50_000_000)?;
    let before = inspector.call_cleanup_count(&mut *store)?;
    let result = inspector.call_inspect(&mut *store, Resource::new_borrow(rep))?;
    assert_eq!(inspector.call_cleanup_count(&mut *store)?, before + 1);
    Ok(result)
}

fn inspect(
    store: &mut Store<Documents>,
    inspector: &Inspector,
    grant: &Grant,
) -> wasmtime::Result<Result<Vec<Heading>, InspectError>> {
    let Some(rep) = store.data().representation(grant) else {
        wasmtime::bail!("foreign document grant");
    };
    raw_inspect(store, inspector, rep)
}

pub fn verify(
    engine: &Engine,
    component: &Component,
    template: &str,
    guest: &[u8],
) -> wasmtime::Result<()> {
    let sources = vec![
        "# A\r\n## 子😀\n\n[private](../secret.md)\n".into(),
        "# Private\n".into(),
    ];
    let mut documents = Documents::new(sources.clone());
    let a = documents.issue(0).unwrap();
    let alias = a.clone();
    let (mut store, inspector) = setup(engine, component, documents)?;
    assert_eq!(
        inspect(&mut store, &inspector, &a)?.unwrap_err(),
        InspectError::Denied
    );
    store.data_mut().allowed = true;
    assert_eq!(
        raw_inspect(&mut store, &inspector, 1)?.unwrap_err(),
        InspectError::UnknownGrant
    );
    assert_eq!(store.data().attempts, 2);
    assert!(store.data().reads.is_empty());
    let held = inspect(&mut store, &inspector, &a)?.unwrap();
    assert_eq!(held.len(), 2);
    assert_eq!(held[0].text, "A");
    assert_eq!(held[1].text, "子😀");
    assert_eq!(held[1].parent, Some(0));
    assert_eq!((held[1].start, held[1].end), (5, 16));
    assert_eq!(store.data().reads, vec![0]);

    let b = store.data_mut().issue(1).unwrap();
    assert_eq!(
        inspect(&mut store, &inspector, &b)?.unwrap()[0].text,
        "Private"
    );
    assert!(store.data_mut().revoke(&a));
    let renewed = store.data_mut().issue(0).unwrap();
    for old in [&a, &alias] {
        assert_eq!(
            inspect(&mut store, &inspector, old)?.unwrap_err(),
            InspectError::UnknownGrant
        );
        assert_eq!(store.data().reads, vec![0, 1]);
    }
    assert_eq!(
        rows(&inspect(&mut store, &inspector, &renewed)?.unwrap()),
        rows(&held)
    );
    store.data_mut().allowed = false;
    assert_eq!(
        inspect(&mut store, &inspector, &b)?.unwrap_err(),
        InspectError::Denied
    );
    assert_eq!(store.data().reads, vec![0, 1, 0]);
    store.data_mut().allowed = true;
    assert_eq!(
        inspect(&mut store, &inspector, &b)?.unwrap()[0].text,
        "Private"
    );
    assert_eq!(store.data().attempts, 9);
    assert_eq!(store.data().reads, vec![0, 1, 0, 1]);

    let mut foreign = Documents::new(vec!["# Foreign\n".into()]);
    foreign.allowed = true;
    let own = foreign.issue(0).unwrap();
    assert_eq!(
        foreign.representation(&own),
        store.data().representation(&a)
    );
    let (mut foreign, other) = setup(engine, component, foreign)?;
    let error = inspect(&mut foreign, &other, &a).unwrap_err();
    assert_eq!(error.to_string(), "foreign document grant");
    assert!(!foreign.data_mut().revoke(&a));
    assert_eq!(foreign.data().attempts, 0);
    assert!(foreign.data().reads.is_empty());
    assert_eq!(other.call_cleanup_count(&mut foreign)?, 0);
    assert_eq!(
        inspect(&mut foreign, &other, &own)?.unwrap()[0].text,
        "Foreign"
    );

    // A fresh instance receives only A, although the host also has a live B grant.
    let mut documents = Documents::new(sources);
    documents.allowed = true;
    let a = documents.issue(0).unwrap();
    let b = documents.issue(1).unwrap();
    let rep = documents.representation(&b).unwrap();
    let operand = "(call $read (local.get $document) (i32.const 16))";
    assert_eq!(template.matches(operand).count(), 1);
    let forged = template.replacen(
        operand,
        &format!("(call $read (i32.const {rep}) (i32.const 16))"),
        1,
    );
    let forged = Component::from_binary(engine, &embedding::embed_guest(&forged, guest)?)?;
    let (mut denied, attacker) = setup(engine, &forged, documents)?;
    let error = inspect(&mut denied, &attacker, &a).unwrap_err();
    assert!(
        format!("{error:#}").contains("unknown handle index"),
        "{error:#}"
    );
    assert_eq!(denied.data().attempts, 0);
    assert!(denied.data().reads.is_empty());
    drop(denied); // Do not reuse a trapping instance or infer post-trap cleanup.
    drop(store);
    assert_eq!(held[1].text, "子😀");
    Ok(())
}
