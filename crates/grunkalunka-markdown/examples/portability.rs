//! Fixed-fixture portability probe, not a public guest ABI.
use grunkalunka_markdown::{SourceIndex, code_blocks, outline, prose_text, summarize};
use std::cell::RefCell;

const SOURCES: &[&str] = &[
    "",
    "# Notes\n\nA *small* [link](next.md).\n\n### Detail\n`inline`\n",
    "\u{feff}# 猫😀\r\n\r\n[é](other.md)\rOnly\n\tend\u{301}\u{2028}\0",
    "> # Quote\n> [![Alt](image.png)](note.md)\n\n- ## Item\n\n# Again\n# Again\n",
    "```sh title\r\n<literal> &amp;\r\n```\r\n\n    indented\n\n<div>hidden</div>\n",
    "Title\n=====\n\n[One][id] [missing]\n\n[id]: <javascript:alert(1)> \"inert\"\n",
];

thread_local! {
    static RESULT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn project(source: &str) -> String {
    let index = SourceIndex::new(source);
    let positions: Vec<_> = (0..=source.len())
        .map(|offset| {
            let position = index.position(offset);
            (position, position.and_then(|value| index.byte_offset(value)))
        })
        .collect();
    let lines: Vec<_> = (0..index.line_count())
        .map(|line| index.line_span(line))
        .collect();
    format!(
        "summary={:?};outline={:?};code={:?};prose={:?};lines={lines:?};positions={positions:?}",
        summarize(source),
        outline(source),
        code_blocks(source),
        prose_text(source)
    )
}

// SAFETY: These probe-specific names are unique in this example's Wasm module.
#[cfg_attr(target_arch = "wasm32", unsafe(no_mangle))]
pub extern "C" fn markdown_probe_count() -> u32 {
    SOURCES.len() as u32
}

// SAFETY: Unique export name; no caller pointers or references cross this boundary.
#[cfg_attr(target_arch = "wasm32", unsafe(no_mangle))]
pub extern "C" fn markdown_probe_render(index: u32) -> i32 {
    RESULT.with(|result| {
        let mut bytes = result.borrow_mut();
        bytes.clear();
        let Some(source) = SOURCES.get(index as usize) else {
            return -1;
        };
        *bytes = project(source).into_bytes();
        i32::try_from(bytes.len()).expect("fixed fixture output fits i32")
    })
}

// SAFETY: Unique export name; bounds-checked scalar access only, with -1 at EOF.
#[cfg_attr(target_arch = "wasm32", unsafe(no_mangle))]
pub extern "C" fn markdown_probe_byte(offset: u32) -> i32 {
    RESULT.with(|result| {
        result
            .borrow()
            .get(offset as usize)
            .map_or(-1, |byte| i32::from(*byte))
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::io::Result<()> {
    use std::io::Write;
    let mut output = std::io::BufWriter::new(std::io::stdout().lock());
    for source in SOURCES {
        for byte in project(source).bytes() {
            write!(output, "{byte:02x}")?;
        }
        writeln!(output)?;
    }
    output.flush()
}

#[cfg(target_arch = "wasm32")]
fn main() {}
