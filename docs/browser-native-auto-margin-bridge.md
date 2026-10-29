# Real-engine horizontal auto-margin geometry and hit witness

The existing [native block comparison bridge](browser-native-block-bridge.md)
contains a selected **integer-pixel**, normal-flow CSS horizontal
`margin-left/right: auto` experiment added after native support in
[#246](https://github.com/tseguine/grunkalunka/pull/246) and parity in
[#250](https://github.com/tseguine/grunkalunka/pull/250).
It checks the independent caller-resolved Rust layout against actual
Chromium, Firefox and WebKit geometry, computed used margins and DOM hit
targets. It does not parse HTML/CSS or infer native expectations from the
observed browser output.

## Fixed, isolated reference group

The same 650px fixed-height `scroll-spacer` begins at document y278 and
ends at y928. Earlier groups finish at y467. The new source-checked
`auto-host` is 200px wide and 64px tall, with four normal-flow children.
The next sibling follows at y531. Thus the original scroll-target
position y928 and full document extent 958px are unchanged.

| CSS element | x | y | width | height | Used horizontal margins |
| --- | ---: | ---: | ---: | ---: | --- |
| `auto-host` | 0 | 467 | 200 | 64 | 0, 0 |
| `auto-centered` | 80 | 467 | 40 | 8 | 80, 80 |
| `auto-left` | 140 | 475 | 50 | 7 | 140, 10 |
| `auto-right` | 12 | 482 | 30 | 6 | 12, 158 |
| `auto-limited` | 60 | 488 | 80 | 6 | 60, 60 |
| `auto-sibling` | 0 | 531 | 20 | 5 | 0, 0 |

The center child has `width: 40px; margin: 0 auto`. The one-sided
children have `margin-left: auto; margin-right: 10px` and
`margin-left: 12px; margin-right: auto` respectively. The constrained
child uses `width: auto; max-width: 80px; margin: 0 auto`; a computed
width limitation leaves 120px for the two auto margins. All declared
heights and horizontal dimensions have integral CSS-pixel solutions.

For every phase, the browser fixture reads real
`getBoundingClientRect()` results for all six boxes and
`getComputedStyle(...).marginLeft/marginRight` for all four children.
The independently built Rust example asserts fixed literal border-box
rectangles and derives used margin strings from the engine's own
margin/border geometry. The native fixture is not supplied any of the
live browser measurements.

## Real style mutation and hit probes

The original center state has `margin-left: auto`; an actual inline
style change sets `margin-left: 20px` while retaining the auto right
margin. The center box moves from x80 to x20 with used right margin
140px, while the host, other children and sibling stay fixed. Removing
the inline declaration restores the original frame.

Each phase captures **eight actual** `document.elementFromPoint`
queries, normalized to the six selected witness IDs or explicit null.
The native receipt checks the same points independently with
`hit_test` on its recomputed frame.

| Probe | Viewport point | Centered/restored | Left fixed |
| --- | --- | --- | --- |
| Center original | (81, 468) | `auto-centered` | `auto-host` |
| Center shifted | (21, 468) | `auto-host` | `auto-centered` |
| One-sided left | (141, 476) | `auto-left` | `auto-left` |
| One-sided right | (13, 483) | `auto-right` | `auto-right` |
| Width-limited | (61, 489) | `auto-limited` | `auto-limited` |
| Parent only | (195, 525) | `auto-host` | `auto-host` |
| Following sibling | (1, 533) | `auto-sibling` | `auto-sibling` |
| Outside group | (221, 468) | null | null |

Importantly, selected **margin space is not the element's hit box**:
the parent can be hit in the unoccupied horizontal margins around its
centered child.

## Fail-closed evidence, limits and replay

The existing `native-block-bridge.mjs` comparator requires a complete
`autoMarginStates` map with `centered`, `leftFixed`, `restored` in
**both** sources. Each phase needs exactly six finite border rectangles,
four pairs of nonnegative CSS-pixel used-margin strings and eight
known-or-null target observations. Missing, malformed or unexpected
fields invalidate the comparison; independent discrepancies are reported
as `auto-margin-box`, `auto-margin-computed` and
`auto-margin-hit` with phase, element/probe and JSON path.

The source SHA-256, engine/browser versions, selected 800×600 DPR1
viewport and Playwright provenance remain mandatory. Canonical Node
regressions verify missing and intentionally incorrect receipts, but
synthetic captures **do not count as** live browser evidence. The same
independent `browser-native-bridge.yml` workflow runs the Rust fixture
and real Chromium, Firefox and WebKit comparison; no workflow, job,
matrix or dependency was added for this witness. See
[reproduction commands](browser-native-block-bridge.md#reproduce-with-a-real-browser).

The headless engine is still limited to nonnegative, exact integer
CSS pixels. This witness does not claim correctness for negative or
fractional used margins, percentage margins, writing direction and
overconstraint, margin collapsing, other formatting contexts, DOM event
dispatch, text layout, or CSS parsing/cascade.
