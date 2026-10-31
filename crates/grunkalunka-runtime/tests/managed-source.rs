use grunkalunka_compiler::{compile, compile_async, compile_value};
use grunkalunka_runtime::{Value, ValueInstance};

#[path = "managed/common.rs"]
mod common;

#[test]
fn source_object_aliases_preserve_identity() {
    compare("const o={};const alias=o;return o===alias?a:b;");
    compare("const o={};const fresh={};return o!==fresh?a:b;");
    compare("return {}==={}?a:b;");
    compare("const o={x:{}};return o.x!==a&&o.x!==null?a:b;");
    compare("const o={x:false,y:0,z:null};return o.x!==o.y&&o.z!==undefined?a:b;");
}

#[test]
fn source_properties_preserve_values_and_saved_reads() {
    compare("const o={x:a};const old=o.x;o.x=b;return old===a&&o.x===b?a:b;");
}

#[test]
fn source_managed_truthiness_keeps_primitive_kinds_and_truthy_objects() {
    for value in ["a", "false", "true", "null", "undefined", "{}"] {
        compare(&format!("const o={{x:{value}}};return o.x?a:b;"));
    }
    compare("const o={};return o?a:b;");
}

#[test]
fn source_object_rebinding_preserves_saved_references() {
    compare("let o={x:a};const old=o;o={x:b};return old.x===a&&o.x===b?a:b;");
}

#[test]
fn source_managed_observations_preserve_primitive_export_kinds() {
    for ts in [false, true] {
        let equality = compile_value(
            "export function run(a,b){const o={x:a};return o.x===b;}",
            ts,
        )
        .unwrap();
        let mut equality = ValueInstance::new(&equality.wasm).unwrap();
        let nullish = compile_value(
            "export function run(a,b){const o={x:a};return o.x?null:undefined;}",
            ts,
        )
        .unwrap();
        let mut nullish = ValueInstance::new(&nullish.wasm).unwrap();
        for a in [0.0_f64, -0.0, 1.0, -7.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(equality.run(a,a).unwrap(),Value::Boolean(v) if v==!a.is_nan()));
            let v = nullish.run(a, 7.0).unwrap();
            assert!(if a != 0.0 && !a.is_nan() {
                matches!(v, Value::Null)
            } else {
                matches!(v, Value::Undefined)
            });
        }
    }
}

#[test]
fn source_object_initializers_and_assignments_keep_effect_order() {
    for body in [
        "let n=0;const o={x:(n++,a),y:(n++,b),x:(n++,b)};return n===3&&o.x===b&&o.y===b?a:b;",
        "const o={a,b};return o.a===a&&o.b===b?a:b;",
        "let o={x:a};const old=o;const out=(o.x=(o={x:b},b));return old.x===b&&o.x===b&&out===b?a:b;",
        "let n=0;let o={x:a};const old=o;(n++,o).x=(n++,o={x:b},a);return n===2&&old.x===a&&o.x===b?a:b;",
        "const o={x:a,y:b};o.x=o.y=a;return o.x===a&&o.y===a?a:b;",
        "const o={x:a};let v=o.x;v=b;return v===b&&o.x===a?a:b;",
        "let n=a;const o={x:n=b};return o.x===b&&n===b?a:b;",
        "const f=(x,y)=>x+y;const o={x:f.length};return o.x===2?a:b;",
        "let i=0;const o={x:a};while(i<2){o.x=i++;}return o.x===1?a:b;",
        "const o={};for(let i=0;i<2;i++){o.x=i?b:a;}return o.x===b?a:b;",
        "let n=0;const f=()=>++n;const o={x:f(),y:f()};return o.x===1&&o.y===2?a:b;",
    ] {
        compare(body);
    }
}

#[test]
fn source_objects_keep_selected_branches_and_nested_reference_values() {
    for body in [
        "const o=a?{x:a}:{x:b};return o.x===a?a:b;",
        "let n=0;const o=a?(n++,{x:a}):(n+=2,{x:b});return (a?n===1:n===2)?a:b;",
        "const child={x:a};const o={child,other:{x:b}};return o.child===child&&o.other!==child?a:b;",
        "const v=a?{}:false;return v?a:b;",
        "const o={x:a?{}:b};return o.x?a:b;",
    ] {
        compare(body);
    }
}

#[test]
fn source_property_names_preserve_utf16_and_absent_values() {
    for body in [
        r#"const o={"\ud800\0":a,"\ufffd\0":b};return o["\ud800\0"]===a&&o["\ufffd\0"]===b?a:b;"#,
        r#"const o={"":a,"\0":b};return o[""]===a&&o["\0"]===b?a:b;"#,
        "const o={['x']:a,length:b};o['x']=b;return o.x===b&&o.length===b?a:b;",
        "const o={x:a};return o.absent===undefined&&o.x!==null?a:b;",
    ] {
        compare(body);
    }
}

#[test]
fn source_object_annotations_do_not_change_runtime_identity() {
    compare_typed(
        "const o={x:a};const alias=o;return o===alias?a:b;",
        "const o:number={x:a};const alias:boolean=o;return (o as unknown)===(alias!)?a:b;",
    );
}

#[test]
fn source_managed_boundaries_explicitly_reject_unimplemented_semantics() {
    for body in [
        "const o={};return o[a]?a:b;",
        "const o={get x(){return a;}};return o.x?a:b;",
        "const o={x(){return a;}};return a;",
        "const o={...{x:a}};return a;",
        "const o={1:a};return a;",
        "const o={x:()=>a};return a;",
        "const f=()=>a;const o={x:f};return a;",
        "const o={x:Promise.resolve(a)};return o.x===a?a:b;",
        "const p=Promise.resolve(a);const o={x:p};return o.x===a?a:b;",
        "const o={x:1n};return a;",
        "const o=true?{}:{get x(){return a;}};return a;",
        "const o={x:a};return o.x;",
        "const o={};return o;",
        "let o={};o=a;return a;",
        "const o={};o={};return a;",
        "const o={};return o==null?a:b;",
        "const o={x:{}};const x=o.x;return x.y?a:b;",
        "const o={};Object.setPrototypeOf(o,{});return a;",
    ] {
        for ts in [false, true] {
            assert!(
                compile(&format!("export function run(a,b){{{body}}}"), ts).is_err(),
                "{body}"
            );
        }
    }
    for ts in [false, true] {
        assert!(compile("function get(o){return o.x;}export function run(a,b){const o={x:a};return get(o);}",ts).is_err());
        let error = compile_async(
            "export async function run(a,b){const o={x:a};return o.x?a:b;}",
            ts,
        )
        .err()
        .expect("managed async frames remain unsupported");
        assert!(error.contains("managed"), "{error}");
    }
}

#[test]
fn source_proto_guards_cover_actual_native_intrinsic_names() {
    let output = std::process::Command::new("node")
        .args([
            "-e",
            "console.log(Object.getOwnPropertyNames(Object.prototype).join('\\n'))",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let names = String::from_utf8(output.stdout).unwrap();
    assert!(names.lines().any(|n| n == "__proto__"));
    for name in names.lines() {
        for body in [
            format!("const o={{}};return o[{name:?}]?a:b;"),
            format!("const o={{{name:?}:a}};return a;"),
            format!("const o={{}};o[{name:?}]=a;return a;"),
        ] {
            for ts in [false, true] {
                let error = compile(&format!("export function run(a,b){{{body}}}"), ts)
                    .err()
                    .expect("intrinsic prototype property must reject");
                assert!(error.contains("prototype"), "{body}: {error}");
            }
        }
    }
}

#[test]
fn source_object_depth_checks_run_on_a_small_native_stack() {
    for (depth, admitted) in [(16, true), (140, false)] {
        let source = format!(
            "export function run(a,b){{return ({0}a{1})?a:b;}}",
            "{x:".repeat(depth),
            "}".repeat(depth)
        );
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(result.is_ok(), admitted);
        if let Err(error) = result {
            assert!(error.contains("nesting"), "{error}");
        }
    }
}

fn compare(body: &str) {
    compare_typed(body, body);
}

fn compare_typed(body: &str, typed: &str) {
    let js = format!("export function run(a,b){{{body}}}");
    let ts = format!("export function run(a,b){{{typed}}}");
    common::compare_sources(&js, &ts);
}
