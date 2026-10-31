use grunkalunka_json::{ErrorKind, JsonValue, Limit, Limits, parse};

fn limits() -> Limits {
    Limits {
        max_input_units: 4096,
        max_depth: 128,
        max_values: 1024,
        max_work: 16384,
    }
}

#[test]
fn parses_json_null() {
    assert!(matches!(
        parse(&"null".encode_utf16().collect::<Vec<_>>(), limits()).unwrap(),
        JsonValue::Null
    ));
}

#[test]
fn rejects_empty_junk_and_trailing_values() {
    for text in ["", "undefined", "nul", "nullx", "null null"] {
        assert!(
            parse(&text.encode_utf16().collect::<Vec<_>>(), limits()).is_err(),
            "{text:?}"
        );
    }
}

#[test]
fn admits_only_json_whitespace_around_null() {
    assert!(matches!(
        parse(&"\t\r\n null ".encode_utf16().collect::<Vec<_>>(), limits()).unwrap(),
        JsonValue::Null
    ));
    assert!(parse(&"\u{a0}null".encode_utf16().collect::<Vec<_>>(), limits()).is_err());
}

#[test]
fn parses_boolean_literals_without_extensions() {
    for (text, expected) in [("true", true), ("false", false), ("\nfalse\t", false)] {
        let value = parse(&text.encode_utf16().collect::<Vec<_>>(), limits()).unwrap();
        assert!(matches!(value,JsonValue::Boolean(actual) if actual==expected));
    }
    for text in ["True", "FALSE", "truex", "false false"] {
        assert!(parse(&text.encode_utf16().collect::<Vec<_>>(), limits()).is_err());
    }
}

#[test]
fn parses_json_numbers_with_binary64_edge_cases() {
    for (text, expected) in [
        ("0", 0.0),
        ("-0", -0.0),
        ("12.5", 12.5),
        ("-2E+3", -2000.0),
        ("1e400", f64::INFINITY),
        ("-1e400", f64::NEG_INFINITY),
        ("1e-4000", 0.0),
        ("5e-324", f64::from_bits(1)),
    ] {
        let value = parse(&text.encode_utf16().collect::<Vec<_>>(), limits()).unwrap();
        let JsonValue::Number(actual) = value else {
            panic!("expected Number")
        };
        assert_eq!(actual.to_bits(), expected.to_bits(), "{text}");
    }
    for text in [
        "01", "-01", "+1", ".5", "1.", "1e", "1e+", "--1", "NaN", "Infinity", "0x1", "1_0",
    ] {
        assert!(
            parse(&text.encode_utf16().collect::<Vec<_>>(), limits()).is_err(),
            "{text}"
        );
    }
}

#[test]
fn parses_json_strings_as_lossless_utf16_code_units() {
    for (text, expected) in [
        ("\"\"", vec![]),
        ("\"abc\"", vec![97, 98, 99]),
        (
            "\"\\\"\\\\\\/\\b\\f\\n\\r\\t\"",
            vec![34, 92, 47, 8, 12, 10, 13, 9],
        ),
        (
            "\"\\u0000\\ud800\\uD834\\uDD1e\"",
            vec![0, 0xd800, 0xd834, 0xdd1e],
        ),
    ] {
        let value = parse(&text.encode_utf16().collect::<Vec<_>>(), limits()).unwrap();
        let JsonValue::String(units) = value else {
            panic!("expected String")
        };
        assert_eq!(units, expected);
    }
    let raw = [34, 0xd800, 34];
    let JsonValue::String(units) = parse(&raw, limits()).unwrap() else {
        panic!("expected String")
    };
    assert_eq!(units, [0xd800]);
    for text in [
        "\"",
        "\"x\n\"",
        "\"\\x20\"",
        "\"\\u123\"",
        "\"\\uZZZZ\"",
        "\"x\"true",
        "\"\\",
    ] {
        assert!(
            parse(&text.encode_utf16().collect::<Vec<_>>(), limits()).is_err(),
            "{text:?}"
        );
    }
}

#[test]
fn parses_nested_arrays_and_rejects_elisions_or_trailing_commas() {
    let value = parse(
        &"[null,true,-0,\"x\",[false]]"
            .encode_utf16()
            .collect::<Vec<_>>(),
        limits(),
    )
    .unwrap();
    let JsonValue::Array(items) = value else {
        panic!("expected Array")
    };
    assert_eq!(items.len(), 5);
    assert!(matches!(items[0], JsonValue::Null));
    assert!(matches!(items[1], JsonValue::Boolean(true)));
    assert!(matches!(items[2],JsonValue::Number(n) if n.to_bits()==(-0.0_f64).to_bits()));
    assert!(matches!(&items[3],JsonValue::String(s) if s==&[120]));
    assert!(matches!(&items[4],JsonValue::Array(a) if matches!(a[0],JsonValue::Boolean(false))));
    assert!(matches!(parse(&[91,93],limits()).unwrap(),JsonValue::Array(a) if a.is_empty()));
    for text in ["[", "[1,]", "[,1]", "[1,,2]", "[1 2]", "[true]null"] {
        assert!(
            parse(&text.encode_utf16().collect::<Vec<_>>(), limits()).is_err(),
            "{text}"
        );
    }
}

#[test]
fn arrays_obey_requested_container_depth() {
    let mut one = limits();
    one.max_depth = 1;
    assert!(parse(&"[null]".encode_utf16().collect::<Vec<_>>(), one).is_ok());
    assert!(parse(&"[[null]]".encode_utf16().collect::<Vec<_>>(), one).is_err());
}

#[test]
fn parses_objects_with_last_duplicate_value_and_first_key_position() {
    let source = "{\"a\":1,\"10\":\"ten\",\"2\":\"two\",\"a\":2,\"\\u0061\":3,\"__proto__\":true}";
    let JsonValue::Object(entries) =
        parse(&source.encode_utf16().collect::<Vec<_>>(), limits()).unwrap()
    else {
        panic!("expected Object")
    };
    assert_eq!(
        entries
            .iter()
            .map(|(k, _)| String::from_utf16(k).unwrap())
            .collect::<Vec<_>>(),
        ["a", "10", "2", "__proto__"]
    );
    assert!(matches!(entries[0].1,JsonValue::Number(n) if n==3.0));
    assert!(matches!(entries[3].1, JsonValue::Boolean(true)));
    assert!(matches!(parse(&[123,125],limits()).unwrap(),JsonValue::Object(v) if v.is_empty()));
}

#[test]
fn parses_nested_objects_and_requires_quoted_keys_colons_and_commas() {
    let value = parse(
        &"{\"a\":[{\"b\":\"\\ud800\"}]}"
            .encode_utf16()
            .collect::<Vec<_>>(),
        limits(),
    )
    .unwrap();
    let JsonValue::Object(root) = value else {
        panic!("expected Object")
    };
    let JsonValue::Array(items) = &root[0].1 else {
        panic!("expected Array")
    };
    let JsonValue::Object(inner) = &items[0] else {
        panic!("expected Object")
    };
    assert!(matches!(&inner[0].1,JsonValue::String(s) if s==&[0xd800]));
    for text in [
        "{",
        "{a:1}",
        "{'a':1}",
        "{\"a\"1}",
        "{\"a\":}",
        "{\"a\":1,}",
        "{\"a\":1 \"b\":2}",
    ] {
        assert!(
            parse(&text.encode_utf16().collect::<Vec<_>>(), limits()).is_err(),
            "{text}"
        );
    }
}

#[test]
fn input_and_value_limits_are_inclusive_and_not_syntax_errors() {
    let mut bound = limits();
    bound.max_input_units = 3;
    assert_eq!(
        parse(&[110, 117, 108, 108], bound).unwrap_err().kind,
        ErrorKind::Limit(Limit::Input)
    );
    bound.max_input_units = 4;
    assert!(parse(&[110, 117, 108, 108], bound).is_ok());
    bound = limits();
    bound.max_values = 0;
    assert_eq!(
        parse(&[110, 117, 108, 108], bound).unwrap_err().kind,
        ErrorKind::Limit(Limit::Values)
    );
    bound.max_values = 1;
    assert!(parse(&[91, 93], bound).is_ok());
    assert_eq!(
        parse(&[91, 110, 117, 108, 108, 93], bound)
            .unwrap_err()
            .kind,
        ErrorKind::Limit(Limit::Values)
    );
    bound.max_values = 2;
    assert!(parse(&[91, 110, 117, 108, 108, 93], bound).is_ok());
    let duplicate = "{\"a\":0,\"a\":1}".encode_utf16().collect::<Vec<_>>();
    assert_eq!(
        parse(&duplicate, bound).unwrap_err().kind,
        ErrorKind::Limit(Limit::Values)
    );
    assert_eq!(
        parse(&[43, 49], limits()).unwrap_err().kind,
        ErrorKind::Syntax
    );
}

#[test]
fn depth_zero_admits_scalars_and_maximum_depth_is_validated() {
    let mut bound = limits();
    bound.max_depth = 0;
    assert!(parse(&[110, 117, 108, 108], bound).is_ok());
    for text in ["[]", "{}"] {
        assert_eq!(
            parse(&text.encode_utf16().collect::<Vec<_>>(), bound)
                .unwrap_err()
                .kind,
            ErrorKind::Limit(Limit::Depth)
        );
    }
    for count in [128, 129] {
        let text = format!("{}null{}", "[".repeat(count), "]".repeat(count))
            .encode_utf16()
            .collect::<Vec<_>>();
        let result = parse(&text, limits());
        if count == 128 {
            assert!(result.is_ok());
        } else {
            assert_eq!(result.unwrap_err().kind, ErrorKind::Limit(Limit::Depth));
        }
    }
    bound = limits();
    bound.max_depth = 129;
    assert_eq!(
        parse(&[110, 117, 108, 108], bound).unwrap_err().kind,
        ErrorKind::InvalidLimits
    );
}

#[test]
fn work_limit_bills_input_values_and_duplicate_key_comparisons() {
    let mut bound = limits();
    bound.max_work = 4;
    assert_eq!(
        parse(&[110, 117, 108, 108], bound).unwrap_err().kind,
        ErrorKind::Limit(Limit::Work)
    );
    bound.max_work = 5;
    assert!(parse(&[110, 117, 108, 108], bound).is_ok());
    let text = "{\"a\":0,\"a\":1}".encode_utf16().collect::<Vec<_>>();
    bound.max_work = text.len() + 3 + 1;
    assert_eq!(
        parse(&text, bound).unwrap_err().kind,
        ErrorKind::Limit(Limit::Work)
    );
    bound.max_work += 1;
    assert!(parse(&text, bound).is_ok());
}

#[test]
fn resource_failure_does_not_certify_valid_syntax() {
    let malformed = [43, 49]; // +1 is not a JSON number.
    let mut bound = limits();
    bound.max_work = 0;
    assert_eq!(
        parse(&malformed, bound).unwrap_err().kind,
        ErrorKind::Limit(Limit::Work)
    );
    bound.max_work = 3;
    bound.max_values = 0;
    assert_eq!(
        parse(&malformed, bound).unwrap_err().kind,
        ErrorKind::Limit(Limit::Values)
    );
    assert_eq!(
        parse(&malformed, limits()).unwrap_err().kind,
        ErrorKind::Syntax
    );
}

#[test]
fn mixed_depth_boundary_parses_without_recursive_native_frames() {
    let mut source = String::new();
    let mut closing = String::new();
    let mut overflow_offset = 0;
    for level in 0..128 {
        if level % 2 == 0 {
            source.push_str("{\"n\":");
            closing.push('}');
        } else {
            source.push('[');
            closing.push(']');
        }
        if level == 127 {
            overflow_offset = source.encode_utf16().count();
        }
    }
    source.push_str("null");
    source.extend(closing.chars().rev());
    let text: Vec<u16> = source.encode_utf16().collect();

    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let value = parse(&text, limits()).unwrap();
            let mut cursor = &value;
            for level in 0..128 {
                cursor = if level % 2 == 0 {
                    let JsonValue::Object(entries) = cursor else {
                        panic!("expected object at {level}")
                    };
                    &entries[0].1
                } else {
                    let JsonValue::Array(items) = cursor else {
                        panic!("expected array at {level}")
                    };
                    &items[0]
                };
            }
            assert!(matches!(cursor, JsonValue::Null));

            let mut bound = limits();
            bound.max_depth = 127;
            let error = parse(&text, bound).unwrap_err();
            assert_eq!(error.kind, ErrorKind::Limit(Limit::Depth));
            assert_eq!(error.offset, overflow_offset);
        })
        .unwrap()
        .join()
        .unwrap();
}
