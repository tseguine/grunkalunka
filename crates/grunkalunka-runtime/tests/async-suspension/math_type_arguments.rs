use super::*;

fn compare_typed(plain: &str, typed: &str) {
    assert_eq!(
        compile_async(plain, false).unwrap().wasm,
        compile_async(typed, true)
            .unwrap_or_else(|error| panic!("{typed}: {error}"))
            .wasm,
    );
    compare_async_sources(
        &[plain.to_string()],
        &[(4.0, 2.0), (0.0, -0.0), (f64::NAN, f64::INFINITY)],
    );
}

#[test]
fn math_type_arguments_erase_inside_async_intrinsics() {
    for method in ["abs", "sqrt", "floor", "ceil", "trunc"] {
        for input in ["await a", "null", "true", "", "a<b"] {
            let plain = source_for(&format!("return Math.{method}({input});"));
            let typed = plain.replace(
                &format!("Math.{method}("),
                &format!("Math.{method}<Missing>("),
            );
            compare_typed(&plain, &typed);
        }
    }
}

#[test]
fn math_type_arguments_preserve_numeric_and_primitive_execution() {
    use grunkalunka_compiler::{compile, compile_value};
    use grunkalunka_runtime::{NumericInstance, ValueInstance};
    let vectors = [
        (4.0, 2.0),
        (0.0, -0.0),
        (-0.0, -2.0),
        (1.5, -1.5),
        (-1.5, 1.5),
        (f64::NAN, f64::INFINITY),
        (f64::INFINITY, f64::NEG_INFINITY),
        (-2.0, -4.0),
    ];
    let bits = |value: f64| {
        if value.is_nan() {
            "nan".to_string()
        } else {
            format!("{:016x}", value.to_bits())
        }
    };
    let mut cases = Vec::new();
    for method in ["abs", "sqrt", "floor", "ceil", "trunc"] {
        for body in [
            format!("return Math.{method}(a);"),
            format!("return Math.{method}(true);"),
            format!("return Math.{method}(null);"),
            format!("return Math.{method}();"),
            format!("return Math.{method}(a,a=b)+a;"),
            format!("return Math.{method}(a<b,a=b);"),
            format!("const undefined=a;return Math.{method}();"),
        ] {
            let plain = format!("export function run(a,b) {{ {body} }}");
            let typed = plain.replace(
                &format!("Math.{method}("),
                &format!("Math.{method}<number>("),
            );
            for primitive in [false, true] {
                let compiler = if primitive { compile_value } else { compile };
                let wasm = compiler(&typed, true).unwrap().wasm;
                assert_eq!(compiler(&plain, false).unwrap().wasm, wasm, "{typed}");
                let rows = if primitive {
                    let mut run = ValueInstance::new(&wasm).unwrap();
                    vectors
                        .iter()
                        .map(|&(a, b)| match run.run(a, b).unwrap() {
                            Value::Number(value) => bits(value),
                            value => panic!("Math returned {value:?}: {typed}"),
                        })
                        .collect::<Vec<_>>()
                } else {
                    let mut run = NumericInstance::new(&wasm).unwrap();
                    vectors
                        .iter()
                        .map(|&(a, b)| bits(run.run(a, b).unwrap()))
                        .collect::<Vec<_>>()
                };
                cases.push(format!(
                    "{{source:{plain:?},wasm:{wasm:?},primitive:{primitive},rust:{rows:?}}}"
                ));
            }
        }
    }
    run_node(&format!(
        r#"import assert from 'node:assert/strict';
const view=new DataView(new ArrayBuffer(8));
function bits(value){{if(Number.isNaN(value))return 'nan';view.setFloat64(0,value);return view.getBigUint64(0).toString(16).padStart(16,'0');}}
const inputs=[[4,2],[0,-0],[-0,-2],[1.5,-1.5],[-1.5,1.5],[NaN,Infinity],[Infinity,-Infinity],[-2,-4]];
for(const c of [{}]){{
 const native=new Function(c.source.replace('export function','function')+';return run;')();
 const guest=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array(c.wasm))).exports.run;
 for(const [i,args] of inputs.entries()){{const expected=bits(native(...args));const raw=guest(...args);if(c.primitive)assert.equal(raw[0],3);assert.equal(bits(c.primitive?raw[1]:raw),expected,c.source);assert.equal(c.rust[i],expected,c.source);}}
}}
console.log('ASYNC_TEST_DONE');"#,
        cases.join(",")
    ));
}

#[test]
fn math_type_arguments_preserve_async_snapshots_private_values_and_failure() {
    for method in ["abs", "sqrt", "floor", "ceil", "trunc"] {
        for body in [
            format!("let x=a;return Math.{method}(x,await(x=b))+(+x);"),
            format!("try{{return Math.{method}(a,await Promise.reject(b));}}catch(e){{return e;}}"),
            format!(
                "try{{return Math.{method}(await Promise.reject(b),a=b);}}catch(e){{return a;}}"
            ),
            format!("const p=Promise.resolve(a);return Math.{method}(p);"),
            format!("const f=x=>Math.{method}(x);const p=Promise.resolve(a);f(p);return f(b);"),
            format!(
                "const p=Promise.resolve(a);let x=a?p:b;const read=()=>Math.{method}(x);return read();"
            ),
        ] {
            let plain = source_for(&body);
            let typed = plain.replace(
                &format!("Math.{method}("),
                &format!("Math.{method}<never>("),
            );
            compare_typed(&plain, &typed);
        }
    }
}

#[test]
fn math_type_arguments_keep_runtime_bindings_and_admission_limits() {
    use grunkalunka_compiler::{compile, compile_value};
    let plain = "export function run(a,b) { return Math.abs(a); }";
    for metadata in [
        "Missing",
        "never",
        "typeof absent",
        "Missing,Other",
        "{call:()=>void}",
    ] {
        let typed = plain.replace("Math.abs(", &format!("Math.abs<{metadata}>("));
        assert_eq!(
            compile(plain, false).unwrap().wasm,
            compile(&typed, true).unwrap().wasm
        );
    }
    for declaration in ["type Math=never;", "interface Math {}"] {
        let typed = format!(
            "{declaration}{}",
            plain.replace("Math.abs(", "Math.abs<Math>(")
        );
        assert_eq!(
            compile(plain, false).unwrap().wasm,
            compile(&typed, true).unwrap().wasm
        );
    }
    for count in [1000, 1001] {
        let args = vec!["a"; count].join(",");
        let plain = format!("export function run(a,b){{return Math.abs({args});}}");
        let typed = plain.replace("Math.abs(", "Math.abs<never>(");
        if count == 1000 {
            assert_eq!(
                compile(&plain, false).unwrap().wasm,
                compile(&typed, true).unwrap().wasm
            );
        } else {
            assert!(compile(&typed, true).err().unwrap().contains("1000"));
        }
    }
    for body in [
        "const Math=a;return Math.abs<never>(b);",
        "return Math.random<never>(a);",
        "return Math.abs<never>(...a);",
        "return Math.abs?.<never>(a);",
        "return (Math.abs)<never>(a);",
        "return Math['abs']<never>(a);",
        "return Math.abs<never>('1');",
        "return Math.abs<never>({});",
        "return Math.abs<never>(1n);",
        "return Math.abs<never>(absent);",
        "Math.abs=a;return Math.abs<never>(b);",
        "const f=x=>x;return f<never>(a);",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(compile(&source, true).is_err(), "{source}");
        assert!(compile_value(&source, true).is_err(), "{source}");
        assert!(compile_async(&source_for(body), true).is_err(), "{body}");
    }
}
