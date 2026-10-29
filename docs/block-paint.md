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
  `Frame`. `Display::Contents` removes the wrapper's paint box but retains
  its descendants in normal depth-first paint order. `visibility:hidden`
  instead preserves layout: the painter
  skips that node's own paints, but still visits descendants so an explicitly
  visible child can paint.
- Caller-resolved `DescendantClip::PaddingBox` clips descendants'
  rectangles against all clipping ancestors; the parent still paints its
  own border/background normally. Nested and disjoint intersections are
  checked before applying the existing viewport clip.
- Invalid root/parent indices, excessive depth, malformed nested box
  rectangles and mismatched `Frame`/paint-list/visibility/clip/contents
  lengths fail closed.
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

## Clipping limitation

The optional native `descendant_clip` controls only block background and
border paint. `Canvas::blit_mono` remains an explicit caller-owned
operation on document-space coordinates and does not automatically inherit
the element's clips. Layout and document scroll extents also remain
unchanged. The integration test `tests/descendant_clip.rs` verifies
exact output pixels for a clipped child and nested/disjoint clips alongside
the headless hit tester. This is not CSS overflow, compositing or a
general glyph clipping implementation.

## Boxless wrapper painting

The optional paint module does not draw a `Display::Contents` node's
background, border or padding: it has no principal box. Its children
still paint in the original tree order, subject to inherited visibility
and any earlier ancestor padding-box clips. An inactive `display:none`
subtree still does not paint, and malformed boxless metadata fails closed.
The ordinary native `tests/display_contents.rs` exercises exact paint
pixels and sibling order without claiming full browser conformance.

## Deliberate omissions

There is no general HTML/CSS parsing, selector matching, computed style,
font shaping, glyph rasterization, inline layout, alpha/compositing, gradients,
shadows, image paint, z-index/stacking contexts, scrollbars, viewport-to-screen
compositor, hit testing, accessibility or scripting. This simplified painter
does not establish CSS Painting or screenshot equivalence. A future host
could separately supply text glyphs and a presentation surface; that decision
is independent of WIT grants, the runtime/compiler, and the browser shell.


## Typed semantic document to RGB pixels

The optional headless `semantic_ppm` example composes the independently
implemented `layout_semantic`, `layout`, and `paint` boundaries. It accepts
a caller-built body/main/heading/list/paragraph tree and explicit heights for
each text leaf, then prints a 320x120 PPM in which colored blocks indicate the
text anchors. There is no HTML parser, font rasterizer, application script, or
ambient file/network access. Reducing the list to `hidden` produces a smaller
layout and moves the paragraph to a new location.

```sh
cargo run -q -p grunkalunka-layout --example semantic_ppm --locked --offline > outline.ppm
cargo test -p grunkalunka-layout --locked --offline
```

The `tests/visibility.rs` integration tests additionally assert inherited
visibility, explicit visible descendants and exactly hidden or painted
RGB pixels without changing any block geometry.

The independent `tests/semantic_paint.rs` test checks ten exact expanded
pixel samples, four collapsed-state samples, the two visible text-anchor lists,
and that the PPM bytes change. Its expected coordinates/colors are manually
specified, not copied from the rendered frame. This is an implementation-side
integration demonstration, **not** CSS screenshot conformance, glyph display
or a selected A1 application.

## Explicit packed monochrome ink

The standalone [bitpacked ink contract](mono-bitmap-ink.md) adds an optional
Canvas::blit_mono method for **caller-supplied** MSB-first row-padded 1-bit masks.
It overlays opaque RGB ink at selected bits while leaving zero bits and earlier
box paint intact, with validated input and viewport clipping. This does not
implement fonts, Unicode shaping, text measurement or glyph rasterization.

## Explicit global viewport scrolling (experimental)

The frame already tracks document-space scroll extents. A host may now call
`paint_scrolled(frame, nodes, clear, scroll_x, scroll_y)` and
`hit_test_scrolled(frame, nodes, viewport_x, viewport_y, scroll_x, scroll_y)`
with the **same explicit offsets**. Paint and hit selection use a document
coordinate translated from the viewport; neither operation mutates layout,
scrolls implicitly, or dispatches input events. The existing `paint` and
`hit_test` functions remain zero-offset wrappers.

`Canvas::blit_mono` still accepts document-space bitmap positions and now
clips masks on left/top as well as right/bottom in scrolled views. The frame's
scroll extents bound offsets; out-of-range offsets return errors rather than
silently clamping. New integration tests verify exact pixel and target
coordinates, zero-offset parity, partial bitmap clipping, and invalid offset
rejection without reference-browser involvement.

This is a **single viewport origin only**, not CSS `overflow`, nested
scroll containers, scrollbars, scroll anchoring, sticky positioning, wheel
input, or a browser scroll lifecycle. In particular, `scroll_width` and
`scroll_height` describe this toy layout's positive margin-box extents,
not general DOM `scrollWidth` / `scrollHeight` semantics.
