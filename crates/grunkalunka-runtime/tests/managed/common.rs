use grunkalunka_compiler::compile;
use grunkalunka_runtime::NumericInstance;
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{Condvar, Mutex, OnceLock, mpsc},
    time::Duration,
};

// Libtest uses short-lived test threads. Reuse a bounded pool of Node processes,
// while keeping one fresh VM context per request for isolation.
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
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut request = String::with_capacity(script.len() * 2);
        for &byte in script.as_bytes() {
            request.push(HEX[usize::from(byte >> 4)] as char);
            request.push(HEX[usize::from(byte & 0x0f)] as char);
        }
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

// The mutex only protects checkout/return. Executing an oracle request never
// holds the pool lock, so independent tests can run concurrently.
struct NodeOraclePool {
    state: Mutex<NodeOraclePoolState>,
    available: Condvar,
    limit: usize,
}

#[derive(Default)]
struct NodeOraclePoolState {
    idle: Vec<NodeOracle>,
    live: usize,
}

struct NodeOracleLease<'a> {
    pool: &'a NodeOraclePool,
    oracle: Option<NodeOracle>,
    reusable: bool,
}

impl NodeOraclePool {
    fn new(limit: usize) -> Self {
        assert!(limit > 0);
        Self {
            state: Mutex::new(NodeOraclePoolState::default()),
            available: Condvar::new(),
            limit,
        }
    }

    fn acquire(&self) -> NodeOracleLease<'_> {
        let mut state = self.state.lock().unwrap();
        loop {
            if let Some(oracle) = state.idle.pop() {
                return NodeOracleLease {
                    pool: self,
                    oracle: Some(oracle),
                    reusable: false,
                };
            }
            if state.live < self.limit {
                // Reserve a slot, but spawn Node outside the mutex.
                state.live += 1;
                return NodeOracleLease {
                    pool: self,
                    oracle: None,
                    reusable: false,
                };
            }
            state = self.available.wait(state).unwrap();
        }
    }

    fn evaluate(&self, script: &str, source: &str) -> String {
        let mut lease = self.acquire();
        let output = lease
            .oracle
            .get_or_insert_with(NodeOracle::new)
            .evaluate(script, source);
        // An evaluation that panics discards its worker and frees the slot.
        lease.reusable = true;
        output
    }
}

impl Drop for NodeOracleLease<'_> {
    fn drop(&mut self) {
        let mut state = self.pool.state.lock().unwrap();
        if self.reusable {
            state.idle.push(self.oracle.take().unwrap());
        } else {
            state.live -= 1;
        }
        self.pool.available.notify_one();
    }
}

fn node_oracle_pool() -> &'static NodeOraclePool {
    static POOL: OnceLock<NodeOraclePool> = OnceLock::new();
    POOL.get_or_init(|| {
        let parallelism = std::thread::available_parallelism().map_or(2, |n| n.get());
        NodeOraclePool::new(parallelism.min(4))
    })
}

#[test]
fn node_oracle_pool_reuses_bounded_workers_across_test_threads() {
    use std::{collections::HashSet, sync::Arc};

    let pool = Arc::new(NodeOraclePool::new(2));
    let mut threads = Vec::new();
    for task in 0..6 {
        let pool = Arc::clone(&pool);
        threads.push(std::thread::spawn(move || {
            let mut pids = Vec::new();
            for iteration in 0..4 {
                let script =
                    format!("return require('node:process').pid + ':{task}:{iteration}';");
                let result = pool.evaluate(&script, "oracle pool smoke");
                let (pid, marker) = result.split_once(':').unwrap();
                assert_eq!(marker, format!("{task}:{iteration}"));
                pids.push(pid.to_owned());
            }
            pids
        }));
    }
    let mut pids = HashSet::new();
    for thread in threads {
        for pid in thread.join().unwrap() {
            pids.insert(pid);
        }
    }
    assert!(!pids.is_empty() && pids.len() <= 2);
    let state = pool.state.lock().unwrap();
    assert_eq!(state.live, state.idle.len());
    assert_eq!(state.live, pids.len());
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
        let output = node_oracle_pool().evaluate(&script, source);
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
