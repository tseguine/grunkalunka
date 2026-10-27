# Markdown inspection

`grunkalunka-markdown` 0.0.1 is a standalone source-string library. Call
`summarize(&str)` to obtain headings, links and images, or `code_blocks(&str)` for
separate inert code-block records. It performs no filesystem
or network access and needs no browser, renderer, resolver or host capability.

```rust
let summary = grunkalunka_markdown::summarize("# Notes\n\n[Next](next.md)");
assert_eq!(summary.headings[0].level, 1);
assert_eq!(summary.links[0].destination, "next.md");
```

Each list is in source-start order. Headings contain a level and projected text;
links and images contain projected label/alt text, destination and title. Empty
headings and labels are retained. Descendant text and inline code concatenate;
soft and hard breaks become `\n`. Nested image alt text contributes once to the
image and each enclosing link/heading. Formatting markers and inline HTML tags
do not contribute text, while ordinary text between inline HTML tags does.

Every summary record includes a half-open `Range<usize>` into the original UTF-8 source,
covering the complete construct rather than only its projected text. Heading
ranges include markers and, when present in the parser's range, a terminating
newline. Reference-link ranges cover the use site, not the definition. Entities,
escapes, titles and destinations may be normalized by the parser; original source
is available through the range. Consumers retaining ranges must retain the same
source. This summary is a projection, not a lossless converter or editor model.

Parsing uses exactly `pulldown-cmark = 0.13.5`, with Cargo default features disabled
and runtime `Options::empty()`. This selects the pinned parser's CommonMark
defaults; it is not an independent conformance certification. Extensions such as
wikilinks, heading attributes, strikethrough, tables and task-list syntax are not
enabled. Unresolved references stay text. Standalone reference definitions,
literal code and raw HTML blocks add no summary records; HTML links/images are not parsed.

`outline(&str)` adds parent indices and section ranges to the same heading
projection for document navigation. See [heading outlines](markdown-outline.md)
for level-jump, duplicate-label, container and original-byte-range policies.

`code_blocks(&str) -> Vec<CodeBlock>` is additive and leaves `Summary` and
`summarize` unchanged. Each owned record contains `text: String`,
`info: Option<String>` and `span: Range<usize>`. Records follow source order,
including empty fenced blocks. `None` identifies indented code; `Some("")`
identifies a fence with empty info. Other `Some` values contain the entire
parser-normalized info string, including additional words. No language or command
is selected from it. Inline code and raw HTML blocks produce no code-block record.

Code-block text concatenates parser `Text` events with `push_str`, without extra
trimming, dedenting, unescaping, newline insertion or Markdown parsing. The parser
itself can remove structural indentation/container prefixes, normalize CRLF to LF,
and trim/unescape info strings. Body entities, escapes, HTML and Markdown remain
literal parser text. A final newline is not invented when the parser omits one.

A code-block span is exactly the pinned parser's `Start(CodeBlock)` range: half-open
original UTF-8 byte offsets. It may exclude first-line indentation/container
prefixes and the newline following a closing fence. A contiguous slice can include
later container prefixes even though projected text omits them. It is neither a
body-only range nor a promise to own complete physical lines. Preserve the unchanged
source when retaining spans; normalized text length must not be used to adjust them.
This projection does not provide round-trip editing or a lossless document model.

Code-block text and info are inert data. Labels such as `bash`, `mermaid` or
`dataview` trigger no execution, highlighting, rendering, resource resolution or
host capability. The API has the same caller-owned input/resource bounds as the
summary API and adds no parser features or dependencies.

Destinations are data, including relative paths, external URLs and unsafe schemes.
The adapter does not open, validate, rewrite or authorize them. Consumers supply
their own URL/path policy and capability checks before following any reference.
It does not render or sanitize content. Parsing allocates memory proportional to
the parser and projected output; callers must bound untrusted input/resources.
The [portability check](markdown-portability.md) compares native and core-Wasm
projections on fixed fixtures. It does not establish a WIT application boundary.

The upstream parser is MIT licensed, inspected at
[v0.13.5](https://github.com/pulldown-cmark/pulldown-cmark/tree/v0.13.5).
Its disabled defaults omit the CLI and HTML renderer. The selected parser graph
is pulldown-cmark, bitflags, memchr and unicase; `Cargo.lock` pins exact releases.
The repository's license script screens metadata for the whole locked graphs,
including build/target packages; that is not a redistribution notice audit.

Verification from the workspace root:

```text
cargo test -p grunkalunka-markdown --locked --offline -j 2
cargo clippy -p grunkalunka-markdown --all-targets --locked --offline -j 2 -- -D warnings
cargo fmt --all --check
node scripts/licenses.mjs
```

Tests cover ATX/Setext headings, Unicode byte spans, inline/reference/autolinks,
inert unsafe URLs, titles, empty constructs, nested images/links/headings,
line breaks, literal code, raw HTML and the disabled extension policy.
Code-block tests additionally pin empty/unterminated and mixed fence forms,
indentation and tabs, normalized full info, literal bodies, LF/CRLF/EOF behavior,
Unicode/container byte ranges, source order and unchanged mixed summary results.
