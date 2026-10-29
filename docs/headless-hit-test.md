# Headless block hit testing

The optional grunkalunka-layout::hit_test module consumes the same already-laid-out
Frame and explicit parent-before-child tree used by the toy RGB painter. Given
a viewport point, it selects the **topmost targetable border box** under that
point. It uses no browser globals, input devices, event dispatch, CSS parser,
fonts, images or external dependencies.

## Selected input/geometry contract

- Each HitNode supplies its parent index (matching the layout tree) and an
  explicit targetable Boolean. An unpainted box may still be targetable.
  This is caller input policy, **not** a CSS pointer-events implementation.
  Resolved inherited `Frame::visibility` is a separate, necessary filter;
  hidden nodes cannot be selected, but explicitly visible descendants can.
- The topmost candidate is chosen by the reverse of the toy painter's
  **depth-first traversal**: descendants over parents, later siblings over
  earlier siblings. The flat parent-before-child representation need not
  itself be preorder.
- Only visible **border rectangles** are hit regions; margins are excluded.
  Descendants can be hit outside their parent's border because the selected
  toy renderer currently leaves overflow visible.
- Points beyond the viewport return no target, even if document scroll
  extents contain visible boxes there. The separately implemented
  `hit_test_scrolled` translates viewport points by a checked global
  document-space offset; nested scroll containers are not represented.
- The function validates all tree and box inputs before accepting any hit:
  viewport dimensions, exact frame/node/visibility count, at most 4,096 nodes, depth
  at most 64, ordered valid parents, no visible descendants of hidden parents,
  checked rectangle arithmetic and content-within-padding-within-border-
  within-margin geometry. Malformed input returns a typed error.
- A non-targetable parent does not prevent a targetable descendant from
  receiving hits, and a non-targetable topmost element permits fallthrough.

## Replay

After the locked Rust workspace is built:

    cargo test -p grunkalunka-layout --locked --offline
    cargo run -q -p grunkalunka-layout --example headless_hit_test --locked --offline

The independent `tests/visibility.rs` additionally verifies that
`visibility:hidden` retains boxes but suppresses hit targets, that
an explicitly visible descendant can receive hits, and that
`display:none` remains different.

The original example has overlapping siblings and a descendant whose box
extends beyond its fixed-height parent. Six independent native tests specify
exact topmost targets, disabled-input fallthrough, margin exclusion, hidden
subtree omission, overflow hits, and malformed tree/frame failure behavior.
No reference browser supplies the expected node indices.

## Intentional limitations

This is a bounded **input-targeting primitive**, not DOM Event dispatch,
relatedTarget adjustment, pointer capture, CSS transforms, z-index or stacking
contexts, overflow clipping modes, scrolling, shadow DOM retargeting, pseudo-
elements, text/glyph selection, or web platform conformance. The existing
browser-oracle Event tests remain reference behavior, not implementation tests.
Actual mouse/touch input, coordinate translation, event paths and dispatch
would be separate explicitly measured components.


## Host-controlled selection and repaint

The additional `click_reflow_ppm` example composes the existing, independently
tested typed semantic tree, headless hit tester, block layout and RGB painter.
It first lays out a heading/list/paragraph at 320×120; a fixed synthetic input
at (26,42) selects the first list item. An explicit **caller policy** then hides
the parent list, relays out the same tree and writes the recomputed PPM image.
This intentionally has no DOM event dispatcher, JS callback, trusted OS input,
CSS selector, browser shell or capability-grant semantics.

```sh
cargo test -p grunkalunka-layout --locked --offline
cargo run -q -p grunkalunka-layout --example click_reflow_ppm --locked --offline > toggled.ppm
```

The independent `tests/headless_input_reflow.rs` checks literal pre/post hit
targets, full visible text-anchor indices, five explicitly chosen pixel colors
in both layouts, the paragraph's new y coordinate and changed PPM bytes. A
second test disables hit targets while checking that painting remains unchanged.
Expected values are specified outside the implementations; no reference-browser
results are imported. This is a **toy host interaction** rather than DOM dispatch,
script-lowered source behavior, or an accepted A1 application.
