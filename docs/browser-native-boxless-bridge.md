# Source-checked native display:contents versus real browser geometry

The existing independent [native block bridge](browser-native-block-bridge.md)
now measures a small CSS `display: contents` wrapper before, during and
after a real DOM `display: block` override. The purpose is to establish
selected browser evidence for Grunkalunka's **caller-resolved** boxless
normal-flow feature, not HTML parsing, CSS cascade or general layout
conformance.

## Independent fixture and native goldens

The reference HTML appends a 40×24px content-box wrapper inside the
existing fixed-height 650px scroll spacer. It declares `display: contents`
and also has a 2px top margin, a 200px left margin, 2px border and 3px
padding. Those edges do not create a principal box while the element
is `display: contents`. Its children are 24×10px and 12×6px blocks;
the following sibling is 20×5px with a 200px left margin.

The reference browser measures each child and sibling using actual
`getBoundingClientRect()` calls. The wrapper's computed `display`
value and `getClientRects().length !== 0` record whether it has a
principal box, independently of the Rust model. The latter constructs
a caller-resolved block tree with the preceding groups' reviewed fixed
occupied heights, and asserts these literal border rectangles:

| Node | Contents: x,y,w,h | Block: x,y,w,h |
| --- | --- | --- |
| First child | 0,415,24,10 | 205,422,24,10 |
| Second child | 0,425,12,6 | 205,432,12,6 |
| Following sibling | 200,431,20,5 | 200,451,20,5 |
| Wrapper | No box | 200,417,50,34 |

The wrapper's explicit dimensions, margin, border and padding affect
normal block flow only in the second state. Its child block coordinates
change, and the following sibling moves down 20px. With the original
fixed-height scroll spacer unchanged, the earlier document height
remains 958px and the independent scroll target starts at y=928.

## Three real-engine phases and hit probes

The HTML source records the original contents state, changes
`contentsWrapper.style.display` to `"block"`, then removes the inline
override and captures the restored state. Each phase records actual
`document.elementFromPoint` results for seven fixed viewport points.

| Probe | Point | Contents/restored | Block |
| --- | --- | --- | --- |
| Flattened first child | (1,416) | first child | null |
| Flattened second child | (1,427) | second child | null |
| Wrapper border | (201,418) | null | wrapper |
| Repositioned first child | (206,424) | null | first child |
| Initial sibling | (202,432) | sibling | wrapper |
| Repositioned sibling | (202,452) | null | sibling |
| Outside | (251,431) | null | null |

All IDs in the right-hand columns refer to the selected witness
elements. A null means no selected element is hit, not that the
browser returned no DOM node.

The independently projected Rust sample calls `layout` and
`hit_test` in all three states. It must find no box for the
contents wrapper, find the normal block rectangle after the override,
restore the entire frame exactly, and retain original scroll extents.

The comparator requires a complete `boxlessStates` map, including
all three child/sibling rectangles, computed display mode, actual
principal-box presence and seven hit probes for each phase. It
reports exact field-level differences and rejects missing, malformed,
extra or nonfinite evidence. Source SHA-256, native/browser identities,
viewport and browser provenance remain mandatory. Canonical offline
`node:test` regressions check controlled differences and invalid
receipts; only the hosted Chromium, Firefox and WebKit jobs supply
independent browser observations.

## Limits

This selected comparison does not implement or claim CSS
`display: contents` accessibility mappings, DOM event bubbling,
anonymous inline formatting, replaced-element exceptions,
pseudo-elements, flex/grid items, box fragmentation or non-integer
CSS layout. It reuses the existing native bridge workflow and
runner matrix without adding CI jobs. Reproduce using the commands
in the [native bridge guide](browser-native-block-bridge.md#reproduce-with-a-real-browser).
