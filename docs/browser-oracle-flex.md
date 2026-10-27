# Selected flexbox layout and live CSSOM oracle

`tests/browser-oracles/flex-mutations.html` is an original self-contained,
font-independent fixture, not a copied Web Platform Test. Three empty flex items
sit in a 300×40 CSS-pixel row with 10px initial gap. Their fixed flex bases are
50 and 30px; the middle item has zero basis and normally consumes positive free
space with grow factor one. Explicit height 20px, `min-width:0`, no padding or
borders make the measured geometry independent of glyph metrics or default
minimum content sizes. The browser records nine synchronous phases after
individual CSSOM mutations; **the expected geometry is never sent to the page**.

The independent `tools/browser-oracles/flex.mjs` calculates the expected item
positions from the CSS flex main-axis sizing algorithm and gap spacing. Because
all bases, sizes, gaps and free space are integral, the selected tests require
exact pixel equality rather than rounding tolerances:

| Phase | Item content widths (α/β/γ) | Item x positions (α/β/γ) | Rule |
| --- | --- | --- | --- |
| initial | 50 / 200 / 30 | 0 / 60 / 270 | Middle item grows into 200px positive free space |
| no-growth | 50 / 0 / 30 | 0 / 60 / 70 | Removed grow capacity leaves 200px unused |
| restore-growth | 50 / 200 / 30 | 0 / 60 / 270 | Restore declared grow factor |
| increase-basis | 80 / 170 / 30 | 0 / 90 / 270 | First fixed basis rises from 50px to 80px |
| increase-gap | 80 / 150 / 30 | 0 / 100 / 270 | Two gaps rise from 10px to 20px |
| reverse-axis | 80 / 150 / 30 | 220 / 50 / 0 | Main-start moves to the right edge |
| restore-gap | 80 / 170 / 30 | 220 / 40 / 0 | Reversed layout with 10px gap |
| restore-axis | 80 / 170 / 30 | 0 / 90 / 270 | Forward axis again |
| restore-basis | 50 / 200 / 30 | 0 / 60 / 270 | Full layout recovery |

The expectations use the [CSS Flexbox main sizing algorithm](https://www.w3.org/TR/css-flexbox-1/#layout-algorithm),
[flex direction](https://www.w3.org/TR/css-flexbox-1/#flex-direction-property),
[box alignment gap](https://www.w3.org/TR/css-align-3/#gaps), and
[CSSOM style updates](https://www.w3.org/TR/cssom-1/#dom-cssstyledeclaration-setproperty).
The verifier also compares computed display/flex properties, complete rectangles,
actual inline property values, row gap/direction, phase ordering, and exact
source/engine/host/viewport provenance.

`node --test tests/browser-oracles/flex-mutations.test.mjs` exercises the
independent verifier and its runner without requiring a browser. The managed
Chromium/Firefox/WebKit smoke separately captures the page twice through the
existing production file-origin collector and tests three guarded **source
mutations**: skip the no-growth update, skip the gap increase, and skip main-axis
reversal. Every mutation supplies its own source hash and must satisfy a complete
alternate manifest **before** the baseline's targeted semantic failure. An
unrelated corrupt observation or a mere hash mismatch cannot count as detection.
Source snapshots, raw captures and summary are retained under `flex/` before
verification. Existing block-layout, cascade, diagnostic-isolation and event
checks remain mandatory and use the same bounded smoke workflow.

Nine selected synchronous flex transitions do not establish full layout/WPT
conformance, painting, resize/layout-thrashing scheduling, auto minimum-size
behavior, text measurement, accessibility, a native renderer or browser feature
support. Engine consensus is not a substitute for the independent CSS rules.
