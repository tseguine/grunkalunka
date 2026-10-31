use super::*;

#[test]
fn function_call_forwards_arguments_after_this_arg() {
    compare_function_programs(&["const f=x=>x+a;return f.call(null,b);"], false);
}

#[test]
fn function_call_keeps_runtime_callee_snapshot_before_this_arg() {
    compare_functions(&[
        "let f=x=>x+a;const g=x=>x-a;return f.call((f=g,b),b);",
        "const f=a?x=>x+a:x=>x-a;return f.call(null,b);",
    ]);
}

#[test]
fn function_call_forwards_callback_and_factory_signatures() {
    compare_function_programs(
        &[
            "const f=(g,x)=>g(x);return f.call(null,y=>y+a,b);",
            "const f=a?(g,x)=>g(x):(g,x)=>x;return +f.call(null,y=>+y,false);",
            "const make=x=>y=>x+y;const f=make.call(null,a);return f(b);",
            "const make=a?x=>y=>x+y:x=>y=>x-y;return make.call(null,a)(b);",
        ],
        false,
    );
}

#[test]
fn function_call_evaluates_ignored_receivers_once_and_keeps_capture_cells() {
    compare_function_programs(
        &[
            "const f=()=>a;return f.call();",
            "const f=()=>a;return f.call(null);",
            "let n=0;const f=x=>x+a;const out=f.call((n++,null),b,n+=2);return out+n;",
            "let n=a;const f=x=>n+x;return f.call((n=b,null),a);",
            "const f=x=>x+a;return f.call(()=>b,b);",
            "let n=0;const make=()=>{n++;return ()=>a;},f=x=>x+b;const out=f.call(make(),a);return out+n;",
            "let n=0;const make=x=>y=>x+y;const f=make.call((n++,null),a);return f(b)+n;",
            "let n=0;const make=a?x=>y=>x+y:x=>y=>x-y;return make.call((n++,null),a)(b)+n;",
            "const f=x=>x+a;return ((f.call))(null,b);",
            "const f=x=>x+a;return ((f)).call(null,b);",
        ],
        false,
    );
}

#[test]
fn function_call_preserves_named_module_calls_and_erased_types() {
    let source = "function helper(x){return x+1;}export function run(a,b){return helper.call(null,b)+helper(b);}";
    let artifact = compile(source, false).unwrap();
    let mut host = NumericInstance::new(&artifact.wasm).unwrap();
    let inputs = [
        (4.0, 2.0),
        (0.0, -0.0),
        (-2.0, -4.0),
        (f64::NAN, f64::INFINITY),
    ];
    let rust: Vec<_> = inputs
        .iter()
        .map(|&(a, b)| bits(host.run(a, b).unwrap()))
        .collect();
    let native = source.replace("export function", "function");
    node(&format!(
        "const assert=require('node:assert/strict');function bits(x){{if(Number.isNaN(x))return 'nan';const v=new DataView(new ArrayBuffer(8));v.setFloat64(0,x);return v.getBigUint64(0).toString(16).padStart(16,'0');}}const native=Function({native:?}+';return run;')();const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({:?}))).exports;const rust={rust:?};let i=0;for(const [a,b]of[[4,2],[0,-0],[-2,-4],[NaN,Infinity]]){{const expected=bits(native(a,b));assert.equal(bits(e.run(a,b)),expected);assert.equal(rust[i++],expected);}}",
        artifact.wasm
    ));
    let js = "export function run(a,b){const f=a?x=>x+b:x=>x-b;return f.call(null,a);}";
    let ts = "export function run(a:never,b:Missing):Wrong{const f=a?(x:Wrong):never=>x+b:(x:never):Wrong=>x-b;return((f as Missing).call)<never>(null,a);}";
    assert_eq!(
        compile(js, false).unwrap().wasm,
        compile(ts, true).unwrap().wasm
    );
    let metadata = js.replace(
        ".call(",
        &format!(".call<{}>(", vec!["Missing"; 1001].join(",")),
    );
    assert!(metadata.len() < 16 * 1024);
    assert_eq!(
        compile(js, false).unwrap().wasm,
        compile(&metadata, true).unwrap().wasm
    );
    for null_source in [
        "export function run(a,b){const f=x=>null;return f.call(null,b);}",
        "function helper(x){return null;}export function run(a,b){return helper(b);}",
        "function helper(x){return null;}export function run(a,b){return helper.call(null,b);}",
    ] {
        let value = compile_value(null_source, false).unwrap();
        let typed = null_source
            .replace("function helper(x)", "function helper<T>(x:never):Wrong")
            .replace("function run(a,b)", "function run(a:never,b:Missing):Wrong");
        assert_eq!(value.wasm, compile_value(&typed, true).unwrap().wasm);
        let baseline = compile_value("export function run(a,b){return null;}", false).unwrap();
        let mut host = ValueInstance::new(&value.wasm).unwrap();
        for (a, b) in inputs {
            assert!(matches!(host.run(a, b).unwrap(), Value::Null));
        }
        let native = null_source.replace("export function", "function");
        node(&format!(
            "const assert=require('node:assert/strict');const native=Function({native:?}+';return run;')();const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({:?}))).exports;const baseline=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({:?}))).exports;for(const[a,b]of[[4,2],[0,-0],[-2,-4],[NaN,Infinity]]){{assert.equal(native(a,b),null);assert.deepEqual(e.run(a,b),baseline.run(a,b));}}",
            value.wasm, baseline.wasm
        ));
    }
}

#[test]
fn function_call_keeps_the_two_mib_native_stack_contract() {
    let source = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let body = "const make=a?x=>y=>x+y:x=>y=>x-y;return make.call(null,a).call(null,b);";
            compile(&format!("export function run(a,b){{{body}}}"), false).unwrap();
            body.to_string()
        })
        .unwrap()
        .join()
        .unwrap();
    compare_function_programs(&[&source], false);
}

#[test]
fn function_call_keeps_intrinsic_property_and_signature_guards() {
    // The numeric entry profile still requires numeric module-helper bodies.
    for call in ["helper(b)", "helper.call(null,b)"] {
        let source =
            format!("function helper(x){{return null;}}export function run(a,b){{return {call};}}");
        assert!(compile(&source, false).is_err());
    }
    for body in [
        "const f=x=>x;return f.call(null);",
        "const f=x=>x;return f.call?.(null,a);",
        "const f=x=>x;return f?.call(null,a);",
        "const f=x=>x;return f.call(null,...a);",
        "const f=x=>x;return f.call(...a);",
        "const f=x=>x;return f['call'](null,a);",
        "const f=x=>x;f.call=x=>b;return f.call(null,a);",
        "const f=a?x=>x:b;return f.call(null,a);",
        "return Promise.resolve.call(null,a);",
        "return Math.abs.call(null,a);",
        "const f=x=>x;const c=f.call;return c(null,a);",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            assert!(compile(&source, ts).is_err(), "{source}");
        }
    }
}

#[test]
fn function_call_receiver_does_not_supply_a_declared_parameter() {
    let source =
        "function identity(x) { return x; } export function run(a,b) { return identity.call(a); }";
    for typescript in [false, true] {
        let error = compile(source, typescript)
            .err()
            .expect("missing forwarded input");
        assert!(
            error.contains("call is missing declared arguments"),
            "{error}"
        );
    }
}

#[test]
fn function_call_keeps_source_argument_and_callee_depth_limits() {
    for count in [999, 1000] {
        let left = (0..count)
            .map(|i| format!("x{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let right = (0..count)
            .map(|i| format!("y{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let body = format!(
            "const f=a?({left})=>x{}:({right})=>-y{};return f.call(null,{});",
            count - 1,
            count - 1,
            vec!["b"; count].join(",")
        );
        assert!(body.len() < 16 * 1024 - 40);
        if count == 999 {
            compare_functions(&[&body]);
        } else {
            assert!(
                compile(&format!("export function run(a,b){{{body}}}"), false)
                    .err()
                    .unwrap()
                    .contains("1000 ordinary")
            );
        }
    }
    for count in [126, 127] {
        let body = format!(
            "const f=x=>x+b;return {}f.call{}(null,a);",
            "(".repeat(count),
            ")".repeat(count)
        );
        if count == 126 {
            compare_function_programs(&[&body], false);
        } else {
            assert!(compile(&format!("export function run(a,b){{{body}}}"), false).is_err());
        }
        for (helper, args) in [
            ("function helper(x){return x+1;}", "null,a"),
            ("function helper(){return 1;}", ""),
        ] {
            let source = format!(
                "{helper}export function run(a,b){{return {}helper.call{}({args});}}",
                "(".repeat(count),
                ")".repeat(count)
            );
            if count == 126 {
                assert!(compile(&source, false).is_ok());
            } else {
                assert!(compile(&source, false).is_err(), "{source}");
            }
        }
    }
}
