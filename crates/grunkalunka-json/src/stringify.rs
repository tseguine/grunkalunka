use crate::JsonValue;

#[derive(Clone, Copy, Debug)]
pub struct StringifyLimits {
    /// Inclusive output length in UTF-16 code units, checked before appending.
    pub max_output_units: usize,
    /// Inclusive container nesting depth, from zero through 128.
    pub max_depth: usize,
    /// Visited values, including containers; object keys are not values.
    pub max_values: usize,
    /// Deterministic accounting for values, code units, key search and ordering.
    /// See `docs/json.md`; this is not an instruction or allocator guarantee.
    pub max_work: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StringifyError {
    /// A manually constructed Object has repeated keys; parsed objects never do.
    DuplicateKey,
    InvalidLimits,
    Limit(StringifyLimit),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StringifyLimit {
    Output,
    Depth,
    Values,
    Work,
}

/// Serialize owned JSON data to compact UTF-16 text with ECMAScript formatting.
///
/// Objects must have unique keys in first insertion order. This data operation
/// does not invoke JavaScript callbacks, getters, `toJSON`, replacers or `space`.
/// Resource failures can precede detection of malformed internal data.
pub fn stringify(value: &JsonValue, limits: StringifyLimits) -> Result<Vec<u16>, StringifyError> {
    if limits.max_depth > 128 {
        return Err(StringifyError::InvalidLimits);
    }
    let mut writer = Writer {
        output: Vec::new(),
        remaining_values: limits.max_values,
        remaining_work: limits.max_work,
        limits,
    };
    writer.value(value, 0)?;
    Ok(writer.output)
}

struct Writer {
    output: Vec<u16>,
    remaining_values: usize,
    remaining_work: usize,
    limits: StringifyLimits,
}

impl Writer {
    fn charge(&mut self, work: usize) -> Result<(), StringifyError> {
        self.remaining_work = self
            .remaining_work
            .checked_sub(work)
            .ok_or(StringifyError::Limit(StringifyLimit::Work))?;
        Ok(())
    }

    fn emit(&mut self, units: &[u16]) -> Result<(), StringifyError> {
        if self
            .output
            .len()
            .checked_add(units.len())
            .is_none_or(|end| end > self.limits.max_output_units)
        {
            return Err(StringifyError::Limit(StringifyLimit::Output));
        }
        self.charge(units.len())?;
        self.output.extend_from_slice(units);
        Ok(())
    }

    fn container(&self, depth: usize, children: usize) -> Result<(), StringifyError> {
        if depth >= self.limits.max_depth {
            return Err(StringifyError::Limit(StringifyLimit::Depth));
        }
        // Every direct child costs at least one value. Check before allocating
        // object ordering metadata or visiting any child.
        if children > self.remaining_values {
            return Err(StringifyError::Limit(StringifyLimit::Values));
        }
        Ok(())
    }

    fn value(&mut self, value: &JsonValue, depth: usize) -> Result<(), StringifyError> {
        self.charge(1)?;
        self.remaining_values = self
            .remaining_values
            .checked_sub(1)
            .ok_or(StringifyError::Limit(StringifyLimit::Values))?;
        match value {
            JsonValue::Null => self.emit(&[110, 117, 108, 108]),
            JsonValue::Boolean(true) => self.emit(&[116, 114, 117, 101]),
            JsonValue::Boolean(false) => self.emit(&[102, 97, 108, 115, 101]),
            JsonValue::Number(n) => {
                if !n.is_finite() {
                    return self.emit(&[110, 117, 108, 108]);
                }
                self.emit(
                    &ryu_js::Buffer::new()
                        .format_finite(*n)
                        .encode_utf16()
                        .collect::<Vec<_>>(),
                )
            }
            JsonValue::String(units) => self.quote(units),
            JsonValue::Array(values) => {
                self.container(depth, values.len())?;
                self.emit(&[91])?;
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        self.emit(&[44])?;
                    }
                    self.value(value, depth + 1)?;
                }
                self.emit(&[93])
            }
            JsonValue::Object(entries) => {
                self.container(depth, entries.len())?;
                let count = entries.len();
                if count > 1 {
                    let levels = (usize::BITS - (count - 1).leading_zeros()) as usize;
                    let ordering = count
                        .checked_mul(levels)
                        .ok_or(StringifyError::Limit(StringifyLimit::Work))?;
                    self.charge(ordering)?;
                }
                let mut ordered = Vec::new();
                for (index, (key, _)) in entries.iter().enumerate() {
                    self.charge(key.len())?;
                    for (previous, _) in &entries[..index] {
                        self.charge(previous.len().min(key.len()) + 1)?;
                        if previous == key {
                            return Err(StringifyError::DuplicateKey);
                        }
                    }
                    ordered.push((array_index(key).map_or((1, 0), |n| (0, n)), &entries[index]));
                }
                ordered.sort_by_key(|(order, _)| *order);
                self.emit(&[123])?;
                for (index, (_, (key, value))) in ordered.into_iter().enumerate() {
                    if index > 0 {
                        self.emit(&[44])?;
                    }
                    self.quote(key)?;
                    self.emit(&[58])?;
                    self.value(value, depth + 1)?;
                }
                self.emit(&[125])
            }
        }
    }

    fn quote(&mut self, units: &[u16]) -> Result<(), StringifyError> {
        self.charge(units.len())?;
        self.emit(&[34])?;
        let mut i = 0;
        while i < units.len() {
            let unit = units[i];
            let escape = match unit {
                8 => Some(98),
                9 => Some(116),
                10 => Some(110),
                12 => Some(102),
                13 => Some(114),
                34 | 92 => Some(unit),
                _ => None,
            };
            if let Some(short) = escape {
                self.emit(&[92, short])?;
            } else if (0xd800..=0xdbff).contains(&unit)
                && units
                    .get(i + 1)
                    .is_some_and(|next| (0xdc00..=0xdfff).contains(next))
            {
                self.emit(&[unit, units[i + 1]])?;
                i += 1;
            } else if unit < 32 || (0xd800..=0xdfff).contains(&unit) {
                self.emit(&format!("\\u{unit:04x}").encode_utf16().collect::<Vec<_>>())?;
            } else {
                self.emit(&[unit])?;
            }
            i += 1;
        }
        self.emit(&[34])
    }
}

fn array_index(key: &[u16]) -> Option<u32> {
    if key.is_empty() || key.len() > 10 || (key.len() > 1 && key[0] == 48) {
        return None;
    }
    let mut value = 0_u32;
    for &digit in key {
        if !(48..=57).contains(&digit) {
            return None;
        }
        value = value.checked_mul(10)?.checked_add(u32::from(digit - 48))?;
    }
    (value < u32::MAX).then_some(value)
}
