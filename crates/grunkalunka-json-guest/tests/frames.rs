use grunkalunka_json_guest::{GuestError, JsonGuest, MAX_HEAP_FRAMES, MAX_ROOTS};

#[test]
fn frame_tokens_are_unique_and_only_the_current_frame_can_be_changed() {
    let mut guest = JsonGuest::default();
    let outer = guest.heap_enter().unwrap();
    let inner = guest.heap_enter().unwrap();
    assert_ne!(outer, 0);
    assert!(inner > outer);
    assert_eq!(guest.heap_clear(outer), Err(GuestError::InvalidFrame));
    assert_eq!(guest.heap_clear(0), Err(GuestError::InvalidFrame));
    guest.heap_clear(inner).unwrap();
}

#[test]
fn internal_allocations_reclaim_unprotected_strings_and_preserve_external_roots() {
    let mut guest = JsonGuest::default();
    guest.input_buffer(1).unwrap()[0] = 65;
    let external = guest.string_from_input().unwrap();
    let frame = guest.heap_enter().unwrap();
    guest
        .input_buffer(3)
        .unwrap()
        .copy_from_slice(&[0xd800, 0, 0xdc00]);
    let first = guest.heap_string_from_input(frame).unwrap();
    assert_eq!(guest.string_units(first).unwrap(), [0xd800, 0, 0xdc00]);
    let mut previous = first;
    for unit in 0..100 {
        guest.input_buffer(1).unwrap()[0] = unit;
        let next = guest.heap_string_from_input(frame).unwrap();
        assert!(next > previous);
        assert_eq!(guest.string_units(previous), Err(GuestError::InvalidHandle));
        assert_eq!(guest.string_units(next).unwrap(), [unit]);
        assert_eq!(guest.string_units(external).unwrap(), [65]);
        previous = next;
    }
}

#[test]
fn pins_protect_parent_and_current_aliases_and_can_be_refreshed_without_collection() {
    let mut guest = JsonGuest::default();
    let outer = guest.heap_enter().unwrap();
    guest.input_buffer(1).unwrap()[0] = 65;
    let parent = guest.heap_string_from_input(outer).unwrap();
    guest.heap_pin(outer, parent).unwrap();
    let inner = guest.heap_enter().unwrap();
    guest.input_buffer(1).unwrap()[0] = 66;
    let child = guest.heap_string_from_input(inner).unwrap();
    guest.heap_pin(inner, child).unwrap();
    guest.heap_pin(inner, parent).unwrap();
    assert_eq!(guest.heap_pin(outer, parent), Err(GuestError::InvalidFrame));
    for _ in 0..100 {
        guest.heap_clear(inner).unwrap();
        guest.heap_pin(inner, child).unwrap();
        guest.heap_string_from_input(inner).unwrap();
        assert_eq!(guest.string_units(parent).unwrap(), [65]);
        assert_eq!(guest.string_units(child).unwrap(), [66]);
    }
    guest.heap_clear(inner).unwrap();
    guest.heap_string_from_input(inner).unwrap();
    assert_eq!(guest.string_units(child), Err(GuestError::InvalidHandle));
    assert_eq!(guest.string_units(parent).unwrap(), [65]);
}

#[test]
fn pins_reject_external_and_missing_roots_without_changing_protected_values() {
    let mut guest = JsonGuest::default();
    let external = guest.string_from_input().unwrap();
    let frame = guest.heap_enter().unwrap();
    let internal = guest.heap_string_from_input(frame).unwrap();
    guest.heap_pin(frame, internal).unwrap();
    assert_eq!(
        guest.heap_pin(frame, external),
        Err(GuestError::NotInternal)
    );
    assert!(guest.output().is_empty());
    assert_eq!(guest.heap_pin(frame, 0), Err(GuestError::InvalidHandle));
    guest.heap_string_from_input(frame).unwrap();
    assert_eq!(guest.string_units(internal).unwrap(), []);
    assert_eq!(guest.string_units(external).unwrap(), []);
}

#[test]
fn leaving_transfers_a_result_to_its_parent_then_to_external_ownership() {
    let mut guest = JsonGuest::default();
    let outer = guest.heap_enter().unwrap();
    guest.input_buffer(1).unwrap()[0] = 65;
    let parent = guest.heap_string_from_input(outer).unwrap();
    guest.heap_pin(outer, parent).unwrap();
    let inner = guest.heap_enter().unwrap();
    guest.input_buffer(1).unwrap()[0] = 66;
    let result = guest.heap_string_from_input(inner).unwrap();
    guest.heap_leave(inner, Some(result)).unwrap();
    assert_eq!(guest.heap_clear(inner), Err(GuestError::InvalidFrame));
    for _ in 0..100 {
        guest.heap_string_from_input(outer).unwrap();
        assert_eq!(guest.string_units(result).unwrap(), [66]);
        assert_eq!(guest.string_units(parent).unwrap(), [65]);
    }
    guest.heap_leave(outer, Some(result)).unwrap();
    assert_eq!(guest.string_units(parent), Err(GuestError::InvalidHandle));
    let next = guest.heap_enter().unwrap();
    assert!(next > inner);
    for _ in 0..100 {
        guest.heap_string_from_input(next).unwrap();
    }
    guest.heap_abort();
    assert_eq!(guest.string_units(result).unwrap(), [66]);
    assert_eq!(guest.heap_clear(next), Err(GuestError::InvalidFrame));
    guest.release(result).unwrap();
    assert_eq!(guest.string_units(result), Err(GuestError::InvalidHandle));
}

#[test]
fn failed_leave_preserves_frames_roots_and_input_and_abort_preserves_external_owners() {
    let mut guest = JsonGuest::default();
    guest.input_buffer(1).unwrap()[0] = 88;
    let external = guest.string_from_input().unwrap();
    let outer = guest.heap_enter().unwrap();
    let keep = guest.heap_string_from_input(outer).unwrap();
    guest.heap_pin(outer, keep).unwrap();
    let inner = guest.heap_enter().unwrap();
    for (frame, result, expected) in [
        (outer, None, GuestError::InvalidFrame),
        (inner, Some(0), GuestError::InvalidHandle),
        (inner, Some(external), GuestError::NotInternal),
    ] {
        guest.string_units(keep).unwrap();
        assert_eq!(guest.heap_leave(frame, result), Err(expected));
        assert!(guest.output().is_empty());
        assert_eq!(guest.heap_clear(outer), Err(GuestError::InvalidFrame));
        assert_eq!(guest.string_units(keep).unwrap(), [88]);
    }
    let child = guest.heap_string_from_input(inner).unwrap();
    assert_eq!(guest.string_units(child).unwrap(), [88]);
    guest.heap_leave(inner, None).unwrap();
    assert_eq!(guest.string_units(child), Err(GuestError::InvalidHandle));
    guest.heap_abort();
    guest.heap_abort();
    assert_eq!(guest.string_units(keep), Err(GuestError::InvalidHandle));
    assert_eq!(guest.string_units(external).unwrap(), [88]);
    let new_frame = guest.heap_enter().unwrap();
    assert!(new_frame > inner);
    let after = guest.heap_string_from_input(new_frame).unwrap();
    assert!(after > child);
    assert_eq!(guest.string_units(after).unwrap(), [88]);
}

#[test]
fn frame_capacity_errors_preserve_the_current_frame_and_abort_reclaims_capacity() {
    let mut guest = JsonGuest::default();
    let mut frames = Vec::new();
    for _ in 0..MAX_HEAP_FRAMES {
        frames.push(guest.heap_enter().unwrap());
    }
    let last = *frames.last().unwrap();
    assert_eq!(guest.heap_enter(), Err(GuestError::FrameLimit));
    guest.heap_clear(last).unwrap();
    guest.heap_abort();
    let next = guest.heap_enter().unwrap();
    assert!(next > last);
    assert_eq!(guest.heap_clear(last), Err(GuestError::InvalidFrame));
}

#[test]
fn internal_and_external_roots_share_the_existing_quota_without_losing_live_values() {
    let mut guest = JsonGuest::default();
    guest.input_buffer(1).unwrap()[0] = 48;
    let mut external = Vec::new();
    for _ in 0..MAX_ROOTS - 1 {
        external.push(guest.parse_input().unwrap());
    }
    let frame = guest.heap_enter().unwrap();
    let held = guest.heap_string_from_input(frame).unwrap();
    guest.heap_pin(frame, held).unwrap();
    assert_eq!(
        guest.heap_string_from_input(frame),
        Err(GuestError::RootLimit)
    );
    assert!(guest.output().is_empty());
    assert_eq!(guest.string_units(held).unwrap(), [48]);
    guest.heap_clear(frame).unwrap();
    let next = guest.heap_string_from_input(frame).unwrap();
    assert!(next > held);
    assert_eq!(guest.string_units(held), Err(GuestError::InvalidHandle));
    guest.heap_leave(frame, Some(next)).unwrap();
    guest.heap_abort();
    assert_eq!(guest.string_units(next).unwrap(), [48]);
    for root in external {
        assert_eq!(guest.stringify(root).unwrap(), [48]);
    }
}
