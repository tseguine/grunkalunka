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
pub enum JsonValue {
    Null,
    Boolean(bool),
    Number(f64),
    String(Vec<u16>),
    Array(Vec<JsonValue>),
    Object(Vec<(Vec<u16>, JsonValue)>),
}

// Derived Clone walks owned children recursively and can overflow the native
// stack on manually assembled trees (parsing is separately limited to 128).
// Assemble copies in postorder on the heap while preserving input order.
impl Clone for JsonValue {
    fn clone(&self) -> Self {
        enum Work<'a> {
            Value(&'a JsonValue),
            Array(usize),
            Object(Vec<Vec<u16>>),
        }

        let mut work = vec![Work::Value(self)];
        let mut done = Vec::new();
        while let Some(task) = work.pop() {
            match task {
                Work::Value(value) => match value {
                    JsonValue::Null => done.push(JsonValue::Null),
                    JsonValue::Boolean(value) => done.push(JsonValue::Boolean(*value)),
                    JsonValue::Number(value) => done.push(JsonValue::Number(*value)),
                    JsonValue::String(units) => done.push(JsonValue::String(units.clone())),
                    JsonValue::Array(elements) => {
                        work.push(Work::Array(elements.len()));
                        for element in elements.iter().rev() {
                            work.push(Work::Value(element));
                        }
                    }
                    JsonValue::Object(entries) => {
                        work.push(Work::Object(
                            entries.iter().map(|(key, _)| key.clone()).collect(),
                        ));
                        for (_, value) in entries.iter().rev() {
                            work.push(Work::Value(value));
                        }
                    }
                },
                Work::Array(count) => {
                    let children = done.split_off(done.len() - count);
                    done.push(JsonValue::Array(children));
                }
                Work::Object(keys) => {
                    let children = done.split_off(done.len() - keys.len());
                    done.push(JsonValue::Object(keys.into_iter().zip(children).collect()));
                }
            }
        }
        done.pop().expect("the root JSON value was cloned")
    }
}

// Manually assembled JSON trees can exceed the parser's depth-128 limit.
// Explicit consuming release avoids Rust's recursive implicit drop glue.
// The public enum keeps its by-value pattern matching, so ordinary automatic
// destruction may still recurse; callers with unbounded trees use this method.
impl JsonValue {
    pub fn drop_iteratively(self) {
        let mut pending = vec![self];
        while let Some(value) = pending.pop() {
            match value {
                Self::Array(elements) => pending.extend(elements),
                Self::Object(entries) => {
                    pending.extend(entries.into_iter().map(|(_, value)| value));
                }
                Self::Null | Self::Boolean(_) | Self::Number(_) | Self::String(_) => {}
            }
        }
    }
}

// Derived Debug recursively formats nested JSON. A heap worklist preserves
// the compact and alternate pretty representations without deep Rust calls.
impl std::fmt::Debug for JsonValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        enum Work<'a> {
            Value(&'a JsonValue, usize),
            Entry(&'a [u16], &'a JsonValue, usize),
            Text(&'static str),
            Indent(usize),
            EndContainer(usize),
            EndEntry(usize),
        }

        fn indent(f: &mut std::fmt::Formatter<'_>, level: usize) -> std::fmt::Result {
            for _ in 0..level {
                f.write_str("    ")?;
            }
            Ok(())
        }

        fn units(
            f: &mut std::fmt::Formatter<'_>,
            values: &[u16],
            level: usize,
            pretty: bool,
        ) -> std::fmt::Result {
            if !pretty {
                return write!(f, "{values:?}");
            }
            f.write_str("[")?;
            if !values.is_empty() {
                f.write_str("\n")?;
                for value in values {
                    indent(f, level + 1)?;
                    writeln!(f, "{value},")?;
                }
                indent(f, level)?;
            }
            f.write_str("]")
        }

        let pretty = f.alternate();
        let mut pending = vec![Work::Value(self, 0)];
        while let Some(task) = pending.pop() {
            match task {
                Work::Text(text) => f.write_str(text)?,
                Work::Indent(level) => indent(f, level)?,
                Work::EndContainer(level) => {
                    indent(f, level + 1)?;
                    f.write_str("],\n")?;
                    indent(f, level)?;
                    f.write_str(")")?;
                }
                Work::EndEntry(level) => {
                    f.write_str(",\n")?;
                    indent(f, level)?;
                    f.write_str(")")?;
                }
                Work::Entry(key, value, level) => {
                    if pretty {
                        f.write_str("(\n")?;
                        indent(f, level + 1)?;
                        units(f, key, level + 1, true)?;
                        f.write_str(",\n")?;
                        indent(f, level + 1)?;
                        pending.push(Work::EndEntry(level));
                        pending.push(Work::Value(value, level + 1));
                    } else {
                        f.write_str("(")?;
                        units(f, key, level, false)?;
                        f.write_str(", ")?;
                        pending.push(Work::Text(")"));
                        pending.push(Work::Value(value, level));
                    }
                }
                Work::Value(value, level) => match value {
                    Self::Null => f.write_str("Null")?,
                    Self::Boolean(value) => {
                        if pretty {
                            f.write_str("Boolean(\n")?;
                            indent(f, level + 1)?;
                            writeln!(f, "{value:?},")?;
                            indent(f, level)?;
                            f.write_str(")")?;
                        } else {
                            write!(f, "Boolean({value:?})")?;
                        }
                    }
                    Self::Number(value) => {
                        if pretty {
                            f.write_str("Number(\n")?;
                            indent(f, level + 1)?;
                            writeln!(f, "{value:?},")?;
                            indent(f, level)?;
                            f.write_str(")")?;
                        } else {
                            write!(f, "Number({value:?})")?;
                        }
                    }
                    Self::String(value) => {
                        if pretty {
                            f.write_str("String(\n")?;
                            indent(f, level + 1)?;
                            units(f, value, level + 1, true)?;
                            f.write_str(",\n")?;
                            indent(f, level)?;
                            f.write_str(")")?;
                        } else {
                            f.write_str("String(")?;
                            units(f, value, level, false)?;
                            f.write_str(")")?;
                        }
                    }
                    Self::Array(elements) => {
                        if pretty {
                            f.write_str("Array(\n")?;
                            indent(f, level + 1)?;
                            f.write_str("[")?;
                            if !elements.is_empty() {
                                f.write_str("\n")?;
                                pending.push(Work::EndContainer(level));
                                for element in elements.iter().rev() {
                                    pending.push(Work::Text(",\n"));
                                    pending.push(Work::Value(element, level + 2));
                                    pending.push(Work::Indent(level + 2));
                                }
                            } else {
                                f.write_str("],\n")?;
                                indent(f, level)?;
                                f.write_str(")")?;
                            }
                        } else {
                            f.write_str("Array([")?;
                            pending.push(Work::Text("])"));
                            for (index, element) in elements.iter().enumerate().rev() {
                                if index + 1 < elements.len() {
                                    pending.push(Work::Text(", "));
                                }
                                pending.push(Work::Value(element, level));
                            }
                        }
                    }
                    Self::Object(entries) => {
                        if pretty {
                            f.write_str("Object(\n")?;
                            indent(f, level + 1)?;
                            f.write_str("[")?;
                            if !entries.is_empty() {
                                f.write_str("\n")?;
                                pending.push(Work::EndContainer(level));
                                for (key, value) in entries.iter().rev() {
                                    pending.push(Work::Text(",\n"));
                                    pending.push(Work::Entry(key, value, level + 2));
                                    pending.push(Work::Indent(level + 2));
                                }
                            } else {
                                f.write_str("],\n")?;
                                indent(f, level)?;
                                f.write_str(")")?;
                            }
                        } else {
                            f.write_str("Object([")?;
                            pending.push(Work::Text("])"));
                            for (index, (key, value)) in entries.iter().enumerate().rev() {
                                if index + 1 < entries.len() {
                                    pending.push(Work::Text(", "));
                                }
                                pending.push(Work::Entry(key, value, level));
                            }
                        }
                    }
                },
            }
        }
        Ok(())
    }
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
    fn object_key(&mut self) -> Result<Vec<u16>, ParseError> {
        self.space();
        let (consumed, key) = quoted(&self.text[self.pos..]).ok_or_else(|| self.error())?;
        self.pos += consumed;
        self.space();
        if !self.take(58) {
            return Err(self.error());
        }
        Ok(key)
    }

    fn value(&mut self, depth: usize) -> Result<JsonValue, ParseError> {
        // Keep incomplete containers on the heap, not the native call stack.
        // The stack length is also the number of active nonempty containers.
        enum Frame {
            Array(Vec<JsonValue>),
            Object {
                entries: Vec<(Vec<u16>, JsonValue)>,
                key: Vec<u16>,
            },
        }

        let mut frames = Vec::new();
        let mut depth = depth;
        loop {
            self.space();
            self.charge(1)?;
            self.remaining_values = self
                .remaining_values
                .checked_sub(1)
                .ok_or_else(|| self.limit(Limit::Values))?;
            let rest = &self.text[self.pos..];
            let literal = [
                (&[110, 117, 108, 108][..], JsonValue::Null),
                (&[116, 114, 117, 101][..], JsonValue::Boolean(true)),
                (&[102, 97, 108, 115, 101][..], JsonValue::Boolean(false)),
            ]
            .into_iter()
            .find(|(token, _)| rest.starts_with(token));
            let mut value = if let Some((token, value)) = literal {
                self.pos += token.len();
                value
            } else if self.take(91) {
                if depth >= self.limits.max_depth {
                    return Err(self.limit(Limit::Depth));
                }
                self.space();
                if self.take(93) {
                    JsonValue::Array(Vec::new())
                } else {
                    frames.push(Frame::Array(Vec::new()));
                    depth += 1;
                    continue;
                }
            } else if self.take(123) {
                if depth >= self.limits.max_depth {
                    return Err(self.limit(Limit::Depth));
                }
                self.space();
                if self.take(125) {
                    JsonValue::Object(Vec::new())
                } else {
                    let key = self.object_key()?;
                    frames.push(Frame::Object {
                        entries: Vec::new(),
                        key,
                    });
                    depth += 1;
                    continue;
                }
            } else if rest.first() == Some(&34) {
                let (consumed, value) = quoted(rest).ok_or_else(|| self.error())?;
                self.pos += consumed;
                JsonValue::String(value)
            } else {
                let end = self.text[self.pos..]
                    .iter()
                    .position(|u| matches!(*u, 9 | 10 | 13 | 32 | 44 | 93 | 125))
                    .map_or(self.text.len(), |i| self.pos + i);
                let value = number(&self.text[self.pos..end]).ok_or_else(|| self.error())?;
                self.pos = end;
                JsonValue::Number(value)
            };

            // Complete containers from the innermost outward. A comma
            // suspends this reduction so the next loop parses a sibling.
            loop {
                let Some(frame) = frames.last_mut() else {
                    return Ok(value);
                };
                match frame {
                    Frame::Array(values) => {
                        values.push(value);
                        self.space();
                        if self.take(93) {
                            let Some(Frame::Array(values)) = frames.pop() else {
                                unreachable!("array frame")
                            };
                            value = JsonValue::Array(values);
                            depth -= 1;
                        } else if self.take(44) {
                            break;
                        } else {
                            return Err(self.error());
                        }
                    }
                    Frame::Object { entries, key } => {
                        let key = std::mem::take(key);
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
                            let Some(Frame::Object { entries, .. }) = frames.pop() else {
                                unreachable!("object frame")
                            };
                            value = JsonValue::Object(entries);
                            depth -= 1;
                        } else if self.take(44) {
                            let next_key = self.object_key()?;
                            let Some(Frame::Object { key, .. }) = frames.last_mut() else {
                                unreachable!("object frame")
                            };
                            *key = next_key;
                            break;
                        } else {
                            return Err(self.error());
                        }
                    }
                }
            }
        }
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
