use grunkalunka_json_guest::{GuestError, JsonGuest, MAX_INPUT_UNITS, MAX_ROOTS};

#[test]
fn empty_input_can_create_an_owned_string() {
    let mut guest = JsonGuest::default();
    guest.input_buffer(0).unwrap();
    let id = guest.string_from_input().unwrap();
    assert_ne!(id, 0);
    assert_eq!(guest.stringify(id).unwrap(), [34, 34]);
    guest.release(id).unwrap();
}

#[test]
fn raw_string_creation_preserves_code_units_after_input_changes() {
    let mut guest = JsonGuest::default();
    guest
        .input_buffer(3)
        .unwrap()
        .copy_from_slice(&[0xd800, 0, 0xdc00]);
    let id = guest.string_from_input().unwrap();
    guest.input_buffer(1).unwrap()[0] = 88;
    let next = guest.string_from_input().unwrap();
    assert!(next > id);
    assert_eq!(
        guest.stringify(id).unwrap(),
        r#""\ud800\u0000\udc00""#.encode_utf16().collect::<Vec<_>>()
    );
    assert_eq!(guest.stringify(next).unwrap(), [34, 88, 34]);
    let again = guest.string_from_input().unwrap();
    assert!(guest.output().is_empty());
    assert_eq!(guest.string_units(again).unwrap(), [88]);
}

#[test]
fn inspection_copies_string_units_and_rejects_other_roots() {
    let mut guest = JsonGuest::default();
    guest
        .input_buffer(3)
        .unwrap()
        .copy_from_slice(&[65, 0xd800, 0]);
    let raw = guest.string_from_input().unwrap();
    let snapshot = guest.string_units(raw).unwrap().to_vec();
    assert_eq!(snapshot, [65, 0xd800, 0]);
    let json = r#""\udfff""#.encode_utf16().collect::<Vec<_>>();
    guest
        .input_buffer(json.len())
        .unwrap()
        .copy_from_slice(&json);
    let parsed = guest.parse_input().unwrap();
    assert_eq!(guest.string_units(parsed).unwrap(), [0xdfff]);
    guest.input_buffer(1).unwrap()[0] = 48;
    let number = guest.parse_input().unwrap();
    assert_eq!(guest.string_units(raw).unwrap(), snapshot);
    assert_eq!(guest.string_units(number), Err(GuestError::NotString));
    assert!(guest.output().is_empty());
    guest.string_units(raw).unwrap();
    assert_eq!(guest.string_units(0), Err(GuestError::InvalidHandle));
    assert!(guest.output().is_empty());
    guest.release(raw).unwrap();
    assert_eq!(guest.string_units(raw), Err(GuestError::InvalidHandle));
    assert_eq!(snapshot, [65, 0xd800, 0]);
}

#[test]
fn string_roots_share_quotas_and_failed_preparation_leaves_empty_input() {
    let mut guest = JsonGuest::default();
    guest.input_buffer(MAX_INPUT_UNITS).unwrap().fill(0xffff);
    let keep = guest.string_from_input().unwrap();
    assert_eq!(
        guest.string_units(keep).unwrap(),
        vec![0xffff; MAX_INPUT_UNITS]
    );
    assert_eq!(
        guest.input_buffer(MAX_INPUT_UNITS + 1),
        Err(GuestError::InputLimit)
    );
    let empty = guest.string_from_input().unwrap();
    assert!(guest.string_units(empty).unwrap().is_empty());
    guest.input_buffer(1).unwrap()[0] = 48;
    let mut ids = vec![keep, empty];
    for _ in 2..MAX_ROOTS {
        ids.push(guest.parse_input().unwrap());
    }
    guest.string_units(keep).unwrap();
    assert_eq!(guest.string_from_input(), Err(GuestError::RootLimit));
    assert!(guest.output().is_empty());
    assert_eq!(guest.parse_input(), Err(GuestError::RootLimit));
    guest.release(empty).unwrap();
    let next = guest.string_from_input().unwrap();
    assert!(next > *ids.last().unwrap());
    assert_eq!(guest.string_units(empty), Err(GuestError::InvalidHandle));
    assert_eq!(guest.string_units(next).unwrap(), [48]);
    assert_eq!(guest.string_units(keep).unwrap().len(), MAX_INPUT_UNITS);
}

#[test]
fn raw_string_inspection_survives_json_output_expansion_failure() {
    use grunkalunka_json::{StringifyError, StringifyLimit};
    let mut guest = JsonGuest::default();
    guest.input_buffer(12_000).unwrap().fill(0xd800);
    let id = guest.string_from_input().unwrap();
    assert_eq!(
        guest.stringify(id),
        Err(GuestError::Stringify(StringifyError::Limit(
            StringifyLimit::Output
        )))
    );
    assert!(guest.output().is_empty());
    assert_eq!(guest.string_units(id).unwrap(), vec![0xd800; 12_000]);
}

#[test]
fn input_is_initialized_and_bounded() {
    let mut guest = JsonGuest::default();
    assert_eq!(guest.input_buffer(4).unwrap(), [0; 4]);
    guest
        .input_buffer(4)
        .unwrap()
        .copy_from_slice(&[1, 2, 3, 4]);
    assert_eq!(guest.input_buffer(4).unwrap(), [0; 4]);
    assert_eq!(guest.input_buffer(0).unwrap(), []);
    assert_eq!(
        guest.input_buffer(MAX_INPUT_UNITS).unwrap().len(),
        MAX_INPUT_UNITS
    );
    assert_eq!(
        guest.input_buffer(MAX_INPUT_UNITS + 1),
        Err(GuestError::InputLimit)
    );
}

#[test]
fn parse_and_stringify_keep_multiple_owned_roots() {
    let mut guest = JsonGuest::default();
    guest
        .input_buffer(4)
        .unwrap()
        .copy_from_slice(&"null".encode_utf16().collect::<Vec<_>>());
    let null = guest.parse_input().unwrap();
    assert_ne!(null, 0);
    assert_eq!(
        guest.stringify(null).unwrap(),
        "null".encode_utf16().collect::<Vec<_>>()
    );
    let text = r#"{"s":"\ud800","2":true,"1":-0}"#.encode_utf16().collect::<Vec<_>>();
    guest
        .input_buffer(text.len())
        .unwrap()
        .copy_from_slice(&text);
    let object = guest.parse_input().unwrap();
    assert_ne!(object, null);
    assert_eq!(
        guest.stringify(object).unwrap(),
        r#"{"1":0,"2":true,"s":"\ud800"}"#.encode_utf16().collect::<Vec<_>>()
    );
    assert_eq!(
        guest.stringify(null).unwrap(),
        "null".encode_utf16().collect::<Vec<_>>()
    );
}

#[test]
fn live_root_quota_is_inclusive() {
    let mut guest = JsonGuest::default();
    guest.input_buffer(1).unwrap()[0] = 48;
    let mut ids = Vec::new();
    for _ in 0..MAX_ROOTS {
        ids.push(guest.parse_input().unwrap());
    }
    assert_eq!(guest.parse_input(), Err(GuestError::RootLimit));
    for id in ids {
        assert_eq!(guest.stringify(id).unwrap(), [48]);
    }
}

#[test]
fn release_reclaims_quota_without_reusing_ids() {
    let mut guest = JsonGuest::default();
    guest.input_buffer(1).unwrap()[0] = 48;
    let mut ids = Vec::new();
    for _ in 0..MAX_ROOTS {
        ids.push(guest.parse_input().unwrap());
    }
    guest.stringify(ids[0]).unwrap();
    guest.release(ids[0]).unwrap();
    assert!(guest.output().is_empty());
    assert_eq!(guest.release(ids[0]), Err(GuestError::InvalidHandle));
    assert_eq!(guest.stringify(ids[0]), Err(GuestError::InvalidHandle));
    let replacement = guest.parse_input().unwrap();
    assert!(!ids.contains(&replacement));
    assert_eq!(guest.stringify(ids[1]).unwrap(), [48]);
}

#[test]
fn failures_clear_output_and_preserve_earlier_roots() {
    let mut guest = JsonGuest::default();
    guest
        .input_buffer(4)
        .unwrap()
        .copy_from_slice(&[110, 117, 108, 108]);
    let id = guest.parse_input().unwrap();
    guest.stringify(id).unwrap();
    assert_eq!(guest.stringify(0), Err(GuestError::InvalidHandle));
    assert!(guest.output().is_empty());
    guest.stringify(id).unwrap();
    guest.input_buffer(1).unwrap()[0] = 91;
    assert!(matches!(guest.parse_input(), Err(GuestError::Parse(_))));
    assert!(guest.output().is_empty());
    assert_eq!(guest.stringify(id).unwrap(), [110, 117, 108, 108]);
    assert_eq!(
        guest.input_buffer(MAX_INPUT_UNITS + 1),
        Err(GuestError::InputLimit)
    );
    assert!(guest.output().is_empty());
    assert!(matches!(guest.parse_input(), Err(GuestError::Parse(_))));
}

#[test]
fn parse_value_and_depth_quotas_are_preserved() {
    use grunkalunka_json::{ErrorKind, Limit};
    let mut guest = JsonGuest::default();
    for (text, expected) in [
        (format!("[{}0]", "0,".repeat(4094)), None),
        (format!("[{}0]", "0,".repeat(4095)), Some(Limit::Values)),
        (format!("{}0{}", "[".repeat(128), "]".repeat(128)), None),
        (
            format!("{}0{}", "[".repeat(129), "]".repeat(129)),
            Some(Limit::Depth),
        ),
    ] {
        let text = text.encode_utf16().collect::<Vec<_>>();
        guest
            .input_buffer(text.len())
            .unwrap()
            .copy_from_slice(&text);
        match expected {
            None => {
                let id = guest.parse_input().unwrap();
                guest.release(id).unwrap();
            }
            Some(limit) => assert!(
                matches!(guest.parse_input(),Err(GuestError::Parse(e)) if e.kind==ErrorKind::Limit(limit))
            ),
        }
    }
}

#[test]
fn output_quota_does_not_destroy_the_root() {
    use grunkalunka_json::{StringifyError, StringifyLimit};
    let mut guest = JsonGuest::default();
    let text = std::iter::once(34)
        .chain(std::iter::repeat_n(0xd800, 12_000))
        .chain(std::iter::once(34))
        .collect::<Vec<_>>();
    guest
        .input_buffer(text.len())
        .unwrap()
        .copy_from_slice(&text);
    let id = guest.parse_input().unwrap();
    assert_eq!(
        guest.stringify(id),
        Err(GuestError::Stringify(StringifyError::Limit(
            StringifyLimit::Output
        )))
    );
    assert!(guest.output().is_empty());
    guest.release(id).unwrap();
}
