use grunkalunka_compiler::compile_value;
use grunkalunka_runtime::{Value, ValueInstance};

#[test]
fn compiled_null_is_a_null_value() {
    let artifact = compile_value("export function run(a,b) { return null; }", false).unwrap();
    let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
    assert!(matches!(instance.run(3.0, -0.0).unwrap(), Value::Null));
}

#[test]
fn compiled_primitives_preserve_kind_and_number_bits() {
    for (expression, expected) in [
        ("undefined", Value::Undefined),
        ("false", Value::Boolean(false)),
        ("true", Value::Boolean(true)),
        ("a", Value::Number(-0.0)),
        ("a+b", Value::Number(7.0)),
        ("a<b", Value::Boolean(true)),
        ("NaN", Value::Number(f64::NAN)),
    ] {
        let source = format!("export function run(a,b) {{ return {expression}; }}");
        let artifact = compile_value(&source, false).unwrap();
        let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
        let actual = instance.run(-0.0, 7.0).unwrap();
        assert_value(&actual, &expected);
    }
}

fn assert_value(actual: &Value, expected: &Value) {
    match (actual, expected) {
        (Value::Undefined, Value::Undefined) | (Value::Null, Value::Null) => (),
        (Value::Boolean(a), Value::Boolean(b)) => assert_eq!(a, b),
        (Value::Number(a), Value::Number(b)) if b.is_nan() => assert!(a.is_nan()),
        (Value::Number(a), Value::Number(b)) => assert_eq!(a.to_bits(), b.to_bits()),
        _ => panic!("expected {expected:?}, got {actual:?}"),
    }
}

#[test]
fn mixed_returns_keep_branches_effects_and_snapshots() {
    for (body, yes, no) in [
        (
            "return a ? null : false;",
            Value::Null,
            Value::Boolean(false),
        ),
        (
            "return a ? b : (b>0 ? true : undefined);",
            Value::Number(7.0),
            Value::Boolean(true),
        ),
        (
            "return (b+=2, a ? b : undefined);",
            Value::Number(9.0),
            Value::Undefined,
        ),
        (
            "let x=b; if(a) { x++; return x<9; } return null;",
            Value::Boolean(true),
            Value::Null,
        ),
        (
            "return a ? (b+(b=2)) : (b=(a+4));",
            Value::Number(9.0),
            Value::Number(4.0),
        ),
    ] {
        let source = format!("export function run(a,b) {{{body}}}");
        let artifact = compile_value(&source, false).unwrap();
        let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
        assert_value(&instance.run(1.0, 7.0).unwrap(), &yes);
        assert_value(&instance.run(0.0, 7.0).unwrap(), &no);
    }
}

#[test]
fn calls_use_the_callees_result_type() {
    for source in [
        "export function run(a,b) { return a ? run(0,b) : null; }",
        "function plus(x,y) {return x+y;} export function run(a,b) {return a ? plus(b,2) : run(1,b);}",
        "function rem(x,y) {return x%y;} export function run(a,b) {return rem(a,b);}",
    ] {
        let artifact = compile_value(source, false).unwrap();
        let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
        let expected = if source.contains("plus") {
            Value::Number(9.0)
        } else if source.contains("rem") {
            Value::Number(1.0)
        } else {
            Value::Null
        };
        assert_value(&instance.run(1.0, 7.0).unwrap(), &expected);
    }
    for source in [
        "export function run(a,b) {return run(0,b)+1;}",
        "function helper(a,b) {return run(a,b);} export function run(a,b) {return helper(a,b);}",
    ] {
        let error = match compile_value(source, false) {
            Ok(_) => panic!("General result used as Number"),
            Err(error) => error,
        };
        assert!(
            error.contains("value-returning calls require a General result position"),
            "{error}"
        );
    }
}

// These are finite primitive-return API cases, separate from the numeric corpus.
fn source_pairs() -> Vec<(String, String)> {
    let mut pairs = [
        "return null;",
        "return undefined;",
        "return false;",
        "return true;",
        "return a;",
        "return b;",
        "return -0;",
        "return NaN;",
        "return Infinity;",
        "return a+b;",
        "return a%b;",
        "return Math.sqrt(a);",
        "return a<b;",
        "return !a;",
        "const flag=a<b; return flag;",
        "const n=a?b:0; return n;",
        "return a?null:false;",
        "return a ? b : (b>0 ? true : undefined);",
        "return (b+=2, a ? b : undefined);",
        "return a ? b+(b=2) : (b=a+4);",
        "let x=0; while(x<2) { if(a) return null; x++; } return undefined;",
        "if(a) return true; else return b;",
        "return a ? run(0,b) : null;",
    ]
    .into_iter()
    .map(|body| {
        (
            format!("export function run(a,b) {{{body}}}"),
            format!("export function run(a:number,b:number):unknown {{{body}}}"),
        )
    })
    .collect::<Vec<_>>();
    pairs.extend([
        ("export function run(undefined,b) {return undefined;}",
         "export function run(undefined:number,b:number):unknown {return undefined;}"),
        ("export function run(a,b) {return a ? null : b;}",
         "export function run<T>(a:number,b:number):unknown {return ((a ? null : b) as unknown)!;}"),
        ("function rem(x,y) {return x%y;} export function run(a,b) {return a ? rem(a,b) : undefined;}",
         "function rem<T>(x:number,y:number):number {return x%y;} export function run(a:number,b:number):unknown {return a ? rem(a,b) : undefined;}"),
        ("export function run(a,b) {return (a+b, a>b);}",
         "type R=unknown; export function run(a:number,b:number):R {return ((a+b, a>b) satisfies R);}"),
    ].map(|(js,ts)|(js.to_owned(),ts.to_owned())));
    pairs
}

fn observation(value: &Value) -> String {
    match value {
        Value::Undefined => "undefined".into(),
        Value::Null => "null".into(),
        Value::Boolean(value) => format!("boolean:{value}"),
        Value::Number(value) if value.is_nan() => "number:nan".into(),
        Value::Number(value) => format!("number:{:016x}", value.to_bits()),
    }
}

#[test]
fn js_and_ts_primitive_returns_match_original_js_in_both_engines() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let inputs = [
        (0.0, 0.0),
        (-0.0, -0.0),
        (1.0, 7.0),
        (-1.0, 2.0),
        (2.0, -3.0),
        (f64::NAN, 1.0),
        (1.0, f64::NAN),
        (f64::INFINITY, 2.0),
        (f64::NEG_INFINITY, f64::INFINITY),
        (f64::MAX, -f64::MAX),
        (f64::from_bits(1), f64::MIN_POSITIVE),
        (-f64::from_bits(1), -0.0),
        (0.5, 0.25),
        (4294967297.0, -3.0),
    ];
    let mut script = String::from(
        r#"
'use strict';
const assert = require('node:assert/strict');
const buffer = new ArrayBuffer(8), view = new DataView(buffer);
function fromBits(hex) {view.setBigUint64(0,BigInt('0x'+hex));return view.getFloat64(0);}
function observe(x) {
  if(x === undefined) return 'undefined';
  if(x === null) return 'null';
  if(typeof x === 'boolean') return 'boolean:'+x;
  assert.equal(typeof x, 'number');
  if(Number.isNaN(x)) return 'number:nan';
  view.setFloat64(0,x);return 'number:'+view.getBigUint64(0).toString(16).padStart(16,'0');
}
function decode([tag,payload]) {
  if(tag === 0 || tag === 1) {assert.ok(Object.is(payload,0));return tag === 0 ? undefined : null;}
  if(tag === 2) {assert.ok(Object.is(payload,0) || payload === 1);return payload === 1;}
  assert.equal(tag,3);return payload;
}
let calls = 0;
const inputs = [
"#,
    );
    for (a, b) in inputs {
        script.push_str(&format!(
            "[fromBits('{:016x}'),fromBits('{:016x}')],\n",
            a.to_bits(),
            b.to_bits()
        ));
    }
    script.push_str("];\nconst cases = [\n");
    let mut programs = 0;
    for (js, ts) in source_pairs() {
        assert!(
            js.is_ascii() && ts.is_ascii(),
            "test script literal encoding"
        );
        for (source, typescript) in [(&js, false), (&ts, true)] {
            let artifact = compile_value(source, typescript)
                .unwrap_or_else(|error| panic!("{source}: {error}"));
            let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
            let expected = inputs
                .iter()
                .map(|&(a, b)| observation(&instance.run(a, b).unwrap()))
                .collect::<Vec<_>>();
            script.push_str(&format!(
                "{{source:{js:?},bytes:{:?},expected:{expected:?}}},\n",
                artifact.wasm
            ));
            programs += 1;
        }
    }
    script.push_str(
        r#"];
for(const {source,bytes,expected} of cases) {
  const original = Function(source.replace('export function','function')+';return run;')();
  const run = new WebAssembly.Instance(new WebAssembly.Module(Uint8Array.from(bytes))).exports.run;
  inputs.forEach(([a,b],i) => {
    const reference = observe(original(a,b));
    assert.equal(expected[i],reference,source+' Wasmtime '+i);
    assert.equal(observe(decode(run(a,b))),reference,source+' Node-Wasm '+i);
    calls++;
  });
}
console.log('PASS '+cases.length+' programs '+calls+' calls per engine');
"#,
    );
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
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        format!(
            "PASS {programs} programs {} calls per engine",
            programs * inputs.len()
        )
    );
}

#[test]
fn value_profile_keeps_scope_and_reports_remaining_gaps() {
    for source in [
        "export function run(a,b) {const x=null; return x;}",
        "export function run(a,b) {let x=undefined; return x;}",
        "export function run(a,b) {return 'string';}",
        "export function run(a,b) {return a ? null : missing;}",
        "export function run(a,b) {return null+1;}",
        "export function run(a,b) {return null===undefined;}",
        "export function run(a,b) {return null||a;}",
        "export function run(a,b) {return;}",
        "export function run(a,b) {if(a) return null;}",
        "export function run(a,b) {return run?.(a,b);}",
        "export function run(a,b) {return run(...a);}",
        "export function run(a,b) {return run(a);}",
        "export function run(a,b) {return Math.random();}",
        "export function run(a,b) {let undefined=1; {return undefined; let undefined=2;}}",
    ] {
        assert!(compile_value(source, false).is_err(), "{source}");
    }
    assert!(
        compile_value(
            "export function run<T>(a:number,b:number) {return run<T>(a,b);}",
            true
        )
        .is_err()
    );
    for depth in [128, 129] {
        let source = format!(
            "export function run(a,b) {{return {}null{};}}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        assert_eq!(compile_value(&source, false).is_ok(), depth == 128);
    }
    // An annotation cannot turn a Number parameter into null or Boolean.
    for expression in ["a as null", "a as boolean"] {
        let source = format!("export function run(a:number,b:number) {{return {expression};}}");
        let artifact = compile_value(&source, true).unwrap();
        let mut instance = ValueInstance::new(&artifact.wasm).unwrap();
        assert_value(&instance.run(-0.0, 0.0).unwrap(), &Value::Number(-0.0));
    }
}
