#[test]
fn conditional_helpers_keep_values_and_undefined_distinct() {
    for body in [
        "if(x)return x;",
        "if(x)return null;",
        "{if(x)return false;}",
        "if(x){if(x>1)return 3;}else{if(x<0)return 4;}",
    ] {
        for ts in [false, true] {
            let source = format!("function f(x){{{body}}}export function run(a,b){{return f(a);}}");
            assert!(crate::compile_value(&source, ts).is_ok(), "{source}");
            assert!(crate::compile(&source, ts).is_err(), "{source}");
            let source = source.replace("return f(a)", "return +f(a)");
            assert!(crate::compile(&source, ts).is_ok(), "{source}");
        }
    }
}

#[test]
fn exhaustive_numeric_helpers_keep_the_numeric_interface_and_source_guards() {
    for body in [
        "if(x)return x;else return 2;",
        "{return x;}",
        "if(x){return x;}else{return 2;}",
    ] {
        let source = format!("function f(x){{{body}}}export function run(a,b){{return f(a);}}");
        assert!(crate::compile(&source, false).is_ok(), "{source}");
    }
    for body in [
        "if(x)return missing;",
        "if(x)return x;else Math.random();",
        "return x;Math.random();",
    ] {
        let source = format!("function f(x){{{body}}}export function run(a,b){{return f(a);}}");
        assert!(crate::compile_value(&source, false).is_err(), "{source}");
    }
}

#[test]
fn conditional_tail_queries_keep_small_stack_and_statement_depth_guards() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for depth in [32, 65] {
                let source = format!(
                    "function f(x){{{}if(x)return x;{}}}export function run(a,b){{return f(a);}}",
                    "{".repeat(depth),
                    "}".repeat(depth)
                );
                assert_eq!(
                    crate::compile_value(&source, false).is_ok(),
                    depth == 32,
                    "{depth}"
                );
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
