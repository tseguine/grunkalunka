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
