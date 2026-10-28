# Real-engine baseline smoke

The separate `Browser engine smoke` CI workflow runs Chromium/Blink,
Firefox/Gecko and WebKit in independent, non-fail-fast Linux jobs. Missing
binaries or failed launches fail the job; no engine is silently skipped.
It leaves the root compiler/runtime dependencies and canonical workflow alone.

Hosted smoke jobs now run in the official Playwright v1.64.1 Noble
container (version tag plus immutable digest, see the workflow), which includes the three Playwright browsers and Linux dependencies.
The workflow still installs its optional pinned Playwright **Node package**.
The hosted `node:test` suite verifies the browser path and package revision;
it no longer invokes `install --with-deps` and downloads
Ubuntu packages afresh on every runner. The container runs as UID 1001 rather
than root. See [Playwright's CI guidance](https://playwright.dev/docs/ci).
The image tag must match `tools/browser-oracles/package.json` (enforced by a
workflow regression test), and the digest is pinned to the exact image observed in all three successful
hosted jobs on the first image trial. Future image changes require an explicit
review and fresh hosted browser evidence.
Actual launches and tests, not the existence of the image, are the execution gate.
The change is subject to hosted validation, including cold image retrieval;
preprovisioning is intended to avoid flaky apt mirrors, not a promised speedup.
Local non-container collection still requires installing the engines and system
dependencies appropriate to the local OS.

After installing the optional dependency and browsers as in `README.md`:

```sh
node --test tools/browser-oracles/verify-baseline.test.mjs
BROWSER_ENGINE=chromium BROWSER_NODE_VERSION="$(node --version | cut -c 2-)" \
BROWSER_EVIDENCE_DIR=target/browser-smoke/local-new \
node --test tools/browser-oracles/hosted/smoke.test.mjs
```

Each run requires a new output directory. It copies the self-contained baseline
fixture to a retained read-only snapshot, checks its SHA-256 against each receipt,
and captures twice in fresh browser instances. The independent verifier requires
all three observation IDs and all ten requested computed properties. Selected
geometry is derived from CSS percentage widths and box sizing, not another engine:
parent content width 210, child content width 105 and border width 119 CSS px.
See the specification references and arithmetic in `verify-baseline.mjs`.

A separate source copy changes the child width from 50% to 60%. It must produce
126 CSS px content width and 140 border width, fail the original source/geometry
checks, and yield an observable comparator difference. This comparison of
intentionally different source hashes is a sensitivity experiment, NOT a normal
cross-engine comparison. Repeated identical input must compare exactly equal.

CI retains source copies, raw captures, comparison reports, browser version and
host provenance, font inventory, the resolved npm lockfile and Playwright browser
revision manifest. The optional package version is pinned, but installation still
resolves dependencies: retaining its generated lockfile records that run, not a
claim of a frozen transitive dependency or OS/font image. CI bounds the smoke
process to 240 seconds (then forced termination after 10 more), within a
15-minute job budget including container image initialization and package setup.

This is one selected block-layout example and harness sensitivity evidence, not
WPT execution, broad standards conformance, screenshot equivalence or native
Grunkalunka rendering. Font-dependent inline dimensions and parent height are
recorded and checked for same-engine repeatability, not asserted across engines.

Diagnostic JSON is transported in a hidden element, not rendered into the page.
[HTML's hidden state](https://html.spec.whatwg.org/multipage/interaction.html#the-hidden-attribute)
and [nonrendering box behavior](https://www.w3.org/TR/css-display-3/#valdef-display-none)
motivate a live-layout probe at 800x600 and 320x64 CSS px. After fonts are ready,
emptying, enlarging and restoring the diagnostics must preserve measured boxes,
viewport, scroll position and document extents exactly. A visible 4,096-line
control must generate boxes and increase scroll height, then rehiding it must
restore the original layout. Raw phases are saved in `diagnostic-isolation.json`
before verification; the existing process watchdog bounds the entire probe.
This is a selected-fixture isolation check, not a general layout-stability proof.

Hosted browser observations and the cross-engine report now execute as ordinary
`node:test` files under `tools/browser-oracles/hosted/`. GitHub Actions
selects the pinned image, sets the engine and evidence directories, and uploads
unmodified receipts; no fixture expectations or source-mutation assertions
are implemented in workflow YAML. Offline contracts remain separate from
these real-engine tests and require no Playwright installation.
