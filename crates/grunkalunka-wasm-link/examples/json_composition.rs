//! Build and execute the real JSON guest plus a generated fixture; no host imports.
use grunkalunka_wasm_link::{LinkLimits, compose};
use std::{error::Error, fs};
use wasmtime::{Config, Engine, Instance, Module, Store};
type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn unhex(text: &str) -> Result<Vec<u16>> {
    if !text.is_ascii() || !text.len().is_multiple_of(4) {
        return Err("invalid UTF16 oracle".into());
    }
    (0..text.len())
        .step_by(4)
        .map(|i| u16::from_str_radix(&text[i..i + 4], 16).map_err(Into::into))
        .collect()
}
fn output(store: &mut Store<()>, instance: Instance) -> Result<Vec<u16>> {
    let ptr = instance
        .get_typed_func::<(), u32>(&mut *store, "gk_json_output_ptr")?
        .call(&mut *store, ())?;
    let len = instance
        .get_typed_func::<(), u32>(&mut *store, "gk_json_output_len")?
        .call(&mut *store, ())?;
    if ptr % 2 != 0 || len > 65536 {
        return Err("output range/alignment".into());
    }
    let mut bytes = vec![0; len as usize * 2];
    instance
        .get_memory(&mut *store, "memory")
        .ok_or("memory")?
        .read(&*store, ptr as usize, &mut bytes)?;
    Ok(bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes(*b))
        .collect())
}
fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 4 {
        return Err(
            "usage: json_composition <runtime.wasm> <generated.wat> <output.wasm> <expected.tsv>"
                .into(),
        );
    }
    let runtime = fs::read(&args[0])?;
    let generated = wat::parse_file(&args[1])?;
    let linked = compose(
        &runtime,
        &generated,
        LinkLimits {
            max_input_bytes: 1_048_576,
            max_output_bytes: 2_097_152,
            max_items: 16_384,
            max_generated_data_bytes: 65_536,
        },
    )?;
    fs::write(&args[2], &linked)?;
    let expected = fs::read_to_string(&args[3])?
        .lines()
        .map(unhex)
        .collect::<Result<Vec<_>>>()?;
    if expected.len() != 2 {
        return Err("exactly two fixture oracles required".into());
    }
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let module = Module::from_binary(&engine, &linked)?;
    assert_eq!(module.imports().len(), 0);
    let mut a = Store::new(&engine, ());
    a.set_fuel(100_000_000)?;
    let ia = Instance::new(&mut a, &module, &[])?;
    let round = ia.get_typed_func::<u32, u32>(&mut a, "roundtrip")?;
    let release = ia.get_typed_func::<u32, u32>(&mut a, "gk_json_release")?;
    let mut saved = Vec::new();
    for i in 0..500 {
        a.set_fuel(100_000_000)?;
        let kind = i % 2;
        let id = round.call(&mut a, kind)?;
        assert_ne!(id, 0);
        let text = output(&mut a, ia)?;
        assert_eq!(text, expected[kind as usize]);
        if i == 0 {
            saved = text;
        }
        assert_eq!(release.call(&mut a, id)?, 0);
    }
    let sum = ia.get_typed_func::<(f64, f64), f64>(&mut a, "sum")?;
    for (x, y) in [
        (0., -0.),
        (-0., -0.),
        (41., 1.),
        (f64::INFINITY, f64::NEG_INFINITY),
        (f64::NAN, 1.),
    ] {
        a.set_fuel(100_000_000)?;
        let actual = sum.call(&mut a, (x, y))?;
        let want = x + y;
        assert!(if want.is_nan() {
            actual.is_nan()
        } else {
            actual.to_bits() == want.to_bits()
        });
    }
    assert_eq!(
        ia.get_typed_func::<(), (i32, f64)>(&mut a, "value")?
            .call(&mut a, ())?,
        (3, 42.)
    );
    let mut b = Store::new(&engine, ());
    b.set_fuel(100_000_000)?;
    let ib = Instance::new(&mut b, &module, &[])?;
    let id = ib
        .get_typed_func::<u32, u32>(&mut b, "roundtrip")?
        .call(&mut b, 1)?;
    assert_ne!(id, 0);
    assert_eq!(output(&mut b, ib)?, expected[1]);
    assert_eq!(saved, expected[0]);
    println!(
        "PASS composed JSON Wasmtime: 500 alternating literal lifecycles, helpers, data/type/function remaps, numeric and paired ABI, instance isolation/copies; {} bytes, zero imports",
        linked.len()
    );
    Ok(())
}
