//! Malformed canonical results must trap rather than become successful policy errors.
use std::fmt::Write as _;
use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};

wasmtime::component::bindgen!({
    inline: "
        package grunkalunka:wire-check;
        world wire-check {
            enum read-error { denied }
            export read: func() -> result<string, read-error>;
            export cleanup-count: func() -> u32;
        }
    ",
    world: "wire-check",
});

fn engine() -> Engine {
    let mut config = Config::new();
    config.consume_fuel(true);
    Engine::new(&config).unwrap()
}

// The only successful payload is stored at byte 64. Header words may deliberately
// lie about that storage. This is an independent transport fixture, not a parser.
fn fixture(
    engine: &Engine,
    tag: u32,
    pointer: u32,
    length: u32,
    result_pointer: u32,
    text: &[u8],
) -> (Store<()>, WireCheck) {
    assert!(text.len() <= 32);
    let mut memory = vec![0; 96];
    memory[0..4].copy_from_slice(&tag.to_le_bytes());
    memory[4..8].copy_from_slice(&pointer.to_le_bytes());
    memory[8..12].copy_from_slice(&length.to_le_bytes());
    memory[64..64 + text.len()].copy_from_slice(text);
    let mut data = String::new();
    for byte in memory {
        write!(data, "\\{byte:02x}").unwrap();
    }
    let wat = format!(
        r#"(component
          (type $error (enum "denied"))
          (export $public-error "read-error" (type $error))
          (core module $m
            (memory (export "memory") 1 1)
            (data (i32.const 0) "{data}")
            (global $cleanups (mut i32) (i32.const 0))
            (func (export "read") (result i32) (i32.const {result_pointer}))
            (func (export "cleanup") (param i32)
              (memory.fill (i32.const 0) (i32.const 0) (i32.const 96))
              (global.set $cleanups (i32.add (global.get $cleanups) (i32.const 1))))
            (func (export "cleanup-count") (result i32) (global.get $cleanups)))
          (core instance $i (instantiate $m))
          (func (export "read") (result (result string (error $public-error)))
            (canon lift (core func $i "read") (memory (core memory $i "memory"))
              (post-return (core func $i "cleanup"))))
          (func (export "cleanup-count") (result u32)
            (canon lift (core func $i "cleanup-count")))
        )"#
    );
    let bytes = wat::parse_str(wat).unwrap();
    let component = Component::from_binary(engine, &bytes).unwrap();
    let mut store = Store::new(engine, ());
    store.set_fuel(100_000).unwrap();
    let reader = WireCheck::instantiate(&mut store, &component, &Linker::new(engine)).unwrap();
    (store, reader)
}

#[test]
fn valid_success_and_denial_are_values_with_observable_post_return_cleanup() {
    let engine = engine();
    let expected = "猫😀";
    let (mut store, reader) = fixture(&engine, 0, 64, 7, 0, expected.as_bytes());
    let copied = reader.call_read(&mut store).unwrap().unwrap();
    assert_eq!(reader.call_cleanup_count(&mut store).unwrap(), 1);
    assert_eq!(
        copied, expected,
        "host output survives cleared guest storage"
    );
    drop(store);
    assert_eq!(copied, expected);

    let (mut store, reader) = fixture(&engine, 1, 0, 0, 0, b"");
    assert_eq!(
        reader.call_read(&mut store).unwrap(),
        Err(ReadError::Denied)
    );
    assert_eq!(reader.call_cleanup_count(&mut store).unwrap(), 1);
}

#[test]
fn invalid_utf8_and_invalid_result_discriminants_are_not_policy_denials() {
    let engine = engine();
    for (label, tag, pointer, length, text) in [
        ("invalid UTF-8", 0, 64, 1, &b"\xff"[..]),
        ("truncated UTF-8", 0, 64, 2, "猫".as_bytes()),
        ("invalid result tag", 2, 0, 0, &b""[..]),
        ("invalid error enum", 1, 1, 0, &b""[..]),
    ] {
        let (mut store, reader) = fixture(&engine, tag, pointer, length, 0, text);
        assert!(reader.call_read(&mut store).is_err(), "{label}");
        // Discard the instance: no normal result or post-trap recovery is claimed.
    }
}

#[test]
fn out_of_bounds_and_unaligned_result_storage_is_rejected() {
    let engine = engine();
    for (label, pointer, length, result_pointer) in [
        ("string starts past memory", 65536, 1, 0),
        ("string crosses memory end", 65535, 2, 0),
        ("string length overflows memory", 64, u32::MAX, 0),
        ("result header crosses memory end", 64, 1, 65532),
        ("result header is unaligned", 64, 1, 1),
    ] {
        let (mut store, reader) = fixture(&engine, 0, pointer, length, result_pointer, b"x");
        assert!(reader.call_read(&mut store).is_err(), "{label}");
    }
    let (mut fresh, reader) = fixture(&engine, 0, 64, 1, 0, b"x");
    assert_eq!(reader.call_read(&mut fresh).unwrap().unwrap(), "x");
}
