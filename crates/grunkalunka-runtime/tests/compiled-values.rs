use grunkalunka_compiler::{compile, compile_value};
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};

#[path = "compiled-values/module_helpers.rs"]
mod module_helpers;

#[test]
fn this_parameter_metadata_preserves_primitive_helper_results() {
    compare_primitive_sources(vec![(
        "function f(x){return x?null:undefined;}export function run(a,b){return f(a);}".into(),
        "function f(this:Missing,x){return x?null:undefined;}export function run(this:void,a,b){return f(a);}".into(),
    )]);
}

#[test]
fn reduced_entry_arities_preserve_the_two_number_host_abi() {
    for source in [
        "export function run(){return 4;}",
        "export function run(a) { return a; }",
    ] {
        let wasm = compile(source, false).unwrap().wasm;
        let mut instance = NumericInstance::new(&wasm).unwrap();
        let expected = if source.contains("run()") { 4.0 } else { -3.0 };
        assert_eq!(instance.run(-3.0, 99.0).unwrap(), expected);
    }
    compare_primitive_sources(vec![
        (
            "export function run(a) { return a; }".to_owned(),
            "export function run(a:unknown):unknown { return a; }".to_owned(),
        ),
        (
            "export function run(a) { return a; }".to_owned(),
            "export function run<T>(a:T):T { return a; }".to_owned(),
        ),
        (
            "export function run(){return 4;}".to_owned(),
            "export function run():number{return 4;}".to_owned(),
        ),
    ]);
}

#[test]
fn entry_source_lengths_preserve_the_two_number_host_abi() {
    let pairs = [
        (
            "export function run(){return 4+run.length;}",
            "export function run(this:Missing){return 4+run.length;}",
            4.0,
        ),
        (
            "export function run(a){return a+run.length;}",
            "export function run(this:never,a?:unknown){return a+run.length;}",
            -2.0,
        ),
        (
            "export function run(a,b){return a+b+run.length;}",
            "export function run(this:void,a:number,b:number){return a+b+run.length;}",
            98.0,
        ),
        (
            "export function run(){const f=run;return 4+f.length;}",
            "export function run(this:Missing){const f=run;return 4+f.length;}",
            4.0,
        ),
        (
            "export function run(a){const length=fn=>fn.length;return a+length(run);}",
            "export function run(this:Missing,a){const length=fn=>fn.length;return a+length(run);}",
            -2.0,
        ),
        (
            "export function run(a){let n=0;return(n++,run).length*10+n+a;}",
            "export function run(this:Missing,a){let n=0;return(n++,run).length*10+n+a;}",
            8.0,
        ),
    ];
    for (js, ts, expected) in pairs {
        let plain = compile(js, false).unwrap().wasm;
        let typed = compile(ts, true).unwrap().wasm;
        assert_eq!(plain, typed, "{ts}");
        for wasm in [&plain, &typed] {
            let mut instance = NumericInstance::new(wasm).unwrap();
            assert_eq!(instance.run(-3.0, 99.0).unwrap(), expected, "{js}");
        }
    }
    compare_primitive_sources(
        pairs
            .into_iter()
            .map(|(js, ts, _)| (js.into(), ts.into()))
            .collect(),
    );
}

#[test]
fn computed_pattern_selections_preserve_selected_primitive_tags_and_bits() {
    compare_primitive_sources(
        [
            "const d=Object.create(null);d.k=false;const k=\"k\";let x;({[k]:x}=d);return x;",
            "const d=Object.create(null);d.k=null;const k=\"k\";let x;({[k]:x}=d);return x;",
            "const d=Object.create(null);const k=\"missing\";let x;({[k]:x}=d);return x;",
            "let x;({[a>>>0]:x}=[-0]);return x;",
        ]
        .into_iter()
        .map(|body| {
            (
                format!("export function run(a,b){{{body}}}"),
                format!("export function run(a:number,b:number):unknown{{{body}}}"),
            )
        })
        .collect(),
    );
}

#[test]
fn normal_final_switch_arms_keep_primitive_suffix_values() {
    compare_primitive_sources(
        [
            "let x=null;switch(a){case 0:x=false;}return x;",
            "let x=undefined;switch(a){case 0:return a;default:x=true;}return x;",
        ]
        .into_iter()
        .map(|body| {
            (
                format!("export function run(a,b){{{body}}}"),
                format!("export function run(a:number,b:number):unknown{{{body}}}"),
            )
        })
        .collect(),
    );
}

#[test]
fn nonexhaustive_switch_keeps_primitive_suffix_tags() {
    compare_primitive_sources(
        [
            "switch(a){case 0:return false;}return null;",
            "switch(a){case b:return a;}return undefined;",
        ]
        .into_iter()
        .map(|body| {
            (
                format!("export function run(a,b){{{body}}}"),
                format!("export function run(a:number,b:number):unknown{{{body}}}"),
            )
        })
        .collect(),
    );
}

#[test]
fn returning_switch_arms_preserve_primitive_result_kinds() {
    compare_primitive_sources(
        [
            "switch(a){case 0:return false;case 1:return null;default:return b;}",
            "switch(a){default:return undefined;case b:return a;}",
        ]
        .into_iter()
        .map(|body| {
            (
                format!("export function run(a,b){{{body}}}"),
                format!("export function run(a:number,b:number):unknown{{{body}}}"),
            )
        })
        .collect(),
    );
}

#[test]
fn runtime_dictionary_reads_return_exact_primitive_tags() {
    compare_primitive_sources(
        [
            "const d=Object.create(null);d.x=a;const k='x';return d[k];",
            "const d=Object.create(null);d.x=a?null:b;const k='x';return d[k];",
            "const d=Object.create(null);d.x=false;const k='x';return d[k];",
            "const d=Object.create(null);const k='missing';return d[k];",
        ]
        .into_iter()
        .map(|body| {
            (
                format!("export function run(a,b){{{body}}}"),
                format!("export function run(a:number,b:number):unknown{{{body}}}"),
            )
        })
        .collect(),
    );
}

#[test]
fn boxed_primitive_results_preserve_language_tags_and_number_bits() {
    compare_primitive_sources(
        [
            "const o={v:null};return o.v;",
            "const o={v:undefined};return o.v;",
            "const o={v:true};return o.v;",
            "const o={v:a?false:b};return o.v;",
            "const o={v:a?null:b};return o.v;",
            "const o={v:a?undefined:b};return o.v;",
            "const read=x=>x.v;const o={v:a?true:b};return read(o);",
            "const o={v:a?null:b};return o.v==(o.v=false,b);",
        ]
        .into_iter()
        .map(|body| {
            let js = format!("export function run(a,b){{{body}}}");
            let ts = format!("export function run(a:number,b:number):unknown{{{body}}}");
            assert_eq!(
                compile_value(&js, false).unwrap().wasm,
                compile_value(&ts, true).unwrap().wasm
            );
            (js, ts)
        })
        .collect(),
    );
}

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
    {
        let source = "function helper(a,b) {return run(a,b);} export function run(a,b) {return helper(a,b);}";
        let artifact = compile_value(source, false).unwrap();
        let error = ValueInstance::new(&artifact.wasm)
            .unwrap()
            .run(0.0, 0.0)
            .unwrap_err();
        assert!(matches!(
            error.downcast_ref::<wasmtime::Trap>(),
            Some(wasmtime::Trap::OutOfFuel | wasmtime::Trap::StackOverflow)
        ));
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
        grunkalunka_runtime::Value::TypeError { .. }
        | grunkalunka_runtime::Value::Promise { .. } => {
            panic!("primitive-only fixture received an async object")
        }
        Value::Undefined => "undefined".into(),
        Value::Null => "null".into(),
        Value::Boolean(value) => format!("boolean:{value}"),
        Value::Number(value) if value.is_nan() => "number:nan".into(),
        Value::Number(value) => format!("number:{:016x}", value.to_bits()),
    }
}

#[test]
fn js_and_ts_primitive_returns_match_original_js_in_both_engines() {
    compare_primitive_sources(source_pairs());
}

#[test]
fn string_search_returns_boolean_values_in_the_primitive_profile() {
    compare_primitive_sources(
        [
            "export function run(a,b){return 'abc'.startsWith('a',a);}",
            "export function run(a,b){return 'abc'.includes('b',a);}",
            "export function run(a,b){let flag='abc'.includes('a');flag='abc'.startsWith('b');return flag;}",
            "function has(s,n){return s.includes(n);}export function run(a,b){return has('abc','b');}",
            "export function run(a,b){const box={found:'abc'.includes('a')};return box.found===true;}",
            "export function run(a,b){return 'abc'.includes('a')&&'abc'.startsWith('b');}",
        ]
        .map(|js| (js.to_owned(), js.replace("run(a,b)", "run(a:number,b:number)")))
        .into(),
    );
    // Own-property reads retain the existing unknown boxed carrier; the public
    // primitive result profile does not acquire an unchecked Boolean projection.
    let error = compile_value(
        "export function run(a,b){const box={found:'abc'.includes('a')};return box.found;}",
        false,
    )
    .err()
    .expect("boxed properties still require a supported managed consumer");
    assert!(error.contains("unsupported"), "{error}");
    assert!(!error.contains("internal IR"), "{error}");
}

fn compare_primitive_sources(pairs: Vec<(String, String)>) {
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
    for (js, ts) in pairs {
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
fn value_profile_generic_call_types_keep_recursive_resource_limits() {
    let typed = "export function run<T>(a:number,b:number) {return run<T>(a,b);}";
    let plain = "export function run(a,b) {return run(a,b);}";
    let artifact = compile_value(typed, true).unwrap();
    assert_eq!(artifact.wasm, compile_value(plain, false).unwrap().wasm);
    let error = ValueInstance::new(&artifact.wasm)
        .unwrap()
        .run(0.0, 0.0)
        .unwrap_err();
    assert!(matches!(
        error.downcast_ref::<wasmtime::Trap>(),
        Some(wasmtime::Trap::OutOfFuel | wasmtime::Trap::StackOverflow)
    ));
}

#[test]
fn value_profile_keeps_scope_and_reports_remaining_gaps() {
    for source in [
        "export function run(a,b) {return 'string';}",
        "export function run(a,b) {return a ? null : missing;}",
        "export function run(a,b) {return run?.(a,b);}",
        "export function run(a,b) {return run(...a);}",
        "export function run(a,b) {return run(a);}",
        "export function run(a,b) {return Math.random();}",
        "export function run(a,b) {let undefined=1; {return undefined; let undefined=2;}}",
    ] {
        assert!(compile_value(source, false).is_err(), "{source}");
    }
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

#[test]
fn omitted_named_helper_parameters_keep_undefined_result_tags() {
    compare_primitive_sources(
        [
            "function identity(x) { return x; } export function run(a,b) { return identity(); }",
            "function identity(x) { return x; } export function run(a,b) { return identity.call(a); }",
            "function f(x){return x;}export function run(a,b){return f();}",
            "function f(x){return x;}export function run(a,b){if(a)return f();return f(b);}",
            "function f(x,y){return y;}export function run(a,b){return f(a);}",
            "function f(x){x=3;return x;}export function run(a,b){return f();}",
        ]
        .into_iter()
        .map(|source| (source.to_owned(), source.to_owned()))
        .collect(),
    );
}

#[test]
fn omitted_local_arrow_results_keep_exact_primitive_tags() {
    compare_primitive_sources(
        [
            "const f=x=>x;return f();",
            "return (x=>x)();",
            "const f=(x,y)=>y;return f(a);",
            "const f=x=>x;return a?f():f(b);",
            "const f=(x,y)=>y??x;return f(false);",
            "const f=(x,y)=>y??x;return f(null);",
        ]
        .into_iter()
        .map(|body| {
            (
                format!("export function run(a,b){{{body}}}"),
                format!("export function run(a:number,b:number):unknown{{{body}}}"),
            )
        })
        .collect(),
    );
}

#[test]
fn omitted_runtime_closure_results_keep_exact_primitive_tags() {
    compare_primitive_sources(
        [
            "let f=x=>x;return f();",
            "const f=a?(x=>x):(x=>x);return f();",
            "const o={f:x=>x};return o.f();",
            "const fs=[x=>x];return fs[0]();",
            "let f=(x,y)=>y??x;return f(false);",
            "let f=(x,y)=>y??x;return f(null);",
        ]
        .into_iter()
        .map(|body| {
            (
                format!("export function run(a,b){{{body}}}"),
                format!("export function run(a:number,b:number):unknown{{{body}}}"),
            )
        })
        .collect(),
    );
}

#[test]
fn factory_created_local_closures_return_exact_primitive_tags() {
    compare_primitive_sources(
        [
            "const f=x=>()=>x;const g=f();return g();",
            "const f=x=>()=>x;const g=f(false);return g();",
            "const f=x=>()=>x;const g=f(null);return g();",
            "const f=x=>()=>x;const g=f(a?null:b);return g();",
            "const f=x=>()=>x;const g=f();const alias=g;return alias();",
        ]
        .into_iter()
        .map(|body| {
            (
                format!("export function run(a,b){{{body}}}"),
                format!("export function run(a:number,b:number):unknown{{{body}}}"),
            )
        })
        .collect(),
    );
}

#[test]
fn former_helper_arity_and_type_guards_preserve_exact_primitive_results() {
    compare_primitive_sources(
        [
            (
                "function id(x){return x;}export function run(a,b){return id();}",
                "function id(x){return x;}export function run(a,b){return id<never>();}",
            ),
            (
                "function identity(x){return x;} export function run(a,b){return identity();}",
                "function identity(x){return x;} export function run(a,b){return identity();}",
            ),
        ]
        .into_iter()
        .map(|(js, ts)| (js.to_owned(), ts.to_owned()))
        .collect(),
    );
}

#[test]
fn omitted_callback_scalar_keeps_the_exact_former_value_guard_source() {
    let source = "export function run(a,b){const f=a?(fn,x)=>fn(x):(fn,x)=>x;return f(y=>y);}";
    compare_primitive_sources(vec![(source.to_owned(), source.to_owned())]);
}

#[test]
fn bare_returns_preserve_undefined_through_control_and_finalizers() {
    compare_primitive_sources([
        "return;",
        "return a;return;",
        "return a;throw b;",
        "if(a)return;return b;",
        "const undefined=b;return;",
        "try{return;}finally{a=b;}",
        "try{if(a)return;return b;}finally{b++;}",
        "try{return b;}finally{if(a)return;}",
        "for(let i=0;i<2;i++){if(a)return;}return b;",
    ].into_iter().map(|body|(
        format!("export function run(a,b) {{{body}}}"),
        format!("export function run(a:number,b:number):unknown {{{body}}}"),
    )).chain(std::iter::once((
        "function f(x){if(x)return null;return;}export function run(a,b){return f(a);}".to_owned(),
        "function f(x:number):unknown{if(x)return null;return;}export function run(a:number,b:number):unknown{return f(a);}".to_owned(),
    ))).collect());
}

#[test]
fn bare_private_helper_keeps_undefined_distinct_from_number_branches() {
    let source = "function f(x,y){if(x)return y;return;}export function run(a,b){return f(a,b);}";
    compare_primitive_sources(vec![(source.to_owned(), source.to_owned())]);
}

#[test]
fn implicit_returns_preserve_undefined_after_loops_catches_and_finalizers() {
    compare_primitive_sources(
        [
            "",
            "a++;",
            "const undefined=b;",
            "if(a) return null;",
            "if(a)return;",
            "{if(a)return b;}",
            "while(a){if(b)break;return null;}",
            "do{if(a)continue;return b;}while(0);",
            "do{if(a)break;return b;}while(0);",
            "switch(a){case 1:return null;default:b++;}",
            "try{if(a)return null;}finally{b++;}",
            "try{a++;}finally{if(b)return b;}",
            "try{throw a;}catch(e){if(b)return e;}",
        ]
        .into_iter()
        .map(|body| {
            (
                format!("export function run(a,b) {{{body}}}"),
                format!("export function run(a:number,b:number):unknown {{{body}}}"),
            )
        })
        .collect(),
    );
}

#[path = "compiled-values/var_bindings.rs"]
mod var_bindings;
