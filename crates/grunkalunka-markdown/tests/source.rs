use grunkalunka_markdown::{SourceIndex, SourcePosition, summarize};

fn position(line: usize, utf16_column: usize) -> SourcePosition {
    SourcePosition {
        line,
        utf16_column,
    }
}

#[test]
fn empty_input_and_terminal_newlines_keep_an_addressable_final_line() {
    for (source, spans) in [
        ("", vec![(0, 0)]),
        ("a", vec![(0, 1)]),
        ("\n", vec![(0, 0), (1, 1)]),
        ("\r", vec![(0, 0), (1, 1)]),
        ("\r\n", vec![(0, 0), (2, 2)]),
        ("\r\n\n\r", vec![(0, 0), (2, 2), (3, 3), (4, 4)]),
    ] {
        let index = SourceIndex::new(source);
        assert_eq!(index.line_count(), spans.len());
        for (line, (start, end)) in spans.into_iter().enumerate() {
            assert_eq!(index.line_span(line), Some(start..end));
            assert_eq!(index.byte_offset(position(line, 0)), Some(start));
        }
        let end = index.position(source.len()).unwrap();
        assert_eq!(index.byte_offset(end), Some(source.len()));
    }
}

#[test]
fn mixed_line_endings_exclude_terminators_and_do_not_alias_crlf_interiors() {
    let index = SourceIndex::new("a\r\n猫\rb\n");
    for (offset, expected) in [
        (0, position(0, 0)),
        (1, position(0, 1)),
        (3, position(1, 0)),
        (6, position(1, 1)),
        (7, position(2, 0)),
        (8, position(2, 1)),
        (9, position(3, 0)),
    ] {
        assert_eq!(index.position(offset), Some(expected));
        assert_eq!(index.byte_offset(expected), Some(offset));
    }
    for offset in [2, 4, 5, 10, usize::MAX] {
        assert_eq!(index.position(offset), None);
    }
    assert_eq!(index.line_span(1), Some(3..6));
    assert_eq!(index.line_span(4), None);
}

#[test]
fn utf16_columns_preserve_bom_combining_marks_tabs_nul_and_unicode_separators() {
    let source = "\u{feff}aé猫😀e\u{301}\t\0\u{2028}\u{2029}";
    let index = SourceIndex::new(source);
    assert_eq!(index.line_count(), 1);
    let boundaries = [
        (0, 0),
        (3, 1),
        (4, 2),
        (6, 3),
        (9, 4),
        (13, 6),
        (14, 7),
        (16, 8),
        (17, 9),
        (18, 10),
        (21, 11),
        (24, 12),
    ];
    for (byte, column) in boundaries {
        assert_eq!(index.position(byte), Some(position(0, column)));
        assert_eq!(index.byte_offset(position(0, column)), Some(byte));
    }
    assert_eq!(index.byte_offset(position(0, 5)), None);
    for byte in 0..source.len() {
        if !source.is_char_boundary(byte) {
            assert_eq!(index.position(byte), None);
        }
    }
    for bad in [
        position(0, 13),
        position(0, usize::MAX),
        position(1, 0),
        position(usize::MAX, 0),
    ] {
        assert_eq!(index.byte_offset(bad), None);
    }
}

#[test]
fn actual_summary_ranges_map_into_original_source_not_projected_labels() {
    let source = "# Heading\r\n😀 [猫](next.md)\r\n";
    let index = SourceIndex::new(source);
    let summary = summarize(source);
    assert_eq!(
        index.position(summary.headings[0].span.start),
        Some(position(0, 0))
    );
    assert_eq!(
        index.position(summary.headings[0].span.end),
        Some(position(1, 0))
    );
    assert_eq!(
        index.position(summary.links[0].span.start),
        Some(position(1, 3))
    );
    for span in [&summary.headings[0].span, &summary.links[0].span] {
        for byte in [span.start, span.end] {
            assert_eq!(index.byte_offset(index.position(byte).unwrap()), Some(byte));
        }
    }
    assert_eq!(&source[summary.links[0].span.clone()], "[猫](next.md)");
}

// An intentionally slow prefix oracle: normalize line endings in each prefix,
// rather than using a stored line table or searching that table.
fn reference(source: &str, byte: usize) -> Option<SourcePosition> {
    if !source.is_char_boundary(byte)
        || (byte > 0
            && source.as_bytes().get(byte - 1) == Some(&b'\r')
            && source.as_bytes().get(byte) == Some(&b'\n'))
    {
        return None;
    }
    let prefix = source[..byte].replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<_> = prefix.split('\n').collect();
    Some(position(
        lines.len() - 1,
        lines.last().unwrap().encode_utf16().count(),
    ))
}

#[test]
fn every_short_mixed_source_matches_independent_prefix_and_inverse_searches() {
    let alphabet = ["a", "é", "😀", "\r", "\n", "\t"];
    for length in 0..=4 {
        for mut encoded in 0..alphabet.len().pow(length) {
            let mut source = String::new();
            for _ in 0..length {
                source.push_str(alphabet[encoded % alphabet.len()]);
                encoded /= alphabet.len();
            }
            let index = SourceIndex::new(&source);
            let expected: Vec<_> = (0..=source.len())
                .filter_map(|byte| reference(&source, byte).map(|pos| (byte, pos)))
                .collect();
            for byte in 0..=source.len() + 1 {
                assert_eq!(
                    index.position(byte),
                    reference(&source, byte),
                    "{source:?} at {byte}"
                );
            }
            let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
            let lines: Vec<_> = normalized.split('\n').collect();
            assert_eq!(index.line_count(), lines.len());
            for (line, text) in lines.iter().enumerate() {
                assert_eq!(&source[index.line_span(line).unwrap()], *text);
                for column in 0..=text.encode_utf16().count() + 1 {
                    let pos = position(line, column);
                    let byte = expected.iter().find(|(_, p)| *p == pos).map(|(b, _)| *b);
                    assert_eq!(index.byte_offset(pos), byte, "{source:?} at {pos:?}");
                }
            }
        }
    }
}

#[test]
fn long_unicode_lines_and_many_empty_lines_keep_exact_coordinates() {
    let source = format!("{}{}last", "😀".repeat(4096), "\r\n".repeat(4096));
    let index = SourceIndex::new(&source);
    assert_eq!(index.position(16384), Some(position(0, 8192)));
    assert_eq!(index.byte_offset(position(0, 8191)), None);
    assert_eq!(index.line_count(), 4097);
    assert_eq!(index.position(source.len()), Some(position(4096, 4)));
    assert_eq!(index.byte_offset(position(4096, 4)), Some(source.len()));
}
