//! Core-Wasm entry points. Symbol export attributes are the only unsafe operations.
//! Input/output are initialized Rust-owned buffers; no raw pointer is accepted.
use super::{GuestError, JsonGuest};
use grunkalunka_json::ErrorKind;
use std::sync::Mutex;

#[derive(Default)]
struct State {
    guest: JsonGuest,
    status: u32,
    offset: u32,
}

impl State {
    fn record(&mut self, error: Option<&GuestError>) {
        self.offset = 0;
        self.status = match error {
            None => 0,
            Some(GuestError::InputLimit) => 1,
            Some(GuestError::RootLimit) => 2,
            Some(GuestError::HandleExhausted) => 3,
            Some(GuestError::InvalidHandle) => 4,
            Some(GuestError::Parse(e)) => {
                self.offset = e.offset as u32;
                match e.kind {
                    ErrorKind::Syntax => 5,
                    ErrorKind::Limit(_) => 6,
                    ErrorKind::InvalidLimits => 8,
                }
            }
            Some(GuestError::Stringify(grunkalunka_json::StringifyError::Limit(_))) => 7,
            Some(GuestError::Stringify(_)) => 8,
            Some(GuestError::NotString) => 9,
            Some(GuestError::FrameLimit) => 10,
            Some(GuestError::InvalidFrame) => 11,
            Some(GuestError::NotInternal) => 12,
            Some(GuestError::FrameExhausted) => 13,
        };
    }
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with_state<T>(f: impl FnOnce(&mut State) -> T) -> T {
    let mut guard = STATE.lock().expect("JSON guest lock poisoned");
    f(guard.get_or_insert_with(State::default))
}

#[unsafe(no_mangle)]
pub extern "C" fn gk_heap_parse(frame: u32, text: u32) -> u32 {
    with_state(|state| {
        let result = state.guest.heap_parse(frame, text);
        state.record(result.as_ref().err());
        result.unwrap_or(0)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn gk_heap_stringify(frame: u32, value: u32) -> u32 {
    with_state(|state| {
        let result = state.guest.heap_stringify(frame, value);
        state.record(result.as_ref().err());
        result.unwrap_or(0)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn gk_heap_enter() -> u32 {
    with_state(|state| {
        let result = state.guest.heap_enter();
        state.record(result.as_ref().err());
        result.unwrap_or(0)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn gk_heap_clear(frame: u32) -> u32 {
    with_state(|state| {
        let result = state.guest.heap_clear(frame);
        state.record(result.as_ref().err());
        state.status
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn gk_heap_pin(frame: u32, id: u32) -> u32 {
    with_state(|state| {
        let result = state.guest.heap_pin(frame, id);
        state.record(result.as_ref().err());
        state.status
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn gk_heap_string(frame: u32) -> u32 {
    with_state(|state| {
        let result = state.guest.heap_string_from_input(frame);
        state.record(result.as_ref().err());
        result.unwrap_or(0)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn gk_heap_leave(frame: u32, returned: u32) -> u32 {
    with_state(|state| {
        let result = state
            .guest
            .heap_leave(frame, (returned != 0).then_some(returned));
        state.record(result.as_ref().err());
        state.status
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn gk_heap_abort() -> u32 {
    with_state(|state| {
        state.guest.heap_abort();
        state.record(None);
        state.status
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn gk_json_input_resize(units: u32) -> u32 {
    with_state(|state| {
        let result = state
            .guest
            .input_buffer(units as usize)
            .map(|input| input.as_mut_ptr() as u32);
        state.record(result.as_ref().err());
        result.unwrap_or(0)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn gk_json_parse() -> u32 {
    with_state(|state| {
        let result = state.guest.parse_input();
        state.record(result.as_ref().err());
        result.unwrap_or(0)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn gk_json_string_from_input() -> u32 {
    with_state(|state| {
        let result = state.guest.string_from_input();
        state.record(result.as_ref().err());
        result.unwrap_or(0)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn gk_json_status() -> u32 {
    with_state(|state| state.status)
}

#[unsafe(no_mangle)]
pub extern "C" fn gk_json_error_offset() -> u32 {
    with_state(|state| state.offset)
}

#[unsafe(no_mangle)]
pub extern "C" fn gk_json_input_len() -> u32 {
    with_state(|state| state.guest.input.len() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn gk_json_stringify(id: u32) -> u32 {
    with_state(|state| {
        let result = state.guest.stringify(id).map(|_| ());
        state.record(result.as_ref().err());
        state.status
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn gk_json_string_units(id: u32) -> u32 {
    with_state(|state| {
        let result = state.guest.string_units(id).map(|_| ());
        state.record(result.as_ref().err());
        state.status
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn gk_json_release(id: u32) -> u32 {
    with_state(|state| {
        let result = state.guest.release(id);
        state.record(result.as_ref().err());
        state.status
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn gk_json_output_ptr() -> u32 {
    with_state(|state| state.guest.output.as_ptr() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn gk_json_output_len() -> u32 {
    with_state(|state| state.guest.output.len() as u32)
}
