use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};
#[test]
fn null_compound_addition_returns_and_stores_number() {
    let artifact = compile(
        "export function run(a,b) {let value=null; return (value+=1)+(+value);}",
        false,
    )
    .unwrap();
    assert_eq!(
        NumericInstance::new(&artifact.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap(),
        2.0
    );
}

#[test]
fn number_target_compound_addition_converts_boolean_rhs() {
    let artifact = compile("export function run(a,b) {return a+=true;}", false).unwrap();
    assert_eq!(
        NumericInstance::new(&artifact.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap(),
        5.0
    );
}

fn number_bits(value: f64) -> String {
    if value.is_nan() {
        "nan".into()
    } else {
        format!("{:016x}", value.to_bits())
    }
}
fn node(script: &str, expected: &str) {
    use std::io::Write;
    use std::process::{Command, Stdio};
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
    assert_eq!(String::from_utf8(result.stdout).unwrap().trim(), expected);
}

#[test]
fn arithmetic_compounds_match_independent_four_kind_goldens() {
    let values = [
        ("undefined", f64::NAN),
        ("null", 0.0),
        ("false", 0.0),
        ("true", 1.0),
        ("-0", -0.0),
        ("NaN", f64::NAN),
        ("Infinity", f64::INFINITY),
        ("1.5", 1.5),
    ];
    let mut records = Vec::new();
    for (left, x) in values {
        for (right, y) in values {
            for op in ["+=", "-=", "*=", "/=", "%="] {
                let expected = match op {
                    "+=" => x + y,
                    "-=" => x - y,
                    "*=" => x * y,
                    "/=" => x / y,
                    "%=" => x % y,
                    _ => unreachable!(),
                };
                let js = format!(
                    "export function run(a,b) {{let value=null; value={left}; return value{op}{right};}}"
                );
                for ts in [false, true] {
                    let source = if ts {
                        js.replace("run(a,b)", "run(a:number,b:number)")
                    } else {
                        js.clone()
                    };
                    for value in [false, true] {
                        let artifact = if value {
                            compile_value(&source, ts)
                        } else {
                            compile(&source, ts)
                        }
                        .unwrap();
                        let actual = if value {
                            match ValueInstance::new(&artifact.wasm)
                                .unwrap()
                                .run(4.0, 2.0)
                                .unwrap()
                            {
                                Value::Number(n) => n,
                                other => panic!("expected Number: {other:?}"),
                            }
                        } else {
                            NumericInstance::new(&artifact.wasm)
                                .unwrap()
                                .run(4.0, 2.0)
                                .unwrap()
                        };
                        assert_eq!(number_bits(actual), number_bits(expected), "{source}");
                        records.push(format!(
                            "{{source:{js:?},bytes:{:?},value:{value},expected:{:?}}}",
                            artifact.wasm,
                            number_bits(expected)
                        ));
                    }
                }
            }
        }
    }
    let script = format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}],view=new DataView(new ArrayBuffer(8));
function bits(n){{assert.equal(typeof n,'number');if(Number.isNaN(n))return'nan';view.setFloat64(0,n);return view.getBigUint64(0).toString(16).padStart(16,'0');}}
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();assert.equal(bits(original(4,2)),c.expected,c.source+' standardgolden');const module=new WebAssembly.Module(Uint8Array.from(c.bytes));assert.deepEqual(WebAssembly.Module.imports(module),[]);let actual=new WebAssembly.Instance(module).exports.run(4,2);if(c.value){{assert.equal(actual[0],3);actual=actual[1];}}assert.equal(bits(actual),c.expected,c.source+' Node-Wasm');}}console.log('PASS '+cases.length+' primitive compound modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 1280 primitive compound modules");
}

#[test]
fn compound_assignments_capture_left_before_rhs_effects_and_store_once() {
    let bodies = [
        "let value=null; value=a; const result=(value+=(value=true)); return result*10+(+value);",
        "let value=true; const result=(value-=(value=null)); return result*10+(+value);",
        "a=null; const result=(a+=(a=b)); return result+(+a);",
        "let count=0,value=null; const result=(value+=(count++,a)); return result+count;",
        "let value=false,rhs=null; rhs=b; return value*=rhs;",
        "let value=true; return value/=a;",
        "let value=undefined; return value-=b;",
        "let x=null,y=false; return x+=(y+=true);",
        "let value=null; const saved=(value+=true); value=null; return saved;",
        "let value=null; for(let i=0;i<3;i++){value+=true;} return +value;",
        "let value; if(a)value=a; return value+=b;",
        "return a%=true;",
        "let value=null; return value+=Math.abs(b);",
        "if(a){let value=null;return value+=run(0,b);}return b;",
        "let value=null; value=a; const alias=value; value*=false; return +(alias===null?b:alias);",
        "let value=false; const result=(value%=(value=true)); return result+(+value);",
    ];
    let vectors = [
        (0.0, 0.0),
        (-0.0, -0.0),
        (4.0, 2.0),
        (0.0, -1.0),
        (1.0, -0.0),
        (f64::NAN, 1.0),
        (0.0, f64::NAN),
        (f64::INFINITY, f64::NEG_INFINITY),
    ];
    let mut records = Vec::new();
    for body in bodies {
        let js = format!("export function run(a,b) {{{body}}}");
        for ts in [false, true] {
            let source = if ts {
                js.replace("run(a,b)", "run(a:number,b:number)")
            } else {
                js.clone()
            };
            for value in [false, true] {
                let artifact = if value {
                    compile_value(&source, ts)
                } else {
                    compile(&source, ts)
                }
                .unwrap();
                let mut observed = Vec::new();
                for (a, b) in vectors {
                    let n = if value {
                        match ValueInstance::new(&artifact.wasm)
                            .unwrap()
                            .run(a, b)
                            .unwrap()
                        {
                            Value::Number(n) => n,
                            other => panic!("expected Number: {other:?}"),
                        }
                    } else {
                        NumericInstance::new(&artifact.wasm)
                            .unwrap()
                            .run(a, b)
                            .unwrap()
                    };
                    observed.push(format!("{:?}", number_bits(n)));
                }
                records.push(format!(
                    "{{source:{js:?},bytes:{:?},value:{value},observed:[{}]}}",
                    artifact.wasm,
                    observed.join(",")
                ));
            }
        }
    }
    let script = format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}],vectors=[[0,0],[-0,-0],[4,2],[0,-1],[1,-0],[NaN,1],[0,NaN],[Infinity,-Infinity]],view=new DataView(new ArrayBuffer(8));
function bits(n){{assert.equal(typeof n,'number');if(Number.isNaN(n))return'nan';view.setFloat64(0,n);return view.getBigUint64(0).toString(16).padStart(16,'0');}}
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;for(let i=0;i<vectors.length;i++){{const expected=bits(original(...vectors[i]));assert.equal(c.observed[i],expected,c.source+' Wasmtime');let actual=run(...vectors[i]);if(c.value){{assert.equal(actual[0],3);actual=actual[1];}}assert.equal(bits(actual),expected,c.source+' Node-Wasm');}}}}console.log('PASS '+cases.length+' primitive compound effect modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 64 primitive compound effect modules");
}

#[test]
fn exact_previous_compound_guards_execute_original_sources() {
    let cases = [
        (
            "export function run(a,b) {let value=null; value&=a; return a;}",
            4.0,
            false,
        ),
        (
            "export function run(a,b) {let value=null; value<<=a; return a;}",
            4.0,
            false,
        ),
        ("export function run(a,b) {a&=true; return a;}", 0.0, false),
        (
            "export function run(a,b) {a>>>=false; return a;}",
            4.0,
            false,
        ),
        (
            "export function run(a,b) { const flag=true; a+=flag; return a; }",
            5.0,
            false,
        ),
        (
            "export function run(a,b) { a+=true; return a; }",
            5.0,
            false,
        ),
        ("export function run(a,b) {a+=true; return a;}", 5.0, false),
        (
            "export function run(a,b) {let value=null; value+=a; return a;}",
            4.0,
            false,
        ),
        (
            "export function run(a,b) {a=null; a+=1; return +a;}",
            1.0,
            false,
        ),
        (
            "export function run(a,b) {let value=null; value+=1; return value;}",
            1.0,
            true,
        ),
        (
            "export function run(a,b) {let x; x+=a; return a;}",
            4.0,
            false,
        ),
    ];
    let mut records = Vec::new();
    for (source, expected, value_only) in cases {
        if value_only {
            assert!(compile(source, false).is_err());
        }
        for ts in [false, true] {
            for value in [false, true] {
                if value_only && !value {
                    continue;
                }
                let artifact = if value {
                    compile_value(source, ts)
                } else {
                    compile(source, ts)
                }
                .unwrap();
                let actual = if value {
                    match ValueInstance::new(&artifact.wasm)
                        .unwrap()
                        .run(4.0, 2.0)
                        .unwrap()
                    {
                        Value::Number(n) => n,
                        other => panic!("{other:?}"),
                    }
                } else {
                    NumericInstance::new(&artifact.wasm)
                        .unwrap()
                        .run(4.0, 2.0)
                        .unwrap()
                };
                assert_eq!(actual, expected);
                records.push(format!(
                    "{{source:{source:?},bytes:{:?},value:{value},expected:{expected}}}",
                    artifact.wasm
                ));
            }
        }
    }
    node(
        &format!(
            r#"const assert=require('node:assert/strict'); for(const c of [{}]){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();assert.equal(original(4,2),c.expected);let n=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run(4,2);if(c.value){{assert.equal(n[0],3);n=n[1];}}assert.equal(n,c.expected);}}console.log('PASS 42 exact compound guard modules');"#,
            records.join(",")
        ),
        "PASS 42 exact compound guard modules",
    );
}

#[test]
fn arithmetic_compounds_keep_other_operator_kind_and_binding_boundaries() {
    for body in [
        "let value=null; value**=a; return a;",
        "const value=null; value+=a; return a;",
        "value+=a; let value=null; return a;",
        "let value=(value+=a); return a;",
        "let value=null; value.x+=a; return a;",
        "let value=null; value+=\"x\"; return a;",
        "let value=null; value+=1n; return a;",
        "let value=null; value+=({}); return a;",
        "a+=(\"x\" as number); return a;",
        "if(false){a+=Math.random();} return a;",
    ] {
        let source = format!("export function run(a,b) {{{body}}}");
        let error = compile(&source, true).err().unwrap();
        assert!(!error.contains("internal IR"), "{source}: {error}");
    }
}

#[test]
fn primitive_compounds_retain_depth_local_and_statement_budgets() {
    for (depth, admitted) in [(125, true), (129, false)] {
        let source = format!(
            "export function run(a,b) {{let value=null; return {}value+=true{};}}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        let result = compile(&source, false);
        if admitted {
            result.unwrap();
        } else {
            assert!(result.err().unwrap().contains("expression nesting exceeds"));
        }
    }
    for count in [128, 129] {
        let declarations = (0..count)
            .map(|i| format!("x{i}=null"))
            .collect::<Vec<_>>()
            .join(",");
        let source =
            format!("export function run(a,b) {{let {declarations}; x0+=true; return a;}}");
        let result = compile(&source, false);
        if count == 128 {
            result.unwrap();
        } else {
            assert!(result.err().unwrap().contains("at most 128 source locals"));
        }
    }
    for count in [254, 255] {
        let source = format!(
            "export function run(a,b) {{let value=null; {} return a;}}",
            "value+=true;".repeat(count)
        );
        let result = compile(&source, false);
        if count == 254 {
            result.unwrap();
        } else {
            assert!(result.err().unwrap().contains("at most 256 statements"));
        }
    }
}
