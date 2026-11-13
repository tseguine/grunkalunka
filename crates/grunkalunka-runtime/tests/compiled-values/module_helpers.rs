use super::*;

#[test]
fn scalar_pattern_general_helper_parameters_preserve_tags() {
    compare_helpers(&[
        "export function run(a,b){const f=x=>{[x]=[false];return x;};return f(null);}",
        "export function run(a,b){const f=x=>{({x}={x:null});return x;};return f(false);}",
        "export function run(a,b){const f=x=>{({x}={});return x;};return f(null);}",
    ]);
}

#[test]
fn scalar_iteration_assignments_preserve_selected_primitive_tags() {
    compare_helpers(&[
        "export function run(a,b){let x=0;for(x of [a,b]){}return x;}",
        "export function run(a,b){let x=undefined;for(x of [null]){}return x;}",
        "export function run(a,b){let x=undefined;for(x of [false]){}return x;}",
        "export function run(a,b){let x;for(x of []){}return x;}",
        "export function run(a,b){let x=0;for(x of [a])x=false;return x;}",
    ]);
}

#[test]
fn member_iteration_heads_preserve_primitive_tags_and_number_bits() {
    compare_helpers(&[
        "export function run(a,b){const o={x:undefined};for(o.x of [a,b]){}return o.x;}",
        "export function run(a,b){const o={x:undefined};for(o.x of [null]){}return o.x;}",
        "export function run(a,b){const o={x:undefined};for(o.x of [false]){}return o.x;}",
        "export function run(a,b){const o={x:undefined};for(o.x of []){}return o.x===undefined;}",
    ]);
}

#[test]
fn optional_delete_returns_boolean_tags_for_present_and_nullish_receivers() {
    compare_helpers(&[
        "export function run(a,b){const o=a?{x:b}:null;return delete o?.x;}",
        "export function run(a,b){const xs=a?[b]:undefined;return delete xs?.[a>>>0];}",
        "export function run(a,b){const k='x';return delete null?.[k];}",
    ]);
}

#[test]
fn optional_dictionary_reads_preserve_selected_primitive_tags_and_number_bits() {
    compare_helpers(&[
        "export function run(a,b){const d=Object.create(null);d.x=b;const o=a?d:null;const k='x';return o?.[k];}",
        "export function run(a,b){const d=Object.create(null);d.x=null;const k='x';return d?.[k];}",
        "export function run(a,b){const d=Object.create(null);d.x=false;const k=a?'x':'missing';return d?.[k];}",
        "export function run(a,b){const k='x';return null?.[k];}",
    ]);
}

#[test]
fn function_for_in_keeps_primitive_tags_after_empty_enumeration() {
    compare_helpers(&[
        "export function run(a,b){let value=null;const f=x=>x+a;for(const k in f)value=a;return value;}",
        "export function run(a,b){let value=false;const f=x=>x+a;for(const k in f)value=a;return value;}",
    ]);
}

#[test]
fn optional_number_string_indices_return_boolean_comparison_tags() {
    compare_helpers(&[
        "export function run(a,b){const s=a?'ab':null;return s?.[b]===undefined;}",
        "export function run(a,b){const s='ab';return s?.[a/2]==='a';}",
    ]);
}

#[test]
fn optional_string_length_and_comparisons_preserve_primitive_tags() {
    compare_helpers(&[
        "export function run(a,b){const s=a?'ab':null;return s?.length;}",
        "export function run(a,b){const s=a?'ab':undefined;return s?.['length'];}",
        "export function run(a,b){const s=a?'ab':null;return s?.[0]==='a';}",
    ]);
}

#[test]
fn unsigned_optional_reads_preserve_primitive_tags_and_number_bits() {
    compare_helpers(&[
        "export function run(a,b){const xs=a?[b]:null;return xs?.[0>>>0];}",
        "export function run(a,b){const xs=[null,false,undefined];return xs?.[a>>>0];}",
        "export function run(a,b){return null?.[a>>>0];}",
    ]);
}

#[test]
fn optional_static_reads_preserve_exact_primitive_tags_and_number_bits() {
    compare_helpers(&[
        "export function run(a,b){const o=a?{x:b}:null;return o?.x;}",
        "export function run(a,b){const o={x:null};return o?.x;}",
        "export function run(a,b){const o=a?{x:false}:undefined;return o?.x;}",
        "export function run(a,b){const xs=a?[b]:null;return xs?.[0];}",
        "export function run(a,b){return null?.x;}",
        "export function run(a,b){const xs=a?[b]:null;return xs?.length;}",
    ]);
}

#[test]
fn scalar_for_in_keeps_primitive_tags_when_no_iteration_executes() {
    compare_helpers(&[
        "export function run(a,b){let value=null;for(const k in false)value=a;return value;}",
        "export function run(a,b){var key;let value=undefined;for(var key in null)value=a;return value;}",
        "export function run(a,b){let value=false;for(const k in undefined)value=a;return value;}",
    ]);
    // Loop-head facts still include possible String keys. Preserve this
    // projection guard rather than treating an empty loop as flow narrowing.
    let source = "export function run(a,b){var key;for(var key in null){}return key;}";
    for typed in [false, true] {
        let error = compile_value(source, typed)
            .err()
            .expect("mixed loop-head projection stays guarded");
        assert!(
            error.contains("managed value requires an identity or managed storage position"),
            "{error}"
        );
    }
}

#[test]
fn numeric_property_updates_return_number_tags_for_every_admitted_key_kind() {
    compare_helpers(&[
        "export function run(a,b){const o={n:false};return o.n++;}",
        "export function run(a,b){const o={n:null};return ++o.n;}",
        "export function run(a,b){const xs=[a];return xs[0>>>0]++;}",
        "export function run(a,b){const d=Object.create(null);d.x=false;const k='x';return d[k]++;}",
        "export function run(a,b){const o={};return o.n++;}",
        "export function run(a,b){const o={n:a?false:null};return o.n-=b;}",
    ]);
}

#[test]
fn unsigned_logical_writes_preserve_tags_and_bits() {
    compare_helpers(&[
        "export function run(a,b){const xs=[a];return xs[0>>>0]||=b;}",
        "export function run(a,b){const xs=[a];return xs[0>>>0]&&=b;}",
        "export function run(a,b){const xs=[null,false,undefined];return xs[a>>>0]??=b;}",
    ]);
}

#[test]
fn unsigned_property_reads_preserve_tags_and_bits() {
    compare_helpers(&[
        "export function run(a,b){const xs=[a,b];return xs[a>>>0];}",
        "export function run(a,b){const xs=[null,false,undefined];return xs[a>>>0];}",
    ]);
}

#[test]
fn unsigned_property_writes_return_exact_rhs_tags_and_bits() {
    compare_helpers(&[
        "export function run(a,b){const xs=[];return xs[a>>>0]=b;}",
        "export function run(a,b){const o={};return o[a>>>0]=null;}",
        "export function run(a,b){const d=Object.create(null);return d[a>>>0]=false;}",
    ]);
}

#[test]
fn unsigned_in_keys_return_boolean_completions() {
    compare_helpers(&[
        "export function run(a,b){const xs=[a,,b];return (a>>>0) in xs;}",
        "export function run(a,b){const o={0:undefined};return (a>>>0) in o;}",
    ]);
}

#[test]
fn uint32_delete_keys_return_boolean_completions() {
    compare_helpers(&[
        "export function run(a,b){const xs=[a,b];return delete xs[a>>>0];}",
        "export function run(a,b){const o={0:a};return delete o[a>>>0];}",
    ]);
}

#[test]
fn debugger_statements_preserve_primitive_and_helper_completions() {
    compare_helpers(&[
        "debugger;export function run(a,b){return a;}",
        "export function run(a,b){debugger;return a;}",
        "export function run(a,b){if(a){debugger;}else{debugger;}return null;}",
        "function f(){debugger;}export function run(a,b){return f();}",
        "export function run(a,b){const f=()=>{debugger;return false;};return f();}",
        "export function run(a,b){try{debugger;return a;}finally{debugger;}}",
    ]);
}

#[test]
fn literal_call_spreads_preserve_argument_order_defaults_and_callee_snapshots() {
    compare_helpers(&[
        "function f(x){return x;}export function run(a,b){return f(...[a]);}",
        "export function run(a,b){let f=x=>x;return f(...[a]);}",
        "function f(x,y){return x+y;}export function run(a,b){return f(...[a,b]);}",
        "export function run(a,b){const f=(x,y)=>x+y;return f(...[a],b);}",
        "export function run(a,b){const o={f:(x,y)=>x+y};return o.f(...[a,b]);}",
        "function f(x=3){return x;}export function run(a,b){return f(...[]);}",
        "function f(x,y){return y;}export function run(a,b){return f(...[a]);}",
        "function f(x){return x;}export function run(a,b){let n=0;const x=f(...[(n=1,a),(n=n*10+2,b)],n=n*10+3);return n;}",
        "export function run(a,b){let f=x=>x;return f(...[a,f=x=>b]);}",
        "function f(x,y){return x+y;}export function run(a,b){return f.call(null,...[a,b]);}",
        "function f(x,y){return x+y;}export function run(a,b){return f(...[...[a],b]);}",
        "function f(x=0){return x;}export function run(a,b){return f(...[null]);}",
        "function f(x=0){return x;}export function run(a,b){return f(...[false]);}",
        "function f(x,y=x){return y;}export function run(a,b){return f(...[a,undefined]);}",
    ]);
}

#[test]
fn unary_parameter_defaults_preserve_tags_truthiness_and_saved_arguments() {
    compare_helpers(&[
        "function f(x,y=void x){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=+x){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=-x){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=-x){return y;}export function run(a,b){return f(null);}",
        "function f(x,y=+x){return y;}export function run(a,b){return f(false);}",
        "function f(x,y=+x){return y;}export function run(a,b){return f();}",
        "function f(x,y=!x){return y;}export function run(a,b){return f(a);}",
        "function f(x,y=!x){return y;}export function run(a,b){return f('');}",
        "function f(x,y=!x){return y;}export function run(a,b){return f({});}",
        "function f(x,y=+!!x){return y;}export function run(a,b){return f('abc');}",
        "function f(x,y=+void x){return y;}export function run(a,b){return f([]);}",
        "export function run(a,b){const o={f:(x,y=-x)=>y};return o.f(a);}",
        "export function run(a,b){const f=(x,y=-x)=>()=>y;const g=f(a);return g();}",
        "function f(x,y=-x){return y;}export function run(a,b){return f(a,b);}",
        "export function run(a,b){const f=(x,y=-x)=>y;return f(a,undefined,a=b);}",
    ]);
}

#[test]
fn arithmetic_parameter_defaults_keep_number_bits_and_supplied_tags() {
    compare_helpers(&[
        "function f(x=1**2){return x;}export function run(a,b){return f();}",
        "function f(x,y=x**2){return y;}export function run(a,b){return f(a);}",
        "function f(x,z=x**2){return z;}export function run(a,b){return f(a);}",
        "function f(x,y=x**-1){return y;}export function run(a,b){return f(a,b);}",
        "function f(x,y=x**-1){return y;}export function run(a,b){return f(null);}",
        "function f(x=NaN**0){return x;}export function run(a,b){return f();}",
        "function f(x=1+2){return x;}export function run(a,b){return f();}",
        "export function run(a,b){const f=(x=1+2)=>x;return f();}",
        "function f(x=2*(3+4)){return x;}export function run(a,b){return f();}",
        "function f(x=-0/3){return x;}export function run(a,b){return f();}",
        "function f(x=-4%2){return x;}export function run(a,b){return f();}",
        "function f(x=1/0){return x;}export function run(a,b){return f();}",
        "function f(x=0/0){return x;}export function run(a,b){return f();}",
        "function f(x=Infinity-Infinity){return x;}export function run(a,b){return f();}",
        "function f(x=void (1+2)){return x;}export function run(a,b){return f();}",
        "function f(x=1+2){return x;}export function run(a,b){return f(a);}",
        "function f(x=1+2){return x;}export function run(a,b){return f(null);}",
        "function f(x=1+2){return x;}export function run(a,b){return f(false);}",
        "export function run(a,b){const o={f:(x=2*(3+4))=>x};return o.f();}",
        "export function run(a,b){const f=(x=1+2,y=x)=>()=>y;const g=f();return g();}",
    ]);
}

#[test]
fn earlier_parameter_defaults_preserve_tags_order_aliases_and_shared_calls() {
    compare_helpers(&[
        "function f(x,y=x){return y;}export function run(a,b){return f(a);}",
        "function f(x=3,y=x,z=y){return z;}export function run(a,b){return f();}",
        "function f(x,y=x){return y;}export function run(a,b){return f();}",
        "function f(x,y=x){return y;}export function run(a,b){return f(null);}",
        "function f(x,y=x){return y;}export function run(a,b){return f(false);}",
        "function f(x,y=x){return y;}export function run(a,b){return f(-0);}",
        "function f(x,y=x){return y;}export function run(a,b){return f(NaN);}",
        "function f(x,y=x){return y;}export function run(a,b){return f(a,b);}",
        "function f(x,y=x){x=9;return y;}export function run(a,b){return f(a);}",
        "function f(x=3,y=x){var x=9;return y;}export function run(a,b){return f();}",
        "function f(undefined,x=undefined){return x;}export function run(a,b){return f(a);}",
        "function f(Infinity,x=Infinity){return x;}export function run(a,b){return f(a);}",
        "function f(NaN,x=NaN){return x;}export function run(a,b){return f(a);}",
        "export function run(a,b){const f=(undefined,x=undefined)=>x;return f();}",
        "export function run(a,b){const f=(x,y=x)=>y;return f(a);}",
        "export function run(a,b){const f=(x=3,y=x)=>y;return f();}",
        "export function run(a,b){const f=(x=3,y=x)=>()=>y;const g=f();return g();}",
        "export function run(a,b){const o={f:(x=3,y=x)=>y};return o.f();}",
        "function f(x,y=x){return y;}export function run(a,b){const g=f;return g(a);}",
        "export function run(a,b){const f=(x,y=x)=>y;return f(a,undefined,a=b);}",
    ]);
}

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
                compile_value(source, false)
                    .unwrap_or_else(|error| panic!("{source}: {error}"))
                    .wasm,
                compile_value(&typed, true)
                    .unwrap_or_else(|error| panic!("{typed}: {error}"))
                    .wasm,
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
        "function helper(x){for(let i=x;true;i++){if(x)return i;else return 2;}}export function run(a,b){return helper(a);}",
        "function helper(x){for(;;){if(x)break;return x;}}export function run(a,b){return helper(a);}",
        "function helper(x){while(true){if(x)return x;else return 7;}}export function run(a,b){return helper(a);}",
        "function helper(x){while(true){if(x)break;return x;}}export function run(a,b){return helper(a);}",
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

#[test]
fn synchronous_arrow_defaults_preserve_selections_captures_and_callable_abi() {
    compare_helpers(&[
        "export function run(a,b){return ((x=3)=>x)();}",
        "export function run(a,b){const f=(x=3)=>x;return f();}",
        "export function run(a,b){const f=(x=3)=>x;return f(a);}",
        "export function run(a,b){const f=(x=3)=>x;return f(undefined);}",
        "export function run(a,b){const f=(x=3)=>x;return f(null);}",
        "export function run(a,b){const f=(x=3)=>x;return f(false);}",
        "export function run(a,b){const f=(x=-0)=>x;return f();}",
        "export function run(a,b){const f=(x=true)=>x;return f();}",
        "export function run(a,b){const f=(x=null)=>x;return f();}",
        "export function run(a,b){const f=(x=undefined)=>x;return f();}",
        "export function run(a,b){const f=(x='abc')=>x.length;return f();}",
        "export function run(a,b){const f=(x='abc')=>x.length;return f('hello');}",
        r"export function run(a,b){const f=(x='\u{1f600}\ud800')=>x.charCodeAt(2)+x.length;return f();}",
        "export function run(a,b){const f=(x=3,y=4)=>x*10+y;return f();}",
        "export function run(a,b){const f=(x=3,y=4)=>x*10+y;return f(undefined,a);}",
        "export function run(a,b){const f=(x=3)=>{x++;return x;};return f();}",
        "export function run(a,b){const f=(x=3)=>()=>x;const g=f();return g();}",
        "export function run(a,b){const f=(x=3)=>y=>x+y;return f()(a);}",
        "export function run(a,b){const f=(x=3)=>x;return f(a,a=b)+a;}",
        "export function run(a,b){const f=(x=3)=>x;return f(undefined,a=b)+a;}",
        "export function run(a,b){let x=a;const f=(y=3)=>y+x;x=b;return f();}",
        "export function run(a,b){const f=(x=3)=>x;const g=f;return g();}",
        "export function run(a,b){const f=a?((x=3)=>x):((x=4)=>x);return f();}",
        "export function run(a,b){let f=(x=3)=>x;const result=f(undefined,f=(x=4)=>x);return result+f();}",
        "export function run(a,b){const o={f:(x=3)=>x};return o.f();}",
        "export function run(a,b){const f=(x=3)=>x;const invoke=g=>g(a);return invoke(f);}",
        "export function run(a,b){const f=(x=3)=>x;const invoke=g=>g(undefined);return invoke(f);}",
        "export function run(a,b){const f=(x=3)=>x;const invoke=g=>g(null);return invoke(f);}",
        "export function run(a,b){const f=(x=3)=>x;const invoke=g=>g(false);return invoke(f);}",
        "export function run(a,b){const f=(x=3)=>x;return f.length;}",
        "export function run(a,b){const f=(x,y=3,z)=>y;return f.length;}",
        "export function run(a,b){const f=(x=3)=>x;const g=f;return g.length;}",
    ]);
}

#[test]
fn intrinsic_and_void_parameter_defaults_keep_tags_resolution_and_selection() {
    compare_helpers(&[
        "function f(x=Infinity){return x;}export function run(a,b){return f();}",
        "function f(x=-Infinity){return x;}export function run(a,b){return f(undefined);}",
        "function f(x=NaN){return x;}export function run(a,b){return f();}",
        "function f(x=Infinity){return x;}export function run(a,b){return f(a);}",
        "function f(x=Infinity){return x;}export function run(a,b){return f(null);}",
        "function f(x=NaN){return x;}export function run(a,b){return f(false);}",
        "function f(x=void 0){return x;}export function run(a,b){return f();}",
        "function f(x=void 'abc'){return x;}export function run(a,b){return f();}",
        "function f(x=void void null){return x;}export function run(a,b){return f(a);}",
        "function f(x=Infinity){var Infinity=3;return x;}export function run(a,b){return f();}",
        "export function run(a,b){const f=(x=-Infinity)=>x;return f();}",
        "export function run(a,b){const f=(x=void 0)=>x;return f();}",
        "export function run(a,b){const o={f:(x=NaN)=>x};return o.f();}",
        "export function run(a,b){const o={f:(x=void null)=>x};return o.f();}",
        "function f(x=Infinity){const get=()=>x;x=3;return get();}export function run(a,b){return f();}",
        "export function run(a,b){const f=(x=Infinity)=>x;return f.length;}",
    ]);
}

#[test]
fn boxed_key_guest_operations_execute_on_fast_pr_profile() {
    // Full managed-source oracle sweeps run on master. Exercise one real
    // Wasm-backed witness for each shared boxed-key lowering on PRs too.
    compare_helpers(&[
        "export function run(a,b){const d=Object.create(null);const k=a?'x':b;return d[k]??=b;}",
        "export function run(a,b){const d=Object.create(null);const k=a?'x':b;d[k]=b;return ++d[k];}",
        "export function run(a,b){const d=Object.create(null);d.x=b;const row=a?d:null;const k=a?'x':b;return delete row?.[k];}",
        "export function run(a,b){const d=Object.create(null);d.x=b;const k=a?'x':b;let v;({[k]:v}=d);return v;}",
        "export function run(a,b){const d=Object.create(null);const k=a?'x':b;[d[k]]=[b];return d[k];}",
    ]);
}

#[test]
fn string_primitive_keys_use_guest_topropertykey_in_fast_pr_smoke() {
    compare_helpers(&[
        "export function run(a,b){const s='ab';const k=a?0:false;return s[k]===(a?'a':undefined);}",
        "export function run(a,b){const s='ab';const k=a?1:null;return s[k]===(a?'b':undefined);}",
        "export function run(a,b){const s='ab';const k=a?true:0;return s[k]===(a?undefined:'a');}",
    ]);
}

#[test]
fn optional_string_primitive_keys_keep_nullish_skip_on_fast_pr_profile() {
    compare_helpers(&[
        "export function run(a,b){const s=a?'ab':null;const k=b?0:false;return s?.[k]===(a&&b?'a':undefined);}",
        "export function run(a,b){const s=a?'ab':undefined;const k=b?1:null;return s?.[k]===(a&&b?'b':undefined);}",
    ]);
}

#[test]
fn boxed_primitive_string_destructuring_runs_on_fast_pr_profile() {
    compare_helpers(&[
        "export function run(a,b){const k=a?0:false;const {[k]:ch}='ab';return ch===(a?'a':undefined);}",
        "export function run(a,b){const k=a?1:null;const {[k]:ch='?'}='ab';return ch===(a?'b':'?');}",
        "export function run(a,b){const k=a?0:false;const {[k]:ch,...rest}='ab';return Object.keys(rest).length===(a?1:2);}",
    ]);
}

#[test]
fn boxed_string_destructuring_assignments_run_in_fast_pr_profile() {
    compare_helpers(&[
        "export function run(a,b){const k=a?0:false;let ch;({[k]:ch}='ab');return ch===(a?'a':undefined);}",
        "export function run(a,b){const k=a?1:null;let ch;({[k]:ch='?'}='ab');return ch===(a?'b':'?');}",
    ]);
}

#[test]
fn numeric_keys_in_string_assignment_patterns_execute_on_fast_pr_profile() {
    compare_helpers(&[
        "export function run(a,b){let x;({[a]:x}='text');return x===(a===0?'t':undefined)?1:0;}",
    ]);
}
