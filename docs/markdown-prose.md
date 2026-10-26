# Markdown prose for indexing

`grunkalunka_markdown::prose_text(&str) -> String` extracts a plain-text projection
for prose search and indexing. It uses the same pinned parser and disabled
extension policy as [Markdown inspection](markdown.md). No renderer, resource
resolver, filesystem, network, index database or host capability is involved.

The projection includes heading and paragraph text, tight and loose list items,
block-quote contents, inline code, link labels and image alt text. Descendant text
is visited once: an image nested in a heading/link does not duplicate its label.
Link destinations and titles are not appended. An autolink's displayed label still
contains its URL, and an unresolved reference remains ordinary parser text.

Fenced/indented code blocks and raw HTML blocks are excluded. Use `code_blocks()`
to index code separately. Inline HTML tags and comments add no text; ordinary
parser text between inline tags remains. HTML semantics are not interpreted:
`a<br>b` becomes `ab`, not a rendered line break. This is not visible-text extraction
from a browser DOM and must not be used as an HTML sanitizer.

Block boundaries, thematic breaks and soft/hard line breaks separate nonempty
text runs with one newline. Repeated structural boundaries do not add blank lines,
and no final structural newline is appended. Existing whitespace in parser text
is otherwise retained, including spaces inside inline code. Entities, escapes,
Unicode and CRLF follow the parser's normalization. Empty headings and excluded
blocks add no labels or synthetic text.

The result is an owned string, not source-preserving Markdown. Search offsets in
it are not original UTF-8 offsets: normalization can change both text and length.
Use the existing heading/outline records for source navigation. Highlight mapping,
query matching, case folding, tokenization, ranking and document storage are
separate responsibilities. Output may contain `<` and `&`; escape it when embedding
it in markup rather than treating it as already-safe HTML.

The implementation consumes parser events in one pass without building the summary
or resolving its references. It does not add parser options or dependencies, and
it does not change `summarize`, `outline` or `code_blocks`. Callers remain responsible
for bounding input and allocations. No performance or Wasm-packaging claim follows.

`cargo test -p grunkalunka-markdown --locked --offline` exercises ten integration
tests and the function's documentation example, alongside the existing projections.
Cases cover excluded-only input, block separation, nested labels, inert links,
tight/loose containers, raw HTML, code exclusion, Unicode/CRLF and 2,048 sections.
These tests are included in the existing workspace checks.
