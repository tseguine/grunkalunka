use super::*;

fn compare_sync_finalizers(sources: &[&str]) {
    compare_async_sources(
        &sources
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
fn async_callers_compose_with_local_and_named_synchronous_finalizers() {
    compare_sync_finalizers(&[
        "function keep(x){try{return x;}finally{x++;}}export async function run(a,b){const first=keep(a);await b;return first+keep(b);}",
        "export async function run(a,b){let n=0;const keep=x=>{try{return x;}finally{n=n*10+1;}};const first=keep(a);await b;const last=keep(b);return first+last+n;}",
        "export async function run(a,b){let n=0;const recover=()=>{try{try{throw a;}finally{n++;}}catch(e){return +e;}};const first=recover();await b;return first+n;}",
        "function recover(x){try{throw x;}catch(e){return +e;}finally{x=9;}}export async function run(a,b){try{return recover(a)+(await b);}catch(e){return 99;}}",
        "export async function run(a,b){const keep=()=>{try{return a;}finally{if(b)return b;}};await b;return keep();}",
        "export async function run(a,b){let n=0;const leaf=()=>{try{return a;}finally{n=n*10+1;}};const middle=()=>{try{return leaf();}finally{n=n*10+2;}};const value=middle();await b;return value+n;}",
        "export async function run(a,b){let n=0;try{return a;}finally{const keep=()=>{try{return b;}finally{n=1;}};n+=keep();await b;}}",
    ]);
}

#[test]
fn synchronous_finalizers_preserve_private_promise_and_mixed_result_snapshots() {
    compare_sync_finalizers(&[
        "export async function run(a,b){let n=0;const p=Promise.resolve(a);const keep=()=>{try{return p;}finally{n++;}};const q=keep();const before=n;return q===p?(await q)+before:b;}",
        "export async function run(a,b){let n=0;const reject=()=>{try{return Promise.reject(a);}catch(e){return Promise.resolve(b);}finally{n++;}};try{return(await reject())+n;}catch(e){return(+e)+n;}}",
        "export async function run(a,b){const p=Promise.resolve(a);const choose=()=>{try{return a?p:null;}finally{a=0;}};return await choose();}",
        "export async function run(a,b){const p=Promise.resolve(a);let choose=a?(()=>{try{return p;}finally{b;}}):(()=>{try{return false;}finally{a;}});return await choose();}",
        "export async function run(a,b){const p=Promise.resolve(a);let choose=x=>{try{return x?p:b;}finally{if(b)return null;}};return await choose(a);}",
        "export async function run(a,b){let p=Promise.resolve(a);const old=p;const keep=()=>{try{return p;}finally{p=Promise.resolve(b);}};const q=keep();return q===old&&q!==p?await q:b;}",
        "export async function run(a,b){const p=Promise.resolve(a);const id=x=>{try{return x;}finally{b;}};id(p);return await id(a?false:b);}",
        "export async function run(a,b){let keep=()=>{try{return Promise.resolve(a);}finally{b;}};return await keep();}",
        "export async function run(a,b){let reject=()=>{try{return Promise.reject(a);}finally{b;}};try{return await reject();}catch(e){return e;}}",
        "export async function run(a,b){let n=0;const launch=(fn,x)=>{try{return fn(x);}finally{n++;}};let child=async x=>{await x;return x;};const p=launch(child,a);const before=n;return(await p)+before;}",
    ]);
}

#[test]
fn nested_atomic_async_settlement_keeps_synchronous_finalizer_state_per_owner() {
    compare_sync_finalizers(&[
        "export async function run(a,b){const f=async x=>{const g=()=>{try{return x;}finally{x+=b;}};throw g();};try{return await f(a);}catch(e){return e;}}",
        "export async function run(a,b){const f=async x=>{await b;const h=async()=>{const own=()=>{try{return x;}finally{x+=b;}};throw own();};return await h();};const g=async x=>{await a;const h=async()=>{const own=()=>{try{return x;}finally{x+=a;}};throw own();};return await h();};const p=f(a),q=g(b);let first=0;try{await p;}catch(e){first=+e;}try{await q;}catch(e){return first+(+e);}return 0;}",
    ]);
}

#[test]
fn async_callers_retain_cells_created_or_mutated_by_synchronous_finalizers() {
    compare_sync_finalizers(&[
        "export async function run(a,b){let n=0;const make=x=>{try{return()=>x;}finally{x=b;n++;}};const first=make(a);await a;const last=make(b);await b;return first()+last()+n;}",
        "export async function run(a,b){let first=()=>0,last=()=>0,write=()=>0;const populate=()=>{for(let i=0;i<2;i++){try{if(i===0)continue;}finally{let value=i;const read=()=>value;if(i===0){first=read;write=()=>{value=9;return 0;};}last=read;}}return 0;};populate();await a;write();await b;return first()*10+last();}",
        "export async function run(a,b){let read=()=>0,write=()=>0;const keep=()=>{try{try{throw a;}catch(e){read=()=>+e;write=()=>{e=b;return 0;};return read();}}finally{write();}};const old=keep();await b;return old+read();}",
    ]);
}

#[test]
fn synchronous_finalizers_keep_erasure_and_their_own_throw_reason_boundary() {
    let plain = "export async function run(a,b){const p=Promise.resolve(a);const choose=x=>{try{return x?p:b;}finally{b=a;}};return await choose(a);}";
    let typed = "export async function run(a:Missing,b:never):Wrong{const p:Other=Promise.resolve(a as Bad);const choose=(x:never):Wrong=>{try{return(x as Bad)?(p as Wrong):(b as Other);}finally{b=a as never;}};return await choose(a as Wrong);}";
    compare_sync_finalizers(&[plain]);
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true).unwrap().wasm
    );
    for source in [
        "export async function run(a,b){const p=Promise.resolve(a);const recover=()=>{try{try{throw p;}finally{b;}}catch(e){return e===p?1:0;}};return recover();}",
        "export async function run(a,b){const p=Promise.resolve(a);let recover=()=>{try{try{throw(a?p:b);}finally{a;}}catch(e){return e===p?1:0;}};return recover();}",
    ] {
        for typed in [false, true] {
            let error = compile_async(source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(
                error.contains("synchronous throw requires a represented primitive reason"),
                "{source}: {error}"
            );
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
    for source in [
        "export async function run(a,b){const fail=()=>{try{throw a;}finally{return b;}};try{return fail();}catch(e){return e;}}",
        "export async function run(a,b){const fail=()=>{try{if(a)throw b;return 0;}finally{a;}};try{return fail();}catch(e){return e;}}",
    ] {
        for typed in [false, true] {
            let error = compile_async(source, typed)
                .err()
                .unwrap_or_else(|| panic!("admitted {source}"));
            assert!(error.contains("unsupported"), "{source}: {error}");
            assert!(!error.contains("internal IR"), "{source}: {error}");
        }
    }
}

#[test]
fn a_synchronous_finalizer_cannot_mask_an_async_callers_host_fuel_failure() {
    // The infinite body is only entered through the bounded Wasmtime adapter.
    let source = "export async function run(a,b){const keep=()=>{try{while(a){a=a;}}finally{return b;}};return keep();}";
    for typed in [false, true] {
        let artifact = compile_async(source, typed).unwrap();
        assert!(AsyncInstance::new(&artifact.wasm, 1.0, 42.0).is_err());
        let mut call = AsyncInstance::new(&artifact.wasm, 0.0, 42.0).unwrap();
        while call.run_one_job().unwrap() {}
        assert!(matches!(
            call.state().unwrap(),
            AsyncState::Fulfilled(Value::Number(42.0))
        ));
    }
}
