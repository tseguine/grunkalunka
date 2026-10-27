# Selected CSS Grid automatic placement and CSSOM mutation oracle

`tests/browser-oracles/grid-placement.html` is an original, text-free, fixed-size
fixture for **CSS Grid row-major auto-placement in order-modified document
order**. Four empty items have no intrinsic sizes, borders, margins, padding,
or minimum-size constraints. The container is 160×120px, explicitly tracks
40px/70px (temporarily 50px/60px and then 40px/50px/30px), `grid-auto-rows:20px`,
initial `column-gap:10px` and `row-gap:5px`, and stretch alignment. Because
all track sizes are fixed lengths, all selected rectangles have exact integral
coordinates. These expectations are calculated separately from the browser
capture and are never injected into the page.

The verifier follows [CSS Grid § auto-placement](https://www.w3.org/TR/css-grid-1/#auto-placement-algo),
[CSS Grid `order` behavior](https://www.w3.org/TR/css-grid-1/#order-property),
[explicit track sizing](https://www.w3.org/TR/css-grid-1/#explicit-grids), and
[CSS Box Alignment gaps](https://www.w3.org/TR/css-align-3/#gaps). In this
restricted case, the auto-placement cursor fills tracks in order, then moves
to the next 20px row separated by the row gap. The `order:-1` mutation moves
`delta` to the first placement slot, leaving the original DOM order unchanged.

| Phase | Columns | Gaps (col,row) | Item rectangles `(x,y,width)` in DOM order |
| --- | --- | --- | --- |
| initial | 40/70 | 10/5 | (0,0,40) (50,0,70) (0,25,40) (50,25,70) |
| widen-first-track | 50/60 | 10/5 | (0,0,50) (60,0,60) (0,25,50) (60,25,60) |
| restore-tracks | 40/70 | 10/5 | Same as initial |
| increase-column-gap | 40/70 | 25/5 | (0,0,40) (65,0,70) (0,25,40) (65,25,70) |
| increase-row-gap | 40/70 | 25/15 | (0,0,40) (65,0,70) (0,35,40) (65,35,70) |
| order-delta-first | 40/70 | 25/15 | (65,0,70) (0,35,40) (65,35,70) (0,0,40) |
| restore-order | 40/70 | 25/15 | Same as increase-row-gap |
| restore-gaps | 40/70 | 10/5 | Same as initial |
| three-columns | 40/50/30 | 10/5 | (0,0,40) (50,0,50) (110,0,30) (0,25,40) |
| restore-two-columns | 40/70 | 10/5 | Same as initial |

The verifier checks all ten complete snapshots: computed/inline CSS, DOM
order, computed item widths/placement, item rectangles, container rectangle,
source hash and collector/host/browser/viewport provenance. Three intentionally
modified HTML sources skip the temporary track width change, the row-gap
increase, or the order mutation. Each requires an **entire valid alternate
trace** with the changed source SHA-256, followed by baseline rejection at
the intended first divergent phase. Fresh contexts must repeat exactly and
all sources/captures are retained before validation.

Offline tests use explicitly synthetic receipts to validate the model, errors
and artifact retention, **not to claim any browser passed**. Normative selected
execution requires real managed Chromium, Firefox and WebKit jobs plus the
joined report and full CI on the exact head. Engine agreement is useful
comparison, not standards proof. This fixture does not exercise intrinsic or
fractional tracks, dense packing, named grid lines, spans, writing modes,
fragmentation or the complete Web Platform Tests.
