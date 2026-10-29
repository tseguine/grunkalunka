use super::*;

fn compare_named(programs: &[&str]) {
    compare_async_sources(
        &programs
            .iter()
            .map(|source| (*source).to_string())
            .collect::<Vec<_>>(),
        &[
            (4.0, 2.0),
            (0.0, -0.0),
            (-2.0, -4.0),
            (f64::NAN, f64::INFINITY),
        ],
    );
}

#[test]
fn named_async_helpers_accept_primitive_inputs() {
    compare_named(&[
        "async function f(x){return x;}export async function run(a,b){return await f(null);}",
        "async function f(x){return x;}export async function run(a,b){return await f(false);}",
        "async function f(x){await x;return x;}export async function run(a,b){return await f(undefined);}",
        "async function f(x){const old=x;await x;return old;}export async function run(a,b){return await f(a?null:false);}",
        "async function f(x){await x;return x;}export async function run(a,b){const p=f(a),q=f(null);const l=await p,r=await q;return l===a&&r===null;}",
        "async function f(x){return x;}export async function run(a,b){f(null);return await f(a);}",
        "async function f(x){const read=()=>x;await x;x++;return read();}export async function run(a,b){return await f(null);}",
    ]);
}

#[test]
fn named_async_primitive_inputs_follow_aliases_and_forwarding() {
    compare_named(&[
        "async function f(x){await x;return x;}export async function run(a,b){const h=f;return await h(null);}",
        "async function f(x){return x;}async function g(x){return x;}export async function run(a,b){const h=a?f:g;return await h(false);}",
        "async function f(x){return x;}export async function run(a,b){const apply=(fn,x)=>fn(x);return await apply(f,null);}",
        "async function f(x){return x;}export async function run(a,b){const id=fn=>fn;return await id(f)(false);}",
        "async function f(x){return x;}export async function run(a,b){const make=()=>f;return await make()(null);}",
        "async function f(x){return await g(x);}async function g(x){return x;}export async function run(a,b){return await f(false);}",
    ]);
}

#[test]
fn named_async_primitive_inputs_preserve_mixed_frames_and_cells() {
    compare_named(&[
        "function add(x,y){return x+y;}async function f(n,x,m){await x;return x===null?add(n,m):n;}export async function run(a,b){return await f(a,null,b);}",
        "async function f(n,x,m){const read=()=>x;const old=x;await n;x=m;return old===null?read():old;}export async function run(a,b){const p=f(a,null,b),q=f(b,false,a);const x=await p,y=await q;return y===false?x:y;}",
        "async function f(x,n){const read=()=>x;await n;x??=n;return read();}export async function run(a,b){return await f(a?null:false,b);}",
        "async function f(n,x){if(n<=0)return x;await n;return await f(n-1,x);}export async function run(a,b){return await f(3,a?null:false);}",
        "async function f(n,x){if(n<=0)return x;return await g(n-1,x);}async function g(n,x){if(n<=0)return x;await n;return await f(n-1,x);}export async function run(a,b){return await f(4,null);}",
        "async function f(x){const read=()=>x;await x;x=!x;return read();}export async function run(a,b){const p=f(null),q=f(false);const x=await q,y=await p;return x===y;}",
    ]);
    for count in [17, 32] {
        let parameters = (0..count)
            .map(|i| format!("p{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let arguments = (0..count)
            .map(|i| match i {
                1 => "null",
                5 => "false",
                _ => "a",
            })
            .collect::<Vec<_>>()
            .join(",");
        let source = format!(
            "async function f({parameters}){{await p1;return p1===null&&p5===false?p0+p{}:-1;}}export async function run(a,b){{return await f({arguments});}}",
            count - 1
        );
        compare_named(&[&source]);
    }
}

#[test]
fn named_async_primitive_inputs_preserve_argument_and_rejection_order() {
    compare_named(&[
        "async function f(x,y){await y;return x;}export async function run(a,b){let x=null;return await f(x,await(x=false));}",
        "async function f(x){return x;}export async function run(a,b){let x=null;const p=f(x,x=false);return (await p)===null&&x===false;}",
        "async function f(x){throw x;}export async function run(a,b){let n=0;try{return await f(null,await Promise.reject(b),n=1);}catch(e){return n;}}",
        "async function f(x,y){throw x;}export async function run(a,b){let n=0;try{return await f(await Promise.reject(null),n=1);}catch(e){return n;}}",
        "async function f(x){try{throw x;}finally{await x;}}export async function run(a,b){try{return await f(false);}catch(e){return e;}}",
        "async function f(x){return Promise.reject(x);}export async function run(a,b){try{return await f(null);}catch(e){return e;}}",
    ]);
}

#[test]
fn named_async_primitive_inputs_keep_kind_shape_and_erasure_guards() {
    let plain = "async function f(x,n){const read=()=>x;await n;return read();}export async function run(a,b){return await f(a?null:false,b);}";
    let typed = "async function f<T>(x:never,n:Wrong):Promise<number>{const read=()=>x;await(n as any);return read() as never;}export async function run(a:never,b:Wrong):Promise<string>{return await f((a?null:false) as number,b as never);}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    compare_named(&[plain]);
    for value in ["'x'", "[]", "({})", "1n", "TypeError()", "x=>x", "missing"] {
        let source = format!(
            "async function f(x){{return x;}}export async function run(a,b){{return await f({value});}}"
        );
        for ts in [false, true] {
            assert!(compile_async(&source, ts).is_err(), "{source}");
        }
    }
}

#[test]
fn named_async_helpers_run_before_and_after_their_declaration() {
    compare_named(&[
        "export async function run(a,b){return await twice(a);}async function twice(x){return (await x)*2;}",
        "async function twice(x){return (await x)*2;}export async function run(a,b){return await twice(a);}",
        "async function first(x){return await second(x);}export async function run(a,b){return await first(a);}async function second(x){return (await x)+1;}",
        "function add(x,y){return x+y;}async function work(x){return add(await x,1);}export async function run(a,b){return await work(a);}",
        "async function work(x){await x;return x;}function launch(x){work(x);return x+1;}export async function run(a,b){return launch(a);}",
        "async function zero(){return -0;}export async function run(a,b){return await zero();}",
    ]);
}

#[test]
fn named_async_helpers_preserve_primitive_settlement_and_finally_adoption() {
    let mut programs = Vec::new();
    for value in ["x", "null", "undefined", "false", "true", "-0", "0/0"] {
        programs.push(format!("async function f(x){{await x;return {value};}}export async function run(a,b){{return await f(a);}}"));
        programs.push(format!("async function f(x){{throw {value};}}export async function run(a,b){{try{{return await f(a);}}catch(e){{return e;}}}}"));
    }
    let extra = [
        "async function f(x){return;}export async function run(a,b){return await f(a);}",
        "async function f(x){if(x)return false;}export async function run(a,b){return await f(a);}",
        "async function f(x){return Promise.resolve(x);}export async function run(a,b){return await f(a);}",
        "async function f(x){try{return Promise.reject(x);}catch(e){return 999;}}export async function run(a,b){try{return await f(b);}catch(e){return e;}}",
        "async function f(x){try{return g(x);}finally{await x;}}async function g(x){throw x;}export async function run(a,b){try{return await f(b);}catch(e){return e;}}",
        "async function f(x){try{throw x;}finally{return null;}}export async function run(a,b){return await f(a);}",
        "async function f(x){try{await Promise.reject(x);}catch(e){await x;return e;}}export async function run(a,b){return await f(b);}",
    ];
    programs.extend(extra.iter().map(|s| s.to_string()));
    compare_named(&programs.iter().map(String::as_str).collect::<Vec<_>>());
}

#[test]
fn named_async_helpers_keep_identities_forwarding_and_callable_references() {
    compare_named(&[
        "async function f(x){await x;return x;}export async function run(a,b){const p=f(a),q=f(a);await p;await q;return p!==q;}",
        "async function f(x){await x;return x;}export async function run(a,b){const p=f(a),q=p;await q;return p===q;}",
        "async function f(x){await x;return x;}export async function run(a,b){const relay=x=>f(x);return await relay(a);}",
        "async function f(x){await x;return x;}export async function run(a,b){const relay=async x=>f(x);return await relay(a);}",
        "async function f(x){await x;return x;}export async function run(a,b){const make=()=>x=>f(x);const call=make();return await call(b);}",
        "async function f(x){await x;return x;}export async function run(a,b){const h=f;return await h(b);}",
        "async function f(x){await x;return x;}async function g(x){await x;return -x;}export async function run(a,b){let h=a?f:g;return await h(b);}",
        "async function f(x){await x;return x;}export async function run(a,b){const apply=(fn,x)=>fn(x);return await apply(f,b);}",
    ]);
}

#[test]
fn named_async_helpers_preserve_argument_order_and_skipped_startup_on_failure() {
    compare_named(&[
        "async function f(x,y){await x;return x-y;}export async function run(a,b){return await f(a,await(a=b));}",
        "async function f(x){await x;return x;}export async function run(a,b){let n=0;const p=f((n=n*10+1,a),n=n*10+2);await p;return n;}",
        "async function f(x,y){throw x;}export async function run(a,b){let n=0;try{return await f(await Promise.reject(b),n=1);}catch(e){return n;}}",
        "async function f(x){throw x;}export async function run(a,b){try{return await f(a,await Promise.reject(b));}catch(e){return e;}}",
        "async function f(x){const read=()=>x;const old=read();await x;x++;return old;}export async function run(a,b){const p=f(a);a=b;return await p;}",
        "async function f(x){const inc=()=>++x;await x;return inc()+inc();}export async function run(a,b){const p=f(a),q=f(b);return +(await p)+(+(await q));}",
    ]);
}

#[test]
fn named_async_helpers_keep_recursive_and_nested_activation_ownership() {
    compare_named(&[
        "async function f(n){if(n<=0)return -0;const old=await f(n-1);return old+n;}export async function run(a,b){return (await f(4))+a;}",
        "async function f(n){if(n<=0)return n;return await g(n-1);}async function g(n){if(n<=0)return n;return await f(n-1);}export async function run(a,b){return await f(4);}",
        "async function f(x){const read=()=>x;await x;x++;return read();}export async function run(a,b){const p=f(a),q=f(b);const x=await q;return x+(+(await p));}",
        "async function f(x){const g=async()=>{await x;return x;};return await g();}export async function run(a,b){return await f(a);}",
        "async function f(x){const g=async()=>{await x;return x;};let p=g();const h=async()=>{await x;return p;};p=h();try{await p;}catch(e){return e;}}export async function run(a,b){return await f(a);}",
    ]);
}

#[test]
fn named_async_helpers_keep_erasure_width_source_and_domain_boundaries() {
    let plain =
        "async function f(x){return await x;}export async function run(a,b){return await f(a);}";
    let typed = "async function f<T>(x:Missing):Wrong{return await(x as never);}export async function run(a:Bad,b:never):Wrong{return await f(a as any);}";
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    for count in [17, 32] {
        let params = (0..count)
            .map(|i| format!("p{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let args = vec!["a"; count].join(",");
        let source = format!(
            "async function f({params}){{await p0;return p0+p{};}}export async function run(a,b){{return await f({args});}}",
            count - 1
        );
        compare_async_sources(&[source], &[(4.0, 2.0), (0.0, -0.0)]);
    }
    // Admission is separate from the normal adapter's per-boundary instruction
    // fuel. Execute the widest physical activation under an explicit probe budget.
    let params = (0..1000)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let args = vec!["a"; 1000].join(",");
    let wide = format!(
        "async function f({params}){{await p0;return p0+p999;}}export async function run(a,b){{return await f({args});}}"
    );
    for ts in [false, true] {
        let wasm = compile_async(&wide, ts).unwrap().wasm;
        let limited = match AsyncInstance::new(&wasm, 4.0, 2.0) {
            Err(error) => error,
            Ok(mut run) => loop {
                match run.run_one_job() {
                    Ok(true) => (),
                    Ok(false) => panic!("wide activation unexpectedly fit the default fuel"),
                    Err(error) => break error,
                }
            },
        };
        assert!(format!("{limited:?}").contains("fuel"));
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);
        let engine = wasmtime::Engine::new(&config).unwrap();
        let module = wasmtime::Module::new(&engine, &wasm).unwrap();
        let mut store = wasmtime::Store::new(&engine, ());
        store.set_fuel(200_000).unwrap();
        let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
        let start = instance
            .get_typed_func::<(f64, f64), (i32, i32, f64)>(&mut store, "start")
            .unwrap();
        let resume = instance
            .get_typed_func::<(i32, f64), (i32, i32, f64)>(&mut store, "resume")
            .unwrap();
        let queued = instance.get_func(&mut store, "take_jobs").is_some();
        let mut state = start.call(&mut store, (4.0, 2.0)).unwrap();
        let mut jobs = 0;
        while state.0 == 1 {
            jobs += 1;
            assert!(jobs <= 32);
            store.set_fuel(200_000).unwrap();
            let input = if queued { (0, 0.0) } else { (state.1, state.2) };
            state = resume.call(&mut store, input).unwrap();
        }
        assert_eq!(state, (0, 3, 8.0));
        assert_eq!(jobs, 2);
        run_node(&format!(
            r#"import assert from 'node:assert/strict';import {{pathToFileURL}} from 'node:url';const {{createAsyncFunction}}=await import(pathToFileURL(process.argv[2]));const native=(()=>{{{};return run;}})();assert.equal(await native(4,2),8);assert.equal(await createAsyncFunction(new Uint8Array({wasm:?}))(4,2),8);"#,
            wide.replacen("export async function run", "async function run", 1)
        ));
    }
    for count in [31, 32] {
        let source = format!(
            "async function f(x){{return await x;}}export async function run(a,b){{{}return b;}}",
            "await f(a);".repeat(count)
        );
        assert_eq!(
            compile_async(&source, false).is_ok(),
            count == 31,
            "{count} root plus1 helper source awaits"
        );
    }
    for ts in [false, true] {
        for source in [
            "async function f(x){return x;}export function run(a,b){return a;}",
            "async function f(x,y){return x;}export async function run(a,b){return await f(a);}",
            "async function f(x=1){return x;}export async function run(a,b){return await f(a);}",
            "async function f(...x){return a;}export async function run(a,b){return await f(a);}",
            "async function* f(x){return x;}export async function run(a,b){return await f(a);}",
            "async function f(x){return this;}export async function run(a,b){return await f(a);}",
            "async function f(x){return arguments;}export async function run(a,b){return await f(a);}",
            "async function f(x){return x;}export async function run(a,b){f=a;return await f(a);}",
            "async function f(x){return x;}export async function run(a,b){return await f?.(a);}",
        ] {
            assert!(compile_async(source, ts).is_err(), "{source}");
        }
    }
}
