use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let manifest = PathBuf::from(args.next().ok_or("missing manifest")?);
    let output = PathBuf::from(args.next().ok_or("missing output directory")?);
    fs::create_dir_all(&output)?;
    let mut summary = String::from("id\tprofile\tlanguage\texpected\tactual\n");
    let mut cases = 0usize;
    let mut mismatches = 0usize;
    for line in fs::read_to_string(manifest)?.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 5 {
            return Err(format!("expected five TSV columns: {line:?}").into());
        }
        let [id, profile, language, expected, path] = fields.as_slice() else {
            unreachable!()
        };
        if !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            return Err(format!("invalid case id: {id:?}").into());
        }
        let source = fs::read_to_string(path)?;
        let profile = profile.to_string();
        let typescript = match *language {
            "js" => false,
            "ts" => true,
            _ => return Err(format!("unknown language: {language}").into()),
        };
        let compile_profile = profile.clone();
        let result = std::thread::Builder::new()
            .name(format!("parity-{id}"))
            .stack_size(2 * 1024 * 1024)
            .spawn(move || match compile_profile.as_str() {
                "numeric" => grunkalunka_compiler::compile(&source, typescript),
                "value" => grunkalunka_compiler::compile_value(&source, typescript),
                "async" => grunkalunka_compiler::compile_async(&source, typescript),
                "heap" => grunkalunka_compiler::compile_heap_value(&source, typescript),
                "ui" => grunkalunka_compiler::compile_ui_import(&source, typescript),
                _ => Err(format!("harness: unknown profile {compile_profile}")),
            })?
            .join();
        let stem = output.join(id);
        let actual = match result {
            Ok(Ok(artifact)) => {
                fs::write(stem.with_extension("wat"), artifact.wat)?;
                fs::write(stem.with_extension("wasm"), artifact.wasm)?;
                "ok"
            }
            Ok(Err(error)) => {
                fs::write(stem.with_extension("error"), error)?;
                "error"
            }
            Err(error) => {
                let error = error
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| error.downcast_ref::<&str>().copied())
                    .unwrap_or("non-string panic");
                fs::write(stem.with_extension("panic"), error)?;
                "panic"
            }
        };
        summary.push_str(&format!(
            "{id}\t{profile}\t{language}\t{expected}\t{actual}\n"
        ));
        cases += 1;
        if *expected != "observe" && *expected != actual {
            mismatches += 1;
            eprintln!("unexpected status: {id} expected {expected} got {actual}");
        }
    }
    fs::write(output.join("summary.tsv"), summary)?;
    println!("cases={cases} expectation_mismatches={mismatches}");
    if mismatches != 0 {
        std::process::exit(1);
    }
    Ok(())
}
