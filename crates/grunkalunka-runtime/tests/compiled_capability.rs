//! Connect real source compilation to a private WIT capability fixture.
use grunkalunka_compiler::compile;
use wasmtime::component::{Component, HasSelf, Linker, Resource};
use wasmtime::{Config, Engine, Store, Trap};

wasmtime::component::bindgen!({
    path: "tests/fixtures/compiled-capability",
    world: "calculator",
    with: { "grunkalunka:compiled/rates.rate": BackingRate },
    imports: { default: trappable },
});

pub struct BackingRate;

#[derive(Debug, PartialEq)]
enum AccessError {
    Denied,
    UnknownGrant,
}

impl std::fmt::Display for AccessError {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(output, "rate policy: {self:?}")
    }
}
impl std::error::Error for AccessError {}

#[derive(Default)]
struct Host {
    allowed: bool,
    value: f64,
    attempts: usize,
    reads: usize,
}

impl grunkalunka::compiled::rates::Host for Host {}

impl grunkalunka::compiled::rates::HostRate for Host {
    fn read(&mut self, rate: Resource<Rate>) -> wasmtime::Result<f64> {
        self.attempts += 1;
        if !self.allowed {
            wasmtime::bail!(AccessError::Denied);
        }
        if rate.rep() != 7 {
            wasmtime::bail!(AccessError::UnknownGrant);
        }
        self.reads += 1;
        Ok(self.value)
    }

    fn drop(&mut self, _: Resource<Rate>) -> wasmtime::Result<()> {
        wasmtime::bail!("this fixture does not transfer owned rates")
    }
}

fn engine() -> Engine {
    let mut config = Config::new();
    config.consume_fuel(true);
    Engine::new(&config).unwrap()
}

fn component(engine: &Engine, body: &str, typescript: bool) -> Component {
    let params = if typescript {
        "a: number, b: number"
    } else {
        "a, b"
    };
    let source = format!("export function run({params}) {{ {body} }}");
    let artifact = compile(&source, typescript).unwrap();
    assert_eq!(wat::parse_str(&artifact.wat).unwrap(), artifact.wasm);
    let fields = artifact.wat.strip_prefix("(module").unwrap();
    let core = format!("(core module{fields}");
    let template = include_str!("fixtures/compiled-capability/calculator.wat.in");
    assert_eq!(template.matches("{{COMPILED_MODULE}}").count(), 1);
    let bytes = wat::parse_str(template.replace("{{COMPILED_MODULE}}", &core)).unwrap();
    Component::from_binary(engine, &bytes).unwrap()
}

fn instantiate(engine: &Engine, component: &Component, allowed: bool) -> (Store<Host>, Calculator) {
    let mut linker = Linker::new(engine);
    Calculator::add_to_linker::<_, HasSelf<_>>(&mut linker, |host| host).unwrap();
    let mut store = Store::new(
        engine,
        Host {
            allowed,
            value: 3.0,
            ..Host::default()
        },
    );
    store.set_fuel(100_000).unwrap();
    let calculator = Calculator::instantiate(&mut store, component, &linker).unwrap();
    (store, calculator)
}

#[test]
fn compiled_js_and_ts_use_the_same_component_under_distinct_host_profiles() {
    let engine = engine();
    for typescript in [false, true] {
        let component = component(
            &engine,
            "const total = a * b; return total > 100 ? total - 10 : total;",
            typescript,
        );
        let mut absent = Store::new(&engine, ());
        absent.set_fuel(100_000).unwrap();
        let error = Calculator::instantiate(&mut absent, &component, &Linker::new(&engine))
            .err()
            .expect("the rate capability must be explicitly linked");
        assert!(format!("{error:#}").contains("grunkalunka:compiled/rates"));

        let (mut denied, calculator) = instantiate(&engine, &component, false);
        let error = calculator
            .call_run(&mut denied, Resource::new_borrow(7), 12.0)
            .unwrap_err();
        assert_eq!(
            error.downcast_ref::<AccessError>(),
            Some(&AccessError::Denied)
        );
        assert_eq!((denied.data().attempts, denied.data().reads), (1, 0));

        let (mut granted, calculator) = instantiate(&engine, &component, true);
        for (units, rate, expected) in [(12.0, 3.0, 36.0), (20.0, 7.0, 130.0), (0.0, 7.0, 0.0)] {
            granted.data_mut().value = rate;
            assert_eq!(
                calculator
                    .call_run(&mut granted, Resource::new_borrow(7), units)
                    .unwrap(),
                expected
            );
        }
        assert_eq!((granted.data().attempts, granted.data().reads), (3, 3));
    }
}

#[test]
fn changing_only_source_changes_the_component_computation() {
    let engine = engine();
    for typescript in [false, true] {
        for (body, expected) in [("return a * b;", 36.0), ("return a * b + 1;", 37.0)] {
            let component = component(&engine, body, typescript);
            let (mut store, calculator) = instantiate(&engine, &component, true);
            assert_eq!(
                calculator
                    .call_run(&mut store, Resource::new_borrow(7), 12.0)
                    .unwrap(),
                expected
            );
            assert_eq!((store.data().attempts, store.data().reads), (1, 1));
        }
    }
}

#[test]
fn an_unknown_host_representation_fails_before_reading_the_backing_value() {
    let engine = engine();
    let component = component(&engine, "return a * b;", false);
    let (mut store, calculator) = instantiate(&engine, &component, true);
    let error = calculator
        .call_run(&mut store, Resource::new_borrow(8), 12.0)
        .unwrap_err();
    assert_eq!(
        error.downcast_ref::<AccessError>(),
        Some(&AccessError::UnknownGrant)
    );
    assert_eq!((store.data().attempts, store.data().reads), (1, 0));
}

#[test]
fn source_guest_fuel_exhaustion_stays_distinct_from_capability_denial() {
    let engine = engine();
    for typescript in [false, true] {
        let component = component(&engine, "while (a) {} return b;", typescript);
        let (mut store, calculator) = instantiate(&engine, &component, true);
        store.set_fuel(10_000).unwrap();
        let error = calculator
            .call_run(&mut store, Resource::new_borrow(7), 1.0)
            .unwrap_err();
        assert_eq!(error.downcast_ref::<Trap>(), Some(&Trap::OutOfFuel));
        assert_eq!((store.data().attempts, store.data().reads), (1, 1));
        // A fresh instance of the same component can run its finite path.
        // This does not claim borrow cleanup or instance reuse after the trap.
        let (mut fresh, calculator) = instantiate(&engine, &component, true);
        assert_eq!(
            calculator
                .call_run(&mut fresh, Resource::new_borrow(7), 0.0)
                .unwrap(),
            3.0
        );
    }
}
