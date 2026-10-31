use grunkalunka_compiler::compile;
use grunkalunka_runtime::NumericInstance;

pub fn compare_sources(js: &str, ts: &str) {
    use std::{
        io::Write,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let inputs = [
        0.0,
        -0.0,
        1.0,
        -7.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        f64::from_bits(1),
    ];
    for (source, typed) in [(js, false), (ts, true)] {
        let artifact = compile(source, typed).unwrap_or_else(|e| panic!("{source}: {e}"));
        let script = format!(
            r#"const assert=require('node:assert/strict');
const native=new Function({native:?})();
const run=new WebAssembly.Instance(new WebAssembly.Module(new Uint8Array({wasm:?}))).exports.run;
const inputs=[0,-0,1,-7,Infinity,-Infinity,NaN,Number.MIN_VALUE];
const bits=x=>{{if(Number.isNaN(x))return 'NaN';const v=new DataView(new ArrayBuffer(8));v.setFloat64(0,x);return v.getBigUint64(0).toString(16);}};
for(const a of inputs)for(const b of inputs){{const expected=native(a,b);assert.equal(bits(run(a,b)),bits(expected));console.log(bits(expected));}}"#,
            native = format!("{};return run;", js.replace("export function", "function")),
            wasm = artifact.wasm
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
        let started = Instant::now();
        while child.try_wait().unwrap().is_none() {
            if started.elapsed() > Duration::from_secs(30) {
                child.kill().unwrap();
                let out = child.wait_with_output().unwrap();
                panic!(
                    "managed source Node timeout: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}: {}",
            source,
            String::from_utf8_lossy(&output.stderr)
        );
        let output = String::from_utf8(output.stdout).unwrap();
        let mut expected = output.lines();
        let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
        for a in inputs {
            for b in inputs {
                let actual = instance.run(a, b).unwrap();
                let expected = expected.next().unwrap();
                if expected == "NaN" {
                    assert!(actual.is_nan(), "{source}");
                } else {
                    assert_eq!(
                        actual.to_bits(),
                        u64::from_str_radix(expected, 16).unwrap(),
                        "{source}: {a},{b}"
                    );
                }
            }
        }
        assert!(expected.next().is_none());
    }
}
