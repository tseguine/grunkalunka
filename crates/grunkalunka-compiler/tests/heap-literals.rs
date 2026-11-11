use grunkalunka_compiler::compile_heap_value;

#[test]
fn empty_module_items_preserve_the_heap_entry() {
    let entry = "export function run(a,b){return 'value';}";
    for ts in [false, true] {
        let expected = compile_heap_value(entry, ts).unwrap().wasm;
        let named = "void 0;export { entry as run };function entry(a,b){return 'value';}";
        assert_eq!(compile_heap_value(named, ts).unwrap().wasm, expected);
        assert_eq!(
            compile_heap_value(&format!(";export {{}};{entry};"), ts)
                .unwrap()
                .wasm,
            expected
        );
        assert!(compile_heap_value(&format!("export {{}} from 'other';{entry}"), ts).is_err());
    }
}

#[test]
fn reduced_heap_entry_arities_keep_the_two_number_abi() {
    let full = compile_heap_value("export function run(a,b){return 'value';}", false).unwrap();
    assert!(full.wat.contains("(param f64 f64)"));
    for (source, typescript) in [
        ("export function run(){return 'value';}", false),
        ("export function run(a){return 'value';}", false),
        ("export function run():string{return 'value';}", true),
        (
            "export function run(a?:unknown):string{return 'value';}",
            true,
        ),
    ] {
        assert_eq!(
            compile_heap_value(source, typescript).unwrap().wasm,
            full.wasm,
            "{source}"
        );
    }
    for source in [
        "export function run(a,b,c){return 'value';}",
        "export function run(a,...rest){return 'value';}",
    ] {
        let error = compile_heap_value(source, false).err().unwrap();
        assert!(error.contains("at most two ordinary"), "{error}");
    }
}

#[test]
fn inert_source_statements_preserve_the_heap_entry() {
    let plain = "export function run(a,b){return 'value';}";
    let expected = compile_heap_value(plain, false).unwrap().wasm;
    for (source, typescript) in [
        (
            "export function run(a,b){;debugger;return 'value';debugger;;}",
            false,
        ),
        (
            "export function run<T>(a:T,b:T){type S=string;debugger;return ('value' as S);interface After{x:T}}",
            true,
        ),
    ] {
        assert_eq!(
            compile_heap_value(source, typescript).unwrap().wasm,
            expected,
            "{source}"
        );
    }
    for body in ["let x=0;return 'value';", "return 'value';return 'other';"] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(compile_heap_value(&source, false).is_err(), "{source}");
    }
}

#[test]
fn discarded_primitive_literals_preserve_the_heap_entry() {
    let plain = "export function run(a,b){return 'value';}";
    let expected = compile_heap_value(plain, false).unwrap().wasm;
    for (source, typescript) in [
        (
            "export function run(a,b){0;false;null;'before';`text`;void 0;return 'value';1n;'after';}",
            false,
        ),
        (
            "export function run(a:number,b:number){(0 as unknown);return 'value';(`after` satisfies string);}",
            true,
        ),
    ] {
        assert_eq!(
            compile_heap_value(source, typescript).unwrap().wasm,
            expected,
            "{source}"
        );
    }
    for body in [
        "missing;return 'value';",
        "sideEffect();return 'value';",
        "a++;return 'value';",
        "void sideEffect();return 'value';",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(compile_heap_value(&source, false).is_err(), "{source}");
    }
}

#[test]
fn heap_json_call_type_arguments_are_erased_metadata() {
    let plain = "export function run(a,b){return JSON.stringify(\"x\");}";
    let typed = "export function run(a:number,b:number){return JSON.stringify<number>(\"x\");}";
    assert_eq!(
        compile_heap_value(plain, false).unwrap().wasm,
        compile_heap_value(typed, true).unwrap().wasm
    );
    for metadata in [
        "never",
        "Missing",
        "typeof absent",
        "Missing,Other",
        "{call:()=>void}",
    ] {
        for expression in [
            "JSON.stringify(\"A\\ud800\\0\")",
            "JSON.stringify(JSON.parse(\"{\\\"x\\\":[1,true,null]}\"))",
            "JSON.stringify(JSON.parse(JSON.stringify(\"x\")))",
        ] {
            let plain = format!("export function run(a,b){{return {expression};}}");
            let typed = plain
                .replace("JSON.stringify(", &format!("JSON.stringify<{metadata}>("))
                .replace("JSON.parse(", &format!("JSON.parse<{metadata}>("));
            assert_eq!(
                compile_heap_value(&plain, false).unwrap().wasm,
                compile_heap_value(&typed, true).unwrap().wasm,
                "{typed}"
            );
        }
    }
}

#[test]
fn heap_json_call_types_do_not_establish_payload_or_binding_proofs() {
    for expression in [
        "JSON.parse<string>(\"null\")",
        "JSON.stringify<never>(JSON.parse<string>(JSON.parse<string>(\"null\")))",
        "JSON.parse<Missing>()",
        "JSON.stringify<Missing>()",
        "JSON.stringify<string>(1)",
        "JSON.parse<string>(1)",
        "JSON.stringify<string>({})",
        "JSON.stringify<string>([])",
        "JSON.stringify<string>(absent)",
        "JSON.stringify<never>(\"x\", null)",
        "JSON.parse<never>(\"null\", a)",
        "JSON.stringify<never>(...a)",
        "JSON[\"stringify\"]<never>(\"x\")",
        "JSON?.stringify<never>(\"x\")",
        "JSON.stringify?.<never>(\"x\")",
        "JSON.other<never>(\"x\")",
        "a.stringify<never>(\"x\")",
    ] {
        let source = format!("export function run(a,b){{return {expression};}}");
        assert!(compile_heap_value(&source, true).is_err(), "{source}");
    }
    let shadowed = "export function run(JSON,b){return JSON.stringify<never>(\"x\");}";
    assert!(compile_heap_value(shadowed, true).is_err(), "{shadowed}");
}

#[test]
fn heap_json_call_metadata_keeps_source_and_expression_budgets() {
    for (calls, admitted) in [(127, true), (128, false)] {
        let plain = format!(
            "export function run(a,b){{return {}\"\"{};}}",
            "JSON.stringify(".repeat(calls),
            ")".repeat(calls)
        );
        let typed = plain.replace("JSON.stringify(", "JSON.stringify<never>(");
        assert_eq!(compile_heap_value(&typed, true).is_ok(), admitted);
        if admitted {
            assert_eq!(
                compile_heap_value(&plain, false).unwrap().wasm,
                compile_heap_value(&typed, true).unwrap().wasm
            );
        }
    }
    let plain = "export function run(a,b){return JSON.stringify(\"x\");}";
    let types = vec!["never"; 1001].join(",");
    let typed = plain.replace("JSON.stringify(", &format!("JSON.stringify<{types}>("));
    assert_eq!(
        compile_heap_value(plain, false).unwrap().wasm,
        compile_heap_value(&typed, true).unwrap().wasm
    );
    let prefix = "export function run(a,b){return JSON.stringify<\"";
    let suffix = "\">(\"x\");}";
    let exact = format!(
        "{prefix}{}{suffix}",
        "x".repeat(16_384 - prefix.len() - suffix.len())
    );
    assert_eq!(
        compile_heap_value(plain, false).unwrap().wasm,
        compile_heap_value(&exact, true).unwrap().wasm
    );
    assert!(
        compile_heap_value(&(exact + " "), true)
            .err()
            .unwrap()
            .contains("16 KiB")
    );
}

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
