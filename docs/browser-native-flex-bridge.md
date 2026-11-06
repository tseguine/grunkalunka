# Native fixed-size rows, alignment and overlap versus browsers

This bridge compares the native row layout engine with actual Chromium, Firefox
and WebKit measurements. Its Rust input and HTML source are separate: the native
example constructs resolved styles directly, and the HTML records browser
rectangles and hit targets. Neither implementation reads the other's geometry.
Selected overlap and item-alignment phases also assert actual native canvas pixels, checking that
native painting and hit testing agree on the visible item. Browser pixel parity
is not measured by this bridge.

- [Native input and independent literal assertions](../crates/grunkalunka-layout/examples/native_flex_bridge.rs)
- [HTML source](../tests/browser-oracles/native-flex-bridge.html)
- [Exact comparator](../tests/browser-oracles/native-flex-bridge.mjs)
- [Offline comparator contracts](../tests/browser-oracles/native-flex-bridge.test.mjs)
- [Existing hosted browser suite](../tools/browser-oracles/hosted/native-bridge.test.mjs)

The selected CSS profile uses `display:flex`, `flex-direction:row`,
`align-content:flex-start`, and `flex:none;min-width:0` on the items. Baseline
main-axis and cross-axis alignment is `flex-start`; the alignment phases
independently set `safe flex-end`, `safe center`, or main-axis `space-between`.
Per-item phases set `align-self` while retaining the row's separate default.
The body uses `display:flow-root` to prevent root margin collapse, matching
the native engine's explicit noncollapsing block flow. A `display:contents`
wrapper contributes two items without contributing a box. The wrapper's
otherwise nonzero width, padding and border must have no geometric effect.

## Independent witness

The viewport is 800 by 600 CSS pixels at DPR 1. The row begins at (20,20),
has 150 pixels of content width, 4 pixels of padding and a 2 pixel border.
Its three item outer widths are 64, 64 and 42 pixels; their outer heights
are 24, 42 and 18 pixels. The second item includes asymmetric margins and
a child whose width is 50% of the item's content width. A following block
tests the row's consumed normal-flow height. A body minimum height of 1200
pixels permits an actual page scroll.

Each phase starts from the original declarations, with only its named
changes applied. The source records six named border rectangles and nine
fixed viewport hit probes per phase: 180 rectangle slots and 270 hit slots
across 30 phases. Rectangles are converted to document coordinates using the
browser's actual scroll offsets. The boxless wrapper is omitted from the
rectangle map; suppressed boxes remain explicit nulls.

| Phase | Change from the baseline | Host border size | Third item border origin | Following block y |
| --- | --- | --- | --- | --- |
| `nowrap` | No wrap, column gap 10, row gap 8 | 162×54 | (174,26) | 80 |
| `wrap` | Enable wrapping | 162×80 | (26,76) | 106 |
| `wide` | Wrap with content width 220 | 232×54 | (174,26) | 80 |
| `columnGap` | Wrap with column gap 24 | 162×86 | (114,58) | 112 |
| `rowGap` | Wrap with row gap 18 | 162×90 | (26,86) | 116 |
| `hidden` | Wrap with the second item hidden | 162×36 | (100,26) | 62 |
| `relative` | Wrap with the second item displaced by (8,9) | 162×80 | (26,76) | 106 |
| `clipped` | Wrap with content height 36 and padding-box clipping | 162×48 | (26,76) | 74 |
| `scrolled` | Wrap and actual page scroll y=10 | 162×80 | (26,76) | 106 |
| `justifyEnd` | Width 220, justify `safe flex-end` | 232×54 | (204,26) | 80 |
| `justifyCenter` | Width 220, justify `safe center` | 232×54 | (189,26) | 80 |
| `spaceBetween` | Width 230, justify `space-between` | 242×54 | (214,26) | 80 |
| `alignEnd` | Width 220, height 60, align items `safe flex-end` | 232×72 | (174,68) | 98 |
| `alignCenter` | Width 220, height 60, align items `safe center` | 232×72 | (174,47) | 98 |
| `wrapAlign` | Wrap, justify `space-between`, align items `safe center` | 162×80 | (26,76) | 106 |
| `wrapSingle` | Wrap, width 220, height 60, align items `safe flex-end` | 232×72 | (174,50) | 98 |
| `safeOverflow` | Height 20, justify and align items `safe flex-end` | 162×32 | (174,28) | 58 |
| `overlapClipped` | Same as `safeOverflow`, with padding-box clipping | 162×32 | (174,28) | 58 |
| `nestedOverlap` | Second item becomes a row with top margin 0; inner item is 100×8 | 162×52 | (174,26) | 78 |
| `nestedOverflow` | Same as `safeOverflow`; second item becomes a row with a 100×40 inner item | 162×32 | (174,28) | 58 |
| `relativeAlign` | Width 220, height 60, center both axes, second item displaced by (8,9) | 232×72 | (189,47) | 98 |
| `clippedAlign` | Width 216, height 28, justify center, align items end, padding-box clipping | 228×40 | (187,36) | 66 |
| `scrolledAlign` | Same alignment as `alignCenter`, actual page scroll y=10 | 232×72 | (174,47) | 98 |
| `selfAlignMixed` | Width 220, height 60, row center; first item start, second item end, third auto | 232×72 | (174,47) | 98 |
| `selfAlignAuto` | Same as `selfAlignMixed`, second item reset to auto | 232×72 | (174,47) | 98 |
| `selfAlignWrap` | Wrap, width 220, height 60, row center; first item end, third item start | 232×72 | (174,26) | 98 |
| `selfAlignOverflow` | Width 220, height 20, row start; items center/end/center | 232×32 | (174,27) | 58 |
| `selfAlignClipped` | Width 220, height 36, clip; items end/center/center, second item displaced down 15 | 232×48 | (174,35) | 74 |
| `selfAlignScrolled` | Same as `selfAlignClipped`, actual page scroll y=10 | 232×48 | (174,35) | 74 |
| `restored` | Remove all phase declarations and restore scroll y=0 | 162×54 | (174,26) | 80 |

All center/end declarations in the table use the `safe` modifier. A phase
wraps only where explicitly stated. At width 220 the row's 190px occupied
main size leaves 30px: end alignment adds 30px to each item's x position,
while center adds 15px. At width 230, `space-between` adds 20px to each
existing 10px gap. With wrapping, distribution is calculated independently
per line; the one-item second line in `wrapAlign` remains at main-start.

The 60px definite height fills the single nowrap line in `alignEnd`, so
the first, second and third item border y coordinates are 62, 46 and 68.
`wrapSingle` intentionally puts the same items in one *wrapping* line:
with `align-content:flex-start`, its natural 42px line height gives y
coordinates 44, 28 and 50 instead. The asymmetric second-item margins
count toward the space used before alignment, and its inner child must
follow the aligned subtree.

`safeOverflow` has negative main-axis free space and two items taller
than its 20px line. Safe alignment falls back to start for those overflowing
dimensions while the smaller third item still moves to cross-end. The
following block retains its baseline 6px top margin, placing it at y=58.
The `gap-wrap` probe at (118,62) lies inside both that block and the overflowing
second item; the flex item must paint above the ordinary block and receive
the hit as `row-b`.

`overlapClipped` preserves every layout rectangle from `safeOverflow`.
Its padding clip ends at y=50, excluding the second item at that probe, so
`gap-wrap` must instead hit `row-after`. The `nowrap` probe at (180,30)
lies on the host's right border, outside the descendant clip, and must hit
`row-host` despite the third item's unchanged overlapping rectangle.

The two nested phases keep the second item in the outer row and make its
child a fixed-size item of a nested row. The inner item explicitly uses
`flex:none;min-width:0`, so its 100px width overflows its 50px containing
content box. In `nestedOverlap`, (180,30) lies inside both that inner item
and the later outer item `row-c`. The later outer item must win: nested
items retain their parent item's atomic painting scope. In `nestedOverflow`,
the 40px-tall inner item reaches (118,62) and must paint above the following
ordinary block, producing `row-inner`. Together these probes reject both
global postponement of every nested item and deferral limited to the local
flex container. All inner, second-item and following-block declarations
reset before the next independent phase.

`clippedAlign` separately makes the same `gap-wrap` probe
distinguish padding-box clipping after alignment: (118,62) lies inside the
second item's unchanged border rectangle, below the padding clip's bottom
edge at y=58, and above the following block's y=66. Its target must be null;
without clipping it would hit `row-b`.

The per-item phases independently exercise [the native override contract](native-flex-rows.md#per-item-overrides).
In `selfAlignMixed`, the first item stays at y=26, the second moves to y=46,
and the third uses the row's center default at y=47. The boxless wrapper is
given its own `align-self: safe flex-end` and `align-items: flex-start`; neither
changes the actual items' selection of their generated parent row. An
`align-self` declaration on the second item's ordinary child also has no effect
of its own. Resetting the second item to `auto` moves its border and child up
exactly nine pixels while preserving every other rectangle.

`selfAlignWrap` has one actual wrapped line of natural height 42 despite the
60px container. The first item's end override therefore places it at y=44;
the third item's start override keeps it at y=26. `selfAlignOverflow` makes
the first two items fall back to start independently while centering the
shorter third item by one pixel. The clipped pair composes independent item
alignment with a relative offset and actual viewport scrolling. The second
item's border remains at y=43 in both receipts, but its (118,62) viewport probe
changes from `row-b` to null when scrolling moves that probe below the
padding clip and above the following block.

The second item and its child must have null boxes in `hidden`. Relative
displacement preserves the original line assignment and following block
position. Clipping leaves the children's layout rectangles intact while
restricting hit eligibility; page scrolling preserves document geometry
while changing viewport hits.
The native example checks its complete final frame against its initial
frame. The comparator independently requires both native and browser
baseline rectangles and hit targets to be restored, and requires document
geometry to agree between `wrap`/`scrolled`, `alignCenter`/`scrolledAlign`,
`selfAlignClipped`/`selfAlignScrolled`, and `safeOverflow`/`overlapClipped`.

### Original paint-order regression

The first alignment witness at
[`70c2043153cbc3181b2a69c3788e0f653c3e0a05`](https://github.com/tseguine/grunkalunka/commit/70c2043153cbc3181b2a69c3788e0f653c3e0a05)
retained the following block's baseline 6px top margin in `safeOverflow`.
That placed its top at y=58, overlapping the second flex item. The
[independent three-engine run](https://github.com/tseguine/grunkalunka/actions/runs/41909076116)
failed with exactly one baseline difference in every engine:
`safeOverflow.hits.gap-wrap` was `row-after` natively and `row-b` in the
browser. Every measured rectangle and every other hit agreed. Both flex
source controls rejected the same additional baseline mismatch.

The native implementation at that revision used tree-order painting and hit testing.
[CSS Flexbox item painting](https://www.w3.org/TR/css-flexbox-1/#painting)
instead uses inline-block painting, with ordinary in-flow block backgrounds
and borders appearing earlier in the
[CSS painting order](https://www.w3.org/TR/CSS22/zindex.html#painting-order).
The alignment-only stack temporarily increased the following block's margin
to 16px, moving it clear of the item while retaining safe-alignment coverage.
This witness restores the original 6px margin and directly exercises the
bounded non-positioned flex-item ordering behavior, with clipping and nested
item scopes tested independently.
[Issue #533](https://github.com/tseguine/grunkalunka/issues/533) retains the
original reproduction and completion criterion. Those original 144 rectangle
slots, 216 hit slots, and exact live source controls remain included in the
expanded witness.

### Native overlap and item-alignment pixels

The native example paints distinct opaque item colors through `paint_scrolled`
and reads actual canvas pixels. Overlap probes share the points that
distinguish browser hit ordering; two extra item probes distinguish a child's
paint before and after its parent's auto reset:

| Phase | Native pixel | Visible item | Expected RGB |
| --- | --- | --- | --- |
| `safeOverflow` | (118,62) | `row-b` | (0,0,255) |
| `overlapClipped` | (118,62) | `row-after` | (255,128,0) |
| `nestedOverlap` | (180,30) | `row-c` | (128,0,128) |
| `nestedOverflow` | (118,62) | `row-inner` | (0,255,0) |
| `selfAlignMixed` | (110,52) | `row-inner` | (0,255,0) |
| `selfAlignAuto` | (110,52) | `row-b` | (0,0,255) |
| `selfAlignWrap` | (180,30) | `row-c` | (128,0,128) |
| `selfAlignOverflow` | (118,62) | `row-b` | (0,0,255) |
| `selfAlignClipped` | (118,62) | `row-b` | (0,0,255) |
| `selfAlignScrolled` | (118,62) | Background | (255,255,255) |

Each assertion fails before the native JSON receipt is emitted if painting
disagrees. The browser evidence remains independent rectangle and hit
observations; these native pixel regressions do not introduce screenshot
comparison, a new receipt schema, or an additional dependency.

## Source validation and negative controls

The comparator requires the complete 30-phase schema, exact rectangle and
probe IDs, exact integer coordinates, the fixed viewport, and source-hashed
Playwright provenance identifying the engine family and browser version.
Missing samples, unknown targets, unexpected nulls, nonfinite/fractional
geometry, missing source metadata and inconsistent baseline channels fail.
All numeric comparisons are exact; there is no tolerance setting.

After baseline parity, a separate browser instance executes a new read-only
HTML snapshot in which the `columnGap` phase's actual CSS assignment changes
from 24 to 32 pixels. It is compared with the unchanged native receipt while
validating provenance against the modified source bytes. Exactly two
differences must result:

- `columnGap.boxes.row-c.x`: 114 becomes 122.
- `columnGap.hits.gap-wrap`: `row-c` becomes `row-host`.

A second fresh browser capture changes only `justifyEnd`'s assignment from
`safe flex-end` to `flex-start`. The complete alternate capture must remain
valid and produce exactly these nine differences against the unchanged
native receipt:

| Observation in `justifyEnd` | Original | Mutated |
| --- | --- | --- |
| `boxes.row-a.x` | 56 | 26 |
| `boxes.row-b.x` | 135 | 105 |
| `boxes.row-inner.x` | 138 | 108 |
| `boxes.row-c.x` | 204 | 174 |
| `hits.a` | `row-host` | `row-a` |
| `hits.b` | `row-a` | `row-inner` |
| `hits.nowrap` | `row-b` | `row-c` |
| `hits.gap-wrap` | `row-host` | `row-b` |
| `hits.gap` | `row-a` | `row-host` |

The exact source replacement includes the phase's sample assignment, so
the separate overflow phase's identical alignment declaration is retained.

A third fresh capture changes only the following block's explicit
`position:static` assignment in `safeOverflow` to `position:relative`, with
no offsets. This changes its CSS painting category without changing any
rectangle. Exactly one hit must change: `safeOverflow.hits.gap-wrap` switches
from `row-b` to `row-after`. Every other phase, all 180 rectangle slots,
and restoration remain unchanged. The positioned variant is a browser-only
negative control; native input remains in the non-positioned profile and
does not acquire general positioned precedence or `z-index` support.

A fourth capture changes only the second item's `align-self` in
`selfAlignMixed`, from `safe flex-end` to `flex-start`. The container and all
other items keep their declarations. Exactly three differences must result:

| Observation in `selfAlignMixed` | Original | Mutated |
| --- | --- | --- |
| `boxes.row-b.y` | 46 | 28 |
| `boxes.row-inner.y` | 49 | 31 |
| `hits.b` | `row-host` | `row-inner` |

The exact source replacement includes the phase's sample assignment, leaving
the separate auto-reset and overflow declarations unchanged.

All four mutations validate provenance against their own source bytes.
A hash mismatch or any unexpected observation cannot count as successful
detection. The original source, all four modified sources, original native
receipt, fresh browser captures and comparison reports are retained. These
controls check that real source changes reach geometry and input observations;
synthetic offline receipts do not establish those results.

## Execution and ownership

The native example and both bridge receipts use the existing
`browser-native-bridge.yml` jobs. There is one native build job and the
existing three-engine matrix, with no additional workflow or dependency.
The original block bridge and its three live mutation controls still run,
along with all four flex controls. Alignment and overlap use the existing fixture, example,
comparator and hosted suite without changing the workflow or command catalog.
Both PR/push source filters and the independent impact-workflow descriptor
include the flex source and comparator inputs.

Offline contracts use the canonical browser test command:

```sh
node scripts/check-browser-oracles.mjs
```

With the optional reference engines already installed, a separate manual
capture can be reproduced with a fresh output directory:

```sh
mkdir -p target/native-flex
cargo run --locked --offline -p grunkalunka-layout --example native_flex_bridge > target/native-flex/native.json
node tools/browser-oracles/collect.mjs --fixture=tests/browser-oracles/native-flex-bridge.html --engine=all --output-dir=target/native-flex/browser
node tests/browser-oracles/native-flex-bridge.mjs target/native-flex/native.json target/native-flex/browser/chromium.json tests/browser-oracles/native-flex-bridge.html
```

The comparator exits with 0 for equality, 1 for a measured difference and 2
for invalid or incompatible evidence. Hosted results remain separate from
offline contracts and local captures using different engine versions.

This witness does not implement HTML/CSS parsing or establish general flex
conformance. Flex grow/shrink, intrinsic item width, automatic minimum sizing,
other alignment distributions, cross-line alignment, baseline alignment,
order, reverse axes, writing modes, text shaping, subpixel geometry and
general positioned/stacking behavior remain outside
the measured profile. The browser-engine changes do not alter A1 application,
worker, session or packaging code, or compiler/runtime interfaces.
