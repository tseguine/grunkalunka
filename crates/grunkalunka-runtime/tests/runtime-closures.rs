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
    compare_functions(&bodies);
}

#[test]
fn mutable_functions_keep_alias_snapshots_and_shared_function_cells() {
    compare_functions(&[
        "let f=()=>a; return f();",
        "let f=a?()=>a:()=>b;return f();",
        "const make=x=>()=>++x;let f=make(a);return f();",
        "let f=()=>a;f=()=>b;return f();",
        "let f=a?()=>a:()=>b;const alias=f;f=()=>b;return f()*10+alias();",
        "let f=x=>x+a;return f((f=x=>x-b,a));",
        "let f=(x,y)=>a+x+y;return f((f=(x,y)=>b-x-y,a++),a++);",
        "let f=()=>a;let g=()=>b;f=g=()=>a+b;return f()*10+g();",
        "let f=()=>a;const g=(f=()=>b);f=()=>a;return g()*10+f();",
        "let f=()=>a;a?(f=()=>b):(f=()=>a+b);return f();",
        "let f=()=>a;f=a?()=>a+b:()=>a-b;return f();",
        "let f=()=>a;const g=a?(f=()=>a+b):(f=()=>a-b);return f()+g();",
        "let f=()=>a;f=(a++,f=()=>b,()=>a);return f();",
        "let f=()=>a;const read=()=>f();const set=()=>{f=()=>b;return 1;};read();set();return read();",
        "let f=()=>a;const read=a?()=>f():()=>f()+1;f=()=>b;return read();",
        "let f=()=>a;const set=a?()=>{f=()=>b;return f();}:()=>{f=()=>a+b;return f();};set();return f();",
        "const make=x=>()=>++x;let f=make(a);const alias=f;f=make(b);return f()*100+alias()*10+f();",
        "let f=()=>a;for(let i=0;i<3;i++){let x=i;f=()=>x; a+=f();}return a+f();",
        "let f=()=>a;let x=f();f=()=>b;x=f();return x;",
        "let f=()=>a;let x=f();x=true;f=()=>+x;return f();",
        "let f=()=>a;{let f=()=>b;f=()=>a+b;a+=f();}return f();",
        "let f=()=>a;const alias=f;f=alias;return f();",
        "let f=x=>x;const mutate=()=>{f=x=>x+b;return a;};return f(mutate())+f(a);",
        "let f=()=>a;const g=(f=()=>b);let x=g();x|=1;return x;",
        "let f=x=>x;f=x=>{if(x<=0)return b;return x+f(x-1);};return f(4);",
        "let f=x=>x;let g=x=>x;f=x=>{if(x<=0)return b;return x+g(x-1);};g=x=>{if(x<=0)return a;return x+f(x-1);};return f(4);",
    ]);
    let source = "export function run(a,b){let f=x=>a+x;const old=f;const before=f((f=x=>b-x,3));return before*100+f(3)*10+old(1);}";
    let artifact = compile(source, false).unwrap();
    let mut host = NumericInstance::new(&artifact.wasm).unwrap();
    assert_eq!(host.run(4.0, 2.0).unwrap(), 695.0);
    assert_eq!(host.run(0.0, 2.0).unwrap(), 291.0);
}

#[test]
fn known_local_functions_receive_callback_pairs_and_keep_argument_snapshots() {
    compare_functions(&[
        "const apply=(fn,x)=>fn(x);return apply(x=>x+a,b);",
        "const apply=fn=>fn();return apply(()=>a);",
        "const apply=(fn,x)=>fn(x);const f=x=>x+a;return apply(f,b);",
        "const apply=(fn,x)=>fn(x);let f=x=>x+a;return apply(f,(f=x=>x-b,a))+f(a);",
        "const apply=(fn,x)=>{const alias=fn;return alias(x);};return apply(x=>x+a,b);",
        "const apply=(fn,x)=>{let alias=fn;alias=x=>x-b;return alias(x)+fn(x);};return apply(x=>x+a,b);",
        "const apply=(fn,x)=>{fn=x=>x-b;return fn(x);};let f=x=>x+a;return apply(f,b)+f(a);",
        "const apply=(fn,x)=>{let y=fn(x);y|=1;return y;};return apply(x=>x+a,b);",
        "const apply=(fn,x)=>fn(x);const relay=(g,y)=>apply(g,y);return relay(x=>x+a,b);",
        "const apply=(fn,x)=>fn(x);const alias=apply;const relay=(g,y)=>alias(g,y);return relay(x=>x+a,b);",
        "const apply=(fn,x)=>fn(x);return apply(a?x=>x+a:x=>x-b,b);",
        "const apply=(fn,x)=>{const call=()=>fn(x);return call();};return apply(x=>x+a,b);",
        "const apply=(fn,x)=>{let y=0;for(let i=0;i<3;i++)y+=fn(x+i);return y;};return apply(x=>x+a,b);",
        "const make=fn=>x=>fn(x);const f=make(x=>x+a);return f(b);",
        "const make=fn=>()=>fn();const f=a?make(()=>a):make(()=>b);return f();",
        "const combine=(f,g,x)=>f()+g(x);return combine(()=>a,x=>x+b,a);",
        "let f=()=>a;const unused=(g,x)=>x;return unused(f,a)+f();",
        "const apply=(fn,x)=>{const other=fn=>fn+x;return fn(x)+other(x);};return apply(x=>x+a,b);",
        "const apply=(fn,x)=>{const g=a?fn:(x=>x-b);return g(x);};return apply(x=>x+a,b);",
        "const apply=(fn,x)=>{let g=x=>x;g=fn;return g(x);};return apply(x=>x+a,b);",
        "const identity=fn=>fn;const apply=(fn,x)=>fn(x);return apply(x=>identity(x)+a,b);",
    ]);
}

#[test]
fn local_callback_constraints_reject_conflicting_kinds_and_runtime_signatures() {
    for body in [
        "const apply=(fn,x)=>fn(x);return apply(a,b);",
        "const apply=(fn,x)=>fn(x);return apply(()=>a,b);",
        "const apply=fn=>fn();return apply(x=>x);",
        "const apply=fn=>fn(a)+fn(a,b);return a;",
        "const apply=fn=>{const alias=fn;return alias(a)+fn(a,b);};return a;",
        "const apply=(fn,x)=>fn(x);const relay=f=>apply(f,a)+f(a,b);return a;",
        "const apply=(fn,x)=>fn(x);return apply(async x=>x,b);",
        "const apply=(fn,x)=>fn(x);return apply(x=>true,b);",
        "const apply=fn=>{fn=a;return fn();};return apply(()=>a);",
        "const apply=fn=>fn;return apply(()=>a);",
        "const apply=(fn,x)=>fn(x);const f=a?apply:apply;return f(x=>x,b);",
        "let apply=(fn,x)=>fn(x);return apply(x=>x,b);",
        "const apply=(fn,x)=>fn?.(x);return a;",
        "const apply=(fn,x)=>fn(...x);return a;",
        "const apply=(fn,x)=>fn(x.bad);return a;",
    ] {
        for typescript in [false, true] {
            let source = format!("export function run(a,b){{{body}}}");
            assert!(compile(&source, typescript).is_err(), "{source}");
            assert!(compile_value(&source, typescript).is_err(), "{source}");
            let source = source.replace("export function", "export async function");
            assert!(compile_async(&source, typescript).is_err(), "{source}");
        }
    }
}

#[test]
fn local_callback_arguments_keep_wide_logical_lists_on_the_existing_abi() {
    let parameters = (0..1000)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let arguments = vec!["a"; 1000].join(",");
    let body = format!("const apply=fn=>fn({arguments});return apply(({parameters})=>p0+p999+b);");
    compare_functions(&[&body]);
    let parameters = (0..999)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let arguments = vec!["a"; 999].join(",");
    let body = format!("const apply=(fn,{parameters})=>fn(p998);return apply(x=>x+b,{arguments});");
    compare_functions(&[&body]);
    let source = format!(
        "export function run(a,b){{const apply=fn=>fn({});return a;}}",
        vec!["a"; 1001].join(",")
    );
    assert!(
        compile(&source, false)
            .err()
            .unwrap()
            .contains("at most 1000")
    );
}

#[test]
fn callback_definition_and_specialization_keep_original_resource_limits() {
    for (wrappers, admitted) in [(127, true), (128, false)] {
        let source = format!(
            "export function run(a,b){{const apply=fn=>fn();return apply({}()=>a{});}}",
            "(".repeat(wrappers),
            ")".repeat(wrappers)
        );
        assert_eq!(
            compile(&source, false).is_ok(),
            admitted,
            "{wrappers} callback wrappers"
        );
    }
    for (locals, admitted) in [(127, true), (128, false)] {
        let declarations = (0..locals)
            .map(|i| format!("let v{i}=a;"))
            .collect::<String>();
        let source = format!(
            "export function run(a,b){{const apply=(fn,x)=>{{{declarations}return fn(x);}};return apply(x=>x+b,a);}}"
        );
        assert_eq!(
            compile(&source, false).is_ok(),
            admitted,
            "{locals} body locals"
        );
    }
    for (statements, admitted) in [(253, true), (254, false)] {
        let source = format!(
            "export function run(a,b){{const apply=(fn,x)=>{{{}return fn(x);}};return apply(x=>x+b,a);}}",
            "a=a;".repeat(statements)
        );
        assert_eq!(
            compile(&source, false).is_ok(),
            admitted,
            "{statements} body statements"
        );
    }
    for (calls, admitted) in [(85, true), (86, false)] {
        let source = format!(
            "export function run(a,b){{const apply=fn=>fn();return({});}}",
            vec!["apply(()=>a)"; calls].join(",")
        );
        assert_eq!(
            compile(&source, false).is_ok(),
            admitted,
            "{calls} expanded callback calls"
        );
    }
    for (locals, admitted) in [(128, true), (129, false)] {
        let mut declarations = String::from("const f0=fn=>fn();");
        for i in 1..locals {
            declarations.push_str(&format!("const f{i}=fn=>f{}(fn);", i - 1));
        }
        let source = format!(
            "export function run(a,b){{{declarations}return f{}(()=>a);}}",
            locals - 1
        );
        let result = compile(&source, false);
        assert_eq!(
            result.is_ok(),
            admitted,
            "{locals} forwarding helpers: {:?}",
            result.as_ref().err()
        );
        if admitted {
            assert_eq!(
                NumericInstance::new(&result.unwrap().wasm)
                    .unwrap()
                    .run(4.0, 2.0)
                    .unwrap(),
                4.0
            );
        }
    }
}

#[test]
fn nested_callback_argument_constructors_have_an_observed_small_stack_bound() {
    let mut outcomes = Vec::new();
    for depth in 0..10 {
        let mut callback = String::from("()=>a");
        for _ in 0..depth {
            callback = format!("()=>apply({callback})");
        }
        let source =
            format!("export function run(a,b){{const apply=fn=>fn();return apply({callback});}}");
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        outcomes.push(result.is_ok());
        if let Ok(artifact) = result {
            assert_eq!(
                NumericInstance::new(&artifact.wasm)
                    .unwrap()
                    .run(4.0, 2.0)
                    .unwrap(),
                4.0
            );
        }
    }
    assert!(outcomes[0] && outcomes[1]);
    assert!(!outcomes[9]);
    println!("callback argument construction on 2MiB stack: {outcomes:?}");
}

fn compare_functions(bodies: &[&str]) {
    let inputs = [
        (4.0, 2.0),
        (0.0, 2.0),
        (-0.0, -2.0),
        (f64::NAN, f64::INFINITY),
    ];
    let mut script = String::from(
        "const assert=require('node:assert/strict');function bits(x){if(Number.isNaN(x))return 'nan';const v=new DataView(new ArrayBuffer(8));v.setFloat64(0,x,true);return v.getBigUint64(0,true).toString(16).padStart(16,'0');}\n",
    );
    for &body in bodies {
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
fn mutable_function_slots_require_function_initializers_same_arity_and_plain_writes() {
    for body in [
        "let f;f=()=>a;return f();",
        "let f=a;f=()=>b;return f();",
        "let f=()=>a;f=b;return a;",
        "let f=()=>a;f=null;return a;",
        "let f=()=>a;f=true;return a;",
        "let f=()=>a;f=x=>x;return a;",
        "let f=x=>x;let g=()=>a;f=g;return a;",
        "let f=()=>a;f=()=>true;return a;",
        "let f=()=>a;f+=()=>b;return a;",
        "let f=()=>a;f&&=()=>b;return a;",
        "let f=()=>a;f||=()=>b;return a;",
        "let f=()=>a;f??=()=>b;return a;",
        "let f=()=>a;++f;return a;",
        "let f=()=>a;f--;return a;",
        "const f=a?()=>a:()=>b;f=()=>b;return a;",
        "let f=()=>a;return +(f=()=>b);",
        "let f=()=>a;return f=()=>b;",
        "let f=()=>f();return a;",
    ] {
        for typescript in [false, true] {
            let source = format!("export function run(a,b){{{body}}}");
            assert!(compile(&source, typescript).is_err(), "{source}");
            assert!(compile_value(&source, typescript).is_err(), "{source}");
            let asynchronous = source.replace("export function", "export async function");
            assert!(
                compile_async(&asynchronous, typescript).is_err(),
                "{asynchronous}"
            );
        }
    }
}

#[test]
fn recursive_function_rebinding_respects_fuel_and_next_invocation_reset() {
    let source = "export function run(a,b){let f=()=>b;if(a)f=()=>f();return f();}";
    let artifact = compile(source, false).unwrap();
    let mut host = NumericInstance::new(&artifact.wasm).unwrap();
    let error = host.run(1.0, 2.0).unwrap_err();
    assert_eq!(error.downcast_ref::<Trap>(), Some(&Trap::OutOfFuel));
    assert_eq!(host.run(0.0, 2.0).unwrap(), 2.0);
    assert_eq!(host.run(0.0, 3.0).unwrap(), 3.0);
}

#[test]
fn function_assignment_cursor_crossings_are_bounded_on_a_small_native_stack() {
    let mut outcomes = Vec::new();
    for depth in [0, 1, 6, 12, 15, 16, 30] {
        let mut value = String::from("()=>a");
        for _ in 0..depth {
            value = format!("(f={value},()=>b)");
        }
        let source = format!("export function run(a,b){{let f=()=>a;f={value};return f();}}");
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        outcomes.push(result.is_ok());
        if depth <= 15 {
            let artifact = result.unwrap_or_else(|error| panic!("depth{depth}: {error}"));
            assert_eq!(
                NumericInstance::new(&artifact.wasm)
                    .unwrap()
                    .run(4.0, 2.0)
                    .unwrap(),
                if depth == 0 { 4.0 } else { 2.0 }
            );
        } else {
            assert!(
                result
                    .err()
                    .unwrap()
                    .contains("16 active function-value cursors")
            );
        }
    }
    println!("function assignment cursor admission on 2MiB stack: {outcomes:?}");
}

#[test]
fn function_assignment_keeps_source_depth_body_costs_and_generation_budgets() {
    for (wrappers, admitted) in [(127, true), (128, false)] {
        let source = format!(
            "export function run(a,b){{let f=()=>a;f={}()=>b{};return f();}}",
            "(".repeat(wrappers),
            ")".repeat(wrappers)
        );
        assert_eq!(
            compile(&source, false).is_ok(),
            admitted,
            "{wrappers} assignment wrappers"
        );
    }
    for (wrappers, admitted) in [(124, true), (125, false)] {
        let source = format!(
            "export function run(a,b){{let f=x=>x;return f((f={}x=>x+b{},a));}}",
            "(".repeat(wrappers),
            ")".repeat(wrappers)
        );
        assert_eq!(
            compile(&source, false).is_ok(),
            admitted,
            "{wrappers} nested argument wrappers"
        );
    }
    for (assignments, admitted) in [(128, true), (129, false)] {
        let source = format!(
            "export function run(a,b){{let f=()=>a;{}()=>b;return f();}}",
            "f=".repeat(assignments)
        );
        assert_eq!(
            compile(&source, false).is_ok(),
            admitted,
            "{assignments} assignment chain"
        );
    }
    for (locals, admitted) in [(127, true), (128, false)] {
        let declarations = (0..locals)
            .map(|i| format!("let v{i}=a;"))
            .collect::<String>();
        let source = format!(
            "export function run(a,b){{let f=()=>a;f=()=>{{{declarations}return a;}};return f();}}"
        );
        assert_eq!(
            compile(&source, false).is_ok(),
            admitted,
            "{locals} body locals"
        );
    }
    for (statements, admitted) in [(252, true), (253, false)] {
        let source = format!(
            "export function run(a,b){{let f=()=>a;f=()=>{{{}return a;}};return f();}}",
            "a=a;".repeat(statements)
        );
        assert_eq!(
            compile(&source, false).is_ok(),
            admitted,
            "{statements} body statements"
        );
    }
    for (calls, admitted) in [(254, true), (255, false)] {
        let source = format!(
            "export function run(a,b){{let f=()=>a;f=()=>b;return({});}}",
            vec!["f()"; calls].join(",")
        );
        assert_eq!(
            compile(&source, false).is_ok(),
            admitted,
            "{calls} generated calls"
        );
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
