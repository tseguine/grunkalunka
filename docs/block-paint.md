# Headless RGB paint-surface experiment

The **optional** `grunkalunka_layout::paint` module consumes an existing
`Frame` from the headless block-layout engine, plus a parallel set of
`PaintNode` entries. It creates a bounded, opaque RGB pixel buffer entirely
in memory. This is the first diagnostic **visible-pixel output** for the toy
renderer, not full HTML/CSS rendering, image decoding, font rendering, native
window display, or a commitment to the final rendering architecture.

## Admitted behavior

- Solid RGB backgrounds fill the padding box (including content); a second
  optional RGB paint covers the four border strips without coloring margins.
- Parents paint before descendants, siblings in their declared order, using
  explicit tree traversal rather than assuming the flat parent-before-child
  input is itself preorder. Later opaque paint covers earlier pixels.
- `display:none` and descendants stay absent, following the layout
  `Frame`. Children with overflow visible can paint beyond parent borders,
  but coordinates are clipped to the output viewport.
- Invalid root/parent indices, excessive depth, malformed nested box
  rectangles and mismatched `Frame`/paint-list lengths fail closed.
  Integer arithmetic is checked. Pixel allocation is limited to **4 million
  RGB pixels** and viewport dimensions are already bounded by the core.
- `Canvas.pixel(x,y)` permits diagnostic sampling. `Canvas.ppm()` returns
  uncompressed binary PPM **P6** bytes, without filesystem or window authority.

## Run it

After building with the repository Rust toolchain and locked workspace:

```sh
cargo test -p grunkalunka-layout --locked --offline
cargo run -q -p grunkalunka-layout --example headless_ppm --locked --offline > boxes.ppm
```

The example produces a small 64x48 RGB raster: a bordered panel with two
colored content rows. It does not draw their text. A PPM viewer can display
the binary result; there is no PNG encoder, graphics crate or window dependency.

Unit tests independently specify selected exact pixel colors, border strips,
paint ordering for non-preorder input, hidden subtree exclusion, viewport
clipping, PPM bytes, invalid geometry and memory quotas. The raw `Frame`
and box invariants come from the separate layout implementation. No browser
engine generates the expected pixel colors or paints.

## Deliberate omissions

There is no general HTML/CSS parsing, selector matching, computed style,
font shaping, glyph rasterization, inline layout, alpha/compositing, gradients,
shadows, image paint, z-index/stacking contexts, scrollbars, viewport-to-screen
compositor, hit testing, accessibility or scripting. This simplified painter
does not establish CSS Painting or screenshot equivalence. A future host
could separately supply text glyphs and a presentation surface; that decision
is independent of WIT grants, the runtime/compiler, and the browser shell.
