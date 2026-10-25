use grunkalunka_json::{JsonValue, StringifyError, StringifyLimit, StringifyLimits, stringify};

fn limits() -> StringifyLimits {
    StringifyLimits {
        max_output_units: 16384,
        max_depth: 128,
        max_values: 1024,
        max_work: 65536,
    }
}

#[test]
fn serializes_null() {
    assert_eq!(
        stringify(&JsonValue::Null, limits()).unwrap(),
        "null".encode_utf16().collect::<Vec<_>>()
    );
}

#[test]
fn serializes_both_booleans() {
    for (value, expected) in [(true, "true"), (false, "false")] {
        assert_eq!(
            stringify(&JsonValue::Boolean(value), limits()).unwrap(),
            expected.encode_utf16().collect::<Vec<_>>()
        );
    }
}

#[test]
fn numbers_use_ecmascript_decimal_text_and_nonfinite_null() {
    for (value, expected) in [
        (0.0, "0"),
        (-0.0, "0"),
        (1.25, "1.25"),
        (1e21, "1e+21"),
        (1e20, "100000000000000000000"),
        (1e-6, "0.000001"),
        (1e-7, "1e-7"),
        (f64::from_bits(1), "5e-324"),
        (f64::MAX, "1.7976931348623157e+308"),
        (f64::NAN, "null"),
        (f64::INFINITY, "null"),
        (f64::NEG_INFINITY, "null"),
    ] {
        assert_eq!(
            stringify(&JsonValue::Number(value), limits()).unwrap(),
            expected.encode_utf16().collect::<Vec<_>>(),
            "bits={:016x}",
            value.to_bits()
        );
    }
}

#[test]
fn strings_escape_json_controls_and_lone_surrogates_but_preserve_pairs() {
    for (input, expected) in [
        (vec![], "\"\"".encode_utf16().collect::<Vec<_>>()),
        (
            vec![34, 92, 47, 8, 9, 10, 12, 13, 0, 31],
            "\"\\\"\\\\/\\b\\t\\n\\f\\r\\u0000\\u001f\""
                .encode_utf16()
                .collect(),
        ),
        (vec![97, 0x2028, 0xffff], vec![34, 97, 0x2028, 0xffff, 34]),
        (vec![0xd834, 0xdd1e], vec![34, 0xd834, 0xdd1e, 34]),
        (
            vec![0xd800, 0xd834, 0xdd1e, 0xdc00],
            [
                "\"\\ud800".encode_utf16().collect::<Vec<_>>(),
                vec![0xd834, 0xdd1e],
                "\\udc00\"".encode_utf16().collect(),
            ]
            .concat(),
        ),
    ] {
        assert_eq!(
            stringify(&JsonValue::String(input), limits()).unwrap(),
            expected
        );
    }
}

#[test]
fn arrays_keep_order_and_nesting_without_extra_separators() {
    for (value, expected) in [
        (JsonValue::Array(vec![]), "[]"),
        (
            JsonValue::Array(vec![
                JsonValue::Null,
                JsonValue::Boolean(false),
                JsonValue::Number(-0.0),
                JsonValue::String(vec![120]),
                JsonValue::Array(vec![JsonValue::Number(f64::INFINITY)]),
            ]),
            "[null,false,0,\"x\",[null]]",
        ),
    ] {
        assert_eq!(
            stringify(&value, limits()).unwrap(),
            expected.encode_utf16().collect::<Vec<_>>()
        );
    }
}

#[test]
fn objects_put_array_index_keys_first_and_keep_other_insertion_order() {
    let value = JsonValue::Object(
        [
            "a",
            "10",
            "2",
            "01",
            "0",
            "4294967294",
            "4294967295",
            "-0",
            "__proto__",
            "",
        ]
        .into_iter()
        .enumerate()
        .map(|(i, key)| (key.encode_utf16().collect(), JsonValue::Number(i as f64)))
        .collect(),
    );
    let expected = "{\"0\":4,\"2\":2,\"10\":1,\"4294967294\":5,\"a\":0,\"01\":3,\"4294967295\":6,\"-0\":7,\"__proto__\":8,\"\":9}";
    assert_eq!(
        stringify(&value, limits()).unwrap(),
        expected.encode_utf16().collect::<Vec<_>>()
    );
    assert_eq!(
        stringify(&JsonValue::Object(vec![]), limits()).unwrap(),
        [123, 125]
    );
}

#[test]
fn objects_quote_utf16_keys_and_serialize_nested_values() {
    let value = JsonValue::Object(vec![(
        vec![0xd800, 34],
        JsonValue::Array(vec![JsonValue::Object(vec![(
            vec![97],
            JsonValue::Boolean(true),
        )])]),
    )]);
    assert_eq!(
        stringify(&value, limits()).unwrap(),
        "{\"\\ud800\\\"\":[{\"a\":true}]}"
            .encode_utf16()
            .collect::<Vec<_>>()
    );
}

#[test]
fn manually_constructed_duplicate_keys_are_invalid_internal_data() {
    let value = JsonValue::Object(vec![
        (vec![97], JsonValue::Null),
        (vec![97], JsonValue::Boolean(true)),
    ]);
    assert_eq!(
        stringify(&value, limits()).unwrap_err(),
        StringifyError::DuplicateKey
    );
}

#[test]
fn output_and_value_limits_are_inclusive_across_the_entire_tree() {
    let mut bound = limits();
    bound.max_output_units = 3;
    assert_eq!(
        stringify(&JsonValue::Null, bound).unwrap_err(),
        StringifyError::Limit(StringifyLimit::Output)
    );
    bound.max_output_units = 4;
    assert_eq!(
        stringify(&JsonValue::Null, bound).unwrap(),
        [110, 117, 108, 108]
    );
    bound.max_output_units = 7;
    assert_eq!(
        stringify(&JsonValue::String(vec![0xd800]), bound).unwrap_err(),
        StringifyError::Limit(StringifyLimit::Output)
    );
    bound.max_output_units = 8;
    assert!(stringify(&JsonValue::String(vec![0xd800]), bound).is_ok());
    let pair = JsonValue::Array(vec![JsonValue::Null, JsonValue::Null]);
    bound.max_output_units = 10;
    assert_eq!(
        stringify(&pair, bound).unwrap_err(),
        StringifyError::Limit(StringifyLimit::Output)
    );
    bound.max_output_units = 11;
    assert!(stringify(&pair, bound).is_ok());
    bound = limits();
    bound.max_values = 0;
    assert_eq!(
        stringify(&JsonValue::Null, bound).unwrap_err(),
        StringifyError::Limit(StringifyLimit::Values)
    );
    bound.max_values = 1;
    assert!(stringify(&JsonValue::Array(vec![]), bound).is_ok());
    let value = JsonValue::Array(vec![JsonValue::Null, JsonValue::Array(vec![])]);
    bound.max_values = 2;
    assert_eq!(
        stringify(&value, bound).unwrap_err(),
        StringifyError::Limit(StringifyLimit::Values)
    );
    bound.max_values = 3;
    assert!(stringify(&value, bound).is_ok());
}

#[test]
fn depth_limits_count_containers_and_reject_unsupported_configuration() {
    let mut bound = limits();
    bound.max_depth = 0;
    assert!(stringify(&JsonValue::Null, bound).is_ok());
    for value in [JsonValue::Array(vec![]), JsonValue::Object(vec![])] {
        assert_eq!(
            stringify(&value, bound).unwrap_err(),
            StringifyError::Limit(StringifyLimit::Depth)
        );
    }
    bound.max_depth = 1;
    assert!(stringify(&JsonValue::Array(vec![JsonValue::Null]), bound).is_ok());
    assert_eq!(
        stringify(&JsonValue::Array(vec![JsonValue::Object(vec![])]), bound).unwrap_err(),
        StringifyError::Limit(StringifyLimit::Depth)
    );
    let mut value = JsonValue::Null;
    for _ in 0..128 {
        value = JsonValue::Array(vec![value]);
    }
    assert!(stringify(&value, limits()).is_ok());
    value = JsonValue::Array(vec![value]);
    assert_eq!(
        stringify(&value, limits()).unwrap_err(),
        StringifyError::Limit(StringifyLimit::Depth)
    );
    bound.max_depth = 129;
    assert_eq!(
        stringify(&JsonValue::Null, bound).unwrap_err(),
        StringifyError::InvalidLimits
    );
}

#[test]
fn work_bills_values_output_strings_key_search_and_ordering() {
    let object = JsonValue::Object(vec![
        (vec![98], JsonValue::Number(0.0)),
        (vec![97], JsonValue::Number(1.0)),
    ]);
    for (value, work) in [
        (JsonValue::Null, 5),
        (JsonValue::String(vec![0xd800]), 10),
        (
            JsonValue::Array(vec![JsonValue::Null, JsonValue::Boolean(false)]),
            15,
        ),
        // Output13 + values3 + key scans2 + quoted units2 + comparison2 + order2.
        (object, 24),
    ] {
        let mut bound = limits();
        bound.max_work = work - 1;
        assert_eq!(
            stringify(&value, bound).unwrap_err(),
            StringifyError::Limit(StringifyLimit::Work)
        );
        bound.max_work = work;
        assert!(stringify(&value, bound).is_ok());
    }
}
