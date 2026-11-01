//! Bounded owned-JSON transport. This is not the compiler's JavaScript heap ABI.
#![cfg_attr(not(target_arch = "wasm32"), forbid(unsafe_code))]

use grunkalunka_json::{JsonValue, Limits, StringifyLimits, parse, stringify};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
mod heap_json_tests;

#[derive(Debug, PartialEq, Eq)]
pub enum GuestError {
    InputLimit,
    RootLimit,
    HandleExhausted,
    InvalidHandle,
    NotString,
    InvalidFrame,
    FrameExhausted,
    NotInternal,
    FrameLimit,
    Parse(grunkalunka_json::ParseError),
    Stringify(grunkalunka_json::StringifyError),
}

pub const MAX_INPUT_UNITS: usize = 32_768;
pub const MAX_OUTPUT_UNITS: usize = 65_536;
pub const MAX_ROOTS: usize = 64;
pub const MAX_HEAP_FRAMES: usize = 128;

pub struct JsonGuest {
    input: Vec<u16>,
    output: Vec<u16>,
    roots: BTreeMap<u32, JsonValue>,
    next_id: Option<u32>,
    frames: Vec<RootFrame>,
    next_frame_id: Option<u32>,
    internal_roots: BTreeSet<u32>,
}

struct RootFrame {
    id: u32,
    pins: BTreeSet<u32>,
}

impl Default for JsonGuest {
    fn default() -> Self {
        Self {
            input: Vec::new(),
            output: Vec::new(),
            roots: BTreeMap::new(),
            next_id: Some(1),
            frames: Vec::new(),
            next_frame_id: Some(1),
            internal_roots: BTreeSet::new(),
        }
    }
}

impl JsonGuest {
    /// Parse a live String root into a temporary owned JSON tree.
    pub fn heap_parse(&mut self, frame: u32, text: u32) -> Result<u32, GuestError> {
        let id = self.heap_allocation_id(frame)?;
        let value = self.roots.get(&text).ok_or(GuestError::InvalidHandle)?;
        let JsonValue::String(units) = value else {
            return Err(GuestError::NotString);
        };
        let value = parse(
            units,
            Limits {
                max_input_units: MAX_INPUT_UNITS,
                max_depth: 128,
                max_values: 4096,
                max_work: 1_048_576,
            },
        )
        .map_err(GuestError::Parse)?;
        self.heap_insert(id, value)
    }

    /// Serialize a live JSON root into a temporary raw String root.
    pub fn heap_stringify(&mut self, frame: u32, value: u32) -> Result<u32, GuestError> {
        let id = self.heap_allocation_id(frame)?;
        self.stringify(value)?;
        let units = std::mem::take(&mut self.output);
        self.heap_insert(id, JsonValue::String(units))
    }

    fn heap_allocation_id(&mut self, frame: u32) -> Result<u32, GuestError> {
        self.output.clear();
        if self.frames.last().is_none_or(|top| top.id != frame) {
            return Err(GuestError::InvalidFrame);
        }
        self.next_id.ok_or(GuestError::HandleExhausted)
    }

    fn heap_insert(&mut self, id: u32, value: JsonValue) -> Result<u32, GuestError> {
        self.collect_internal_roots();
        if self.roots.len() >= MAX_ROOTS {
            return Err(GuestError::RootLimit);
        }
        self.roots.insert(id, value);
        self.internal_roots.insert(id);
        self.next_id = id.checked_add(1);
        Ok(id)
    }

    /// Finish the current frame, transferring an optional internal result.
    pub fn heap_leave(&mut self, frame: u32, returned: Option<u32>) -> Result<(), GuestError> {
        self.output.clear();
        if self.frames.last().is_none_or(|top| top.id != frame) {
            return Err(GuestError::InvalidFrame);
        }
        if let Some(id) = returned {
            if !self.roots.contains_key(&id) {
                return Err(GuestError::InvalidHandle);
            }
            if !self.internal_roots.contains(&id) {
                return Err(GuestError::NotInternal);
            }
        }
        self.frames.pop();
        if let Some(id) = returned {
            if let Some(parent) = self.frames.last_mut() {
                parent.pins.insert(id);
            } else {
                self.internal_roots.remove(&id);
            }
        }
        self.collect_internal_roots();
        Ok(())
    }

    /// Discard all temporary frames and roots, preserving external owners.
    pub fn heap_abort(&mut self) {
        self.output.clear();
        self.frames.clear();
        self.collect_internal_roots();
    }

    /// Protect a live internal root in the current frame.
    pub fn heap_pin(&mut self, frame: u32, id: u32) -> Result<(), GuestError> {
        self.output.clear();
        let top = self
            .frames
            .last_mut()
            .filter(|top| top.id == frame)
            .ok_or(GuestError::InvalidFrame)?;
        if !self.roots.contains_key(&id) {
            return Err(GuestError::InvalidHandle);
        }
        if !self.internal_roots.contains(&id) {
            return Err(GuestError::NotInternal);
        }
        top.pins.insert(id);
        Ok(())
    }

    /// Construct a temporary String in the current frame.
    pub fn heap_string_from_input(&mut self, frame: u32) -> Result<u32, GuestError> {
        self.output.clear();
        if self.frames.last().is_none_or(|top| top.id != frame) {
            return Err(GuestError::InvalidFrame);
        }
        self.next_id.ok_or(GuestError::HandleExhausted)?;
        self.collect_internal_roots();
        let id = self.string_from_input()?;
        self.internal_roots.insert(id);
        Ok(id)
    }

    fn collect_internal_roots(&mut self) {
        let protected: BTreeSet<_> = self
            .frames
            .iter()
            .flat_map(|frame| frame.pins.iter().copied())
            .collect();
        for id in &self.internal_roots {
            if !protected.contains(id) {
                if let Some(value) = self.roots.remove(id) {
                    value.drop_iteratively();
                }
            }
        }
        self.internal_roots.retain(|id| self.roots.contains_key(id));
    }

    /// Enter a checked temporary-root frame.
    pub fn heap_enter(&mut self) -> Result<u32, GuestError> {
        self.output.clear();
        if self.frames.len() >= MAX_HEAP_FRAMES {
            return Err(GuestError::FrameLimit);
        }
        let id = self.next_frame_id.ok_or(GuestError::FrameExhausted)?;
        self.frames.push(RootFrame {
            id,
            pins: BTreeSet::new(),
        });
        self.next_frame_id = id.checked_add(1);
        Ok(id)
    }

    /// Clear pins only for the current frame token.
    pub fn heap_clear(&mut self, frame: u32) -> Result<(), GuestError> {
        self.output.clear();
        let top = self
            .frames
            .last_mut()
            .filter(|top| top.id == frame)
            .ok_or(GuestError::InvalidFrame)?;
        top.pins.clear();
        Ok(())
    }

    /// Prepare initialized input units. Clears prior input and output, also on error.
    pub fn input_buffer(&mut self, units: usize) -> Result<&mut [u16], GuestError> {
        self.output.clear();
        self.input.clear();
        if units > MAX_INPUT_UNITS {
            return Err(GuestError::InputLimit);
        }
        self.input.resize(units, 0);
        Ok(&mut self.input)
    }

    pub fn output(&self) -> &[u16] {
        &self.output
    }

    /// Create an independent owned String root from the current raw UTF16 input.
    pub fn string_from_input(&mut self) -> Result<u32, GuestError> {
        self.output.clear();
        if self.roots.len() >= MAX_ROOTS {
            return Err(GuestError::RootLimit);
        }
        let id = self.next_id.ok_or(GuestError::HandleExhausted)?;
        self.roots.insert(id, JsonValue::String(self.input.clone()));
        self.next_id = id.checked_add(1);
        Ok(id)
    }

    /// Copy the raw units of a live String root into the output buffer.
    pub fn string_units(&mut self, id: u32) -> Result<&[u16], GuestError> {
        self.output.clear();
        let value = self.roots.get(&id).ok_or(GuestError::InvalidHandle)?;
        let JsonValue::String(units) = value else {
            return Err(GuestError::NotString);
        };
        self.output.extend_from_slice(units);
        Ok(&self.output)
    }

    /// Parse the current input into an independent owned root with a nonreused ID.
    /// Failure leaves all existing roots intact and clears the last output.
    pub fn parse_input(&mut self) -> Result<u32, GuestError> {
        self.output.clear();
        if self.roots.len() >= MAX_ROOTS {
            return Err(GuestError::RootLimit);
        }
        let id = self.next_id.ok_or(GuestError::HandleExhausted)?;
        let value = parse(
            &self.input,
            Limits {
                max_input_units: MAX_INPUT_UNITS,
                max_depth: 128,
                max_values: 4096,
                max_work: 1_048_576,
            },
        )
        .map_err(GuestError::Parse)?;
        self.roots.insert(id, value);
        self.next_id = id.checked_add(1);
        Ok(id)
    }

    /// Serialize one live root. The next mutating call invalidates this output.
    pub fn stringify(&mut self, id: u32) -> Result<&[u16], GuestError> {
        self.output.clear();
        let value = self.roots.get(&id).ok_or(GuestError::InvalidHandle)?;
        self.output = stringify(
            value,
            StringifyLimits {
                max_output_units: MAX_OUTPUT_UNITS,
                max_depth: 128,
                max_values: 4096,
                max_work: 1_048_576,
            },
        )
        .map_err(GuestError::Stringify)?;
        Ok(&self.output)
    }

    /// Drop a root and reclaim its live quota. Its ID is never assigned again.
    pub fn release(&mut self, id: u32) -> Result<(), GuestError> {
        self.output.clear();
        self.roots
            .remove(&id)
            .ok_or(GuestError::InvalidHandle)?
            .drop_iteratively();
        self.internal_roots.remove(&id);
        for frame in &mut self.frames {
            frame.pins.remove(&id);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_frame_token_never_reuses_after_leave_or_abort() {
        let mut guest = JsonGuest {
            next_frame_id: Some(u32::MAX),
            ..JsonGuest::default()
        };
        let frame = guest.heap_enter().unwrap();
        assert_eq!(frame, u32::MAX);
        assert_eq!(guest.heap_enter(), Err(GuestError::FrameExhausted));
        guest.heap_clear(frame).unwrap();
        guest.heap_leave(frame, None).unwrap();
        guest.heap_abort();
        assert_eq!(guest.heap_enter(), Err(GuestError::FrameExhausted));
    }

    #[test]
    fn exhausted_root_ids_do_not_collect_existing_internal_values() {
        let mut guest = JsonGuest {
            next_id: Some(u32::MAX),
            ..JsonGuest::default()
        };
        let frame = guest.heap_enter().unwrap();
        let last = guest.heap_string_from_input(frame).unwrap();
        assert_eq!(last, u32::MAX);
        assert_eq!(
            guest.heap_string_from_input(frame),
            Err(GuestError::HandleExhausted)
        );
        assert_eq!(guest.string_units(last).unwrap(), []);
        guest.heap_leave(frame, Some(last)).unwrap();
        assert_eq!(guest.string_units(last).unwrap(), []);
    }

    #[test]
    fn explicit_release_does_not_accumulate_internal_metadata_or_stale_pins() {
        let mut guest = JsonGuest::default();
        let outer = guest.heap_enter().unwrap();
        let root = guest.heap_string_from_input(outer).unwrap();
        guest.heap_pin(outer, root).unwrap();
        let inner = guest.heap_enter().unwrap();
        guest.heap_pin(inner, root).unwrap();
        guest.release(root).unwrap();
        assert!(guest.internal_roots.is_empty());
        assert!(guest.frames.iter().all(|frame| frame.pins.is_empty()));
        for _ in 0..1000 {
            let root = guest.heap_string_from_input(inner).unwrap();
            guest.heap_pin(inner, root).unwrap();
            guest.release(root).unwrap();
            assert!(guest.internal_roots.is_empty());
            assert!(guest.frames.iter().all(|frame| frame.pins.is_empty()));
        }
    }

    #[test]
    fn terminal_handle_is_valid_once_and_never_wraps() {
        let mut guest = JsonGuest {
            next_id: Some(u32::MAX),
            ..JsonGuest::default()
        };
        guest.input_buffer(1).unwrap()[0] = 48;
        let last = guest.parse_input().unwrap();
        assert_eq!(last, u32::MAX);
        guest.stringify(last).unwrap();
        assert_eq!(guest.parse_input(), Err(GuestError::HandleExhausted));
        assert!(guest.output().is_empty());
        assert_eq!(guest.stringify(last).unwrap(), [48]);
        guest.release(last).unwrap();
        assert_eq!(guest.parse_input(), Err(GuestError::HandleExhausted));
        assert_eq!(guest.stringify(last), Err(GuestError::InvalidHandle));
    }

    #[test]
    fn strings_and_parsed_values_share_terminal_id_exhaustion() {
        let mut guest = JsonGuest {
            next_id: Some(u32::MAX - 1),
            ..JsonGuest::default()
        };
        guest.input_buffer(1).unwrap()[0] = 48;
        let string = guest.string_from_input().unwrap();
        let number = guest.parse_input().unwrap();
        assert_eq!((string, number), (u32::MAX - 1, u32::MAX));
        assert_eq!(guest.string_from_input(), Err(GuestError::HandleExhausted));
        assert!(guest.output().is_empty());
        assert_eq!(guest.string_units(string).unwrap(), [48]);
        assert_eq!(guest.string_units(number), Err(GuestError::NotString));
        guest.release(string).unwrap();
        assert_eq!(guest.string_from_input(), Err(GuestError::HandleExhausted));
        assert_eq!(guest.parse_input(), Err(GuestError::HandleExhausted));
        assert_eq!(guest.string_units(string), Err(GuestError::InvalidHandle));
        assert_eq!(guest.stringify(number).unwrap(), [48]);
    }
}

#[cfg(target_arch = "wasm32")]
mod exports;
