use grunkalunka_markdown::{outline, summarize};

#[test]
fn documents_without_headings_have_no_synthetic_root() {
    for source in [
        "",
        "Preamble only.\n\n[link](note.md)\n",
        "```md\n# Code, not a heading\n```\n",
        "    # Indented code\n",
        "<div>\n# Raw HTML\n</div>\n",
        "`# Inline code`\n\n\\# Escaped marker\n",
    ] {
        assert!(outline(source).is_empty(), "{source}");
    }
}

#[test]
fn gaps_siblings_and_roots_have_explicit_parents_and_section_ends() {
    let source = concat!(
        "Preamble\n\n## Intro\nbody\n\n",
        "#### Deep\ntext\n\n#### Peer\ntext\n\n",
        "### Mid\ntext\n\n# Next\ntext\n\n###### Last"
    );
    let entries = outline(source);
    let markers = [
        "## Intro",
        "#### Deep",
        "#### Peer",
        "### Mid",
        "# Next",
        "###### Last",
    ];
    let starts: Vec<_> = markers
        .iter()
        .map(|text| source.find(text).unwrap())
        .collect();
    let parents = [None, Some(0), Some(0), Some(0), None, Some(4)];
    let ends = [
        starts[4],
        starts[2],
        starts[3],
        starts[4],
        source.len(),
        source.len(),
    ];
    assert_eq!(entries.len(), markers.len());
    for (index, entry) in entries.iter().enumerate() {
        assert_eq!(entry.parent, parents[index]);
        assert_eq!(entry.section_span, starts[index]..ends[index]);
        assert_eq!(entry.heading.span.start, starts[index]);
    }
    assert_eq!(entries[0].heading.level, 2);
    assert_eq!(entries[1].heading.level, 4);
    assert_eq!(entries[5].heading.level, 6);
    assert_eq!(
        &source[entries[1].section_span.clone()],
        "#### Deep\ntext\n\n"
    );
}

#[test]
fn projected_labels_empty_headings_and_duplicates_are_retained() {
    let source = concat!(
        "# [Hi ![*cat*](cat.png)](javascript:alert) &amp; `x`\n\n",
        "##\n\nSame\n---\n\n## Same"
    );
    let summary = summarize(source);
    let entries = outline(source);
    let labels: Vec<_> = entries
        .iter()
        .map(|entry| entry.heading.text.as_str())
        .collect();
    assert_eq!(labels, ["Hi cat & x", "", "Same", "Same"]);
    for (entry, heading) in entries.iter().zip(&summary.headings) {
        assert_eq!(&entry.heading, heading);
    }
    assert_eq!(entries[0].parent, None);
    for entry in &entries[1..] {
        assert_eq!(entry.parent, Some(0));
    }
    assert_ne!(entries[2].heading.span, entries[3].heading.span);
    assert_eq!(summarize(source), summary);
}

#[test]
fn unicode_crlf_and_eof_use_original_byte_offsets() {
    let source = "Préface 🦀\r\n\r\n# 猫\r\nbody\r\n\r\n## café\r\nlast 🦀";
    let entries = outline(source);
    assert_eq!(entries.len(), 2);
    let root = source.find("# 猫").unwrap();
    let child = source.find("## café").unwrap();
    assert_eq!(entries[0].section_span, root..source.len());
    assert_eq!(entries[1].section_span, child..source.len());
    assert_eq!(
        &source[entries[1].section_span.clone()],
        "## café\r\nlast 🦀"
    );
    assert_eq!(entries[1].parent, Some(0));
    for entry in &entries {
        for span in [&entry.heading.span, &entry.section_span] {
            assert!(span.start <= span.end && span.end <= source.len());
            assert!(source.is_char_boundary(span.start));
            assert!(source.is_char_boundary(span.end));
        }
    }
}

#[test]
fn setext_and_atx_headings_share_the_same_hierarchy() {
    let source = "Preface\n\nMain *title*\n===\n\ntext\n\nSub `code`\n---\n\n## Peer\n\n# Final";
    let entries = outline(source);
    let labels: Vec<_> = entries
        .iter()
        .map(|entry| entry.heading.text.as_str())
        .collect();
    assert_eq!(labels, ["Main title", "Sub code", "Peer", "Final"]);
    assert_eq!(entries[1].parent, Some(0));
    assert_eq!(entries[2].parent, Some(0));
    assert_eq!(entries[3].parent, None);
    assert_eq!(entries[0].section_span.start, source.find("Main").unwrap());
    assert_eq!(entries[0].section_span.end, source.find("# Final").unwrap());
    assert_eq!(entries[1].section_span.end, source.find("## Peer").unwrap());
    assert_eq!(entries[2].section_span.end, source.find("# Final").unwrap());
}

#[test]
fn container_headings_follow_source_levels_not_container_ancestry() {
    let source = "# Root\n\n> ## Quoted\n>\n> text\n\n- ### Listed\n\n# Next\n";
    let entries = outline(source);
    let summary = summarize(source);
    let parents = [None, Some(0), Some(1), None];
    assert_eq!(entries.len(), parents.len());
    assert_eq!(entries.len(), summary.headings.len());
    for (index, entry) in entries.iter().enumerate() {
        assert_eq!(entry.parent, parents[index]);
        assert_eq!(entry.heading, summary.headings[index]);
        assert_eq!(entry.section_span.start, entry.heading.span.start);
    }
    assert_eq!(entries[1].heading.text, "Quoted");
    assert_eq!(entries[2].heading.text, "Listed");
    assert_eq!(entries[1].section_span.end, entries[3].heading.span.start);
    assert_eq!(entries[2].section_span.end, entries[3].heading.span.start);
}

#[test]
fn every_four_heading_level_sequence_matches_independent_searches() {
    for mut code in 0_usize..6_usize.pow(4) {
        let mut source = String::new();
        for index in 0..4 {
            let level = code % 6 + 1;
            code /= 6;
            source.push_str(&format!(
                "{} h{index}\n\nbody {index}\n\n",
                "#".repeat(level)
            ));
        }
        let headings = summarize(&source).headings;
        let entries = outline(&source);
        assert_eq!(entries.len(), 4);
        for (index, entry) in entries.iter().enumerate() {
            // Search independently rather than copying the implementation's stack.
            let parent = (0..index).rfind(|&other| headings[other].level < entry.heading.level);
            let end = headings
                .iter()
                .skip(index + 1)
                .find(|next| next.level <= entry.heading.level)
                .map_or(source.len(), |next| next.span.start);
            assert_eq!(entry.parent, parent, "{source}");
            assert_eq!(
                entry.section_span,
                entry.heading.span.start..end,
                "{source}"
            );
            if let Some(parent) = entry.parent {
                assert!(parent < index);
                assert!(entries[parent].section_span.start <= entry.section_span.start);
                assert!(entry.section_span.end <= entries[parent].section_span.end);
            }
        }
    }
}

#[test]
fn many_duplicate_siblings_stay_distinct_and_flat() {
    let line = "# Same\n";
    let source = line.repeat(2048);
    let entries = outline(&source);
    assert_eq!(entries.len(), 2048);
    for (index, entry) in entries.iter().enumerate() {
        assert_eq!(entry.parent, None);
        assert_eq!(entry.heading.text, "Same");
        assert_eq!(
            entry.section_span,
            index * line.len()..(index + 1) * line.len()
        );
    }
}
