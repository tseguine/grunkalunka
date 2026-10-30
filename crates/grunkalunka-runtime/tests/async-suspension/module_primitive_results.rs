use super::*;

fn compare_helpers(sources: &[&str]) {
    compare_async_sources(
        &sources
            .iter()
            .map(|source| (*source).to_owned())
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
fn named_module_primitive_results_cross_awaits_and_async_returns() {
    compare_helpers(&[
        "function helper(x){return x?null:false;}export async function run(a,b){const value=helper(a);await b;return value;}",
        "function helper(x){return x?undefined:false;}export async function run(a,b){return await helper(a);}",
        "function helper(x){return x?null:x;}export async function run(a,b){await b;return helper(a);}",
        "function helper(x){return next(x);}function next(x){return x?true:null;}export async function run(a,b){return await helper(a);}",
    ]);
}

#[test]
fn named_module_primitive_results_compose_with_closures_and_catches() {
    compare_helpers(&[
        "function helper(x){return x?null:false;}export async function run(a,b){const f=async x=>{await b;return helper(x);};return await f(a);}",
        "function helper(x){return x?null:false;}export async function run(a,b){const f=x=>helper(x);const result=f(a);await b;return result;}",
        "function helper(x){return x?null:false;}export async function run(a,b){try{throw helper(a);}catch(e){await b;return e;}}",
        "function helper(x){return x?null:false;}export async function run(a,b){try{await Promise.reject(helper(a));}catch(e){return e+b;}}",
    ]);
}
