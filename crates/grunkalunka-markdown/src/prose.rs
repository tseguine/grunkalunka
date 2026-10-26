use pulldown_cmark::{Event, Options, Parser, TagEnd};

/// Project Markdown prose into plain text for indexing, without resolving links.
///
/// Includes headings, paragraphs, list/quote contents, inline code and image alt
/// text exactly once. Excludes code blocks, raw HTML blocks, inline HTML markup,
/// destinations and titles. Autolink labels still contain their displayed URL.
/// Parser text normalization is retained; block boundaries and soft/hard breaks
/// insert one newline between nonempty text runs, without an added final newline.
///
/// This is not rendered HTML, sanitized markup or a source-position map. Output
/// offsets cannot be used as original-source offsets. Caller input limits apply.
///
/// ```
/// assert_eq!(
///     grunkalunka_markdown::prose_text("# Notes\n\nA *small* [link](note.md)."),
///     "Notes\nA small link."
/// );
/// ```
pub fn prose_text(source: &str) -> String {
    let mut output = String::new();
    let mut boundary = false;
    let mut excluded = false;
    for event in Parser::new_ext(source, Options::empty()) {
        match event {
            Event::Start(tag) => {
                let end = tag.to_end();
                if matches!(end, TagEnd::CodeBlock | TagEnd::HtmlBlock) {
                    excluded = true;
                }
                boundary |= block_boundary(end);
            }
            Event::End(tag) => {
                if matches!(tag, TagEnd::CodeBlock | TagEnd::HtmlBlock) {
                    excluded = false;
                }
                boundary |= block_boundary(tag);
            }
            Event::Text(text) | Event::Code(text) if !excluded && !text.is_empty() => {
                if boundary && !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str(&text);
                boundary = false;
            }
            Event::SoftBreak | Event::HardBreak | Event::Rule => boundary = true,
            _ => {}
        }
    }
    output
}

fn block_boundary(tag: TagEnd) -> bool {
    matches!(
        tag,
        TagEnd::Paragraph
            | TagEnd::Heading(_)
            | TagEnd::BlockQuote(_)
            | TagEnd::List(_)
            | TagEnd::Item
            | TagEnd::CodeBlock
            | TagEnd::HtmlBlock
    )
}
