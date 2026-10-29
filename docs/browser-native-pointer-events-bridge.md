# Real-engine pointer-events inheritance witness

The existing [native block comparison bridge](browser-native-block-bridge.md)
now includes a selected HTML `pointer-events` inheritance experiment. This
checks Grunkalunka's caller-resolved native input target policy against fresh
Chromium, Firefox and WebKit DOM observations without parsing CSS or implementing
DOM event dispatch. CSS UI Level 4 defines `pointer-events: none` as excluding
the element from hit testing while allowing an explicitly `auto` descendant
to participate; the test isolates exactly that behavior.

## Source-checked CSS group and independent native geometry

The original fixed-height 650px scroll spacer begins at y278, ends at
y928 and already contains earlier reference groups. A new 30×26px
`pointer-events: none` parent begins at (350,436). Its first child
inherits `none` and measures 10×6px, while the 14×7px next child
explicitly uses `pointer-events: auto`. A 20×5px ordinary sibling follows
the parent in flow. The checked reference rectangles are:

| Element | x | y | width | height |
| --- | ---: | ---: | ---: | ---: |
| `pointer-host` | 350 | 436 | 30 | 26 |
| `pointer-inherited` | 350 | 436 | 10 | 6 |
| `pointer-restored` | 350 | 442 | 14 | 7 |
| `pointer-sibling` | 350 | 462 | 20 | 5 |

Reference HTML obtains these from real `getBoundingClientRect()` calls
and reads each `getComputedStyle(...).pointerEvents`, with Boolean
eligibility recorded for `auto`. The native Rust example constructs its
own resolved tree from reviewed fixed occupied heights, emits the same
rectangle/channel shape and independently asserts the literal geometry,
a 958px document scroll height and the unchanged y928 scroll target.

## Actual pointer phase transitions

The HTML source records `pointer-events:none`, changes only the
parent's inline style to `auto`, and restores the original declaration.
For each state it samples `document.elementFromPoint` at six
fixed viewport points, retaining only IDs of the four selected elements.

| Probe | Point | Parent none | Parent auto |
| --- | --- | --- | --- |
| Inherited child | (351,438) | null | `pointer-inherited` |
| Parent area | (375,439) | null | `pointer-host` |
| Explicit auto child | (351,444) | `pointer-restored` | `pointer-restored` |
| Lower parent area | (375,452) | null | `pointer-host` |
| Following sibling | (351,464) | `pointer-sibling` | `pointer-sibling` |
| Outside parent | (385,440) | null | null |

Null means no element within this witness set was returned. The
normal-flow rectangles do not move across the none/auto/restored
transitions; the native sample separately calls its existing
`hit_test` with `Frame::pointer_events` and independently supplied
caller targetability for those four nodes.

The comparator requires the exact `pointerStates` map in all three
phases, with complete computed eligibility, rectangles and hit results.
It rejects missing, malformed, extra or nonfinite evidence and reports
individual `pointer-computed`, `pointer-box` and `pointer-hit`
differences. The original fixture SHA-256, engine version, viewport,
device pixel ratio and automation provenance remain mandatory.

## Limits and replay

The same independent native example and the same three hosted browser
engines run through `browser-native-bridge.yml`; no new workflow or
matrix is added. Offline canonical `node:test` regressions require
intentional discrepancy detection and fail-closed schema behavior, but
cannot substitute for actual browser observations.

Only a small, normal-flow hit-target experiment is covered. It does
not implement DOM event paths, bubbling/capture, pointerenter/leave,
pointer capture, keyboard navigation, focus, SVG-specific values,
accessibility or browser CSS cascade. Use the
[native bridge guide](browser-native-block-bridge.md#reproduce-with-a-real-browser)
for reproduction.
