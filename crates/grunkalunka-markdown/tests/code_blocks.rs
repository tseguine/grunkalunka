use grunkalunka_markdown::{CodeBlock, Heading, Link, Summary, code_blocks, summarize};

#[test]
fn empty_source_has_no_code_blocks() {
    assert_eq!(code_blocks(""), Vec::<CodeBlock>::new());
}

#[test]
fn empty_closed_and_unclosed_fences_are_records() {
    for (source, span) in [("```\n```\n", 0..7), ("```", 0..3)] {
        assert_eq!(
            code_blocks(source),
            vec![CodeBlock {
                text: String::new(),
                info: Some(String::new()),
                span
            }]
        );
    }
}

#[test]
fn body_text_and_full_info_keep_the_pinned_parser_projection() {
    for (source, text, info, span) in [
        ("~~~\nbody", "body", Some(""), 0..8),
        ("~~~lang\nx\n~~~~\n", "x\n", Some("lang"), 0..14),
        ("````\nx\n```\n", "x\n```\n", Some(""), 0..11),
        (
            "```  rust title\\* &amp;  \n<&amp;>\\*\n```\n",
            "<&amp;>\\*\n",
            Some("rust title* &"),
            0..39,
        ),
        ("```x\r\n猫\r\n```\r\n", "猫\n", Some("x"), 0..14),
        ("    a\n      b\n\n    c\n\n", "a\n  b\n\nc\n", None, 4..21),
        ("\talpha\n\t\tbeta\n", "alpha\n\tbeta\n", None, 1..14),
        ("    tail", "tail", None, 4..8),
        ("```\nx  \n\n```\n", "x  \n\n", Some(""), 0..12),
    ] {
        assert_eq!(
            code_blocks(source),
            vec![CodeBlock {
                text: text.into(),
                info: info.map(str::to_owned),
                span
            }],
            "{source:?}"
        );
    }
}

#[test]
fn fence_indentation_zero_through_three_and_four_space_code_are_distinct() {
    for (indent, span) in [(0, 0..11), (1, 1..14), (2, 2..17), (3, 3..20)] {
        let prefix = " ".repeat(indent);
        let source = format!("{prefix}```rs\n{prefix}x\n{prefix}```\n");
        assert_eq!(
            code_blocks(&source),
            vec![CodeBlock {
                text: "x\n".into(),
                info: Some("rs".into()),
                span
            }]
        );
    }
    assert_eq!(
        code_blocks("    ```rs\n    x\n    ```\n"),
        vec![CodeBlock {
            text: "```rs\nx\n```\n".into(),
            info: None,
            span: 4..24
        }]
    );
}

#[test]
fn container_ranges_are_exact_parser_ranges_not_whole_physical_lines() {
    for (source, info, span) in [
        ("> ```rs\n> a\n> ```\n", "rs", 2..17),
        ("- item\n\n  ```\n  x\n  ```\n", "", 10..23),
    ] {
        let text = if info == "rs" { "a\n" } else { "x\n" };
        assert_eq!(
            code_blocks(source),
            vec![CodeBlock {
                text: text.into(),
                info: Some(info.into()),
                span
            }]
        );
    }
}

#[test]
fn source_order_includes_empty_blocks_and_excludes_outside_text() {
    assert_eq!(
        code_blocks("```\n```\n\n    tail"),
        vec![
            CodeBlock {
                text: String::new(),
                info: Some(String::new()),
                span: 0..7
            },
            CodeBlock {
                text: "tail".into(),
                info: None,
                span: 13..17
            },
        ]
    );
    assert_eq!(
        code_blocks("before\n\n```\nx\n```\n\nafter"),
        vec![CodeBlock {
            text: "x\n".into(),
            info: Some(String::new()),
            span: 8..17
        }]
    );
}

#[test]
fn literal_body_and_unknown_info_do_not_create_summary_records() {
    let source = "```bash run=now\n# heading [link](javascript:alert) ![image](file)\n<b>&amp;</b> \\*\n```\n";
    let blocks = code_blocks(source);
    assert_eq!(
        blocks,
        vec![CodeBlock {
            text: "# heading [link](javascript:alert) ![image](file)\n<b>&amp;</b> \\*\n".into(),
            info: Some("bash run=now".into()),
            span: 0..source.len() - 1,
        }]
    );
    assert_eq!(summarize(source), Summary::default());
}

#[test]
fn inline_code_raw_html_and_plain_text_are_excluded() {
    for source in [
        "plain [[wiki]]",
        "`inline code`",
        "<pre>\n```rust\nx\n```\n</pre>\n",
        "\n    \n",
    ] {
        assert!(code_blocks(source).is_empty(), "{source:?}");
    }
}

#[test]
fn unicode_prefix_and_body_use_original_byte_offsets() {
    let source = "é\n\n```x\n猫\n```\n";
    assert_eq!(
        code_blocks(source),
        vec![CodeBlock {
            text: "猫\n".into(),
            info: Some("x".into()),
            span: 4..16
        }]
    );
    for block in code_blocks(source) {
        assert!(source.is_char_boundary(block.span.start));
        assert!(source.is_char_boundary(block.span.end));
        assert_eq!(&source[block.span], "```x\n猫\n```");
    }
}

#[test]
fn mixed_summary_records_keep_their_existing_text_and_ranges() {
    let source = "# H\n\n```md\n# fake [x](u)\n```\n\n[real](dest)\n";
    let start = source.find("[real]").unwrap();
    assert_eq!(
        summarize(source),
        Summary {
            headings: vec![Heading {
                level: 1,
                text: "H".into(),
                span: 0..4
            }],
            links: vec![Link {
                text: "real".into(),
                destination: "dest".into(),
                title: String::new(),
                span: start..source.len() - 1
            }],
            images: Vec::new(),
        }
    );
    assert_eq!(code_blocks(source).len(), 1);
}
