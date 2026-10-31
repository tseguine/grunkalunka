#[path = "managed/common.rs"]
mod common;

#[test]
fn source_closures_capture_managed_bindings_instead_of_value_copies() {
    let js = "export function run(a,b){let o={x:a};const old=o;let f=()=>o===old?a:b;o={x:b};return f();}";
    common::compare_sources(js, js);
}

#[test]
fn managed_captures_preserve_aliases_properties_and_saved_values() {
    for body in [
        "const o={x:a};const alias=o;const old=o.x;let f=()=>{o.x=b;return 0;};f();return alias===o&&old===a&&o.x===b?a:b;",
        "let x=[a];const old=x;let f=()=>{x=[b];return old[0]===a?a:b;};const r=f();return x!==old&&x[0]===b?r:b;",
        "let x=[a];let f=()=>{const old=x[0];x[0]=b;return old===a&&x[0]===b?a:b;};return f();",
        "const x=[];x[0]=x;let f=()=>x[0]===x?a:b;return f();",
        "let text='old';let f=()=>text==='old'?a:b;text='new';return f();",
        "let text='old';let f=()=>{text='new';return 0;};f();return text==='new'?a:b;",
        "const o={x:a};let v=o.x;let f=()=>v===a?a:b;v='changed';return f();",
        "let o={x:a};const old=o;let set=()=>{o={x:b};return o===old?1:0;};set();return o===old?a:b;",
        "let o={x:a};const old=o;const choose=()=>{o={x:b};return a;};old.x=choose();return old!==o&&old.x===a?a:b;",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&js, &js);
    }
}
#[test]
fn selected_closures_share_numeric_general_and_managed_cells() {
    for body in [
        "let o={x:a};const old=o;let f=a?()=>o===old?a:b:()=>o!==old?b:a;o={x:b};return f();",
        "const o={x:a};const read=()=>o.x===b?a:b;const write=()=>{o.x=b;return 0;};write();return read();",
        "let n=a;const o={x:b};const one=()=>{n++;o.x=n;return 0;};const two=()=>{n--;return n;};let f=a?one:two;f();return o.x===n?a:b;",
        "let p=null;const o={};let f=a?()=>{p=true;return o===o?1:0;}:()=>{p=false;return o===o?1:0;};f();return p===true?a:b;",
        "let text='old';const before=text;const f=a?()=>text==='old'?a:b:()=>text==='new'?b:a;text='new';return f()+(before==='old'?1:0);",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&js, &js);
    }
}
#[test]
fn returned_closures_keep_managed_bindings_after_the_factory_returns() {
    for body in [
        "let make=n=>{const o={x:n};return ()=>o.x===n?a:b;};const f=make(a);const g=make(b);return f()+g();",
        "const o={x:a};let make=n=>()=>{o.x=n;return o.x===n?a:b;};const f=make(a);const g=make(b);return f()+g()+f();",
        "let make=n=>{let text='old';return ()=>{const old=text;text='new';return old==='old'?n:b;};};const f=make(a);return f()+f();",
        "let make=n=>{const x=[n];return ()=>x[0]===n?a:b;};const f=make(a);const g=make(b);return f()+g()+f();",
        "const factory=a?n=>{const o={x:n};return ()=>o.x===n?a:b;}:n=>{const x=[n];return ()=>x[0]===n?b:a;};const f=factory(a);return f()+f();",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&js, &js);
    }
}
#[test]
fn managed_closures_pass_through_callbacks_and_preserve_call_effects() {
    for body in [
        "const o={x:a};const f=()=>o.x===b?a:b;const apply=(fn,x)=>{o.x=x;return fn();};return apply(f,b);",
        "const o={x:a};const apply=a?(fn,x)=>{o.x=x;return fn();}:(fn,x)=>{o.x=x;return fn();};return apply(()=>o.x===b?a:b,b);",
        "const x=[a];let f=()=>x[0]===a?a:b;const invoke=fn=>fn();const old=f;f=()=>x[0]===b?b:a;x[0]=b;return invoke(old)+invoke(f);",
        "let o={};let f=()=>o===o?a:b;return f((o={},a),(o={},b));",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&js, &js);
    }
}
#[test]
fn managed_loop_cells_and_type_metadata_keep_source_semantics() {
    for body in [
        "let first=()=>a;let last=()=>b;for(let i=0;i<3;i++){const o={x:i};if(i===0)first=()=>o.x===0?a:b;last=()=>o.x===2?b:a;}return first()+last();",
        "const o={x:a};let f=()=>o.x===a?a:b;{const o={x:b};const g=()=>o.x===b?b:a;f();g();}return f();",
        "const o={x:a};const f=()=>{const o={x:b};return o.x===b?b:a;};return f()+(o.x===a?a:b);",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&js, &js);
    }
    common::compare_sources(
        "export function run(a,b){let o={x:a};let f=()=>o.x===a?a:b;o={x:b};return f();}",
        "export function run(a:string,b:boolean):void{let o:number={x:a};let f=():object=>o.x===(a as number)?a:b;o=({x:b} satisfies string);return (f as Function)();}",
    );
}

#[test]
fn original_capture_guards_now_have_positive_source_execution() {
    for body in [
        "const o={};const f=()=>o?a:b;return f();",
        "const x=[a];const f=()=>x.length;return f();",
        "const x=[a];const f=()=>()=>x.length;return f()();",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&js, &js);
    }
}
#[test]
fn managed_closure_arguments_results_and_async_frames_remain_explicit_gaps() {
    for body in [
        "const o={x:a};let f=v=>v.x===a?a:b;return f(o);",
        "const o={};let f=()=>o;return f()===o?a:b;",
        "const x=[];let f=()=>x;return f().length;",
        "const text='x';let f=()=>text;return f()==='x'?a:b;",
        "const o={};const f=()=>{o.toString();return a;};return a;",
        "const o={};const f=()=>{o.x=Symbol();return a;};return a;",
        "const o={};const f=()=>{o.x=1n;return a;};return a;",
        "const o={};const f=()=>{if(0)o[b];return a;};return a;",
        "let o={};const f=()=>{o=a;return b;};return f();",
        "const o={};const f=()=>{o={};return a;};return f();",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            let e = grunkalunka_compiler::compile(&js, ts)
                .err()
                .unwrap_or_else(|| panic!("admitted {js}"));
            assert!(e.contains("unsupported"), "{js}: {e}");
            assert!(!e.contains("internal IR"), "{js}: {e}");
        }
    }
    for js in [
        "export async function run(a,b){const o={};let f=()=>o===o?a:b;await 0;return f();}",
        "export async function run(a,b){let make=n=>{const x=[n];return ()=>x.length;};const f=make(a);await 0;return f();}",
    ] {
        for ts in [false, true] {
            let e = grunkalunka_compiler::compile_async(js, ts).err().unwrap();
            assert!(e.contains("managed") || e.contains("GC"), "{e}");
        }
    }
}

// Keep an actual generated closure alive after its defining invocation returns.
// This is a test-only observation/root, not a public reference ABI or host heap.
fn rooted_closure_observer(wat: &str) -> Vec<u8> {
    assert!(wat.contains("$gc_frame"));
    let lines = wat.lines().collect::<Vec<_>>();
    let mut result = String::new();
    let mut copies = 0;
    for (index, line) in lines.iter().enumerate() {
        result.push_str(line);
        result.push('\n');
        if index >= 2
            && lines[index - 2] == "array.set $gc_frame"
            && lines[index - 1].starts_with("i32.const ")
            && line.starts_with("local.get ")
        {
            // Preserve the code/environment pair on the operand stack while
            // copying its two lanes into test-only traced roots.
            result.push_str("global.set $test_env\nglobal.set $test_code\nglobal.get $test_code\nglobal.get $test_env\n");
            copies += 1;
        }
    }
    assert!(
        copies > 0,
        "test source must construct a captured runtime closure"
    );
    let end = result.rfind(')').unwrap();
    result.truncate(end);
    result.push_str(
        r#"
      (global $test_env (mut (ref null $gc_frame)) (ref.null $gc_frame))
      (global $test_code (mut i32) (i32.const -1))
      (func (export "read_saved") (result f64)
        global.get $test_env i32.const 0 array.new_default $gc_frame
        global.get $test_code call_indirect (type $closure))
      (func (export "clear_saved")
        ref.null $gc_frame global.set $test_env i32.const -1 global.set $test_code))"#,
    );
    wat::parse_str(result).unwrap()
}
#[test]
fn actual_source_closures_survive_copying_collection_after_defining_calls_return() {
    use wasmtime::{Collector, Config, Engine, Instance, Module, Store};
    let mut config = Config::new();
    config.consume_fuel(true).collector(Collector::Copying);
    let engine = Engine::new(&config).unwrap();
    for source in [
        "export function run(a,b){const o={x:a};const old=o.x;let f=a?()=>o.x===b&&old===a?1:0:()=>o.x===b&&old===a?2:0;o.x=b;return f();}",
        "export function run(a,b){let make=n=>{const o={x:n};return ()=>o.x===n?1:0;};const f=make(a);return f();}",
        "export function run(a,b){const x=[];x[0]=x;let text='old';const old=text;let f=a?()=>x[0]===x&&text==='new'&&old==='old'?1:0:()=>x[0]===x&&text==='new'&&old==='old'?2:0;text='new';return f();}",
    ] {
        common::compare_sources(source, source);
        for ts in [false, true] {
            let artifact = grunkalunka_compiler::compile(source, ts).unwrap();
            let module = Module::new(&engine, rooted_closure_observer(&artifact.wat)).unwrap();
            let mut store = Store::new(&engine, ());
            store.set_fuel(100_000).unwrap();
            let instance = Instance::new(&mut store, &module, &[]).unwrap();
            let run = instance
                .get_typed_func::<(f64, f64), f64>(&mut store, "run")
                .unwrap();
            let read = instance
                .get_typed_func::<(), f64>(&mut store, "read_saved")
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
                    let actual = read.call(&mut store, ()).unwrap();
                    assert_eq!(actual.to_bits(), expected.to_bits(), "{source}");
                }
                clear.call(&mut store, ()).unwrap();
                store.gc(None).unwrap();
                assert!(
                    read.call(&mut store, ()).is_err(),
                    "cleared test root must not expose a closure"
                );
            }
        }
    }
}

#[test]
fn captured_references_keep_ordinary_helper_transport() {
    common::compare_sources(
        "function set(o,v){o.x=v;return o;}export function run(a,b){const o={x:a};let f=()=>set(o,b)===o?a:b;return f();}",
        "function set(o:number,v:string):boolean{o.x=v;return o;}export function run(a:boolean,b:string):void{const o={x:a};let f=()=>set(o,b)===o?a:b;return f();}",
    );
    for js in [
        "function check(o,n){let f=n?()=>o.x===n?1:0:()=>o.x===n?2:0;return f();}export function run(a,b){const o={x:a};return check(o,a);}",
        "function check(o,n){const old=o;let f=()=>{o={x:n};return 0;};f();return o!==old?1:0;}export function run(a,b){const o={x:a};return check(o,b)+(o.x===a?1:0);}",
    ] {
        common::compare_sources(js, js);
    }
}
#[test]
fn managed_capture_source_budgets_keep_small_stack_diagnostics() {
    for (depth, admitted) in [(16, true), (140, false)] {
        let source = format!(
            "export function run(a,b){{const o={{x:a}};let f=()=>{}(o.x===a?a:b){};return f();}}",
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
        if let Err(e) = result {
            assert!(e.contains("nesting"), "{e}");
        }
    }
}

#[test]
fn captured_managed_values_and_functions_keep_type_inspection() {
    for body in [
        "const o={x:a};let f=()=>typeof o.x==='number'?a:b;return f();",
        "let text='';const o={};let f=()=>typeof o==='object'&&(text||'fallback')==='fallback'?a:b;text='new';return f();",
        "const o={};let f=()=>o===o?a:b;let g=()=>typeof f==='function'?f():b;return g();",
    ] {
        let js = format!("export function run(a,b){{{body}}}");
        common::compare_sources(&js, &js);
    }
}
