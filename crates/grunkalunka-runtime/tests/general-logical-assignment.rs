use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};
#[test]
fn original_numeric_nullish_assignment_preserves_its_left_number() {
    let source = "export function run(a,b) { return a ??= b; }";
    let artifact = compile(source, false).unwrap();
    assert_eq!(
        NumericInstance::new(&artifact.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap(),
        4.0
    );
}

#[test]
fn nullish_assignment_erased_targets_preserve_types_and_limits() {
    let js = "export function run(a,b){a=null;return a??=false;}";
    let ts = "export function run(a:never,b:Missing):number{a=null;return (a as number)??=(false as unknown as number);}";
    let artifact = compile_value(js, false).unwrap();
    assert_eq!(artifact.wasm, compile_value(ts, true).unwrap().wasm);
    assert!(matches!(
        ValueInstance::new(&artifact.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap(),
        Value::Boolean(false)
    ));
    for depth in [127, 128] {
        for typed in [false, true] {
            let rhs = if typed {
                format!("b{}", "!".repeat(depth))
            } else {
                format!("{}b{}", "(".repeat(depth), ")".repeat(depth))
            };
            let source = format!("export function run(a,b){{return a??={rhs};}}");
            let result = compile(&source, typed);
            if depth == 127 {
                let artifact = result.unwrap();
                assert_eq!(
                    NumericInstance::new(&artifact.wasm)
                        .unwrap()
                        .run(-0.0, 2.0)
                        .unwrap()
                        .to_bits(),
                    (-0.0f64).to_bits()
                );
            } else {
                assert!(result.err().unwrap().contains("nesting"));
            }
        }
    }
}
#[test]
fn general_null_or_assignment_stores_selected_boolean() {
    let artifact = compile_value(
        "export function run(a,b) {let value=null;value ||= true;return value;}",
        false,
    )
    .unwrap();
    assert!(matches!(
        ValueInstance::new(&artifact.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap(),
        Value::Boolean(true)
    ));
}

#[test]
fn number_initialized_cell_can_receive_boolean_through_or_assignment() {
    let artifact = compile_value(
        "export function run(a,b) {let value=0;value ||= true;return value;}",
        false,
    )
    .unwrap();
    assert!(matches!(
        ValueInstance::new(&artifact.wasm)
            .unwrap()
            .run(4.0, 2.0)
            .unwrap(),
        Value::Boolean(true)
    ));
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

fn encode(value: &Value) -> String {
    match value {
        Value::Undefined => "[0]".into(),
        Value::Null => "[1]".into(),
        Value::Boolean(b) => format!("[2,{b}]"),
        Value::Number(n) => format!("[3,{:?}]", number_bits(*n)),
    }
}

fn standard_number(value: &Value) -> f64 {
    match value {
        Value::Undefined => f64::NAN,
        Value::Null => 0.0,
        Value::Boolean(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Number(n) => *n,
    }
}
#[test]
fn logical_assignments_select_and_store_exact_kind_with_independent_goldens() {
    let cases = [
        ("undefined", Value::Undefined, false),
        ("null", Value::Null, false),
        ("false", Value::Boolean(false), false),
        ("true", Value::Boolean(true), true),
        ("0", Value::Number(0.0), false),
        ("-0", Value::Number(-0.0), false),
        ("NaN", Value::Number(f64::NAN), false),
        ("Infinity", Value::Number(f64::INFINITY), true),
        ("-Infinity", Value::Number(f64::NEG_INFINITY), true),
        ("1.5", Value::Number(1.5), true),
    ];
    let mut records = Vec::new();
    for (left, left_value, truthy) in &cases {
        for (right, right_value, _) in &cases {
            for op in ["||=", "&&=", "??="] {
                let take = match op {
                    "&&=" => *truthy,
                    "||=" => !*truthy,
                    _ => matches!(left_value, Value::Null | Value::Undefined),
                };
                let selected = if take { right_value } else { left_value };
                for stored in [false, true] {
                    for ts in [false, true] {
                        for value in [false, true] {
                            let expression = format!("value {op} rhs");
                            let tail = if stored {
                                format!(
                                    "{expression};return {}value;",
                                    if value { "" } else { "+" }
                                )
                            } else {
                                format!("return {}({expression});", if value { "" } else { "+" })
                            };
                            let js = format!(
                                "export function run(a,b) {{let value=null,rhs=null;value={left};rhs={right};{tail}}}"
                            );
                            let source = if ts {
                                js.replace("run(a,b)", "run(a:number,b:number)")
                            } else {
                                js.clone()
                            };
                            let artifact = if value {
                                compile_value(&source, ts)
                            } else {
                                compile(&source, ts)
                            }
                            .unwrap();
                            let expected = if value {
                                selected.clone()
                            } else {
                                Value::Number(standard_number(selected))
                            };
                            let actual = if value {
                                ValueInstance::new(&artifact.wasm)
                                    .unwrap()
                                    .run(4.0, 2.0)
                                    .unwrap()
                            } else {
                                Value::Number(
                                    NumericInstance::new(&artifact.wasm)
                                        .unwrap()
                                        .run(4.0, 2.0)
                                        .unwrap(),
                                )
                            };
                            assert_eq!(encode(&actual), encode(&expected), "{source}");
                            records.push(format!(
                                "{{source:{js:?},bytes:{:?},value:{value},expected:{}}}",
                                artifact.wasm,
                                encode(&expected)
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
function encode(x){{if(x===undefined)return[0];if(x===null)return[1];if(typeof x==='boolean')return[2,x];assert.equal(typeof x,'number');if(Number.isNaN(x))return[3,'nan'];view.setFloat64(0,x);return[3,view.getBigUint64(0).toString(16).padStart(16,'0')];}}
function decode([tag,payload]){{if(tag<2){{assert.ok(Object.is(payload,0));return tag===0?undefined:null;}}if(tag===2){{assert.ok(Object.is(payload,0)||payload===1);return!!payload;}}assert.equal(tag,3);return payload;}}
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();assert.deepEqual(encode(original(4,2)),c.expected,c.source+' standardgolden');const module=new WebAssembly.Module(Uint8Array.from(c.bytes));assert.deepEqual(WebAssembly.Module.imports(module),[]);const result=new WebAssembly.Instance(module).exports.run(4,2);assert.deepEqual(encode(c.value?decode(result):result),c.expected,c.source+' Node-Wasm');}}console.log('PASS '+cases.length+' logical assignment golden modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 2400 logical assignment golden modules");
}

#[test]
fn general_logical_assignments_skip_rhs_and_preserve_selected_effects() {
    let bodies = [
        "let count=0,value=null;const result=(value&&=(count++,true));return count*10+(result===null?1:0);",
        "let count=0,value=true;const result=(value||=(count++,null));return count*10+(result===true?1:0);",
        "let count=0,value=null;const result=(value||=(count++,a));return count*10+(+result)+(+value);",
        "let count=0,value=true;const result=(value&&=(count++,b));return count*10+(+result)+(+value);",
        "let value=true;const result=(value&&=(value=null));return result===null&&value===null?1:0;",
        "let value=false;const result=(value||=(value=a));return +result+(+value);",
        "let value=false;const saved=(value||=true);value=null;return saved===true?1:0;",
        "a&&=true;return a===true?b:+a;",
        "a||=false;return a===false?b:+a;",
        "let value=a;const saved=value;value&&=true;return +saved;",
        "let value;value||=a;return +value;",
        "let value=false,sum=0;for(let i=0;i<2;i++){value||=(sum++,a);}return sum*10+(+value);",
        "if(a){let value=null;value||=run(0,b);return +value;}return b;",
        "let value=null;if(value||=a)return b;return a;",
        "let value=a;return +(value&&=null);",
        "let x=null,y=true;const result=(x||=(y&&=a));return +result+(+x)+(+y);",
        "let x=null;x??=a;return a;",
        "let count=0,value=null;const result=(value??=(count++,a));return count*10+(+result)+(+value);",
        "let value=false;value??=b++;return b;",
        "let value=0;value??=b++;return b;",
        "let value=-0;value??=b++;return b;",
        "let value=NaN;value??=b++;return b;",
        "a=null;a??=b;return +a;",
        "let value=a;value??=null;return +value;",
        "let value=null;const result=(value??=(value=false,b));return +result+(+value);",
        "let value=null;const read=()=>value;const result=(value??=(value=false,b));return +result+(+read());",
        "let x=null,y=undefined;const result=(x??=(y??=b));return +result+(+x)+(+y);",
        "let value;let i=0;while(i++<2){value??=i;}return +value;",
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
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run;for(let i=0;i<vectors.length;i++){{const expected=bits(original(...vectors[i]));assert.equal(c.observed[i],expected,c.source+' Wasmtime');let actual=run(...vectors[i]);if(c.value){{assert.equal(actual[0],3);actual=actual[1];}}assert.equal(bits(actual),expected,c.source+' Node-Wasm');}}}}console.log('PASS '+cases.length+' General logical assignment effect modules');
"#,
        records.join(",")
    );
    node(
        &script,
        "PASS 112 General logical assignment effect modules",
    );
}

#[test]
fn exact_previous_general_logical_guards_execute_original_sources() {
    let cases = [
        (
            "export function run(a,b) {let value=null; value&&=a; return a;}",
            Value::Number(4.0),
            true,
        ),
        (
            "export function run(a,b) {a=null; a&&=b; return +a;}",
            Value::Number(0.0),
            true,
        ),
        (
            "export function run(a,b) {let value=null; value&&=a; return value;}",
            Value::Null,
            false,
        ),
        (
            "export function run(a,b) {let value=null; value||=a; return value;}",
            Value::Number(4.0),
            false,
        ),
        (
            "export function run(a,b) {let x; x&&=a; return a;}",
            Value::Number(4.0),
            true,
        ),
    ];
    let mut records = Vec::new();
    for (source, expected, numeric) in cases {
        if !numeric {
            assert!(compile(source, false).is_err());
        }
        for ts in [false, true] {
            for value in [false, true] {
                if !numeric && !value {
                    continue;
                }
                let artifact = if value {
                    compile_value(source, ts)
                } else {
                    compile(source, ts)
                }
                .unwrap();
                let actual = if value {
                    ValueInstance::new(&artifact.wasm)
                        .unwrap()
                        .run(4.0, 2.0)
                        .unwrap()
                } else {
                    Value::Number(
                        NumericInstance::new(&artifact.wasm)
                            .unwrap()
                            .run(4.0, 2.0)
                            .unwrap(),
                    )
                };
                assert_eq!(encode(&actual), encode(&expected), "{source}");
                records.push(format!(
                    "{{source:{source:?},bytes:{:?},value:{value},expected:{}}}",
                    artifact.wasm,
                    encode(&expected)
                ));
            }
        }
    }
    let script = format!(
        r#"
const assert=require('node:assert/strict'),cases=[{}],view=new DataView(new ArrayBuffer(8));
function encode(x){{if(x===undefined)return[0];if(x===null)return[1];if(typeof x==='boolean')return[2,x];assert.equal(typeof x,'number');if(Number.isNaN(x))return[3,'nan'];view.setFloat64(0,x);return[3,view.getBigUint64(0).toString(16).padStart(16,'0')];}}
function decode([tag,payload]){{if(tag<2){{assert.ok(Object.is(payload,0));return tag===0?undefined:null;}}if(tag===2){{assert.ok(Object.is(payload,0)||payload===1);return!!payload;}}assert.equal(tag,3);return payload;}}
for(const c of cases){{const original=Function('"use strict";'+c.source.replace('export function','function')+';return run;')();assert.deepEqual(encode(original(4,2)),c.expected);const raw=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(c.bytes))).exports.run(4,2);assert.deepEqual(encode(c.value?decode(raw):raw),c.expected);}}console.log('PASS 16 exact General logical guard modules');
"#,
        records.join(",")
    );
    node(&script, "PASS 16 exact General logical guard modules");
}

#[test]
fn general_logical_assignments_keep_unrepresented_and_binding_guards() {
    for source in [
        "export function run(a,b) {const x=0;return x&&=b;}",
        "export function run(a,b) {const x=1;return x||=b;}",
        "export function run(a,b) {let x=(x||=b);return a;}",
        "export function run(a,b) {let x;{x&&=a;let x=null;}return a;}",
        "export function run(a,b) {clock||=b;return a;}",
        "export function run(a,b) {a.x&&=b;return a;}",
        "export function run(a,b) {const x=1;return x??=b;}",
        "export function run(a,b) {a.x??=b;return a;}",
        "export function run(a,b) {clock??=b;return a;}",
        "export function run(a,b) {let x=(x??=b);return a;}",
    ] {
        let error = compile_value(source, false).err().unwrap();
        assert!(!error.contains("internal IR"), "{source}: {error}");
    }
    for (op, start) in [("&&=", "0"), ("||=", "1"), ("??=", "1")] {
        for rhs in ["Math.random()", "\"x\"", "1n"] {
            let source = format!("export function run(a,b) {{let x={start};return x {op} {rhs};}}");
            let error = compile_value(&source, false).err().unwrap();
            assert!(!error.contains("internal IR"), "{source}: {error}");
            assert!(
                !error.contains("primitive value position"),
                "{source}: {error}"
            );
        }
    }
    for op in ["&&=", "||=", "??="] {
        let source = format!("export function run(a,b) {{return a {op} true;}}");
        assert!(
            compile(&source, false)
                .err()
                .unwrap()
                .contains("General logical assignments require a primitive value position")
        );
    }
}

#[test]
fn logical_write_cells_retain_depth_local_statement_and_parameter_budgets() {
    for (depth, admitted) in [(125, true), (129, false)] {
        let source = format!(
            "export function run(a,b) {{let value=null;return +({}value||=true{});}}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        let result = compile(&source, false);
        if admitted {
            result.unwrap();
        } else {
            assert!(result.err().unwrap().contains("nesting exceeds"));
        }
    }
    for count in [128, 129] {
        let declarations = (0..count)
            .map(|i| format!("x{i}=0"))
            .collect::<Vec<_>>()
            .join(",");
        let source =
            format!("export function run(a,b) {{let {declarations};x0||=true;return +x0;}}");
        let result = compile(&source, false);
        if count == 128 {
            assert_eq!(
                NumericInstance::new(&result.unwrap().wasm)
                    .unwrap()
                    .run(4.0, 2.0)
                    .unwrap(),
                1.0
            );
        } else {
            assert!(result.err().unwrap().contains("at most 128 source locals"));
        }
    }
    for count in [254, 255] {
        let source = format!(
            "export function run(a,b) {{let value=null;{}return a;}}",
            "value||=true;".repeat(count)
        );
        let result = compile(&source, false);
        if count == 254 {
            result.unwrap();
        } else {
            assert!(result.err().unwrap().contains("at most 256 statements"));
        }
    }
    for count in [1000, 1001] {
        let names = (0..count).map(|i| format!("p{i:x}")).collect::<Vec<_>>();
        let args = vec!["0"; count].join(",");
        let source = format!(
            "function f({}){{p0||=true;return +p0;}}export function run(a,b){{return f({args});}}",
            names.join(",")
        );
        assert!(source.len() < 16384);
        let result = compile(&source, false);
        if count == 1000 {
            let artifact = result.unwrap();
            assert_eq!(
                NumericInstance::new(&artifact.wasm)
                    .unwrap()
                    .run(4.0, 2.0)
                    .unwrap(),
                1.0
            );
            node(
                &format!(
                    "const assert=require('node:assert/strict');const run=new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from({:?}))).exports.run;assert.equal(run(4,2),1);console.log('PASS 1000 logical parameter cells');",
                    artifact.wasm
                ),
                "PASS 1000 logical parameter cells",
            );
        } else {
            assert!(
                result
                    .err()
                    .unwrap()
                    .contains("at most 1000 function parameters")
            );
        }
    }
}
