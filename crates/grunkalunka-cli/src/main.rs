//! Build and execute the maintained numeric and primitive-return profiles.
mod help;

use grunkalunka_compiler as compiler;
use grunkalunka_runtime::{NumericInstance, Value, ValueInstance};

use std::{
    env,
    error::Error,
    fs::{self, OpenOptions},
    io::{self, BufRead, Write},
    path::Path,
    time::Instant,
};

fn compile_file(
    path: &str,
    value_profile: bool,
    diagnostics: bool,
) -> Result<compiler::Artifact, Box<dyn Error>> {
    let typescript = match Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("ts") => true,
        Some("js") => false,
        _ => return Err("source extension must be .js or .ts".into()),
    };
    if diagnostics {
        eprintln!("DIAG phase=source event=start path={path:?}");
    }
    let source = fs::read_to_string(path).inspect_err(|error| {
        if diagnostics {
            eprintln!("DIAG phase=source event=error kind={:?}", error.kind());
        }
    })?;
    if diagnostics {
        eprintln!("DIAG phase=source event=ok bytes={}", source.len());
        eprintln!(
            "DIAG phase=compile event=start profile={} language={}",
            if value_profile { "value" } else { "numeric" },
            if typescript { "ts" } else { "js" }
        );
    }
    let started = Instant::now();
    let result = if value_profile {
        compiler::compile_value(&source, typescript)
    } else {
        compiler::compile(&source, typescript)
    };
    if diagnostics {
        eprintln!(
            "DIAG phase=compile event={} elapsed_ms={}",
            if result.is_ok() { "ok" } else { "error" },
            started.elapsed().as_millis()
        );
    }
    Ok(result?)
}

fn evaluate(path: &str, value_profile: bool, diagnostics: bool) -> Result<(), Box<dyn Error>> {
    if diagnostics {
        eprintln!("DIAG phase=wasm-read event=start path={path:?}");
    }
    let bytes = fs::read(path).inspect_err(|error| {
        if diagnostics {
            eprintln!("DIAG phase=wasm-read event=error kind={:?}", error.kind());
        }
    })?;
    if diagnostics {
        eprintln!("DIAG phase=wasm-read event=ok bytes={}", bytes.len());
        eprintln!(
            "DIAG phase=instantiate event=start profile={}",
            if value_profile { "value" } else { "numeric" }
        );
    }
    let started = Instant::now();
    if value_profile {
        let instance = ValueInstance::new(&bytes);
        if diagnostics {
            eprintln!(
                "DIAG phase=instantiate event={} elapsed_ms={}",
                if instance.is_ok() { "ok" } else { "error" },
                started.elapsed().as_millis()
            );
        }
        let mut instance = instance?;
        input_pairs(diagnostics, |a, b| {
            let value = instance.run(a, b).map_err(|error| format!("{error:#}"))?;
            match value {
                Value::TypeError { .. } | Value::Promise { .. } => {
                    return Err("the primitive value protocol does not admit async objects".into());
                }
                Value::Undefined => println!("undefined"),
                Value::Null => println!("null"),
                Value::Boolean(value) => println!("boolean:{}", u8::from(value)),
                Value::Number(value) => println!("number:{:016x}", value.to_bits()),
            }
            Ok(())
        })
    } else {
        let instance = NumericInstance::new(&bytes);
        if diagnostics {
            eprintln!(
                "DIAG phase=instantiate event={} elapsed_ms={}",
                if instance.is_ok() { "ok" } else { "error" },
                started.elapsed().as_millis()
            );
        }
        let mut instance = instance?;
        input_pairs(diagnostics, |a, b| {
            let value = instance.run(a, b)?;
            println!("{:016x}", value.to_bits());
            Ok(())
        })
    }
}

fn input_pairs(
    diagnostics: bool,
    mut evaluate: impl FnMut(f64, f64) -> Result<(), Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let mut count = 0;
    for (index, line) in io::stdin().lock().lines().enumerate() {
        let line_number = index + 1;
        if index >= 4096 {
            if diagnostics {
                eprintln!("DIAG phase=input event=error line={line_number} reason=limit");
            }
            return Err("at most 4096 input pairs are admitted".into());
        }
        let line = line.inspect_err(|error| {
            if diagnostics {
                eprintln!("DIAG phase=input event=error line={line_number} reason=read");
            }
        })?;
        let inputs = line.split_whitespace().collect::<Vec<_>>();
        if inputs.len() != 2 || inputs.iter().any(|input| input.len() != 16) {
            if diagnostics {
                eprintln!("DIAG phase=input event=error line={line_number} reason=shape");
            }
            return Err("expected two 16-digit hexadecimal binary64 bit patterns per line".into());
        }
        let a_bits = u64::from_str_radix(inputs[0], 16).inspect_err(|error| {
            if diagnostics {
                eprintln!("DIAG phase=input event=error line={line_number} reason=hex-a");
            }
        })?;
        let b_bits = u64::from_str_radix(inputs[1], 16).inspect_err(|error| {
            if diagnostics {
                eprintln!("DIAG phase=input event=error line={line_number} reason=hex-b");
            }
        })?;
        if let Err(error) = evaluate(f64::from_bits(a_bits), f64::from_bits(b_bits)) {
            if diagnostics {
                eprintln!(
                    "DIAG phase=execute event=error line={line_number} a_bits={a_bits:016x} b_bits={b_bits:016x}"
                );
            }
            return Err(error);
        }
        count += 1;
    }
    if diagnostics {
        eprintln!("DIAG phase=execute event=ok pairs={count}");
    }
    Ok(())
}

fn execute(diagnostics: bool) -> Result<(), Box<dyn Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command] if matches!(command.as_str(), "--help" | "-h") => print!("{}", help::TEXT),
        [command] if matches!(command.as_str(), "--version" | "-V") => {
            println!("grunkalunka {}", env!("CARGO_PKG_VERSION"));
        }
        [command, source, destination] if command == "compile" || command == "compile-value" => {
            // Finish parsing/admission/encoding before opening the destination.
            // create_new also prevents accidental overwrite and stale-output success.
            let artifact = compile_file(source, command == "compile-value", diagnostics)?;
            OpenOptions::new().write(true).create_new(true).open(destination)?.write_all(&artifact.wasm)?;
            println!("COMPILED {} bytes: {source} -> {destination}", artifact.wasm.len());
        }
        [command, source] if command == "wat" || command == "wat-value" => print!("{}", compile_file(source, command == "wat-value", diagnostics)?.wat),
        [command, path] if command == "eval" || command == "eval-value" => evaluate(path, command == "eval-value", diagnostics)?,
        _ if arguments.first().is_some_and(|command| matches!(command.as_str(), "compile-value" | "wat-value" | "eval-value")) =>
            return Err("usage: grunkalunka compile-value SOURCE.{js,ts} OUT.wasm | wat-value SOURCE | eval-value OUT.wasm < input-bits.txt".into()),
        _ => return Err("usage: grunkalunka compile SOURCE.{js,ts} OUT.wasm | wat SOURCE | eval OUT.wasm < input-bits.txt".into()),
    }
    Ok(())
}

fn main() {
    if let Err(error) =
        execute(matches!(env::var("GRUNKALUNKA_DIAGNOSTICS"), Ok(value) if value == "1"))
    {
        eprintln!("ERROR: {error}");
        std::process::exit(1);
    }
}
