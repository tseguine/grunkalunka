# Markdown inspection

`grunkalunka-markdown` 0.0.1 is a standalone source-string library. Call
`summarize(&str)` to obtain headings, links and images. It performs no filesystem
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

Every record includes a half-open `Range<usize>` into the original UTF-8 source,
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
literal code and raw HTML blocks add no records; HTML links/images are not parsed.

Destinations are data, including relative paths, external URLs and unsafe schemes.
The adapter does not open, validate, rewrite or authorize them. Consumers supply
their own URL/path policy and capability checks before following any reference.
It does not render or sanitize content. Parsing allocates memory proportional to
the parser and projected output; callers must bound untrusted input/resources.
No Wasm target build or WIT boundary is demonstrated by this native library unit.

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
