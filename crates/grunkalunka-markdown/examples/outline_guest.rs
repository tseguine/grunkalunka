//! Bounded document input for a private component adapter, not a public ABI.
use grunkalunka_markdown::{OutlineEntry, outline};
use std::cell::RefCell;

const INPUT_LIMIT: usize = 4096;
const HEADING_LIMIT: usize = 64;
const TEXT_LIMIT: usize = 4096;

struct State {
    input: Vec<u8>,
    entries: Vec<OutlineEntry>,
    // 0: receiving; 1: ready; negative values are sticky until begin.
    status: i32,
}

impl State {
    fn fail(&mut self, status: i32) -> i32 {
        self.entries.clear();
        self.status = status;
        status
    }
}

thread_local! {
    static STATE: RefCell<State> = const { RefCell::new(State {
        input: Vec::new(), entries: Vec::new(), status: -3,
    }) };
}

// SAFETY: All export names are unique in this example. Only scalars cross its ABI.
#[cfg_attr(target_arch = "wasm32", unsafe(no_mangle))]
pub extern "C" fn outline_begin() {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.input.clear();
        state.entries.clear();
        state.status = 0;
    });
}

// SAFETY: Unique export name; bounds-checked byte ingestion, no caller pointer.
#[cfg_attr(target_arch = "wasm32", unsafe(no_mangle))]
pub extern "C" fn outline_push(byte: u32) -> i32 {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if state.status < 0 {
            return state.status;
        }
        if state.status != 0 || byte > 255 {
            return state.fail(-3);
        }
        if state.input.len() == INPUT_LIMIT {
            return state.fail(-1);
        }
        state.input.push(byte as u8);
        0
    })
}

// SAFETY: Unique export name; parsing uses only the instance-owned input buffer.
#[cfg_attr(target_arch = "wasm32", unsafe(no_mangle))]
pub extern "C" fn outline_finish() -> i32 {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if state.status < 0 {
            return state.status;
        }
        if state.status != 0 {
            return state.fail(-3);
        }
        let Ok(source) = std::str::from_utf8(&state.input) else {
            return state.fail(-2);
        };
        let entries = outline(source);
        if entries.len() > HEADING_LIMIT
            || entries
                .iter()
                .map(|entry| entry.heading.text.len())
                .sum::<usize>()
                > TEXT_LIMIT
        {
            return state.fail(-4);
        }
        state.entries = entries;
        state.status = 1;
        state.entries.len() as i32
    })
}

// SAFETY: Unique export name; all indices are checked, and bounded offsets fit i32.
#[cfg_attr(target_arch = "wasm32", unsafe(no_mangle))]
pub extern "C" fn outline_field(index: u32, field: u32) -> i32 {
    STATE.with(|state| {
        let state = state.borrow();
        if state.status != 1 {
            return -1;
        }
        let Some(entry) = state.entries.get(index as usize) else {
            return -1;
        };
        match field {
            0 => i32::from(entry.heading.level),
            1 => entry.heading.span.start as i32,
            2 => entry.heading.span.end as i32,
            3 => entry.section_span.end as i32,
            4 => entry.parent.map_or(-1, |parent| parent as i32),
            5 => entry.heading.text.len() as i32,
            _ => -1,
        }
    })
}

// SAFETY: Unique export name; only an owned result byte or -1 is returned.
#[cfg_attr(target_arch = "wasm32", unsafe(no_mangle))]
pub extern "C" fn outline_text_byte(index: u32, offset: u32) -> i32 {
    STATE.with(|state| {
        let state = state.borrow();
        if state.status != 1 {
            return -1;
        }
        state
            .entries
            .get(index as usize)
            .and_then(|entry| entry.heading.text.as_bytes().get(offset as usize))
            .map_or(-1, |byte| i32::from(*byte))
    })
}

#[cfg(target_arch = "wasm32")]
fn main() {}

// Independent native projection: do not use the scalar getters under test.
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{Read, Write};
    let mut bytes = Vec::new();
    std::io::stdin()
        .take((INPUT_LIMIT + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > INPUT_LIMIT {
        return Err("input limit".into());
    }
    let entries = outline(std::str::from_utf8(&bytes)?);
    if entries.len() > HEADING_LIMIT
        || entries
            .iter()
            .map(|entry| entry.heading.text.len())
            .sum::<usize>()
            > TEXT_LIMIT
    {
        return Err("result limit".into());
    }
    let mut output = std::io::BufWriter::new(std::io::stdout().lock());
    for entry in entries {
        write!(
            output,
            "{} {} {} {} {} {} ",
            entry.heading.level,
            entry.heading.span.start,
            entry.heading.span.end,
            entry.section_span.end,
            entry.parent.map_or(-1, |parent| parent as i32),
            entry.heading.text.len()
        )?;
        for byte in entry.heading.text.bytes() {
            write!(output, "{byte:02x}")?;
        }
        writeln!(output)?;
    }
    output.flush()?;
    Ok(())
}
