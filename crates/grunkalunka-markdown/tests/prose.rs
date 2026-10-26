use grunkalunka_markdown::prose_text;

#[test]
fn empty_or_excluded_documents_produce_no_text() {
    for source in [
        "",
        "\n\n",
        "#\n\n##\n",
        "[r]: note.md \"Title\"\n",
        "```js\nsecret()\n```\n",
        "    secret()\n",
        "<div>\nsecret\n</div>\n",
        "<!-- hidden -->\n",
        "---\n\n***\n",
    ] {
        assert_eq!(prose_text(source), "", "{source}");
    }
}

#[test]
fn headings_paragraphs_and_rules_separate_words_without_extra_blank_lines() {
    let source = "# First\n\nOne *bold* sentence.\n\n---\n\nSecond\n===\n\nLast";
    assert_eq!(
        prose_text(source),
        "First\nOne bold sentence.\nSecond\nLast"
    );
    assert_eq!(prose_text("# one\n##\n### two\n"), "one\ntwo");
}

#[test]
fn labels_alt_text_and_inline_code_are_not_duplicated() {
    let source = "# [before ![**猫** `x`](cat.png \"Cat\") after](note.md)\n";
    assert_eq!(prose_text(source), "before 猫 x after");
    assert_eq!(
        prose_text("![outer ![inner](i.png)](o.png) [label](javascript:alert \"Title\")"),
        "outer inner label"
    );
}

#[test]
fn links_contribute_labels_not_destinations_titles_or_definitions() {
    let source = "[note][r] ![alt](image.png) <https://example.org>\n\n[r]: secret.md \"Title\"\n";
    assert_eq!(prose_text(source), "note alt https://example.org");
    assert_eq!(prose_text("[missing][absent]"), "[missing][absent]");
}

#[test]
fn code_blocks_are_excluded_without_joining_surrounding_words() {
    let source = "before\n\n```js\nsecret()\n```\n\nafter\n\n    hidden\n\nlast";
    assert_eq!(prose_text(source), "before\nafter\nlast");
    assert_eq!(
        prose_text("Inline `a < b` remains.\n\n~~~\nhidden"),
        "Inline a < b remains."
    );
}

#[test]
fn inline_html_is_markup_only_and_html_blocks_are_uninterpreted() {
    let source = "one<b>bold</b>two<!-- hidden -->\n\n<div>\nsecret\n</div>\n\nlast";
    assert_eq!(prose_text(source), "oneboldtwo\nlast");
    assert_eq!(prose_text("a<br>b"), "ab");
}

#[test]
fn tight_loose_and_nested_containers_keep_text_in_source_order() {
    let source = concat!(
        "- one\n- two\n  - nested\n\n- three\n\n  second paragraph\n\n",
        "> quoted\n>\n> more\n\noutside\n"
    );
    assert_eq!(
        prose_text(source),
        "one\ntwo\nnested\nthree\nsecond paragraph\nquoted\nmore\noutside"
    );
    assert_eq!(prose_text("1. first\n2. second\n"), "first\nsecond");
}

#[test]
fn breaks_crlf_unicode_entities_and_escapes_follow_parser_projection() {
    let source = "a\r\nb  \r\nc\\\r\nd\r\n\r\n# É 🦀 &amp; \\*literal\\*\r\n";
    assert_eq!(prose_text(source), "a\nb\nc\nd\nÉ 🦀 & *literal*");
    assert_eq!(prose_text("`a  b` &lt;b&gt;"), "a  b <b>");
}

#[test]
fn disabled_extensions_remain_literal_text() {
    assert_eq!(
        prose_text("~~literal~~ [[wiki]] {#id}\n\n- [ ] task\n"),
        "~~literal~~ [[wiki]] {#id}\n[ ] task"
    );
}

#[test]
fn wide_documents_do_not_merge_or_duplicate_sections() {
    let source = "# Same\n\nword ![alt](img)\n\n```\nsecret\n```\n\n".repeat(2048);
    let result = prose_text(&source);
    let expected = vec!["Same\nword alt"; 2048].join("\n");
    assert_eq!(result, expected);
    assert!(!result.contains("secret"));
    assert!(!result.ends_with('\n'));
}
