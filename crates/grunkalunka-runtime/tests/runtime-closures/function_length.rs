use super::*;

fn compare_lengths(bodies: &[&str]) {
    for body in bodies {
        let source = format!("export function run(a,b){{{body}}}");
        compare_length_source(&source);
    }
}

fn compare_length_source(source: &str) {
    let mut script = String::from(
        "const assert=require('node:assert/strict');function bits(x){if(Number.isNaN(x))return 'nan';const v=new DataView(new ArrayBuffer(8));v.setFloat64(0,x,true);return v.getBigUint64(0,true).toString(16).padStart(16,'0');}\n",
    );
    for ts in [false, true] {
        let number = compile(source, ts).unwrap_or_else(|error| panic!("{source}: {error}"));
        let value = compile_value(source, ts).unwrap_or_else(|error| panic!("{source}: {error}"));
        let mut numeric = NumericInstance::new(&number.wasm).unwrap();
        let mut primitive = ValueInstance::new(&value.wasm).unwrap();
        let mut rust_numeric = Vec::new();
        let mut rust_value = Vec::new();
        for (a, b) in [
            (4.0, 2.0),
            (0.0, -0.0),
            (-2.0, -4.0),
            (f64::NAN, f64::INFINITY),
        ] {
            let Value::Number(actual) = primitive.run(a, b).unwrap() else {
                panic!("{source}: wrong length result tag")
            };
            rust_value.push(bits(actual));
            rust_numeric.push(bits(numeric.run(a, b).unwrap()));
        }
        let native = source.replace("export function", "function");
        script.push_str(&format!("{{const native=Function({native:?}+';return run;')();const e=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({:?}))).exports;const v=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({:?}))).exports;const nr={rust_numeric:?},vr={rust_value:?};let i=0;for(const [a,b]of[[4,2],[0,-0],[-2,-4],[NaN,Infinity]]){{const expected=bits(native(a,b));assert.equal(bits(e.run(a,b)),expected);const value=v.run(a,b);assert.equal(value[0],3);assert.equal(bits(value[1]),expected);assert.equal(nr[i],expected);assert.equal(vr[i++],expected);}}}}\n",number.wasm,value.wasm));
    }
    node(&script);
}

#[test]
fn function_length_reads_declared_parameters_without_calling_the_body() {
    compare_lengths(&[
        "return (()=>1).length;",
        "return (x=>x).length;",
        "return ((x,y)=>x+y).length;",
        "return ((x,y,z)=>x+y+z).length;",
        "const f=(x,y)=>x+y;let n=f.length;n+=b;return n;",
    ]);
}

#[test]
fn function_length_reads_module_entry_declarations() {
    compare_lengths(&["return run.length;"]);
    compare_length_source(
        "function helper(x,y,z){return x+y+z;}export function run(a,b){return helper.length+run.length;}",
    );
    compare_length_source(
        "export function run(a,b){return (helper).length;}function helper(x,y,z){return x+y+z;}",
    );
    compare_lengths(&["{const run=x=>x;return run.length;}"]);
}

#[test]
fn function_length_preserves_receivers_captures_and_dormant_bodies() {
    compare_lengths(&[
        "let n=0;const f=(x,y)=>{n++;return x+y;};return f.length*10+n;",
        "let n=0;const f=a?(x,y)=>{n++;return x+y+a;}:(x,y)=>{n+=2;return x-y+b;};const alias=f;return alias.length*10+n;",
        "let n=0;const make=x=>{n++;return(y,z)=>x+y+z;};return make(a).length*10+n;",
        "let n=0;const f=(x,y)=>x+y;return(n=n+1,f).length*10+n;",
        "let f=(x,y)=>x+y;const g=(p,q)=>p-q;const n=(f=g).length;return n+f(a,b);",
        "const id=fn=>fn;return id((x,y,z)=>x+y+z).length;",
        "const f=(fn,x)=>fn(x);return f.length;",
        "const f=(fn,x)=>fn(x);f(y=>y,b);return f.length;",
        "const f=a?x=>+x:x=>+x;f(null);return f.length;",
        "const length=fn=>fn.length;return length((x,y)=>x+y);",
        "const length=fn=>{const alias=fn;return alias.length;};return length((x,y)=>x+y);",
        "const length=fn=>fn.length;const apply=(f,x)=>f(x);return apply(length,(x,y,z)=>x+y+z);",
        "let n=0;const make=x=>(n++,()=>x);make(a).length;return n;",
        "const f=(x,y)=>x+y;return f.length?a:b;",
    ]);
}

#[test]
fn function_length_keeps_erasure_width_and_receiver_guards() {
    let plain = "export function run(a,b){const f=(x,y)=>x+y;return f.length;}";
    let typed = "export function run(a:never,b:Missing):Wrong{const f=<T>(x:never,y:Missing):Wrong=>x+y;return(f as any).length;}";
    for compiler in [compile, compile_value] {
        assert_eq!(
            compiler(plain, false).unwrap().wasm,
            compiler(typed, true).unwrap().wasm
        );
    }
    for count in [0, 1, 17, 32, 1000] {
        let params = (0..count)
            .map(|i| format!("p{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let body = format!("return (({params})=>0).length;");
        compare_lengths(&[&body]);
    }
    let params = (0..1001)
        .map(|i| format!("p{i}"))
        .collect::<Vec<_>>()
        .join(",");
    assert!(
        compile(
            &format!("export function run(a,b){{return (({params})=>0).length;}}"),
            false
        )
        .is_err()
    );
    for depth in [127, 128] {
        let source = format!(
            "export function run(a,b){{return {}run{}.length;}}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        assert_eq!(compile(&source, false).is_ok(), depth == 127, "{depth}");
    }
    for body in [
        "return a.length;",
        "return null.length;",
        "return missing.length;",
        "const f=x=>x;return f.name;",
        "const f=x=>x;return f['length'];",
        "const f=x=>x;return f?.length;",
        "const f=x=>x;f.length=0;return f.length;",
        "const f=x=>x;delete f.length;return f.length;",
        "let f=a?()=>a:x=>x;return f.length;",
        "const f=(...x)=>0;return f.length;",
        "const f=(x=0)=>x;return f.length;",
        "const f=()=>missing;return f.length;",
        "return Promise.resolve(a).length;",
    ] {
        let source = format!("export function run(a,b){{{body}}}");
        for compiler in [compile, compile_value] {
            assert!(compiler(&source, false).is_err(), "{source}");
        }
    }
}
