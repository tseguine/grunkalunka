# Native border-box sizing versus real browser observations

The existing independent [native block-box bridge](browser-native-block-bridge.md)
now carries a third, separate three-element geometry group selected to verify
explicit `box-sizing: border-box` lengths in the native headless experiment.
It is **not** an HTML parser, CSS cascade, layout engine conformance suite or
proof that the native renderer supports the browser's entire box model.

## Measured source versus caller-resolved native inputs

The source-checked `tests/browser-oracles/native-block-bridge.html` fixture
appends an isolated `#border-host` after the previous three-box and
percentage-height groups. The host declares a 120×80 CSS-pixel **border box**
with 2px borders, 4px padding and a 10px top margin. Its content box is
108×68. Two nested elements request 50%/50% **border-box** sizing with
independent 1px borders and 2px padding, then 25%/25% ordinary content-box
sizing. All dimensions resolve to exact integer CSS pixels; no subpixel
rounding is involved.

The real Chromium/Firefox/WebKit source reads `getBoundingClientRect()`
for these three elements. Separately, `grunkalunka-layout` builds a
caller-resolved Rust tree (with fixed placeholders for the preceding layout)
and asserts these independent expected border rectangles:

| Element | x | y | width | height |
| --- | ---: | ---: | ---: | ---: |
| `border-host` | 0 | 198 | 120 | 80 |
| `border-first` | 6 | 207 | 54 | 34 |
| `border-second` | 6 | 246 | 27 | 17 |

The Rust example also asserts the host's 108×68 and first child's 48×28
content sizes. These are native goldens, not numbers extracted from browser
captures. The existing comparator independently requires the exact
`borderBoxes` map in both source receipts. It reports a field-specific
`border-box` difference or rejects missing, extra, nonfinite or otherwise
incompatible evidence. Source SHA-256 and fixed 800×600 DPR1 provenance
checks remain mandatory.

## Execution and limits

This uses the existing `browser-native-bridge.yml` workflow, not another
matrix or Actions check. The same native Rust artifact is captured once and
compared against fresh Chromium, Firefox and WebKit engines. The original
hide/restore, hit-target and percentage-height channels remain independently
required. Their negative mutation controls must continue to detect exactly
their reviewed CSS deviations, with no unplanned border-box differences.

Reproduction uses the commands in
[the native bridge guide](browser-native-block-bridge.md#reproduce-with-a-real-browser).
The canonical offline `node:test` suite checks schema, comparison and
controlled mismatch behavior without claiming to have executed browsers.
The native `box_sizing.rs` integration suite covers additional geometry
without HTML/CSS interpretation. This deliberately excludes fractional
percentage lengths, margin collapse, min/max and intrinsic sizing, inline
layout, nested scrolling, paint and event semantics.
