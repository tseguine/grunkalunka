//! Bounded owned-JSON transport. This is not the compiler's JavaScript heap ABI.
#![cfg_attr(not(target_arch = "wasm32"), forbid(unsafe_code))]

use grunkalunka_json::{JsonValue, Limits, StringifyLimits, parse, stringify};
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq)]
pub enum GuestError {
    InputLimit,
    RootLimit,
    HandleExhausted,
    InvalidHandle,
    Parse(grunkalunka_json::ParseError),
    Stringify(grunkalunka_json::StringifyError),
}

pub const MAX_INPUT_UNITS: usize = 32_768;
pub const MAX_OUTPUT_UNITS: usize = 65_536;
pub const MAX_ROOTS: usize = 64;

pub struct JsonGuest {
    input: Vec<u16>,
    output: Vec<u16>,
    roots: BTreeMap<u32, JsonValue>,
    next_id: Option<u32>,
}

impl Default for JsonGuest {
    fn default() -> Self {
        Self {
            input: Vec::new(),
            output: Vec::new(),
            roots: BTreeMap::new(),
            next_id: Some(1),
        }
    }
}

impl JsonGuest {
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
        self.roots.remove(&id).ok_or(GuestError::InvalidHandle)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

#[cfg(target_arch = "wasm32")]
mod exports;
