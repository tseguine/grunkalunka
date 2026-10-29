# A1 candidate: source-scripted Markdown outline presentation

This is the first **application-integration** unit following the maintainer's
A1 workstream assignment, informed by [candidate #54](https://github.com/tseguine/grunkalunka/issues/54)
and [steering #52](https://github.com/tseguine/grunkalunka/issues/52).
It is not acceptance of the whole A1 application, a final app/product choice or
an assertion that Grunkalunka now implements HTML, DOM, CSS or a renderer.

`examples/note-preview/render.mjs` accepts an **already authorized, trusted-host**
result with the lifted Markdown/WIT outline shape, its explicit source bytes,
profile and per-call backing-read count. The presentation layer performs **no
filesystem or network access**. Granted input yields deterministic inert semantic
HTML with source byte ranges; denied input yields a static access-denied state,
rejecting any supplied heading records or protected backing reads. It strictly
validates UTF-8 source length (4,096 bytes), heading count (64), aggregate UTF-8
label bytes (4,096), nonoverlapping heading source ranges, parent indices,
section boundaries and the selected visible count. Text uses HTML text escaping
(including NUL substitution). Relative Markdown paths are never resolved.

`toggle.js` and `toggle.ts` are real source-level JS/TS fixtures. The existing
compiler emits core Wasm from each. The Node host uses the **existing bounded
numeric Wasm runner** to execute that artifact with `(expanded, headingCount)`;
the *Wasm result*, not a JavaScript reimplementation of the selector, chooses
collapsed or expanded HTML. Tests mutate the actual compiled source and require
the rendered HTML to change. The source is not allowed direct DOM, document or
filesystem access. No new compiler/IR/lowering, public WIT ABI, Rust renderer,
shared runner, dependencies or Actions jobs are introduced.

Run the focused tests after building the workspace (including the CLI executable):

```sh
node --test tests/applications/note-preview.test.mjs
```

The canonical command catalog registers this ordinary test under one independent
manifest and an impact-selection descriptor; there is no separate PR workflow.

## Deliberate evidence limits / next A1 tasks

This unit uses **representative, independently checked host-provided heading
records**, not a live WIT component call. Its permission tests validate the
presentation adapter's input contract, **not** the guest's real read counters.
Existing D1 tests separately prove the actual guest's grant/deny properties.
The next integration unit must execute the actual Markdown/WIT guest under both
profiles and feed its owned result and real per-call counters to this adapter,
checking the native Markdown projection against the same source bytes. It must
not fake a permitted WIT read using a synthetic object.

After that, a separate bounded step must feed content/visibility into
Grunkalunka's **native** layout/paint/input pipeline (with explicit HTML parsing
or a documented semantic adapter). Outputting an HTML string is not evidence
that the native renderer displays it. The final A1 acceptance also requires
real build/run/debug/package flows and the same runnable application under two
capability profiles. Those criteria remain **Planned**.

No browser-oracle artifacts, current P1 measurement code or compiler-lowering
paths are changed by this preparatory A1 unit. The proposed note viewer remains
a small integration target; a richer Obshittian-style application, writable
vault, persistence and a public source-language DOM API remain unselected.
