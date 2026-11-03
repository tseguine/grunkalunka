//! Mixed While/DoWhile regressions for the shared cursor.
#[cfg(test)]
mod tests {
    #[test]
    fn mixed_nested_while_and_do_while_use_bounded_native_stack() {
        for (depth, admitted) in [(1, true), (16, true), (32, true), (65, false)] {
            let mut body = String::from("break;");
            for level in 0..depth {
                body = if level % 2 == 0 {
                    format!("while(a){body}")
                } else {
                    format!("do {body} while(a);")
                };
            }
            let source = format!("export function run(a,b){{{body}return b;}}");
            for typescript in [false, true] {
                let source = source.clone();
                let result = std::thread::Builder::new()
                    .stack_size(2 * 1024 * 1024)
                    .spawn(move || crate::compile(&source, typescript))
                    .expect("small-stack source loop thread")
                    .join()
                    .expect("source loop lowering must not overflow");
                if admitted {
                    let artifact = result.unwrap_or_else(|error| panic!("{depth}: {error}"));
                    assert!(!artifact.wasm.is_empty());
                } else {
                    let error = result.err().expect("over-depth loop must be rejected");
                    assert!(
                        error.contains("64") || error.contains("128"),
                        "{depth} loops: {error}"
                    );
                }
            }
        }
    }
}
