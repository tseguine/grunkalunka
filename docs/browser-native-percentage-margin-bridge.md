# Real-engine percentage margins relative to containing width

The existing [native block/browser bridge](browser-native-block-bridge.md)
includes an isolated normal-flow CSS percentage-margin experiment for the
headless `PercentageMargins` input. The native implementation
([#258](https://github.com/tseguine/grunkalunka/pull/258)) selects exact
integer percentage margins; the independent three-engine witness
([#261](https://github.com/tseguine/grunkalunka/pull/261)) compares real
Chromium, Firefox and WebKit results without a CSS parser or generating
native expectations from browser observations.

## Fixed 200px containing width and reviewed positions

The original 650px fixed-height `scroll-spacer` starts at document y278,
ends at y928 and retains the separate offscreen scroll target at y928.
Earlier source-checked groups consume y0–574. The new 200px content-width
host has 4px padding, 2px border and 90px fixed content height, yielding
a border rectangle of `(0,574,212,102)`. The selected child declares:

`width:150px; height:8px; margin: 10% 5% 15% 20%`

All **four** percentages use the host's original 200px CONTENT width,
including `margin-top` and `margin-bottom`. Thus the selected used
margins are `top:20px; right:10px; bottom:30px; left:40px`.
Vertical margins do not depend on the parent's 90px content height.

| Element | Original x | Original y | Width | Height | After mutation y |
| --- | ---: | ---: | ---: | ---: | ---: |
| `percent-host` | 0 | 574 | 212 | 102 | 574 |
| `percent-first` | 46 | 600 | 150 | 8 | 590 |
| `percent-next` | 6 | 638 | 24 | 6 | 628 |
| `percent-after` | 250 | 676 | 20 | 5 | 676 |

The child fits the host's horizontal margin/width equation exactly:
40px + 150px + 10px = 200px. This avoids the **overconstrained**
fixed-width case, in which CSS may redistribute horizontal used margins
and Firefox's `getComputedStyle` exposes a larger right margin. The
native model does not implement that surplus redistribution.

The next in-flow child has zero vertical margins, preserving the first
child's bottom separation without adding two nonzero adjacent margins
that could collapse. Host border/padding separate the first child's
top margin from the parent, and the host's fixed height protects the
outside sibling.

A real inline mutation replaces only `percent-first`'s
`margin-top:10%` with `margin-top:5%`, decreasing its used top margin
from 20px to 10px. The first child and the following in-flow child
move upward by 10px. Horizontal margins and x/width values remain
unchanged; the fixed-height host and outside sibling do not move.
Removing the inline declaration restores all observations.

## Actual viewport scroll and hit observations

Because this group begins below the 800×600 viewport, reference
source explicitly executes `window.scrollTo(0,200)`. It captures real
`getBoundingClientRect` values, adds the **observed scroll offsets**
to report document-space coordinates, and reads
`getComputedStyle` for all four used margin values. Each phase
also records eight actual `document.elementFromPoint` hits at
viewport coordinates; afterward the source restores scroll to y0
before the existing document scroll witness runs.

| Probe | Viewport point | Original/restored | Changed top |
| --- | --- | --- | --- |
| Baseline first | (75,401) | `percent-first` | `percent-host` |
| Shifted first | (75,392) | `percent-host` | `percent-first` |
| Top margin | (75,385) | `percent-host` | `percent-host` |
| Baseline next | (7,440) | `percent-next` | `percent-host` |
| Shifted next | (7,430) | `percent-host` | `percent-next` |
| Host bottom | (180,450) | `percent-host` | `percent-host` |
| Following sibling | (251,478) | `percent-after` | `percent-after` |
| Outside witness | (350,450) | null | null |

The independent Rust example builds a caller-resolved normal-flow tree
at document y574, checks literal border/margin rectangles, and obtains
all expected targets with `hit_test_scrolled(frame, …, 0, 200)`
instead of DOM measurements. `null` means none of the four selected
elements is a hit; other browser ancestors may still exist.

## Validation, reproduction and limits

The comparator requires both native and browser receipts to include
`percentStates.original/changed/restored`. Every phase must have
an exact scrollY of 200, four complete finite document-space
rectangles, four resolved CSS-pixel margin strings, and eight known
target-or-null hits. Missing, surplus, malformed, nonfinite or unknown
fields make the receipt invalid. Exact disagreements report
`percent-margin-box`, `percent-margin-computed` and
`percent-margin-hit` with phase and field.

The original fixture SHA-256, fixed viewport at DPR1, browser
version and Playwright provenance remain mandatory. Canonical
`node:test` negative controls test intentional mismatch detection,
but are not real-engine evidence. The existing independent
`browser-native-bridge.yml` workflow runs the Rust sample and three
fresh browser comparisons; no new workflow, status check, runner or
matrix is introduced. Refer to the
[replay instructions](browser-native-block-bridge.md#reproduce-with-a-real-browser).

Only selected nonnegative whole-pixel percentages in horizontal writing
mode are covered. Fractional results, negative margins, overconstrained
fixed-width surplus redistribution, margin collapsing, other writing
modes, CSS calc expressions, DOM event dispatch, text layout and general
CSS conformance remain outside this experiment.
