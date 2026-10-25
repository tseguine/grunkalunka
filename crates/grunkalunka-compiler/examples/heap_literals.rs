use std::{error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let directory = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("fixture directory required")?,
    );
    let mut sources = fs::read_dir(&directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    sources.sort();
    for path in sources {
        let Some(language @ ("js" | "ts")) = path.extension().and_then(|value| value.to_str())
        else {
            continue;
        };
        let artifact = grunkalunka_compiler::compile_heap_value(
            &fs::read_to_string(&path)?,
            language == "ts",
        )?;
        fs::write(
            path.with_extension(format!("{language}.wasm")),
            artifact.wasm,
        )?;
        fs::write(path.with_extension(format!("{language}.wat")), artifact.wat)?;
    }
    Ok(())
}
