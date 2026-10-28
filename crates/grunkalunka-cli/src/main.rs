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
};

fn compile_file(path: &str, value_profile: bool) -> Result<compiler::Artifact, Box<dyn Error>> {
    let typescript = match Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("ts") => true,
        Some("js") => false,
        _ => return Err("source extension must be .js or .ts".into()),
    };
    let source = fs::read_to_string(path)?;
    Ok(if value_profile {
        compiler::compile_value(&source, typescript)?
    } else {
        compiler::compile(&source, typescript)?
    })
}

fn evaluate(path: &str, value_profile: bool) -> Result<(), Box<dyn Error>> {
    let bytes = fs::read(path)?;
    if value_profile {
        let mut instance = ValueInstance::new(&bytes)?;
        input_pairs(|a, b| {
            let value = instance.run(a, b).map_err(|error| format!("{error:#}"))?;
            match value {
                Value::TypeError { .. } => {
                    return Err("the primitive value protocol does not admit Error objects".into());
                }
                Value::Undefined => println!("undefined"),
                Value::Null => println!("null"),
                Value::Boolean(value) => println!("boolean:{}", u8::from(value)),
                Value::Number(value) => println!("number:{:016x}", value.to_bits()),
            }
            Ok(())
        })
    } else {
        let mut instance = NumericInstance::new(&bytes)?;
        input_pairs(|a, b| {
            let value = instance.run(a, b)?;
            println!("{:016x}", value.to_bits());
            Ok(())
        })
    }
}

fn input_pairs(
    mut evaluate: impl FnMut(f64, f64) -> Result<(), Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    for (index, line) in io::stdin().lock().lines().enumerate() {
        if index >= 4096 {
            return Err("at most 4096 input pairs are admitted".into());
        }
        let line = line?;
        let inputs = line.split_whitespace().collect::<Vec<_>>();
        if inputs.len() != 2 || inputs.iter().any(|input| input.len() != 16) {
            return Err("expected two 16-digit hexadecimal binary64 bit patterns per line".into());
        }
        let a = f64::from_bits(u64::from_str_radix(inputs[0], 16)?);
        let b = f64::from_bits(u64::from_str_radix(inputs[1], 16)?);
        evaluate(a, b)?;
    }
    Ok(())
}

fn execute() -> Result<(), Box<dyn Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command] if matches!(command.as_str(), "--help" | "-h") => print!("{}", help::TEXT),
        [command] if matches!(command.as_str(), "--version" | "-V") => {
            println!("grunkalunka {}", env!("CARGO_PKG_VERSION"));
        }
        [command, source, destination] if command == "compile" || command == "compile-value" => {
            // Finish parsing/admission/encoding before opening the destination.
            // create_new also prevents accidental overwrite and stale-output success.
            let artifact = compile_file(source, command == "compile-value")?;
            OpenOptions::new().write(true).create_new(true).open(destination)?.write_all(&artifact.wasm)?;
            println!("COMPILED {} bytes: {source} -> {destination}", artifact.wasm.len());
        }
        [command, source] if command == "wat" || command == "wat-value" => print!("{}", compile_file(source, command == "wat-value")?.wat),
        [command, path] if command == "eval" || command == "eval-value" => evaluate(path, command == "eval-value")?,
        _ if arguments.first().is_some_and(|command| matches!(command.as_str(), "compile-value" | "wat-value" | "eval-value")) =>
            return Err("usage: grunkalunka compile-value SOURCE.{js,ts} OUT.wasm | wat-value SOURCE | eval-value OUT.wasm < input-bits.txt".into()),
        _ => return Err("usage: grunkalunka compile SOURCE.{js,ts} OUT.wasm | wat SOURCE | eval OUT.wasm < input-bits.txt".into()),
    }
    Ok(())
}

fn main() {
    if let Err(error) = execute() {
        eprintln!("ERROR: {error}");
        std::process::exit(1);
    }
}
