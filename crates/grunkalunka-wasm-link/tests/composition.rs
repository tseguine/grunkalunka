use grunkalunka_wasm_link::{LinkError, LinkLimit, LinkLimits, compose};
use wasmtime::{Engine, Instance, Module, Store};

fn limits() -> LinkLimits {
    LinkLimits {
        max_input_bytes: 1_048_576,
        max_output_bytes: 2_097_152,
        max_items: 16_384,
        max_generated_data_bytes: 65_536,
    }
}

fn wasm(text: &str) -> Vec<u8> {
    wat::parse_str(text).unwrap()
}

fn run(bytes: &[u8], export: &str) -> i32 {
    let engine = Engine::default();
    let module = Module::from_binary(&engine, bytes).unwrap();
    assert_eq!(module.imports().len(), 0);
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    instance
        .get_typed_func::<(), i32>(&mut store, export)
        .unwrap()
        .call(&mut store, ())
        .unwrap()
}

#[test]
fn empty_generated_module_preserves_the_runtime() {
    let runtime = wasm(
        r#"(module (memory (export "memory") 1) (func (export "base") (result i32) i32.const 41))"#,
    );
    let generated = wasm(r#"(module (import "runtime" "memory" (memory 1)))"#);
    let result = compose(&runtime, &generated, limits()).unwrap();
    assert_eq!(result, runtime);
    assert_eq!(run(&result, "base"), 41);
}

#[test]
fn malformed_inputs_and_byte_quotas_fail_without_an_artifact() {
    let runtime = wasm(r#"(module (memory (export "memory") 1))"#);
    let generated = wasm(r#"(module (import "runtime" "memory" (memory 1)))"#);
    assert!(matches!(
        compose(b"bad", &generated, limits()),
        Err(LinkError::InvalidRuntime(_))
    ));
    assert!(matches!(
        compose(&runtime, b"bad", limits()),
        Err(LinkError::InvalidGenerated(_))
    ));
    let mut q = limits();
    q.max_input_bytes = runtime.len().max(generated.len());
    q.max_output_bytes = runtime.len();
    assert_eq!(compose(&runtime, &generated, q).unwrap(), runtime);
    q.max_input_bytes -= 1;
    assert_eq!(
        compose(&runtime, &generated, q),
        Err(LinkError::Limit(LinkLimit::Input))
    );
    q = limits();
    q.max_output_bytes = runtime.len() - 1;
    assert_eq!(
        compose(&runtime, &generated, q),
        Err(LinkError::Limit(LinkLimit::Output))
    );
}

#[test]
fn imported_functions_helpers_and_block_types_are_remapped() {
    let runtime = wasm(
        r#"(module (memory (export "memory") 1)
      (func (param f64) (result f64) local.get 0)
      (func (export "base") (param i32) (result i32) local.get 0 i32.const 40 i32.add)
      (func (export "other") (result i32) i32.const 9))"#,
    );
    let generated = wasm(
        r#"(module (import "runtime" "memory" (memory 1))
      (import "runtime" "base" (func $base (param i32) (result i32)))
      (type $pair (func (result i32 i32)))
      (func $helper (param i32) (result i32) local.get 0 call $base)
      (func (export "run") (result i32) i32.const 2 call $helper)
      (func (export "pair") (result i32) block (type $pair) i32.const 19 i32.const 23 end i32.add))"#,
    );
    let linked = compose(&runtime, &generated, limits()).unwrap();
    assert_eq!(run(&linked, "run"), 42);
    assert_eq!(run(&linked, "pair"), 42);
    assert_eq!(run(&linked, "other"), 9);
    let bodies = |bytes: &[u8]| {
        wasmparser::Parser::new(0)
            .parse_all(bytes)
            .filter_map(|p| match p.unwrap() {
                wasmparser::Payload::CodeSectionEntry(body) => Some(body.as_bytes().to_vec()),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let original = bodies(&runtime);
    assert_eq!(&bodies(&linked)[..original.len()], original);
}

#[test]
fn passive_literals_are_shifted_past_active_runtime_data() {
    let runtime = wasm(r#"(module (memory (export "memory") 1) (data (i32.const 0) "R"))"#);
    let generated = wasm(
        r#"(module (import "runtime" "memory" (memory 1))
      (func (export "literal") (result i32)
        i32.const 16 i32.const 0 i32.const 1 memory.init 0
        i32.const 16 i32.load8_u)
      (data "G"))"#,
    );
    let linked = compose(&runtime, &generated, limits()).unwrap();
    assert_eq!(run(&linked, "literal"), 71);
    let mut counts = Vec::new();
    for p in wasmparser::Parser::new(0).parse_all(&linked) {
        if let wasmparser::Payload::DataCountSection { count, .. } = p.unwrap() {
            counts.push(count);
        }
    }
    assert_eq!(counts, [2]);
}

#[test]
fn data_drop_does_not_drop_runtime_segments() {
    let runtime = wasm(
        r#"(module (memory (export "memory") 1)
      (func (export "base") (result i32) i32.const 0 i32.const 0 i32.const 1 memory.init 0 i32.const 0 i32.load8_u)
      (data "\0b"))"#,
    );
    let generated = wasm(
        r#"(module (import "runtime" "memory" (memory 1))
      (func (export "get") (result i32) i32.const 8 i32.const 0 i32.const 1 memory.init 0 i32.const 8 i32.load8_u)
      (func (export "drop") data.drop 0) (data "\16"))"#,
    );
    let linked = compose(&runtime, &generated, limits()).unwrap();
    let engine = Engine::default();
    let module = Module::from_binary(&engine, &linked).unwrap();
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).unwrap();
    let get = instance
        .get_typed_func::<(), i32>(&mut store, "get")
        .unwrap();
    let base = instance
        .get_typed_func::<(), i32>(&mut store, "base")
        .unwrap();
    assert_eq!(get.call(&mut store, ()).unwrap(), 22);
    assert_eq!(base.call(&mut store, ()).unwrap(), 11);
    instance
        .get_typed_func::<(), ()>(&mut store, "drop")
        .unwrap()
        .call(&mut store, ())
        .unwrap();
    assert!(get.call(&mut store, ()).is_err());
    assert_eq!(base.call(&mut store, ()).unwrap(), 11);
}

#[test]
fn imports_and_exports_must_match_exactly() {
    let runtime = wasm(
        r#"(module (memory (export "memory") 1) (func (export "f") (result i32) i32.const 1))"#,
    );
    let import = |module: &str, name: &str, signature: &str| {
        wasm(&format!(
            r#"(module (import "runtime" "memory" (memory 1)) (import "{module}" "{name}" (func {signature})))"#
        ))
    };
    for (module, name) in [
        ("other", "f"),
        ("runtime", "missing"),
        ("runtime", "memory"),
    ] {
        assert_eq!(
            compose(&runtime, &import(module, name, "(result i32)"), limits()),
            Err(LinkError::MissingImport {
                module: module.into(),
                name: name.into()
            })
        );
    }
    assert_eq!(
        compose(
            &runtime,
            &import("runtime", "f", "(param i32) (result i32)"),
            limits()
        ),
        Err(LinkError::SignatureMismatch("f".into()))
    );
    let duplicate = wasm(
        r#"(module (import "runtime" "memory" (memory 1)) (func (export "f") (result i32) i32.const 2))"#,
    );
    assert_eq!(
        compose(&runtime, &duplicate, limits()),
        Err(LinkError::DuplicateExport("f".into()))
    );
}

#[test]
fn unsupported_profiles_fail_instead_of_being_silently_linked() {
    let runtime = wasm(r#"(module (memory (export "memory") 1))"#);
    for source in [
        r#"(module)"#,
        r#"(module (import "runtime" "memory" (memory 1 2)))"#,
        r#"(module (import "runtime" "memory" (memory 2)))"#,
        r#"(module (import "runtime" "wrong" (memory 1)))"#,
        r#"(module (memory 1))"#,
        r#"(module (import "runtime" "memory" (memory 1)) (global i32 (i32.const 0)))"#,
        r#"(module (import "runtime" "memory" (memory 1)) (table 1 funcref))"#,
        r#"(module (import "runtime" "memory" (memory 1)) (func $s) (start $s))"#,
        r#"(module (import "runtime" "memory" (memory 1)) (data (i32.const 0) "x"))"#,
        r#"(module (import "runtime" "memory" (memory 1)) (export "m" (memory 0)))"#,
        r#"(module (import "runtime" "memory" (memory 1)) (type (func (param externref))))"#,
    ] {
        assert!(
            compose(&runtime, &wasm(source), limits()).is_err(),
            "{source}"
        );
    }
    let generated = wasm(r#"(module (import "runtime" "memory" (memory 1)))"#);
    for source in [
        r#"(module)"#,
        r#"(module (memory (export "other") 1))"#,
        r#"(module (import "host" "f" (func)) (memory (export "memory") 1))"#,
        r#"(module (memory (export "memory") i64 1))"#,
        r#"(module (memory (export "memory") 1 2 shared))"#,
        r#"(module (memory (export "memory") 1) (memory 1))"#,
        r#"(module (memory (export "memory") 1) (func $s) (start $s))"#,
        r#"(module (memory (export "memory") 1) (type (struct (field i32))))"#,
    ] {
        assert!(
            compose(&wasm(source), &generated, limits()).is_err(),
            "{source}"
        );
    }
}

#[test]
fn runtime_tables_globals_elements_data_and_selected_metadata_survive() {
    use std::borrow::Cow;
    use wasm_encoder::Section;
    let mut runtime = wasm(
        r#"(module (type $t (func (result i32))) (memory (export "memory") 1)
      (table 1 funcref) (global i32 (i32.const 17))
      (func $target (type $t) global.get 0)
      (func (export "base") (result i32) i32.const 0 call_indirect (type $t))
      (elem (i32.const 0) $target) (data (i32.const 0) "R") (data "K"))"#,
    );
    for name in ["producers", "target_features"] {
        wasm_encoder::CustomSection {
            name: Cow::Borrowed(name),
            data: Cow::Borrowed(&[0]),
        }
        .append_to(&mut runtime);
    }
    let generated = wasm(
        r#"(module (import "runtime" "memory" (memory 1)) (func (export "added") (result i32) i32.const 7))"#,
    );
    let linked = compose(&runtime, &generated, limits()).unwrap();
    assert_eq!(run(&linked, "base"), 17);
    assert_eq!(run(&linked, "added"), 7);
    let sections = |bytes: &[u8], ids: &[u8]| {
        wasmparser::Parser::new(0)
            .parse_all(bytes)
            .filter_map(|p| {
                p.unwrap().as_section().and_then(|(id, r)| {
                    ids.contains(&id)
                        .then(|| (id, bytes[r.start as usize..r.end as usize].to_vec()))
                })
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        sections(&runtime, &[4, 5, 6, 9, 0]),
        sections(&linked, &[4, 5, 6, 9, 0])
    );
    let data = |bytes: &[u8]| {
        wasmparser::Parser::new(0)
            .parse_all(bytes)
            .flat_map(|p| match p.unwrap() {
                wasmparser::Payload::DataSection(s) => s
                    .into_iter()
                    .map(|d| {
                        let r = d.unwrap().range;
                        bytes[r.start as usize..r.end as usize].to_vec()
                    })
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(data(&runtime), data(&linked));
    wasm_encoder::CustomSection {
        name: Cow::Borrowed(".debug_info"),
        data: Cow::Borrowed(&[0]),
    }
    .append_to(&mut runtime);
    assert!(matches!(
        compose(&runtime, &generated, limits()),
        Err(LinkError::UnsupportedRuntime(_))
    ));
}

#[test]
fn generated_data_and_combined_item_limits_are_inclusive() {
    let runtime = wasm(r#"(module (memory (export "memory") 1) (data "a") (data "b"))"#);
    let generated =
        wasm(r#"(module (import "runtime" "memory" (memory 1)) (data "cd") (data "ef"))"#);
    let mut q = limits();
    q.max_generated_data_bytes = 4;
    q.max_items = 4;
    let result = compose(&runtime, &generated, q).unwrap();
    q.max_generated_data_bytes = 3;
    assert_eq!(
        compose(&runtime, &generated, q),
        Err(LinkError::Limit(LinkLimit::Data))
    );
    q.max_generated_data_bytes = 4;
    q.max_items = 3;
    assert_eq!(
        compose(&runtime, &generated, q),
        Err(LinkError::Limit(LinkLimit::Items))
    );
    q = limits();
    q.max_output_bytes = result.len();
    assert_eq!(compose(&runtime, &generated, q).unwrap(), result);
    q.max_output_bytes -= 1;
    assert_eq!(
        compose(&runtime, &generated, q),
        Err(LinkError::Limit(LinkLimit::Output))
    );
    q = limits();
    q.max_items = 0;
    assert_eq!(
        compose(&runtime, &generated, q),
        Err(LinkError::Limit(LinkLimit::Items))
    );
}

#[test]
fn explicit_recursion_groups_are_outside_the_plain_type_profile() {
    let runtime = wasm(r#"(module (memory (export "memory") 1) (rec (type (func))))"#);
    let generated = wasm(r#"(module (import "runtime" "memory" (memory 1)))"#);
    assert!(matches!(
        compose(&runtime, &generated, limits()),
        Err(LinkError::UnsupportedRuntime(_))
    ));
}

#[test]
fn compact_imports_charge_each_logical_import() {
    use std::borrow::Cow;
    use wasm_encoder::{EntityType, ImportSection, Imports, MemoryType, TypeSection};
    let runtime = wasm(
        r#"(module (memory (export "memory") 1) (func (export "f") (result i32) i32.const 1))"#,
    );
    let mut types = TypeSection::new();
    types.ty().function([], [wasm_encoder::ValType::I32]);
    let mut imports = ImportSection::new();
    imports.import(
        "runtime",
        "memory",
        MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        },
    );
    imports.imports(Imports::Compact2 {
        module: "runtime",
        ty: EntityType::Function(0),
        names: Cow::Borrowed(&["f"; 16]),
    });
    let mut app = wasm_encoder::Module::new();
    app.section(&types).section(&imports);
    let generated = app.finish();
    let mut q = limits();
    q.max_items = 2;
    assert_eq!(
        compose(&runtime, &generated, q),
        Err(LinkError::Limit(LinkLimit::Items))
    );
    q.max_items = 17;
    assert_eq!(compose(&runtime, &generated, q).unwrap(), runtime);
}
