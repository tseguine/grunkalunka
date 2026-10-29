# Inherited visibility: source-checked native/browser witness

This selected fixture compares `Visibility::{Inherit,Hidden,Visible}` in the
headless Grunkalunka layout, hit and paint pipeline with independent
Chromium, Firefox and WebKit DOM observations. It is **not** a CSS parser,
an implementation of `pointer-events`, or a claim of general browser
rendering conformance.

## Fixture and independent native geometry

Inside the original fixed-height 650px scroll spacer, below the existing
400×80px dimension-bound group, the source HTML adds a 40×25px
`visibility: hidden` block with a 300px left margin. It contains a
14×6px child explicitly styled `visibility: visible` and a 12×5px
child inheriting the hidden value. A 20×5px visible sibling follows the
hidden parent in normal flow, also offset 300px from the left.

The Rust sample independently constructs a caller-resolved block tree
with the same already reviewed preceding fixed heights, and asserts the
following literal border rectangles:

| Element | x | y | width | height |
| --- | ---: | ---: | ---: | ---: |
| `visibility-host` | 300 | 358 | 40 | 25 |
| `visibility-child` | 300 | 358 | 14 | 6 |
| `visibility-inherited` | 300 | 364 | 12 | 5 |
| `visibility-sibling` | 300 | 383 | 20 | 5 |

These boxes, the document scroll height of 958px and the earlier scroll
target's y=928 remain identical when only the visibility style toggles.

## Three independent observation phases

The HTML source records `getBoundingClientRect`, browser-resolved
`getComputedStyle(...).visibility` and actual
`document.elementFromPoint` targets in three states: originally hidden,
after setting `visibility-host.style.visibility = "visible"`, and after
removing that inline override. The native sample separately re-runs
the layout with explicit caller-resolved visibility and uses
`hit_test` with a fixed independent targetability policy.

| Probe | Viewport point | Hidden | Visible override | Restored |
| --- | --- | --- | --- | --- |
| Child | (301, 359) | `visibility-child` | `visibility-child` | `visibility-child` |
| Parent area | (318, 360) | null | `visibility-host` | null |
| Inherited child | (301, 366) | null | `visibility-inherited` | null |
| Following sibling | (301, 385) | `visibility-sibling` | `visibility-sibling` | `visibility-sibling` |

The comparator requires a complete `visibilityStates` record with all
four rectangles, four computed visibility Booleans and four hit targets
for each of the three states. It rejects partial, nonfinite, unknown or
extra fields and reports exact geometry, computed visibility and hit
differences independently. The original fixture SHA-256, browser engine,
Playwright version, viewport and DPR1 provenance remain mandatory.

## Replay and boundaries

This extends the **existing** `browser-native-bridge.yml` matrix and the
canonical offline `node:test` regression suite; no new workflow or
status check is added. The native Rust evidence and fresh browser captures
are archived separately. Offline synthetic receipts exercise comparison
failures but are not substituted for live browser results.

Only selected normal-flow rectangles and `elementFromPoint` targets are
compared. This does not exercise DOM event dispatch, CSS selectors or
cascade resolution, `visibility: collapse`, accessibility, text shaping,
clipping, stacking contexts, focus, animations, pointer capture or
`display: contents`. Replay using the same native example and optional
Playwright commands in
[the native bridge guide](browser-native-block-bridge.md#reproduce-with-a-real-browser).
