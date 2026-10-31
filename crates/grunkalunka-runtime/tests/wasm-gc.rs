use grunkalunka_runtime::NumericInstance;
use std::{
    io::Write,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};
use wasmtime::{Collector, Config, Engine, Instance, Module, Store};

fn graph_module() -> Vec<u8> {
    wat::parse_str(
        r#"(module
          (rec (type $cell (struct
            (field (mut f64)) (field (mut (ref null $cell))))))
          (type $items (array (mut (ref null $cell))))
          (global $saved (mut (ref null $items)) (ref.null $items))
          (func (export "run") (param f64 f64) (result f64)
            (local $left (ref null $cell)) (local $alias (ref null $cell))
            (local $right (ref null $cell))
            local.get 0 ref.null $cell struct.new $cell
            local.tee $left local.set $alias
            local.get 1 ref.null $cell struct.new $cell local.set $right
            local.get $left local.get $left struct.set $cell 1
            local.get $left local.get $right array.new_fixed $items 2
            global.set $saved
            global.get $saved i32.const 0 array.get $items
            local.get 1 struct.set $cell 0
            local.get $left local.get $alias ref.eq i32.eqz
            if unreachable end
            local.get $left local.get $right ref.eq
            if unreachable end
            local.get $left struct.get $cell 1 local.get $left ref.eq i32.eqz
            if unreachable end
            local.get $alias struct.get $cell 0)
          (func (export "read") (result f64)
            (local $left (ref null $cell))
            global.get $saved i32.const 0 array.get $items local.set $left
            local.get $left struct.get $cell 1 local.get $left ref.eq i32.eqz
            if unreachable end
            local.get $left struct.get $cell 0))"#,
    )
    .unwrap()
}

#[test]
fn numeric_boundary_executes_gc_backed_state() {
    let mut instance = NumericInstance::new(&graph_module()).unwrap();
    for expected in [4.0, -2.0, 0.0, -0.0, f64::INFINITY] {
        assert_eq!(
            instance.run(13.0, expected).unwrap().to_bits(),
            expected.to_bits()
        );
    }
}

#[test]
fn copying_collection_preserves_global_array_edges_and_cycles() {
    let mut config = Config::new();
    config.consume_fuel(true).collector(Collector::Copying);
    let engine = Engine::new(&config).unwrap();
    let module = Module::from_binary(&engine, &graph_module()).unwrap();
    let mut store = Store::new(&engine, ());
    store.set_fuel(10_000).unwrap();
    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    let run = instance
        .get_typed_func::<(f64, f64), f64>(&mut store, "run")
        .unwrap();
    let read = instance
        .get_typed_func::<(), f64>(&mut store, "read")
        .unwrap();
    for expected in [1.0_f64, -0.0, -7.0, f64::INFINITY] {
        store.set_fuel(10_000).unwrap();
        assert_eq!(
            run.call(&mut store, (3.0, expected)).unwrap().to_bits(),
            expected.to_bits()
        );
        for _ in 0..3 {
            store.gc(None).unwrap();
            assert_eq!(
                read.call(&mut store, ()).unwrap().to_bits(),
                expected.to_bits()
            );
        }
    }
}

fn node(source: &str) -> Output {
    let mut command = Command::new("node");
    let mut child = command
        .args(["--input-type=module", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(source.as_bytes())
        .unwrap();
    let started = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() > Duration::from_secs(30) {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!(
                "Node GC test exceeded 30 seconds: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    child.wait_with_output().unwrap()
}

#[test]
fn gc_aliases_mutation_and_identity_match_native_javascript() {
    let source = format!(
        "const bytes=new Uint8Array({:?});\n{}",
        graph_module(),
        include_str!("wasm-gc.mjs")
    );
    let output = node(&source);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS native JS / WasmGC"));
}

#[test]
fn numeric_boundary_still_rejects_reference_results() {
    let wasm = wat::parse_str(
        r#"(module
      (type $cell (struct (field f64)))
      (func (export "run") (param f64 f64) (result (ref $cell))
        local.get 0 struct.new $cell))"#,
    )
    .unwrap();
    Module::from_binary(&Engine::default(), &wasm).unwrap();
    assert!(NumericInstance::new(&wasm).is_err());
}
