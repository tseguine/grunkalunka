//! Private Canonical ABI ownership observations; no public resource policy.
use std::collections::BTreeMap;
use wasmtime::component::{Component, HasSelf, Linker, Resource};
use wasmtime::{Engine, Store};

wasmtime::component::bindgen!({
    path: "tests/fixtures/resource-ownership",
    world: "owner",
    with: { "grunkalunka:owned-test/cells.cell": BackingCell },
    imports: { default: trappable },
});

pub struct BackingCell;

const CELL: u32 = 7;

struct OwnedHost {
    live: BTreeMap<u32, f64>,
    reads: usize,
    destructors: usize,
}

impl OwnedHost {
    fn with_cell() -> Self {
        Self {
            live: [(CELL, 9.0)].into(),
            reads: 0,
            destructors: 0,
        }
    }
}

impl grunkalunka::owned_test::cells::Host for OwnedHost {}

impl grunkalunka::owned_test::cells::HostCell for OwnedHost {
    fn read(&mut self, cell: Resource<Cell>) -> wasmtime::Result<f64> {
        self.reads += 1;
        assert!(!cell.owned());
        if let Some(value) = self.live.get(&cell.rep()) {
            Ok(*value)
        } else {
            wasmtime::bail!("test backing cell is not live")
        }
    }

    fn drop(&mut self, cell: Resource<Cell>) -> wasmtime::Result<()> {
        self.destructors += 1;
        assert!(cell.owned());
        assert_eq!(self.live.remove(&cell.rep()), Some(9.0));
        Ok(())
    }
}

fn instantiate() -> (Store<OwnedHost>, Owner) {
    let engine = Engine::default();
    let bytes = wat::parse_str(include_str!("fixtures/resource-ownership/owner.wat")).unwrap();
    let component = Component::from_binary(&engine, &bytes).unwrap();
    let mut linker = Linker::new(&engine);
    Owner::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state).unwrap();
    let mut store = Store::new(&engine, OwnedHost::with_cell());
    let owner = Owner::instantiate(&mut store, &component, &linker).unwrap();
    (store, owner)
}

#[test]
fn explicit_guest_consumption_destroys_the_owned_backing_once() {
    let (mut store, owner) = instantiate();
    owner
        .call_consume(&mut store, Resource::new_own(CELL))
        .unwrap();
    assert_eq!(store.data().destructors, 1);
    assert!(store.data().live.is_empty());
    assert_eq!(store.data().reads, 0);
}

#[test]
fn owned_roundtrip_then_consumption_uses_the_returned_token() {
    let (mut store, owner) = instantiate();
    let returned = owner
        .call_roundtrip(&mut store, Resource::new_own(CELL))
        .unwrap();
    assert!(returned.owned());
    assert_eq!(returned.rep(), CELL);
    assert_eq!(store.data().live.get(&CELL), Some(&9.0));
    assert_eq!((store.data().reads, store.data().destructors), (0, 0));
    assert_eq!(
        owner
            .call_peek(&mut store, Resource::new_borrow(returned.rep()))
            .unwrap(),
        9.0
    );
    assert_eq!((store.data().reads, store.data().destructors), (1, 0));
    assert_eq!(store.data().live.get(&CELL), Some(&9.0));
    // Transfer the actual lifted token onward; do not remint a host representation.
    owner.call_consume(&mut store, returned).unwrap();
    assert_eq!((store.data().reads, store.data().destructors), (1, 1));
    assert!(store.data().live.is_empty());
}

#[test]
fn repeated_borrows_release_guest_handles_without_destroying_backing() {
    let (mut store, owner) = instantiate();
    for _ in 0..3 {
        assert_eq!(
            owner
                .call_peek(&mut store, Resource::new_borrow(CELL))
                .unwrap(),
            9.0
        );
        assert_eq!(store.data().destructors, 0);
        assert_eq!(store.data().live.get(&CELL), Some(&9.0));
    }
    assert_eq!(store.data().reads, 3);
}

#[test]
fn owned_result_lifting_removes_the_saved_guest_index_before_replay() {
    let (mut store, owner) = instantiate();
    let returned = owner
        .call_roundtrip(&mut store, Resource::new_own(CELL))
        .unwrap();
    assert!(returned.owned());
    assert_eq!(returned.rep(), CELL);
    // IMMEDIATE replay: no fresh resource argument/allocation can reuse the index.
    // The guest saved its actual lowered index, not a forged Rust resource rep.
    let error = owner.call_replay(&mut store).unwrap_err();
    assert!(
        format!("{error:#}").contains("unknown handle index"),
        "{error:#}"
    );
    eprintln!("owned stale-index replay: {error:#}");
    assert_eq!((store.data().reads, store.data().destructors), (0, 0));
    assert_eq!(store.data().live.get(&returned.rep()), Some(&9.0));
    // No guest call or post-trap cleanup assertion follows this trap.
}

#[test]
fn duplicate_guest_drop_traps_after_exactly_one_host_destructor() {
    let (mut store, owner) = instantiate();
    let error = owner
        .call_double_drop(&mut store, Resource::new_own(CELL))
        .unwrap_err();
    assert!(
        format!("{error:#}").contains("unknown handle index"),
        "{error:#}"
    );
    eprintln!("duplicate owned drop: {error:#}");
    assert_eq!(store.data().destructors, 1);
    assert!(store.data().live.is_empty());
    assert_eq!(store.data().reads, 0);
    // This instance is independent of the stale-index expected-trap case.
}
