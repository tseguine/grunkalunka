# Real-engine percentage margins relative to containing width

The existing [native block/browser bridge](browser-native-block-bridge.md)
includes an isolated normal-flow CSS percentage-margin experiment for the
headless `PercentageMargins` input. The native implementation
([#258](https://github.com/tseguine/grunkalunka/pull/258)) selects exact
integer percentage margins, and the follow-up source-checked witness
compares **actual** Chromium, Firefox and WebKit values. Neither
source parses the other or derives independent expectations from its
observations.

## Fixed 200px containing width and reviewed positions

The original 650px fixed-height `scroll-spacer` begins at document y278,
ends at y928 and retains the separate offscreen document target at y928.
Earlier reference groups consume y0–574. The new 200px content-width host
has 4px padding, 2px border and a fixed 90px content height, so its
border box is `(0,574,212,102)`. The selected child declares:

`width:40px; height:8px; margin: 10% 5% 15% 20%`

All **four** percentages use the host's 200px **content width**, including
`margin-top` and `margin-bottom`. This gives used margins of
`top:20px; right:10px; bottom:30px; left:40px`. Vertical margins do
not use the host's 90px height.

| Element | x | y | width | height |
| --- | ---: | ---: | ---: | ---: |
| `percent-host` | 0 | 574 | 212 | 102 |
| `percent-first` | 46 | 600 | 40 | 8 |
| `percent-next` | 6 | 638 | 24 | 6 |
| `percent-after` | 250 | 676 | 20 | 5 |

The next in-flow child has no margin of its own, retaining the first
child's 30px bottom separation without introducing two nonzero adjacent
margins that could collapse in real CSS. Border/padding edges separate
the first child's 20px top margin from its parent. The host's fixed
90px content height and border/padding isolate the following sibling
at y676. All the selected cases intentionally avoid unsupported
margin collapsing, subpixel rounding and negative margins.

A real inline style mutation replaces only `percent-first`'s
`margin-left:20%` with `margin-left:10%`, changing the first
border rectangle x from 46 to 26, and the used left margin
from 40px to 20px. Top/right/bottom, next sibling and following
sibling stay unchanged. Removing the inline style restores all
original measured values.

## Actual viewport scrolling and hit observations

Because this group starts below the initial 800×600 viewport, the
reference fixture first executes `window.scrollTo(0,200)`.
It captures *live* CSS rectangles through `getBoundingClientRect`
and adds the **observed** viewport scroll offsets to report
document-space rectangles. Computed used margins come from
`getComputedStyle`, and hit targets come from eight real
`document.elementFromPoint` probes in that scrolled viewport.
Each phase reports `scrollY:200`, and the viewport is restored
to y0 before the original scrolling witness is executed.

| Probe | Viewport point | Original/restored | Changed margin |
| --- | --- | --- | --- |
| Baseline only | (75,401) | `percent-first` | `percent-host` |
| Shifted only | (27,401) | `percent-host` | `percent-first` |
| First center | (50,402) | `percent-first` | `percent-first` |
| Top margin | (47,390) | `percent-host` | `percent-host` |
| Next in flow | (7,440) | `percent-next` | `percent-next` |
| Host bottom | (180,450) | `percent-host` | `percent-host` |
| Following sibling | (251,478) | `percent-after` | `percent-after` |
| Outside | (350,450) | null | null |

The independent Rust example reconstructs a caller-resolved normal-flow
tree starting at y574, asserting the literal rectangles and used
margin boxes. It obtains native targets with
`hit_test_scrolled(frame, …, 0, 200)`, not with source-side DOM
measurements. `null` means the hit was not one of the four
deliberately selected witness elements; the root/spacer can still
exist in the browser.

## Evidence contract, replay and limits

The existing comparator requires **both** receipts to contain
`percentStates.original/changed/restored`, each with a real 200px
scroll, four finite named border rectangles, all four computed used
margin strings, and eight named target-or-null probes. It rejects
missing/malformed/surplus/nonfinite values and reports independent
`percent-margin-box`, `percent-margin-computed` and
`percent-margin-hit` discrepancies with phase and field.

The original fixture SHA-256, fixed 800×600 DPR1 viewport,
Playwright/browser versions and source provenance remain mandatory.
Ordinary `node:test` negative controls verify comparator sensitivity
but are **not** evidence of actual CSS parity. The existing
`browser-native-bridge.yml` workflow compiles the independent Rust
example and compares freshly collected Chromium, Firefox and WebKit
observations without another CI workflow/job/matrix. Refer to the
[native bridge reproduction instructions](browser-native-block-bridge.md#reproduce-with-a-real-browser).

Only selected, nonnegative, integral, horizontal-writing-mode margins
are covered; this is not general browser CSS margin conformance.
Percentage calculations with fractional results, signed margins,
margin collapse, writing modes, arbitrary CSS calc expressions,
text shaping, browser script event propagation or the final renderer
architecture are outside the selected experiment.
