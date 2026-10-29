# Real-engine four-sided percentage-padding witness

This selected [native block/browser observation bridge](browser-native-block-bridge.md)
checks the caller-resolved, exact-integer
[`PercentagePadding` layout feature](https://github.com/tseguine/grunkalunka/pull/265)
against fresh Chromium, Firefox and WebKit CSS measurements in the
[three-engine parity unit](https://github.com/tseguine/grunkalunka/pull/267).
The Rust example constructs its own tree and expectations; it neither
parses the source HTML/CSS nor uses browser measurements as native inputs.

## Isolated content-width percentage padding

The retained reference fixture has a fixed-height `scroll-spacer`
from document y278 to y928. All preceding groups end at document y681,
where the new group begins. It consists of:

- `padding-host`: 160px content width, 90px fixed content height,
  4px fixed padding and 2px border. Border box (0,681,172,102).
- `padding-first`: 40×10px content size, 1px border and CSS
  `padding: 10% 5% 15% 20%` under the host.
- `padding-inner`: 12×5px unpositioned child inside the first
  element's content box.
- `padding-next`: 24×6px in-flow sibling of the first child.
- `padding-after`: 20×5px outside sibling after the fixed-height
  host, with a 250px left margin.

All four percentage padding sides resolve against the host's original
**160px CONTENT width**, including *top and bottom*. The first box's
used padding is `top:16px; right:8px; bottom:24px; left:32px`.
Its content starts at (39,704); the border is (6,687,82,52), and
the padding rectangle is (7,688,80,50). The host's 90px content
height is **not** the basis of the vertical percentages.

| Element | Initial x | Initial y | Initial width×height | After reducing top padding |
| --- | ---: | ---: | --- | --- |
| `padding-host` | 0 | 681 | 172×102 | Unchanged |
| `padding-first` | 6 | 687 | 82×52 | 82×44 |
| `padding-inner` | 39 | 704 | 12×5 | y696 |
| `padding-next` | 6 | 739 | 24×6 | y731 |
| `padding-after` | 250 | 783 | 20×5 | Unchanged |

An actual inline style mutation changes
`padding-first.style.paddingTop` from the CSS `10%` declaration to
`5%`, reducing used top padding from 16px to 8px. The
first element's content and nested child move up by 8px, its border
box shrinks by 8px, and the next sibling moves up by 8px in normal
flow. The host is fixed-height, and the outside sibling remains at
y783. Removing the inline declaration restores the entire state.

## Real viewport scroll and hit probes

The selected group is below the 800×600 viewport. The HTML fixture
executes **actual `window.scrollTo(0,350)`** before taking any
percentage-padding observations. It records `getBoundingClientRect`
plus the observed scroll offset to give document-space rectangles,
`getComputedStyle` values for all four used padding sides, and
eight independent `document.elementFromPoint` hit probes.
The original page scroll witness is preserved by restoring scroll to
zero immediately afterward.

| Probe | Viewport point | Initial/restored | Top reduced |
| --- | --- | --- | --- |
| Original inner | (40,355) | `padding-inner` | `padding-first` |
| Shifted inner | (40,347) | `padding-first` | `padding-inner` |
| Original next | (7,391) | `padding-next` | `padding-host` |
| Shifted next | (7,383) | `padding-first` | `padding-next` |
| Left padding | (10,347) | `padding-first` | `padding-first` |
| Host background | (150,400) | `padding-host` | `padding-host` |
| Outside sibling | (251,435) | `padding-after` | `padding-after` |
| Outside witness | (300,401) | null | null |

Native independently builds the same selected geometry using a
caller-resolved `Style::percentage_padding` and asserts literal
rectangles, padding and content geometry before and after the 5%
top change. Native input targets come from the actual
`hit_test_scrolled(frame, nodes, x, y, 0, 350)` implementation.
`null` means no element in the selected five-element witness set
was returned, not that all DOM ancestors are non-targetable.

## Evidence and limitations

The bridge comparator requires `paddingPercentStates` in **both**
native and browser receipts. Each of the mandatory
`original`, `topReduced` and `restored` phases includes an
exact 350px scroll, five finite border rectangles with all fields,
four nonnegative CSS-pixel used-padding strings, and exactly eight
known-ID-or-null input targets. Missing, surplus, malformed or
nonfinite values are invalid, not skipped or treated as zero.
Field differences report `percent-padding-box`,
`percent-padding-computed`, and `percent-padding-hit` with phase
and JSON path.

The fixture source SHA-256, Playwright engine/version, exact 800×600
viewport and DPR 1 continue to be required. Ordinary canonical
`node:test` tests exercise intentional errors and missing fields,
but their synthetic receipts are **not** independent browser evidence.
The existing
[`browser-native-bridge.yml`](../.github/workflows/browser-native-bridge.yml)
runs one native Rust example and separate real Chromium, Firefox and
WebKit comparisons without an extra workflow or runner matrix.
See [the bridge replay instructions](browser-native-block-bridge.md#reproduce-with-a-real-browser).

This is selected nonnegative integer percentage padding in a horizontal
writing mode, **not** full CSS layout conformance. Fractional CSS pixel
rounding, logical writing modes, `calc()` expressions, text shaping,
CSS cascade/parsing, DOM event paths and layout mechanisms outside
normal-flow blocks remain unimplemented.
