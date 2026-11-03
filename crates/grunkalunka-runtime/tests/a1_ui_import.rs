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
    calls: Vec<(f64, f64)>,
}

impl grunkalunka::a1::ui::Host for Host {
    fn select_visible(&mut self, expanded: f64, count: f64) -> wasmtime::Result<f64> {
        self.attempts += 1;
        self.calls.push((expanded, count));
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
    config
        .consume_fuel(true)
        .collector(wasmtime::Collector::Copying);
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

// Compare the same source with native JavaScript, the exact core-Wasm bytes in
// Node, and the actual Rust WIT provider. Provider call arguments and resulting
// state are part of the observation, not just the source function's return.
fn compare_composed_source(source: &str) {
    let script = r#"
      const assert = require('node:assert/strict');
      const fs = require('node:fs');
      const source = process.argv[1];
      const wit = JSON.parse(process.argv[2]);
      let calls = [], visible = 0;
      const choose = (expanded, count) => {
        assert.equal(count, 2);
        calls.push([expanded, count]);
        visible = expanded > 0 ? count : 0;
        return visible;
      };
      const native = new Function('choose', '"use strict"; ' + source
        .replace('import { selectVisible as choose } from "grunkalunka:a1/ui";', '')
        .replace('export function', 'function') + '; return run;')(choose);
      const module = new WebAssembly.Module(fs.readFileSync(0));
      assert.deepEqual(WebAssembly.Module.imports(module), [{
        module: 'grunkalunka:a1/ui', name: 'select-visible', kind: 'function'
      }]);
      const guest = new WebAssembly.Instance(module, {
        'grunkalunka:a1/ui': { 'select-visible': choose }
      }).exports.run;
      for (const [index, a] of [-3, 0, 1, 3].entries()) {
        calls = []; visible = 0;
        const expected = [native(a, 2), visible, calls];
        calls = []; visible = 0;
        assert.deepEqual([guest(a, 2), visible, calls], expected);
        assert.deepEqual(wit[index], expected);
      }
    "#;
    let engine = engine();
    let mut twins = Vec::new();
    for ts in [false, true] {
        let input = if ts {
            source.replace(
                "function run(a, b)",
                "function run(a: number, b: number): number",
            )
        } else {
            source.to_owned()
        };
        let artifact =
            compile_ui_import(&input, ts).unwrap_or_else(|error| panic!("{input}: {error}"));
        twins.push(artifact.wasm.clone());
        let component = component(&engine, &artifact);
        let (mut store, instance) = linked(&engine, &component, true);
        let mut observations = Vec::new();
        for a in [-3.0, 0.0, 1.0, 3.0] {
            store.gc(None).unwrap();
            store.set_fuel(100_000).unwrap();
            store.data_mut().calls.clear();
            store.data_mut().visible = 0;
            let result = instance.call_run(&mut store, a, 2.0).unwrap();
            let calls = store
                .data()
                .calls
                .iter()
                .map(|(expanded, count)| format!("[{expanded},{count}]"))
                .collect::<Vec<_>>()
                .join(",");
            observations.push(format!("[{result},{},[{calls}]]", store.data().visible));
        }
        let mut child = Command::new("node")
            .args([
                "-e",
                script,
                source,
                &format!("[{}]", observations.join(",")),
            ])
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
            "{input}: {}",
            String::from_utf8_lossy(&result.stderr)
        );

        let (mut denied, instance) = linked(&engine, &component, false);
        let error = instance.call_run(&mut denied, 1.0, 2.0).unwrap_err();
        assert_eq!(
            error.downcast_ref::<AccessError>(),
            Some(&AccessError::Denied)
        );
        assert_eq!((denied.data().attempts, denied.data().transitions), (1, 0));
    }
    assert_eq!(
        twins[0], twins[1],
        "TS annotations must not change source calls"
    );
}

#[test]
fn imported_service_composes_with_named_forward_and_recursive_helpers() {
    for source in [
        // The export, forward helper, and provider all have the same ABI, so
        // wrong numeric function indices can silently return the wrong value.
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
           export function run(a, b) { return adjust(a, b); }
           function adjust(expanded, count) {
             let x = expanded;
             return choose((x += count), count, (x = 0)) + x;
           }"#,
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
           function even(x, count) { if (x > 0) return odd(x - 1, count); return choose(x, count); }
           function odd(x, count) { if (x > 0) return even(x - 1, count); return choose(x + 1, count); }
           export function run(a, b) { return even(a, b) + 10; }"#,
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
           export function run(a, b) { if (a > 0) return run(a - 1, b); return choose(a, b); }"#,
        // An ordinary parameter shadow must not acquire the imported binding.
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
           function sum(choose, count) { return choose + count; }
           export function run(a, b) { return choose(sum(a, b), b); }"#,
    ] {
        compare_composed_source(source);
    }
}

#[test]
fn imported_service_composes_with_runtime_closures_and_shared_cells() {
    for body in [
        "let state=a;let dispatch=x=>choose(state+x,b);state+=1;return dispatch(b)+state;",
        "let n=a;let f=x=>choose((n+=x),b);const old=f;f=x=>choose((n-=x),b);return old(b)+f(b)+n;",
        "let make=x=>y=>choose(x+y,b);const first=make(a),second=make(b);return first(b)*10+second(a);",
        "let first=()=>a,last=()=>b;for(let i=0;i<3;i++){const f=()=>choose(i,b);if(i===0)first=f;last=f;}return first()*10+last();",
    ] {
        compare_composed_source(&format!(
            "import {{ selectVisible as choose }} from \"grunkalunka:a1/ui\";\n\
             export function run(a, b) {{ {body} }}"
        ));
    }
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
           function select(x, count) { return choose(x, count); }
           export function run(a, b) { let f=x=>select(x+a,b);return f(b); }"#,
    );
}

#[test]
fn imported_service_composes_with_gc_captures_and_function_identities() {
    for body in [
        "let state={expanded:a};const initial=state;let dispatch=()=>choose(state.expanded===a?1:0,b);state={expanded:b};return dispatch()+(initial===state?10:20);",
        "let x=a;let f=()=>choose(x,b);const handlers=[f];x=b;return handlers[0]()+(handlers[0]===f?10:20);",
        "let make=x=>{const state={expanded:x};return ()=>choose(state.expanded===a?1:0,b);};const f=make(a),g=make(b);return f()*10+g()+(f===g?100:200);",
    ] {
        compare_composed_source(&format!(
            "import {{ selectVisible as choose }} from \"grunkalunka:a1/ui\";\n\
             export function run(a, b) {{ {body} }}"
        ));
    }
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
           function handler(x, count) { return choose(x, count); }
           export function run(a, b) {
             const actions={handler};const alias=handler;
             return actions.handler(a,b)+(actions.handler===alias?10:20);
           }"#,
    );
}

#[test]
fn imported_handler_combines_runtime_headings_and_string_labels() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
           export function run(a, b) {
             const headings=['One','Two😀'];
             const name=a>0?'Two😀':'';
             const label=name+' selected';
             const selected=headings.at(a>0?-1:0);
             let dispatch=()=>choose(name.length>0&&label.length>0&&selected===name?1:0,b);
             const saved=dispatch;
             dispatch=()=>choose(0,b);
             return saved()+label.length;
           }"#,
    );
}

#[test]
fn imported_handler_keeps_selected_and_label_snapshots_after_mutation() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
           export function run(a, b) {
             const headings=['One','Two'];
             const selected=headings.at(a>0?-1:0);
             headings[1]='renamed';
             let name='One';
             const label=name+(name=a>0?'Two':'',' selected');
             let dispatch=()=>choose(selected===name&&label==='One selected'&&label.length===12?1:0,b);
             return dispatch()+label.length;
           }"#,
    );
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
    ];
    for input in unsupported {
        assert!(compile_ui_import(&input, false).is_err(), "{input}");
    }
}
