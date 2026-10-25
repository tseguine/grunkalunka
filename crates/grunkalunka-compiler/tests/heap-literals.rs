use grunkalunka_compiler::compile_heap_value;

#[test]
fn empty_string_uses_the_checked_heap_profile() {
    let artifact = compile_heap_value("export function run(a,b){return \"\";}", false).unwrap();
    assert!(artifact.wat.contains("\"gk_heap_enter\""));
    assert!(artifact.wat.contains("\"gk_heap_string\""));
    assert!(artifact.wat.contains("\"gk_heap_leave\""));
}

#[test]
fn deferred_function_results_are_not_plain_strings() {
    for source in [
        "export async function run(a,b){return \"s\";}",
        "export function* run(a,b){return \"s\";}",
    ] {
        assert!(compile_heap_value(source, false).is_err(), "{source}");
    }
}
