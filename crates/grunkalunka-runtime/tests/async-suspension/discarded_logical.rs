use super::*;

fn compare_discarded(sources: &[&str]) {
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
fn discarded_logical_assignments_keep_promise_effects() {
    compare_discarded(&[
        "export async function run(a,b){let q=Promise.resolve(b);a&&(q=Promise.resolve(a));return await q;}",
        "export async function run(a,b){let q=Promise.resolve(b);a||(q=Promise.resolve(a));return await q;}",
        "export async function run(a,b){let q=Promise.resolve(b);const x=a?null:false;x??(q=Promise.resolve(a));return await q;}",
        "export async function run(a,b){let q=Promise.resolve(b);a&&(b||(q=Promise.resolve(a)));return await q;}",
    ]);
}

#[test]
fn discarded_logical_awaits_and_failures_keep_their_selected_effects() {
    compare_discarded(&[
        "export async function run(a,b){let q=Promise.resolve(b);(await a)&&(q=Promise.resolve(await b));return await q;}",
        "export async function run(a,b){let q=Promise.resolve(b);try{a&&(await Promise.reject(b),q=Promise.resolve(a));return await q;}catch(e){return (await q)+e;}}",
        "export async function run(a,b){let q=Promise.resolve(b);try{(await Promise.reject(a))&&(q=Promise.resolve(b));}catch(e){return (await q)+e;}}",
    ]);
}

#[test]
fn discarded_logical_nullish_tests_preserve_left_function_and_promise_values() {
    compare_discarded(&[
        "export async function run(a,b){let q=Promise.resolve(b);const p=Promise.resolve(null);p??(q=Promise.resolve(a));return await q;}",
        "export async function run(a,b){let q=Promise.resolve(b);let x=a?Promise.resolve(null):null;x??(q=Promise.resolve(a));return await q;}",
        "export async function run(a,b){let q=Promise.resolve(b);const f=x=>x+a;f??(q=Promise.resolve(a));return await q;}",
        "function helper(x){return x?null:false;}export async function run(a,b){let q=Promise.resolve(b);helper(a)??(q=Promise.resolve(a));return await q;}",
    ]);
}
