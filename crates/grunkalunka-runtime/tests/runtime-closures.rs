use grunkalunka_compiler::{compile, compile_async, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};
use std::{
    io::Write,
    process::{Command, Stdio},
};
use wasmtime::{Engine, Instance, Module, Store, Trap};

#[path = "runtime-closures/function_length.rs"]
mod function_length;

#[path = "runtime-closures/function_call.rs"]
mod function_call;

#[path = "runtime-closures/discarded_logical.rs"]
mod discarded_logical;

#[path = "runtime-closures/for_let_cells.rs"]
mod for_let_cells;

#[test]
fn eager_async_closures_keep_synchronous_prefix_effects() {
    compare_functions(&[
        "let n=a;const f=a?async x=>(n+=x,x):async x=>(n-=x,x);f(b);return n;",
        "const f=a?async()=>false:async()=>null;return f()?a:b;",
        "const f=a?async()=>false:async()=>null;const p=f();return p?a:b;",
        "let n=0;const f=a?async x=>(n++,x):async x=>(n+=2,x);f(b,n+=4);return n;",
        "let f=async x=>(a++,x);f((f=async x=>(a+=2,x),b));return a;",
        "const make=x=>async()=>x;const f=make(a);f();return b;",
        "const apply=(fn,x)=>fn(x);apply(async x=>(a++,x),b);return a;",
    ]);
}

#[test]
fn unused_eager_async_constructors_preserve_their_original_sources() {
    compare_function_programs(
        &[
            "const f=async()=>a;return a;",
            "let make=x=>{return async()=>x;};return a;",
        ],
        false,
    );
    for body in [
        "const f=async()=>a;return a;",
        "let make=x=>{return async()=>x;};return a;",
    ] {
        for ts in [false, true] {
            let artifact =
                compile_value(&format!("export function run(a,b){{{body}}}"), ts).unwrap();
            let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
            for a in [4.0f64, 0.0, -0.0, f64::NAN] {
                let Value::Number(actual) = instance.run(a, 2.0).unwrap() else {
                    panic!("unused constructor changed Number result");
                };
                assert_eq!(bits(actual), bits(a));
            }
        }
    }
}

#[test]
fn selected_unused_callback_peers_preserve_numeric_inputs() {
    compare_functions(&[
        "const f=a?(p,x,fn,y)=>fn(p):(p,x,fn,y)=>p;return f(b,a,p=>p,false);",
        "const f=a?(fn,x)=>fn(x):(fn,x)=>x;return f(y=>-y,b);",
        "const f=a?(x,fn)=>x:(x,fn)=>fn(x);return f(b,y=>-y);",
        "const f=a?(fn,x)=>{return fn(x);}:(fn,x)=>{return x;};const alias=f;return alias(y=>y+b,a);",
        "let f=(fn,x)=>fn(x);if(a)f=(fn,x)=>x;return f(y=>-y,b);",
        "let f=(fn,x)=>fn(x);const old=f;f=(fn,x)=>x;return old(y=>-y,b)+f(y=>y+a,b);",
        "let f=(fn,x)=>fn(x);return f((f=(fn,x)=>x,y=>y+b),a);",
        "let n=0;const f=a?(fn,x)=>fn(x):(fn,x)=>x;const out=f((n++,y=>(n+=2,y)),b);return out+n;",
        "let n=0;const f=a?(fn,x)=>fn(x):(fn,x)=>x;const out=f(y=>(n++,y),b,(n+=4));return out+n;",
        "const use=(fn,x)=>fn(x),skip=(fn,x)=>x;const f=a?use:skip;return f(y=>-y,b);",
        "const make=()=>a?(fn,x)=>fn(x):(fn,x)=>x;const f=make();return f(y=>-y,b);",
        "const make=()=>{if(a)return (fn,x)=>fn(x);return (fn,x)=>x;};const f=make();return f(y=>-y,b);",
        "const f=a?(fn,x)=>fn(x):(fn,x)=>+x;return f(y=>-y,null);",
        "const f=a?(fn,x)=>fn(x):(fn,x)=>+x;return f(y=>+y,false);",
        "const f=a?(fn,x)=>fn(x):(fn,x)=>x;const relay=(fn,x)=>f(fn,x);return relay(y=>-y,b);",
        "const f=a?(fn,x)=>fn(x):(fn,x)=>x;const id=x=>x;return f(y=>-y,b)+id(a)+id(y=>y)(b);",
        "const f=a?(left,x,right)=>left(x):(left,x,right)=>right(x);return f(y=>y+b,a,y=>y-b);",
        "const f=a?(fn,x)=>fn(x):(fn,x)=>x;return f(y=>y,1/b);",
        "return (a?(fn,x)=>fn(x):(fn,x)=>x)(y=>-y,b);",
        "const make=()=>a?(fn,x)=>fn(x):(fn,x)=>x;return make()(y=>-y,b);",
        "const f=a?(fn,x)=>fn(y=>y,x):(fn,x)=>x;return f((g,y)=>g(y),b);",
        "const f=a?(fn,x)=>fn(x):(fn,x)=>{const read=()=>fn;return x;};return f(y=>-y,b);",
    ]);
}

#[test]
fn callback_return_origin_calls_keep_union_and_binding_semantics_on_small_stack() {
    // Callee, actual arguments and returned-arrow bodies can all contain
    // nested calls. None should consume another native Rust stack frame.
    for (body, expected) in [
        (
            "const make=()=>a?(fn,x)=>fn(x):(fn,x)=>x;const f=make();return f(y=>-y,b);",
            -2.0,
        ),
        (
            "const f=a?(fn,x)=>fn(x):(fn,x)=>x;const relay=(fn,x)=>f(fn,x);return relay(y=>-y,b);",
            -2.0,
        ),
        (
            "const id=fn=>fn;const f=id(id(id(x=>x+b)));return f(a);",
            6.0,
        ),
    ] {
        for typescript in [false, true] {
            let source = format!("export function run(a,b){{{body}}}");
            let artifact = std::thread::Builder::new()
                .name(
                    "callback_return_origin_calls_keep_union_and_binding_semantics_on_small_stack"
                        .to_owned(),
                )
                .stack_size(2 * 1024 * 1024)
                .spawn(move || compile(&source, typescript))
                .unwrap()
                .join()
                .unwrap()
                .unwrap_or_else(|error| panic!("{body}: {error}"));
            assert_eq!(
                NumericInstance::new(&artifact.wasm)
                    .unwrap()
                    .run(4.0, 2.0)
                    .unwrap(),
                expected,
                "{body}",
            );
        }
    }
}

#[test]
fn unused_callback_peers_keep_complete_signature_guards() {
    for body in [
        "const f=a?(fn,x)=>fn(x):(fn,x)=>+fn+x;return f(y=>y,b);",
        "const f=a?(fn,x)=>fn(x):(fn,x)=>{fn=0;return x;};return f(y=>y,b);",
        "const f=a?(fn,x)=>fn(x):(fn,x)=>x;f(y=>y,b);return f(y=>()=>y,b);",
        "const f=a?(fn,x)=>fn(x):(fn,x)=>x;return f(a?y=>y:y=>()=>y,b);",
        "const f=a?(fn,x)=>fn(x):(fn,x)=>x;return f(...a);",
        "const f=a?(fn,x)=>fn(x):(fn,x)=>x;return f(y=>y);",
    ] {
        for ts in [false, true] {
            let source = format!("export function run(a,b){{{body}}}");
            assert!(compile(&source, ts).is_err(), "{source}");
            assert!(compile_value(&source, ts).is_err(), "{source}");
        }
    }
}

#[test]
fn runtime_closures_preserve_primitive_argument_frames() {
    compare_functions(&[
        "const f=a?x=>+x:x=>-x;return f(null);",
        "const f=a?x=>+x:x=>-x;return f(undefined);",
        "const f=a?x=>+x:x=>-x;return f(true);",
        "const f=a?x=>+x:x=>-x;return f(false);",
        "const f=a?x=>+x:x=>-x;f(null);return f(b);",
        "const f=a?(x,y,z)=>+x+y+(+z):(x,y,z)=>-x-y-(+z);return f(true,a,false);",
        "const h=a?(fn,x,z)=>fn(+x)+(+z):(fn,x,z)=>fn(-x)-(+z);return h(y=>y+b,null,true);",
        "const h=a?(x,fn,z)=>fn(+x)+(+z):(x,fn,z)=>fn(-x)-(+z);return h(null,y=>y+b,true);",
        "let x=null;const f=a?(x,y)=>+x+y:(x,y)=>-x-y;return f(x,(x=a,a))+(+x);",
        "let f=x=>+x;return 1/f((f=x=>-x,null));",
        "let f=x=>+x;const old=f;f=x=>-x;return old(true)+f(false);",
        "let f=x=>{const old=+x;x=null;return old+(+x);};return f(a);",
        "const f=a?x=>{const read=()=>+x;const old=read();x=b;return old+read();}:x=>+x;return f(null);",
        "const make=a?x=>()=>+x:x=>()=>-x;return make(null)();",
        "const make=a?x=>{return ()=>+x;}:x=>{return ()=>-x;};return make(true)();",
        "const make=a?()=>x=>+x:()=>x=>-x;const f=make();return f(null);",
        "let make=x=>()=>{const old=+x;x=b;return old;};const f=make(true),g=make(null);return f()*100+f()*10+g();",
        "let f=x=>+x;f(null);const apply=fn=>fn(a);return apply(f);",
        "const f=x=>+x;const g=x=>-x;f(null);let h=a?f:g;return h(b);",
        "const make=()=>x=>+x;const f=make();const g=x=>-x;f(null);let h=a?f:g;return h(b);",
    ]);
}

#[test]
fn runtime_general_frames_retain_width_and_input_guards() {
    let parameters = (0..1000)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let mut inputs = vec!["null"; 1000];
    inputs[999] = "true";
    let arguments = inputs.join(",");
    let body = format!(
        "const f=a?({parameters})=>+p0+(+p999):({parameters})=>+p0+(+p999);return f({arguments});"
    );
    // This is a frame/arity boundary, not a promise about the default fuel quota.
    let mut config = wasmtime::Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config).unwrap();
    for ts in [false, true] {
        let source = format!("export function run(a,b){{{body}}}");
        let artifact = compile(&source, ts).unwrap();
        let module = Module::new(&engine, &artifact.wasm).unwrap();
        let mut store = Store::new(&engine, ());
        store.set_fuel(1_000_000).unwrap();
        let instance = Instance::new(&mut store, &module, &[]).unwrap();
        let run = instance
            .get_typed_func::<(f64, f64), f64>(&mut store, "run")
            .unwrap();
        for (a, b) in [
            (4.0, 2.0),
            (0.0, -0.0),
            (-0.0, -2.0),
            (f64::NAN, f64::INFINITY),
        ] {
            store.set_fuel(1_000_000).unwrap();
            assert_eq!(run.call(&mut store, (a, b)).unwrap(), 1.0);
        }
        node(&format!(
            "const assert=require('node:assert/strict'),native=(a,b)=>{{{body}}},run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from({:?}))).exports.run;for(const[a,b]of[[4,2],[0,-0],[-0,-2],[NaN,Infinity]]){{assert.equal(native(a,b),1);assert.equal(run(a,b),1);}}",
            artifact.wasm
        ));
    }
    for source in [
        format!(
            "export function run(a,b){{let f=({parameters},extra)=>+extra;return f({arguments},true);}}"
        ),
        "export function run(a,b){let f=x=>+x;f(null);return f();}".into(),
        "export function run(a,b){let f=(x=null)=>+x;return f(null);}".into(),
        "export function run(a,b){let f=x=>+x;return f('text');}".into(),
        "export function run(a,b){let f=x=>+x;return f({});}".into(),
        "export function run(a,b){let f=x=>+x;return f([]);}".into(),
        "export function run(a,b){let f=x=>+x;return f(...a);}".into(),
    ] {
        for ts in [false, true] {
            assert!(compile(&source, ts).is_err(), "{source}");
            assert!(compile_value(&source, ts).is_err(), "{source}");
        }
    }
    for ts in [false, true] {
        assert!(compile("export function run(a,b){let f=x=>x;return f(null);}", ts).is_err());
    }
}

#[test]
fn callback_origins_propagate_primitive_input_requirements() {
    compare_functions(&[
        "const apply=(fn,x)=>fn(x);return apply(y=>+y,null);",
        "const apply=(fn,x)=>fn(x);return apply(y=>+y,true);",
        "const apply=(fn,x)=>fn(x);return apply(y=>+y,undefined);",
        "const apply=(fn,x)=>fn(x);const relay=(f,x)=>apply(f,x);return relay(y=>+y,null);",
        "const apply=(fn,x)=>fn(x);const f=y=>+y;const alias=f;return apply(alias,false);",
        "const apply=(fn,x)=>fn(x);return apply(a?y=>+y:y=>-y,null);",
        "const apply=(fn,x)=>{const alias=fn;return alias(x);};return apply(y=>+y,null);",
        "const make=fn=>x=>fn(x);const f=make(y=>+y);return f(null);",
        "let n=a;const apply=(fn,x)=>fn(x);return apply(y=>(n+=b,+y+n),null)+n;",
        "const id=x=>x;const apply=(fn,x)=>fn(x);apply(y=>+y,null);return id(a)+id(x=>x)(b);",
    ]);
}

#[test]
fn computed_call_inputs_retain_number_results() {
    compare_functions(&[
        "let make=x=>()=>x;const id=x=>x;return id(make(a)());",
        "const make=x=>()=>x+b;let f=x=>x;return f(make(a)());",
        "const make=x=>()=>x;const apply=(f,x)=>f(x);return apply(y=>y,make(a)());",
        "const make=x=>()=>x;const id=x=>x;return id((a?make(a):make(b))());",
    ]);
}

#[test]
fn runtime_selection_joins_primitive_input_requirements() {
    compare_functions(&[
        "const f=x=>+x;let p=f;p(null);const g=x=>-x;let h=a?f:g;return h(b);",
        "const f=x=>+x;let p=f;p(undefined);const g=x=>-x;let h=a?f:g;return h(b);",
        "const f=x=>+x;let p=f;p(false);const g=x=>-x;const alias=g;let h=a?f:alias;return h(b);",
        "const f=x=>+x;let p=f;p(null);const g=x=>-x;let h=f??g;return h(b);",
        "const f=x=>+x;let p=f;p(null);const g=x=>-x;let h=f&&g;return h(b);",
        "const f=x=>+x;let p=f;p(null);const g=x=>-x;let h=f;if(a)h=g;return h(b);",
        "const f=x=>+x;let p=f;p(null);const g=x=>-x;const k=x=>x*2;let h=a?f:g;let j=b?g:k;return h(b)+j(a);",
        "const f=(x,y)=>+x+y;let p=f;p(null,b);const g=(x,y)=>-x+y;let h=a?f:g;return h(b,a);",
        "let n=a;const f=x=>(n+=b,+x);let p=f;p(null);const g=x=>(n+=a,-x);let h=a?f:g;return h(b)+n;",
        "let f=x=>+x;let p=f;p(null);const g=x=>-x;let h=a?f:g;return 1/h((h=g,null));",
        "const f=(fn,x)=>fn(x);let p=f;p(y=>+y,null);const g=(fn,x)=>fn(x)-b;let h=a?f:g;return h(y=>+y,b);",
    ]);
}

#[test]
fn immediate_selections_join_private_primitive_inputs() {
    compare_functions(&[
        "const f=x=>+x;let p=f;p(null);const g=x=>-x;return (a?f:g)(b);",
        "const f=x=>+x;let p=f;p(undefined);const g=x=>-x;return (f&&g)(b);",
        "const f=x=>+x;let p=f;p(false);const g=x=>-x;return (f??g)(b);",
        "const f=x=>+x;let p=f;p(null);const g=x=>-x;return (a,(a?f:g))(b);",
        "const f=x=>+x;let p=f;p(null);const g=x=>-x;const id=x=>x;return id(a?f:g)(b);",
        "let n=a;const f=x=>+x;let p=f;p(null);const g=x=>-x;return (a?(n++,f):(n+=b,g))(b)+n;",
        "let f=x=>+x;let p=f;p(null);const g=x=>-x;return 1/(a?f:g)((f=g,null));",
        "const f=(cb,x)=>cb(x);let p=f;p(y=>+y,null);const g=(cb,x)=>cb(x)-b;return (a?f:g)(y=>+y,b);",
    ]);
}

#[test]
fn promise_parameter_calls_preserve_synchronous_prefixes() {
    compare_function_programs(
        &[
            "const consume=p=>{p=Promise.resolve(b,a++);return p;};consume(Promise.resolve(a,a++));return a;",
            "let consume=p=>{p&&=Promise.resolve(b,a++);return p;};consume(Promise.resolve(false));return a;",
            "let consume=p=>{p||=Promise.resolve(b,a++);return p;};consume(Promise.resolve(null));return a;",
            "const id=p=>p;return id(Promise.resolve(false))?a:b;",
            "const apply=(fn,p)=>fn(p);apply(p=>Promise.resolve(p,a++),Promise.resolve(b));return a;",
            "const make=a?p=>()=>p:p=>()=>p;make(Promise.resolve(a,a++))();return a;",
            "let p=Promise.resolve(a);const f=a?(x,y)=>+x+(+y):(x,y)=>+x+(+y);return f(null,(p=Promise.resolve(b),false));",
        ],
        false,
    );
}

#[test]
fn runtime_promise_results_preserve_synchronous_prefixes() {
    compare_function_programs(
        &[
            "let make=x=>Promise.resolve(x,a++);make(a);return a;",
            "const make=a?x=>Promise.resolve(x,a++):x=>Promise.resolve(x,a=b);const p=make(a);return a;",
            "let n=0;const make=a?()=>Promise.resolve((n++,false)):()=>Promise.resolve((n+=2,null));return make()?n:-1;",
            "let p=Promise.resolve(a);let get=()=>p;const old=get();p=Promise.resolve(b);return old&&get()?a:b;",
            "const apply=(fn,x)=>fn(x);apply(x=>Promise.resolve(x,a++),a);return a;",
            "const make=a?()=>x=>Promise.resolve(x,a++):()=>x=>Promise.resolve(x,a=b);make()(b);return a;",
        ],
        false,
    );
}

#[test]
fn known_promise_results_preserve_synchronous_prefixes() {
    compare_function_programs(
        &[
            "const make=x=>Promise.resolve(x,a++);make(a);return a;",
            "let n=0;const make=x=>Promise.resolve((n++,x));const p=make(a);return n;",
            "const make=x=>{const p=Promise.resolve(x,a=b);return p;};const p=make(a);return a;",
            "const make=x=>Promise.resolve(x,a++);return make(a)?a:b;",
            "let p=Promise.resolve(a);const get=()=>p;const q=get();return q?1:0;",
            "const make=x=>{return Promise.resolve(x,a++);};make(a);return a;",
            "let n=0;const make=x=>{n++;return Promise.resolve(x);};return n;",
        ],
        false,
    );
}

#[test]
fn fulfilled_promise_expressions_preserve_synchronous_effects() {
    compare_function_programs(
        &[
            "let p=Promise.resolve(a);p=Promise.resolve(b,a++);return a;",
            "const p=Promise.resolve(a);const q=(a++,p);return a;",
            "let p=Promise.resolve(false);p&&=Promise.resolve(b,a++);return a;",
            "const p=Promise.resolve(a);p||Promise.resolve(b,a++);return a;",
            "Promise.resolve(a,a=b);return a;",
            "let n=0;const f=()=>a;f(Promise.resolve(b,n++));return n;",
            "let p=Promise.resolve(a);const set=()=>{p=Promise.resolve(b,a++);return a;};return set();",
            "const p=Promise.resolve(false);return p?1:0;",
        ],
        false,
    );
}

#[test]
fn fulfilled_promise_arguments_preserve_synchronous_effects() {
    compare_function_programs(
        &[
            "const p=Promise.resolve();return a;",
            "const p=Promise.resolve(a,a=b);return a;",
            "let n=0;const p=Promise.resolve(n++,n=n*10+2,n=n*10+3);return n;",
            "let n=0;const p=Promise.resolve(a,n++);const q=Promise.resolve(p,n++);return n;",
            "const f=()=>{const p=Promise.resolve(a,a=b);return a;};return f();",
        ],
        false,
    );
}

#[test]
fn fulfilled_promise_construction_preserves_synchronous_prefixes() {
    compare_function_programs(
        &[
            "const p=Promise.resolve(a);return b;",
            "let x=a;const p=Promise.resolve(x++);return x;",
            "let n=0;const p=Promise.resolve((n++,a));const q=p;return n;",
            "const f=()=>{const p=Promise.resolve(a);return b;};return f();",
            "const p=Promise.resolve(false);const q=Promise.resolve(p);return a;",
        ],
        false,
    );
}

#[test]
fn lone_factories_infer_unused_returned_constructor_groups() {
    compare_function_programs(
        &[
            "const make=()=>a?()=>null:()=>b;return a;",
            "let make=x=>{if(x)return ()=>a;return ()=>null;};return a;",
            "const make=()=>{if(a)return ()=>null;return ()=>b;};return a;",
            "const make=()=>{const f=()=>null;const g=()=>b;return a?f:g;};return a;",
            "const make=()=> (()=>null)||(()=>b);return a;",
            "const make=()=> (()=>null)??(()=>b);return a;",
            "let n=0;const make=()=>(n++,a?()=>null:()=>b);return n;",
            "const id=x=>x;const make=()=>a?()=>null:()=>b;return id(a)+id(b);",
            "const id=x=>x;const make=()=>a?()=>null:()=>b;const apply=(fn,x)=>fn(x);return apply(id(x=>x+a),b);",
            "const make=()=>a?()=>null:()=>b;return +make()();",
        ],
        false,
    );
}

#[test]
fn unused_nested_factory_results_share_scalar_leaf_interfaces() {
    compare_function_programs(
        &[
            "let make=x=>y=>()=>x+y;make=x=>y=>()=>null;return a;",
            "const one=x=>y=>()=>x+y;const two=x=>y=>()=>null;let make=a?one:two;return a;",
            "let make=x=>y=>()=>x+y;make&&=x=>y=>()=>null;return a;",
            "let make=x=>y=>()=>x+y;make||=x=>y=>()=>null;return a;",
            "let make=x=>y=>()=>x+y;make??=x=>y=>()=>null;return a;",
            "let make=x=>{return y=>{return ()=>x+y;};};make=x=>{return y=>{return ()=>null;};};return a;",
            "const read=()=>a?()=>null:()=>b;const same=()=>read;let make=a?(()=>same):(()=>same);return a;",
            "let n=0;let make=x=>y=>()=>(n++,x+y);make=x=>y=>()=>(n++,null);return n;",
        ],
        false,
    );
    let plain = "export function run(a,b){let make=x=>y=>()=>x+y;make=x=>y=>()=>null;return a;}";
    let typed = "export function run(a:never,b:Missing):Wrong{let make=(x:any)=>(y:any)=>()=>x+y;make=(x:any)=>(y:any)=>()=>null;return a as Wrong;}";
    assert_eq!(
        compile(plain, false).unwrap().wasm,
        compile(typed, true).unwrap().wasm
    );
}

#[test]
fn joined_result_interfaces_keep_number_only_call_sites() {
    compare_function_programs(
        &[
            "let f=()=>a;f=()=>true;return a;",
            "let f=()=>a;f&&=()=>true;return a;",
            "let f=()=>a;f||=()=>true;return a;",
            "let f=()=>a;f??=()=>true;return a;",
        ],
        false,
    );
    compare_functions(&[
        "const f=()=>null;const g=()=>b;let h=a?f:g;return (a?g:g)();",
        "const f=()=>null;const g=()=>b;let h=a?f:g;return +h();",
        "const f=()=>null;const g=()=>b;let h=a?f:g;h();return g()+b;",
        "const f=()=>null;const g=()=>b;let h=a?f:g;return h()?a:b;",
        "const f=()=>null;const g=()=>{return b;};let h=a?f:g;h();return g()+b;",
        "const f=()=>null;const g=()=>b;const k=()=>a;let h=a?f:g;let j=b?g:k;return (a?k:g)();",
    ]);
}

#[test]
fn immediate_join_callee_and_argument_type_wrappers_erase() {
    let plain =
        "export function run(a,b){const f=x=>+x;let p=f;p(null);const g=x=>-x;return (a?f:g)(b);}";
    let typed = "export function run(a:never,b:Missing):Wrong{const f=(x:any)=>+x;let p=f;p(null);const g=(x:any)=>-x;return ((a?f:g) as any)(b as never) as Wrong;}";
    assert_eq!(
        compile(plain, false).unwrap().wasm,
        compile(typed, true).unwrap().wasm
    );
}

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
fn function_logical_assignments_preserve_selected_and_stored_pairs() {
    compare_function_programs(
        &[
            "let f=()=>a;f&&=()=>b;return a;",
            "let f=()=>a;f||=()=>b;return a;",
            "let f=()=>a;f??=()=>b;return a;",
        ],
        false,
    );
    compare_functions(&[
        "let f=()=>a;f&&=()=>b;return f();",
        "let f=()=>a;f||=()=>b;return f();",
        "let f=()=>a;f??=()=>b;return f();",
        "let f=()=>a;const old=f;const selected=f&&=()=>b;f=()=>a+b;return old()+selected()+f();",
        "let f=()=>a;const selected=f||=()=>b;f=()=>b;return selected()+f();",
        "let f=()=>a;const selected=f??=()=>b;f=()=>b;return selected()+f();",
        "let n=0;const make=x=>{n++;return ()=>x;};let f=()=>a;f&&=make(b);return f()+n;",
        "let n=0;const make=x=>{n++;return ()=>x;};let f=()=>a;f||=make(b);return f()+n;",
        "let n=0;const make=x=>{n++;return ()=>x;};let f=()=>a;f??=make(b);return f()+n;",
        "let f=()=>a;const read=()=>f();const set=()=>{f&&=()=>b;return f();};return read()+set()+read();",
        "let f=x=>x+a;return (f&&=x=>x-b)(a)+f(b);",
        "let f=(x,y)=>x+y+a;return (f||=(x,y)=>x-y+b)(a,b);",
        "let f=(x,y)=>x+y+a;return (f??=(x,y)=>x-y+b)(a,b);",
        "let f=()=>a;let g=()=>b;const h=f&&=(g&&=()=>a+b);return h()+f()+g();",
        "let f=()=>null;f&&=()=>false;return +(f());",
        "let f=()=>null;f||=()=>false;return +(f());",
        "let f=()=>null;f??=()=>false;return +(f());",
        "const apply=(fn,x)=>{fn&&=y=>y+b;return fn(x);};return apply(x=>x+a,a);",
        "const apply=(fn,x)=>fn(x);const twice=(fn,x)=>fn(fn(x));let h=apply;h&&=twice;return h(x=>x+b,a);",
        "const make=x=>()=>x+a;const other=x=>()=>x-b;let h=make;h??=other;return h(a)();",
        "let f=()=>a;if(f&&=()=>b)return f();return 99;",
        "let f=()=>a;if(f||=()=>b)return f();return 99;",
        "let f=()=>a;if(f??=()=>b)return f();return 99;",
    ]);
}

#[test]
fn function_logical_assignment_checks_skipped_rhs_source_depth() {
    for operator in ["&&=", "||=", "??="] {
        for (wrappers, admitted) in [(127, true), (128, false)] {
            let source = format!(
                "export function run(a,b){{let f=()=>a;f{operator}{}()=>b{};return f();}}",
                "(".repeat(wrappers),
                ")".repeat(wrappers)
            );
            for typescript in [false, true] {
                assert_eq!(
                    compile(&source, typescript).is_ok(),
                    admitted,
                    "{operator}/{wrappers}"
                );
                assert_eq!(
                    compile_value(&source, typescript).is_ok(),
                    admitted,
                    "value/{operator}/{wrappers}"
                );
            }
        }
    }
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
fn concise_local_results_preserve_existing_function_pairs_and_creator_effects() {
    compare_functions(&[
        "const id=fn=>fn;const f=id(()=>a);return f();",
        "const id=fn=>fn;let f=id(x=>x+a);return f(b);",
        "const choose=(f,g,c)=>c?f:g;const h=choose(()=>a,()=>b,a);return h();",
        "const id=x=>x;const f=id(()=>a);return id(b)+f();",
        "const id=x=>x;const forward=fn=>id(fn);const f=forward(()=>a);return id(b)+f();",
        "const id=fn=>fn;const apply=(fn,x)=>fn(x);return apply(id(x=>x+a),b);",
        "let current=()=>a;const get=()=>current;const f=get();current=()=>b;return f()*10+current();",
        "let current=()=>a;const get=x=>current;const f=get((current=()=>b,1));current=()=>a;return f();",
        "const pass=fn=>(fn(),fn);const f=pass(()=>++a);return f()+a;",
        "const choose=(f,g,c)=>c?f:g;let f=()=>a;const h=choose(f,(f=()=>b),a);return h()*10+f();",
        "const replace=fn=>(fn=()=>b);const f=replace(()=>a);return f();",
        "const id=x=>x;const replace=fn=>(fn=id(()=>b));const f=replace(()=>a);return f();",
        "const get=()=>()=>b;const replace=fn=>(fn=get());const f=replace(()=>a);return f();",
        "const choose=(f,g)=>a?(b++,f):(a++,g);const h=choose(()=>a,()=>b);return h()+a+b;",
        "const make=x=>()=>++x;const id=fn=>fn;const f=id(make(a));return f()*10+f();",
        "const make=fn=>()=>fn();const id=fn=>fn;const f=id(make(()=>a));return f();",
        "const get=()=>a?()=>a:()=>b;const f=get();return f();",
        "const get=x=>(a++,()=>x+a);const f=get(b++);return f()+b;",
        "const choose=(f,g,c)=>c?f:g;const f=choose(x=>x+a,x=>x-b,a);return f(b);",
        "const id=x=>x;const f=(id)(()=>a);return f();",
        "const f=(fn=>fn)(()=>a);return f();",
        "const id=fn=>fn;return id(x=>x+a)(b);",
        "const choose=(f,g,c)=>c?f:g;return choose(()=>a,()=>b,a)();",
        "const get=()=>a?()=>a:()=>b;return get()();",
        "return (a?()=>a:()=>b)();",
        "let f=()=>a;return (f=()=>b)();",
        "const id=fn=>fn;let f=x=>x+a;return id(f)((f=x=>x-b,b))+f(a);",
    ]);
}

#[test]
fn local_function_result_arguments_run_once_in_caller_order() {
    compare_functions(&[
        "let current=()=>a;const get=(x,y)=>(a=x*10+y,current);const f=get(a++,a+=10);return f()+a;",
    ]);
}

#[test]
fn function_results_do_not_cross_unsupported_kinds_or_runtime_signatures() {
    for body in [
        "const id=fn=>fn;return id(()=>a);",
        "const id=fn=>fn;return +id(()=>a);",
        "const choose=(f,g,c)=>c?f:g;const h=choose(()=>a,x=>x,a);return h();",
        "const choose=(f,g,c)=>c?f:g;const h=choose(()=>a,b,a);return h();",
        "const id=fn=>fn;const f=id?.(()=>a);return f();",
        "const id=fn=>fn;const f=id(...a);return f();",
        "const id=fn=>fn;const f=id(()=>a);return f?.();",
    ] {
        for typescript in [false, true] {
            let source = format!("export function run(a,b){{{body}}}");
            assert!(compile(&source, typescript).is_err(), "{source}");
            assert!(compile_value(&source, typescript).is_err(), "{source}");
            let source = source.replace("export function", "export async function");
            assert!(compile_async(&source, typescript).is_err(), "{source}");
        }
    }
    let source = "export async function run(a,b){const id=fn=>fn;return await id(()=>a);}";
    assert!(compile_async(source, false).is_err());
}

#[test]
fn function_results_keep_wide_leaf_calls_and_small_stack_limits() {
    let parameters = (0..1000)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let arguments = vec!["a"; 1000].join(",");
    let body = format!("const id=fn=>fn;return id(({parameters})=>p0+p999+b)({arguments});");
    compare_functions(&[&body]);
    let mut nested = Vec::new();
    let mut forwarded = Vec::new();
    for depth in 0..10 {
        let mut value = String::from("()=>a");
        for _ in 0..depth {
            value = format!("id({value})");
        }
        let source =
            format!("export function run(a,b){{const id=fn=>fn;const f={value};return f();}}");
        let result = std::thread::Builder::new()
            .name("function_results_keep_wide_leaf_calls_and_small_stack_limits".to_owned())
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        nested.push(result.is_ok());
        if let Ok(artifact) = result {
            assert_eq!(
                NumericInstance::new(&artifact.wasm)
                    .unwrap()
                    .run(4.0, 2.0)
                    .unwrap(),
                4.0
            );
        }
        let mut declarations = String::from("let current=()=>a;const get0=()=>current;");
        for i in 1..=depth {
            declarations.push_str(&format!("const get{i}=()=>get{}();", i - 1));
        }
        let source =
            format!("export function run(a,b){{{declarations}const f=get{depth}();return f();}}");
        let result = std::thread::Builder::new()
            .name("function_results_keep_wide_leaf_calls_and_small_stack_limits".to_owned())
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        forwarded.push(result.is_ok());
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
    assert_eq!(
        nested,
        [true, true, true, true, true, true, true, true, false, false]
    );
    assert_eq!(
        forwarded,
        [true, true, true, true, true, true, true, true, false, false]
    );
    println!("function-result admission on 2MiB stack: nested {nested:?}, forwarded {forwarded:?}");
}

#[test]
fn local_function_results_preserve_source_depth_costs_and_expansion_limits() {
    for (wrappers, admitted) in [(127, true), (128, false)] {
        let source = format!(
            "export function run(a,b){{const id=fn=>fn;const f=id({}()=>a{});return f();}}",
            "(".repeat(wrappers),
            ")".repeat(wrappers)
        );
        assert_eq!(
            compile(&source, false).is_ok(),
            admitted,
            "{wrappers} return-argument wrappers"
        );
    }
    for (locals, admitted) in [(125, true), (126, false)] {
        let declarations = (0..locals)
            .map(|i| format!("let v{i}=a;"))
            .collect::<String>();
        let source = format!(
            "export function run(a,b){{const id=fn=>fn;const read=()=>{{{declarations}return a;}};const f=id(read);return f();}}"
        );
        assert_eq!(
            compile(&source, false).is_ok(),
            admitted,
            "{locals} leaf body locals"
        );
    }
    for (calls, admitted) in [(85, true), (86, false)] {
        let source = format!(
            "export function run(a,b){{const id=fn=>fn;return({});}}",
            vec!["id(()=>a)()"; calls].join(",")
        );
        assert_eq!(
            compile(&source, false).is_ok(),
            admitted,
            "{calls} returned-function calls"
        );
    }
}

#[test]
fn function_result_nesting_counts_across_bodies_and_numeric_prefixes() {
    for branches in [16, 64] {
        let mut declarations = String::from("let current=()=>a;const get0=()=>current;");
        for i in 1..=3 {
            declarations.push_str(&format!(
                "const get{i}=()=>{}get{}();",
                "a?current:".repeat(branches),
                i - 1
            ));
        }
        let source =
            format!("export function run(a,b){{{declarations}const f=get3();return f();}}");
        let result = std::thread::Builder::new()
            .name("function_result_nesting_counts_across_bodies_and_numeric_prefixes".to_owned())
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(result.is_ok(), branches == 16);
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
    let numeric = format!("{}a", "a?a:".repeat(65));
    let source = format!(
        "export function run(a,b){{const numeric=()=>{numeric};const get=()=>{}(numeric(),()=>a);const f=get();return f();}}",
        "a?(()=>a):".repeat(64)
    );
    let result = std::thread::Builder::new()
        .name("function_result_nesting_counts_across_bodies_and_numeric_prefixes".to_owned())
        .stack_size(2 * 1024 * 1024)
        .spawn(move || compile(&source, false))
        .unwrap()
        .join()
        .unwrap();
    assert!(result.err().unwrap().contains("128"));
}

#[test]
fn local_callback_constraints_reject_conflicting_kinds_and_runtime_signatures() {
    for body in [
        "const apply=(fn,x)=>fn(x);return apply(a,b);",
        "const apply=fn=>fn();return apply(x=>x);",
        "const apply=fn=>{fn=a;return fn();};return apply(()=>a);",
        "const apply=fn=>fn;return apply(()=>a);",
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
            .name(
                "nested_callback_argument_constructors_have_an_observed_small_stack_bound"
                    .to_owned(),
            )
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
    compare_function_programs(bodies, true);
}

fn compare_function_programs(bodies: &[&str], require_call: bool) {
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
            if require_call {
                assert!(artifact.wat.contains("call_indirect"));
            }
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
    for expression in ["f", "+f", "f+1"] {
        let source =
            format!("export function run(a,b){{const f=a?()=>a:()=>b;return {expression};}}");
        assert!(compile_value(&source, false).is_err(), "{source}");
    }
    for body in [
        "const f=a?()=>a:()=>b;return await f;",
        "const f=a?()=>a:()=>b;return await Promise.resolve(f);",
        "const f=a?()=>a:()=>b;return await Promise.reject(f);",
        "const f=a?()=>a:()=>b;return f;",
    ] {
        let source = format!("export async function run(a,b){{{body}}}");
        assert!(compile_async(&source, false).is_err(), "{source}");
    }
}

#[test]
fn mutable_function_slots_require_function_initializers_and_unchanged_signatures() {
    for body in [
        "let f;f=()=>a;return f();",
        "let f=a;f=()=>b;return f();",
        "let f=()=>a;f=b;return a;",
        "let f=()=>a;f=null;return a;",
        "let f=()=>a;f=true;return a;",
        "let f=()=>a;f=x=>x;return a;",
        "let f=x=>x;let g=()=>a;f=g;return a;",
        "let f=()=>a;f&&=x=>x;return a;",
        "let f=()=>a;f||=x=>x;return a;",
        "let f=()=>a;f??=x=>x;return a;",
        "const f=()=>a;f&&=()=>b;return a;",
        "const f=()=>a;f||=()=>b;return a;",
        "const f=()=>a;f??=()=>b;return a;",
        "let f=()=>a;f+=()=>b;return a;",
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
            .name(
                "function_assignment_cursor_crossings_are_bounded_on_a_small_native_stack"
                    .to_owned(),
            )
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
            .name("runtime_constructor_lowering_has_an_observed_small_stack_bound".to_owned())
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
    // The original header cell and first iteration's fresh binding take
    // 32 bytes; each body/call plus the next header binding takes 48 bytes.
    // Exercise the largest complete run below the unchanged 1 MiB cap,
    // then its immediate successor and reuse after the allocator trap.
    assert_eq!(run.call(&mut store, (1.0, 21844.0)).unwrap(), 21843.0);
    assert_eq!(top.get(&mut store).i32(), Some(1048544));
    assert_eq!(memory.size(&store), 16);
    let error = run.call(&mut store, (1.0, 21845.0)).unwrap_err();
    assert_eq!(
        error.downcast_ref::<Trap>(),
        Some(&Trap::UnreachableCodeReached)
    );
    assert_eq!(top.get(&mut store).i32(), Some(1048576));
    assert_eq!(run.call(&mut store, (1.0, 1.0)).unwrap(), 0.0);
    assert_eq!(top.get(&mut store).i32(), Some(80));
    node(&format!(
        "const assert=require('node:assert/strict');const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({wasm:?}))).exports;assert.equal(e.run(1,21844),21843);assert.equal(e.top.value,1048544);assert.equal(e.arena.buffer.byteLength,1048576);assert.throws(()=>e.run(1,21845),WebAssembly.RuntimeError);assert.equal(e.top.value,1048576);assert.equal(e.run(1,1),0);assert.equal(e.top.value,80);"
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

#[test]
fn selected_higher_order_functions_receive_callback_pairs() {
    compare_functions(&[
        "const apply=(fn,x)=>fn(x);const twice=(fn,x)=>fn(fn(x));const h=a?apply:twice;return h(x=>x+b,a);",
        "const apply=(fn,x)=>fn(x);const f=a?apply:apply;return f(x=>x,b);",
        "let apply=(fn,x)=>fn(x);return apply(x=>x,b);",
        "let h=(fn,x)=>fn(x);const old=h;h=(fn,x)=>fn(x)+1;return old(x=>x+a,b)*10+h(x=>x-b,a);",
        "let h=(fn,x)=>fn(x);let cb=x=>x+a;return h(cb,(h=(fn,x)=>fn(x)*10,cb=x=>x-b,a))+h(cb,b);",
        "const h=a?(fn,x)=>{const invoke=()=>fn(x);fn=y=>y+b;return invoke();}:(fn,x)=>fn(x);return h(x=>x+a,b);",
        "const h=a?(fn,x)=>{const make=()=>()=>fn(x);const read=make();fn=y=>y+b;return read();}:(fn,x)=>fn(x);return h(x=>x+a,b);",
        "const h=a?(fn,x)=>{const invoke=a?()=>fn(x):()=>fn(x)+1;fn=y=>y+b;return invoke();}:(fn,x)=>fn(x);return h(x=>x+a,b);",
        "const h=a?(f,g,x)=>f()+g(x):(f,g,x)=>g(f()+x);return h(()=>a,x=>x+b,b);",
        "const h=a?fn=>fn():fn=>fn()+b;return h(()=>a);",
        "const h=a?(fn,x)=>fn(x):(fn,x)=>fn(x)+b;const relay=(g,x)=>h(g,x);return relay(x=>x+a,b);",
        "const h=a?(fn,x)=>{let y=0;for(let i=0;i<3;i++)y+=fn(x+i);return y;}:(fn,x)=>fn(x);return h(x=>x+a,b);",
        "const make=x=>()=>++x;const h=a?fn=>fn()*10+fn():fn=>fn();return h(make(b));",
        "let h=(fn,x)=>fn(x);h=(fn,x)=>{if(x<=0)return fn(x);return h(fn,x-1)+fn(x);};return h(x=>x+b,3);",
        "const id=fn=>fn;const apply=(fn,x)=>fn(x);const twice=(fn,x)=>fn(fn(x));const h=id(a?apply:twice);return h(x=>x+b,a);",
        "const id=fn=>fn;const apply=(fn,x)=>fn(x);const twice=(fn,x)=>fn(fn(x));return id(a?apply:twice)(x=>x+b,a);",
        "const select=(f,g,c)=>c?f:g;const apply=(fn,x)=>fn(x);const twice=(fn,x)=>fn(fn(x));const h=select(apply,twice,a);return h(x=>x+b,a);",
        "const make=()=> (fn,x)=>fn(x)+b;let h=make();return h(x=>x+a,b);",
        "const outer=(fn,x)=>fn(x);const make=()=>outer;const h=a?make():make();return h(x=>x+b,a);",
    ]);
}

#[test]
fn runtime_calls_evaluate_surplus_arguments_after_the_callee_snapshot() {
    compare_functions(&[
        "const f=a?(()=>a):(()=>b);return f(a);",
        "const f=a?(x=>x+a):(x=>x-b);return f(a,(a=b,null));",
        "let f=x=>x+a;return f(a,(f=x=>x-b,a=b,null));",
        "const make=x=>{return y=>x+y+a;};const f=make(a);return f(b,a=b,null);",
        "const invoke=fn=>fn(a,b);return invoke(x=>x+b);",
        "const invoke=fn=>fn(a,b)+fn(b,a,b);return invoke(()=>a);",
        "const apply=(fn,x)=>fn(x,a);const invoke=(cb,x)=>cb(y=>y+b,x,a);return invoke(apply,a);",
        "const make=offset=>fn=>()=>fn(a,b)+offset;return make(a)(x=>x+b)();",
        "let make=x=>()=>x+a;return make(a,(make=x=>()=>x-b,a=b,null))();",
        "const make=a?(x=>()=>x+a):(x=>()=>x-b);return make(a,a=b)();",
    ]);
}

#[test]
fn surplus_callback_calls_keep_one_declared_signature() {
    compare_function_programs(
        &[
            "const apply=(fn,x)=>fn(x);return apply(()=>a,b);",
            "const f=a?()=>a:()=>b;return f(a);",
            "const h=a?(fn,x)=>fn(x):(fn,x)=>fn(x);return h(()=>a,b);",
            "const f=fn=>fn();const g=fn=>fn(a);const h=f&&g;return h(()=>b);",
            "const f=fn=>fn();const g=fn=>fn(a);const h=f||g;return h(()=>b);",
            "const f=fn=>fn();const g=fn=>fn(a);const h=f??g;return h(()=>b);",
            "const apply=fn=>fn(a)+fn(a,b);return a;",
            "const apply=fn=>{const alias=fn;return alias(a)+fn(a,b);};return a;",
            "const apply=(fn,x)=>fn(x);const relay=f=>apply(f,a)+f(a,b);return a;",
            "const h=a?fn=>fn():fn=>fn(a);return h(()=>b);",
            "let make=x=>(fn,y)=>fn(y);return make(a)(()=>b,a);",
            "const apply=fn=>fn(a)+fn(a,b);return apply(x=>x+b);",
            "const apply=fn=>{const alias=fn;return alias(a)+fn(a,b);};return apply(()=>a);",
            "const apply=(fn,x)=>fn(x);const relay=f=>apply(f,a)+f(a,b);return relay(x=>x+b);",
        ],
        false,
    );
}

#[test]
fn runtime_surplus_arguments_keep_width_and_missing_input_guards() {
    let arguments = vec!["a"; 1000].join(",");
    compare_functions(&[&format!("let f=()=>a;return f({arguments});")]);
    for body in [
        format!("let f=()=>a;return f({arguments},a);"),
        "let f=x=>x+a;return f();".into(),
        "let f=x=>x+a;return f(a,...b);".into(),
        "let f=x=>x+a;return f?.(a,b);".into(),
        "const invoke=fn=>fn(a);return invoke((x,y)=>x+y);".into(),
        "const invoke=fn=>fn();return invoke(x=>x);".into(),
        "const invoke=fn=>fn(a,b);return invoke(async x=>x);".into(),
        "const cycle=fn=>fn(fn);return a;".into(),
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            assert!(compile(&source, ts).is_err(), "{source}");
            assert!(compile_value(&source, ts).is_err(), "{source}");
        }
    }
}

#[test]
fn known_local_calls_evaluate_surplus_arguments_before_binding() {
    compare_function_programs(
        &[
            "const f=x=>x+a;return f(b,a=b);",
            "const f=x=>x; return f(a,b);",
            "const pick=()=>{return ()=>a;};const f=pick(a);return f();",
            "const f=x=>x+a;return f(a,a=b);",
            "const f=a=>a;return f(a,a=b);",
            "const f=x=>x+a;return f(a,(a=b,null));",
            "const f=()=>a;return f(a=b,undefined,false);",
            "let x=a;const f=x=>x+a;return f(b,x=a+1,a=x);",
            "const f=x=>{return x+a;};return f(a,a=b,null);",
            "const call=(fn,x)=>fn(x);return call(y=>y+b,a,b);",
            "const make=x=>()=>x+a;const f=make(a,a=b);return f();",
            "const make=x=>{const f=()=>x+a;return f;};return make(a,a=b)();",
            "const make=x=>y=>()=>x+y+a;return make(a,a=b)(b)();",
            "const id=x=>x;const f=id(()=>a,a=b,null);return f()+id(a,undefined);",
            "const f=x=>x+a;const make=y=>(a=y,()=>y);return f(a,make(b));",
            "const f=x=>x+a;return f(a,(a=b,x=>null));",
        ],
        false,
    );
}

#[test]
fn known_local_surplus_arguments_keep_width_and_source_guards() {
    let arguments = vec!["a"; 1000].join(",");
    compare_function_programs(&[&format!("const f=()=>a;return f({arguments});")], false);
    for body in [
        format!("const f=()=>a;return f({arguments},a);"),
        "const f=x=>x+a;return f();".into(),
        "const f=x=>x+a;return f(a,...b);".into(),
        "const f=x=>x+a;return f?.(a,b);".into(),
        "const f=()=>a;return f(()=>missing);".into(),
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            assert!(compile(&source, ts).is_err(), "{source}");
            assert!(compile_value(&source, ts).is_err(), "{source}");
        }
    }
}

#[test]
fn primitive_callbacks_keep_numeric_consumers_and_identity() {
    compare_functions(&[
        "const apply=(fn,x)=>fn(x);return +apply(x=>null,a);",
        "const apply=(fn,x)=>fn(x);return +apply(x=>undefined,a);",
        "const apply=(fn,x)=>fn(x);return apply(x=>x?false:b,a)?a:b;",
        "const apply=(fn,x)=>fn(x);let value=apply(x=>null,a);value=b;return +value;",
        "const apply=(fn,x)=>fn(x);const invoke=(cb,x)=>cb(y=>y?false:b,x);return +invoke(apply,a);",
        "const apply=(fn,x)=>fn(x);const id=x=>x;const f=x=>x+b;return +apply(x=>null,a)+id(a)+id(f)(b);",
    ]);
}

#[test]
fn primitive_callback_results_do_not_become_numeric_exports() {
    for body in [
        "const apply=(fn,x)=>fn(x);return apply(x=>null,a);",
        "const h=a?(fn,x)=>fn(x):(fn,x)=>fn(x);return h(y=>true,b);",
        "const invoke=(cb,x)=>cb(y=>y+b,x);return invoke((fn,x)=>null,a);",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            assert!(compile(&source, ts).is_err(), "{source}");
        }
    }
}

#[test]
fn callbacks_accept_further_numeric_callbacks() {
    compare_functions(&[
        "const apply=(fn,x)=>fn(x);const invoke=(cb,x)=>cb(y=>y+b,x);return invoke(apply,a);",
        "const apply=(fn,x)=>fn(x);const invoke=a?((cb,x)=>cb(y=>y+b,x)):((cb,x)=>cb(y=>y-b,x));return invoke(apply,a);",
        "const apply=(fn,x)=>fn(x);const invoke=(cb,fn,x)=>cb(fn,x);return invoke(apply,z=>z+b,a);",
        "const apply=(fn,x)=>fn(x);const invoke=(cb,fn,x)=>cb(fn,x);const relay=(g,fn,x)=>invoke(g,fn,x);return relay(apply,z=>z+b,a);",
        "const apply=(fn,x)=>fn(x);const invoke=(cb,x)=>{const alias=cb;return alias(y=>y+b,x);};return invoke(apply,a);",
        "const id=x=>x;const fn=z=>z+b;return id(a)+id(fn)(b);",
        "const apply=(fn,x)=>fn(x);const middle=(cb,x)=>cb(y=>y+b,x);const outer=(step,x)=>step(apply,x);return outer(middle,a);",
        "const apply=(fn,x)=>fn(x);const invoke=(cb,fn,x)=>cb(fn,x);const make=seed=>(fn,x)=>fn(x)+seed;return invoke(make(a),z=>z+b,b);",
        "const invoke=(cb,x)=>cb(y=>y+b,x);let apply=(fn,x)=>fn(x);const old=apply;const out=invoke(apply,(apply=(fn,x)=>fn(x)-b,a));return out+old(y=>y,b);",
        "const apply=(fn,x)=>fn(x);const invoke=(cb,x)=>{const alias=cb;cb=(fn,y)=>fn(y)+b;return alias(y=>y+a,x)+cb(y=>y-b,x);};return invoke(apply,a);",
        "const apply=(fn,x)=>fn(x);const make=offset=>(cb,x)=>()=>cb(y=>y+offset,x);const f=make(a)(apply,b);return f();",
        "const apply=(fn,x)=>fn(x);const invoke=(cb,fn,x)=>{const alias=fn;return cb(alias,x);};return invoke(apply,y=>y+b,a);",
        "const apply=(fn,x)=>fn(x);const invoke=(cb,x)=>cb(y=>y+b,x);const id=x=>x;const fn=y=>y+a;return invoke(apply,a)+id(a)+id(fn)(b);",
        "const apply=(fn,x)=>fn(x);const invoke=(cb,x)=>cb(y=>y+b,x);const make=offset=>(fn,y)=>fn(y)+offset;let fn=y=>y+a;return invoke(make(a),b)+fn(a);",
        "let n=0;const make=offset=>(fn,y)=>(n++,fn(y)+offset);const invoke=(cb,x)=>cb(y=>y+b,x);return invoke(make(a),b)+n;",
    ]);
}

#[test]
fn higher_callback_shapes_keep_arity_async_and_cycle_guards() {
    for body in [
        "const invoke=(cb,x)=>cb(y=>y+b,x);return invoke(async(fn,x)=>fn(x),a);",
        "const invoke=(cb,x)=>cb(y=>y+b,x);return invoke((fn,x)=>fn(),a);",
        "const invoke=(cb,x)=>cb(y=>y+b,x);return invoke((fn,x)=>fn(x),()=>a);",
        "const cycle=fn=>fn(fn);return a;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typescript in [false, true] {
            assert!(compile(&source, typescript).is_err(), "{source}");
            assert!(compile_value(&source, typescript).is_err(), "{source}");
        }
    }
}

#[test]
fn higher_order_signatures_reject_mixed_parameter_kinds_and_non_leaf_callbacks() {
    for body in [
        "const h=a?(fn,x)=>fn(x):(fn,x)=>fn+x;return h(y=>y,b);",
        "let h=(fn,x)=>fn(x);h=(fn,x)=>fn+x;return h(y=>y,b);",
        "const h=a?(fn,x)=>fn(x):(fn,x)=>fn(x);return h(a,b);",
        "const h=a?(fn,x)=>fn(x):(fn,x)=>fn(x);return h((g,y)=>g(y),b);",
        "const h=a?(fn,x)=>fn(x):(fn,x)=>fn(x);return h(y=>()=>y,b);",
        "const h=a?(fn,x)=>fn(x):(fn,x)=>fn(x);return h?.(y=>y,b);",
        "const h=a?(fn,x)=>fn(x):(fn,x)=>fn(x);return h(y=>y,...b);",
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
fn higher_order_frames_keep_one_thousand_logical_arguments_and_erased_types() {
    let parameters = (0..999)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let arguments = vec!["a"; 999].join(",");
    compare_functions(&[&format!(
        "let h=(fn,{parameters})=>fn(p0)+fn(p998);return h(x=>x+b,{arguments});"
    )]);
    let parameters = (0..1000)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let arguments = vec!["a"; 1000].join(",");
    compare_functions(&[&format!(
        "let h=fn=>fn({arguments});return h(({parameters})=>p0+p999+b);"
    )]);
    let plain =
        "export function run(a,b){const h=a?(fn,x)=>fn(x):(fn,x)=>fn(fn(x));return h(y=>y+b,a);}";
    let typed = "export function run(a:never,b:Missing):Wrong {const h=(a?((fn:Wrong,x:string)=>fn(x)):((fn:Missing,x:boolean)=>fn(fn(x)))) as unknown as Wrong;return (h as any)(((y:Missing)=>y+b) satisfies Wrong,a as never);}";
    assert_eq!(
        compile(plain, false).unwrap().wasm,
        compile(typed, true).unwrap().wasm
    );
    let plain = plain.replace("export function", "export async function");
    let typed = typed.replace("export function", "export async function");
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
}

#[test]
fn runtime_callback_arguments_keep_the_parent_generated_region_budget() {
    for (regions, admitted) in [(40, true), (70, false)] {
        let source = format!(
            "export function run(a,b){{let cb=x=>x+b;const get=()=>{}cb;let h=(fn,x)=>fn(x);return {}h(get(),b);}}",
            "a?cb:".repeat(regions),
            "a?0:".repeat(regions)
        );
        // A generous probe stack lets an erroneous admission report a normal
        // failed assertion rather than exhausting the test runner's native stack.
        let source_for_thread = source.clone();
        let result = std::thread::Builder::new()
            .name("runtime_callback_arguments_keep_the_parent_generated_region_budget".to_owned())
            .stack_size(16 * 1024 * 1024)
            .spawn(move || compile(&source_for_thread, false))
            .unwrap()
            .join()
            .unwrap();
        if admitted {
            let artifact = result.unwrap_or_else(|error| panic!("{source}: {error}"));
            let mut host = NumericInstance::new(&artifact.wasm).unwrap();
            assert_eq!(host.run(0.0, 2.0).unwrap(), 4.0);
        } else {
            let error = result
                .err()
                .expect("parent and callback bodies must share the generated region budget");
            assert!(error.contains("128"), "{error}");
            let error = std::thread::Builder::new()
                .name(
                    "runtime_callback_arguments_keep_the_parent_generated_region_budget".to_owned(),
                )
                .stack_size(2 * 1024 * 1024)
                .spawn(move || {
                    compile(&source, false)
                        .err()
                        .expect("small stack rejects deep callback")
                })
                .unwrap()
                .join()
                .unwrap();
            assert!(error.contains("128"), "{error}");
        }
    }
}

#[test]
fn block_local_results_return_selected_functions_and_captured_cells() {
    compare_functions(&[
        "const apply=(fn,x)=>fn(x);const twice=(fn,x)=>fn(fn(x));const pick=flag=>{if(flag)return apply;return twice;};const h=pick(a);return h(x=>x+b,a);",
        "const make=flag=>{if(flag)return (fn,x)=>fn(x);return (fn,x)=>fn(fn(x));};const h=make(a);return h(x=>x+b,a);",
        "const id=fn=>{return fn;};const apply=(fn,x)=>fn(x);const twice=(fn,x)=>fn(fn(x));const h=id(a?apply:twice);return h(x=>x+b,a);",
        "const choose=(flag,fn)=>{if(flag)return fn;return ()=>b;};const f=choose(a,()=>a);return f();",
        "const id=fn=>{return fn;};const f=id(()=>a);return f();",
        "const get=()=>{return ()=>b;};const replace=fn=>{fn=get();return fn;};const f=replace(()=>a);return f();",
        "const identity=fn=>{return fn;};const saved=identity(a?()=>a:()=>b);const number=identity(a);return saved()+number;",
        "const choose=(flag,left,right)=>{if(flag)return left;return right;};const f=choose(a,x=>x+a,x=>x-b);return f(b);",
        "const forward=fn=>{const alias=fn;return alias;};const f=forward(a?()=>a:()=>b);return f();",
        "const wrap=fn=>{return ()=>fn(b);};const f=wrap(a?x=>x+a:x=>x-b);return f();",
        "const make=()=>{return ()=>a;};return make()();",
        "const make=x=>{return ()=>++x;};const f=make(a);return f();",
        "const pick=flag=>{if(flag)return x=>x+a;return x=>x-b;};const f=pick(a);return f(b);",
        "const pick=flag=>{const left=x=>x+a;const right=x=>x-b;if(flag)return left;return right;};const f=pick(a);return f(b);",
        "const pick=flag=>{let selected=flag?()=>a:()=>b;return selected;};const f=pick(a);return f();",
        "const pick=flag=>{let value=a;if(flag){if(b)return ()=>++value;return ()=>--value;}return ()=>value+b;};const f=pick(a);return f()+f();",
        "const pick=flag=>{for(let i=0;i<3;i++){if(flag&&i===1)return ()=>i+b;}return ()=>a;};const f=pick(a);return f();",
        "const pick=flag=>{let i=0;while(i<3){i++;if(flag)return ()=>i+b;}return ()=>i;};const f=pick(a);return f();",
        "const pick=flag=>{do{if(flag)return ()=>a;}while(0);return ()=>b;};const f=pick(a);return f();",
        "const pick=(x,y)=>{a+=10;if(x)return ()=>x*100+y*10+a;return ()=>y+a;};const f=pick(a++,a++);return f()+a;",
        "const current=a?()=>a:()=>b;const pick=flag=>{if(flag)return current;return ()=>a+b;};const f=pick(a);a++;return f();",
        "let current=a?()=>a:()=>b;const pick=()=>{return current;};const f=pick();current=()=>b*10;return f()+current();",
        "const pick=flag=>{if(flag)return (a++,()=>a);return (b++,()=>b);};const f=pick(a);return f()+a+b;",
    ]);
}

#[test]
fn block_function_results_keep_return_kind_and_export_boundaries() {
    for body in [
        "const pick=flag=>{if(flag)return (fn,x)=>fn(x);return (fn,x)=>fn+x;};return a;",
        "const pick=()=>{return ()=>a;};return pick();",
        "const pick=flag=>{if(flag)return ()=>a;return b;};return a;",
        "const pick=flag=>{if(flag)return ()=>a;return x=>x;};return a;",
        "const pick=flag=>{if(flag)return ()=>a;};return a;",
        "const pick=()=>{return async()=>a;};const f=pick();return f();",
        "const pick=()=>{return ()=>a;};const f=pick?.();return f();",
        "const pick=()=>{return ()=>a;};const f=pick(...a);return f();",
        "const pick=()=>{return later;const later=()=>a;};return a;",
        "const pick=()=>{return x=>x+a;};const f=a?pick:pick;return f();",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typescript in [false, true] {
            assert!(compile(&source, typescript).is_err(), "{source}");
            assert!(compile_value(&source, typescript).is_err(), "{source}");
        }
    }
}

#[test]
fn block_function_results_keep_small_native_stack_and_expansion_limits() {
    let outcomes = std::thread::Builder::new()
        .name("block_function_results_keep_small_native_stack_and_expansion_limits".to_owned())
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            (0..11)
                .map(|depth| {
                    let mut body = String::from("const get0=()=>{return ()=>a;};");
                    for index in 1..=depth {
                        body.push_str(&format!(
                            "const get{index}=()=>{{return get{}();}};",
                            index - 1
                        ));
                    }
                    let source = format!(
                        "export function run(a,b){{{body}const f=get{depth}();return f();}}"
                    );
                    compile(&source, false).is_ok()
                })
                .collect::<Vec<_>>()
        })
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(
        outcomes,
        vec![
            true, true, true, true, true, true, true, false, false, false, false
        ]
    );
    println!("block function-result lowering on 2MiB stack: {outcomes:?}");
    // Generated result regions accumulate across block call boundaries as well
    // as within a single cursor; source bodies alone are each below the limit.
    for (nested, admitted) in [(16, true), (64, false)] {
        let mut body =
            String::from("const current=a?()=>a:()=>b;const get0=()=>{return current;};");
        for index in 1..=3 {
            body.push_str(&format!(
                "const get{index}=()=>{{return {}get{}();}};",
                "a?current:".repeat(nested),
                index - 1
            ));
        }
        let source = format!("export function run(a,b){{{body}const f=get3();return f();}}");
        let result = std::thread::Builder::new()
            .name("block_function_results_keep_small_native_stack_and_expansion_limits".to_owned())
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(result.is_ok(), admitted);
        if let Err(error) = result {
            assert!(error.contains("128"), "{error}");
        }
    }
    // One initial constructor and the final indirect call consume two units;
    // each factory call consumes its specialization and returned constructor.
    for (calls, admitted) in [(127, true), (128, false)] {
        let calls = vec!["pick()"; calls].join(",");
        let source = format!(
            "export function run(a,b){{const pick=()=>{{return ()=>a;}};let f=()=>b;f=({calls});return f();}}"
        );
        let result = compile(&source, false);
        assert_eq!(result.is_ok(), admitted, "{source}: {:?}", result.err());
    }
}

#[test]
fn runtime_selected_factories_return_fresh_closure_pairs() {
    compare_functions(&[
        "const one=x=>()=>++x;const two=x=>()=>x+b;const make=a?one:two;const f=make(a);return f()+f();",
        "const get=()=>a?()=>a:()=>b;let f=get;return f()();",
        "let current=()=>a;let get=()=>current;return get()();",
        "const one=x=>()=>++x;const two=x=>()=>x+b;const make=a?one:two;const f=make(a);const g=make(b);return f()*100+g()*10+f();",
        "let make=x=>()=>x+a;const old=make;make=x=>()=>x-b;const f=old(a);const g=make(b);a++;return f()*10+g();",
        "let leaf=x=>x+a;const make=a?()=>leaf:()=>leaf;const f=make();leaf=x=>x-b;return f(a)+leaf(a);",
        "let make=x=>()=>x+a;const f=make((make=x=>()=>x-b,a));return f()+make(b)();",
        "let make=x=>(a++,()=>x);make(b);return a;",
        "let n=0;const make=a?(n++,x=>()=>++x):(n+=2,x=>()=>x+b);const f=make(a);return f()+n;",
        "const id=fn=>fn;let make=x=>id(()=>x);return make(a)()+make(b)();",
        "let make=(fn,x)=>(fn(x),fn);const f=make(y=>y+a,b);return f(b);",
        "const factory=a?(fn,x)=>()=>fn(x):(fn,x)=>()=>fn(x)+b;const f=factory(x=>x+a,b);return f()+f();",
        "let make=x=>()=>x;make=x=>x<=0?()=>b:make(x-1);return make(3)();",
        "const factory=a?()=>x=>x+a:()=>x=>x-b;const f=factory();return f(b);",
        "const make=a?x=>()=>x+a:x=>()=>x-b;const alias=make;return alias(a)()+make(b)();",
        "let make=(x,y)=>()=>x*10+y;return make(a++,a++)()+a;",
        "let current=()=>a;let make=x=>(current=()=>x,current);const old=make(b);make(a);return old()+current();",
        "const make=a?x=>()=>++x:x=>()=>--x;const f=make(a);const alias=f;return f()+alias();",
        "let make=x=>()=>x;const read=()=>make;const get=read();make=x=>()=>x+b;return get(a)()+make(a)();",
        "return (a?x=>()=>x+a:x=>()=>x-b)(a)();",
        "let n=0;const make=(n++,a?x=>()=>x+n:x=>()=>x-n);return make((n++,b))()+n;",
        "const build=x=>{let y=x;return ()=>++y;};let make=x=>build(x);const f=make(a);return f()+f();",
        "const pick=c=>{if(c)return x=>()=>x+a;return x=>()=>x-b;};const make=pick(a);return make(b)();",
        "let make=x=>()=>{return ++x;};const f=make(a);const g=make(b);return f()*100+g()*10+f();",
    ]);
}

#[test]
fn runtime_block_factories_return_selected_function_pairs() {
    compare_functions(&[
        "let make=x=>{return ()=>x+a;};return make(b)();",
        "const make=a?(x=>{return ()=>x+a;}):(x=>{return ()=>x-b;});return make(b)();",
        "let make=x=>{if(x)return ()=>x+a;return ()=>x-b;};return make(a)();",
        "let make=x=>{let y=x;return ()=>++y;};const f=make(a),g=make(b);return f()+g()+f();",
        "let make=x=>{const f=()=>x+a;return f;};return make(b)();",
        "let make=x=>{let f=()=>x+a;const old=f;f=()=>x-b;return old;};return make(b)();",
        "let make=x=>{let i=0;while(i<3){i++;if(x)return ()=>i+b;}return ()=>i;};return make(a)();",
        "let make=x=>{for(let i=0;i<3;i++){if(x)return ()=>i+b;}return ()=>x;};return make(a)();",
        "let make=x=>{do{return ()=>x+b;}while(x);};return make(a)();",
        "let make=x=>{{const y=x+b;return ()=>y;}};return make(a)();",
        "let make=x=>{const f=()=>{return x+b;};return ()=>f();};return make(a)();",
        "let make=(fn,x)=>{return ()=>fn(x);};return make(y=>y+b,a)();",
        "let make=x=>{return ()=>x+a;};const old=make;const f=make((make=x=>{return ()=>x-b;},b));return f()+make(a)()+old(b)();",
        "let n=0;let make=x=>{n++;if(x)return ()=>n+x;return ()=>n-x;};const f=make(a),g=make(b);return f()+g()+n;",
        "let make=x=>{return ()=>x+a;};make&&=x=>{return ()=>x-b;};return make(b)();",
        "let make=x=>{return ()=>x+a;};make??=x=>{return ()=>x-b;};return make(b)();",
        "const first=x=>{return ()=>x+a;};const second=x=>()=>x-b;let make=a?first:second;return make(b)();",
    ]);
}

#[test]
fn runtime_block_factories_keep_completion_signature_and_target_guards() {
    for body in [
        "let make=x=>{if(x)return ()=>a;};return a;",
        "let make=x=>{if(x)return ()=>a;return;};return a;",
        "let make=x=>{if(x)return ()=>a;return b;};return a;",
        "let make=x=>{if(x)return ()=>a;return y=>y;};return a;",
        "let make=x=>{while(x){return ()=>a;}};return a;",
        "let make=x=>{return later;const later=()=>a;};return a;",
        "let make=async x=>{return ()=>x;};return a;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typescript in [false, true] {
            assert!(compile(&source, typescript).is_err(), "{source}");
            assert!(compile_value(&source, typescript).is_err(), "{source}");
            assert!(
                compile_async(
                    &source.replace("export function", "export async function"),
                    typescript
                )
                .is_err(),
                "{source}"
            );
        }
    }
}

#[test]
fn runtime_block_factory_result_cursors_remain_bounded_on_a_small_stack() {
    for (count, admitted) in [(1, true), (8, true), (9, false)] {
        let mut expression = "a".to_string();
        for _ in 0..count {
            expression = format!("make({expression})()");
        }
        let source = format!(
            "export function run(a,b){{let make=x=>{{return ()=>x;}};return {expression};}}"
        );
        let result = std::thread::Builder::new()
            .name("runtime_block_factory_result_cursors_remain_bounded_on_a_small_stack".to_owned())
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(
            result.is_ok(),
            admitted,
            "{count} nested block factory results"
        );
        if let Ok(artifact) = result {
            assert_eq!(
                NumericInstance::new(&artifact.wasm)
                    .unwrap()
                    .run(4.0, 2.0)
                    .unwrap(),
                4.0
            );
        } else {
            assert!(result.err().unwrap().contains("8 active"));
        }
    }
}

#[test]
fn runtime_factories_return_further_factories() {
    compare_functions(&[
        "let make=x=>()=>()=>x;return make(a)()();",
        "let make=x=>y=>()=>x+y;return make(a)(b)();",
        "let make=a?(x=>y=>()=>x+y):(x=>y=>()=>x-y);return make(a)(b)();",
        "let make=x=>y=>z=>()=>x+y+z;return make(a)(b)(3)();",
        "let make=x=>{let z=x;return y=>()=>++z+y;};const middle=make(a);const f=middle(b),g=middle(a);return f()+g()+f();",
        "let make=x=>{return y=>{return ()=>x+y;};};return make(a)(b)();",
        "let make=x=>y=>()=>x+y;const middle=make(a);make=x=>y=>()=>x-y;return middle(b)()+make(a)(b)();",
        "let make=x=>y=>()=>x+y;let middle=make(a);const f=middle((middle=make(b),a));return f()+middle(b)();",
        "let state=()=>a;let make=x=>y=>()=>state()+x+y;const f=make(a)(b);state=()=>b;return f();",
        "let make=(fn,x)=>y=>()=>fn(x)+y;return make(z=>z+b,a)(b)();",
        "let make=x=>()=>()=>null;return +make(a)()();",
        "let make=x=>y=>()=>x+y;make&&=x=>y=>()=>x-y;return make(a)(b)();",
        "let make=x=>y=>()=>x+y;make??=x=>y=>()=>x-y;return make(a)(b)();",
    ]);
    compare_function_programs(&["let make=x=>{return ()=>()=>x;};return a;"], false);
}

#[test]
fn nested_runtime_factory_signatures_keep_small_stack_and_width_bounds() {
    for (depth, admitted) in [(1, true), (5, true), (8, false)] {
        let mut returned = "x".to_string();
        for _ in 0..depth {
            returned = format!("()=>{returned}");
        }
        let source = format!(
            "export function run(a,b){{let make=x=>{returned};return make(a){};}}",
            "()".repeat(depth)
        );
        let result = std::thread::Builder::new()
            .name("nested_runtime_factory_signatures_keep_small_stack_and_width_bounds".to_owned())
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(result.is_ok(), admitted, "{depth} returned function layers");
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
    let parameters = (0..1000)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let arguments = vec!["a"; 1000].join(",");
    compare_functions(&[&format!(
        "let make=x=>({parameters})=>()=>x+p0+p999;return make(b)({arguments})();"
    )]);
    let source =
        format!("export function run(a,b){{let make=x=>({parameters},extra)=>()=>x+p0;return a;}}");
    for typescript in [false, true] {
        assert!(compile(&source, typescript).is_err());
        assert!(compile_value(&source, typescript).is_err());
    }
}

#[test]
fn returned_functions_receive_numeric_leaf_callbacks() {
    compare_function_programs(
        &[
            "let make=x=>{return (fn,y)=>fn(y);};return a;",
            "let make=x=>(fn,y)=>()=>fn(y);return a;",
        ],
        false,
    );
    compare_functions(&[
        "let make=x=>{return (fn,y)=>fn(y)+x;};return make(a)(z=>z+b,a);",
        "let make=x=>(fn,y)=>fn(y)+x;const apply=make(a);return apply(z=>z+b,a);",
        "let make=a?(x=>(fn,y)=>fn(y)+x):(x=>(fn,y)=>fn(y)-x);return make(a)(z=>z+b,b);",
        "let make=x=>(fn,y)=>()=>fn(y)+x;const f=make(a)(z=>z+b,a);return f();",
        "let make=x=>{return (fn,y)=>{return ()=>fn(y)+x;};};return make(a)(z=>z+b,b)();",
        "let make=x=>(fn,y)=>{const old=fn;fn=z=>z+b;return old(y)+fn(y)+x;};return make(a)(z=>z+a,b);",
        "let make=x=>(fn,y)=>{const read=()=>fn(y);fn=z=>z+b;return read()+x;};return make(a)(z=>z+a,b);",
        "let make=x=>(f,g,y)=>f(y)+g(y)+x;return make(a)(z=>z+a,z=>z-b,b);",
        "let make=x=>(fn,y)=>()=>fn(y);let fn=z=>z+a;const f=make(a)(fn,(fn=z=>z-b,b));return f()+fn(a);",
        "let make=x=>(fn,y)=>()=>fn(y);const apply=make(a);const f=apply(z=>z+a,b);make=x=>(fn,y)=>()=>fn(y)+x;return f()+make(a)(z=>z-b,b)();",
        "let make=x=>(fn,y)=>fn(y)+x;let apply=make(a);return (apply&&=(fn,y)=>fn(y)-b)(z=>z+a,b);",
    ]);
}

#[test]
fn runtime_factory_results_require_matching_parameter_and_result_chains() {
    for body in [
        "const make=a?()=>()=>a:()=>x=>x;return make()();",
        "const make=a?()=>()=>a:()=>b;return make()();",
        "let make=x=>()=>x;make=x=>y=>x+y;return make(a)();",
        "let make=x=>()=>x;make=x=>x;return make(a)();",
        "let make=x=>()=>null;make=x=>y=>false;return make(a)();",
        "let make=x=>y=>()=>x+y;make=x=>()=>()=>x;return a;",
        "let make=x=>()=>x;return make(a);",
        "let make=x=>()=>x;return +make(a);",
        "let make=x=>()=>x;return make?.(a)();",
        "let make=x=>()=>x;return make(...a)();",
        "let make=x=>()=>x;return make()();",
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
fn returned_function_callbacks_keep_numeric_leaf_kind_and_arity_guards() {
    for body in [
        "let make=x=>(fn,y)=>fn(y);return make(a)(a,b);",
        "let make=x=>(fn,y)=>fn(y);return make(a)(async z=>z,a);",
        "let make=x=>fn=>fn(1);const apply=g=>g(1);return make(a)(apply);",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typescript in [false, true] {
            assert!(compile(&source, typescript).is_err(), "{source}");
            assert!(compile_value(&source, typescript).is_err(), "{source}");
        }
    }
}

#[test]
fn returned_callback_argument_frames_keep_the_existing_width_limit() {
    let parameters = (0..999)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let arguments = vec!["a"; 999].join(",");
    compare_functions(&[&format!(
        "let make=x=>(fn,{parameters})=>fn(p998)+x;return make(a)(z=>z+b,{arguments});"
    )]);
    let source = format!(
        "export function run(a,b){{let make=x=>(fn,{parameters},extra)=>fn(p998)+x;return a;}}"
    );
    for typescript in [false, true] {
        assert!(compile(&source, typescript).is_err());
        assert!(compile_value(&source, typescript).is_err());
    }
}

#[test]
fn primitive_factory_leaves_keep_numeric_coercion_and_predicate_positions() {
    compare_functions(&[
        "let make=x=>()=>true;return +make(a)();",
        "let make=x=>()=>false;return +make(a)();",
        "let make=x=>()=>null;return +make(a)();",
        "let make=x=>()=>undefined;return +make(a)();",
        "let make=x=>()=>x?null:false;return make(a)()?a:b;",
        "let make=x=>()=>{if(x)return b;};return +make(a)();",
        "let make=x=>()=>x?null:false;const f=make(a);return +(f??make(b))();",
    ]);
}

#[test]
fn runtime_factory_calls_bound_native_cursor_reentry() {
    for (count, admitted) in [(8, true), (9, false)] {
        let mut expression = "a".to_string();
        for _ in 0..count {
            expression = format!("make({expression})()");
        }
        let source = format!("export function run(a,b){{let make=x=>()=>x;return {expression};}}");
        let result = std::thread::Builder::new()
            .name("runtime_factory_calls_bound_native_cursor_reentry".to_owned())
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        if admitted {
            let artifact = result.unwrap();
            let mut host = NumericInstance::new(&artifact.wasm).unwrap();
            assert_eq!(host.run(4.0, 2.0).unwrap(), 4.0);
        } else {
            let error = result.err().expect("runtime result frames must be bounded");
            assert!(error.contains("8 active"), "{error}");
        }
    }
}

#[test]
fn runtime_factories_keep_wide_lists_and_erased_types() {
    let parameters = (0..1000)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let arguments = vec!["a"; 1000].join(",");
    compare_functions(&[
        &format!("let make=({parameters})=>()=>p0+p999+b;return make({arguments})();"),
        &format!("let make=x=>({parameters})=>p0+p999+x;return make(b)({arguments});"),
    ]);
    let plain = "export function run(a,b){let make=x=>()=>x+b;return make(a)();}";
    let typed = "export function run(a:never,b:Missing):Wrong {let make:Wrong=((x:Missing)=>((()=>x+b) as Wrong)) as any;return ((make as any)(a as never) as Wrong)();}";
    assert_eq!(
        compile(plain, false).unwrap().wasm,
        compile(typed, true).unwrap().wasm
    );
    let plain = plain.replace("export function", "export async function");
    let typed = typed.replace("export function", "export async function");
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
}

#[test]
fn runtime_factory_calls_share_the_existing_expansion_limit() {
    for (calls, admitted) in [(127, true), (128, false)] {
        let expressions = vec!["make(a)()"; calls].join(",");
        let source =
            format!("export function run(a,b){{let make=x=>()=>x;return ({expressions});}}");
        let result = compile(&source, false);
        assert_eq!(result.is_ok(), admitted, "{calls} factory/leaf pairs");
        if !admitted {
            assert!(result.err().unwrap().contains("256"));
        }
    }
}

#[test]
fn runtime_factory_pairs_keep_primitive_tags_and_recover_after_fuel_failure() {
    let artifact = compile_value(
        "export function run(a,b){let make=x=>()=>x+b;return a?make(a)():true;}",
        false,
    )
    .unwrap();
    let mut host = ValueInstance::new(&artifact.wasm).unwrap();
    assert!(matches!(host.run(4.0, 2.0).unwrap(), Value::Number(6.0)));
    assert!(matches!(host.run(0.0, 2.0).unwrap(), Value::Boolean(true)));
    node(&format!(
        "const assert=require('node:assert/strict');const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({:?}))).exports;assert.deepEqual(e.run(4,2),[3,6]);assert.deepEqual(e.run(0,2),[2,1]);",
        artifact.wasm
    ));
    let artifact=compile("export function run(a,b){let make=x=>()=>x;if(a){make=x=>make(x);return make(a)();}return make(b)();}",false).unwrap();
    let mut host = NumericInstance::new(&artifact.wasm).unwrap();
    let error = host.run(1.0, 2.0).unwrap_err();
    assert!(format!("{error:#}").contains("fuel"), "{error:#}");
    assert_eq!(host.run(0.0, 2.0).unwrap(), 2.0);
}

#[test]
fn nullish_function_results_keep_the_left_code_capture_and_effects() {
    compare_functions(&[
        "const f=()=>a;const g=()=>b;return (f??g)();",
        "let calls=0;const make=x=>(calls++,()=>x);const h=make(a++)??make(b++);return h()+calls*100+a*10+b;",
        "let f=()=>a;const g=()=>b;const h=f??(f=g);f=g;return h()*10+f();",
        "const make=x=>()=>++x;const h=make(a)??make(b);const alias=h;return h()*10+alias();",
        "const f=()=>a;const g=()=>b;const h=(a?f:g)??(a?g:f);return h();",
        "const f=()=>a;const g=()=>b;const pick=(x,y)=>{if(a)return x??y;return ()=>b;};return pick(f,g)();",
        "const f=()=>()=>a;const g=()=>()=>b;const h=f??g;return h()();",
        "const f=()=>a;const g=()=>b;const h=(f??g)||g;return h();",
        "const f=()=>a;const g=()=>b;const h=f??(f&&g);return h();",
    ]);
}

#[test]
fn nullish_function_results_preserve_callback_signatures() {
    compare_functions(&[
        "const f=(fn,x)=>fn(x);const g=(fn,x)=>fn(fn(x));const h=f??g;const relay=(cb,x)=>h(cb,x);return relay(x=>x+b,a);",
        "const make=()=>{return ((fn,x)=>fn(x))??((fn,x)=>fn(fn(x)));};const h=make();const relay=(cb,x)=>h(cb,x);return relay(x=>x+b,a);",
    ]);
}

#[test]
fn nullish_function_results_work_in_numeric_accumulators() {
    compare_functions(&[
        "const f=x=>x+b;const g=x=>x-b;const h=f??g;let s=a;for(let i=0;i<16;i++)s=h(s);return s;",
    ]);
}

#[test]
fn logical_function_results_keep_the_selected_code_and_capture() {
    compare_functions(&[
        "const f=()=>a;const g=()=>b;const h=f&&g;return h();",
        "const f=()=>a;const g=()=>b;const h=f||g;return h();",
        "const f=()=>a;const g=()=>b;return (f&&g)()+(f||g)();",
        "let calls=0;const f=()=>++calls;const g=()=>--calls;const h=(calls+=10,f)||(calls+=100,g);return h()*1000+calls;",
        "let calls=0;const f=()=>++calls;const g=()=>--calls;const h=(calls+=10,f)&&(calls*=2,g);return h()*1000+calls;",
        "let f=()=>a;const old=f;const h=f||(f=()=>b);f=()=>b;return h()*100+old()*10+f();",
        "let f=()=>a;const h=f&&(f=()=>b);return h()*10+f();",
        "const make=x=>()=>x;let n=0;const h=make((n++,a))&&make((n++,b));return h()+n*100;",
        "const make=x=>()=>x;let n=0;const h=make((n++,a))||make((n++,b));return h()+n*100;",
        "const f=()=>a;const g=()=>b;const h=a?(f&&g):(g||f);return h();",
        "const f=()=>a;const g=()=>b;let h=()=>0;const read=()=>h();h=(f&&g)||f;return read();",
        "const f=()=>a;const g=()=>b;const h=(f||g)&&g;return (h?1:2)+h();",
        "const f=x=>x+b;const g=x=>x-b;const h=f&&g;let s=0;for(let i=0;i<16;i++)s=h(s);return s+a;",
        "const f=x=>x+b;const g=x=>x-b;const h=f||g;let s=0;for(let i=0;i<16;i++)s=h(s);return s+a;",
    ]);
}

#[test]
fn logical_function_results_preserve_higher_order_and_returned_signatures() {
    compare_functions(&[
        "const apply=(fn,x)=>fn(x);const twice=(fn,x)=>fn(fn(x));const h=apply&&twice;return h(x=>x+b,a);",
        "const apply=(fn,x)=>fn(x);const twice=(fn,x)=>fn(fn(x));const h=apply||twice;return h(x=>x+b,a);",
        "const apply=(fn,x)=>fn(x);const twice=(fn,x)=>fn(fn(x));const h=apply&&twice;const relay=(g,x)=>h(g,x);return relay(x=>x+b,a);",
        "const make=()=>{return ((fn,x)=>fn(x))||((fn,x)=>fn(fn(x)));};const h=make();const relay=(g,x)=>h(g,x);return relay(x=>x+b,a);",
        "const pick=(f,g)=>f&&g;const apply=(fn,x)=>fn(x);const twice=(fn,x)=>fn(fn(x));const h=pick(apply,twice);return h(x=>x+b,a);",
        "const pick=(f,g)=>{return f||g;};const apply=(fn,x)=>fn(x);const twice=(fn,x)=>fn(fn(x));return pick(apply,twice)(x=>x+b,a);",
        "const make=()=>{return ((fn,x)=>fn(x))||((fn,x)=>fn(fn(x)));};return make()(x=>x+b,a);",
        "const f=()=>()=>a;const g=()=>()=>b;const h=f&&g;return h()();",
    ]);
}

#[test]
fn logical_function_results_keep_primitive_tags_and_erase_type_claims() {
    for operator in ["&&", "||", "??"] {
        let plain = format!(
            "export function run(a,b){{const f=()=>a;const g=()=>b;const h=f{operator}g;return h();}}"
        );
        let typed = format!(
            "export function run(a:never,b:Missing):Wrong {{const f=()=>a;const g=()=>b;const h=((f as Missing){operator}(g satisfies Wrong)) as never;return (h as number)();}}"
        );
        let artifact = compile_value(&plain, false).unwrap();
        assert_eq!(artifact.wasm, compile_value(&typed, true).unwrap().wasm);
        let mut host = ValueInstance::new(&artifact.wasm).unwrap();
        for (a, b) in [(4.0, 2.0), (-0.0, 0.0), (f64::NAN, f64::INFINITY)] {
            let expected = if operator == "&&" { b } else { a };
            let Value::Number(actual) = host.run(a, b).unwrap() else {
                panic!("selected numeric function result must keep its host tag");
            };
            assert_eq!(bits(actual), bits(expected));
        }
        node(&format!(
            "const assert=require('node:assert/strict');const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({:?}))).exports;assert.deepEqual(e.run(4,2),[3,{}]);",
            artifact.wasm,
            if operator == "&&" { 2 } else { 4 }
        ));
    }
}

#[test]
fn logical_function_results_keep_source_depth_limits_on_a_small_stack() {
    for operator in ["&&", "||", "??"] {
        for (depth, admitted) in [(32, true), (128, false)] {
            let mut expression = String::from("f");
            for _ in 0..depth {
                expression = format!("({expression}{operator}f)");
            }
            let source = format!(
                "export function run(a,b){{const f=()=>a;const h={expression};return h();}}"
            );
            let result = std::thread::Builder::new()
                .name(
                    "logical_function_results_keep_source_depth_limits_on_a_small_stack".to_owned(),
                )
                .stack_size(2 * 1024 * 1024)
                .spawn(move || compile(&source, false))
                .unwrap()
                .join()
                .unwrap();
            assert_eq!(result.is_ok(), admitted, "{operator} depth {depth}");
            if let Ok(artifact) = result {
                assert_eq!(
                    NumericInstance::new(&artifact.wasm)
                        .unwrap()
                        .run(4.0, 2.0)
                        .unwrap(),
                    4.0
                );
            } else if let Err(error) = result {
                assert!(
                    error.contains("nesting") || error.contains("depth"),
                    "{error}"
                );
            }
        }
    }
}

#[test]
fn logical_function_results_reject_mixed_signatures_kinds_and_host_transport() {
    for operator in ["&&", "||", "??"] {
        for body in [
            format!("const f=()=>a;const g=x=>x;const h=f{operator}g;return h();"),
            format!(
                "const f=(fn,x)=>fn(x);const g=(fn,x)=>fn+x;const h=f{operator}g;return h(y=>y,b);"
            ),
            format!("const f=()=>a;const h=f{operator}b;return h();"),
            format!("const f=()=>a;const h=b{operator}f;return h();"),
            format!("const f=()=>a;const g=()=>b;return f{operator}g;"),
            format!("const f=()=>a;const g=()=>b;return +(f{operator}g);"),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for typescript in [false, true] {
                assert!(compile(&source, typescript).is_err(), "{source}");
                assert!(compile_value(&source, typescript).is_err(), "{source}");
            }
        }
        let source = format!(
            "export function run(a,b){{const f=()=>a;const g=()=>b;return (f{operator}g)===f;}}"
        );
        for typescript in [false, true] {
            assert!(
                compile(&source, typescript).is_err(),
                "numeric exports cannot return Boolean"
            );
        }
    }
}

#[test]
fn nullish_selection_accepts_constructor_free_generic_formals() {
    // Exact former rejection: numeric preliminary checking must not block the
    // actual function-kind specialization of this constructor-free selector.
    let source = "export function run(a,b){const f=()=>a;const g=()=>b;const pick=(x,y)=>x??y;return pick(f,g)();}";
    compare_functions(&[source
        .strip_prefix("export function run(a,b){")
        .unwrap()
        .strip_suffix('}')
        .unwrap()]);
}

#[test]
fn function_values_are_truthy_without_invoking_their_bodies() {
    compare_function_programs(
        &[
            "let make=x=>()=>x;if(make(a))return a;return b;",
            "let calls=0;let make=x=>(calls++,()=>++calls);return (make?10:20)+calls;",
            "let calls=0;let make=x=>(calls++,()=>++calls);return (make(a++)?1:2)*100+calls*10+a;",
            "let calls=0;const f=a?()=>++calls:()=>--calls;return (f?10:20)+calls;",
            "const f=()=>a;if(f)return b;return a;",
            "const f=a?()=>a:()=>b;return +!f+ +!!f;",
            "return (()=>a)?b:a;",
            "let f=()=>a;const old=f;f=()=>b;return (old?10:20)+(f?1:2);",
            "let calls=0;const f=()=>++calls;const g=()=>--calls;if(f&&g)return calls+1;return 99;",
            "let calls=0;const f=()=>++calls;if(f||(calls++,0))return calls;return 99;",
            "let calls=0;const f=()=>++calls;if(0&&(calls++,f))return 99;return calls;",
            "let calls=0;const f=()=>++calls;if(f&&(calls++,0))return 99;return calls;",
            "const f=()=>a;return (a?f:0)?b:a;",
            "let calls=0;const f=()=>++calls;const g=()=>--calls;return ((f??g)?10:20)+calls;",
            "const f=()=>a;return (a?f:(x=>x))?b:a;",
            "let calls=0;const make=x=>{calls++;return ()=>++calls;};return (make(a++)?1:2)*100+calls*10+a;",
            "let f=()=>0;let calls=0;return (f=(calls++,()=>++calls))?calls:99;",
            "const id=fn=>fn;let calls=0;const f=a?()=>++calls:()=>--calls;return (id((calls++,f))?10:20)+calls;",
            "const get=()=>{a++;return ()=>b++;};if(get())return a+b;return 0;",
            "const h=a?(fn,x)=>fn(x):(fn,x)=>fn(fn(x));return h?10:20;",
            "const f=()=>a;const read=()=>f?b:a;return read();",
            "const f=()=>a;const read=a?()=>f?b:a:()=>!f?b:a;return read();",
            "let calls=0;const f=()=>++calls;let i=0;while(f){i++;if(i>=2)break;}return i+calls;",
            "let calls=0;const f=()=>++calls;let i=0;for(;f;i++){if(i>=2)break;}return i+calls;",
            "let calls=0;const f=()=>++calls;let i=0;do{i++;if(i>=2)break;}while(f);return i+calls;",
            "let calls=0;const f=()=>++calls;let i=0;while((calls++,f)){i++;if(i>=2)break;}return i*10+calls;",
        ],
        false,
    );
}

#[test]
fn function_negation_keeps_primitive_boolean_results_and_numeric_boundaries() {
    let plain = "export function run(a,b){const f=a?()=>a:()=>b;return (f?1:2)+ +!f;}";
    let typed = "export function run(a:never,b:Missing):Wrong {const f:Missing=(a?()=>a:()=>b) as unknown as Missing;return ((f as never)?1:2)+ +!(f satisfies Wrong);}";
    assert_eq!(
        compile(plain, false).unwrap().wasm,
        compile(typed, true).unwrap().wasm
    );
    for (expression, expected) in [("!f", false), ("!!f", true)] {
        let source =
            format!("export function run(a,b){{const f=a?()=>a:()=>b;return {expression};}}");
        for typescript in [false, true] {
            assert!(
                compile(&source, typescript).is_err(),
                "numeric profile still requires explicit conversion"
            );
            let artifact = compile_value(&source, typescript).unwrap();
            let mut host = ValueInstance::new(&artifact.wasm).unwrap();
            assert!(matches!(host.run(0.0,2.0).unwrap(),Value::Boolean(value) if value==expected));
            node(&format!(
                "const assert=require('node:assert/strict');const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({:?}))).exports;assert.deepEqual(e.run(0,2),[2,{}]);",
                artifact.wasm,
                u8::from(expected)
            ));
        }
    }
    for body in [
        "const f=a?()=>a:()=>b;return +f;",
        "const f=a?()=>a:()=>b;return f+1;",
        "const f=a?()=>a:()=>b;return f;",
        "const f=a?()=>a:()=>b;return f&&b;",
        "const f=a?()=>a:()=>b;return f||b;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for typescript in [false, true] {
            assert!(compile(&source, typescript).is_err(), "{source}");
            assert!(compile_value(&source, typescript).is_err(), "{source}");
        }
    }
}

#[test]
fn function_truthiness_constructor_checks_are_bounded_on_a_small_native_stack() {
    for (depth, admitted) in [(2, true), (20, false)] {
        let mut expression = String::from("a");
        for _ in 0..depth {
            expression = format!("((()=>{expression})?a:b)");
        }
        let source = format!("export function run(a,b){{return {expression};}}");
        let result = std::thread::Builder::new()
            .name(format!("truthiness_depth_{depth}"))
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(result.is_ok(), admitted);
        if let Err(error) = result {
            assert!(
                error.contains("function-value") || error.contains("closure lowering"),
                "{error}"
            );
        }
    }
    for (regions, admitted) in [(40, true), (70, false)] {
        let source = format!(
            "export function run(a,b){{let cb=()=>a;const get=()=>{{return {}cb;}};return {}(get()?a:b);}}",
            "a?cb:".repeat(regions),
            "a?0:".repeat(regions)
        );
        let result = std::thread::Builder::new()
            .name(format!("truthiness_regions_{regions}"))
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        match result {
            Ok(artifact) => {
                assert!(admitted);
                assert_eq!(
                    NumericInstance::new(&artifact.wasm)
                        .unwrap()
                        .run(0.0, 2.0)
                        .unwrap(),
                    0.0
                );
            }
            Err(error) => {
                assert!(!admitted);
                assert!(error.contains("128"), "{error}");
            }
        }
    }
}

#[test]
fn promise_returning_callbacks_keep_synchronous_profile_guards() {
    for body in [
        "let make=x=>async()=>x;return make(a)();",
        "const h=a?(fn,x)=>fn(x):(fn,x)=>fn(x);return h(async y=>y,b);",
        "const apply=(fn,x)=>fn(x);return apply(async x=>x,b);",
        "const id=fn=>fn;const f=id(async()=>a);return f();",
    ] {
        for typescript in [false, true] {
            let source = format!("export function run(a,b){{{body}}}");
            assert!(compile(&source, typescript).is_err(), "{source}");
            assert!(compile_value(&source, typescript).is_err(), "{source}");
        }
    }
}
