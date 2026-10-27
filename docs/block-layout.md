# Headless block-box layout experiment

This is an implementation-side browser experiment, separate from the Playwright
reference oracle suite. The dependency-free Rust crate grunkalunka-layout takes
a caller-built rooted tree and explicitly resolved CSS-pixel styles. It does
not access browser globals, files, font services, WIT capabilities or the app
workstream. The output is the content, padding, border and margin rectangles
of visible blocks, plus scroll extents.

## Supported now

- Normal-flow vertical block stacking in insertion order, one document root.
- Auto widths fill the available content width after edges; explicitly fixed
  content widths may overflow. Auto heights contain laid-out children and
  an optional caller-supplied intrinsic content height.
- Nonnegative integer CSS-pixel margins, borders and padding; no margin collapse.
- Display:none removes an entire subtree. Changing the display value and
  running layout again restores boxes without modifying document ownership.
- Checked arithmetic and bounds: at most 4,096 nodes, depth 64 and
  1–16,384 CSS pixels per viewport dimension. Malformed parent indices
  and overflowing dimensions fail instead of wrapping or silently truncating.

## What is deliberately NOT implemented

This is not CSS 2.1 block formatting conformance. There is no HTML parser, CSS
tokenizer, selectors/cascade, text shaping, inline formatting, negative or auto
margins, collapsed margins, floats, positioning, percentages, scrollbars,
painting, hit testing, event loop or native window. Input text heights are
provided by a separate caller, not guessed. All coordinates are unsigned
integers. This is a small, experimental implementation component, NOT an
A1 application or a commitment to the final renderer architecture.

## Reproduction

After installing the repository Rust toolchain and fetching locked dependencies:

```sh
cargo test -p grunkalunka-layout --locked --offline
cargo run -p grunkalunka-layout --example headless_outline --locked --offline
```

The tests exercise independent literal coordinates for an outline-like tree,
display toggling and sibling reflow, overflow beyond fixed parent height,
zero-width overconstraint, malformed trees and integer overflow. The example
prints rectangles; it does not paint pixels.

## Next independently verifiable unit

An explicit, separately owned adapter can parse a very small known semantic
HTML/CSS subset (for example main/h1/ol/li/p, display and fixed dimensions),
construct the tree, and pass in measured text heights from an explicit service.
Parser choice, glyph metrics and host presentation remain open. As required by
the browser roadmap, the crate is independent of a browser shell and can be
exercised by a headless caller. This step does not authorize editing the
compiler, Markdown/WIT code or the unselected application pilot.
