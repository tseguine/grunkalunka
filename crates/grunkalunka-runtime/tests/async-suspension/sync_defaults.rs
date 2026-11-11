use super::*;

fn compare_defaults(sources: &[&str]) {
    compare_async_sources(
        &sources
            .iter()
            .map(|source| (*source).into())
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
fn synchronous_named_defaults_preserve_values_and_caller_reactions() {
    compare_defaults(&[
        "function f(x=3){return x;}export async function run(a,b){return f();}",
        "function f(x=-0){return x;}export async function run(a,b){const saved=f();await b;return saved;}",
        "function f(x=3){return x;}export async function run(a,b){await b;return f(a?undefined:null);}",
        "function f(x=3){return x;}export async function run(a,b){return f(a,await(a=b))+a;}",
        "function f(x,y=x+2){return y;}export async function run(a,b){return f(await a);}",
        "function f(x,y=x??3){return y;}export async function run(a,b){return await f(a?null:false);}",
    ]);
}

#[test]
fn synchronous_default_references_and_strings_compose_with_suspension() {
    compare_defaults(&[
        "function f(x,y=x){return y;}export async function run(a,b){const source={v:a};const saved=f(source);await b;return saved===source?+saved.v:0;}",
        "function f(row,label=row.label??'missing'){return label.length;}export async function run(a,b){await b;return f({label:a?'ready':undefined});}",
        "function f(prefix,label=prefix+':ok'){return label.length;}export async function run(a,b){const saved=f('go');await b;return saved+f('go','');}",
    ]);
}

#[test]
fn synchronous_arrow_defaults_keep_captures_and_emitted_calls() {
    compare_defaults(&[
        "export async function run(a,b){const f=(x=3)=>x;return f();}",
        "export async function run(a,b){const f=(x,y=x||5)=>y;return f(a);}",
        "export async function run(a,b){const f=(x,y=x+2)=>y;return f(await a);}",
        "export async function run(a,b){let f=(x=3)=>x;const saved=f(a?undefined:false);await b;return saved;}",
        "export async function run(a,b){const o={f:(x=3)=>x};await b;return o.f()+o.f.length;}",
        "export async function run(a,b){const make=(x=3)=>()=>x;const f=make();await b;return f();}",
        "export async function run(a,b){const f=(x=3)=>x;const invoke=g=>g(undefined);await b;return invoke(f);}",
    ]);
}
