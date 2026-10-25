//! Check the actual import-free JSON guest against UTF16 oracles from the Node runner.
use std::{error::Error, fs};
use wasmtime::{Config, Engine, Instance, Module, Store};
type Result<T> = std::result::Result<T, Box<dyn Error>>;

struct Guest {
    store: Store<()>,
    instance: Instance,
}

impl Guest {
    fn new(engine: &Engine, module: &Module) -> Result<Self> {
        let mut store = Store::new(engine, ());
        store.set_fuel(100_000_000)?;
        let instance = Instance::new(&mut store, module, &[])?;
        Ok(Self { store, instance })
    }

    fn call0(&mut self, name: &str) -> Result<u32> {
        self.store.set_fuel(100_000_000)?;
        Ok(self
            .instance
            .get_typed_func::<(), u32>(&mut self.store, name)?
            .call(&mut self.store, ())?)
    }

    fn call1(&mut self, name: &str, n: u32) -> Result<u32> {
        self.store.set_fuel(100_000_000)?;
        Ok(self
            .instance
            .get_typed_func::<u32, u32>(&mut self.store, name)?
            .call(&mut self.store, n)?)
    }

    fn prepare(&mut self, units: &[u16]) -> Result<()> {
        let ptr = self.call1("gk_json_input_resize", units.len().try_into()?)?;
        assert_ne!(ptr, 0);
        assert_eq!(ptr % 2, 0);
        assert_eq!(self.call0("gk_json_input_len")? as usize, units.len());
        assert_eq!(self.call0("gk_json_status")?, 0);
        let memory = self
            .instance
            .get_memory(&mut self.store, "memory")
            .ok_or("missing memory")?;
        let mut zero = vec![0xff; units.len() * 2];
        memory.read(&self.store, ptr as usize, &mut zero)?;
        assert!(zero.iter().all(|b| *b == 0));
        let bytes = units
            .iter()
            .flat_map(|u| u.to_le_bytes())
            .collect::<Vec<_>>();
        memory.write(&mut self.store, ptr as usize, &bytes)?;
        Ok(())
    }

    fn output(&mut self) -> Result<Vec<u16>> {
        let ptr = self.call0("gk_json_output_ptr")?;
        let len = self.call0("gk_json_output_len")?;
        assert_eq!(ptr % 2, 0);
        assert!(len <= 65536);
        let memory = self
            .instance
            .get_memory(&mut self.store, "memory")
            .ok_or("missing memory")?;
        let mut bytes = vec![0; len as usize * 2];
        memory.read(&self.store, ptr as usize, &mut bytes)?;
        Ok(bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes(*b))
            .collect())
    }
}

fn unhex(text: &str) -> Result<Vec<u16>> {
    if !text.len().is_multiple_of(4) || !text.is_ascii() {
        return Err("invalid UTF16 oracle hex".into());
    }
    (0..text.len())
        .step_by(4)
        .map(|i| u16::from_str_radix(&text[i..i + 4], 16).map_err(Into::into))
        .collect()
}

fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: json_guest_check <module.wasm> <cases.tsv>".into());
    }
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let bytes = fs::read(&args[0])?;
    let module = Module::from_binary(&engine, &bytes)?;
    assert_eq!(module.imports().len(), 0);
    let mut guest = Guest::new(&engine, &module)?;
    let mut count = 0;
    for line in fs::read_to_string(&args[1])?.lines() {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 3 {
            return Err("invalid oracle record".into());
        }
        let text = unhex(fields[1])?;
        guest.prepare(&text)?;
        let id = guest.call0("gk_json_parse")?;
        match fields[0] {
            "invalid" | "parse-limit" => {
                assert_eq!(id, 0);
                assert_eq!(guest.call0("gk_json_status")?, fields[2].parse::<u32>()?);
                assert!(guest.call0("gk_json_error_offset")? as usize <= text.len());
            }
            "valid" | "output-limit" => {
                assert_ne!(id, 0);
                assert_eq!(guest.call0("gk_json_status")?, 0);
                let status = guest.call1("gk_json_stringify", id)?;
                if fields[0] == "valid" {
                    assert_eq!(status, 0);
                    assert_eq!(guest.output()?, unhex(fields[2])?);
                } else {
                    assert_eq!(status, fields[2].parse::<u32>()?);
                    assert!(guest.output()?.is_empty());
                }
                assert_eq!(guest.call1("gk_json_release", id)?, 0);
                assert_eq!(guest.call1("gk_json_stringify", id)?, 4);
            }
            _ => return Err("unknown oracle record kind".into()),
        }
        assert!(guest.output()?.is_empty());
        count += 1;
    }
    let sample = r#"{"s":"\ud800","n":-0}"#.encode_utf16().collect::<Vec<_>>();
    let expected = r#"{"s":"\ud800","n":0}"#.encode_utf16().collect::<Vec<_>>();
    guest.prepare(&sample)?;
    let keep = guest.call0("gk_json_parse")?;
    assert_ne!(keep, 0);
    assert_eq!(guest.call1("gk_json_stringify", keep)?, 0);
    let saved = guest.output()?;
    assert_eq!(saved, expected);
    for _ in 0..500 {
        guest.prepare(&sample)?;
        let id = guest.call0("gk_json_parse")?;
        assert_ne!(id, 0);
        assert_ne!(id, keep);
        assert_eq!(guest.call1("gk_json_stringify", id)?, 0);
        assert_eq!(guest.output()?, saved);
        assert_eq!(guest.call1("gk_json_release", id)?, 0);
    }
    assert_eq!(guest.call1("gk_json_stringify", keep)?, 0);
    assert_eq!(guest.output()?, saved);
    assert_eq!(guest.call1("gk_json_release", keep)?, 0);
    guest.prepare(&[48])?;
    let mut ids = Vec::new();
    for _ in 0..64 {
        let id = guest.call0("gk_json_parse")?;
        assert_ne!(id, 0);
        ids.push(id);
    }
    assert_eq!(guest.call0("gk_json_parse")?, 0);
    assert_eq!(guest.call0("gk_json_status")?, 2);
    assert_eq!(guest.call1("gk_json_release", ids[0])?, 0);
    let replacement = guest.call0("gk_json_parse")?;
    assert!(replacement > *ids.iter().max().ok_or("no IDs")?);
    assert_eq!(guest.call1("gk_json_release", ids[0])?, 4);
    for &id in &ids[1..] {
        assert_eq!(guest.call1("gk_json_release", id)?, 0);
    }
    assert_eq!(guest.call1("gk_json_release", replacement)?, 0);
    for units in [32769, u32::MAX] {
        assert_eq!(guest.call1("gk_json_input_resize", units)?, 0);
        assert_eq!(guest.call0("gk_json_status")?, 1);
        assert_eq!(guest.call0("gk_json_input_len")?, 0);
        assert_eq!(guest.call0("gk_json_parse")?, 0);
        assert_eq!(guest.call0("gk_json_status")?, 5);
    }
    let mut other = Guest::new(&engine, &module)?;
    other.prepare(&"true".encode_utf16().collect::<Vec<_>>())?;
    let other_id = other.call0("gk_json_parse")?;
    guest.prepare(&"false".encode_utf16().collect::<Vec<_>>())?;
    let own_id = guest.call0("gk_json_parse")?;
    assert_eq!(other.call1("gk_json_stringify", other_id)?, 0);
    assert_eq!(other.output()?, "true".encode_utf16().collect::<Vec<_>>());
    assert_eq!(guest.call1("gk_json_stringify", own_id)?, 0);
    assert_eq!(guest.output()?, "false".encode_utf16().collect::<Vec<_>>());
    assert_eq!(saved, expected);
    println!(
        "PASS Wasmtime JSON guest: {count} original UTF16 oracle records, 500 repeats, quotas/release/stale IDs/instances/snapshots; {} bytes, zero imports",
        bytes.len()
    );
    Ok(())
}
