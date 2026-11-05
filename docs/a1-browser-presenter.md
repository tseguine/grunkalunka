# A1 presenter evidence in real browsers

The A1 live page is now an executable consumer of the existing independent
Chromium, Firefox and WebKit smoke environment. The presenter test serves the
checked-in `examples/note-preview/live-page.mjs` HTML and styles and
`examples/note-preview/live-client.mjs` module over loopback HTTP, then uses real
browser input, canvas and storage APIs. The HTTP backend supplies controlled
frames and records the requests it actually receives.

This is evidence for the **production browser presenter and transport boundary**.
It does not launch the native worker, compile a note application, call the WIT
service, read a user's note, or compare native renderer pixels to a browser
renderer. Native application, provider and queue tests retain their separate
contracts. A passing presenter fixture cannot replace them.

## Observations and independent control

The runner checks semantic observations against literal expected values. Its
expected pointer coordinates, scroll targets, selected note and RGB samples do
not call the production conversion or decode helpers.

| Boundary | Required observation |
| --- | --- |
| Launch and refresh | The fragment disappears, authority stays in tab-scoped session storage, local storage stays empty, and refresh performs another synchronization without consuming an application event. Request URLs contain no authority or query string. |
| Canvas | The real 320 × 160 backing store has exact opaque RGBA samples at three separated coordinates for each committed view. CSS presentation is checked at 640 × 320 and 320 × 160. |
| Pointer and keyboard | Real pointer input at both CSS scales reaches native coordinate `(24,16)`. Enter and Space activate the advertised control, and pointer interaction focuses the canvas. |
| Scrolling | A 64 CSS-pixel wheel movement becomes 32 native pixels at double scale and 64 at native scale. Arrow, Page Down, Home and End reach their expected bounded targets. A consumed wheel does not scroll the page. |
| Visible scroll controls | Real touch activates Page down and Top; normal HTML Enter and Space activate Bottom and Page up. Disabled states match each boundary and unavailable view, touch targets are at least 44 CSS pixels high, the layout changes from four columns to two, and keyboard focus stays on an enabled button. |
| Note replacement | A held selection keeps the prior selected value and pixels while controls are busy. A controlled load failure preserves that note; success commits the new note and pixels together. HTML-looking labels remain literal option text. Reload refreshes the selected note. |
| Lost reply | The backend commits one activation and deliberately truncates its reply. The presenter disables mutations, retains the last completed pixels and exposes Reconnect. Synchronization displays the committed result without repeating the activation or consuming another event. |
| Input generation | Mutations carry the displayed generation. A controlled stale-generation rejection commits no event and disables input until a fresh synchronization. |

A separate fresh browser context receives an alternate copy of the production
client with exactly one served-source mutation: pointer x is offset by one.
Its real request must change from `(24,16)` to `(25,16)`, and the normal literal
expectation must reject that value. The checked-in client is unchanged. Receipts
retain both served hashes and source hashes, so this control establishes that
the observation distinguishes an actual changed client from the original.

The solid fixture frames deliberately isolate transfer and canvas decoding.
Three pixel samples are not a full screenshot comparison, general layout oracle,
or proof of text shaping, accessibility-tool behavior or arbitrary browser input.

## Execution and receipts

`tools/browser-oracles/hosted/a1-presenter.test.mjs` is ordinary `node:test` code
invoked alongside the existing hosted smoke test in each existing engine job.
It requires the same exact Node version, Playwright package and managed-engine
image as that workflow. It adds no Actions job, engine matrix, package version or
native build. The whole step retains its outer timeout; each presenter test and
browser action has its own bound.

The existing artifact `browser-smoke-<engine>` includes:

- `a1-presenter/observations.json`: engine/browser/Node/automation versions, Git
  revision, hashes of proof and production sources, served asset hashes, observed
  views, received mutation and synchronization requests, final fixture state and
  sensitivity-control result.
- `a1-presenter/presenter.png`: the final production page, retained for inspection
  rather than used as a pixel-parity assertion.

The output directory must be fresh; the runner does not overwrite earlier
receipts. Failure writes a failed receipt with completed observations. The local
server bounds source bytes, request bytes, request count, event count, sockets and
input wait time, and closes its held operations and sockets in cleanup. Each
browser run owns fresh contexts and an ephemeral loopback origin.

For a reproduction in the pinned workflow environment, with the optional
`tools/browser-oracles` dependency installed, set `BROWSER_ENGINE`,
`BROWSER_NODE_VERSION` and a fresh `BROWSER_A1_EVIDENCE_DIR`, then run:

```sh
node --test tools/browser-oracles/hosted/a1-presenter.test.mjs
```

`runA1Presenter` also accepts an explicit browser implementation, automation
version, output directory and optional executable path for local diagnostics.
Those receipts describe that actual local engine; they are not the pinned hosted
result. If a local environment cannot start the engine, its fixture unit tests or
an unexecuted runner must not be reported as browser execution.

## Canonical selection and hosted triggers

The ordinary offline fixture tests run through
`node scripts/check-browser-oracles.mjs` without installing Playwright. They check
the controlled backend's deliberate lost-reply and stale-generation behavior,
source mutation and rejected request shapes. The browser test group's selection edge to
the A1 owner covers its actual production imports. Its explicit browser-only
build dependency prevents an edit to the fixture from acquiring native build
prerequisites solely through that consumer edge.

The production client/page, presenter runner/backend and hosted entry point are
exact inputs in both smoke event filters and its independent-workflow descriptor.
Offline-only test edits stay on the canonical runner. Trigger and command drift
tests fail closed. The repaired layout rehearsal also includes the presenter
consumer in its unit graph: building A1 prerequisites for a layout change does
not select that owner's other tests. Its six A1 Node consumers, six native build
packages and layout-only Cargo test/Clippy scope remain unchanged.

See [the external-workflow audit](../ci/impact/EXTERNAL-WORKFLOWS.md) for the
separate trigger contract. A valid audit predicts which jobs should run; only
the exact-head hosted results establish execution in all three engines.
