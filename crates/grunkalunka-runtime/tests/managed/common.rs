use grunkalunka_compiler::compile;
use grunkalunka_runtime::NumericInstance;
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{mpsc, Mutex, OnceLock},
    time::Duration,
};

// One Node oracle per test binary, not one process per source or TypeScript variant.
// A fresh VM context per request retains the previous process-isolation semantics.
const NODE_WORKER: &str = r#"const vm = require('node:vm');
const {createInterface} = require('node:readline');
createInterface({input: process.stdin, crlfDelay: Infinity}).on('line', hex => {
  try {
    const script = Buffer.from(hex, 'hex').toString('utf8');
    const result = vm.runInNewContext('(function(require){' + script + '\n})', {require})(require);
    process.stdout.write('OK ' + result + '\n');
  } catch (error) {
    process.stdout.write('ERR ' + JSON.stringify(error.stack ?? String(error)) + '\n');
  }
});"#;

struct NodeOracle {
    child: Child,
    stdin: ChildStdin,
    responses: mpsc::Receiver<std::io::Result<String>>,
}

impl NodeOracle {
    fn new() -> Self {
        let mut child = Command::new("node")
            .arg("-e")
            .arg(NODE_WORKER)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sender, responses) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        let _ = sender.send(Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "Node oracle exited",
                        )));
                        break;
                    }
                    Ok(_) => {
                        if sender.send(Ok(line)).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = sender.send(Err(error));
                        break;
                    }
                }
            }
        });
        Self {
            child,
            stdin,
            responses,
        }
    }

    fn evaluate(&mut self, script: &str, source: &str) -> String {
        // Encode the complete script as one line without a new Rust dependency
        // or a size limit on wasm bytes/source contents.
        let request: String = script
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        self.stdin.write_all(request.as_bytes()).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
        let line = match self.responses.recv_timeout(Duration::from_secs(30)) {
            Ok(Ok(line)) => line,
            Ok(Err(error)) => panic!("{source}: Node oracle read failed: {error}"),
            Err(error) => {
                let _ = self.child.kill();
                panic!("{source}: Node oracle timed out or exited: {error}");
            }
        };
        let Some(expected) = line.strip_prefix("OK ") else {
            panic!("{source}: Node oracle failed: {}", line.trim_end());
        };
        expected.trim_end().to_owned()
    }
}

fn node_oracle() -> &'static Mutex<NodeOracle> {
    static ORACLE: OnceLock<Mutex<NodeOracle>> = OnceLock::new();
    ORACLE.get_or_init(|| Mutex::new(NodeOracle::new()))
}

pub fn compare_sources(js: &str, ts: &str) {
    compare_sources_with_fuel(js, ts, None);
}

// Large literal semantic oracles can use a bounded core-engine budget. None
// retains the public NumericInstance path and its production per-call limit.
pub fn compare_sources_with_fuel(js: &str, ts: &str, fuel: Option<u64>) {
    compare_sources_with_fuel_and_goldens(js, ts, fuel, &[]);
}

// Fixed integer regressions supplement the shared edge grid without widening
// the inputs of every managed-source test.
pub fn compare_sources_with_fuel_and_goldens(
    js: &str,
    ts: &str,
    fuel: Option<u64>,
    goldens: &[[i32; 3]],
) {
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
const bytes=new Uint8Array({wasm:?});
const run=new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports.run;
const inputs=[0,-0,1,-7,Infinity,-Infinity,NaN,Number.MIN_VALUE];
const goldens={goldens:?};
const bits=x=>{{if(Number.isNaN(x))return 'NaN';const v=new DataView(new ArrayBuffer(8));v.setFloat64(0,x);return v.getBigUint64(0).toString(16);}};
const expected=[];
for(const a of inputs)for(const b of inputs){{const value=native(a,b);assert.equal(bits(run(a,b)),bits(value));expected.push(bits(value));}}
for(const [a,b,value] of goldens){{assert.equal(bits(native(a,b)),bits(value));assert.equal(bits(run(a,b)),bits(value));}}
return expected.join(',');"#,
            native = format!("{};return run;", js.replace("export function", "function")),
            wasm = artifact.wasm
        );
        let output = node_oracle().lock().unwrap().evaluate(&script, source);
        let mut expected = output.split(',');
        let mut run: Box<dyn FnMut(f64, f64) -> wasmtime::Result<f64>> = if let Some(fuel) = fuel {
            let mut config = wasmtime::Config::new();
            config.consume_fuel(true);
            let engine = wasmtime::Engine::new(&config).unwrap();
            let module = wasmtime::Module::from_binary(&engine, &artifact.wasm).unwrap();
            let mut store = wasmtime::Store::new(&engine, ());
            store.set_fuel(fuel).unwrap();
            let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
            let function = instance
                .get_typed_func::<(f64, f64), f64>(&mut store, "run")
                .unwrap();
            Box::new(move |a, b| {
                store.set_fuel(fuel)?;
                function.call(&mut store, (a, b))
            })
        } else {
            let mut instance = NumericInstance::new(&artifact.wasm).unwrap();
            Box::new(move |a, b| instance.run(a, b))
        };
        for a in inputs {
            for b in inputs {
                let actual = run(a, b).unwrap();
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
        assert!(expected.next().is_none(), "{source}");
        for &[a, b, expected] in goldens {
            let actual = run(f64::from(a), f64::from(b)).unwrap();
            assert_eq!(
                actual.to_bits(),
                f64::from(expected).to_bits(),
                "{source}: {a},{b}"
            );
        }
    }
}
