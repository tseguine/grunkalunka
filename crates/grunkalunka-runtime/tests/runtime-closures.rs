use grunkalunka_compiler::{compile, compile_async, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};
use std::{
    io::Write,
    process::{Command, Stdio},
};
use wasmtime::{Engine, Instance, Module, Store, Trap};

fn bits(value: f64) -> String {
    if value.is_nan() {
        "nan".into()
    } else {
        format!("{:016x}", value.to_bits())
    }
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
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn selected_functions_keep_code_captures_and_aliases_in_both_engines() {
    let bodies = [
        "const f=a?()=>a:()=>b;return f();",
        "const f=a?()=>a*10+b:()=>b*10+a;return f();",
        "const f=a?x=>x%a:x=>x%b;return f(b);",
        "const f=a?(x,y)=>x*10+y+a:(x,y)=>x-y;return f(a++,a++);",
        "const f=a?x=>x+a:x=>x-b;return f(b);",
        "let x=a;const f=a?()=>++x:()=>--x;const alias=f;f();return alias();",
        "const read=()=>a;const f=a?()=>read()+b:()=>read()-b;return f();",
        "const f=a?()=>a:()=>b;const g=a?()=>f()+1:()=>f()-1;return g();",
        "const make=x=>()=>++x;const f=a?make(a):make(b);return f()*10+f();",
        "const make=x=>()=>++x;const f=a?make(a):make(b);return f();",
        "const f=a?()=>a:()=>b;const x=f();return x;",
        "const f=a?()=>a:()=>b;let x=f();x+=b;return x;",
        "const f=a?()=>a:()=>b;let x=f();x=true;return +x;",
        "const f=a?x=>{const g=x?()=>x+b:()=>x-b;return g();}:x=>x;return f(a);",
        "let x=0;const f=a?(x+=1,()=>x):(x+=2,()=>x);return f()+x;",
        "const f=a?()=>{if(a)return b;return a;}:()=>{let x=b;return ++x;};return f()+a;",
        "const f=a?x=>{const read=()=>x;x+=b;return read();}:x=>x;return f(a);",
        "const f=a?x=>+(x=true):x=>+x;return f(b);",
        "const f=a?()=>-0:()=>b;return f();",
        "const flag=a<b;const f=a?()=>flag?b:a:()=>flag?a:b;return f();",
        "let x=null;const f=a?()=>+(x=true):()=>+x;f();return +x;",
    ];
    let inputs = [
        (4.0, 2.0),
        (0.0, 2.0),
        (-0.0, -2.0),
        (f64::NAN, f64::INFINITY),
    ];
    let mut script = String::from(
        "const assert=require('node:assert/strict');function bits(x){if(Number.isNaN(x))return 'nan';const v=new DataView(new ArrayBuffer(8));v.setFloat64(0,x,true);return v.getBigUint64(0,true).toString(16).padStart(16,'0');}\n",
    );
    for body in bodies {
        for typescript in [false, true] {
            let parameters = if typescript {
                "a:number,b:number"
            } else {
                "a,b"
            };
            let source = format!("export function run({parameters}){{{body}}}");
            let artifact =
                compile(&source, typescript).unwrap_or_else(|error| panic!("{source}: {error}"));
            assert!(artifact.wat.contains("call_indirect"));
            let mut host = NumericInstance::new(&artifact.wasm).unwrap();
            let rust = inputs
                .iter()
                .map(|&(a, b)| {
                    bits(
                        host.run(a, b)
                            .unwrap_or_else(|error| panic!("{source}: {error:#}")),
                    )
                })
                .collect::<Vec<_>>();
            script.push_str(&format!("{{const native=(a,b)=>{{{body}}};const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({:?}))).exports;const rust={rust:?};let i=0;for(const [a,b] of [[4,2],[0,2],[-0,-2],[NaN,Infinity]]){{const expected=bits(native(a,b));assert.equal(bits(e.run(a,b)),expected);assert.equal(rust[i++],expected);}}}}\n",artifact.wasm));
        }
    }
    node(&script);
}

#[test]
fn selected_numeric_function_results_keep_the_primitive_host_tag() {
    let source = "export function run(a,b){const f=a?()=>a:()=>b;return f();}";
    let artifact = compile_value(source, false).unwrap();
    let mut host = ValueInstance::new(&artifact.wasm).unwrap();
    assert!(matches!(host.run(4.0, 2.0).unwrap(), Value::Number(4.0)));
    assert!(
        matches!(host.run(0.0,-0.0).unwrap(),Value::Number(value) if value.to_bits()==(-0.0f64).to_bits())
    );
    let mixed = compile_value(
        "export function run(a,b){const f=a?()=>a:()=>b;return a?f():true;}",
        false,
    )
    .unwrap();
    let mut host = ValueInstance::new(&mixed.wasm).unwrap();
    assert!(matches!(host.run(4.0, 2.0).unwrap(), Value::Number(4.0)));
    assert!(matches!(host.run(0.0, 2.0).unwrap(), Value::Boolean(true)));
    node(&format!(
        "const assert=require('node:assert/strict');const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({:?}))).exports;assert.deepEqual(e.run(4,2),[3,4]);assert.deepEqual(e.run(0,2),[2,1]);",
        mixed.wasm
    ));
}

#[test]
fn source_helpers_and_nested_lifted_bodies_keep_their_function_indices() {
    let source = "function helper(x){const f=x?()=>x+1:()=>x-1;return f();}export function run(a,b){const f=a?()=>helper(a):()=>helper(b);return f()+helper(b);}";
    let artifact = compile(source, false).unwrap();
    let mut host = NumericInstance::new(&artifact.wasm).unwrap();
    assert_eq!(host.run(4.0, 2.0).unwrap(), 8.0);
    assert_eq!(host.run(0.0, 2.0).unwrap(), 6.0);
    let native = source.replace("export function", "function");
    node(&format!(
        "const assert=require('node:assert/strict');const native=Function({:?}+';return run;')();const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({:?}))).exports;for(const [a,b] of [[4,2],[0,2]])assert.equal(e.run(a,b),native(a,b));",
        native, artifact.wasm
    ));
}

#[test]
fn recursive_source_entry_preserves_runtime_function_environments() {
    let source = "export function run(a,b){const f=a?()=>a:()=>b;if(a<=0)return f();const x=run(a-1,b);return f()+x;}";
    let artifact = compile(source, false).unwrap();
    let mut host = NumericInstance::new(&artifact.wasm).unwrap();
    assert_eq!(host.run(4.0, 2.0).unwrap(), 12.0);
    assert_eq!(host.run(0.0, 2.0).unwrap(), 2.0);
    assert_eq!(host.run(4.0, 2.0).unwrap(), 12.0);
    node(&format!(
        "const assert=require('node:assert/strict');const native=Function({:?}+';return run;')();const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({:?}))).exports;for(const [a,b] of [[4,2],[0,2],[4,2]])assert.equal(e.run(a,b),native(a,b));",
        source.replace("export function", "function"),
        artifact.wasm
    ));
}

#[test]
fn one_thousand_numeric_arguments_use_a_fixed_two_pointer_call_signature() {
    let parameters = (0..1000)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let arguments = vec!["a"; 1000].join(",");
    let source = format!(
        "export function run(a,b){{const f=a?({parameters})=>p0+p999+a:({parameters})=>p0-p999+b;return f({arguments});}}"
    );
    let artifact = compile(&source, false).unwrap();
    assert!(
        artifact
            .wat
            .contains("(type $closure (func (param i32 i32) (result f64)))")
    );
    let mut host = NumericInstance::new(&artifact.wasm).unwrap();
    assert_eq!(host.run(4.0, 2.0).unwrap(), 12.0);
    assert_eq!(host.run(0.0, 2.0).unwrap(), 2.0);
    node(&format!(
        "const assert=require('node:assert/strict');const native=Function({:?}+';return run;')();const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({:?}))).exports;for(const [a,b] of [[4,2],[0,2]])assert.equal(e.run(a,b),native(a,b));",
        source.replace("export function", "function"),
        artifact.wasm
    ));
}

#[test]
fn unsupported_runtime_function_kinds_and_profiles_remain_explicit() {
    for body in [
        "let f=a?()=>a:()=>b;return f();",
        "const f=a?()=>a:()=>b;return f;",
        "const f=a?()=>a:()=>b;return +f;",
        "const f=a?()=>a:()=>b;return f===f;",
        "const f=a?()=>a:()=>b;return f(a);",
        "const f=a?x=>x:x=>x;return f();",
        "const f=a?()=>a:x=>x;return f();",
        "const f=a?()=>true:()=>b;return f();",
        "const f=a?async()=>a:()=>b;return f();",
        "const f=a?()=>a:()=>b;return f?.();",
        "const f=a?x=>x:x=>x;return f(...a);",
        "const f=a?()=>late:()=>b;const late=a;return f();",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(compile(&source, false).is_err(), "{source}");
    }
    for expression in ["f", "f===f", "+f", "f+1"] {
        let source =
            format!("export function run(a,b){{const f=a?()=>a:()=>b;return {expression};}}");
        assert!(compile_value(&source, false).is_err(), "{source}");
    }
    for body in [
        "const f=a?()=>a:()=>b;return await f;",
        "const f=a?()=>a:()=>b;return await Promise.resolve(f);",
        "const f=a?()=>a:()=>b;return await Promise.reject(f);",
        "const f=a?()=>a:()=>b;return f;",
        "const f=a?async()=>a:async()=>b;return await f();",
    ] {
        let source = format!("export async function run(a,b){{{body}}}");
        assert!(compile_async(&source, false).is_err(), "{source}");
    }
}

#[test]
fn runtime_constructor_lowering_has_an_observed_small_stack_bound() {
    let mut outcomes = Vec::new();
    for depth in 0..10 {
        let mut arrow = String::from("()=>a");
        for _ in 0..depth {
            arrow = format!("()=>{{const f=a?{arrow}:()=>b;return f();}}");
        }
        let source = format!("export function run(a,b){{const f=a?{arrow}:()=>b;return f();}}");
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        if let Ok(artifact) = result.as_ref() {
            assert_eq!(
                NumericInstance::new(&artifact.wasm)
                    .unwrap()
                    .run(4.0, 2.0)
                    .unwrap(),
                4.0
            );
        }
        outcomes.push(result.is_ok());
        if depth == 9 {
            assert!(result.err().unwrap().contains("8"));
        }
    }
    assert!(outcomes[0] && outcomes[1]);
    assert!(!outcomes[9]);
    println!("runtime closure nesting admission on 2MiB stack: {outcomes:?}");
}

fn probe(wat: &str, allocator: bool) -> Vec<u8> {
    let mut wat = wat.trim_end().to_owned();
    assert_eq!(wat.pop(), Some(')'));
    wat.push_str("(export \"arena\" (memory 0))(export \"top\" (global $cell_top))");
    if allocator {
        wat.push_str("(export \"allocate\" (func $closure_frame))");
    }
    wat.push(')');
    wat::parse_str(wat).unwrap()
}

#[test]
fn argument_and_environment_frames_share_a_bounded_reusable_arena() {
    let source = "export function run(a,b){let x=0;for(let i=0;i<b;i++){const f=a?()=>i:()=>-i;x=f();}return x;}";
    let artifact = compile(source, false).unwrap();
    let wasm = probe(&artifact.wat, false);
    // Finite allocator stress uses raw engines without the ordinary fuel limit.
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &Module::new(&engine, &wasm).unwrap(), &[]).unwrap();
    let run = instance
        .get_typed_func::<(f64, f64), f64>(&mut store, "run")
        .unwrap();
    let top = instance.get_global(&mut store, "top").unwrap();
    let memory = instance.get_memory(&mut store, "arena").unwrap();
    assert_eq!(run.call(&mut store, (1.0, 32767.0)).unwrap(), 32766.0);
    assert_eq!(top.get(&mut store).i32(), Some(1048560));
    assert_eq!(memory.size(&store), 16);
    let error = run.call(&mut store, (1.0, 32768.0)).unwrap_err();
    assert_eq!(
        error.downcast_ref::<Trap>(),
        Some(&Trap::UnreachableCodeReached)
    );
    assert_eq!(top.get(&mut store).i32(), Some(1048576));
    assert_eq!(run.call(&mut store, (1.0, 1.0)).unwrap(), 0.0);
    assert_eq!(top.get(&mut store).i32(), Some(48));
    node(&format!(
        "const assert=require('node:assert/strict');const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({wasm:?}))).exports;assert.equal(e.run(1,32767),32766);assert.equal(e.top.value,1048560);assert.equal(e.arena.buffer.byteLength,1048576);assert.throws(()=>e.run(1,32768),WebAssembly.RuntimeError);assert.equal(e.top.value,1048576);assert.equal(e.run(1,1),0);assert.equal(e.top.value,48);"
    ));
    let mut host = NumericInstance::new(&artifact.wasm).unwrap();
    let error = host.run(1.0, 32768.0).unwrap_err();
    assert_eq!(error.downcast_ref::<Trap>(), Some(&Trap::OutOfFuel));
    assert_eq!(host.run(1.0, 1.0).unwrap(), 0.0);
}

#[test]
fn frame_allocation_checks_alignment_growth_failure_and_multi_page_growth() {
    let artifact = compile(
        "export function run(a,b){const f=a?()=>1:()=>2;return f();}",
        false,
    )
    .unwrap();
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    for limited in [false, true] {
        let wat = if limited {
            artifact.wat.replace("(memory 1 16)", "(memory 1 1)")
        } else {
            artifact.wat.clone()
        };
        let wasm = probe(&wat, true);
        let instance =
            Instance::new(&mut store, &Module::new(&engine, &wasm).unwrap(), &[]).unwrap();
        let allocate = instance
            .get_typed_func::<i32, i32>(&mut store, "allocate")
            .unwrap();
        let top = instance.get_global(&mut store, "top").unwrap();
        for size in [-1, 1, 15, i32::MAX] {
            assert!(allocate.call(&mut store, size).is_err());
            assert_eq!(top.get(&mut store).i32(), Some(0));
        }
        if limited {
            assert!(allocate.call(&mut store, 65552).is_err());
            assert_eq!(top.get(&mut store).i32(), Some(0));
            assert_eq!(allocate.call(&mut store, 16).unwrap(), 0);
        } else {
            assert_eq!(allocate.call(&mut store, 1048576).unwrap(), 0);
            assert_eq!(top.get(&mut store).i32(), Some(1048576));
            assert!(allocate.call(&mut store, 16).is_err());
        }
        node(&format!(
            "const assert=require('node:assert/strict');const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({wasm:?}))).exports;for(const n of [-1,1,15,2147483647]){{assert.throws(()=>e.allocate(n),WebAssembly.RuntimeError);assert.equal(e.top.value,0);}}if({limited}){{assert.throws(()=>e.allocate(65552),WebAssembly.RuntimeError);assert.equal(e.top.value,0);assert.equal(e.allocate(16),0);}}else{{assert.equal(e.allocate(1048576),0);assert.equal(e.top.value,1048576);assert.equal(e.arena.buffer.byteLength,1048576);assert.throws(()=>e.allocate(16),WebAssembly.RuntimeError);}}"
        ));
    }
}

#[test]
fn dynamic_constructors_keep_source_depth_and_original_body_costs() {
    for (wrappers, admitted) in [(126, true), (127, false)] {
        let source = format!(
            "export function run(a,b){{const f={}(a?()=>a:()=>b){};return f();}}",
            "(".repeat(wrappers),
            ")".repeat(wrappers)
        );
        // One explicit pair around the conditional adds one source node.
        assert_eq!(
            compile(&source, false).is_ok(),
            admitted,
            "{wrappers} conditional wrappers"
        );
    }
    for (wrappers, admitted) in [(125, true), (126, false)] {
        let source = format!(
            "export function run(a,b){{const make=x=>()=>x;const f={}(a?make(a):make(b)){};return f();}}",
            "(".repeat(wrappers),
            ")".repeat(wrappers)
        );
        assert_eq!(
            compile(&source, false).is_ok(),
            admitted,
            "{wrappers} factory wrappers"
        );
    }
    for (locals, admitted) in [(126, true), (127, false)] {
        let declarations = (0..locals)
            .map(|i| format!("let v{i}=a;"))
            .collect::<String>();
        let source = format!(
            "export function run(a,b){{const read=()=>{{{declarations}return a;}};const f=a?read:read;return f();}}"
        );
        assert_eq!(compile(&source, false).is_ok(), admitted);
    }
    for (statements, admitted) in [(252, true), (253, false)] {
        let source = format!(
            "export function run(a,b){{const read=()=>{{{}return a;}};const f=a?read:read;return f();}}",
            "a=a;".repeat(statements)
        );
        assert_eq!(compile(&source, false).is_ok(), admitted);
    }
    for (calls, admitted) in [(254, true), (255, false)] {
        let source = format!(
            "export function run(a,b){{const f=a?()=>a:()=>b;return({});}}",
            vec!["f()"; calls].join(",")
        );
        assert_eq!(compile(&source, false).is_ok(), admitted);
    }
}

#[test]
fn erased_types_preserve_runtime_function_values_and_call_wrappers() {
    let plain = compile(
        "export function run(a,b){const f=a?x=>x+a:x=>x-b;return f(b);}",
        false,
    )
    .unwrap();
    let typed=compile("export function run(a:number,b:number){const f=(a?((x:never):string=>x+a):((x:never):string=>x-b)) as unknown as number;return(f as never)(b);}",true).unwrap();
    assert_eq!(plain.wasm, typed.wasm);
    assert_eq!(
        NumericInstance::new(&typed.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap(),
        6.0
    );
}

#[test]
fn nested_lifting_cannot_push_the_module_past_its_body_limit() {
    for (seed, selections, admitted) in [(false, 64, true), (true, 63, true), (true, 64, false)] {
        let mut source = String::from(
            "export function run(a,b){const leaf=()=>{const g=(a,()=>a);return g();};",
        );
        if seed {
            source.push_str("const seed=(a,()=>a);");
        }
        for i in 0..selections {
            source.push_str(&format!("const f{i}=a?leaf:leaf;"));
        }
        source.push_str("return a;}");
        let result = compile(&source, false);
        assert_eq!(
            result.is_ok(),
            admitted,
            "seed={seed} selections={selections}"
        );
        if !admitted {
            assert!(result.err().unwrap().contains("256 lifted closure bodies"));
        } else if !seed {
            assert!(result.unwrap().wat.contains("(table 257 funcref)"));
        }
    }
}
