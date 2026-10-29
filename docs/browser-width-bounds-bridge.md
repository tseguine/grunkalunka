# Native min/max-width versus real browser geometry

This is a source-checked **selected-fixture** comparison of the bounded
`grunkalunka-layout` min/max-width implementation, not a CSS parser or a
general browser layout compatibility claim. It runs through the existing
[native-to-browser observation bridge](browser-native-block-bridge.md) and
does not allocate additional CI jobs or browser matrices.

## Independent evidence

The HTML fixture adds a width-limited group **inside** the already fixed-height
650px scroll spacer. A separate 400px-wide containing block ensures that
percentage constraints have a stable source-defined basis independent of
scrollbar widths on different hosts. Its first child has these declarations:

- `width: 400px`, `min-width: 45%`, `max-width: 30%`;
- `box-sizing: border-box`, 2px border, 4px padding and a 60px height.

Both percentages refer to the **400px containing content width**: the
maximum is 120px and the minimum is 180px. Because the minimum wins an
incompatible maximum, the expected border-box width is 180px and the
content width is 168px. A nested `width: 50%` content-box child therefore
measures 84px. A sibling with `width: 250px; max-width: 100px` measures
100px. All dimensions are integral.

The reference fixture uses independent real browser
`getBoundingClientRect()` calls to produce a `widthBounds` map.
The Rust `native_block_bridge` example independently builds a
caller-resolved tree using `WidthBound`, asserts its content widths and
original scroll target position, and emits the following literal border
rectangle goldens:

| Box | x | y | width | height |
| --- | ---: | ---: | ---: | ---: |
| `bounds-host` | 0 | 278 | 180 | 60 |
| `bounds-first` | 6 | 284 | 84 | 10 |
| `bounds-second` | 6 | 294 | 100 | 8 |

The existing comparator requires these three named rectangles, checks
every finite coordinate/dimension, and reports field-specific
`width-bound` differences. Source SHA-256 and engine/viewport
provenance remain mandatory. Missing, partial or extra records cannot
be silently interpreted as agreement.

The new group fits entirely inside the prior fixed-height spacer. The
document scroll height remains 958px and the independent target remains
at y=928: its before/scrolled/restored receipt and the earlier block,
reflow, hit and negative-control channels must continue to pass with no
relaxed tolerance.

## Evidence limits and replay

The same native Rust artifact and three hosted reference browsers
(Chromium, Firefox, WebKit) are exercised by the existing bridge workflow.
The offline canonical `node:test` checks cover strict-schema rejection
and controlled single-field differences; they do not claim browser
execution. Use the native bridge guide's reproduction commands to obtain
fresh independent captures.

This tests only explicitly resolved integer CSS-pixel constraints,
content-box/border-box semantics, contradictory min/max precedence,
and one nested percentage. It does not cover intrinsic min/max-content,
fractional pixel rounding, margin collapse, auto-margin distribution,
selectors, transforms or a general CSS formatting algorithm.
