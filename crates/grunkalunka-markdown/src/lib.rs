//! Source-string Markdown inspection without rendering or resource resolution.
//!
//! This is a projection using pulldown-cmark's pinned CommonMark defaults, not a
//! renderer, sanitizer, source-preserving editor or complete document model.

#![forbid(unsafe_code)]

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;

/// Extracted document information.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    /// Headings in source order, including empty headings.
    pub headings: Vec<Heading>,
    /// Markdown links in source order; destinations are inert, unvalidated data.
    pub links: Vec<Link>,
    /// Markdown images in source order, with projected alt text in `Link::text`.
    pub images: Vec<Link>,
}

/// A Markdown link and its full source range at the use site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    /// Descendant text and inline code, with line breaks represented as `\n`.
    pub text: String,
    /// Parser-normalized destination; no URL or path policy has been applied.
    pub destination: String,
    /// Parser-normalized title, or an empty string when absent.
    pub title: String,
    /// Full construct at its use site, as half-open original UTF-8 byte offsets.
    pub span: Range<usize>,
}

/// A heading and its full original Markdown source range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heading {
    /// Heading level from 1 through 6.
    pub level: u8,
    /// Descendant text and inline code, with line breaks represented as `\n`.
    pub text: String,
    /// Full original construct, including its marker and any terminating newline.
    pub span: Range<usize>,
}

#[derive(Clone, Copy)]
enum ReferenceKind {
    Link,
    Image,
}

impl ReferenceKind {
    fn records(self, summary: &mut Summary) -> &mut Vec<Link> {
        match self {
            Self::Link => &mut summary.links,
            Self::Image => &mut summary.images,
        }
    }
}

/// Inspect source using the pinned parser's CommonMark defaults, with no extensions.
///
/// Entities/escapes and inline code are normalized by the parser. Formatting and
/// inline HTML tags contribute no text; text between those tags still contributes.
/// Raw HTML blocks are uninterpreted. Reference links resolve only definitions in
/// this source; unresolved references remain text. No resource is opened or fetched.
///
/// ```
/// let summary = grunkalunka_markdown::summarize("# Notes\n\n[Next](next.md)");
/// assert_eq!(summary.headings[0].text, "Notes");
/// assert_eq!(summary.links[0].destination, "next.md");
/// ```
pub fn summarize(source: &str) -> Summary {
    let mut summary = Summary::default();
    let mut heading: Option<Heading> = None;
    // Allocate records at Start for source order. The stack identifies every
    // active ancestor, so nested alt/label text is appended once to each record.
    let mut references: Vec<(ReferenceKind, usize)> = Vec::new();
    for (event, span) in Parser::new_ext(source, Options::empty()).into_offset_iter() {
        let event = match event {
            Event::SoftBreak | Event::HardBreak => Event::Text("\n".into()),
            other => other,
        };
        match event {
            Event::Start(tag @ (Tag::Link { .. } | Tag::Image { .. })) => {
                let kind = if matches!(tag, Tag::Image { .. }) {
                    ReferenceKind::Image
                } else {
                    ReferenceKind::Link
                };
                let (Tag::Link {
                    dest_url, title, ..
                }
                | Tag::Image {
                    dest_url, title, ..
                }) = tag
                else {
                    unreachable!()
                };
                let records = kind.records(&mut summary);
                references.push((kind, records.len()));
                records.push(Link {
                    text: String::new(),
                    destination: dest_url.into_string(),
                    title: title.into_string(),
                    span,
                });
            }
            Event::End(TagEnd::Link | TagEnd::Image) => {
                references.pop();
            }
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some(Heading {
                    level: level as u8,
                    text: String::new(),
                    span,
                });
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(value) = heading.take() {
                    summary.headings.push(value);
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some(value) = &mut heading {
                    value.text.push_str(&text);
                }
                for &(kind, index) in &references {
                    kind.records(&mut summary)[index].text.push_str(&text);
                }
            }
            _ => {}
        }
    }
    summary
}

/// An inert code block projected by the pinned parser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeBlock {
    /// Concatenated parser text, without additional normalization.
    pub text: String,
    /// None for indented blocks; Some("") for an empty fenced info string.
    /// Otherwise contains the whole parser-normalized info string, not just a language.
    pub info: Option<String>,
    /// Exact parser Start range as half-open original UTF-8 byte offsets.
    /// This need not include first-line indentation or a closing fence's newline.
    pub span: Range<usize>,
}

/// Inspect code blocks as inert data, separately from the document summary.
///
/// Uses the pinned parser's CommonMark defaults without extensions. Text is the
/// concatenation of its Text events, with no extra trimming or normalization.
/// Inline code and raw HTML blocks are excluded. Empty fenced blocks are retained.
/// Info and body text are never executed, rendered or resolved as resources.
/// Ranges refer to the unchanged input, not a body-only or lossless editing span.
pub fn code_blocks(source: &str) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();
    let mut active: Option<usize> = None;
    for (event, span) in Parser::new_ext(source, Options::empty()).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let info = match kind {
                    pulldown_cmark::CodeBlockKind::Indented => None,
                    pulldown_cmark::CodeBlockKind::Fenced(info) => Some(info.into_string()),
                };
                active = Some(blocks.len());
                blocks.push(CodeBlock {
                    text: String::new(),
                    info,
                    span,
                });
            }
            Event::Text(text) => {
                if let Some(index) = active {
                    blocks[index].text.push_str(&text);
                }
            }
            Event::End(TagEnd::CodeBlock) => active = None,
            _ => {}
        }
    }
    blocks
}
