//! Real source-compiled JS/TS -> core-Wasm import -> WIT -> Rust provider.
//! No handwritten adapter calls an import-free compiled function in this test.
use grunkalunka_compiler::{Artifact, compile, compile_ui_import};
use std::io::Write;
use std::process::{Command, Stdio};
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Config, Engine, Store};

wasmtime::component::bindgen!({
    path: "tests/fixtures/a1-ui",
    world: "outline",
    imports: { default: trappable },
});

#[derive(Debug, Eq, PartialEq)]
enum AccessError {
    Denied,
    InvalidCount,
}

impl std::fmt::Display for AccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "A1 UI policy: {self:?}")
    }
}

impl std::error::Error for AccessError {}

#[derive(Default)]
struct Host {
    allowed: bool,
    permitted_count: usize,
    attempts: usize,
    transitions: usize,
    visible: usize,
}

impl grunkalunka::a1::ui::Host for Host {
    fn select_visible(&mut self, expanded: f64, count: f64) -> wasmtime::Result<f64> {
        self.attempts += 1;
        if !self.allowed {
            wasmtime::bail!(AccessError::Denied);
        }
        if !count.is_finite() || count != self.permitted_count as f64 || self.permitted_count > 64 {
            wasmtime::bail!(AccessError::InvalidCount);
        }
        self.visible = if expanded > 0.0 {
            self.permitted_count
        } else {
            0
        };
        self.transitions += 1;
        Ok(self.visible as f64)
    }
}

fn engine() -> Engine {
    let mut config = Config::new();
    config.consume_fuel(true);
    Engine::new(&config).unwrap()
}

fn source(typescript: bool, changed: bool) -> String {
    let (a, b, local) = if typescript {
        ("a: number", "b: number", "let x: number")
    } else {
        ("a", "b", "let x")
    };
    let operation = if changed { "-=" } else { "+=" };
    format!(
        "import {{ selectVisible as choose }} from \"grunkalunka:a1/ui\";\n\
         export function run({a}, {b}) {{ {local} = a;\n\
         return choose((x {operation} b), b, (x = 0)) + x; }}"
    )
}

fn compiled(typescript: bool, changed: bool) -> Artifact {
    compile_ui_import(&source(typescript, changed), typescript).unwrap()
}

fn component(engine: &Engine, artifact: &Artifact) -> Component {
    assert_eq!(wat::parse_str(&artifact.wat).unwrap(), artifact.wasm);
    assert!(
        artifact
            .wat
            .contains("(import \"grunkalunka:a1/ui\" \"select-visible\"")
    );
    assert!(artifact.wat.contains("call $ui_select_visible"));
    let fields = artifact.wat.strip_prefix("(module").unwrap();
    let core = format!("(core module{fields}");
    let template = include_str!("fixtures/a1-ui/outline.wat.in");
    assert_eq!(template.matches("{{COMPILED_MODULE}}").count(), 1);
    let bytes = wat::parse_str(template.replace("{{COMPILED_MODULE}}", &core)).unwrap();
    Component::from_binary(engine, &bytes).unwrap()
}

fn linked(engine: &Engine, component: &Component, allowed: bool) -> (Store<Host>, Outline) {
    let mut linker = Linker::new(engine);
    Outline::add_to_linker::<_, HasSelf<_>>(&mut linker, |host| host).unwrap();
    let mut store = Store::new(
        engine,
        Host {
            allowed,
            permitted_count: 2,
            ..Host::default()
        },
    );
    store.set_fuel(100_000).unwrap();
    let instance = Outline::instantiate(&mut store, component, &linker).unwrap();
    (store, instance)
}

#[test]
fn source_compiled_js_and_ts_invoke_real_rust_wit_ui_provider() {
    let engine = engine();
    let mut twins = Vec::new();
    for ts in [false, true] {
        let artifact = compiled(ts, false);
        twins.push(artifact.wasm.clone());
        assert!(
            compile(&source(ts, false), ts).is_err(),
            "ordinary compilation must not admit runtime imports"
        );
        let component = component(&engine, &artifact);

        let mut missing = Store::new(&engine, ());
        missing.set_fuel(100_000).unwrap();
        let err = Outline::instantiate(&mut missing, &component, &Linker::new(&engine))
            .err()
            .expect("UI service must require explicit provider linking");
        assert!(format!("{err:#}").contains("grunkalunka:a1/ui"));

        let (mut denied, instance) = linked(&engine, &component, false);
        let err = instance.call_run(&mut denied, 1.0, 2.0).unwrap_err();
        assert_eq!(
            err.downcast_ref::<AccessError>(),
            Some(&AccessError::Denied)
        );
        assert_eq!((denied.data().attempts, denied.data().transitions), (1, 0));

        let (mut granted, instance) = linked(&engine, &component, true);
        assert_eq!(instance.call_run(&mut granted, 1.0, 2.0).unwrap(), 2.0);
        assert_eq!(instance.call_run(&mut granted, -3.0, 2.0).unwrap(), 0.0);
        assert_eq!(granted.data().attempts, 2);
        assert_eq!(granted.data().transitions, 2);
        assert_eq!(granted.data().visible, 0);

        // The count is an owned provider fact, not a guest-authorized query.
        let bad = instance.call_run(&mut granted, 1.0, 3.0).unwrap_err();
        assert_eq!(
            bad.downcast_ref::<AccessError>(),
            Some(&AccessError::InvalidCount)
        );
        assert_eq!(granted.data().transitions, 2);
        assert_eq!(granted.data().visible, 0);
    }
    assert_eq!(
        twins[0], twins[1],
        "TS metadata must not change the Wasm ABI"
    );
}

#[test]
fn source_mutation_reaches_provider_and_changes_completed_ui_state() {
    let engine = engine();
    for ts in [false, true] {
        let original = compiled(ts, false);
        let changed = compiled(ts, true);
        assert_ne!(original.wasm, changed.wasm);
        let (mut granted, instance) = linked(&engine, &component(&engine, &original), true);
        assert_eq!(instance.call_run(&mut granted, 1.0, 2.0).unwrap(), 2.0);
        assert_eq!(granted.data().visible, 2);
        let (mut granted, instance) = linked(&engine, &component(&engine, &changed), true);
        assert_eq!(instance.call_run(&mut granted, 1.0, 2.0).unwrap(), 0.0);
        assert_eq!(granted.data().visible, 0);
    }
}

#[test]
fn same_compiled_wasm_executes_in_node_with_an_explicit_import() {
    // The WIT Rust-host test above is the authority evidence. This independent
    // Node/V8 engine only verifies core-Wasm import ABI and source sequencing.
    let script = r#"
      const assert = require('node:assert/strict');
      const fs = require('node:fs');
      const wasm = fs.readFileSync(0);
      const module = new WebAssembly.Module(wasm);
      assert.deepEqual(WebAssembly.Module.imports(module), [{
        module: 'grunkalunka:a1/ui', name: 'select-visible', kind: 'function'
      }]);
      let calls = [];
      const instance = new WebAssembly.Instance(module, {
        'grunkalunka:a1/ui': {
          'select-visible': (expanded, count) => {
            calls.push([expanded, count]);
            return expanded > 0 ? count : 0;
          }
        }
      });
      assert.equal(instance.exports.run(1, 2), 2);
      assert.deepEqual(calls, [[3, 2]]);
      calls = [];
      assert.equal(instance.exports.run(-3, 2), 0);
      assert.deepEqual(calls, [[-1, 2]]);
    "#;
    for ts in [false, true] {
        let artifact = compiled(ts, false);
        let mut child = Command::new("node")
            .arg("-e")
            .arg(script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Node runtime available in canonical CI");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&artifact.wasm)
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "Node import oracle: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

#[test]
fn unsupported_module_and_import_forms_remain_rejected() {
    let valid = source(false, false);
    let unsupported = [
        valid.replace("grunkalunka:a1/ui", "node:fs"),
        valid.replace("selectVisible as choose", "other as choose"),
        valid.replace("{ selectVisible as choose }", "choose"),
        valid.replace("import { selectVisible as choose }", "import * as choose"),
        valid.replace(
            "from \"grunkalunka:a1/ui\";",
            "from \"grunkalunka:a1/ui\";\nimport x from 'x';",
        ),
        valid.replace("choose((x += b), b, (x = 0))", "choose(...[a, b])"),
        valid.replace("choose((x += b), b, (x = 0))", "choose(a)"),
        valid.replace("choose((x += b), b, (x = 0))", "choose?.(a,b)"),
        valid.replace("return choose((x += b), b, (x = 0)) + x;", "return choose;"),
        valid.replace("return choose((x += b), b, (x = 0)) + x;", "return a + b;"),
        valid.replace("choose((x += b), b, (x = 0))", "choose(a,b) + run(a,b)"),
    ];
    for input in unsupported {
        assert!(compile_ui_import(&input, false).is_err(), "{input}");
    }
}
