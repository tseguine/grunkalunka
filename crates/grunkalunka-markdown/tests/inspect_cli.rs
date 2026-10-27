use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

const BINARY: &str = env!("CARGO_BIN_EXE_grunkalunka-markdown-inspect");
const EMPTY: &str = "headings 0\nlinks 0\nimages 0\ncode-blocks 0\n";

fn stdin(source: &[u8]) -> Output {
    let mut child = Command::new(BINARY)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(source).unwrap();
    child.wait_with_output().unwrap()
}

fn success(output: Output) -> String {
    assert!(output.status.success(), "{:?}", output);
    assert!(output.stderr.is_empty());
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn stdin_reports_original_ranges_and_inert_reference_and_code_metadata() {
    let source = concat!(
        "# 猫\r\n\r\n[Next](next.md \"note\") ![Alt](image.png)\n\n",
        "```sh\nno execution\n```\n"
    );
    let output = success(stdin(source.as_bytes()));
    assert!(output.starts_with("headings 1\nheading level=1 0..7 \"猫\"\nlinks 1\n"));
    assert!(output.contains("link 9..31 \"Next\" -> \"next.md\" title=\"note\"\n"));
    assert!(output.contains("images 1\nimage 32..49 \"Alt\" -> \"image.png\" title=\"\"\n"));
    assert!(output.contains("code-blocks 1\n"));
    assert!(output.contains("info=Some(\"sh\") text-bytes=13\n"));
    assert!(!output.contains("no execution"));
    assert_eq!(success(stdin(b"")), EMPTY);
}

#[test]
fn projected_strings_are_escaped_and_unsafe_destinations_remain_data() {
    let source = "# x\u{1b}[31m\n\n[Do not follow](<javascript:alert(1)>)\n";
    let output = success(stdin(source.as_bytes()));
    assert!(!output.contains('\u{1b}'));
    assert!(output.contains(&format!("{:?}", "x\u{1b}[31m")));
    assert!(output.contains("\"javascript:alert(1)\""));
}

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "grunkalunka-inspect-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn explicit_unicode_file_paths_work_outside_the_repository_without_outputs() {
    let directory = Directory::new();
    let name = "notes 猫 & spaces.md";
    let source = "# File\r\n[Reference][id]\r\n\r\n[id]: ../other.md\r\n";
    fs::write(directory.0.join(name), source).unwrap();
    let output = Command::new(BINARY)
        .current_dir(&directory.0)
        .arg(name)
        .output()
        .unwrap();
    assert_eq!(success(output), success(stdin(source.as_bytes())));
    assert_eq!(fs::read_to_string(directory.0.join(name)).unwrap(), source);
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
}

#[test]
fn input_limit_is_inclusive_and_counts_utf8_bytes_not_characters() {
    let source = "é".repeat(524288);
    assert_eq!(success(stdin(source.as_bytes())), EMPTY);
    let oversized = stdin(format!("{source}x").as_bytes());
    assert!(!oversized.status.success());
    assert!(oversized.stdout.is_empty());
    assert!(String::from_utf8(oversized.stderr).unwrap().contains("1048576"));
    let invalid = stdin(&[0xff]);
    assert!(!invalid.status.success());
    assert!(invalid.stdout.is_empty());
    assert!(String::from_utf8(invalid.stderr).unwrap().contains("UTF-8"));
}

#[test]
fn help_and_argument_or_file_errors_have_distinct_exit_statuses() {
    let help = Command::new(BINARY).arg("--help").output().unwrap();
    assert!(success(help).starts_with("usage:"));
    let directory = Directory::new();
    for args in [vec![], vec!["missing.md"], vec!["one.md", "two.md"]] {
        let output = Command::new(BINARY)
            .current_dir(&directory.0)
            .args(args)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8(output.stderr).unwrap().starts_with("ERROR:"));
    }
}
