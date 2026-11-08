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
    compare_helpers(&[
        "function helper(x){return null;}export function run(a,b){return helper();}",
    ]);
    for source in [
        "function helper(x){return null;}export function run(a,b){return helper(...a);}",
        "function helper(x){return null;}export function run(a,b){return helper?.(a);}",
        "function helper(x){return null;}export function run(a,b){return helper(null);}",
        "function helper(x){return Promise.resolve(x);}export function run(a,b){return helper(b);}",
        "function helper(x){return y=>y;}export function run(a,b){return helper(b);}",
        "function helper(x){return 'text';}export function run(a,b){return helper(b);}",
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

#[test]
fn bare_named_helper_return_executes_the_exact_former_guard() {
    compare_helpers(&["function helper(x){return;}export function run(a,b){return helper(b);}"]);
}

#[test]
fn side_effect_only_helpers_preserve_undefined_and_caller_state() {
    compare_helpers(&[
        "function helper(x){}export function run(a,b){return helper(a);}",
        "function helper(x){x++;}export function run(a,b){return helper(a);}",
        "function helper(x){if(x)return;}export function run(a,b){return helper(a);}",
        "function helper(x){const f=()=>x;}export function run(a,b){return helper(a);}",
        "function helper(x){x++;}export function run(a,b){const f=helper;return f(a);}",
        "function helper(x){x++;}export function run(a,b){const f=x=>helper(x);return f(a);}",
        "function helper(o){o.x++;}export function run(a,b){const o={x:a};const result=helper(o);return result??o.x;}",
        "function helper(o){try{if(o.x)return;}finally{o.x++;}}export function run(a,b){const o={x:a};const result=helper(o);return result??o.x;}",
    ]);
}

#[test]
fn conditional_helper_values_keep_the_implicit_undefined_path() {
    compare_helpers(&[
        "function helper(x) { if(x) return x; } export function run(a,b) { return helper(a); }",
        "function helper(x){if(x)return null;}export function run(a,b){return helper(b);}",
        "function helper(x){if(x)return x;}export function run(a,b){return helper(a);}",
        "function helper(x){{if(x)return false;}}export function run(a,b){return helper(a);}",
        "function helper(x){if(x){if(x>1)return 3;}else{if(x<0)return 4;}}export function run(a,b){return helper(a);}",
        "function helper(x){if(x)return x;}export function run(a,b){const f=helper;return f(a);}",
        "function helper(x){if(x)return x;}export function run(a,b){const f=x=>helper(x);return f(a);}",
    ]);
}

#[test]
fn helper_control_flow_preserves_normal_and_abrupt_completions() {
    compare_helpers(&[
        "function helper(x){while(x){return x;}}export function run(a,b){return helper(a);}",
        "function helper(x){for(const y of [x]){if(x)return y;}}export function run(a,b){return helper(a);}",
        "function helper(x){const o=Object.create(null);o.a=x;for(const key in o){if(x)return x;}}export function run(a,b){return helper(a);}",
        "function helper(x){try{throw x;}catch(e){if(e)return e;}finally{x++;}}export function run(a,b){return helper(a);}",
        "function helper(x){for(let i=0;i<2;i++){if(x)return i;}}export function run(a,b){return helper(a);}",
        "function helper(x){do{if(x)break;return x;}while(false);}export function run(a,b){return helper(a);}",
        "function helper(x){do{if(x)continue;return x;}while(false);}export function run(a,b){return helper(a);}",
        "function helper(x){switch(x){case 1:return 3;}}export function run(a,b){return helper(a);}",
        "function helper(x){switch(x){case 0:break;default:return 3;}}export function run(a,b){return helper(a);}",
        "function helper(x){try{if(x)return x;}finally{x++;}}export function run(a,b){return helper(a);}",
        "function helper(x){try{throw x;}catch(e){if(e)return e;}}export function run(a,b){return helper(a);}",
        "function helper(x){try{return x;}finally{if(x)return;}}export function run(a,b){return helper(a);}",
    ]);
}

#[test]
fn primitive_helper_defaults_select_only_undefined() {
    compare_helpers(&[
        "function f(x=3){return x;}export function run(a,b){return f();}",
        "function f(x=3){return x;}export function run(a,b){return f(undefined);}",
        "function f(x=3){return x;}export function run(a,b){return f(a);}",
        "function f(x=3){return x;}export function run(a,b){return f(null);}",
        "function f(x=3){return x;}export function run(a,b){return f(false);}",
        "function f(x=-0){return x;}export function run(a,b){return f();}",
        "function f(x=true){return x;}export function run(a,b){return f();}",
        "function f(x=null){return x;}export function run(a,b){return f();}",
        "function f(x=undefined){return x;}export function run(a,b){return f();}",
        "function f(x='abc'){return x.length;}export function run(a,b){return f();}",
        "function f(x='abc'){return x.length;}export function run(a,b){return f('hello');}",
        r"function f(x='\u{1f600}\ud800'){return x.charCodeAt(2)+x.length;}export function run(a,b){return f();}",
        "function f(x=3,y){return y;}export function run(a,b){return f();}",
        "function f(x=3,y=4){return x*10+y;}export function run(a,b){return f();}",
        "function f(x=3,y=4){return x*10+y;}export function run(a,b){return f(undefined,a);}",
        "function f(x=3){x++;return x;}export function run(a,b){return f();}",
        "function f(x=3){const g=()=>x;return g();}export function run(a,b){return f();}",
        "function f(x=3){if(x>0)return f(x-1);return x;}export function run(a,b){return f();}",
        "function f(x=3){return x;}export function run(a,b){const g=f;return g();}",
        "function f(x=3){return x;}export function run(a,b){return f(a,a=b)+a;}",
        "function f(x=3){return x;}export function run(a,b){return f(undefined,a=b)+a;}",
    ]);
}

#[test]
fn default_parameter_reflection_uses_first_default_position() {
    compare_helpers(&[
        "function f(x=3){return x;}export function run(a,b){return f.length;}",
        "function f(x,y=3,z){return y;}export function run(a,b){return f.length;}",
        "function f(x=3){return x;}export function run(a,b){const g=f;return g.length;}",
        "function f(x=3){return x;}function g(x=4){return x;}export function run(a,b){const h=a?f:g;return h.length;}",
        "function f(x=3){return x;}function g(x){return x;}export function run(a,b){return g.length;}",
    ]);
}
