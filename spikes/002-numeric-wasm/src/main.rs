//! Disposable compiler/Wasmtime harness for locally generated numeric artifacts.
mod compiler;

use std::{
    env,
    error::Error,
    fs::{self, OpenOptions},
    io::{self, BufRead, Write},
    path::Path,
};
use wasmtime::{Config, Engine, Instance, Module, Store};

fn compile_file(path: &str) -> Result<compiler::Artifact, Box<dyn Error>> {
    let typescript = match Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("ts") => true,
        Some("js") => false,
        _ => return Err("source extension must be .js or .ts".into()),
    };
    let source = fs::read_to_string(path)?;
    Ok(compiler::compile(&source, typescript)?)
}

fn evaluate(path: &str) -> Result<(), Box<dyn Error>> {
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let module = Module::from_binary(&engine, &fs::read(path)?)?;
    if module.imports().len() != 0 {
        return Err("this experiment admits no module imports".into());
    }
    let mut store = Store::new(&engine, ());
    store.set_fuel(10_000)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let run = instance.get_typed_func::<(f64, f64), f64>(&mut store, "run")?;
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
        store.set_fuel(10_000)?;
        let value = run.call(&mut store, (a, b))?;
        println!("{:016x}", value.to_bits());
    }
    Ok(())
}

fn execute() -> Result<(), Box<dyn Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command, source, destination] if command == "compile" => {
            // Finish parsing/admission/encoding before opening the destination.
            // create_new also prevents accidental overwrite and stale-output success.
            let artifact = compile_file(source)?;
            OpenOptions::new().write(true).create_new(true).open(destination)?.write_all(&artifact.wasm)?;
            println!("COMPILED {} bytes: {source} -> {destination}", artifact.wasm.len());
        }
        [command, source] if command == "wat" => print!("{}", compile_file(source)?.wat),
        [command, path] if command == "eval" => evaluate(path)?,
        _ => return Err("usage: grunkalunka-numeric-spike compile SOURCE.{js,ts} OUT.wasm | wat SOURCE | eval OUT.wasm < input-bits.txt".into()),
    }
    Ok(())
}

fn main() {
    if let Err(error) = execute() {
        eprintln!("ERROR: {error}");
        std::process::exit(1);
    }
}
