//! Observability stays opt-in and never changes the CLI's stdout transport.
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new(extension: &str, bytes: &[u8]) -> Self {
        let path = std::env::temp_dir().join(format!(
            "grunkalunka-diagnostics-{}-{}.{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
            extension
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .expect("create fixture");
        file.write_all(bytes).expect("write fixture");
        Self(path)
    }

    fn path(&self) -> &str {
        self.0.to_str().expect("UTF-8 temp path")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_file(&self.0).expect("remove fixture");
    }
}

fn invoke(arguments: &[&str], diagnostics: bool, input: &[u8]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_grunkalunka"));
    command
        .args(arguments)
        .env_remove("GRUNKALUNKA_DIAGNOSTICS")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if diagnostics {
        command.env("GRUNKALUNKA_DIAGNOSTICS", "1");
    }
    let mut child = command.spawn().expect("launch CLI");
    child
        .stdin
        .take()
        .expect("CLI stdin")
        .write_all(input)
        .expect("write input");
    child.wait_with_output().expect("wait for CLI")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("UTF-8 stderr")
}

#[test]
fn compile_diagnostics_are_opt_in_and_preserve_wat_output() {
    let source = Fixture::new("js", b"export function run(a,b){ return a+b; }");
    let quiet = invoke(&["wat", source.path()], false, b"");
    assert!(quiet.status.success(), "{quiet:?}");
    assert!(quiet.stderr.is_empty());
    assert!(quiet.stdout.starts_with(b"(module "));

    let traced = invoke(&["wat", source.path()], true, b"");
    assert!(traced.status.success(), "{traced:?}");
    assert_eq!(traced.stdout, quiet.stdout);
    let log = stderr(&traced);
    assert!(log.contains("DIAG phase=source event=start"), "{log}");
    assert!(log.contains("DIAG phase=source event=ok bytes="), "{log}");
    assert!(log.contains("DIAG phase=compile event=start profile=numeric language=js"), "{log}");
    assert!(log.contains("DIAG phase=compile event=ok elapsed_ms="), "{log}");

    let rejected = Fixture::new("js", b"export function run(a,b){ return a.foo; }");
    let failed = invoke(&["wat", rejected.path()], true, b"");
    assert!(!failed.status.success());
    assert!(failed.stdout.is_empty());
    let log = stderr(&failed);
    assert!(log.contains("DIAG phase=compile event=error elapsed_ms="), "{log}");
    assert!(log.contains("ERROR:"), "{log}");
}

#[test]
fn instantiate_and_input_failures_identify_the_phase_without_spamming_values() {
    let bogus_wasm = Fixture::new("wasm", b"not wasm");
    let failed = invoke(&["eval", bogus_wasm.path()], true, b"");
    assert!(!failed.status.success());
    assert!(failed.stdout.is_empty());
    assert!(
        stderr(&failed).contains("DIAG phase=instantiate event=error elapsed_ms="),
        "{failed:?}"
    );

    let artifact = grunkalunka_compiler::compile("export function run(a,b){return a+b;}", false)
        .expect("compile valid numeric fixture");
    let wasm = Fixture::new("wasm", &artifact.wasm);
    let input = b"3ff0000000000000 4000000000000000\n";
    let quiet = invoke(&["eval", wasm.path()], false, input);
    assert!(quiet.status.success(), "{quiet:?}");
    assert!(quiet.stderr.is_empty());
    assert_eq!(quiet.stdout, b"4008000000000000\n");

    let traced = invoke(&["eval", wasm.path()], true, input);
    assert!(traced.status.success(), "{traced:?}");
    assert_eq!(traced.stdout, quiet.stdout);
    let log = stderr(&traced);
    assert!(log.contains("DIAG phase=instantiate event=ok elapsed_ms="), "{log}");
    assert!(log.contains("DIAG phase=execute event=ok pairs=1"), "{log}");
    assert!(!log.contains("a_bits="), "{log}");

    let rejected = invoke(&["eval", wasm.path()], true, b"bad input\n");
    assert!(!rejected.status.success());
    assert!(rejected.stdout.is_empty());
    let log = stderr(&rejected);
    assert!(log.contains("DIAG phase=input event=error line=1 reason=shape"), "{log}");
    assert!(log.contains("ERROR:"), "{log}");
}
