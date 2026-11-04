# Native fixed-size row versus browser bridge

This bridge compares the native row layout engine with actual Chromium, Firefox
and WebKit measurements. Its Rust input and HTML source are separate: the native
example constructs resolved styles directly, and the HTML records browser
rectangles and hit targets. Neither implementation reads the other's geometry.

- [Native input and independent literal assertions](../crates/grunkalunka-layout/examples/native_flex_bridge.rs)
- [HTML source](../tests/browser-oracles/native-flex-bridge.html)
- [Exact comparator](../tests/browser-oracles/native-flex-bridge.mjs)
- [Offline comparator contracts](../tests/browser-oracles/native-flex-bridge.test.mjs)
- [Existing hosted browser suite](../tools/browser-oracles/hosted/native-bridge.test.mjs)

The selected CSS profile uses `display:flex`, `flex-direction:row`,
`justify-content:flex-start`, `align-items:flex-start`,
`align-content:flex-start`, and `flex:none;min-width:0` on the items. The
body uses `display:flow-root` to prevent root margin collapse, matching the
native engine's explicit noncollapsing block flow. A `display:contents`
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
fixed viewport hit probes per phase. Rectangles are converted to document
coordinates using the browser's actual scroll offsets. The boxless wrapper
is omitted from the rectangle map; suppressed boxes remain explicit nulls.

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
| `restored` | Remove all phase declarations and restore scroll y=0 | 162×54 | (174,26) | 80 |

The second item and its child must have null boxes in `hidden`. Relative
displacement preserves the original line assignment and following block
position. Clipping leaves the children's layout rectangles intact while
restricting hit eligibility; page scrolling preserves document geometry
while changing viewport hits.
The native example checks its complete final frame against its initial
frame. The comparator independently requires both native and browser
baseline rectangles and hit targets to be restored.

## Source validation and negative control

The comparator requires the complete ten-phase schema, exact rectangle and
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

The original source, modified source, original native receipt, fresh browser
captures and comparison reports are retained. This checks that a real source
change reaches both geometry and input observations; synthetic offline
receipts do not establish that result.

## Execution and ownership

The native example and both bridge receipts use the existing
`browser-native-bridge.yml` jobs. There is one native build job and the
existing three-engine matrix, with no additional workflow or dependency.
The original block bridge and its three live mutation controls still run.
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
alignment distribution, order, reverse axes, writing modes, text shaping,
subpixel geometry and general positioned/stacking behavior remain outside
the measured profile. The browser-engine changes do not alter A1 application,
worker, session or packaging code, or compiler/runtime interfaces.
