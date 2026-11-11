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

#[test]
fn synchronous_var_helpers_keep_hoisting_and_calls_across_awaits() {
    compare_helpers(&[
        "function helper(x){const missing=local===undefined;var local=x;return missing?local:0;}export async function run(a,b){const first=helper(a);await b;return first+helper(b);}",
        "function helper(x){var x;var local=x;var local;return local;}export async function run(a,b){await b;return helper(a);}",
        "function helper(x){return local;var local=x;}export async function run(a,b){await b;return helper(a);}",
        "function helper(x,y){var total=0;for(var [item] of [[x],[y]]){total+=+item;}return total;}export async function run(a,b){await b;return helper(a,b);}",
    ]);
}

#[test]
fn synchronous_var_closures_keep_cells_and_effects_across_awaits() {
    compare_helpers(&[
        "export async function run(a,b){const helper=x=>{var total=x;for(var i=0;i<3;i++){total+=i;}return total;};const first=helper(a);await b;return first+helper(b);}",
        "export async function run(a,b){const make=x=>{var total=x;return y=>{total+=y;return total;};};const fn=make(a);const first=fn(b);await a;return first+fn(b);}",
        "export async function run(a,b){const make=x=>{var total=x;return async y=>{const before=total;await y;total+=y;return before+total;};};const f=make(a);const g=make(b);const first=await f(b);return first+await g(a);}",
        "export async function run(a,b){const helper=async x=>{const nested=y=>{var local=y;return local;};await b;return nested(x);};return await helper(a);}",
        "export async function run(a,b){let total=a;const helper=()=>{var local=total;total=b;return local;};await b;const saved=helper();return saved+total;}",
    ]);
}
