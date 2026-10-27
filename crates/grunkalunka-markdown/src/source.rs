use std::ops::Range;

/// A zero-based line and UTF-16 code-unit column in the original source.
/// Columns are not grapheme counts or visual/tab-expanded columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourcePosition {
    pub line: usize,
    pub utf16_column: usize,
}

/// Coordinates for one immutable UTF-8 source, independent of Markdown parsing.
///
/// LF, CRLF and lone CR end lines. Terminators are excluded from columns; a
/// trailing terminator creates an empty final line. Empty input has one line.
/// A BOM, tabs, combining marks and Unicode line/paragraph separators are ordinary
/// content. No normalization, unescaping or source-copying is performed.
///
/// ```
/// use grunkalunka_markdown::{SourceIndex, SourcePosition};
/// let index = SourceIndex::new("猫😀\r\nnext");
/// let end = SourcePosition { line: 0, utf16_column: 3 };
/// assert_eq!(index.position(7), Some(end));
/// assert_eq!(index.byte_offset(end), Some(7));
/// assert_eq!(index.position(8), None); // Inside CRLF.
/// assert_eq!(index.line_span(1), Some(9..13));
/// ```
#[derive(Clone, Debug)]
pub struct SourceIndex<'a> {
    source: &'a str,
    lines: Vec<Range<usize>>,
}

impl<'a> SourceIndex<'a> {
    /// Index line boundaries in O(source bytes) time and O(line count) storage.
    pub fn new(source: &'a str) -> Self {
        let bytes = source.as_bytes();
        let mut lines = Vec::new();
        let mut start = 0;
        let mut offset = 0;
        while offset < bytes.len() {
            if matches!(bytes[offset], b'\r' | b'\n') {
                lines.push(start..offset);
                if bytes[offset] == b'\r' && bytes.get(offset + 1) == Some(&b'\n') {
                    offset += 1;
                }
                start = offset + 1;
            }
            offset += 1;
        }
        lines.push(start..source.len());
        Self { source, lines }
    }

    /// Number of indexed lines, including an empty final line when present.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Original half-open byte range of line content, excluding its terminator.
    pub fn line_span(&self, line: usize) -> Option<Range<usize>> {
        self.lines.get(line).cloned()
    }

    /// Convert an original byte offset, including EOF, without clamping.
    /// Rejects out-of-bounds offsets, UTF-8 interiors and the middle of CRLF.
    /// Takes O(log(line count) + bytes before the offset in that line) time.
    pub fn position(&self, byte_offset: usize) -> Option<SourcePosition> {
        if !self.source.is_char_boundary(byte_offset) {
            return None;
        }
        let line = self.lines.partition_point(|span| span.start <= byte_offset) - 1;
        let span = &self.lines[line];
        if byte_offset > span.end {
            return None;
        }
        Some(SourcePosition {
            line,
            utf16_column: self.source[span.start..byte_offset].encode_utf16().count(),
        })
    }

    /// Convert a position to an original byte offset without clamping.
    /// Rejects nonexistent lines, columns past line end and split surrogate pairs.
    /// Scans at most the selected line; line-end positions precede the terminator.
    pub fn byte_offset(&self, position: SourcePosition) -> Option<usize> {
        let span = self.lines.get(position.line)?;
        let mut column = 0;
        for (offset, ch) in self.source[span.clone()].char_indices() {
            if column == position.utf16_column {
                return Some(span.start + offset);
            }
            column += ch.len_utf16();
            if column > position.utf16_column {
                return None;
            }
        }
        (column == position.utf16_column).then_some(span.end)
    }
}
