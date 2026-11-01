#[path = "managed/common.rs"]
mod common;

#[test]
fn source_object_factory_results_keep_identity_and_properties() {
    let js = "export function run(a,b){let f=()=>({x:a});return f().x===a?a:b;}";
    common::compare_sources(js, js);
}

#[test]
fn local_and_selected_results_preserve_references_and_value_kinds() {
    for body in [
        "const f=()=>({x:a});return f().x===a?a:b;",
        "let o={x:a};let f=()=>o;const old=f();o={x:b};return old!==f()&&old.x===a?a:b;",
        "const f=()=>[a,b];const x=f();return x[0]===a&&x.length===2?a:b;",
        "let f=()=> 'text';return f()==='text'?a:b;",
        "const f=a?()=>({x:a}):()=>({x:b});return f().x===a?a:b;",
        "const f=a?()=>({x:a}):()=>b;return typeof f()==='object'?a:b;",
        "let f=()=>typeof a;return f()==='number'?a:b;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&js, &js);
    }
}
#[test]
fn returned_managed_parameters_flow_through_callbacks_and_factories() {
    for body in [
        "const o={x:a};let f=x=>x;return f(o)===o?a:b;",
        "const f=x=>x;return f('text')==='text'?a:b;",
        "const apply=fn=>fn();return apply(()=>({x:a})).x===a?a:b;",
        "const apply=(fn,x)=>fn(x);const o={x:a};return apply(x=>x,o)===o?a:b;",
        "const factory=a?()=>()=>({x:a}):()=>()=>({x:b});const f=factory();return f().x===a?a:b;",
        "let make=n=>()=>({x:n});const f=make(a);const old=f();return old!==f()&&old.x===a?a:b;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&js, &js);
    }
}
#[test]
fn managed_block_results_keep_fallthrough_effects_and_cycles() {
    for body in [
        "let f=n=>{return {x:n};};return f(a).x===a?a:b;",
        "let f=n=>{if(n)return {x:b};};return typeof f(a)==='undefined'?a:b;",
        "let f=n=>{if(n)return;return {x:b};};return typeof f(a)==='undefined'?a:b;",
        "let n=0;const o={x:a};let f=()=>{n++;return o;};const old=f().x;return n===1&&old===a?a:b;",
        "let f=()=>{const x=[];x[0]=x;return x;};const x=f();return x[0]===x?a:b;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&js, &js);
    }
}

#[test]
fn former_result_guards_have_positive_execution_and_annotations_erase() {
    for body in [
        "const o={};let f=()=>o;return f()===o?a:b;",
        "const x=[];let f=()=>x;return f().length;",
        "const text='x';let f=()=>text;return f()==='x'?a:b;",
        "let f=v=>v;return f({})===null?a:b;",
        "const x='v'||'fallback';const f=()=>x;return a;",
        "const f=()=>{return '';};return a;",
        "const f=()=>{return {};};return a;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&js, &js);
    }
    common::compare_sources(
        "export function run(a,b){const f=n=>({x:n});return f(a).x===a?a:b;}",
        "export function run(a:boolean,b:string):void{const f=(n:string):number=>({x:n});return f(a).x===a?a:b;}",
    );
}
#[test]
fn managed_returns_keep_call_snapshots_method_effects_and_iteration_cells() {
    for body in [
        "let n=0;let make=x=>({x:x});const o=make.call((n++,a),b);return o.x===b&&n===1?a:b;",
        "let f=()=>({x:a});const g=()=>({x:b});const o=f((f=g,b));return o.x===a?a:b;",
        "let f=()=>({x:a});const get=()=>f;const old=get();f=()=>({x:b});return old().x===a?a:b;",
        "let first=()=>({x:a});let last=()=>({x:b});for(let i=0;i<3;i++){const n=i;const make=()=>({x:n});if(i===0)first=make;last=make;}return first().x===0&&last().x===2?a:b;",
        "let f=n=>n?'text':null;return f(a)==='text'?a:b;",
        "let f=n=>{if(n)return 'text';return null;};return f(a)==='text'?a:b;",
        "let f=n=>n?{x:b}:a;return f(b)?a:b;",
        "const o={x:a};let f=v=>{const old=v;v={x:b};return old;};return f(o)===o?a:b;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&js, &js);
    }
}
#[test]
fn managed_result_boundaries_reject_unsupported_uses_in_unused_bodies_too() {
    for source in [
        "export function run(a,b){const f=()=>({});return f();}",
        "export function run(a,b){const f=()=>({});return f()+a;}",
        "export function run(a,b){const f=()=>({x:Symbol()});return a;}",
        "export function run(a,b){const f=()=>({x:()=>a});return a;}",
        "export function run(a,b){const f=()=>({x:a});return f()[b];}",
        "export function run(a,b){const f=()=>({});return f().toString();}",
        "export async function run(a,b){let f=()=>({x:a});await 0;return f().x===a?a:b;}",
        "export function run(a,b){let f=n=>n?Promise.resolve(a):{x:b};const v=f(a);return typeof v==='object'?a:b;}",
        "export function run(a,b){let f=a?async()=>a:()=>({x:b});return typeof f()==='object'?a:b;}",
    ] {
        for ts in [false, true] {
            let error = grunkalunka_compiler::compile(source, ts)
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
fn managed_return_planning_preserves_native_stack_limits() {
    for (depth, admitted) in [(16, true), (140, false)] {
        let source = format!(
            "export function run(a,b){{let f=()=>{}({{x:a}}){};return f().x===a?a:b;}}",
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

fn rooted_result_observer(original: &str) -> Vec<u8> {
    let marker =
        "call_indirect (type $closure_managed)\nref.cast (ref null $gc_object)\nlocal.set ";
    assert_eq!(
        original.matches(marker).count(),
        1,
        "source must make one managed object-returning private call"
    );
    let start = original.find(marker).unwrap() + marker.len();
    let end = start + original[start..].find('\n').unwrap();
    let local = original[start..end].parse::<usize>().unwrap();
    let mut wat = original.to_owned();
    wat.insert_str(
        end + 1,
        &format!("local.get {local}\nglobal.set $test_result\n"),
    );
    let close = wat.rfind(')').unwrap();
    wat.truncate(close);
    wat.push_str(
        r#"
      (global $test_result (mut (ref null eq)) (ref.null eq))
      (func (export "read_returned") (result f64)
        global.get $test_result ref.cast (ref null $gc_object)
        struct.get $gc_object 0 i32.const 0 array.get $gc_props
        struct.get $gc_entry 1 struct.get $gc_value 1)
      (func (export "returned_cycle") (result i32)
        global.get $test_result ref.cast (ref null $gc_object)
        struct.get $gc_object 0 i32.const 1 array.get $gc_props
        struct.get $gc_entry 1 struct.get $gc_value 2
        global.get $test_result ref.eq)
      (func (export "clear_returned") ref.null eq global.set $test_result))"#,
    );
    wat::parse_str(wat).unwrap()
}
#[test]
fn generated_managed_results_survive_copying_gc_after_defining_calls_return() {
    use wasmtime::{Collector, Config, Engine, Instance, Module, Store};
    let source = "export function run(a,b){let make=n=>{const o={x:n};o.self=o;return o;};const o=make(a);const old=o.x;o.x=b;return old===a&&o.self===o&&o.x===b?a:b;}";
    common::compare_sources(source, source);
    let mut config = Config::new();
    config.consume_fuel(true).collector(Collector::Copying);
    let engine = Engine::new(&config).unwrap();
    for ts in [false, true] {
        let artifact = grunkalunka_compiler::compile(source, ts).unwrap();
        let module = Module::new(&engine, rooted_result_observer(&artifact.wat)).unwrap();
        let mut store = Store::new(&engine, ());
        store.set_fuel(100_000).unwrap();
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        let run = instance
            .get_typed_func::<(f64, f64), f64>(&mut store, "run")
            .unwrap();
        let read = instance
            .get_typed_func::<(), f64>(&mut store, "read_returned")
            .unwrap();
        let cycle = instance
            .get_typed_func::<(), i32>(&mut store, "returned_cycle")
            .unwrap();
        let clear = instance
            .get_typed_func::<(), ()>(&mut store, "clear_returned")
            .unwrap();
        for (a, b) in [
            (0.0, -0.0),
            (1.0, 7.0),
            (f64::NAN, f64::NAN),
            (f64::INFINITY, f64::NEG_INFINITY),
        ] {
            store.set_fuel(100_000).unwrap();
            run.call(&mut store, (a, b)).unwrap();
            for _ in 0..4 {
                store.gc(None).unwrap();
                let actual = read.call(&mut store, ()).unwrap();
                if b.is_nan() {
                    assert!(actual.is_nan());
                } else {
                    assert_eq!(actual.to_bits(), b.to_bits());
                }
                assert_eq!(cycle.call(&mut store, ()).unwrap(), 1);
            }
            clear.call(&mut store, ()).unwrap();
            store.gc(None).unwrap();
            assert!(read.call(&mut store, ()).is_err());
        }
    }
}

#[test]
fn managed_result_header_captures_reject_until_iteration_environments_exist() {
    let source = "export function run(a,b){let first=()=>({x:a});let last=()=>({x:b});for(let i=0;i<3;i++){const make=()=>({x:i});if(i===0)first=make;last=make;}return first().x===0&&last().x===2?a:b;}";
    for ts in [false, true] {
        let error = grunkalunka_compiler::compile(source, ts)
            .err()
            .unwrap_or_else(|| panic!("admitted shared header cells"));
        assert!(
            error.contains("per-iteration environments (#334)") && !error.contains("internal IR"),
            "{error}"
        );
    }
}
