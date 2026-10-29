# Native min/max-height versus real browser geometry

This bounded witness extends the existing
[native-to-browser comparison bridge](browser-native-block-bridge.md) to
check explicitly resolved height limits against fresh Chromium, Firefox
and WebKit measurements. It is not CSS parsing or general height layout
conformance.

## Deliberately selected source geometry

The browser fixture uses the existing fixed-height 650px scroll spacer,
which already contains a 400×80px definite containing block and the
width-bound witness. It appends a sibling with:

- `height: 90px`, `min-height: 75%`, `max-height: 50%`;
- `box-sizing: border-box`, 2px borders, 4px padding and 80px width.

Both percentage bounds refer to the containing content height of 80px,
so the minimum is 60px and the maximum is 40px. With minimum taking
precedence, the used border-box height is 60px and content height 48px.
A nested child with `height: 50%` must use that 48px content height,
producing 24px; the following fixed 12px child occupies the next slot.

The original source observes real `getBoundingClientRect()` values for
these three elements. Separately, the Rust example creates the same
resolved integer geometry using `HeightBound`, checks the definite
containing box, and asserts independent border-rectangle goldens:

| Element | x | y | width | height |
| --- | ---: | ---: | ---: | ---: |
| `height-limited` | 0 | 338 | 80 | 60 |
| `height-limited-first` | 6 | 344 | 30 | 24 |
| `height-limited-second` | 6 | 368 | 20 | 12 |

The comparison requires a complete `heightBounds` map, validates each
finite rectangle field and produces exact, path-specific `height-bound`
differences. Missing, malformed, partial or surplus fields are rejected
rather than silently normalized. Browser source SHA-256, viewport,
device scale and Playwright provenance remain mandatory.

## Regression scope

This witness remains inside an already fixed-height ancestor. Its
additional descendants do not affect the original 958px document
scroll extent or the existing scroll target, block/reflow, width-bound
and hit-target channels. The existing real CSS width, pointer-events
and percentage-height source-mutation controls continue to require
exactly their original differences.

Normal `node:test` regressions exercise malformed receipts and deliberate
single-field differences; the actual engine jobs retain independent
source-checked captures under the existing `browser-native-bridge.yml`.
There is no additional workflow, matrix or hosted job.

This supports one definite containing height with contradictory percentage
limits and a percentage child. It does not establish browser behavior
for indefinite auto-height ancestors, intrinsic min/max-content, fractional
CSS pixels, margin collapse, vertical alignment, clipping, scrollbar
layout or arbitrary CSS property resolution.
