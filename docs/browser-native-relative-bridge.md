# Real-engine positive relative-position offset witness

The [native block/browser observation bridge](browser-native-block-bridge.md)
contains an isolated selected `position: relative` measurement for
**positive, integral** left/top offsets. It checks the headless
`RelativeOffset` geometry from [#255](https://github.com/tseguine/grunkalunka/pull/255)
against independently observed Chromium, Firefox and WebKit rectangles,
computed CSS offsets and real DOM hit targets (source-checked parity
in [#256](https://github.com/tseguine/grunkalunka/pull/256)).

This experiment does not parse HTML/CSS in Rust, and the browser-side
measurements do not supply the native expectations.

## Source-checked fixed normal-flow geometry

The existing 650px `scroll-spacer` begins at document y278 and ends
at y928. Earlier reference groups consume 536px of page flow. Three
new boxes follow within the fixed spacer, leaving the independent
scroll target at y928 and the full document height at 958px.

The reference HTML specifies an ordinary 60×32px block with a
240px left margin and `position: relative; left:12px; top:9px`.
Its 18×8px child is also relatively positioned (`left:4px; top:5px`).
The following 24×6px sibling is unpositioned with 350px left margin.

| Box | Normal-flow x,y | Visual x,y | Border size |
| --- | --- | --- | --- |
| `relative-host` | 240,536 | 252,545 | 60×32 |
| `relative-child` | 240,536 | 256,550 | 18×8 |
| `relative-sibling` | 350,568 | 350,568 | 24×6 |

The sibling's normal-flow y568 follows the **unshifted** host's
32px occupied height, not its visually translated bottom y577.
The native Rust example reconstructs a separate caller-resolved
tree with literal goldens. It does not consume `getBoundingClientRect`
or any browser receipt as an input.

## Actual browser mutation and hit probes

The fixture captures three real CSS phases: `baseline`, inline
`relativeHost.style.left = "20px"` (`parentMoved`), and
`restored` after removing the override. The parent and nested
child shift +8px in x while the later sibling remains at x350,y568.
Each phase captures **three measured** `getBoundingClientRect`
rectangles, computed `position/left/top` for both positioned
boxes, and seven `document.elementFromPoint` probes:

| Probe | Point | Baseline/restored | Parent moved |
| --- | --- | --- | --- |
| Original parent | (253,547) | `relative-host` | null |
| Moved parent | (314,547) | null | `relative-host` |
| Original child | (257,552) | `relative-child` | null |
| Moved child | (280,552) | `relative-host` | `relative-child` |
| Host interior | (294,565) | `relative-host` | `relative-host` |
| Normal-flow sibling | (351,570) | `relative-sibling` | `relative-sibling` |
| Outside witness | (325,548) | null | null |

`null` means no element in this **selected witness set** was
returned. The independent native receipt runs `hit_test` against
recomputed real layout frames for exactly those fixed coordinates,
asserting its expected targets and full restoration equality.
No browser hit result is synthesized from rectangles.

## Validation, provenance and limits

`native-block-bridge.mjs` requires an exact `relativeStates`
map with `baseline`, `parentMoved`, and `restored` for **both**
native and browser sources. Each phase contains three full finite
rectangles, two `relative` computed position/left/top triplets with
resolved nonnegative pixel offsets, and seven known-ID-or-null hits.
Missing, surplus, malformed or nonfinite data are invalid, not
silently skipped. The report identifies each independent
`relative-box`, `relative-computed` or `relative-hit` disagreement.

The unchanged source SHA-256, browser/Playwright version and viewport
800×600 at DPR 1 remain mandatory. Canonical `node:test` tests
check intentional mismatches and rejected receipts; these are not
substitutes for real-browser evidence. The existing native bridge
workflow runs the Rust witness followed by actual Chromium, Firefox
and WebKit comparisons and retains exact source/engine artifacts,
without adding a workflow, status check or runner matrix. Use the
[bridge reproduction instructions](browser-native-block-bridge.md#reproduce-with-a-real-browser).

Only the selected untransformed positive integer normal-flow case is
covered. This is not full CSS `position` conformance: negative
insets, opposing left/right or top/bottom precedence, subpixel
rounding, z-index, stacking contexts, positioned containing blocks,
other formatting contexts, DOM event propagation, and
absolute/fixed/sticky positioning remain outside this experiment.
