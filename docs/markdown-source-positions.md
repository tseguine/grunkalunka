# Original-source positions

`SourceIndex::new(&str)` in `grunkalunka-markdown` converts original UTF-8 byte
positions to zero-based line/UTF-16 columns and back. This lets a document client
use the byte ranges from summaries, outlines and code blocks without treating
UTF-8 byte counts as JavaScript string offsets. Parsing is not required.

```rust
use grunkalunka_markdown::{SourceIndex, SourcePosition, summarize};

let source = "# Notes\r\n😀 [Next](next.md)\r\n";
let index = SourceIndex::new(source);
let summary = summarize(source);
let link = &summary.links[0];
assert_eq!(index.position(link.span.start), Some(SourcePosition {
    line: 1, utf16_column: 3,
}));
assert_eq!(index.byte_offset(SourcePosition { line: 1, utf16_column: 3 }),
    Some(link.span.start));
```

`position(byte_offset)` and `byte_offset(SourcePosition)` return `None` for invalid
coordinates; they never clamp or round. Byte offsets inside a UTF-8 scalar and
columns between a supplementary character's two UTF-16 units are invalid. EOF is
a valid position. `line_span(line)` returns the original half-open content range,
excluding its terminator, or `None` for a nonexistent line.

LF, CRLF and lone CR terminate lines, following CommonMark's line-ending forms.
A line-end position points just before its terminator; the next line starts after
it. The position between CR and LF in a CRLF pair is rejected, not aliased to either
line. Empty input has one empty line, and a trailing terminator adds a final empty
line. Line/paragraph separators U+2028/U+2029 are content, not extra terminators.

The source is borrowed unchanged. A BOM, tabs, NUL and combining marks retain their
code-unit widths. No normalization, unescaping, tab expansion, grapheme segmentation
or projected-text mapping occurs. These are original-source coordinates, not visual
caret positions or a complete editor protocol implementation. Apply the index only
to ranges from the same source snapshot; rebuild it after edits.

Construction takes linear time in source bytes and stores two offsets per line,
without copying the source. Byte-to-position lookup searches line boundaries then
scans that line's prefix. Position-to-byte lookup scans the selected line until its
column is reached. Repeated queries on long lines can therefore be costly; this is
not a per-character index or incremental editor buffer. Existing caller-owned
input and allocation bounds still apply.

The workspace tests cover Unicode boundaries, CR/LF/CRLF, BOM/EOF/empty lines,
invalid and extreme coordinates, actual Markdown ranges and all 1,555 strings of
length zero through four over a six-token ASCII/Unicode/newline alphabet. A separate
prefix-normalization oracle and inverse search check both directions. Run
`cargo test -p grunkalunka-markdown --locked --offline`; canonical CI includes it.

Related: [Markdown inspection](markdown.md), [CommonMark characters and lines](https://spec.commonmark.org/0.31.2/#characters-and-lines).
