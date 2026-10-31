use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};

#[test]
fn null_postfix_increment_returns_converted_old_number_and_stores_one() {
    let artifact = compile(
        "export function run(a,b) {let value=null; const old=value++; return old*10+value;}",
        false,
    )
    .unwrap();
    assert_eq!(
        NumericInstance::new(&artifact.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap(),
        1.0
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
fn primitive_updates_match_numeric_goldens_and_both_engines() {
    let cases = [
        ("null", 0.0),
        ("undefined", f64::NAN),
        ("false", 0.0),
        ("true", 1.0),
        ("0", 0.0),
        ("-0", -0.0),
        ("NaN", f64::NAN),
        ("Infinity", f64::INFINITY),
        ("-Infinity", f64::NEG_INFINITY),
        ("1.5", 1.5),
        ("9007199254740992", 9007199254740992.0),
        ("5e-324", f64::from_bits(1)),
    ];
    let mut records = Vec::new();
    for (initial, old) in cases {
        for increment in [false, true] {
            let next = if increment { old + 1.0 } else { old - 1.0 };
            for prefix in [false, true] {
                let op = if increment { "++" } else { "--" };
                let expression = if prefix {
                    format!("{op}value")
                } else {
                    format!("value{op}")
                };
                for stored in [false, true] {
                    let expected = if stored || prefix { next } else { old };
                    let tail = if stored {
                        format!("{expression}; return +value;")
                    } else {
                        format!("return {expression};")
                    };
                    // Force a General slot even for Number samples.
                    let js = format!(
                        "export function run(a,b) {{let value=null; value={initial}; {tail}}}"
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
                                    other => panic!("expected numeric update, got {other:?}"),
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
    }
    let script = format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}],view=new DataView(new ArrayBuffer(8));
function bits(n){{assert.equal(typeof n,'number');if(Number.isNaN(n))return'nan';view.setFloat64(0,n);return view.getBigUint64(0).toString(16).padStart(16,'0');}}
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();assert.equal(bits(original(4,2)),c.expected,c.source+' standardgolden');const module=new WebAssembly.Module(Uint8Array.from(c.bytes));assert.deepEqual(WebAssembly.Module.imports(module),[]);let actual=new WebAssembly.Instance(module).exports.run(4,2);if(c.value){{assert.equal(actual[0],3);actual=actual[1];}}assert.equal(bits(actual),c.expected,c.source+' Node-Wasm');}}console.log('PASS '+cases.length+' primitive update modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 384 primitive update modules");
}

#[test]
fn updates_preserve_snapshots_order_aliases_and_parameter_cells() {
    let bodies = [
        "let x=null; const old=x++; return old*10+(+x);",
        "let x=false; const old=x--; return old*10+(+x);",
        "let x=true; return ++x+(+x);",
        "let x=undefined; const old=x++; return old+(+x);",
        "let x=null; const saved=x; x++; return saved===null?+x:99;",
        "let x=null; const old=x++; x=false; return old+(x==false?10:0);",
        "let x=null; return x++ + x++;",
        "let x=false; return ++x + --x;",
        "let x=null; for(let i=0;i<3;i++){x++;} return +x;",
        "let x=false; while((+x)<3){x++;} return +x;",
        "let x=null; return a?x++:++x;",
        "a=null; return a++ + (+a);",
        "a=false; const old=--a; return old+(+a);",
        "let x=null; x=a; return x++ + b;",
        "let x=null; const old=(x++,x--); return old*10+(+x);",
        "let x=true; const alias=x; --x; return alias===true?+x:99;",
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
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;for(let i=0;i<vectors.length;i++){{const expected=bits(original(...vectors[i]));assert.equal(c.observed[i],expected,c.source+' Wasmtime');let actual=run(...vectors[i]);if(c.value){{assert.equal(actual[0],3);actual=actual[1];}}assert.equal(bits(actual),expected,c.source+' Node-Wasm');}}}}console.log('PASS '+cases.length+' primitive update effect modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 64 primitive update effect modules");
}

#[test]
fn exact_previous_update_guards_execute_unchanged_sources() {
    let cases = [
        (
            "export function run(a,b) {let value=null; value++; return a;}",
            4.0,
            false,
        ),
        (
            "export function run(a,b) {a=null; a++; return +a;}",
            1.0,
            false,
        ),
        (
            "export function run(a,b) {let value=null; value++; return value;}",
            1.0,
            true,
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
            r#"const assert=require('node:assert/strict'); for(const c of [{}]){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();assert.equal(original(4,2),c.expected);let n=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run(4,2);if(c.value){{assert.equal(n[0],3);n=n[1];}}assert.equal(n,c.expected);}}console.log('PASS exact update guards');"#,
            records.join(",")
        ),
        "PASS exact update guards",
    );
}

#[test]
fn updates_keep_binding_source_kind_and_return_guards() {
    for body in [
        "const value=null; value++; return a;",
        "value++; let value=null; return a;",
        "let value=value++; return a;",
        "clock++; return a;",
        "let value={}; value++; return a;",
        "let value={}; --value; return a;",
        "const o={x:a}; let value=o.x; value++; return a;",
        "const o={x:a}; let value=o.x; --value; return a;",
        "let value=1n; value++; return a;",
        "let value=\"0\"; value++; return a;",
        "let value=null; value.x++; return a;",
        "let value=null; value++; return value;",
        "let value=null; if(false) Math.random(); value++; return a;",
    ] {
        let source = format!("export function run(a,b) {{{body}}}");
        let error = compile(&source, false).err().unwrap();
        assert!(error.contains("unsupported"), "{source}: {error}");
        assert!(!error.contains("internal IR"));
    }
}

#[test]
fn primitive_updates_retain_depth_local_and_statement_budgets() {
    for (depth, admitted) in [(125, true), (129, false)] {
        let source = format!(
            "export function run(a,b) {{let value=null; return {}value++{};}}",
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
        let source = format!("export function run(a,b) {{let {declarations}; x0++; return a;}}");
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
            "value++;".repeat(count)
        );
        let result = compile(&source, false);
        if count == 254 {
            result.unwrap();
        } else {
            assert!(result.err().unwrap().contains("at most 256 statements"));
        }
    }
}
