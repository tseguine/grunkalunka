use grunkalunka_runtime::HeapInstance;

fn fixture(run: &str, units: &[u16]) -> Vec<u8> {
    wat::parse_str(fixture_wat(run, units)).unwrap()
}

fn fixture_wat(run: &str, units: &[u16]) -> String {
    let data: String = units
        .iter()
        .flat_map(|unit| unit.to_le_bytes())
        .map(|byte| format!("\\{byte:02x}"))
        .collect();
    format!(
        r#"(module
        (memory (export "memory") 1)
        (data (i32.const 0) "{data}")
        (func (export "run") (param f64 f64) (result i32 f64) {run})
        (func (export "gk_json_string_units") (param i32) (result i32) i32.const 0)
        (func (export "gk_json_output_ptr") (result i32) i32.const 0)
        (func (export "gk_json_output_len") (result i32) i32.const {length})
        (func (export "gk_json_release") (param i32) (result i32) i32.const 0))"#,
        length = units.len(),
    )
}

#[test]
fn empty_string_has_an_owned_host_result() {
    let mut host = HeapInstance::new(&fixture("i32.const 4 f64.const 1", &[])).unwrap();
    assert_eq!(host.run(1.0, 2.0).unwrap(), Vec::<u16>::new());
}

#[test]
fn raw_units_survive_the_guest_and_host_lifetime() {
    let mut host =
        HeapInstance::new(&fixture("i32.const 4 f64.const 1", &[65, 0xd800, 0])).unwrap();
    let value = host.run(1.0, 2.0).unwrap();
    drop(host);
    assert_eq!(value, [65, 0xd800, 0]);
}

#[test]
fn heap_traps_are_terminal_even_when_the_next_call_would_return() {
    for trap in ["unreachable", "loop br 0 end"] {
        let body = format!("local.get 0 f64.const 0 f64.ne if {trap} end i32.const 4 f64.const 1");
        let mut host = HeapInstance::new(&fixture(&body, &[])).unwrap();
        assert!(host.run(1.0, 2.0).is_err(), "{trap}");
        assert!(host.run(0.0, 2.0).is_err(), "reused after {trap}");
    }
}

#[test]
fn string_roots_require_their_tag_and_a_canonical_nonzero_u32_id() {
    for (tag, payload) in [
        (3, "1"),
        (4, "0"),
        (4, "-0"),
        (4, "-1"),
        (4, "1.5"),
        (4, "4294967296"),
        (4, "nan"),
        (4, "inf"),
    ] {
        let body = format!("i32.const {tag} f64.const {payload}");
        let mut host = HeapInstance::new(&fixture(&body, &[])).unwrap();
        let error = host.run(0.0, 0.0).unwrap_err();
        assert!(
            error.to_string().contains("tag or root ID"),
            "{body}: {error}"
        );
        assert!(
            host.run(0.0, 0.0)
                .unwrap_err()
                .to_string()
                .contains("unusable")
        );
    }
}

#[test]
fn malformed_buffers_and_guest_errors_do_not_produce_host_values() {
    let original = fixture_wat("i32.const 4 f64.const 1", &[65]);
    for (from, to) in [
        (
            "gk_json_output_len\") (result i32) i32.const 1",
            "gk_json_output_len\") (result i32) i32.const 32769",
        ),
        (
            "gk_json_output_len\") (result i32) i32.const 1",
            "gk_json_output_len\") (result i32) i32.const 65537",
        ),
        (
            "gk_json_output_len\") (result i32) i32.const 1",
            "gk_json_output_len\") (result i32) i32.const -1",
        ),
        (
            "gk_json_output_ptr\") (result i32) i32.const 0",
            "gk_json_output_ptr\") (result i32) i32.const 1",
        ),
        (
            "gk_json_output_ptr\") (result i32) i32.const 0",
            "gk_json_output_ptr\") (result i32) i32.const 65536",
        ),
        (
            "gk_json_string_units\") (param i32) (result i32) i32.const 0",
            "gk_json_string_units\") (param i32) (result i32) i32.const 4",
        ),
        (
            "gk_json_release\") (param i32) (result i32) i32.const 0",
            "gk_json_release\") (param i32) (result i32) i32.const 4",
        ),
    ] {
        let changed = original.replace(from, to);
        assert_ne!(changed, original);
        let mut host = HeapInstance::new(&wat::parse_str(changed).unwrap()).unwrap();
        assert!(host.run(0.0, 0.0).is_err(), "{to}");
        assert!(
            host.run(0.0, 0.0)
                .unwrap_err()
                .to_string()
                .contains("unusable")
        );
    }
}

#[test]
fn maximum_string_copy_is_owned_before_release_changes_memory() {
    let expected = vec![0xffff; 32_768];
    let source = fixture_wat("i32.const 4 f64.const 1", &expected).replace(
        "gk_json_release\") (param i32) (result i32) i32.const 0",
        "gk_json_release\") (param i32) (result i32) i32.const 0 i32.const 0 i32.store16 i32.const 0",
    );
    let mut host = HeapInstance::new(&wat::parse_str(source).unwrap()).unwrap();
    let actual = host.run(0.0, 0.0).unwrap();
    drop(host);
    assert_eq!(actual, expected);
}

#[test]
fn serialized_strings_can_exceed_the_input_buffer_quota() {
    for length in [32_769, 65_536] {
        let expected = vec![0xffff; length];
        let source = fixture_wat("i32.const 4 f64.const 1", &expected).replace(
            "(memory (export \"memory\") 1)",
            "(memory (export \"memory\") 2)",
        );
        let mut host = HeapInstance::new(&wat::parse_str(source).unwrap()).unwrap();
        assert_eq!(host.run(0.0, 0.0).unwrap(), expected);
    }
}

#[test]
fn initialization_has_fuel_and_uncomposed_imports_are_rejected() {
    let mut source = fixture_wat("i32.const 4 f64.const 1", &[]);
    source.pop();
    source.push_str("(func $init loop br 0 end) (start $init))");
    let error = HeapInstance::new(&wat::parse_str(source).unwrap())
        .err()
        .unwrap();
    assert_eq!(
        error.downcast_ref::<wasmtime::Trap>(),
        Some(&wasmtime::Trap::OutOfFuel)
    );
    let imported = wat::parse_str("(module (import \"env\" \"f\" (func)))").unwrap();
    assert!(
        HeapInstance::new(&imported)
            .err()
            .unwrap()
            .to_string()
            .contains("no module imports")
    );
}
