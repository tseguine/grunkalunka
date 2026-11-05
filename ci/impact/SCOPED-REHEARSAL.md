# Native layout impact rehearsal and historical sample

The original Linux-only advisory job `impact-layout-validation.yml` has been
retired to eliminate redundant PR-triggered runner allocation. The sample is
superseded by the **manual-only** native [two-platform PR rehearsal](../../.github/workflows/impact-platform-rehearsal.yml),
which exercises the **actual production PR selector** on Linux and Windows.
Canonical PR verification also exercises the production selector with the complete
fast PR profile as fallback for unsafe plans; `master` continues to run full checks. Native macOS
verification follows the separate master/manual and PR-baseline policy; the
rehearsal workflow does not allocate an Apple Silicon runner.

## Current native rehearsal

[`platform-rehearsal.mjs`](platform-rehearsal.mjs) replays the source-file list
from [merged PR #75](https://github.com/tseguine/grunkalunka/pull/75) against the
current graph and checkout. Layout now has downstream A1 consumers, so its
independent expected graph selection includes all six application checks: note
preview, service interaction, interaction replay/package, live session, session
transport and live application. Execution projects them through the same fast PR
profile: filtered note preview, full session transport and client contracts, and
two real HTTP smoke scenarios. Deferred exhaustive consumers remain visible in
the graph selection. The inexpensive impact/profile regressions also execute.
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

The workflow no longer runs automatically on planning/CI pull requests. Use
Actions → Cross-platform partial-PR execution rehearsal → Run workflow when a
forced partial-plan execution is useful; ordinary broad CI changes correctly
fall back to the complete fast PR profile. This preserves a diagnostic control
without allocating two extra runners per qualifying PR.

The diagnostic uses the same two-minute test and ten-minute compilation budgets
as production PR checks, inside a 15-minute hosted job. Discovery has a
30-second limit. Full master verification retains the exhaustive scenarios;
this rehearsal does not claim to execute them.

[`platform-rehearsal.test.mjs`](platform-rehearsal.test.mjs) runs the production
planner and executor selection over a bounded unit graph with the current A1
descriptors. It detects a missing layout-consumer edge, omitted application
commands or build prerequisites, and accidental Cargo test expansion. This test
does not supply Cargo metadata or execute native commands; the hosted rehearsal
remains an optional native execution control.

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
