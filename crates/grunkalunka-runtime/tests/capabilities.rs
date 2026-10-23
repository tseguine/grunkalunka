//! A concrete component/host-policy experiment, not a public grant API.
use std::sync::Arc;
use wasmtime::component::{Component, HasSelf, Linker, Resource};
use wasmtime::{Engine, Store};

wasmtime::component::bindgen!({
    path: "tests/fixtures/capabilities",
    world: "runner",
    with: { "grunkalunka:r1/cells.cell": BackingCell },
    imports: { default: trappable },
});

pub struct BackingCell;

#[derive(Debug, PartialEq)]
enum PolicyError {
    Denied,
    Revoked,
    ForeignOwner,
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cell policy: {self:?}")
    }
}

impl std::error::Error for PolicyError {}

#[derive(Default)]
enum Profile {
    #[default]
    Denied,
    Granted,
}

#[derive(Default)]
struct Host {
    owner: Arc<()>,
    profile: Profile,
    value: f64,
    attempts: usize,
    reads: usize,
    generation: u32,
    live: bool,
}

#[derive(Clone)]
struct Grant {
    owner: Arc<()>,
    rep: u32,
}

impl Host {
    fn issue(&mut self, value: f64) -> Grant {
        self.generation = self
            .generation
            .checked_add(1)
            .expect("grant generation exhausted");
        self.value = value;
        self.live = true;
        Grant {
            owner: self.owner.clone(),
            rep: self.generation,
        }
    }

    fn revoke(&mut self) {
        self.live = false;
    }
}

fn call(
    store: &mut Store<Host>,
    runner: &Runner,
    grant: &Grant,
    a: f64,
    b: f64,
) -> wasmtime::Result<f64> {
    // Resource::new_borrow authenticates neither a host token nor its store.
    // Keep the owner's allocation alive through every token to avoid identity reuse.
    if !Arc::ptr_eq(&store.data().owner, &grant.owner) {
        wasmtime::bail!(PolicyError::ForeignOwner);
    }
    runner.call_run(store, Resource::new_borrow(grant.rep), a, b)
}

impl grunkalunka::r1::cells::Host for Host {}

impl grunkalunka::r1::cells::HostCell for Host {
    fn read(&mut self, cell: Resource<Cell>) -> wasmtime::Result<f64> {
        self.attempts += 1;
        if matches!(self.profile, Profile::Denied) {
            wasmtime::bail!(PolicyError::Denied);
        }
        if !self.live || cell.rep() != self.generation {
            wasmtime::bail!(PolicyError::Revoked);
        }
        self.reads += 1;
        Ok(self.value)
    }

    fn drop(&mut self, _: Resource<Cell>) -> wasmtime::Result<()> {
        wasmtime::bail!("this host never transfers owned cells")
    }
}

fn reader(engine: &Engine) -> Component {
    let bytes = wat::parse_str(include_str!("fixtures/capabilities/reader.wat")).unwrap();
    Component::from_binary(engine, &bytes).unwrap()
}

fn instantiate(engine: &Engine, component: &Component, host: Host) -> (Store<Host>, Runner) {
    let mut linker = Linker::new(engine);
    Runner::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state).unwrap();
    let mut store = Store::new(engine, host);
    let runner = Runner::instantiate(&mut store, component, &linker).unwrap();
    (store, runner)
}

#[test]
fn identical_component_has_absent_denied_and_granted_profile_outcomes() {
    let engine = Engine::default();
    let component = reader(&engine);
    let mut absent = Store::new(&engine, ());
    let error = Runner::instantiate(&mut absent, &component, &Linker::new(&engine))
        .err()
        .expect("the cell interface must be explicitly linked");
    assert!(format!("{error:#}").contains("grunkalunka:r1/cells"));

    let (mut denied, runner) = instantiate(&engine, &component, Host::default());
    let cell = denied.data_mut().issue(9.0);
    let error = call(&mut denied, &runner, &cell, 4.0, 1.0).unwrap_err();
    assert_eq!(
        error.downcast_ref::<PolicyError>(),
        Some(&PolicyError::Denied)
    );
    assert_eq!((denied.data().attempts, denied.data().reads), (1, 0));
    // Reuse the exact compiled component. Authority is entirely host-selected.
    let (mut granted, runner) = instantiate(
        &engine,
        &component,
        Host {
            profile: Profile::Granted,
            ..Host::default()
        },
    );
    let cell = granted.data_mut().issue(9.0);
    for (a, b, expected) in [(4.0, 1.0, 14.0), (-2.0, 3.0, 10.0)] {
        assert_eq!(call(&mut granted, &runner, &cell, a, b).unwrap(), expected);
    }
    assert_eq!((granted.data().attempts, granted.data().reads), (2, 2));
}

#[test]
fn revoked_aliases_fail_before_backing_reads_and_new_grants_do_not_revive_them() {
    let engine = Engine::default();
    let component = reader(&engine);
    // Each expected trap gets an equivalent fresh instance. A previous trap must
    // not make a later revocation assertion pass for an unrelated ABI reason.
    for replacement in [false, true] {
        for alias in 0..2 {
            let (mut store, runner) = instantiate(
                &engine,
                &component,
                Host {
                    profile: Profile::Granted,
                    ..Host::default()
                },
            );
            let old = store.data_mut().issue(9.0);
            assert_eq!(call(&mut store, &runner, &old, 1.0, 2.0).unwrap(), 12.0);
            store.data_mut().revoke();
            if replacement {
                let new = store.data_mut().issue(20.0);
                assert_ne!(old.rep, new.rep);
                assert_eq!(call(&mut store, &runner, &new, 1.0, 2.0).unwrap(), 23.0);
            }
            let before = (store.data().attempts, store.data().reads);
            let aliases = [old.clone(), old];
            let error = call(&mut store, &runner, &aliases[alias], 1.0, 2.0).unwrap_err();
            assert_eq!(
                error.downcast_ref::<PolicyError>(),
                Some(&PolicyError::Revoked)
            );
            assert_eq!(
                (store.data().attempts, store.data().reads),
                (before.0 + 1, before.1)
            );
        }
    }
}

#[test]
fn foreign_host_grants_cannot_alias_colliding_resource_representations() {
    let engine = Engine::default();
    let component = reader(&engine);
    let (mut first, first_runner) = instantiate(
        &engine,
        &component,
        Host {
            profile: Profile::Granted,
            ..Host::default()
        },
    );
    let (mut second, second_runner) = instantiate(
        &engine,
        &component,
        Host {
            profile: Profile::Granted,
            ..Host::default()
        },
    );
    let first_grant = first.data_mut().issue(9.0);
    let second_grant = second.data_mut().issue(20.0);
    assert_eq!(first_grant.rep, second_grant.rep);
    assert_eq!(
        call(&mut first, &first_runner, &first_grant, 1.0, 2.0).unwrap(),
        12.0
    );
    assert_eq!(
        call(&mut second, &second_runner, &second_grant, 1.0, 2.0).unwrap(),
        23.0
    );
    let error = call(&mut second, &second_runner, &first_grant, 1.0, 2.0).unwrap_err();
    assert_eq!(
        error.downcast_ref::<PolicyError>(),
        Some(&PolicyError::ForeignOwner)
    );
    assert_eq!((second.data().attempts, second.data().reads), (1, 1));
}

#[test]
fn forged_guest_integer_traps_before_host_dispatch() {
    let engine = Engine::default();
    let component = reader(&engine);
    let (mut store, runner) = instantiate(
        &engine,
        &component,
        Host {
            profile: Profile::Granted,
            ..Host::default()
        },
    );
    let _live = store.data_mut().issue(9.0);
    let error = runner.call_forge(&mut store).unwrap_err();
    assert!(
        format!("{error:#}").contains("unknown handle index"),
        "{error:#}"
    );
    assert_eq!((store.data().attempts, store.data().reads), (0, 0));
}

#[test]
fn retained_borrow_traps_on_return_without_a_host_read() {
    let engine = Engine::default();
    let component = reader(&engine);
    let (mut store, runner) = instantiate(&engine, &component, Host::default());
    let cell = store.data_mut().issue(9.0);
    let error = runner
        .call_retain(&mut store, Resource::new_borrow(cell.rep))
        .unwrap_err();
    assert!(
        format!("{error:#}").contains("borrow handles still remain at the end of the call"),
        "{error:#}"
    );
    assert_eq!((store.data().attempts, store.data().reads), (0, 0));
}

#[test]
fn dropped_borrow_integer_is_stale_on_a_later_call_without_a_fresh_borrow() {
    let engine = Engine::default();
    let component = reader(&engine);
    let (mut store, runner) = instantiate(
        &engine,
        &component,
        Host {
            profile: Profile::Granted,
            ..Host::default()
        },
    );
    let cell = store.data_mut().issue(9.0);
    assert_eq!(
        runner
            .call_stash(&mut store, Resource::new_borrow(cell.rep))
            .unwrap(),
        0.0
    );
    // No new resource argument: do not allocate a fresh guest slot with the same index.
    let error = runner.call_replay(&mut store).unwrap_err();
    assert!(
        format!("{error:#}").contains("unknown handle index"),
        "{error:#}"
    );
    assert_eq!((store.data().attempts, store.data().reads), (0, 0));
}
