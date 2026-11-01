use grunkalunka_json::{JsonValue, StringifyLimits, stringify};

#[test]
fn cloned_json_retains_object_order_and_utf16_payloads() {
    let value = JsonValue::Object(vec![
        (
            vec![0x0061],
            JsonValue::Array(vec![
                JsonValue::String(vec![0xd800, 0xdc00]),
                JsonValue::Number(-0.0),
                JsonValue::Boolean(false),
            ]),
        ),
        (vec![0x0062], JsonValue::Null),
    ]);
    let limits = StringifyLimits {
        max_output_units: 4096,
        max_depth: 128,
        max_values: 4096,
        max_work: 16384,
    };
    assert_eq!(
        stringify(&value, limits).unwrap(),
        stringify(&value.clone(), limits).unwrap(),
    );
}

#[test]
fn deep_manually_built_json_clones_on_two_mib_stack() {
    // Parsing and stringifying have their own depth-128 admission budgets.
    // This deliberately out-of-contract tree isolates Clone's native stack.
    const DEPTH: usize = 4096;
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let mut root = JsonValue::String(vec![0xd800]);
            for depth in 0..DEPTH {
                root = if depth % 2 == 0 {
                    JsonValue::Array(vec![root])
                } else {
                    JsonValue::Object(vec![(vec![0x0061], root)])
                };
            }
            let copy = root.clone();
            // Destruction is an independent stack-depth concern. Consume
            // each synthetic chain explicitly so it cannot mask Clone.
            for mut tree in [root, copy] {
                for depth in (0..DEPTH).rev() {
                    tree = if depth % 2 == 0 {
                        let JsonValue::Array(mut elements) = tree else {
                            panic!("expected array");
                        };
                        elements.pop().expect("single array child")
                    } else {
                        let JsonValue::Object(mut entries) = tree else {
                            panic!("expected object");
                        };
                        let (key, child) = entries.pop().expect("single object entry");
                        assert_eq!(key, vec![0x0061]);
                        child
                    };
                }
                let JsonValue::String(units) = tree else {
                    panic!("expected UTF-16 leaf");
                };
                assert_eq!(units, [0xd800]);
            }
        })
        .expect("native small-stack clone thread")
        .join()
        .expect("iterative JSON cloning succeeded");
}

#[test]
fn deep_owned_json_debug_and_explicit_drop_have_bounded_stack_depth() {
    const DEPTH: usize = 4096;
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let mut root = JsonValue::String(vec![0xd800]);
            for depth in 0..DEPTH {
                root = if depth % 2 == 0 {
                    JsonValue::Array(vec![root])
                } else {
                    JsonValue::Object(vec![(vec![0x0061], root)])
                };
            }
            let rendered = format!("{root:?}");
            assert!(rendered.starts_with("Object([([97], Array(["));
            assert!(rendered.contains("String([55296])"));
            root.drop_iteratively();
        })
        .expect("native small-stack JSON thread")
        .join()
        .expect("iterative debug and release succeeded");
}

#[test]
fn compact_and_alternate_json_debug_keep_derived_shape() {
    let root = JsonValue::Object(vec![
        (vec![0x0061], JsonValue::Array(vec![JsonValue::Null])),
        (vec![0x0062], JsonValue::Boolean(true)),
    ]);
    assert_eq!(
        format!("{root:?}"),
        "Object([([97], Array([Null])), ([98], Boolean(true))])"
    );
    let pretty = format!("{root:#?}");
    assert!(pretty.starts_with("Object(\n    [\n        (\n"));
    assert!(pretty.contains("Null,\n"));
    assert!(pretty.contains("Boolean(\n"));
    assert!(pretty.ends_with("    ],\n)"));
}

#[test]
fn opted_in_raii_json_release_avoids_implicit_recursive_drop() {
    use grunkalunka_json::OwnedJson;

    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut root = JsonValue::String(vec![0xd800]);
            for level in 0..4096 {
                root = if level % 2 == 0 {
                    JsonValue::Array(vec![root])
                } else {
                    JsonValue::Object(vec![(vec![0x0061], root)])
                };
            }
            let owned = OwnedJson::from(root);
            assert!(matches!(&*owned, JsonValue::Object(_)));
            let copy = owned.clone();
            assert!(format!("{copy:?}").contains("String([55296])"));
            // Both deep owned trees drop automatically on a 2 MiB native
            // stack, with no explicit per-node teardown by the test.
        })
        .expect("small-stack owned JSON thread")
        .join()
        .expect("RAII owned JSON teardown");
}

#[test]
fn owned_json_can_return_raw_values_without_losing_scalar_bits() {
    use grunkalunka_json::OwnedJson;

    let value = OwnedJson::new(JsonValue::Number(-0.0));
    let JsonValue::Number(number) = value.into_inner() else {
        panic!("owned wrapper must yield the original enum");
    };
    assert_eq!(number.to_bits(), (-0.0f64).to_bits());
}
