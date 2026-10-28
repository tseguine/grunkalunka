use super::*;

#[test]
fn synchronous_helpers_start_async_callbacks_without_awaiting_them() {
    compare_promises(
        &[
            "const apply=(fn,p,x)=>fn(p,x);const f=async(p,x)=>a?p:x;return await apply(f,Promise.resolve(b),a);",
            "const apply=(fn,x)=>fn(x);let f=async x=>{await x;return b;};return await apply(f,a);",
            "let apply=(fn,x)=>fn(x);const f=async x=>{await b;return x;};return await apply(f,a);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn synchronous_starters_preserve_prefix_effects_and_live_caller_snapshots() {
    compare_promises(
        &[
            "let n=0;let f=async x=>{n=n*10+1;await x;n=n*10+2;return x;};const apply=(fn,x)=>(n=n*10+3,fn(x));const p=apply(f,a);const before=n;return (await p)+before*100+n*10000;",
            "let x=a;const read=()=>x;let f=async n=>{x=b;await n;x=x+1;return read();};const apply=(fn,n)=>fn(n);const before=x;const p=apply(f,a);const prefix=x;return before+prefix*100+(await p)*10000;",
            "let f=async x=>{await x;return x;};let apply=a?(fn,x)=>fn(x):(fn,x)=>fn(x);const x=a+b;return x+(await apply(f,a))*100+x*10000;",
            "let n=0;const f=async x=>{n=n+1;await x;n=n+10;return n;};const apply=(fn,x)=>fn(x);const p=apply(f,a);const before=n;await b;return before*100+(await p);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn nested_starters_keep_sibling_activations_and_forwarded_identities_separate() {
    compare_promises(
        &[
            "const launch=(fn,x)=>fn(x);const forward=(fn,x)=>launch(fn,x);let f=async x=>{await x;return x;};return await forward(f,a);",
            "const launch=(fn,x)=>fn(x);let g=async x=>{await x;return x;};let f=async x=>{await x;return await launch(g,x);};const p=launch(f,a),q=launch(f,b);return (await p)+(await q)*100;",
            "const launch=(fn,x)=>fn(x);let g=async x=>{await x;return x;};let f=async x=>{const before=x;const p=launch(g,x+1);return before+(await p);};return await launch(f,a);",
            "let next=async x=>{await x;return b;};const launch=(fn,x)=>fn(x);let f=async x=>{const before=x;const saved=next;next=async y=>{await y;return b;};const p=launch(saved,x+1);return before+(await p);};next=f;return await launch(f,a);",
            "let f=async x=>{await x;return x;};const make=()=>f;const apply=(fn,x)=>fn(x);return await apply(make(),a);",
            "const launch=(fn,x)=>fn(x);const forward=p=>p;let f=async x=>{await x;return x;};const p=launch(f,a);const q=forward(p);return (p===q)?await q:b;",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn synchronous_starters_return_rejected_children_to_the_awaiting_catch() {
    compare_promises(
        &[
            "const apply=(fn,x)=>fn(x);let f=async x=>{await x;throw b;};try{return await apply(f,a);}catch(e){return +e+100;}",
            "const apply=(fn,x)=>fn(x);let f=async x=>Promise.reject(x);try{return await apply(f,b);}catch(e){return e;}",
            "let n=0;const apply=(fn,x)=>(n=n+1,fn(x));let f=async x=>{await x;throw b;};const p=apply(f,a);const before=n;try{return await p;}catch(e){return before*100+(+e);}",
            "const apply=(fn,x)=>fn(x);let f=async x=>{try{await Promise.reject(x);}catch(e){return +e+1;}};return await apply(f,b);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn synchronous_starters_copy_typed_arguments_and_preserve_erasure() {
    let body = "const apply=(fn,p,x)=>fn(p,x);let f=async(p,x)=>{await x;return p;};const p=Promise.resolve(b);const q=apply(f,p,a);return (await q)+(q===p?100:0);";
    compare_promises(
        &[
            body,
            "const apply=(fn,cb,x)=>fn(cb,x);let f=async(cb,x)=>{await x;return cb(x);};const twice=x=>x*2;return await apply(f,twice,a);",
            "const apply=(fn,x)=>fn(x);let f=async x=>{await a;return x;};return await apply(f,a?null:false);",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
    let plain = source_for(body);
    let typed = plain
        .replace("run(a,b)", "run(a:never,b:Missing):Wrong")
        .replace(
            "(fn,p,x)=>fn(p,x)",
            "(fn:Bad,p:Other,x:Nope):Wrong=>fn(p,x)",
        );
    assert_eq!(
        compile_async(&plain, false).unwrap().wasm,
        compile_async(&typed, true).unwrap().wasm
    );
}

#[test]
fn ordinary_source_helpers_can_start_children_and_return_synchronously() {
    compare_async_sources(
        &[
            "function helper(x){let f=async n=>{await n;return n;};f(x);return x+1;}export async function run(a,b){return helper(a)+(await b);}".into(),
            "function helper(x){let n=x;let f=async y=>{n=n+1;await y;return n;};f(x);return n;}export async function run(a,b){const before=a+b;const x=helper(a),y=helper(b);await a;return before+x*100+y*10000;}".into(),
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn async_prefix_throws_preserve_started_children_and_lexical_handling() {
    compare_promises(
        &[
            "let g=async x=>{await x;return x;};const f=async x=>{g(x);throw b;};try{return await f(a);}catch(e){return e;}",
            "const launch=(fn,x)=>fn(x);let g=async x=>{await x;return x;};const f=async x=>{launch(g,x);throw b;};try{return await f(a);}catch(e){return e;}",
        ],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}
