//! Exercise the actual executable's default stack, independently of test-thread stacks.
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Source(PathBuf);
impl Source {
    fn new(text: &str, extension: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "grunkalunka-depth-{}-{}.{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
            extension
        ));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let source = Self(path);
        file.write_all(text.as_bytes()).unwrap();
        source
    }
}
impl Drop for Source {
    fn drop(&mut self) {
        fs::remove_file(&self.0).unwrap();
    }
}

#[test]
fn admitted_statement_depth_compiles_on_the_cli_default_stack() {
    for depth in [64, 65] {
        for family in ["for", "for-and-expression", "mixed", "while", "if", "block"] {
            let mut body = String::new();
            for index in 0..depth {
                let prefix = match family {
                    "for" | "for-and-expression" => format!("for(let i{index}=0;0;i{index}++)"),
                    "mixed" if index % 2 == 0 => "for(;0;)".into(),
                    "mixed" | "while" => "while(0)".into(),
                    "if" => "if(0)".into(),
                    _ => "{".into(),
                };
                body.push_str(&prefix);
            }
            if family == "for-and-expression" {
                body.push_str(&"0?0:".repeat(128));
                body.push_str("a;");
            } else {
                body.push_str("{}");
            }
            if family == "block" {
                body.push_str(&"}".repeat(depth));
            }
            let source = format!("export function run(a,b){{{body} return a;}}");
            for extension in ["js", "ts"] {
                let source = Source::new(&source, extension);
                for command in ["wat", "wat-value"] {
                    let result = Command::new(env!("CARGO_BIN_EXE_grunkalunka"))
                        .arg(command)
                        .arg(&source.0)
                        .output()
                        .unwrap();
                    let error = String::from_utf8_lossy(&result.stderr);
                    if depth == 64 {
                        assert!(
                            result.status.success(),
                            "{family}/{extension}/{command}: {error}"
                        );
                        assert!(
                            error.is_empty(),
                            "unexpected successful-compile stderr: {error}"
                        );
                        assert!(String::from_utf8_lossy(&result.stdout).starts_with("(module "));
                    } else {
                        assert_eq!(
                            result.status.code(),
                            Some(1),
                            "{family}/{extension}/{command}: {error}"
                        );
                        assert!(error.contains("statement nesting exceeds 64"), "{error}");
                    }
                }
            }
        }
    }
}
