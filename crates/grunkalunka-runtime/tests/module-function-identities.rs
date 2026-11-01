//! Runtime control for the production GC types and module-constant allocation.
//! Reference exports are test observers, not the source/public numeric ABI.
use std::{
    io::Write,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use wasmtime::{Collector, Config, Engine, Instance, Module, Store, Val};

fn module() -> Vec<u8> {
    let types = include_str!("../../grunkalunka-compiler/src/emit/gc_closures.wat");
    wat::parse_str(format!(
        r#"(module
      {types}
      (type $closure (func (param (ref null $gc_frame) (ref null $gc_frame)) (result f64)))
      (table 1 funcref) (elem (i32.const 0) func $body)
      (global $gc_module_function0 (ref $gc_function)
        (struct.new $gc_function (i32.const 0) (ref.null $gc_frame)))
      (func $body (param (ref null $gc_frame) (ref null $gc_frame)) (result f64)
        local.get 1 i32.const 0 array.get $gc_frame struct.get $gc_cell 1)
      (func (export "get") (result eqref) global.get $gc_module_function0)
      (func (export "same") (param eqref) (result i32)
        local.get 0 global.get $gc_module_function0 ref.eq)
      (func (export "run") (param f64 f64) (result f64)
        global.get $gc_module_function0 struct.get $gc_function 1
        i32.const 0 local.get 0 ref.null eq struct.new $gc_cell
        array.new_fixed $gc_frame 1
        global.get $gc_module_function0 struct.get $gc_function 0
        call_indirect (type $closure)))"#
    ))
    .unwrap()
}

#[test]
fn module_functions_are_stable_per_instance_and_survive_copying_gc() {
    let wasm = module();
    let script = format!(
        r#"const assert=require('node:assert/strict');
const module=new WebAssembly.Module(new Uint8Array({wasm:?}));
const left=new WebAssembly.Instance(module).exports,right=new WebAssembly.Instance(module).exports;
const load=()=>{{function f(x){{return x;}}return {{f,run:(a,b)=>f(a)}};}};
const native=load(),other=load();
assert.equal(left.get()===left.get(),native.f===native.f);
assert.equal(left.get()===right.get(),native.f===other.f);
assert.equal(left.same(left.get()),1);assert.equal(right.same(left.get()),0);
const values=[0,-0,1,-7,Infinity,-Infinity,NaN,Number.MIN_VALUE];
const saved=left.get();for(const a of values)for(const b of values){{
 assert.ok(Object.is(left.run(a,b),native.run(a,b)));assert.equal(left.get(),saved);
}}"#
    );
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
    let started = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() > Duration::from_secs(30) {
            child.kill().unwrap();
            let out = child.wait_with_output().unwrap();
            panic!(
                "module identity Node timeout: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mut config = Config::new();
    config.consume_fuel(true).collector(Collector::Copying);
    let engine = Engine::new(&config).unwrap();
    let module = Module::new(&engine, &wasm).unwrap();
    let mut store = Store::new(&engine, ());
    store.set_fuel(1_000_000).unwrap();
    let left = Instance::new(&mut store, &module, &[]).unwrap();
    let right = Instance::new(&mut store, &module, &[]).unwrap();
    let get = left.get_func(&mut store, "get").unwrap();
    let same_left = left.get_func(&mut store, "same").unwrap();
    let same_right = right.get_func(&mut store, "same").unwrap();
    let run = left
        .get_typed_func::<(f64, f64), f64>(&mut store, "run")
        .unwrap();
    let mut identity = [Val::null_any_ref()];
    get.call(&mut store, &[], &mut identity).unwrap();
    let values = [
        0.0,
        -0.0,
        1.0,
        -7.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        f64::from_bits(1),
    ];
    for a in values {
        for b in values {
            let actual = run.call(&mut store, (a, b)).unwrap();
            if a.is_nan() {
                assert!(actual.is_nan());
            } else {
                assert_eq!(actual.to_bits(), a.to_bits());
            }
            store.gc(None).unwrap();
            let mut same = [Val::I32(-1)];
            same_left.call(&mut store, &identity, &mut same).unwrap();
            assert_eq!(same[0].i32(), Some(1));
            same_right.call(&mut store, &identity, &mut same).unwrap();
            assert_eq!(same[0].i32(), Some(0));
        }
    }
}
