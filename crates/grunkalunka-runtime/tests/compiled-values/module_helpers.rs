use super::*;

#[test]
fn named_module_helpers_return_null() {
    let source = "function helper(x){return null;}export function run(a,b){return helper(b);}";
    let artifact = compile_value(source, false).unwrap();
    let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
    assert!(matches!(instance.run(4.0, 2.0).unwrap(), Value::Null));
}

fn compare_helpers(sources: &[&str]) {
    let pairs = sources
        .iter()
        .map(|source| {
            let typed = source
                .replace(
                    "function run(a,b)",
                    "function run(a:number,b:number):unknown",
                )
                .replace("function helper(x)", "function helper<T>(x:number):unknown");
            assert_eq!(
                compile_value(source, false).unwrap().wasm,
                compile_value(&typed, true).unwrap().wasm,
                "{source}"
            );
            ((*source).to_owned(), typed)
        })
        .collect();
    compare_primitive_sources(pairs);
}

#[test]
fn named_module_helpers_preserve_primitive_tags_and_number_bits() {
    compare_helpers(&[
        "function helper(x){return null;}export function run(a,b){return helper(b);}",
        "function helper(x){return undefined;}export function run(a,b){return helper(b);}",
        "function helper(x){return false;}export function run(a,b){return helper(b);}",
        "function helper(x){return true;}export function run(a,b){return helper(b);}",
        "function helper(x){return x<0;}export function run(a,b){return helper(b);}",
        "function helper(x){return !x;}export function run(a,b){return helper(b);}",
        "function helper(x){return void x;}export function run(a,b){return helper(b);}",
        "function helper(x){return x?null:x;}export function run(a,b){return helper(b);}",
        "function helper(x){if(x>0)return false;return x;}export function run(a,b){return helper(b);}",
    ]);
}

#[test]
fn named_module_helpers_propagate_results_through_bindings_calls_and_recursion() {
    compare_helpers(&[
        "function helper(x){const result=x?null:false;return result;}export function run(a,b){return helper(b);}",
        "function helper(x){let result=x;if(x)result=null;return result;}export function run(a,b){return helper(b);}",
        "function relay(x){return helper(x);}function helper(x){return x?undefined:false;}export function run(a,b){return relay(b);}",
        "function helper(x){return other(x);}function other(x){return x?null:false;}export function run(a,b){return helper(b);}",
        "function helper(x){if(x>0)return helper(0);return null;}export function run(a,b){return helper(b);}",
        "function helper(x){if(x>0)return other(0);return null;}function other(x){return helper(x);}export function run(a,b){return helper(b);}",
        "function helper(x){const f=y=>y?null:false;return f(x);}export function run(a,b){return helper(b);}",
        "function helper(a,b){return run(0,b);}export function run(a,b){return a?helper(a,b):null;}",
    ]);
}

#[test]
fn named_module_helpers_keep_snapshots_effects_and_nested_body_ownership() {
    compare_helpers(&[
        "function helper(x){return x?null:false;}export function run(a,b){return helper(a,b+=1)?b:b+2;}",
        "function helper(x){return x?null:false;}export function run(a,b){const f=x=>helper(x);return f(a);}",
        "function helper(x){return x?null:false;}export function run(a,b){let saved=helper(a);helper(b,a=0);return saved;}",
        "function helper(x){const f=y=>null;return x;}export function run(a,b){return helper(a)+b;}",
        "function helper(x){const f=y=>null;const y=x;return y;}export function run(a,b){return helper(a)+b;}",
        "function helper(x){const f=y=>{return null;};const y=x;return y;}export function run(a,b){return helper(a)+b;}",
        "function helper(x){const f=y=>y;return null;}export function run(a,b){return helper(a);}",
        "function helper(x){return x?null:false;}export function run(a,b){return helper(a)+b;}",
        "function helper(x){return x?undefined:true;}export function run(a,b){return +helper(a);}",
        "function helper(x){return x?null:false;}export function run(a,b){return helper(a)??b;}",
    ]);
}

#[test]
fn named_module_helpers_keep_numeric_interfaces_and_source_limits() {
    for numeric in [
        "function helper(x){const y=x;return y;}export function run(a,b){return helper(a)+b;}",
        "function helper(x){const f=y=>null;const y=x;return y;}export function run(a,b){return helper(a)+b;}",
        "function helper(x){const f=y=>{return null;};const y=x;return y;}export function run(a,b){return helper(a)+b;}",
    ] {
        let primitive = compile_value(numeric, false).unwrap();
        assert!(primitive.wat.contains("(func (param f64) (result f64)"));
        let numeric = grunkalunka_compiler::compile(numeric, false).unwrap();
        assert!(numeric.wat.contains("(func (param f64) (result f64)"));
    }
    for count in [1000, 1001] {
        let parameters = (0..count)
            .map(|i| format!("p{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let arguments = vec!["b"; count].join(",");
        let source = format!(
            "function helper({parameters}){{return null;}}export function run(a,b){{return helper({arguments});}}"
        );
        if count == 1000 {
            compare_helpers(&[&source]);
        } else {
            let error = compile_value(&source, false)
                .err()
                .expect("parameter limit");
            assert!(
                error.contains("at most 1000 function parameters"),
                "{error}"
            );
        }
    }
    for depth in [128, 129] {
        let source = format!(
            "function helper(x){{return {}null{};}}export function run(a,b){{return helper(b);}}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        if depth == 128 {
            compare_helpers(&[&source]);
        } else {
            assert!(compile_value(&source, false).is_err());
        }
    }
    for source in [
        "function helper(x){return null;}export function run(a,b){return helper();}",
        "function helper(x){return null;}export function run(a,b){return helper(...a);}",
        "function helper(x){return null;}export function run(a,b){return helper?.(a);}",
        "function helper(x){return null;}export function run(a,b){return helper(null);}",
        "function helper(x){return Promise.resolve(x);}export function run(a,b){return helper(b);}",
        "function helper(x){return y=>y;}export function run(a,b){return helper(b);}",
        "function helper(x){return 'text';}export function run(a,b){return helper(b);}",
        "function helper(x){return;}export function run(a,b){return helper(b);}",
        "function helper(x){if(x)return null;}export function run(a,b){return helper(b);}",
        "function helper(x){return null;}export function run(a,b){helper=a;return helper(b);}",
    ] {
        assert!(compile_value(source, false).is_err(), "{source}");
    }
    assert!(
        grunkalunka_compiler::compile(
            "function helper(x){return null;}export function run(a,b){return helper(b);}",
            false
        )
        .is_err()
    );
}

#[test]
fn named_module_helper_result_planning_fits_the_existing_small_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut source = String::from("function helper0(x){return null;}");
            for index in 1..64 {
                source.push_str(&format!(
                    "function helper{index}(x){{return helper{}(x);}}",
                    index - 1
                ));
            }
            source.push_str("export function run(a,b){return helper63(b);}");
            compile_value(&source, false).unwrap()
        })
        .unwrap()
        .join()
        .unwrap();
}
