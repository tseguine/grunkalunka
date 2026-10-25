use super::*;

fn input(guest: &mut JsonGuest, text: &str) {
    let units: Vec<u16> = text.encode_utf16().collect();
    guest
        .input_buffer(units.len())
        .unwrap()
        .copy_from_slice(&units);
}

#[test]
fn parse_and_stringify_return_internal_roots_in_the_current_frame() {
    let mut guest = JsonGuest::default();
    let frame = guest.heap_enter().unwrap();
    input(&mut guest, "null");
    let text = guest.heap_string_from_input(frame).unwrap();
    let value = guest.heap_parse(frame, text).unwrap();
    assert!(matches!(guest.roots[&value], JsonValue::Null));
    let result = guest.heap_stringify(frame, value).unwrap();
    assert_eq!(guest.string_units(result).unwrap(), [110, 117, 108, 108]);
    guest.heap_leave(frame, Some(result)).unwrap();
    assert_eq!(guest.roots.len(), 1);
    assert!(!guest.internal_roots.contains(&result));
    guest.release(result).unwrap();
    assert!(guest.roots.is_empty());
}

#[test]
fn json_operations_preserve_values_order_and_string_units() {
    for (text, expected) in [
        (" [true, null, -0, 3.5] ", "[true,null,0,3.5]"),
        (
            r#"{"b":1,"2":"two","1":"one","b":3}"#,
            r#"{"1":"one","2":"two","b":3}"#,
        ),
        (r#""\ud800\u0000""#, r#""\ud800\u0000""#),
    ] {
        let mut guest = JsonGuest::default();
        let frame = guest.heap_enter().unwrap();
        input(&mut guest, text);
        let text = guest.heap_string_from_input(frame).unwrap();
        let parsed = guest.heap_parse(frame, text).unwrap();
        assert!(guest.output().is_empty());
        let result = guest.heap_stringify(frame, parsed).unwrap();
        assert!(guest.output().is_empty());
        assert_eq!(
            guest.string_units(result).unwrap(),
            expected.encode_utf16().collect::<Vec<_>>()
        );
    }
}

#[test]
fn repeated_operations_collect_unpinned_operands_and_preserve_external_owners() {
    let mut guest = JsonGuest::default();
    input(&mut guest, "external");
    let external = guest.string_from_input().unwrap();
    let frame = guest.heap_enter().unwrap();
    let mut last = external;
    for _ in 0..500 {
        input(&mut guest, "[1,true,null]");
        let text = guest.heap_string_from_input(frame).unwrap();
        let parsed = guest.heap_parse(frame, text).unwrap();
        let result = guest.heap_stringify(frame, parsed).unwrap();
        assert!(text > last && parsed > text && result > parsed);
        assert_eq!(guest.roots.len(), 2);
        last = result;
    }
    guest.heap_abort();
    assert_eq!(guest.roots.len(), 1);
    assert_eq!(
        guest.string_units(external).unwrap(),
        "external".encode_utf16().collect::<Vec<_>>()
    );
}

#[test]
fn operation_errors_preserve_unpinned_operands_input_and_ids() {
    let mut guest = JsonGuest::default();
    let frame = guest.heap_enter().unwrap();
    input(&mut guest, "not JSON");
    let invalid_text = guest.heap_string_from_input(frame).unwrap();
    let next = guest.next_id;
    let original_input = guest.input.clone();
    guest.stringify(invalid_text).unwrap();
    assert_eq!(
        guest.heap_parse(0, invalid_text),
        Err(GuestError::InvalidFrame)
    );
    assert!(guest.output().is_empty());
    assert!(matches!(
        guest.heap_parse(frame, invalid_text),
        Err(GuestError::Parse(_))
    ));
    assert_eq!(
        guest.heap_stringify(frame, 0),
        Err(GuestError::InvalidHandle)
    );
    assert_eq!(guest.roots.len(), 1);
    assert!(guest.roots.contains_key(&invalid_text));
    assert_eq!(guest.input, original_input);
    assert_eq!(guest.next_id, next);
    assert!(guest.output().is_empty());
    guest.next_id = None;
    assert_eq!(
        guest.heap_parse(frame, invalid_text),
        Err(GuestError::HandleExhausted)
    );
    assert_eq!(
        guest.heap_stringify(frame, invalid_text),
        Err(GuestError::HandleExhausted)
    );
    assert!(guest.roots.contains_key(&invalid_text));
}

#[test]
fn nested_operations_transfer_results_and_validate_current_frames_and_kinds() {
    let mut guest = JsonGuest::default();
    let outer = guest.heap_enter().unwrap();
    input(&mut guest, "null");
    let text = guest.heap_string_from_input(outer).unwrap();
    guest.heap_pin(outer, text).unwrap();
    let inner = guest.heap_enter().unwrap();
    assert_eq!(guest.heap_parse(outer, text), Err(GuestError::InvalidFrame));
    assert_eq!(
        guest.heap_stringify(outer, text),
        Err(GuestError::InvalidFrame)
    );
    let parsed = guest.heap_parse(inner, text).unwrap();
    assert_eq!(guest.heap_parse(inner, parsed), Err(GuestError::NotString));
    let result = guest.heap_stringify(inner, parsed).unwrap();
    guest.heap_leave(inner, Some(result)).unwrap();
    assert!(guest.roots.contains_key(&text));
    guest.heap_leave(outer, Some(result)).unwrap();
    assert_eq!(guest.roots.len(), 1);
    assert_eq!(guest.string_units(result).unwrap(), [110, 117, 108, 108]);
}

#[test]
fn full_external_root_quota_does_not_consume_ids_or_owners() {
    let mut guest = JsonGuest::default();
    input(&mut guest, "null");
    let ids: Vec<_> = (0..MAX_ROOTS)
        .map(|_| guest.string_from_input().unwrap())
        .collect();
    let frame = guest.heap_enter().unwrap();
    let next = guest.next_id;
    assert_eq!(guest.heap_parse(frame, ids[0]), Err(GuestError::RootLimit));
    assert_eq!(
        guest.heap_stringify(frame, ids[0]),
        Err(GuestError::RootLimit)
    );
    assert_eq!(guest.next_id, next);
    assert_eq!(guest.roots.len(), MAX_ROOTS);
    assert!(ids.iter().all(|id| guest.roots.contains_key(id)));
    guest.release(ids[1]).unwrap();
    assert!(guest.heap_parse(frame, ids[0]).is_ok());
}

#[test]
fn serialized_string_bound_is_independent_of_input_and_parser_bounds() {
    let mut guest = JsonGuest::default();
    let frame = guest.heap_enter().unwrap();
    let mut units = vec![0xd800; 10_922];
    units.extend_from_slice(&[65, 65]);
    guest
        .input_buffer(units.len())
        .unwrap()
        .copy_from_slice(&units);
    let raw = guest.heap_string_from_input(frame).unwrap();
    let text = guest.heap_stringify(frame, raw).unwrap();
    assert_eq!(guest.string_units(text).unwrap().len(), MAX_OUTPUT_UNITS);
    assert!(matches!(
        guest.heap_parse(frame, text),
        Err(GuestError::Parse(_))
    ));
    assert!(guest.roots.contains_key(&text));
    units.push(65);
    guest
        .input_buffer(units.len())
        .unwrap()
        .copy_from_slice(&units);
    let too_long = guest.heap_string_from_input(frame).unwrap();
    assert!(matches!(
        guest.heap_stringify(frame, too_long),
        Err(GuestError::Stringify(_))
    ));
    assert!(guest.roots.contains_key(&too_long));
    assert!(guest.output().is_empty());
}

#[test]
fn terminal_root_id_is_assigned_once_by_json_operations() {
    let mut guest = JsonGuest::default();
    let frame = guest.heap_enter().unwrap();
    input(&mut guest, "null");
    let text = guest.heap_string_from_input(frame).unwrap();
    guest.next_id = Some(u32::MAX);
    let last = guest.heap_parse(frame, text).unwrap();
    assert_eq!(last, u32::MAX);
    assert_eq!(
        guest.heap_stringify(frame, last),
        Err(GuestError::HandleExhausted)
    );
    guest.heap_leave(frame, Some(last)).unwrap();
    assert!(matches!(guest.roots[&last], JsonValue::Null));
    guest.release(last).unwrap();
}
