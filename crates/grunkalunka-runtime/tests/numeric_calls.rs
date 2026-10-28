use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};
use wasmtime::Trap;

#[test]
fn direct_calls_preserve_argument_snapshots_and_parameter_isolation() {
    let artifact = compile(
        "function pack(x,y) { x=x*10+y; return x; } export function run(a,b) { let x=5; const result=pack(x,x=3); return result*10+x; }",
        false,
    ).unwrap();
    let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
    assert_eq!(instance.run(0.0, 0.0).unwrap(), 533.0);
}

#[test]
fn helpers_evaluate_and_ignore_surplus_arguments() {
    compare_helper_sources(&[
        "function identity(x) { return x; } export function run(a,b) { return identity(a,b); }",
        "function identity(x){return x;} export function run(a,b){return identity(a,(a=b,null))+a;}",
        "function value(){return 3;} export function run(a,b){const n=value(a=b,undefined,false);return n+a;}",
        "function choose(a){return a;} export function run(a,b){return choose(a,a=b);}",
        "function first(x){return second(x,x=2,null);} function second(x){return x+1;} export function run(a,b){return first(a,b);}",
        "function fold(n,x){if(n<=0)return x;return fold(n-1,x+1,null);} export function run(a,b){return fold(3,a,b);}",
        "export function run(a,b){if(b===12345)return a;return run(a,12345,a=b);}",
        "function identity(x){return x;} export function run(a,b){const make=offset=>(a=offset,()=>offset);return identity(a,make(b))+a;}",
        "function identity(x){return x;} export function run(a,b){return identity(a,()=>null);}",
        "function identity(x){return x;} function pack(x,y){return x*10+y;} export function run(a,b){return pack(a,b,identity(a=b))+a;}",
    ]);
}

#[test]
fn known_local_arrows_preserve_primitive_inputs() {
    compare_helper_sources(&[
        "export function run(a,b){const f=x=>+x;return f(null);}",
        "export function run(a,b){const f=x=>+x;return f(undefined);}",
        "export function run(a,b){const f=x=>+x;return f(true)+f(false);}",
        "export function run(a,b){const f=x=>x===null?1:2;return f(null);}",
        "export function run(a,b){const f=x=>x===undefined?1:2;return f(undefined);}",
        "export function run(a,b){const f=x=>x===true?1:2;return f(true);}",
        "export function run(a,b){const f=x=>x===false?1:2;return f(false);}",
        "export function run(a,b){return (x=>+x)(a?null:false);}",
        "export function run(a,b){const f=x=>+x;const alias=f;return alias(a<b);}",
        "export function run(a,b){const f=x=>+x;return f(a&&null);}",
        "export function run(a,b){const f=x=>+x;return f(a?undefined:b);}",
        "export function run(a,b){const undefined=a;const f=x=>+x;return f(undefined);}",
        "export function run(a,b){let x=null;const f=(x,y)=>+x+(+y);return f(x,x=a)+x;}",
        "export function run(a,b){let x=true;const f=(x,y)=>+x+(+y);return f(x,x=false)+(+x);}",
        "export function run(a,b){const f=x=>{const read=()=>+x;x=a;return read();};return f(null);}",
        "export function run(a,b){const f=x=>{const read=()=>+x;const old=read();x=b;return old+read();};return f(true);}",
        "export function run(a,b){const f=(x,y)=>+x+(+y);return f(null,a,b=a)+b;}",
        "export function run(a,b){const f=x=>{x=null;return +x;};return f(a);}",
        "export function run(a,b){const f=x=>-x;return f(null);}",
        "export function run(a,b){const f=x=>+x;return f(a);}",
        "export function run(a,b){const f=x=>x===null;return +f(null);}",
        "export function run(a,b){const f=x=>!x;return +f(undefined);}",
    ]);
}

#[test]
fn local_call_type_arguments_preserve_numeric_and_primitive_execution() {
    for body in [
        "const f=x=>x;return f(a);",
        "let x=a;const f=(y,z)=>y+z;return f(x,x=b)+x;",
        "const f=x=>+x;return f(null);",
        "const make=x=>()=>x;const f=make(a);return f();",
        "const f=()=>a,g=()=>b;const selected=a?f:g;return selected();",
    ] {
        let plain = format!("export function run(a,b){{{body}}}");
        let typed = plain
            .replace("f(", "f<Missing>(")
            .replace("make(", "make<never>(")
            .replace("selected(", "selected<typeof absent>(");
        for compiler in [compile, compile_value] {
            assert_eq!(
                compiler(&plain, false).unwrap().wasm,
                compiler(&typed, true).unwrap().wasm,
                "{typed}"
            );
        }
        compare_helper_sources(&[&plain]);
    }
}

#[test]
fn local_primitive_inputs_keep_existing_call_and_result_guards() {
    for source in [
        "export function run(a,b){const f=x=>+x;return f();}",
        "export function run(a,b){const f=x=>+x;return f(...a);}",
        "export function run(a,b){const f=x=>+x;return f('text');}",
        "export function run(a,b){const f=x=>+x;return f({});}",
        "export function run(a,b){const f=x=>+x;return f([]);}",
        "export function run(a,b){const f=x=>+x;return f(null,missing);}",
    ] {
        for ts in [false, true] {
            assert!(compile(source, ts).is_err(), "{source}");
            assert!(compile_value(source, ts).is_err(), "{source}");
        }
    }
    let source = "export function run(a,b){const f=x=>x;return f(null);}";
    for ts in [false, true] {
        assert!(compile(source, ts).is_err(), "{source}");
    }
}

#[test]
fn known_local_primitive_results_convert_only_in_explicit_positions() {
    compare_helper_sources(&[
        "export function run(a,b){const f=x=>x;return +f(null);}",
        "export function run(a,b){const f=x=>x;return +f(undefined);}",
        "export function run(a,b){const f=x=>x;return +f(true);}",
        "export function run(a,b){const f=x=>x;return +f(false);}",
        "export function run(a,b){const f=x=>x;f(null);return +f(a);}",
        "export function run(a,b){const id=x=>x;const leaf=()=>a;const f=id(leaf);return id(a)+f();}",
        "export function run(a,b){const id=x=>x;const leaf=()=>null;const f=id(leaf);return id(a)+(+f());}",
        "export function run(a,b){const id=x=>x;const f=id(()=>null);f();return id(a);}",
    ]);
}

#[test]
fn known_factories_capture_primitive_inputs() {
    compare_helper_sources(&[
        "export function run(a,b){const make=x=>()=>+x;return make(null)();}",
        "export function run(a,b){const make=x=>()=>+x;return make(undefined)();}",
        "export function run(a,b){const make=x=>()=>+x;return make(true)()+make(false)();}",
        "export function run(a,b){const make=x=>()=>x===null?1:2;return make(null)();}",
        "export function run(a,b){const make=x=>()=>x===undefined?1:2;return make(undefined)();}",
        "export function run(a,b){const make=x=>()=>+x;const f=make(a?null:false);return f();}",
        "export function run(a,b){const make=x=>()=>+x;const alias=make;const f=alias(a<b);return f();}",
        "export function run(a,b){let x=null;const make=(x,y)=>()=>+x+(+y);const f=make(x,x=a);return f()+x;}",
        "export function run(a,b){const make=x=>()=>{const old=+x;x=b;return old;};const f=make(true),g=make(null);return f()*100+f()*10+g();}",
        "export function run(a,b){const make=(fn,x)=>()=>fn(+x);return make(y=>y+b,null)();}",
        "export function run(a,b){const make=x=>()=>-x;return make(null)();}",
        "export function run(a,b){const make=x=>()=>+x;return make(a)();}",
        "export function run(a,b){const make=x=>()=>+x;const f=make(null,a=b,undefined);return f()+a;}",
        "export function run(a,b){const make=x=>()=>x===null;return +make(null)();}",
    ]);
}

#[test]
fn runtime_factory_type_arguments_preserve_numeric_and_primitive_execution() {
    for body in [
        "const make=a?x=>()=>x+a:x=>()=>x-b;return make(b)();",
        "let make=x=>()=>++x;const f=make(a),g=make(b);return f()*100+g()*10+f();",
        "let make=x=>y=>()=>x+y;return make(a)(b)();",
        "let make=x=>()=>x+a;const f=make((make=x=>()=>x-b,a));return f()+make(b)();",
        "let make=(x,y)=>()=>x*10+y;return make(a++,a++)()+a;",
    ] {
        let plain = format!("export function run(a,b){{{body}}}");
        let typed = plain.replace("make(", "make<Missing>(");
        for compiler in [compile, compile_value] {
            assert_eq!(
                compiler(&plain, false).unwrap().wasm,
                compiler(&typed, true).unwrap().wasm,
                "{typed}"
            );
        }
        compare_helper_sources(&[&plain]);
    }
}

#[test]
fn known_block_factories_capture_primitive_inputs() {
    compare_helper_sources(&[
        "export function run(a,b){const make=x=>{return ()=>+x;};return make(null)();}",
        "export function run(a,b){const make=x=>{return ()=>+x;};return make(undefined)();}",
        "export function run(a,b){const make=x=>{return ()=>+x;};return make(true)()+make(false)();}",
        "export function run(a,b){const make=x=>{if(a)return ()=>+x;return ()=>-x;};return make(null)();}",
        "export function run(a,b){const make=x=>{const read=()=>+x;return read;};const f=make(true);return f();}",
        "export function run(a,b){let x=null;const make=(x,y)=>{return ()=>+x+(+y);};const f=make(x,x=a);return f()+x;}",
        "export function run(a,b){const make=x=>{return ()=>{const old=+x;x=b;return old;};};const f=make(true),g=make(null);return f()*100+f()*10+g();}",
        "export function run(a,b){const make=(fn,x)=>{return ()=>fn(+x);};return make(y=>y+b,null)();}",
        "export function run(a,b){const make=x=>{return ()=>x===null;};return +make(null)();}",
    ]);
}

fn compare_helper_sources(sources: &[&str]) {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let mut script = String::from(
        "const assert=require('node:assert/strict'),view=new DataView(new ArrayBuffer(8));function bits(n){if(Number.isNaN(n))return 'nan';view.setFloat64(0,n);return view.getBigUint64(0).toString(16).padStart(16,'0');}\n",
    );
    for original in sources {
        for ts in [false, true] {
            let source = if ts {
                original.replace("run(a,b)", "run(a:never,b:Missing)")
            } else {
                original.to_string()
            };
            for general in [false, true] {
                let artifact = if general {
                    compile_value(&source, ts)
                } else {
                    compile(&source, ts)
                }
                .unwrap_or_else(|e| panic!("{source}: {e}"));
                let mut numbers = Vec::new();
                if general {
                    let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
                    for (a, b) in [
                        (4.0, 2.0),
                        (0.0, -0.0),
                        (-0.0, -2.0),
                        (f64::NAN, f64::INFINITY),
                    ] {
                        let Value::Number(n) = instance.run(a, b).unwrap() else {
                            panic!("helper Number result")
                        };
                        numbers.push(n);
                    }
                } else {
                    let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
                    for (a, b) in [
                        (4.0, 2.0),
                        (0.0, -0.0),
                        (-0.0, -2.0),
                        (f64::NAN, f64::INFINITY),
                    ] {
                        numbers.push(instance.run(a, b).unwrap());
                    }
                }
                let rust = numbers
                    .into_iter()
                    .map(|n| {
                        if n.is_nan() {
                            "nan".into()
                        } else {
                            format!("{:016x}", n.to_bits())
                        }
                    })
                    .collect::<Vec<_>>();
                script.push_str(&format!("{{const source={original:?},native=Function(\"'use strict';\"+source.replace('export function','function')+';return run;')(),run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from({:?}))).exports.run,rust={rust:?};let i=0;for(const [a,b]of[[4,2],[0,-0],[-0,-2],[NaN,Infinity]]){{const expected=bits(native(a,b)),result=run(a,b);if({general})assert.equal(result[0],3);assert.equal(bits({general}?result[1]:result),expected,source);assert.equal(rust[i++],expected,source);}}}}\n",artifact.wasm));
            }
        }
    }
    let mut child = Command::new("node")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn bitwise_compounds_convert_primitive_operands() {
    let mut sources = vec![
        "export function run(a,b) {let value=null; value&=a; return a;}".to_string(),
        "export function run(a,b) {let value=null; value<<=a; return a;}".to_string(),
        "export function run(a,b) {a&=true; return a;}".to_string(),
        "export function run(a,b) {a>>>=false; return a;}".to_string(),
        "export function run(a,b) { a&=true; return a; }".to_string(),
        "export function run(a,b) { return a&=true; }".to_string(),
        "export function run(a,b) { return a>>=null; }".to_string(),
        "export function run(a,b) { return a|=(a<b); }".to_string(),
    ];
    for operator in ["&", "|", "^", "<<", ">>", ">>>"] {
        for primitive in ["undefined", "null", "true", "false", "a<b"] {
            for body in [
                format!("return a{operator}=({primitive});"),
                format!("let x={primitive};const r=(x{operator}=b);return x===r?r:NaN;"),
            ] {
                sources.push(format!("export function run(a,b){{{body}}}"));
            }
        }
        for body in [
            format!("let x=null;const r=(x{operator}=(x=a,b));return x===r?r:NaN;"),
            format!("let x=a;const r=(x{operator}=(x=false,b));return x===r?r:NaN;"),
            format!(
                "let x=undefined;const mutate=()=>x=a;const r=(x{operator}=mutate());return x===r?r:NaN;"
            ),
            format!("let x=null;const read=()=>x;x{operator}=b;return +read();"),
            format!("let x=null;x=4294967297.9;return x{operator}=b;"),
            format!("let x=a;return x{operator}=4294967329.5;"),
            format!("a{operator}=null;return a;"),
        ] {
            sources.push(format!("export function run(a,b){{{body}}}"));
        }
    }
    compare_helper_sources(&sources.iter().map(String::as_str).collect::<Vec<_>>());
}

#[test]
fn bitwise_compounds_keep_target_and_unsupported_operand_guards() {
    for operator in ["&", "|", "^", "<<", ">>", ">>>"] {
        for rhs in ["\"3\"", "{}", "1n", "()=>a", "missing", "a**b"] {
            for body in [
                format!("return a{operator}=({rhs});"),
                format!("let x=null;return x{operator}=({rhs});"),
            ] {
                let source = format!("export function run(a,b){{{body}}}");
                for ts in [false, true] {
                    assert!(compile(&source, ts).is_err(), "{source}");
                    assert!(compile_value(&source, ts).is_err(), "{source}");
                }
            }
        }
        for body in [
            format!("const x=null;return x{operator}=a;"),
            format!("return x{operator}=a;let x=null;"),
            format!("return a.x{operator}=null;"),
            format!("let f=()=>a;return f{operator}=null;"),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for ts in [false, true] {
                assert!(compile(&source, ts).is_err(), "{source}");
                assert!(compile_value(&source, ts).is_err(), "{source}");
            }
        }
    }
}

#[test]
fn bitwise_not_converts_primitive_operands() {
    let mut sources = vec!["export function run(a,b) { return ~null; }".to_string()];
    for body in [
        "return ~undefined;",
        "return ~true;",
        "return ~false;",
        "return ~(a<b);",
        "return ~(a?true:false);",
        "return ~(true&&false);",
        "return ~(a,null);",
        "return ~(void(a=b));",
        "return ~(a?null:undefined);",
        "const undefined=a;return ~undefined;",
        "let x=null;return ~x;",
        "let x=undefined;return ~x;",
        "let x=a;return (~x)+(x=false,0);",
        "let x=null;return (~x)+(x=a);",
        "let x=null;x=4294967297.9;return ~x;",
        "let x=null;x=-4294967297.9;return ~x;",
        "let x=a?true:b;return ~x;",
        "let x=null;const f=()=>x=a;return (~x)+f();",
        "const f=()=>null;return ~f();",
    ] {
        sources.push(format!("export function run(a,b){{{body}}}"));
    }
    compare_helper_sources(&sources.iter().map(String::as_str).collect::<Vec<_>>());
}

#[test]
fn bitwise_not_keeps_unsupported_operand_and_binding_guards() {
    for body in [
        "return ~\"3\";",
        "return ~({});",
        "return ~1n;",
        "return ~(()=>a);",
        "const f=()=>a;return ~f;",
        "return ~missing;",
        "return ~(a**b);",
        "const x=null;return ~(x=a);",
        "return ~x;let x=null;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for ts in [false, true] {
            assert!(compile(&source, ts).is_err(), "{source}");
            assert!(compile_value(&source, ts).is_err(), "{source}");
        }
    }
}

#[test]
fn binary_bitwise_converts_primitive_operands() {
    let mut sources = vec![
        "export function run(a,b) { return a&true; }".to_string(),
        "export function run(a,b) { return true<<a; }".to_string(),
        "export function run(a,b) { const flag=true; return flag|a; }".to_string(),
        "export function run(a,b) { const flag=true; return flag<<a; }".to_string(),
    ];
    for operator in ["&", "|", "^", "<<", ">>", ">>>"] {
        for primitive in ["undefined", "null", "true", "false", "a<b"] {
            for body in [
                format!("return ({primitive}){operator}a;"),
                format!("return a{operator}({primitive});"),
            ] {
                sources.push(format!("export function run(a,b){{{body}}}"));
            }
        }
        for body in [
            format!("let x=a;return x{operator}(x=false,b);"),
            format!("let x=null;return x{operator}(x=a,b);"),
            format!("let x=a?null:undefined;return x{operator}b;"),
            format!("const p=a<b;return (a=b){operator}p;"),
            format!("let x=null;const f=()=>x=a;return x{operator}f();"),
            format!("let x=null;x=4294967297.9;return x{operator}b;"),
            format!("let x=null;x=-4294967297.9;return a{operator}x;"),
            format!("let x=a?true:null;return x{operator}(b+32.5);"),
        ] {
            sources.push(format!("export function run(a,b){{{body}}}"));
        }
    }
    compare_helper_sources(&sources.iter().map(String::as_str).collect::<Vec<_>>());
}

#[test]
fn binary_bitwise_primitive_inputs_keep_unsupported_operand_guards() {
    for operator in ["&", "|", "^", "<<", ">>", ">>>"] {
        for operand in ["\"3\"", "{}", "1n", "()=>a"] {
            for body in [
                format!("return ({operand}){operator}a;"),
                format!("return a{operator}({operand});"),
            ] {
                let source = format!("export function run(a,b){{{body}}}");
                for ts in [false, true] {
                    assert!(compile(&source, ts).is_err(), "{source}");
                    assert!(compile_value(&source, ts).is_err(), "{source}");
                }
            }
        }
        for body in [
            format!("return a{operator}missing;"),
            format!("return a{operator}(b**2);"),
            format!("return a{operator}(a+\"3\");"),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for ts in [false, true] {
                assert!(compile(&source, ts).is_err(), "{source}");
                assert!(compile_value(&source, ts).is_err(), "{source}");
            }
        }
    }
}

#[test]
fn unary_math_converts_primitive_first_inputs() {
    let mut sources =
        vec!["export function run(a,b) { const flag=true; return Math.abs(flag); }".to_string()];
    for method in ["abs", "sqrt", "floor", "ceil", "trunc"] {
        for first in ["true", "false", "null", "undefined", "a<b"] {
            sources.push(format!(
                "export function run(a,b) {{ return Math.{method}({first}); }}"
            ));
        }
        for body in [
            format!("return Math.{method}(true,a);"),
            format!("return Math.{method}(null,a);"),
            format!("const undefined=a;return Math.{method}(undefined);"),
            format!("return Math.{method}(void(a=b))+a;"),
            format!("const p=a<b;return Math.{method}(p);"),
            format!("let x=a;return Math.{method}(x,x=false)+(+x);"),
            format!("let x=null;return Math.{method}(x,x=a)+x;"),
            format!("let x=undefined;return Math.{method}(x,x=a)||(+x);"),
            format!("let x=a?undefined:null;return Math.{method}(x);"),
            format!("let x=null;const mutate=()=>x=a;return Math.{method}(x,mutate())+x;"),
            format!("const f=()=>null;return Math.{method}(f());"),
        ] {
            sources.push(format!("export function run(a,b){{{body}}}"));
        }
    }
    compare_helper_sources(&sources.iter().map(String::as_str).collect::<Vec<_>>());
}

#[test]
fn unary_math_defaults_missing_input_to_nan() {
    let mut sources = Vec::new();
    for method in ["abs", "sqrt", "floor", "ceil", "trunc"] {
        sources.push(format!(
            "export function run(a,b) {{ return Math.{method}(); }}"
        ));
        for body in [
            format!("return Math.{method}();"),
            format!("const undefined=a;return Math.{method}();"),
            format!("const NaN=a;return Math.{method}();"),
            format!("return Math.{method}()?a:b;"),
            format!("Math.{method}();return ++a;"),
            format!("let x=a;return Math.{method}(Math.{method}(),x=b)||x;"),
            format!("return a||Math.{method}();"),
        ] {
            sources.push(format!("export function run(a,b){{{body}}}"));
        }
    }
    compare_helper_sources(&sources.iter().map(String::as_str).collect::<Vec<_>>());
}

#[test]
fn unary_math_evaluates_surplus_arguments() {
    let mut sources = vec![
        "export function run(a,b) { return Math.abs(a,b); }".to_string(),
        "export function run(a,b){return Math.abs(a,b);}".to_string(),
    ];
    for method in ["abs", "sqrt", "floor", "ceil", "trunc"] {
        if method != "abs" {
            sources.push(format!(
                "export function run(a,b) {{ return Math.{method}(a,b); }}"
            ));
        }
        for body in [
            format!("let x=a;return Math.{method}(x,x=b)+x;"),
            format!("return Math.{method}(a,(a=b,null),false,undefined)+a;"),
            format!("const make=offset=>(a=offset,()=>offset);return Math.{method}(a,make(b))+a;"),
            format!("return Math.{method}(a,()=>null);"),
            format!("return Math.{method}(a,Math.abs(b,a=b))+a;"),
            format!("const p=a<b;return Math.{method}(a,p);"),
        ] {
            sources.push(format!("export function run(a,b){{{body}}}"));
        }
    }
    compare_helper_sources(&sources.iter().map(String::as_str).collect::<Vec<_>>());
}

#[test]
fn unary_math_surplus_arguments_keep_width_type_and_namespace_guards() {
    let extras = vec!["b"; 999].join(",");
    for method in ["abs", "sqrt", "floor", "ceil", "trunc"] {
        let source = format!("export function run(a,b){{return Math.{method}(a,{extras});}}");
        compare_helper_sources(&[&source]);
        let plain = format!("export function run(a,b){{return Math.{method}();}}");
        let typed = format!("export function run(a,b){{return Math.{method}<number>();}}");
        assert_eq!(
            compile(&typed, true).unwrap().wasm,
            compile(&plain, false).unwrap().wasm,
            "{typed}"
        );
        assert_eq!(
            compile_value(&typed, true).unwrap().wasm,
            compile_value(&plain, false).unwrap().wasm,
            "{typed}"
        );
        compare_helper_sources(&[&plain]);
        for body in [
            format!("return Math.{method}(a,{extras},b);"),
            format!("return Math.{method}(\"3\",a);"),
            format!("return Math.{method}({{}},a);"),
            format!("return Math.{method}(1n,a);"),
            format!("return Math.{method}(()=>a,b);"),
            format!("return Math.{method}(a,...b);"),
            format!("return Math.{method}?.(a,b);"),
            format!("let Math=a;return Math.{method}(a,b);"),
            format!("return Math.{method}(a,missing);"),
            format!("return Math.{method}(a,()=>missing);"),
            format!("return Math.{method}(a,Math.{method}=b);"),
            format!("let Math=a;return Math.{method}();"),
            format!("const n=Math.{method}();let Math=b;return n;"),
            format!("return Math.{method}?.();"),
            format!("return Math[\"{method}\"]();"),
            format!("return (Math.{method})();"),
            format!("if(false)Math.{method}=a;return Math.{method}();"),
        ] {
            let source = format!("export function run(a,b){{{body}}}");
            for ts in [false, true] {
                assert!(compile(&source, ts).is_err(), "{source}");
                assert!(compile_value(&source, ts).is_err(), "{source}");
            }
        }
    }
}

#[test]
fn helper_surplus_arguments_keep_width_input_and_source_guards() {
    let arguments = vec!["a"; 1000].join(",");
    let source = format!(
        "function value(){{return 7;}} export function run(a,b){{return value({arguments});}}"
    );
    compare_helper_sources(&[&source]);
    for source in [
        format!(
            "function value(){{return 7;}} export function run(a,b){{return value({arguments},a);}}"
        ),
        "function identity(x){return x;} export function run(a,b){return identity();}".into(),
        "function identity(x){return x;} export function run(a,b){return identity(null,a);}".into(),
        "function identity(x){return x;} export function run(a,b){return identity(a,...b);}".into(),
        "function identity(x){return x;} export function run(a,b){return identity?.(a,b);}".into(),
        "function identity(x){return x;} export function run(a,b){return identity(a,missing);}"
            .into(),
        "function identity(x){return x;} export function run(a,b){return identity(a,()=>missing);}"
            .into(),
        "function identity(x){return x;} export function run(a,b){return identity(a,a**b);}".into(),
    ] {
        for ts in [false, true] {
            assert!(compile(&source, ts).is_err(), "{source}");
            assert!(compile_value(&source, ts).is_err(), "{source}");
        }
    }
}

#[test]
fn excessive_finite_recursion_traps_and_the_instance_can_run_again() {
    // Deliberately deep but finite. Never replay this through a JS oracle.
    let artifact = compile(
        "function down(n) { if(n<=0) return 0; return down(n-1)+1; } export function run(a,b) { if(a) return down(100000); return b; }",
        false,
    ).unwrap();
    let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
    let error = instance.run(1.0, 0.0).unwrap_err();
    let trap = error.downcast_ref::<Trap>().unwrap();
    assert!(
        matches!(trap, Trap::OutOfFuel | Trap::StackOverflow),
        "{error}"
    );
    println!("observed excessive-recursion protection: {trap:?}");
    assert_eq!(instance.run(0.0, 42.0).unwrap(), 42.0);
}
