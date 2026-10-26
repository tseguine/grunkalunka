# Heading-level navigation outlines

`grunkalunka_markdown::outline(&str) -> Vec<OutlineEntry>` adds document-navigation
metadata to the [existing Markdown inspection](markdown.md) library. It is usable
without the JS/TS compiler, a renderer, filesystem access or a host capability.

Every entry contains the existing `Heading` projection, a `parent: Option<usize>`
index into the returned vector, and a `section_span: Range<usize>`. Entries retain
source order. A parent is the nearest preceding heading with a numerically smaller
level. A heading without one is a root, even when its level is greater than one.
Missing levels do not create synthetic headings. Empty and duplicate labels remain
separate entries; the API does not generate URL anchors or persistent identifiers.
Filtering or reordering the vector requires remapping its parent indices.

A section starts at its heading's original byte start and ends immediately before
the next heading of equal or smaller level, or at the original source length.
Parent sections include child sections. Text before the first heading belongs to
no section; a document with no headings returns an empty vector. Keep the unchanged
UTF-8 source with all ranges, including when its line endings are CRLF.

This is a heading-level navigation convention, not an HTML outline algorithm or a
Markdown container tree. Headings inside lists and block quotes participate in the
same source-ordered level sequence. Container starts/ends do not reset ancestry.
Parser heading starts may omit container prefixes, so section slices can include
prefix bytes from the following physical line. They are not safe deletion ranges
or a source-preserving editing model.

The implementation reuses `summarize`, including its pinned parser, projection and
inert-link policy. It currently collects the full summary before consuming its
headings. Outline construction then uses an iterative stack with at most six open
heading levels; there is no recursive output tree. No parser extensions, package
versions, dependency graph, compiler behavior or runtime ABI are changed.

`crates/grunkalunka-markdown/tests/outline.rs` covers gaps, siblings, multiple roots,
preamble, empty/duplicate labels, formatted/Setext headings, container policy,
UTF-8/CRLF/EOF ranges and literal code/HTML exclusions. It also compares all 1,296
four-heading level sequences against independent backward/forward searches and
checks 2,048 duplicate siblings. The public function has a runnable documentation
example. These tests are included in the existing workspace CI, without a new
workflow or publication step.

Run `cargo test -p grunkalunka-markdown --locked --offline` after fetching the
repository's locked dependencies. Full verification remains `node scripts/check.mjs`;
this feature does not establish Wasm packaging or a complete note-taking client.
