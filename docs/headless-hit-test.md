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
- The topmost candidate is chosen by the reverse of the toy painter's
  **depth-first traversal**: descendants over parents, later siblings over
  earlier siblings. The flat parent-before-child representation need not
  itself be preorder.
- Only visible **border rectangles** are hit regions; margins are excluded.
  Descendants can be hit outside their parent's border because the selected
  toy renderer currently leaves overflow visible.
- Points beyond the viewport return no target, even if document scroll
  extents contain visible boxes there. A separate viewport-scroll translation
  or hit-testing in document coordinates is future work, not implicit.
- The function validates all tree and box inputs before accepting any hit:
  viewport dimensions, exact frame/node count, at most 4,096 nodes, depth
  at most 64, ordered valid parents, no visible descendants of hidden parents,
  checked rectangle arithmetic and content-within-padding-within-border-
  within-margin geometry. Malformed input returns a typed error.
- A non-targetable parent does not prevent a targetable descendant from
  receiving hits, and a non-targetable topmost element permits fallthrough.

## Replay

After the locked Rust workspace is built:

    cargo test -p grunkalunka-layout --locked --offline
    cargo run -q -p grunkalunka-layout --example headless_hit_test --locked --offline

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
