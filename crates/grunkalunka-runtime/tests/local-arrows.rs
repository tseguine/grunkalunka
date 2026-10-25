use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};
use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn local_arrows_share_cells_and_preserve_call_semantics_in_both_engines() {
    let cases: [(&str, f64); 18] = [
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
    ];
    let mut script = String::from("const assert=require('node:assert/strict');\n");
    for (body, expected) in cases {
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
        "let f=()=>a; return f();",
        "const f=async()=>a; return f();",
        "const f=()=>{return a;}; return f();",
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
    let source = "export function run(a,b){const read=()=>a; a=b; return read();}";
    let artifact = compile_value(source, false).unwrap();
    let mut host = ValueInstance::new(&artifact.wasm).unwrap();
    assert!(matches!(host.run(4.0,2.0).unwrap(), Value::Number(value) if value==2.0));
    assert!(
        matches!(host.run(4.0,-0.0).unwrap(), Value::Number(value) if value.to_bits()==(-0.0f64).to_bits())
    );
}
