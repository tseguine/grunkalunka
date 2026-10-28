use super::*;

#[test]
fn private_mixed_results_keep_kind_and_identity_in_known_calls() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);const id=x=>x;id(p);return id(b);",
            "const p=Promise.resolve(a);const id=x=>x;id(b);return id(p)===p;",
            "const p=Promise.resolve(a);const choose=x=>x?p:b;return await choose(a);",
            "const p=Promise.resolve(a);const choose=()=>a?p:false;return choose()?b:a;",
            "const p=Promise.resolve(a);const choose=()=>a?p:null;return await(choose()??b);",
            "const id=p=>p;id(Promise.resolve(a));return await id(b);",
            "let id=p=>p;id(Promise.resolve(a));return await id(b);",
            "const id=p=>{p=b;return p;};return await id(Promise.resolve(a));",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn private_mixed_results_cross_runtime_callbacks_and_factories() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);let id=a?x=>x:x=>x;id(p);return id(b);",
            "const p=Promise.resolve(a);let choose=a?()=>p:()=>b;return await choose();",
            "const p=Promise.resolve(a);let choose=a?()=>p:()=>false;return choose()===p;",
            "let f=()=>Promise.resolve(a);f=()=>b;return await f();",
            "const f=a?()=>Promise.resolve(b):()=>b;return await f();",
            "const apply=(fn,x)=>fn(x);apply(x=>x,a);return await apply(x=>Promise.resolve(x),b);",
            "const p=Promise.resolve(a);const id=x=>x;const call=(fn,x)=>fn(x);call(id,p);return call(id,b);",
            "const p=Promise.resolve(a);let id=a?x=>x:x=>x;let call=a?(fn,x)=>fn(x):(fn,x)=>fn(x);call(id,b);return call(id,p)===p;",
            "const p=Promise.resolve(a);const make=x=>()=>x;const makeP=make(p);const makeB=make(b);let f=a?makeP:makeB;return await f();",
            "const p=Promise.resolve(a);const choose=()=>a?p:b;const forward=()=>choose();return await forward();",
            "const p=Promise.resolve(a);let choose=a?()=>p:()=>b;const forward=fn=>fn();return await forward(choose);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn private_mixed_results_preserve_early_returns_and_undefined_paths() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);const choose=x=>{if(x)return p;return b;};return await choose(a);",
            "const p=Promise.resolve(a);const choose=x=>{if(x)return p;};return await choose(a);",
            "const p=Promise.resolve(a);const choose=x=>{if(x)return;return p;};return await choose(a);",
            "const p=Promise.resolve(a);let choose=a?x=>{if(x)return p;return null;}:x=>{if(x)return false;return p;};return await choose(b);",
            "const p=Promise.resolve(a);const choose=x=>{while(x){return p;}return b;};return await choose(a);",
            "const p=Promise.resolve(a);const choose=x=>{const nested=()=>{return b;};if(x)return p;return null;};return choose(a)===p;",
            "const make=()=>{if(a)return Promise.resolve(b);};return b;",
            "const make=()=>{if(a)return Promise.resolve(b);return a;};return b;",
            "const f=a?()=>{if(a)return Promise.resolve(b);}:()=>Promise.resolve(a);return await f();",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn private_mixed_results_keep_snapshots_cells_and_ordered_effects() {
    compare_promises(
        &[
            "const p=Promise.resolve(a);let v=a?p:b;const read=()=>v;const old=read();v=null;await b;return old===p;",
            "const p=Promise.resolve(a);let id=a?x=>x:x=>x;id(p);let v=id(b);const read=()=>v;v=p;await a;return read()===p;",
            "const p=Promise.resolve(a);let id=x=>x;let n=0;const other=x=>a?b:p;id(p);const got=id((id=other,n=n+1,b),n=n+2);return got===b?n:99;",
            "const p=Promise.resolve(a);let n=0;const choose=()=>{n=n+1;return a?p:b;};const v=choose();await b;return v===p?n:n+10;",
            "const p=Promise.resolve(a);const id=x=>x;id(p);let v=b;try{v=id((v=a,await Promise.reject(b)));}catch(e){return v===a;}return false;",
            "const p=Promise.resolve(a);const id=x=>x;id(p);const take=(first,second)=>first===p?second:first;return take(id(b),id(p));",
            "let x=a?Promise.resolve(a):b;const read=()=>x;return read();",
            "let x=a?Promise.resolve(a):b;const id=y=>y;id(a);return id(x);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn private_mixed_results_keep_reactions_and_primitive_error_values() {
    compare_promises(
        &[
            "const p=Promise.reject(a);const choose=()=>b?p:null;let out;try{out=await choose();}catch(e){out=e;}try{await p;}catch(e){}return out;",
            "const p=Promise.reject(a);const choose=()=>b?p:false;const use=async()=>choose();let out;try{out=await use();}catch(e){out=e;}try{await p;}catch(e){}return out;",
            "const p=Promise.resolve(a);let choose=a?()=>p:()=>b;const use=async()=>{await b;return choose();};return await use();",
            "const p=Promise.resolve(a);const id=x=>x;id(p);return id(-0);",
            "const p=Promise.resolve(a);const id=x=>x;id(p);return id(null);",
            "const p=Promise.resolve(a);const id=x=>x;id(p);return id(void 0);",
            "const p=Promise.resolve(a);const id=x=>x;id(p);return id(false);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
    let cycle = "const f=async()=>{await a;return b;};let p=f();const g=async()=>{await a;return p;};p=g();";
    compare_async_sources(
        &[format!(
            "export async function run(a,b){{{cycle}try{{await p;}}catch(e){{const id=x=>x;id(p);const old=id(e);await a;return old===e;}}}}"
        )],
        &[(4.0, 2.0), (0.0, -0.0)],
    );
}

#[test]
fn private_mixed_results_keep_erasure_limits_and_export_boundaries() {
    let body = "const p=Promise.resolve(a);let id=a?x=>x:x=>x;id(p);return await id(b);";
    let plain = source_for(body);
    let typed = plain
        .replace("run(a,b)", "run(a:Missing,b:never):Wrong")
        .replace(
            "a?x=>x:x=>x",
            "(a as never)?(x:Wrong):Bad=>x:(x:Missing):Bad=>x",
        );
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
    for count in [0, 31] {
        let body = format!(
            "const p=Promise.resolve(a);const choose=()=>a?p:b;{}return await choose();",
            "await b;".repeat(count)
        );
        compare_promises(&[&body], &[(4.0, 2.0), (0.0, -0.0)]);
    }
    let over = source_for(&format!(
        "const p=Promise.resolve(a);const choose=()=>a?p:b;{}return await choose();",
        "await b;".repeat(32)
    ));
    assert!(
        compile_async(&over, false)
            .err()
            .unwrap()
            .contains("32 await sites")
    );
    for body in [
        "const p=Promise.resolve(a);const choose=()=>a?p:{};return choose();",
        "const p=Promise.resolve(a);const choose=()=>a?p:()=>b;return choose();",
    ] {
        for ts in [false, true] {
            assert!(compile_async(&source_for(body), ts).is_err(), "{body}");
        }
    }
}
