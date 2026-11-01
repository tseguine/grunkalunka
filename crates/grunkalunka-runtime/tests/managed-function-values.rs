#[path = "managed/common.rs"]
mod common;

#[test]
fn source_function_properties_preserve_binding_identity() {
    let source = "export function run(a,b){const f=()=>a;const o={f};return o.f===f?a:b;}";
    common::compare_sources(source, source);
}

fn compare(body: &str) {
    let source = format!("export function run(a,b){{{body}}}");
    common::compare_sources(&source, &source);
}

#[test]
fn aliases_reboxing_and_distinct_creations_keep_identity() {
    for body in [
        "const f=()=>a;const g=f;const o={f,g};return o.f===o.g&&o.f===f?a:b;",
        "const f=()=>a;const o={f};o.g=f;const x=[f,o.f,o.g];return x[0]===x[1]&&x[1]===x[2]?a:b;",
        "const f=()=>0;const g=()=>0;const x=[f,g];return x[0]!==x[1]&&f!==g?a:b;",
        "const make=()=>()=>0;const f=make();const g=make();return f!==g&&[f][0]===f?a:b;",
        "const make=n=>()=>n;const f=make(a);const g=make(a);return f!==g&&[f][0]===f&&f()===a?a:b;",
        "const o={f:()=>a};const p={f:()=>a};return o.f!==p.f?a:b;",
        "const f=()=>a;return f===f?a:b;",
        "return (()=>a)!==(()=>a)?a:b;",
        "const make=()=>()=>a;return (make())!==make()?a:b;",
        "const make=()=>()=>a;return (make())!==(make())?a:b;",
        "const make=()=>()=>a;return (a?(make()):(make()))!==(make())?a:b;",
        "const f=()=>a;const id=x=>x;return id(f)===f?a:b;",
    ] {
        compare(body);
    }
}

#[test]
fn function_data_remains_callable_through_typed_bindings() {
    for body in [
        "let f=()=>a;const old=f;const o={f};f=()=>b;return o.f===old&&o.f!==f&&old()===a&&f()===b?a:b;",
        "const f=()=>a;const g=(x,y)=>x+y;const x=[f,g,a,'text',null,{}];return x[0]===f&&x[1]===g&&typeof x[0]==='function'&&typeof x[1]==='function'&&g(a,b)===a+b?a:b;",
        "let n=a;const f=()=>n;const x=[f];n=b;return x[0]===f&&f()===b?a:b;",
        "const f=()=>a;const o={f};return o.f&&typeof o.f==='function'&&o.f!==null&&o.f!==a?a:b;",
        "const f=()=>({x:a});const o={f};return o.f===f&&f().x===a?a:b;",
        "let n=0;const f=()=>a;const o={f:(n++,f)};return n===1&&o.f===f?a:b;",
        "const f=()=>()=>a;const o={f};const g=f();return o.f===f&&g()===a?a:b;",
    ] {
        compare(body);
    }
}

#[test]
fn callback_and_factory_storage_evaluate_creation_effects_once() {
    for body in [
        "const wrap=fn=>({fn});const f=()=>a;const o=wrap(f);const p=wrap(f);return o.fn===p.fn&&o.fn===f?a:b;",
        "const wrap=fn=>({fn});const f=()=>a;const o=wrap(f);const p=wrap(f);return o.fn===p.fn?a:b;",
        "let wrap=fn=>({fn,value:fn()});const f=()=>a;const o=wrap(f);const p=wrap(f);return o.fn===p.fn&&o.value===a?a:b;",
        "const wrap=fn=>[fn];const f=()=>a;const o=wrap.call(b,f);return o[0]===f?a:b;",
        "let n=0;const make=()=>{n++;return ()=>a;};const f=make();const o={f};return n===1&&o.f===f?a:b;",
        "let n=0;const make=x=>()=>x;const f=make((n++,a));const o={f};return n===1&&o.f===f&&f()===a?a:b;",
        "let n=0;const make=x=>{n++;return ()=>x;};const f=make(a);const g=make(b);const o={f,g};return n===2&&o.f!==o.g&&f()===a&&g()===b?a:b;",
        "const wrap=fn=>({fn});const forward=fn=>wrap(fn);const f=()=>a;return forward(f).fn===f?a:b;",
        "let f=()=>a;const wrap=(fn,x)=>({fn});const old=f;const o=wrap(f,(f=()=>b,b));return o.fn===old&&o.fn!==f?a:b;",
    ] {
        compare(body);
    }
}

#[test]
fn function_cycles_and_branch_local_bindings_preserve_identity() {
    for body in [
        "const o={};const f=()=>o;const x=[f,o];o.f=f;o.self=o;o.x=x;return o.f===f&&o.self===o&&x[0]===f&&f()===o?a:b;",
        "const f=()=>a;if(a){const x=[f];return x[0]===f?a:b;}const o={f};return o.f===f?a:b;",
        "let first=()=>a;let last=()=>b;for(let i=0;i<3;i++){const n=i;const f=()=>n;if(i===0)first=f;last=f;}const x=[first,last];return x[0]!==x[1]&&first()===0&&last()===2?a:b;",
    ] {
        compare(body);
    }
    common::compare_sources(
        "export function run(a,b){const f=x=>x;const o={f};return o.f===f&&f(a)===a?a:b;}",
        "export function run(a:string,b:boolean):void{const f=(x:boolean):string=>x;const o={f};return o.f===f&&f(a)===a?a:b;}",
    );
}

#[test]
fn former_function_container_guards_execute_unchanged() {
    for body in [
        "const f=()=>a;const o={f};return a;",
        "const o=[()=>a];return a;",
        "const f=()=>({x:()=>a});return a;",
        "const o={x:()=>a};return a;",
        "const f=()=>a;const o={x:f};return a;",
        "let f=()=>a;const x=[f];return typeof f==='function'&&x[0]===f?a:b;",
        "const x={}||(()=>a);return a;",
        "return typeof(a?()=>a:b)==='function'?a:b;",
        "const f=()=>a;const x={}&&f;return x===f?a:b;",
    ] {
        compare(body);
    }
}

#[test]
fn function_data_keeps_unimplemented_boundaries_explicit() {
    for body in [
        "const f=()=>a;const o={f};return f;",
        "const o={f:()=>Symbol()};return a;",
        "const o={f:async()=>{await 0;return a;}};return a;",
        "function f(){return a;}const o={f};return a;",
        "let first=()=>a;for(let i=0;i<3;i++){first=()=>i;}const o={first};return first();",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let error = grunkalunka_compiler::compile(&source, ts)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(
                error.contains("unsupported") && !error.contains("internal IR"),
                "{source}: {error}"
            );
        }
    }
}

#[test]
fn function_data_queries_respect_two_megabyte_native_stacks() {
    for (depth, admitted) in [(16, true), (140, false)] {
        let source = format!(
            "export function run(a,b){{const f=()=>a;const o={{f:{}f{}}};return o.f===f?a:b;}}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || grunkalunka_compiler::compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(
            result.is_ok(),
            admitted,
            "depth={depth}: {:?}",
            result.as_ref().err()
        );
        if let Err(error) = result {
            assert!(error.contains("nesting"), "{error}");
        }
    }
}

// Observe the actual generated carrier. No source function or frame code is
// replaced, and production exports still expose only the numeric boundary.
fn rooted_function_carrier(original: &str) -> Vec<u8> {
    let lines: Vec<_> = original.lines().collect();
    let mut wat = String::new();
    let mut roots = 0;
    for (index, line) in lines.iter().enumerate() {
        wat.push_str(line);
        wat.push('\n');
        if *line == "struct.new $gc_value"
            && index >= 3
            && lines[index - 3] == "i32.const 6"
            && lines[index - 2] == "f64.const 0"
        {
            assert!(lines[index - 1].starts_with("local.get "));
            wat.push_str("global.set $test_carrier\nglobal.get $test_carrier\n");
            roots += 1;
        }
    }
    assert_eq!(roots, 1, "one source function is boxed into a property");
    wat.truncate(wat.rfind(')').unwrap());
    wat.push_str(
        r#"
      (global $test_carrier (mut (ref null $gc_value)) (ref.null $gc_value))
      (func (export "read_saved") (result f64)
        (local $f (ref null $gc_function))
        global.get $test_carrier struct.get $gc_value 2
        ref.cast (ref null $gc_function) local.set $f
        local.get $f struct.get $gc_function 1
        i32.const 0 array.new_default $gc_frame
        local.get $f struct.get $gc_function 0 call_indirect (type $closure))
      (func (export "saved_kind") (result i32)
        global.get $test_carrier struct.get $gc_value 0)
      (func (export "clear_saved") ref.null $gc_value global.set $test_carrier))"#,
    );
    wat::parse_str(wat).unwrap()
}

#[test]
fn generated_stored_function_cycles_survive_copying_gc_after_source_returns() {
    use wasmtime::{Collector, Config, Engine, Instance, Module, Store};
    for source in [
        "export function run(a,b){const o={x:a};const f=()=>o.x===b?1:0;o.f=f;o.self=o;o.x=b;return f();}",
        "export function run(a,b){const state={x:a};const f=()=>state.x===b?1:0;const o={f};state.o=o;state.self=state;state.x=b;return o.f();}",
        "export function run(a,b){const state={x:a};const f=()=>state.x===b?1:0;const o={child:{f}};state.o=o;state.self=state;state.x=b;return o.child.f();}",
    ] {
        common::compare_sources(source, source);
        let mut config = Config::new();
        config.consume_fuel(true).collector(Collector::Copying);
        let engine = Engine::new(&config).unwrap();
        for ts in [false, true] {
            let artifact = grunkalunka_compiler::compile(source, ts).unwrap();
            let module = Module::new(&engine, rooted_function_carrier(&artifact.wat)).unwrap();
            let mut store = Store::new(&engine, ());
            store.set_fuel(100_000).unwrap();
            let instance = Instance::new(&mut store, &module, &[]).unwrap();
            let run = instance
                .get_typed_func::<(f64, f64), f64>(&mut store, "run")
                .unwrap();
            let read = instance
                .get_typed_func::<(), f64>(&mut store, "read_saved")
                .unwrap();
            let kind = instance
                .get_typed_func::<(), i32>(&mut store, "saved_kind")
                .unwrap();
            let clear = instance
                .get_typed_func::<(), ()>(&mut store, "clear_saved")
                .unwrap();
            for inputs in [
                (0.0, -0.0),
                (1.0, 7.0),
                (f64::NAN, f64::NAN),
                (f64::INFINITY, f64::NEG_INFINITY),
            ] {
                store.set_fuel(100_000).unwrap();
                let expected = run.call(&mut store, inputs).unwrap();
                for _ in 0..4 {
                    store.gc(None).unwrap();
                    assert_eq!(kind.call(&mut store, ()).unwrap(), 6);
                    assert_eq!(
                        read.call(&mut store, ()).unwrap().to_bits(),
                        expected.to_bits()
                    );
                }
                clear.call(&mut store, ()).unwrap();
                store.gc(None).unwrap();
                assert!(read.call(&mut store, ()).is_err());
            }
        }
    }
}
