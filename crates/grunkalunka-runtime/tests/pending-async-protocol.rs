use grunkalunka_runtime::{AsyncInstance, AsyncState, Value};

// A small scheduler peer isolates the adapter contract from compiler state
// splitting. Compiler IR tests separately execute actual emitted coroutines.
fn peer(first: i32, next: i32, settle_after: i32, initial: i32, trap: bool) -> Vec<u8> {
    wat::parse_str(format!(
        r#"(module
      (global $issued (mut i32) (i32.const {first}))
      (global $steps (mut i32) (i32.const 0))
      (global $payload (mut f64) (f64.const 0))
      (func (export "start") (param f64 f64) (result i32 i32 f64)
        local.get 1 global.set $payload
        i32.const {initial} i32.const 3 global.get $payload)
      (func (export "resume") (param i32 f64) (result i32 i32 f64)
        {trap_body}
        global.get $steps i32.const 1 i32.add global.set $steps
        i32.const {next} global.set $issued
        i32.const 0 i32.const 1 global.get $steps i32.const {settle_after} i32.ge_s select
        i32.const 3 global.get $payload)
      (func (export "take_jobs") (result i32) (local $n i32)
        global.get $issued local.set $n i32.const 0 global.set $issued local.get $n))"#,
        trap_body = if trap { "unreachable" } else { "" }
    ))
    .unwrap()
}

#[test]
fn queued_adapter_keeps_ready_work_after_root_settlement() {
    let bytes = peer(2, 0, 0, 0, false);
    let mut run = AsyncInstance::new(&bytes, 4.0, -0.0).unwrap();
    assert_eq!(run.pending_job_count().unwrap(), 2);
    for remaining in [1, 0] {
        assert!(
            matches!(run.state().unwrap(), AsyncState::Fulfilled(Value::Number(x)) if x.to_bits()==(-0.0f64).to_bits())
        );
        assert!(run.run_one_job().unwrap());
        assert_eq!(run.pending_job_count().unwrap(), remaining);
    }
    assert!(!run.run_one_job().unwrap());
}

#[test]
fn queued_adapter_counts_new_jobs_and_preserves_the_runtime_boundary() {
    let mut run = AsyncInstance::new(&peer(32, 0, 32, 1, false), 4.0, 2.0).unwrap();
    for left in (0..32).rev() {
        assert_eq!(run.pending_job_count().unwrap(), left + 1);
        assert!(run.run_one_job().unwrap());
        assert_eq!(run.pending_job_count().unwrap(), left);
    }
    assert!(matches!(
        run.state().unwrap(),
        AsyncState::Fulfilled(Value::Number(2.0))
    ));
    assert!(!run.run_one_job().unwrap());
    assert!(AsyncInstance::new(&peer(33, 0, 33, 1, false), 0.0, 0.0).is_err());

    let mut run = AsyncInstance::new(&peer(1, 1, 33, 1, false), 0.0, 0.0).unwrap();
    for _ in 0..31 {
        assert!(run.run_one_job().unwrap());
    }
    assert!(
        run.run_one_job()
            .unwrap_err()
            .to_string()
            .contains("32-job budget")
    );
    assert!(run.state().is_err());
    assert!(run.pending_job_count().is_err());
    assert!(run.run_one_job().is_err());
}

#[test]
fn pending_without_ready_jobs_is_distinct_from_settlement_or_a_trap() {
    let mut run = AsyncInstance::new(&peer(0, 0, 1, 1, false), 0.0, 0.0).unwrap();
    assert!(matches!(run.state().unwrap(), AsyncState::Pending));
    assert_eq!(run.pending_job_count().unwrap(), 0);
    assert!(!run.run_one_job().unwrap());

    let mut run = AsyncInstance::new(&peer(1, 0, 1, 1, true), 0.0, 0.0).unwrap();
    assert!(run.run_one_job().is_err());
    assert!(run.state().is_err());
    assert!(run.pending_job_count().is_err());
    for (first, status) in [(-1, 1), (1, 2), (1, 4)] {
        assert!(AsyncInstance::new(&peer(first, 0, 1, status, false), 0.0, 0.0).is_err());
    }
}

#[test]
fn node_queued_adapter_observes_fifo_budget_and_pending_protocol() {
    use std::{
        io::Write,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let adapter = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("js/async.mjs");
    let mut script = format!(
        "const assert=require('node:assert/strict');(async()=>{{const{{createAsyncFunction}}=await import(require('node:url').pathToFileURL({:?}));\n",
        adapter.to_string_lossy()
    );
    for (first, next, after, status, trap, error) in [
        (2, 0, 2, 1, false, ""),
        (32, 0, 32, 1, false, ""),
        (33, 0, 33, 1, false, "32-job budget"),
        (1, 1, 33, 1, false, "32-job budget"),
        (-1, 0, 1, 1, false, "32-job budget"),
        (1, 0, 1, 2, false, "invalid queued"),
        (1, 0, 1, 1, true, "unreachable"),
        (2, 0, 0, 0, false, ""),
    ] {
        let bytes = peer(first, next, after, status, trap);
        script.push_str(&format!(
            "{{const run=createAsyncFunction(Uint8Array.from({bytes:?}));"
        ));
        if error.is_empty() {
            script.push_str("assert(Object.is(await run(4,-0),-0));");
        } else {
            script.push_str(&format!(
                "await assert.rejects(run(4,2),e=>e.message.includes({error:?}));"
            ));
        }
        script.push_str("}\n");
    }
    let bytes = peer(0, 0, 1, 1, false);
    script.push_str(&format!("{{const run=createAsyncFunction(Uint8Array.from({bytes:?}));let settled=false;run(0,0).then(()=>settled=true,()=>settled=true);await Promise.resolve();await Promise.resolve();assert.equal(settled,false);}}"));
    script.push_str(
        "console.log('PROTOCOL_DONE');})().catch(e=>{console.error(e);process.exitCode=1;});",
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
    let deadline = Instant::now() + Duration::from_secs(30);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("Node async protocol test exceeded 30s");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("PROTOCOL_DONE"),
        "Node exited with an unfinished protocol observation"
    );
}
