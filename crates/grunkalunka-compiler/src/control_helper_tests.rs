#[test]
fn helper_control_flow_keeps_implicit_undefined_in_the_result_domain() {
    for body in [
        "while(x){return x;}",
        "for(let i=0;i<2;i++){if(x)return i;}",
        "for(const y of [x]){if(x)return y;}",
        "const o=Object.create(null);o.a=x;for(const key in o){if(x)return x;}",
        "try{throw x;}catch(e){if(e)return e;}finally{x++;}",
        "do{if(x)break;return x;}while(false);",
        "do{if(x)continue;return x;}while(false);",
        "switch(x){case 1:return 3;}",
        "switch(x){case 0:break;default:return 3;}",
        "try{if(x)return x;}finally{x++;}",
        "try{throw x;}catch(e){if(e)return e;}",
        "try{return x;}finally{if(x)return;}",
    ] {
        let source = format!("function f(x){{{body}}}export function run(a,b){{return f(a);}}");
        for ts in [false, true] {
            let result = crate::compile_value(&source, ts);
            assert!(result.is_ok(), "{source}: {:?}", result.err());
            assert!(crate::compile(&source, ts).is_err(), "{source}");
            let numeric = source.replace("return f(a)", "return +f(a)");
            let result = crate::compile(&numeric, ts);
            assert!(result.is_ok(), "{numeric}: {:?}", result.err());
        }
    }
}

#[test]
fn exhaustive_helper_control_flow_retains_number_interfaces() {
    for body in [
        "do{return x;}while(x);",
        "switch(x){case 1:return 2;default:return x;}",
        "switch(x){case 1:x++;default:return x;}",
        "try{return x;}finally{x++;}",
        "try{if(x)throw x;return 2;}catch(e){return x;}",
        "while(x){break;}return x;",
    ] {
        let source = format!("function f(x){{{body}}}export function run(a,b){{return f(a);}}");
        let result = crate::compile(&source, false);
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
}

#[test]
fn structured_completion_queries_keep_small_stack_and_escaping_throw_guards() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for depth in [8, 65] {
                let body = format!(
                    "{}if(x)return x;{}",
                    "try{".repeat(depth),
                    "}finally{x++;}".repeat(depth)
                );
                let source =
                    format!("function f(x){{{body}}}export function run(a,b){{return f(a);}}");
                assert_eq!(
                    crate::compile_value(&source, false).is_ok(),
                    depth == 8,
                    "{depth}"
                );
            }
            for body in ["while(x){throw x;}", "try{if(x)return x;}finally{throw x;}"] {
                let source =
                    format!("function f(x){{{body}}}export function run(a,b){{return f(a);}}");
                assert!(crate::compile_value(&source, false).is_err(), "{source}");
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
