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

#[test]
fn json_stringify_uses_the_guest_operation() {
    let artifact = compile_heap_value(
        "export function run(a,b){return JSON.stringify(\"\");}",
        false,
    )
    .unwrap();
    assert!(artifact.wat.contains("\"gk_heap_stringify\""));
}

#[test]
fn heap_entry_does_not_silently_discard_parameter_effects() {
    for source in [
        "export function run(a=JSON.parse(\"invalid\"),b){return \"\";}",
        "export function run(a,a){return \"\";}",
        "export function run({a},b){return \"\";}",
        "export function run(...args){return \"\";}",
        "export function other(a,b){return \"\";}",
    ] {
        assert!(compile_heap_value(source, false).is_err(), "{source}");
    }
}

#[test]
fn heap_source_and_expression_budgets_are_explicit() {
    let source = "export function run(a,b){return \"\";}";
    let bounded = format!("{source}{}", " ".repeat(16_384 - source.len()));
    assert!(compile_heap_value(&bounded, false).is_ok());
    assert!(compile_heap_value(&(bounded + " "), false).is_err());
    for (wrappers, admitted) in [(127, true), (128, false)] {
        let source = format!(
            "export function run(a,b){{return {}\"\"{};}}",
            "(".repeat(wrappers),
            ")".repeat(wrappers)
        );
        assert_eq!(compile_heap_value(&source, false).is_ok(), admitted);
    }
    for (calls, admitted) in [(127, true), (128, false)] {
        let source = format!(
            "export function run(a,b){{return {}\"\"{};}}",
            "JSON.stringify(".repeat(calls),
            ")".repeat(calls)
        );
        assert_eq!(compile_heap_value(&source, false).is_ok(), admitted);
    }
}

#[test]
fn json_calls_require_the_pristine_intrinsic_and_supported_operands() {
    for expression in [
        "JSON.parse(\"null\")",
        "JSON.stringify(JSON.parse(JSON.parse(\"null\")))",
        "JSON.parse()",
        "JSON.stringify()",
        "JSON.stringify(1)",
        "JSON.parse(1)",
        "JSON.stringify(\"x\", null)",
        "JSON.parse(\"null\", a)",
        "JSON.stringify(...a)",
        "JSON[\"stringify\"](\"x\")",
        "JSON?.stringify(\"x\")",
        "JSON.stringify?.(\"x\")",
        "JSON.other(\"x\")",
        "a.stringify(\"x\")",
    ] {
        let source = format!("export function run(a,b){{return {expression};}}");
        assert!(compile_heap_value(&source, false).is_err(), "{source}");
    }
    let shadowed = "export function run(JSON,b){return JSON.stringify(\"x\");}";
    assert!(compile_heap_value(shadowed, false).is_err());
    let typed_call =
        "export function run(a:number,b:number){return JSON.stringify<number>(\"x\");}";
    assert!(compile_heap_value(typed_call, true).is_err());
}

#[test]
fn nested_json_calls_use_both_checked_guest_operations() {
    for typescript in [false, true] {
        let artifact = compile_heap_value(
            "export function run(a,b){return JSON.stringify(JSON.parse(\"{\\\"x\\\":[1,true,null]}\"));}",
            typescript,
        )
        .unwrap();
        assert!(artifact.wat.contains("\"gk_heap_parse\""));
        assert!(artifact.wat.contains("\"gk_heap_stringify\""));
    }
}
