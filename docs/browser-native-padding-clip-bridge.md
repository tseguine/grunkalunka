# Native padding-box clipping versus real browser hits

This extends the existing [source-checked native block bridge](browser-native-block-bridge.md)
with a selected test of already-resolved descendant clipping. It compares
actual Chromium, Firefox and WebKit DOM geometry and `elementFromPoint` hits
against an independent `grunkalunka-layout` caller-resolved tree. The native
feature deliberately clips painting and hit targeting only; this is **not**
general CSS `overflow` conformance or a new browser engine.

## Reference source and independent native expectations

The fixed 650px-tall scroll spacer ends at y=928 and already holds the
width/height-bounds and inherited-visibility witnesses. This comparison
adds a 20×12 content-box parent at x=100, y=388, with 2px borders,
3px padding and `overflow: clip`. Its overflowing 40×18 child begins
at (105,393). A 10×5 visible sibling immediately follows the parent.

The independent Rust sample represents preceding occupied heights using
caller-chosen fixed geometry and asserts:

| Box | x | y | Width | Height |
| --- | ---: | ---: | ---: | ---: |
| Parent border | 100 | 388 | 30 | 22 |
| Parent padding (native clip region) | 102 | 390 | 26 | 18 |
| Child border | 105 | 393 | 40 | 18 |
| Following sibling border | 100 | 410 | 10 | 5 |

All box sizes, the 958px document scroll height, and the previous
scroll target at y=928 remain unchanged when the parent toggles clipping.
The browser source independently reads `getBoundingClientRect()` and
computed `overflow-x` from those HTML bytes; it does not consume the
Rust expected values.

## Three real-engine source states

The original fixture records the `overflow: clip` state, changes
`clip-parent.style.overflow` to `visible`, and restores the original
declaration. The same seven viewport points are sampled in each phase
using actual `document.elementFromPoint`.

| Probe | Point | Clipped | Unclipped |
| --- | --- | --- | --- |
| Inside child | (106,395) | child | child |
| At padding edge | (127,395) | child | child |
| In parent's right border | (128,395) | parent | child |
| Outside parent | (135,395) | null | child |
| In parent's bottom border | (106,409) | parent | child |
| Following sibling | (105,412) | sibling | sibling |
| Parent's own top border | (101,389) | parent | parent |

The native sample independently calls `hit_test` in clipped, visible,
and restored states. It retains all computed box rectangles, flags,
document extents and original scroll target, and requires the restored
frame to equal the initial frame exactly.

The existing comparator requires complete `descendantClipStates` maps,
including three exact rectangles, computed mode and all seven hit targets
in each phase. It reports field-level differences in mode, geometry and
hit targeting; missing, malformed, unexpected or nonfinite source
evidence is invalid. Original HTML SHA-256, engine/viewport and
Playwright provenance checks remain mandatory.

## Scope and reproduction

Use the existing hosted `browser-native-bridge.yml` workflow and the
commands in [the native bridge guide](browser-native-block-bridge.md#reproduce-with-a-real-browser).
The canonical offline Node tests validate controlled disagreements and
fail-closed receipt shapes but cannot substitute for fresh real engines.

This comparison tests one CSS `overflow: clip` reference shape only,
without nested DOM scroll containers, clip-margin, radii, transforms,
fractional pixels, text, glyph masking or CSS cascade support in the
native model. **The native document scroll extents still include clipped
descendant geometry**, unlike complete CSS overflow propagation. The
separate `Canvas::blit_mono` operation does not automatically inherit a
node's clip. No additional CI workflow or runner matrix is introduced.
