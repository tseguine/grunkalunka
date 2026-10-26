use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};
use std::{
    io::Write,
    process::{Command, Stdio},
};
use wasmtime::{Engine, Instance, Module, Store, Trap};

fn probe(wat: &str) -> Vec<u8> {
    // Test-only visibility; production modules keep the arena private.
    let mut wat = wat.trim_end().to_owned();
    assert_eq!(wat.pop(), Some(')'));
    wat.push_str("(export \"cells\" (memory 0))(export \"top\" (global $cell_top)))");
    wat::parse_str(wat).unwrap()
}

fn node(script: &str) {
    let mut child = Command::new("node")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn cells_outlive_helpers_and_external_calls_reuse_the_arena_in_both_engines() {
    let source = "function keep(x){const read=()=>x;return read();} export function run(a,b){const x=keep(a);const y=keep(b);return x+y;}";
    let artifact = compile(source, false).unwrap();
    let wasm = probe(&artifact.wat);
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &Module::new(&engine, &wasm).unwrap(), &[]).unwrap();
    let run = instance
        .get_typed_func::<(f64, f64), f64>(&mut store, "run")
        .unwrap();
    let cells = instance.get_memory(&mut store, "cells").unwrap();
    let top = instance.get_global(&mut store, "top").unwrap();
    for (a, b) in [(4.0, 2.0), (7.0, 3.0), (-0.0, f64::from_bits(1))] {
        assert_eq!(
            run.call(&mut store, (a, b)).unwrap().to_bits(),
            (a + b).to_bits()
        );
        assert_eq!(top.get(&mut store).i32(), Some(32));
        for (offset, value) in [(0, a), (16, b)] {
            let mut bytes = [0; 8];
            cells.read(&store, offset, &mut bytes).unwrap();
            assert_eq!(u64::from_le_bytes(bytes), value.to_bits());
        }
    }
    node(&format!(
        "const assert=require('node:assert/strict'); const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({wasm:?}))).exports;for(const [a,b] of [[4,2],[7,3],[-0,Number.MIN_VALUE]]){{assert(Object.is(e.run(a,b),a+b));assert.equal(e.top.value,32);const v=new DataView(e.cells.buffer);assert(Object.is(v.getFloat64(0,true),a));assert(Object.is(v.getFloat64(16,true),b));}}"
    ));
}

#[test]
fn recursive_internal_entry_keeps_caller_cells_and_external_reentry_resets() {
    let source = "export function run(a,b){const read=()=>a;if(a<=0)return read();const x=run(a-1,b);return read()+x;}";
    let mut script = String::from("const assert=require('node:assert/strict');");
    for general in [false, true] {
        let artifact = if general {
            compile_value(source, false)
        } else {
            compile(source, false)
        }
        .unwrap();
        if general {
            let mut host = ValueInstance::new(&artifact.wasm).unwrap();
            for _ in 0..3 {
                assert!(matches!(host.run(4.0, 2.0).unwrap(), Value::Number(10.0)));
            }
        } else {
            let mut host = NumericInstance::new(&artifact.wasm).unwrap();
            for _ in 0..3 {
                assert_eq!(host.run(4.0, 2.0).unwrap(), 10.0);
            }
        }
        script.push_str(&format!("{{const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({:?}))).exports;for(let i=0;i<3;i++)assert.deepEqual(e.run(4,2),{});}}",artifact.wasm,if general {"[3,10]"} else {"10"}));
    }
    node(&script);
}

#[test]
fn arena_growth_limit_and_reuse_after_trap_are_checked_in_both_engines() {
    // Raw engines deliberately omit fuel for this bounded allocator stress test.
    let source = "export function run(a,b){let x=a;for(let i=0;i<b;i++){let y=i;const f=()=>y;x=f();}return x;}";
    let artifact = compile(source, false).unwrap();
    let wasm = probe(&artifact.wat);
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &Module::new(&engine, &wasm).unwrap(), &[]).unwrap();
    let run = instance
        .get_typed_func::<(f64, f64), f64>(&mut store, "run")
        .unwrap();
    let memory = instance.get_memory(&mut store, "cells").unwrap();
    let top = instance.get_global(&mut store, "top").unwrap();
    assert_eq!(run.call(&mut store, (0.0, 4097.0)).unwrap(), 4096.0);
    assert_eq!(memory.size(&store), 2);
    assert_eq!(run.call(&mut store, (0.0, 65536.0)).unwrap(), 65535.0);
    assert_eq!(memory.size(&store), 16);
    assert_eq!(top.get(&mut store).i32(), Some(1048576));
    let error = run.call(&mut store, (0.0, 65537.0)).unwrap_err();
    assert_eq!(
        error.downcast_ref::<Trap>(),
        Some(&Trap::UnreachableCodeReached)
    );
    assert_eq!(run.call(&mut store, (0.0, 1.0)).unwrap(), 0.0);
    assert_eq!(top.get(&mut store).i32(), Some(16));
    node(&format!(
        "const assert=require('node:assert/strict');const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({wasm:?}))).exports;assert.equal(e.run(0,4097),4096);assert.equal(e.cells.buffer.byteLength,131072);assert.equal(e.run(0,65536),65535);assert.equal(e.cells.buffer.byteLength,1048576);assert.equal(e.top.value,1048576);assert.throws(()=>e.run(0,65537),WebAssembly.RuntimeError);assert.equal(e.run(0,1),0);assert.equal(e.top.value,16);"
    ));
    let mut host = NumericInstance::new(&artifact.wasm).unwrap();
    let error = host.run(0.0, 65536.0).unwrap_err();
    assert_eq!(error.downcast_ref::<Trap>(), Some(&Trap::OutOfFuel));
    assert_eq!(host.run(0.0, 1.0).unwrap(), 0.0);

    // Constrain the test module to one page to force memory.grow's -1 path
    // before the allocator's own sixteen-page bound. Production max is unchanged.
    let limited = probe(&artifact.wat.replace("(memory 1 16)", "(memory 1 1)"));
    let instance =
        Instance::new(&mut store, &Module::new(&engine, &limited).unwrap(), &[]).unwrap();
    let run = instance
        .get_typed_func::<(f64, f64), f64>(&mut store, "run")
        .unwrap();
    let top = instance.get_global(&mut store, "top").unwrap();
    let error = run.call(&mut store, (0.0, 4097.0)).unwrap_err();
    assert_eq!(
        error.downcast_ref::<Trap>(),
        Some(&Trap::UnreachableCodeReached)
    );
    assert_eq!(top.get(&mut store).i32(), Some(65536));
    assert_eq!(run.call(&mut store, (0.0, 1.0)).unwrap(), 0.0);
    node(&format!(
        "const assert=require('node:assert/strict');const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({limited:?}))).exports;assert.throws(()=>e.run(0,4097),WebAssembly.RuntimeError);assert.equal(e.top.value,65536);assert.equal(e.cells.buffer.byteLength,65536);assert.equal(e.run(0,1),0);assert.equal(e.top.value,16);"
    ));
}

#[test]
fn only_runtime_references_need_addresses_including_transitive_unused_arrows() {
    for (source, expected) in [
        ("export function run(a,b){const f=()=>1;return a;}", false),
        (
            "export function run(a,b){const f=()=>{const g=()=>a;return b;};return a;}",
            true,
        ),
        (
            "export function run(a:number,b:number){const f=()=>{type T=typeof a;return 1;};return b;}",
            false,
        ),
    ] {
        let artifact = compile(source, true).unwrap();
        assert_eq!(artifact.wat.contains("(memory"), expected, "{source}");
    }
    let parameters = (0..1000)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let arguments = vec!["a"; 1000].join(",");
    let source = format!(
        "function keep({parameters}){{const f=()=>p999;return f();}}export function run(a,b){{return keep({arguments});}}"
    );
    let wasm = probe(&compile(&source, false).unwrap().wat);
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &Module::new(&engine, &wasm).unwrap(), &[]).unwrap();
    assert_eq!(
        instance
            .get_typed_func::<(f64, f64), f64>(&mut store, "run")
            .unwrap()
            .call(&mut store, (4.0, 2.0))
            .unwrap(),
        4.0
    );
    assert_eq!(
        instance
            .get_global(&mut store, "top")
            .unwrap()
            .get(&mut store)
            .i32(),
        Some(16)
    );
}
