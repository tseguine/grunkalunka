# Selected multi-line Flexbox wrapping and CSSOM mutation oracle

`tests/browser-oracles/flex-wrap-lines.html` is an original, single-file,
font-independent oracle. The row is 150px wide and 120px high with three empty,
20px-high flex items, each having `flex:0 0 60px`, except a temporary 90px
basis on the middle item. Its `flex-shrink:0` keeps deliberately overflowing
`nowrap` geometry observable. No margin, padding, border, font, or auto
minimum width affects the selected integers. `align-content:flex-start` and
`align-items:flex-start` avoid stretching and free-space alignment. The fixture
records nine successive computed/inline CSSOM states and complete rectangles;
**it does not receive or calculate expected coordinates**.

The independent verifier applies the [CSS Flexbox line collection algorithm](https://www.w3.org/TR/css-flexbox-1/#algo-line-break),
[`flex-wrap` line behavior](https://www.w3.org/TR/css-flexbox-1/#flex-wrap-property),
and the [CSS Box Alignment row/column gaps](https://www.w3.org/TR/css-align-3/#gaps).
For `wrap`, an item starts a new line when including it and the column gap
would exceed 150px; lines are 20px high and successive lines are offset by
`20px + row-gap`. For `nowrap`, the three items remain on one line even
when their fixed total exceeds 150px. All chosen positions are integral,
so this selected check requires exact equality, not a rounding tolerance.

| Phase | Wrap | Item widths | Column / row gap | x positions | y positions |
| --- | --- | --- | --- | --- | --- |
| initial-nowrap | nowrap | 60/60/60 | 10/10 | 0/70/140 | 0/0/0 |
| enable-wrap | wrap | 60/60/60 | 10/10 | 0/70/0 | 0/0/30 |
| widen-beta | wrap | 60/90/60 | 10/10 | 0/0/0 | 0/30/60 |
| restore-beta | wrap | 60/60/60 | 10/10 | 0/70/0 | 0/0/30 |
| increase-column-gap | wrap | 60/60/60 | 40/10 | 0/0/0 | 0/30/60 |
| increase-row-gap | wrap | 60/60/60 | 40/25 | 0/0/0 | 0/45/90 |
| restore-column-gap | wrap | 60/60/60 | 10/25 | 0/70/0 | 0/0/45 |
| disable-wrap | nowrap | 60/60/60 | 10/25 | 0/70/140 | 0/0/0 |
| restore-wrap | wrap | 60/60/60 | 10/25 | 0/70/0 | 0/0/45 |

The verifier checks all nine complete snapshots: row computed properties,
inline mutations, individual computed CSS and DOM state, all coordinates,
phase order, and collector source/version/host/viewport provenance. Fresh
managed contexts must repeat exactly. Three intentionally changed **HTML
sources** skip the initial wrapping enable, the temporary basis widening,
or the row-gap change. Each has its own SHA-256 receipt and must pass its
entire independently derived alternate trace before the baseline verifier
rejects the targeted phase. All five source/capture pairs remain in the
per-engine artifact, including failed attempts with collected captures.

The no-browser unit tests use **synthetic and clearly labeled** handwritten
receipts solely to exercise the verifier, runner and mutation controls; they
are not evidence of browser execution. Actual normative execution requires
all three managed Chromium, Firefox and WebKit smoke jobs plus the joined
cross-engine report and complete applicable CI on the exact PR head.
Agreement between engines is diagnostic, not proof of full conformance.
These selected cases do not cover flex shrink/grow, wrap-reverse, intrinsic
sizes, baseline alignment, writing modes, paint, or the full WPT suite.
