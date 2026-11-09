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
fn imported_service_composes_with_locally_caught_source_failures() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a, b) {
            try {
                if (a < 0) throw a;
                return choose(a, b);
            } catch (reason) {
                return choose(0, b) + (+reason);
            }
        }"#,
    );
}

#[test]
fn imported_service_composes_with_source_cleanup_and_terminal_provider_failures() {
    for source in [
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a, b) {
            try {
                if (a < 0) throw a;
                return choose(a, b);
            } catch (reason) {
                return choose(0, b) + (+reason);
            } finally {
                choose(0, b);
            }
        }"#,
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a, b) {
            try {
                try { return a; }
                finally { choose(1, b); }
            } catch (reason) {
                return choose(0, b);
            }
        }"#,
    ] {
        compare_composed_source(source);
    }
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
fn imported_handler_applies_fixed_literal_arguments_through_the_real_wit_provider() {
    for source in [
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
           function relay(expanded, count) { return choose(expanded, count); }
           export function run(a, b) { return relay.apply(null, [a, b]); }"#,
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
           export function run(a, b) {
             let offset = 0;
             const relay = (expanded, count) => choose(expanded + offset, count);
             return relay.apply(offset = 1, [(offset = 2, a), b]) ? b : 0;
           }"#,
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
           export function run(a, b) {
             const action = {invoke: (expanded, count) => choose(expanded, count)};
             return action.invoke.apply(null, [a, b]) ? b : 0;
           }"#,
    ] {
        compare_composed_source(source);
    }
}

#[test]
fn imported_handler_builds_repeated_utf16_labels() {
    for body in [
        "const label='-'.repeat(3);return choose(label.length===3?a:0,b);",
        "const label='🍁'.repeat(2);return choose(label.length===4?a:0,b);",
        "const label='xy';return choose(label.repeat(2)==='xyxy'?a:0,b);",
        "let label='ab';const repeated=label.repeat((label='x',2));return choose(repeated==='abab'&&label==='x'?a:0,b);",
    ] {
        compare_composed_source(&format!(
            "import {{ selectVisible as choose }} from \"grunkalunka:a1/ui\";\n\
             export function run(a, b) {{ {body} }}"
        ));
    }
}

#[test]
fn imported_handler_invokes_defaulted_helpers_via_empty_apply() {
    for body in [
        "return choose(f.apply()===1?1:0,b);",
        "let n=0;const result=f['apply'](n=1);return choose(n===1&&result===1?1:0,b);",
    ] {
        // The A1 entry's function-statement profile is deliberately narrower
        // than the named module-helper profile. Test the latter directly.
        compare_composed_source(&format!(
            "import {{ selectVisible as choose }} from \"grunkalunka:a1/ui\";\n\
             function f(x=1){{return x;}}\n\
             export function run(a, b) {{ {body} }}"
        ));
    }
}

#[test]
fn imported_handler_builds_lists_with_array_push() {
    for body in [
        "const actions=[a];const length=actions.push(b);return choose(length===actions.length?a:0,b);",
        "const actions=[a];let step=0;const length=actions.push((step=1,b),(step=step*10+2,a));return choose(length===3&&step===12?a:0,b);",
        "const actions=[a];const length=actions.push(...[b,a]);return choose(length===3&&actions.length===3?a:0,b);",
    ] {
        compare_composed_source(&format!(
            "import {{ selectVisible as choose }} from \"grunkalunka:a1/ui\";\n\
             export function run(a, b) {{ {body} }}"
        ));
    }
}

#[test]
fn imported_handler_prioritizes_actions_with_array_unshift() {
    for body in [
        "const actions=[a,b];const count=actions.unshift(b);return choose(count===3&&actions[0]===b?a:0,b);",
        "const actions=[a];let step=0;const count=actions.unshift((step=1,b),(step=step*10+2,a));return choose(count===3&&step===12&&actions[0]===b?a:0,b);",
    ] {
        compare_composed_source(&format!(
            "import {{ selectVisible as choose }} from \"grunkalunka:a1/ui\";\n\
             export function run(a, b) {{ {body} }}"
        ));
    }
}

#[test]
fn imported_service_observes_private_default_results_through_primitive_comparisons() {
    for source in [
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
           function f(xs,n=xs.length){return n;}
           export function run(a, b) { return choose(f([a,b])===2?1:0,b); }"#,
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
           function f(x,y=x+1){return y;}
           export function run(a, b) { return choose(f(null)===1?1:0,b); }"#,
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
           function f(x=false||7){return x;}
           export function run(a, b) { return choose(f(undefined)===7?1:0,b); }"#,
    ] {
        compare_composed_source(source);
    }
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
fn imported_handler_combines_keyed_state_history_and_label_methods() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
           function labelFor(name) { return name.slice(2).substring(0,5); }
           function keyFor(label) { return label.slice(0,3); }
           export function run(a, b) {
             const label=labelFor(a>0?'# One😀':'# Two😀');
             const key=keyFor(label);
             const state=Object.create(null);
             state[key]=label;
             const history=[label,'stale'];
             const removed=history.pop();
             const selected=history.pop();
             state[key]=selected;
             const dispatch=()=>choose(
               state[key]===label&&removed==='stale'&&history.length===0&&
               label.startsWith('One')&&label.includes('😀')&&
               label.charAt(0)==='O'&&label.charCodeAt(0)===79?1:0,b);
             return dispatch()+label.length;
           }"#,
    );
}

#[test]
fn imported_handler_keeps_dictionary_assignment_snapshots_during_pop() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
           export function run(a, b) {
             let state=Object.create(null);
             const original=state;
             let key='current';
             const history=['One😀','Two'];
             const selected=state[(state=Object.create(null),key)]=
               history.pop((key='changed'));
             const label='Two😀 selected'.slice(0,5);
             const dispatch=()=>choose(
               selected==='Two'&&original.current===selected&&
               original.changed===undefined&&state.current===undefined&&
               state!==original&&key==='changed'&&history.length===1&&
               label.substring(0,3)==='Two'&&a>0?1:0,b);
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

#[test]
fn imported_handler_reads_nested_action_records_before_the_service_call() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        function apply(action, count) {
            const {view:{expanded},meta:{label}} = action;
            const invoke = () => choose(expanded && label.trim().length > 0 ? 1 : 0, count);
            return +invoke();
        }
        export function run(a, b) {
            return apply({view:{expanded:a > 0},meta:{label:' outline '}}, b);
        }"#,
    );
}

#[test]
fn imported_handler_iterates_nested_heading_records_with_source_defaults() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a, b) {
            const action = {
                view:{expanded:a > 0},
                groups:[[{label:' one '},{label:' two ',enabled:true}]]
            };
            const {view:{expanded},groups:[headings]} = action;
            let count = 0, defaults = 0;
            for (const {label,enabled=(defaults++,true)} of headings) {
                if (enabled && label.trim().length > 0) count++;
            }
            const invoke = () => choose(expanded && defaults === 1 ? 1 : 0, count);
            return +invoke();
        }"#,
    );
}

#[test]
fn imported_handler_defaults_optional_action_records_and_heading_options() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a, b) {
            const defaultHeadings = [{label:' one '},{options:{enabled:true}}];
            const action = a > 0 ? {
                view:{expanded:true},
                groups:[defaultHeadings]
            } : {};
            const {view:{expanded=false}={},groups:[headings=defaultHeadings]=[]} = action;
            let count = 0;
            for (const {label=' Untitled ',options:{enabled=true}={}} of headings) {
                if (enabled && label.trim().length > 0) count++;
            }
            return choose(expanded ? 1 : 0, count);
        }"#,
    );
}

#[test]
fn imported_handler_uses_an_independent_rest_list_with_shared_heading_values() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a, b) {
            const heading = {label:' first '};
            const source = [{label:' title '},heading,{label:' second '}];
            const [title,...headings] = source;
            source[1] = {label:' replacement '};
            const retained = headings.at(0) === heading && headings[0].label.trim() === 'first';
            let count = 0;
            for (const {label} of headings) {
                if (label.trim().length > 0) count++;
            }
            return choose(a > 0 && retained && title.label.trim().length > 0 ? 1 : 0, count);
        }"#,
    );
}

#[test]
fn imported_handler_separates_title_metadata_from_numbered_headings() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a, b) {
            const note = {0:' Title ',1:' First ',2:' Second '};
            const {0:title,...headings} = note;
            note[1] = 'Replacement';
            const enabled = a > 0 && title.trim() === 'Title' && headings[1].trim() === 'First';
            return choose(enabled ? 1 : 0, Object.keys(headings).length);
        }"#,
    );
}

#[test]
fn imported_handler_selects_dictionary_state_and_counts_remaining_actions() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a, b) {
            const actions = Object.create(null);
            actions.expand = 1;
            actions.collapse = 0;
            actions.reload = 1;
            const action = a > 0 ? 'expand' : 'collapse';
            const {[action]:selected,...remaining} = actions;
            return choose(action in actions && selected === 1 ? 1 : 0, Object.keys(remaining).length);
        }"#,
    );
}

#[test]
fn imported_handler_detects_explicit_undefined_action_metadata() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a, b) {
            const overrides = Object.create(null);
            const key = 'expanded';
            if (a > 0) overrides[key] = undefined;
            const configured = Object.hasOwn(overrides, key);
            const headings = ['First', 'Second'];
            const count = Object.hasOwn(headings, 1) ? headings.length : 0;
            return choose(configured ? 1 : 0, count);
        }"#,
    );
}

#[test]
fn imported_handler_tests_a_runtime_key_on_an_ordinary_action_record() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a,b) {
            const available = {expand: undefined};
            const action = a > 0 ? 'expand' : 'collapse';
            const headings = ['First','Second'];
            return choose(action in available ? 1 : 0, headings.length);
        }"#,
    );
}

#[test]
fn imported_handler_builds_a_record_with_the_runtime_action_name() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a,b) {
            const action = a > 0 ? 'expand' : 'collapse';
            const available = {reload: true, [action]: undefined};
            return choose('expand' in available ? 1 : 0, Object.keys(available).length);
        }"#,
    );
}

#[test]
fn imported_handler_counts_the_values_of_an_action_record() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a,b) {
            const action = a > 0 ? 'expand' : 'collapse';
            const actions = {reload: 'always', [action]: action};
            const options = Object.values(actions);
            let expanded = 0;
            for (const option of options) {
                if (option === 'expand') expanded = 1;
            }
            return choose(expanded, options.length);
        }"#,
    );
}

#[test]
fn imported_handler_selects_an_action_from_its_enumerated_names() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a,b) {
            const action = a > 0 ? 'expand' : 'collapse';
            const names = Object.keys({reload: true, [action]: undefined});
            let expanded = 0;
            for (const name of names) {
                if (name.trim() === 'expand') expanded = 1;
            }
            return choose(expanded, names.length);
        }"#,
    );
}

#[test]
fn imported_handler_iterates_action_entries_by_name() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a,b) {
            const action = a > 0 ? 'expand' : 'collapse';
            const actions = {reload: 0, [action]: 1};
            const rows = Object.entries(actions);
            let expanded = 0;
            for (const [name, value] of rows) {
                if (name.trim() === 'expand' && value === 1) expanded = 1;
            }
            return choose(expanded, rows.length);
        }"#,
    );
}

#[test]
fn imported_handler_distinguishes_action_lists_from_single_records() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a,b) {
            const options = {items: a > 0 ? ['expand', 'collapse'] : {action: 'reload'}};
            return choose(Array.isArray(options.items) ? 1 : 0, 2);
        }"#,
    );
}

#[test]
fn imported_handler_reads_labels_from_enumerated_action_records() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a,b) {
            const action = a > 0 ? 'expand' : 'collapse';
            const actions = {reload: {label: ' Reload '}, [action]: {label: ' Toggle '}};
            let toggle = 0;
            for (const [name, {label}] of Object.entries(actions)) {
                if (name === 'expand' && label.trim() === 'Toggle') toggle = 1;
            }
            return choose(toggle, Object.keys(actions).length);
        }"#,
    );
}

#[test]
fn imported_handler_counts_characters_in_a_display_label() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a,b) {
            const label = a > 0 ? 'A\u{1f600}' : 'B';
            let characters = 0;
            for (const ch of label) characters++;
            return choose(characters - 1, 2);
        }"#,
    );
}

#[test]
fn imported_handler_selects_from_a_shallow_copy_of_the_action_list() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a,b) {
            const actions = [{name: ' reload '}, {name: a > 0 ? ' expand ' : ' collapse '}];
            const visible = [...actions];
            actions[0] = {name: ' replaced '};
            let expanded = 0;
            for (const {name} of visible) {
                if (name.trim() === 'expand') expanded = 1;
            }
            return choose(expanded, visible.length);
        }"#,
    );
}

#[test]
fn imported_handler_selects_the_last_character_in_a_display_label() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a,b) {
            const label = a > 0 ? 'Go\u{1f600}' : 'Back';
            const characters = [...label];
            return choose(characters.at(-1) === '\u{1f600}' ? 1 : 0, 2);
        }"#,
    );
}

#[test]
fn imported_handler_checks_the_enabled_action_list_by_value() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a,b) {
            const enabled = ['reload', 'expand'];
            const requested = a > 0 ? 'expand' : 'collapse';
            return choose(enabled.includes(requested) ? 1 : 0, enabled.length);
        }"#,
    );
}

#[test]
fn imported_handler_finds_the_last_matching_action_position() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a,b) {
            const names = ['reload', 'expand', 'reload'];
            const action = a > 0 ? 'expand' : 'reload';
            return choose(names.lastIndexOf(action) === 1 ? 1 : 0, 2);
        }"#,
    );
}

#[test]
fn imported_handler_selects_from_a_window_of_actions() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        function page(items, start) { return items.slice(start, start + 2); }
        export function run(a,b) {
            const actions = ['reload', 'expand', 'collapse'];
            const visible = page(actions, a > 0 ? 0 : 1);
            const handle = () => choose(visible.indexOf('expand'), visible.length);
            return +handle();
        }"#,
    );
}

#[test]
fn imported_handler_reads_labels_from_a_sliced_action_page() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        function page(items) { return items.slice(1, 3); }
        export function run(a,b) {
            const actions = [{label:' Ignore '}, {label:' Reload '}, {label:a > 0 ? ' Expand ' : ' Collapse '}];
            const visible = page(actions);
            let expanded = 0;
            for (const {label} of visible) {
                if (label.trim() === 'Expand') expanded = 1;
            }
            const handle = () => choose(expanded, visible.length);
            return +handle();
        }"#,
    );
}

#[test]
fn imported_handler_uses_a_reversed_action_history() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a,b) {
            const history = ['reload', 'expand'];
            const latest = history.slice().reverse();
            const requested = a > 0 ? 'reload' : 'expand';
            const handle = () => choose(latest.indexOf(requested), latest.length);
            return +handle();
        }"#,
    );
}

#[test]
fn imported_handler_consumes_the_first_queued_action() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        export function run(a,b) {
            const queue = [a > 0 ? 'expand' : 'collapse', 'reload', 'refresh'];
            const handle = () => {
                const action = queue.shift();
                return choose(action === 'expand' ? 1 : 0, queue.length);
            };
            return +handle();
        }"#,
    );
}

#[test]
fn imported_handler_reads_reversed_action_labels() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
    export function run(a,b) {
        const actions=[{label:' Reload '},{label:a>0?' Expand ':' Collapse '}];
        actions.reverse();let selected=0;
        for(const {label} of actions){if(label.trim()==='Expand')selected=1;}
        return choose(selected,actions.length);
    }"#,
    );
}

#[test]
fn imported_handler_reads_a_reversed_history_snapshot() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
    export function run(a,b) {
        const history=[{label:' Reload '},{label:a>0?' Expand ':' Collapse '}];
        const visible=history.toReversed();let selected=0;
        for(const {label} of visible){if(label.trim()==='Expand')selected=1;}
        return choose(selected,visible.length);
    }"#,
    );
}

#[test]
fn imported_handler_selects_a_trailing_action_marker() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
    function trailing(label) { return label.at(-1); }
    export function run(a,b) {
        const label = a > 0 ? 'Expand+' : 'Collapse-';
        const handle = () => choose(trailing(label) === '+' ? 1 : 0, 2);
        return +handle();
    }"#,
    );
}

#[test]
fn imported_handler_classifies_a_unicode_action_marker() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
    function marker(label) { return label.codePointAt(0); }
    export function run(a,b) {
        const label = a > 0 ? '\u{1f50d} Find' : '';
        const handle = () => choose(marker(label) === 128269 ? 1 : 0, 2);
        return +handle();
    }"#,
    );
}

#[test]
fn imported_handler_defaults_a_missing_action_marker() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
    function last(label) { return label.at(-1); }
    export function run(a,b) {
        const label = a > 0 ? 'Expand+' : '';
        const {suffix='-'} = {suffix:last(label)};
        const handle = () => choose(suffix.trim() === '+' ? 1 : 0, 2);
        return +handle();
    }"#,
    );
}

#[test]
fn imported_handler_validates_label_utf16() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
    export function run(a,b) {
        const label=a>0?'Find \u{1f50d}':'Broken \ud800';
        const handle=()=>choose(label.isWellFormed()?1:0,2);
        return +handle();
    }"#,
    );
}

#[test]
fn imported_handler_repairs_label_utf16() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
    function repair(s){return s.toWellFormed();}
    export function run(a,b) {
        const label=a>0?'Find \u{1f50d}':'Broken \ud800';
        const clean=repair(label);
        const handle=()=>choose(clean.endsWith('\ufffd')?0:1,2);
        return +handle();
    }"#,
    );
}

#[test]
fn imported_handler_composes_an_action_label() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
    function label(action){return 'Menu'.concat(': ',action);}
    export function run(a,b){const text=label(a>0?'Expand':'Collapse');const handle=()=>choose(text.endsWith('Expand')?1:0,2);return +handle();}
    "#,
    );
}

#[test]
fn imported_handler_builds_a_template_label() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
    function label(action){return `Menu: ${action}`;}
    export function run(a,b){const text=label(a>0?'Expand':'Collapse');const handle=()=>choose(text.endsWith('Expand')?1:0,2);return +handle();}
    "#,
    );
}

#[test]
fn imported_handler_reads_an_action_marker_by_index() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
    function first(label){return label[0];}
    export function run(a,b){const label=a>0?'+ Expand':'';const handle=()=>choose(first(label)==='+'?1:0,2);return +handle();}
    "#,
    );
}

#[test]
fn imported_handler_defaults_a_missing_selected_action_record() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        function pick(actions,index) { return actions.at(index); }
        export function run(a,b) {
            const actions=[{label:' Expand '},{label:' Collapse '}];
            const {action={label:' Collapse '}}={action:pick(actions,a>0?0:8)};
            const handle=()=>choose(action.label.trim()==='Expand'?1:0,actions.length);
            return +handle();
        }"#,
    );
}

#[test]
fn imported_handler_coalesces_an_optional_action_record() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        function pick(actions,index) { return actions.at(index)??{label:' Collapse '}; }
        export function run(a,b) {
            const actions=[{label:' Expand '},null];
            const action=pick(actions,a>0?0:1);
            const handle=()=>choose(action.label.trim()==='Expand'?1:0,actions.length);
            return +handle();
        }"#,
    );
}

#[test]
fn imported_handler_selects_an_action_with_logical_defaults() {
    compare_composed_source(
        r#"import { selectVisible as choose } from "grunkalunka:a1/ui";
        function select(value) {
            const candidate=value>0?{}:null;
            return (candidate&&{label:' Expand '})||{label:' Collapse '};
        }
        export function run(a,b) {
            const action=select(a);
            const handle=()=>choose(action.label.trim()==='Expand'?1:0,2);
            return +handle();
        }"#,
    );
}
