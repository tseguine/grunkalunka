//! Execute the Node literal oracles through composed Wasm and the owned Rust host.
use grunkalunka_runtime::HeapInstance;
use grunkalunka_wasm_link::{LinkLimits, compose};
use std::{error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let guest = fs::read(args.next().ok_or("guest path required")?)?;
    let directory = PathBuf::from(args.next().ok_or("fixture directory required")?);
    let mut cases = 0;
    let mut calls = 0;
    for row in fs::read_to_string(directory.join("oracles.tsv"))?.lines() {
        let (name, hex) = row.split_once('\t').ok_or("invalid oracle row")?;
        let expected = (0..hex.len())
            .step_by(4)
            .map(|i| u16::from_str_radix(&hex[i..i + 4], 16))
            .collect::<Result<Vec<_>, _>>()?;
        let composed = compose(
            &guest,
            &fs::read(directory.join(format!("{name}.wasm")))?,
            LinkLimits {
                max_input_bytes: 1_048_576,
                max_output_bytes: 2_097_152,
                max_items: 16_384,
                max_generated_data_bytes: 65_536,
            },
        )?;
        fs::write(directory.join(format!("{name}.composed.wasm")), &composed)?;
        let mut host = HeapInstance::new(&composed)
            .map_err(|error| format!("{name} initialization: {error:?}"))?;
        let mut retained = Vec::new();
        for repeat in 0..500 {
            let actual = host
                .run(1.0, 2.0)
                .map_err(|error| format!("{name} call {repeat}: {error:?}"))?;
            assert_eq!(actual, expected, "{name}: call {repeat}");
            retained.push(actual);
            calls += 1;
        }
        drop(host);
        assert!(retained.iter().all(|value| *value == expected), "{name}");
        cases += 1;
    }
    assert!(cases > 0);
    fs::write(
        directory.join("wasmtime-receipt.json"),
        format!("{{\"cases\":{cases},\"calls\":{calls}}}\n"),
    )?;
    println!("PASS Wasmtime owned String host: {cases} cases, {calls} calls");
    Ok(())
}
