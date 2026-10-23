use grunkalunka_markdown::{Heading, Link, Summary, summarize};

#[test]
fn empty_document_has_no_records() {
    assert_eq!(summarize(""), Summary::default());
}

#[test]
fn empty_constructs_are_records_and_source_ranges_are_utf8_boundaries() {
    let source = "猫\n\n###\n\n[](empty) ![](empty.png) ![outer ![inner](i.png)](o.png)\n";
    let summary = summarize(source);
    assert_eq!(
        summary.headings,
        vec![Heading {
            level: 3,
            text: "".into(),
            span: 5..9
        }]
    );
    assert_eq!(summary.links[0].text, "");
    assert_eq!(summary.links[0].destination, "empty");
    assert_eq!(
        summary
            .images
            .iter()
            .map(|image| (image.text.as_str(), image.destination.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("", "empty.png"),
            ("outer inner", "o.png"),
            ("inner", "i.png")
        ]
    );
    for span in summary.headings.iter().map(|h| &h.span).chain(
        summary
            .links
            .iter()
            .chain(&summary.images)
            .map(|link| &link.span),
    ) {
        assert!(span.start <= span.end && span.end <= source.len());
        assert!(source.is_char_boundary(span.start) && source.is_char_boundary(span.end));
        assert!(!source[span.clone()].is_empty());
    }
}

#[test]
fn code_html_blocks_and_standalone_definitions_do_not_add_records() {
    for source in [
        "[definition]: notes/a.md \"Title\"\n",
        "`[inline](note.md)`\n\n```md\n# heading\n![img](image.png)\n```\n",
        "<div>\n# fake\n[markdown](note.md)\n<a href=\"x\">HTML link</a><img src=\"x\">\n</div>\n",
    ] {
        assert_eq!(summarize(source), Summary::default(), "{source}");
    }
}

#[test]
fn extensions_are_disabled_and_unresolved_references_stay_text() {
    let summary = summarize("# ~~literal~~ {#id}\n\n[[wiki]] [missing][absent]\n");
    assert_eq!(summary.headings[0].text, "~~literal~~ {#id}");
    assert!(summary.links.is_empty());
    assert!(summary.images.is_empty());
}

#[test]
fn breaks_become_newlines_and_inline_html_markup_is_uninterpreted() {
    let summary = summarize("A *bold*\nline\n===\n\n[soft\nline  \nhard <b>tag</b>](note.md)");
    assert_eq!(summary.headings[0].text, "A bold\nline");
    assert_eq!(summary.links[0].text, "soft\nline\nhard tag");
}

#[test]
fn image_inside_link_inside_heading_contributes_text_once_to_each() {
    let source = "# [before ![**猫** `x`](cat.png \"Cat\") after](note.md)\n";
    let summary = summarize(source);
    assert_eq!(summary.headings[0].text, "before 猫 x after");
    assert_eq!(summary.links[0].text, "before 猫 x after");
    assert_eq!(
        summary.images,
        vec![Link {
            text: "猫 x".into(),
            destination: "cat.png".into(),
            title: "Cat".into(),
            span: 10..source.find(" after").unwrap()
        }]
    );
    assert_eq!(&source[summary.headings[0].span.clone()], source);
    assert_eq!(
        &source[summary.links[0].span.clone()],
        source[2..source.len() - 1].to_string()
    );
}

#[test]
fn inline_reference_and_autolinks_are_data_in_source_order() {
    let source = "é [*note* `x`](notes/a.md \"A title\") [ref][n] <https://example.org> [bad](javascript:alert)\n\n[n]: other.md \"Other\"\n";
    let fragments = [
        "[*note* `x`](notes/a.md \"A title\")",
        "[ref][n]",
        "<https://example.org>",
        "[bad](javascript:alert)",
    ];
    let expected = [
        ("note x", "notes/a.md", "A title"),
        ("ref", "other.md", "Other"),
        ("https://example.org", "https://example.org", ""),
        ("bad", "javascript:alert", ""),
    ];
    assert_eq!(
        summarize(source).links,
        expected
            .into_iter()
            .zip(fragments)
            .map(|((text, destination, title), fragment)| {
                let start = source.find(fragment).unwrap();
                Link {
                    text: text.into(),
                    destination: destination.into(),
                    title: title.into(),
                    span: start..start + fragment.len(),
                }
            })
            .collect::<Vec<_>>()
    );
}

#[test]
fn atx_and_setext_headings_have_projected_text_and_original_byte_spans() {
    let source = "前文\n\n# Hé *there* &amp; `x`\n\nSecond\n------\n";
    assert_eq!(
        summarize(source).headings,
        vec![
            Heading {
                level: 1,
                text: "Hé there & x".into(),
                span: 8..32
            },
            Heading {
                level: 2,
                text: "Second".into(),
                span: 33..47
            },
        ]
    );
}
