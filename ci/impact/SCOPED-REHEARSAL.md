# Native layout impact rehearsal and historical sample

The original Linux-only advisory job `impact-layout-validation.yml` has been
retired to eliminate redundant PR-triggered runner allocation. The sample is
superseded by the native [two-platform PR rehearsal](../../.github/workflows/impact-platform-rehearsal.yml),
which exercises the **actual production PR selector** on Linux and Windows.
Canonical PR verification also exercises the production selector with strict full
fallback for unsafe plans; `master` continues to run full checks. Native macOS
verification follows the separate master/manual and PR-baseline policy; the
rehearsal workflow does not allocate an Apple Silicon runner.

## Current native rehearsal

[`platform-rehearsal.mjs`](platform-rehearsal.mjs) replays the source-file list
from [merged PR #75](https://github.com/tseguine/grunkalunka/pull/75) against the
current graph and checkout. Layout now has downstream A1 consumers, so its
independent expected commands include all five application checks: note preview,
service interaction, interaction replay/package, live session and live application.
Their build closure includes CLI, compiler, layout, Markdown, runtime and Wasm-link.
Only layout is selected for Cargo tests and Clippy; building the other packages
does not select their own tests. Workspace formatting remains global.

After preparing the normal repository dependencies, run
`RUNNER_OS=Linux node ci/impact/platform-rehearsal.mjs` on Linux, or use the
corresponding native `RUNNER_OS` value on Windows or macOS. The current workflow
runs Linux/Windows, and the script accepts a manual native macOS rehearsal.
The script verifies real Cargo metadata, the live canonical catalog and independent
browser triggers before checking and executing its pinned selection. Its receipt
does not establish that a separate browser workflow ran or replace canonical CI.

Selected commands have a ten-minute execution limit within a 35-minute hosted
job. Discovery retains its shorter limit. The former five-minute command limit
terminated the Linux live-session suite after 21 passing cases in
[run 41989066588](https://github.com/tseguine/grunkalunka/actions/runs/41989066588),
while the Windows rehearsal completed its full selected closure. These bounds
accommodate the existing native/WIT consumers without changing the selected
tests, packages, host matrix or canonical test timeouts.

[`platform-rehearsal.test.mjs`](platform-rehearsal.test.mjs) runs the production
planner and executor selection over a bounded unit graph with the current A1
descriptors. It detects a missing layout-consumer edge, omitted application
commands or build prerequisites, and accidental Cargo test expansion. This test
does not supply Cargo metadata or execute native commands; the hosted rehearsal
remains the native execution check.

## Historical manual sample

For reproducing the historical sample locally, the standalone
[`run-layout-sample.mjs`](run-layout-sample.mjs) script and
[`validate-layout-selection.test.mjs`](validate-layout-selection.test.mjs)
regression tests are retained. Its original four-command, layout-only expectation
predates the A1 consumers and deliberately refuses the current wider graph. Use
the current native rehearsal above for current-source execution; the old sample
is not a current passing rehearsal. The regression tests are included in the
existing `ci/impact/*.test.mjs` runner used by canonical CI and the native rehearsal.

The sample replays the exact changed-file list from
[merged PR #75](https://github.com/tseguine/grunkalunka/pull/75):
`crates/grunkalunka-layout/examples/click_reflow_ppm.rs`,
`crates/grunkalunka-layout/tests/headless_input_reflow.rs`, and
`docs/headless-hit-test.md`. If executed, it tests **current checkout source**,
not historic PR #75 source, and refuses execution if its pinned package-scoped
commands or graph assumptions differ. Historical sample receipts are research
evidence, not a required check or proof of dependency completeness.

There is no longer a separate automatically triggered Linux job for this sample.
Do not reintroduce one without an independently justified execution requirement.
