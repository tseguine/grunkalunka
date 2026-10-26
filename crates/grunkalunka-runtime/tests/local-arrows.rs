use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};
use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn local_arrows_share_cells_and_preserve_call_semantics_in_both_engines() {
    let cases: &[(&str, f64)] = &[
        (
            "const make=x=>()=>++x;const f=make(a),g=make(b);return f()*100+f()*10+g();",
            563.0,
        ),
        (
            "const make=x=>()=>++x;const f=make(a);const same=f;const g=make(b);return same()*100+f()*10+g();",
            563.0,
        ),
        (
            "const make=(x,y)=>()=>x*10+y+a;const f=make(a++,a++);return f();",
            51.0,
        ),
        (
            "let x=a;const make=y=>()=>x+=y;const f=make(b),g=make(a);f();return g();",
            10.0,
        ),
        (
            "const make=x=>()=>x++;const f=make(a),g=make(a);return f()*100+f()*10+g();",
            454.0,
        ),
        (
            "const make=x=>y=>()=>++x+y;const maker=make(a);const f=maker(b),g=maker(a);return f()*10+g();",
            80.0,
        ),
        (
            "const read=()=>a;const make=x=>read;const f=make(b);a=b;return f();",
            2.0,
        ),
        (
            "let x=null;const make=y=>()=>+(x=y);const f=make(a);return f();",
            4.0,
        ),
        (
            "const make=x=>()=>+(x=true);const f=make(a);return f();",
            1.0,
        ),
        (
            "const flag=a<b;const make=x=>()=>flag?x:b;const f=make(a);return f();",
            2.0,
        ),
        (
            "const make=x=>()=>{if(x)return ++x;return b;};const f=make(a);return f()+f();",
            11.0,
        ),
        (
            "const call=x=>{const make=y=>()=>x+y;const f=make(b);return f();};return call(a);",
            6.0,
        ),
        ("const f=(x=>()=>++x)(a);return f()+f();", 11.0),
        ("const make=x=>()=>++x;const f=make(a);return f()+a;", 9.0),
        ("const make=x=>()=>x;const f=make(-0);return f();", -0.0),
        (
            "const total=x=>{const make=y=>()=>++y;const f=make(x);return f()+f();};return total(a)+total(b);",
            18.0,
        ),
        (
            "let x=0;const make=y=>()=>++y;for(let i=0;i<4;i++){const f=make(i);x+=f()+f();}return x;",
            24.0,
        ),
        ("const flag=a<b;const f=()=>flag?a:b;return f();", 2.0),
        ("const f=()=>+a;a=true;return f();", 1.0),
        (
            "const f=()=>{const unused=()=>a;a=b;return a;};return f()+a;",
            4.0,
        ),
        ("const read=()=>a; a=b; return read();", 2.0),
        (
            "let x=a; const read=()=>x; const add=()=>x+=b; add(); return read();",
            6.0,
        ),
        (
            "let x=a; const read=()=>x; {let x=b; return read()+x;}",
            6.0,
        ),
        (
            "const read=()=>a; const alias=read; a=b; return alias();",
            2.0,
        ),
        ("const pack=(x,y)=>x*10+y+a; return pack(a++,a++);", 51.0),
        ("const next=x=>x++; return next(a)+next(b)+a;", 10.0),
        ("return (x=>x+b)(a);", 6.0),
        ("const f=x=>(y=>x+y+a)(b); return f(a);", 10.0),
        ("const f=x=>a+x; return f(a=b);", 4.0),
        (
            "let x=0; const next=()=>x++; for(let i=0;i<5;i++)next(); return x;",
            5.0,
        ),
        (
            "let x=0; for(let i=0;i<4;i++){const read=()=>i; x+=read();} return x;",
            6.0,
        ),
        (
            "const read=()=>a; if(b)return read(); return read()+1;",
            4.0,
        ),
        ("const read=()=>a; const x=read(); return x+b;", 6.0),
        ("const read=()=>-0; return read();", -0.0),
        ("const f=x=>(x+=b,x); return f(a)+f(a);", 12.0),
        ("const read=()=>a; a=read(); return read();", 4.0),
        (
            "const read=()=>a; const alias=read; let x=alias(); return x+b;",
            6.0,
        ),
        (
            "let x=a; const read=()=>+x; const set=()=>+(x=true); set(); return read();",
            1.0,
        ),
        ("const f=()=>{if(a)return b;return a;};return f()+a;", 6.0),
        ("const f=()=>{a++;return b;};return f()+a;", 7.0),
        (
            "const f=()=>{for(let i=0;i<3;i++){if(i===1)return a;}return b;};return f()+b;",
            6.0,
        ),
        (
            "const f=x=>{const read=()=>x;x+=b;return read();};return f(a)+f(b);",
            10.0,
        ),
        (
            "const f=x=>{const g=y=>{if(y)return x+y;return a;};return g(b);};return f(a)+f(b);",
            10.0,
        ),
        (
            "const f=()=>{let x=0;for(let i=0;i<4;i++){if(i===1)continue;if(i===3)break;x+=i;}return x;};return f()+a;",
            6.0,
        ),
        (
            "const f=()=>{let i=0;while(i<2){a+=b;i++;}return a;};return f()+a;",
            16.0,
        ),
        (
            "const f=x=>{let y=x;{let x=b;y+=x;}return y;};return f(a)+a;",
            10.0,
        ),
        (
            "const f=x=>{const g=()=>{return x+1;};return g()+1;};return f(a)+b;",
            8.0,
        ),
        ("const f=()=>{a=b;return a;};return a;", 4.0),
        ("const f=()=>{a=b;return a;};return f()+a;", 4.0),
        ("return (x=>{if(x)return x+b;return a;})(a)+a;", 10.0),
        ("const f=()=>{'use strict';return a;};return f()+b;", 6.0),
        (
            "let x=0;const f=()=>{for(let i=0;i<2;i++){if(i)break;}return b;};for(let j=0;j<2;j++){x+=f();}return x;",
            4.0,
        ),
        ("const f=()=>{let x=0;x++;return x;};return f()+f()+a;", 6.0),
        (
            "const f=x=>{if(x)return a;return b;};let x=f(a);a=b;return x+f(0);",
            6.0,
        ),
        ("const f=()=>{return -0;};return f();", -0.0),
        (
            "let x=a;const f=()=>{x=true;return +x;};f();return +x;",
            1.0,
        ),
        (
            "const pack=(x,y)=>{return x*10+y+a;};return pack(a++,a++);",
            51.0,
        ),
        (
            "let x=a;const f=()=>{x+=b;return x;};const g=f;g();return f();",
            8.0,
        ),
    ];
    let mut script = String::from("const assert=require('node:assert/strict');\n");
    for &(body, expected) in cases {
        for typescript in [false, true] {
            let parameters = if typescript {
                "a:number,b:number"
            } else {
                "a,b"
            };
            let source = format!("export function run({parameters}){{{body}}}");
            let artifact =
                compile(&source, typescript).unwrap_or_else(|error| panic!("{source}: {error}"));
            let mut host = NumericInstance::new(&artifact.wasm).unwrap();
            for _ in 0..3 {
                let actual = host.run(4.0, 2.0).unwrap();
                assert_eq!(actual.to_bits(), expected.to_bits(), "{source}");
            }
            script.push_str(&format!(
                "{{const run=(a,b)=>{{{body}}};const wasm=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({:?}))).exports;for(let i=0;i<3;i++)assert(Object.is(wasm.run(4,2),run(4,2)));}}\n",
                artifact.wasm
            ));
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
fn unsupported_closure_lifetimes_and_call_shapes_remain_explicit() {
    for body in [
        "const f=()=>a; return f;",
        "const make=x=>()=>++x;return make(a);",
        "const make=x=>()=>++x;let f=make(a);return f();",
        "const make=x=>()=>++x;const f=a?make(a):make(b);return f();",
        "const make=x=>{return ()=>++x;};const f=make(a);return f();",
        "const make=x=>()=>++x;const f=make(a);return f===f;",
        "const make=x=>()=>++x;const f=make();return f();",
        "const make=x=>()=>++x;const f=make(...a);return f();",
        "const make=x=>()=>++x;const f=make?.(a);return f();",
        "const make=x=>()=>true;return a;",
        "const make=x=>()=>later;const f=make(a);const later=b;return f();",
        "let f=()=>a; return f();",
        "const f=async()=>a; return f();",
        "const f=()=>{if(a)return b;}; return f();",
        "const f=()=>{}; return a;",
        "const f=()=>{return true;}; return a;",
        "const f=(x=1)=>x; return f();",
        "const f=(...x)=>a; return f();",
        "const f=x=>x; return f();",
        "const f=x=>x; return f(a,b);",
        "const f=x=>x; return f(...a);",
        "const f=()=>a; return f?.();",
        "const f=()=>f(); return f();",
        "const f=()=>x; const x=a; return f();",
        "const f=()=>true; return f();",
        "const f=()=>Math.random(); return a;",
        "return (()=>a)(); function hidden() {}",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        assert!(compile(&source, false).is_err(), "{source}");
    }
}

#[test]
fn closure_expansion_limits_bound_generated_work_separately_from_source_limits() {
    for (calls, admitted) in [(256, true), (257, false)] {
        let expression = vec!["f()"; calls].join(",");
        let source = format!("export function run(a,b){{const f=()=>a;return ({expression});}}");
        let result = compile(&source, false);
        assert_eq!(result.is_ok(), admitted);
        if !admitted {
            assert!(
                result
                    .err()
                    .unwrap()
                    .contains("256 local arrow call expansions")
            );
        }
    }
    let mut source = String::from("export function run(a,b){const f0=()=>a;");
    for index in 1..46 {
        source.push_str(&format!(
            "const f{index}=()=>a?(a?(a?f{}():b):b):b;",
            index - 1
        ));
    }
    source.push_str("return f45();}");
    let error = compile(&source, false)
        .err()
        .expect("generated region nesting must be bounded");
    assert!(error.contains("128 nested expression regions"), "{error}");
}

#[test]
fn numeric_local_closure_results_keep_the_primitive_host_tag() {
    let source = "export function run(a,b){const make=x=>()=>++x;const f=make(a),g=make(b);return f()*100+f()*10+g();}";
    let artifact = compile_value(source, false).unwrap();
    let mut host = ValueInstance::new(&artifact.wasm).unwrap();
    assert!(matches!(host.run(4.0, 2.0).unwrap(), Value::Number(563.0)));
    let source = "export function run(a,b){const read=()=>a; a=b; return read();}";
    let artifact = compile_value(source, false).unwrap();
    let mut host = ValueInstance::new(&artifact.wasm).unwrap();
    assert!(matches!(host.run(4.0,2.0).unwrap(), Value::Number(value) if value==2.0));
    assert!(
        matches!(host.run(4.0,-0.0).unwrap(), Value::Number(value) if value.to_bits()==(-0.0f64).to_bits())
    );
    let source = "export function run(a,b){const f=()=>{if(a)return b;return a;};return f()+a;}";
    let artifact = compile_value(source, false).unwrap();
    let mut host = ValueInstance::new(&artifact.wasm).unwrap();
    assert!(matches!(host.run(4.0, 2.0).unwrap(), Value::Number(6.0)));
    assert!(
        compile_value(
            "export function run(a,b){const f=()=>{return true;};return f();}",
            false
        )
        .is_err()
    );
}

#[test]
fn block_arrow_source_costs_are_independent_of_specialization() {
    let locals = (0..20).map(|i| format!("let x{i}=a;")).collect::<String>();
    let calls = ["f()"; 8].join(",");
    let source = format!(
        "export function run(a,b){{const f=()=>{{{locals}return x19;}};return ({calls});}}"
    );
    let artifact = compile(&source, false).unwrap();
    assert_eq!(
        NumericInstance::new(&artifact.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap(),
        4.0
    );
    for (count, admitted) in [(126, true), (127, false)] {
        let locals = (0..count)
            .map(|i| format!("let x{i}=a;"))
            .collect::<String>();
        let source = format!(
            "export function run(a,b){{let outer=a;const f=()=>{{{locals}return a;}};return f();}}"
        );
        assert_eq!(compile(&source, false).is_ok(), admitted);
    }
    for called in [false, true] {
        for (count, admitted) in [(253, true), (254, false)] {
            let source = format!(
                "export function run(a,b){{const f=()=>{{{}return a;}};return {};}}",
                "a=a;".repeat(count),
                if called { "f()" } else { "a" }
            );
            assert_eq!(compile(&source, false).is_ok(), admitted, "{source}");
        }
    }
    let locals = (0..126).map(|i| format!("let x{i}=a;")).collect::<String>();
    let source = format!(
        "export function run(a,b){{const f=()=>{{{locals}return a;}};const alias=f;return alias();}}"
    );
    compile(&source, false).expect("alias must not charge the same body twice");
    for (count, admitted) in [(252, true), (253, false)] {
        let source = format!(
            "export function run(a,b){{const f=()=>{{{}return a;}};const alias=f;return alias();}}",
            "a=a;".repeat(count)
        );
        assert_eq!(compile(&source, false).is_ok(), admitted);
    }
    let source = "export function run(a,b){const f=(x:never):string=>{let y:Missing=x;return y+a;};return f(b);}";
    let artifact = compile(source, true).unwrap();
    assert_eq!(
        NumericInstance::new(&artifact.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap(),
        6.0
    );
}

#[test]
fn nested_block_calls_have_an_observed_stack_bound() {
    for (depth, admitted) in [(8, true), (9, false)] {
        let mut source = String::from("export function run(a,b){");
        for i in 0..depth {
            source.push_str(&format!(
                "const f{i}=()=>{{{}return {};{}}};",
                "{".repeat(10),
                if i == 0 {
                    "a".into()
                } else {
                    format!("f{}()", i - 1)
                },
                "}".repeat(10)
            ));
        }
        source.push_str(&format!("return f{}();}}", depth - 1));
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(result.is_ok(), admitted);
        if admitted {
            let artifact = result.unwrap();
            assert_eq!(
                NumericInstance::new(&artifact.wasm)
                    .unwrap()
                    .run(4.0, 2.0)
                    .unwrap(),
                4.0
            );
        } else {
            assert!(result.err().unwrap().contains("8 nested block-arrow calls"));
        }
    }
    let mut source = String::from("export function run(a,b){");
    for i in 0..8 {
        source.push_str(&format!(
            "const f{i}=()=>{{{}return {};{}}};",
            "{".repeat(20),
            if i == 0 {
                "a".into()
            } else {
                format!("f{}()", i - 1)
            },
            "}".repeat(20)
        ));
    }
    source.push_str("return f7();}");
    assert!(
        compile(&source, false)
            .err()
            .unwrap()
            .contains("128 nested regions")
    );
}

#[test]
fn factory_checks_and_specializations_preserve_source_and_stack_budgets() {
    for (wrappers, admitted) in [(63, true), (64, false)] {
        let mut expression = String::from("make(a)");
        for _ in 0..wrappers {
            expression = format!("({expression} as never)");
        }
        let source = format!(
            "export function run(a,b){{const make=x=>()=>x;const f={expression};return f();}}"
        );
        assert_eq!(
            compile(&source, true).is_ok(),
            admitted,
            "{wrappers} erased call wrappers"
        );
    }
    for (wrappers, admitted) in [(127, true), (128, false)] {
        let source = format!(
            "export function run(a,b){{const make=x=>()=>x;const f={}make(a){};return f();}}",
            "(".repeat(wrappers),
            ")".repeat(wrappers)
        );
        assert_eq!(
            compile(&source, false).is_ok(),
            admitted,
            "{wrappers} call wrappers"
        );
    }
    for (depth, admitted) in [(8, true), (9, false)] {
        let arrows = (0..depth).map(|i| format!("x{i}=>")).collect::<String>();
        let mut source = format!("export function run(a,b){{const make={arrows}()=>x0;");
        for i in 0..depth {
            let callee = if i == 0 {
                "make".into()
            } else {
                format!("f{}", i - 1)
            };
            source.push_str(&format!(
                "const f{i}={callee}({});",
                if i == 0 { "a" } else { "b" }
            ));
        }
        source.push_str(&format!("return f{}();}}", depth - 1));
        let result = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || compile(&source, false))
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(result.is_ok(), admitted);
        if admitted {
            assert_eq!(
                NumericInstance::new(&result.unwrap().wasm)
                    .unwrap()
                    .run(4.0, 2.0)
                    .unwrap(),
                4.0
            );
        } else {
            assert!(result.err().unwrap().contains("8 nested factory results"));
        }
    }
    for (calls, admitted) in [(255, true), (256, false)] {
        let source = format!(
            "export function run(a,b){{const make=x=>()=>x;const f=make(a);return({});}}",
            vec!["f()"; calls].join(",")
        );
        let result = compile(&source, false);
        assert_eq!(result.is_ok(), admitted);
        if !admitted {
            assert!(
                result
                    .err()
                    .unwrap()
                    .contains("256 local arrow call expansions")
            );
        }
    }
    for (locals, admitted) in [(126, true), (127, false)] {
        let declarations = (0..locals)
            .map(|i| format!("let v{i}=a;"))
            .collect::<String>();
        let source = format!(
            "export function run(a,b){{const make=x=>()=>{{{declarations}return x;}};const f=make(a);return f()+f();}}"
        );
        assert_eq!(compile(&source, false).is_ok(), admitted);
    }
    for (statements, admitted) in [(252, true), (253, false)] {
        let source = format!(
            "export function run(a,b){{const make=x=>()=>{{{}return x;}};const f=make(a);return f()+f();}}",
            "a=a;".repeat(statements)
        );
        assert_eq!(compile(&source, false).is_ok(), admitted);
    }
    let js = "export function run(a,b){const make=x=>()=>++x;const f=make(a);return f();}";
    let ts = "export function run(a:number,b:number){const make=((x:never):string=>((()=>++x) as unknown as number)) as any;const f=(make as any)(a);return(f as never)();}";
    let plain = compile(js, false).unwrap();
    let typed = compile(ts, true).unwrap();
    assert_eq!(plain.wasm, typed.wasm);
    assert_eq!(
        NumericInstance::new(&typed.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap(),
        5.0
    );
}
