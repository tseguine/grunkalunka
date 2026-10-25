//! Bounded JSON parsing and compact serialization with lossless UTF-16 strings.
//!
//! This is an owned data tree, not a JavaScript heap or a source-lowering API.
//! See `docs/json.md` for the resource accounting and integration boundary.

mod stringify;
pub use stringify::{StringifyError, StringifyLimit, StringifyLimits, stringify};

/// Parsed JSON data. Strings and keys retain every UTF-16 code unit.
///
/// Object entries retain first insertion order, with duplicate values replaced.
/// This metadata is not JavaScript property enumeration order: integer index keys
/// must be ordered separately when exposing an object to JavaScript operations.
#[derive(Clone, Debug)]
pub enum JsonValue {
    Null,
    Boolean(bool),
    Number(f64),
    String(Vec<u16>),
    Array(Vec<JsonValue>),
    Object(Vec<(Vec<u16>, JsonValue)>),
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Inclusive length of the input in UTF-16 code units.
    pub max_input_units: usize,
    /// Inclusive container nesting depth, from zero through 128.
    pub max_depth: usize,
    /// All visited values, including values later overwritten by duplicate keys.
    pub max_values: usize,
    /// Input units + visited values + conservative key-comparison charges.
    /// This deterministic quota is not an instruction or wall-clock bound.
    pub max_work: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Limit {
    Input,
    Depth,
    Values,
    Work,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    Syntax,
    Limit(Limit),
    InvalidLimits,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    /// Associated UTF-16 input offset (possibly the beginning of a bad token).
    pub offset: usize,
    pub kind: ErrorKind,
}

/// Parse one complete JSON text, accepting only JSON whitespace around it.
///
/// Resource exhaustion can precede syntax validation; `Limit` does not establish
/// that the input is syntactically valid. No reviver or JavaScript coercion runs.
pub fn parse(text: &[u16], limits: Limits) -> Result<JsonValue, ParseError> {
    if limits.max_depth > 128 {
        return Err(ParseError {
            offset: 0,
            kind: ErrorKind::InvalidLimits,
        });
    }
    if text.len() > limits.max_input_units {
        return Err(ParseError {
            offset: 0,
            kind: ErrorKind::Limit(Limit::Input),
        });
    }
    let remaining_work = limits.max_work.checked_sub(text.len()).ok_or(ParseError {
        offset: 0,
        kind: ErrorKind::Limit(Limit::Work),
    })?;
    let mut parser = Parser {
        text,
        pos: 0,
        limits,
        remaining_values: limits.max_values,
        remaining_work,
    };
    let value = parser.value(0)?;
    parser.space();
    if parser.pos == text.len() {
        Ok(value)
    } else {
        Err(parser.error())
    }
}

struct Parser<'a> {
    text: &'a [u16],
    pos: usize,
    limits: Limits,
    remaining_values: usize,
    remaining_work: usize,
}
impl Parser<'_> {
    fn error(&self) -> ParseError {
        ParseError {
            offset: self.pos,
            kind: ErrorKind::Syntax,
        }
    }
    fn limit(&self, kind: Limit) -> ParseError {
        ParseError {
            offset: self.pos,
            kind: ErrorKind::Limit(kind),
        }
    }
    fn charge(&mut self, work: usize) -> Result<(), ParseError> {
        self.remaining_work = self
            .remaining_work
            .checked_sub(work)
            .ok_or_else(|| self.limit(Limit::Work))?;
        Ok(())
    }
    fn space(&mut self) {
        while matches!(self.text.get(self.pos), Some(9 | 10 | 13 | 32)) {
            self.pos += 1;
        }
    }
    fn take(&mut self, unit: u16) -> bool {
        if self.text.get(self.pos) == Some(&unit) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn value(&mut self, depth: usize) -> Result<JsonValue, ParseError> {
        self.space();
        self.charge(1)?;
        self.remaining_values = self
            .remaining_values
            .checked_sub(1)
            .ok_or_else(|| self.limit(Limit::Values))?;
        let rest = &self.text[self.pos..];
        for (literal, value) in [
            (&[110, 117, 108, 108][..], JsonValue::Null),
            (&[116, 114, 117, 101][..], JsonValue::Boolean(true)),
            (&[102, 97, 108, 115, 101][..], JsonValue::Boolean(false)),
        ] {
            if rest.starts_with(literal) {
                self.pos += literal.len();
                return Ok(value);
            }
        }
        if self.take(91) {
            if depth >= self.limits.max_depth {
                return Err(self.limit(Limit::Depth));
            }
            self.space();
            let mut values = Vec::new();
            if self.take(93) {
                return Ok(JsonValue::Array(values));
            }
            loop {
                values.push(self.value(depth + 1)?);
                self.space();
                if self.take(93) {
                    return Ok(JsonValue::Array(values));
                }
                if !self.take(44) {
                    return Err(self.error());
                }
            }
        }
        if self.take(123) {
            if depth >= self.limits.max_depth {
                return Err(self.limit(Limit::Depth));
            }
            self.space();
            let mut entries: Vec<(Vec<u16>, JsonValue)> = Vec::new();
            if self.take(125) {
                return Ok(JsonValue::Object(entries));
            }
            loop {
                self.space();
                let (consumed, key) = quoted(&self.text[self.pos..]).ok_or_else(|| self.error())?;
                self.pos += consumed;
                self.space();
                if !self.take(58) {
                    return Err(self.error());
                }
                let value = self.value(depth + 1)?;
                let mut found = None;
                for (index, (existing, _)) in entries.iter().enumerate() {
                    self.charge(existing.len().min(key.len()) + 1)?;
                    if existing == &key {
                        found = Some(index);
                        break;
                    }
                }
                if let Some(index) = found {
                    entries[index].1 = value;
                } else {
                    entries.push((key, value));
                }
                self.space();
                if self.take(125) {
                    return Ok(JsonValue::Object(entries));
                }
                if !self.take(44) {
                    return Err(self.error());
                }
            }
        }
        if rest.first() == Some(&34) {
            let (consumed, value) = quoted(rest).ok_or_else(|| self.error())?;
            self.pos += consumed;
            return Ok(JsonValue::String(value));
        }
        let end = self.text[self.pos..]
            .iter()
            .position(|u| matches!(*u, 9 | 10 | 13 | 32 | 44 | 93 | 125))
            .map_or(self.text.len(), |i| self.pos + i);
        let value = number(&self.text[self.pos..end]).ok_or_else(|| self.error())?;
        self.pos = end;
        Ok(JsonValue::Number(value))
    }
}

fn quoted(text: &[u16]) -> Option<(usize, Vec<u16>)> {
    if text.first() != Some(&34) {
        return None;
    }
    let mut i = 1;
    let mut result = Vec::new();
    loop {
        let unit = *text.get(i)?;
        i += 1;
        match unit {
            34 => return Some((i, result)),
            92 => {
                let escape = *text.get(i)?;
                i += 1;
                let decoded = match escape {
                    34 | 47 | 92 => escape,
                    98 => 8,
                    102 => 12,
                    110 => 10,
                    114 => 13,
                    116 => 9,
                    117 => {
                        let mut value = 0;
                        for _ in 0..4 {
                            let digit = match *text.get(i)? {
                                n @ 48..=57 => n - 48,
                                n @ 65..=70 => n - 65 + 10,
                                n @ 97..=102 => n - 97 + 10,
                                _ => return None,
                            };
                            value = value * 16 + digit;
                            i += 1;
                        }
                        value
                    }
                    _ => return None,
                };
                result.push(decoded);
            }
            0..=31 => return None,
            other => result.push(other),
        }
    }
}

fn number(text: &[u16]) -> Option<f64> {
    let mut i = usize::from(text.first() == Some(&45));
    match text.get(i) {
        Some(48) => i += 1,
        Some(49..=57) => {
            i += 1;
            while matches!(text.get(i), Some(48..=57)) {
                i += 1;
            }
        }
        _ => return None,
    }
    if text.get(i) == Some(&46) {
        i += 1;
        let start = i;
        while matches!(text.get(i), Some(48..=57)) {
            i += 1;
        }
        if i == start {
            return None;
        }
    }
    if matches!(text.get(i), Some(69 | 101)) {
        i += 1;
        if matches!(text.get(i), Some(43 | 45)) {
            i += 1;
        }
        let start = i;
        while matches!(text.get(i), Some(48..=57)) {
            i += 1;
        }
        if i == start {
            return None;
        }
    }
    if i != text.len() {
        return None;
    }
    text.iter()
        .map(|unit| *unit as u8 as char)
        .collect::<String>()
        .parse()
        .ok()
}
